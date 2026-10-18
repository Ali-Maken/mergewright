//! The terminal session: everything Mergewright switches on in the user's terminal
//! at startup, and switches back off — in the exact reverse order — when it
//! stops, however it stops (PROMPT.md §6, non-negotiable principle 8).
//!
//! [`TerminalSession`] is an RAII guard: hold it for as long as Mergewright draws,
//! drop it and the terminal is the user's again. Restoring is idempotent, so
//! the panic hook ([`crate::crash`]) can restore first and the guard's `Drop`,
//! running later as the stack unwinds, emits nothing a second time.
//!
//! Two things the session touches are deliberately separated: the escape
//! sequences, which are queued to a plain [`Write`] sink so a test can read the
//! exact byte stream back, and raw mode, which is a `termios` call on the
//! controlling terminal and so sits behind the [`RawMode`] seam.

use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};

use crate::cli::COMMAND;
use crossterm::QueueableCommand;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

/// What the session turns on at startup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionOptions {
    /// Push the kitty keyboard protocol's disambiguation flag.
    ///
    /// Off unless the caller knows the terminal supports it: ticket 11 passes
    /// the answer from the detected `Capabilities`. Disambiguation is what
    /// makes `Ctrl+Shift+Z` (redo, DESIGN.md §2.2) distinguishable from
    /// `Ctrl+Z` at all; the legacy encoding sends the same bytes for both.
    pub kitty_keyboard: bool,
}

/// One terminal mode the session owns, listed in the order it is turned on.
/// Teardown walks the applied steps backwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    RawMode,
    AlternateScreen,
    MouseCapture,
    BracketedPaste,
    FocusEvents,
    HiddenCursor,
    KittyKeyboard,
}

/// What [`TerminalSession::suspend`] gave up, for [`TerminalSession::resume`]
/// to put back. Opaque on purpose: which modes were on is the session's
/// business, and there is nothing a caller can usefully do with the list but
/// hand it straight back.
#[derive(Clone, Debug)]
pub struct Suspended(Vec<Step>);

/// Raw mode, behind a seam.
///
/// Unlike every other step this writes no bytes to the output stream — it is a
/// `termios` call on the controlling terminal — so it can neither be captured
/// by a test sink nor be undone by writing an escape sequence.
pub trait RawMode: Send {
    /// Turn raw mode on. Keys arrive unbuffered and unechoed from here.
    ///
    /// # Errors
    /// Whatever the `termios` call failed with — usually "not a terminal".
    fn enable(&mut self) -> io::Result<()>;

    /// Put the terminal back the way it was found.
    ///
    /// # Errors
    /// As [`RawMode::enable`]. The caller reports it and carries on: a shell
    /// left in raw mode is bad, and refusing to finish shutting down is worse.
    fn disable(&mut self) -> io::Result<()>;
}

/// Raw mode on the real controlling terminal.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessRawMode;

impl RawMode for ProcessRawMode {
    fn enable(&mut self) -> io::Result<()> {
        crossterm::terminal::enable_raw_mode()
    }

    fn disable(&mut self) -> io::Result<()> {
        crossterm::terminal::disable_raw_mode()
    }
}

/// The modes currently applied, plus the two things needed to undo them.
///
/// Shared behind an `Arc<Mutex<…>>` with every [`TerminalRestorer`] so that a
/// restore from the panic hook and a restore from `Drop` cannot both fire.
struct SessionState {
    sink: Box<dyn Write + Send>,
    raw: Box<dyn RawMode>,
    applied: Vec<Step>,
}

/// An RAII guard over the terminal modes Mergewright turns on. Dropping it restores
/// the terminal.
pub struct TerminalSession {
    state: Arc<Mutex<SessionState>>,
}

impl TerminalSession {
    /// Set up the real terminal: escape sequences to stdout, raw mode on the
    /// controlling terminal.
    pub fn on_stdout(options: SessionOptions) -> io::Result<Self> {
        Self::new(io::stdout(), ProcessRawMode, options)
    }

    /// Set up a terminal described by an arbitrary sink and raw-mode
    /// implementation. Tests use this to capture the byte stream.
    ///
    /// If any step fails, every step that already succeeded is undone before
    /// the error is returned, so a half-finished setup never leaks modes.
    pub fn new(
        sink: impl Write + Send + 'static,
        raw: impl RawMode + 'static,
        options: SessionOptions,
    ) -> io::Result<Self> {
        let state = SessionState {
            sink: Box::new(sink),
            raw: Box::new(raw),
            applied: Vec::new(),
        };
        let session = Self {
            state: Arc::new(Mutex::new(state)),
        };

        if let Err(error) = session.setup(options) {
            // Roll back rather than hand out a guard over a half-open
            // terminal. A restore failure here is secondary: report the
            // failure that actually stopped startup.
            if let Err(restore_error) = session.restore() {
                report_to_stderr(&restore_error);
            }
            return Err(error);
        }
        Ok(session)
    }

    /// Push the kitty keyboard protocol's disambiguation flag now, after the
    /// session is already open.
    ///
    /// Startup cannot decide this up front: whether the terminal supports the
    /// protocol is something only the capability probe knows, and the probe
    /// needs raw mode — which means the session has to exist first. The flag
    /// joins the applied steps, so teardown pops it in the right order like
    /// any other.
    ///
    /// # Errors
    /// The sequence could not be written to the terminal.
    pub fn enable_kitty_keyboard(&self) -> io::Result<()> {
        let mut state = lock(&self.state)?;
        state.apply(Step::KittyKeyboard)?;
        state.sink.flush()
    }

    /// A handle that can restore this session from anywhere — specifically
    /// from the panic hook, which runs before `Drop` does.
    pub fn restorer(&self) -> TerminalRestorer {
        TerminalRestorer {
            state: Arc::clone(&self.state),
        }
    }

    /// Put every applied mode back, in reverse order. Idempotent: a second
    /// call writes nothing.
    pub fn restore(&self) -> io::Result<()> {
        restore(&self.state)
    }

    /// Give the terminal up for a moment: the same teardown [`restore`] does,
    /// but remembering what was turned on so [`Self::resume`] can put exactly
    /// that back (m3-working-tree-undo-and-stash/05).
    ///
    /// For `$EDITOR` and for `Ctrl+Z`: both hand the terminal to something
    /// that expects to find it the way the shell left it. Restoring and then
    /// setting up again from scratch would not do — setup applies the kitty
    /// keyboard flag only when `SessionOptions` said so, and by now that
    /// answer came from a capability probe nobody kept.
    ///
    /// # Errors
    /// A mode could not be undone. The session is left as close to open as it
    /// can be made, because the caller is not going to hand the terminal over
    /// after all and Mergewright has to keep drawing into it.
    pub fn suspend(&self) -> io::Result<Suspended> {
        let steps = lock(&self.state)?.applied.clone();
        match restore(&self.state) {
            Ok(()) => Ok(Suspended(steps)),
            Err(error) => {
                if let Err(second) = self.resume(Suspended(steps)) {
                    report_to_stderr(&second);
                }
                Err(error)
            }
        }
    }

    /// Take the terminal back, applying what [`Self::suspend`] recorded in
    /// the order it was first applied in.
    ///
    /// Says nothing about the screen's contents: whatever ran in between drew
    /// over every cell, so the caller redraws in full.
    ///
    /// # Errors
    /// A mode could not be turned back on. Mergewright is then drawing into a
    /// terminal that is not in the state it thinks it is; the caller stops.
    pub fn resume(&self, suspended: Suspended) -> io::Result<()> {
        let mut state = lock(&self.state)?;
        for step in suspended.0 {
            state.apply(step)?;
        }
        state.sink.flush()
    }

    fn setup(&self, options: SessionOptions) -> io::Result<()> {
        let mut state = lock(&self.state)?;
        state.apply(Step::RawMode)?;
        state.apply(Step::AlternateScreen)?;
        state.apply(Step::MouseCapture)?;
        state.apply(Step::BracketedPaste)?;
        state.apply(Step::FocusEvents)?;
        state.apply(Step::HiddenCursor)?;
        if options.kitty_keyboard {
            state.apply(Step::KittyKeyboard)?;
        }
        state.sink.flush()
    }
}

/// Deliberately opaque: the interesting state is the terminal itself, and
/// reading it would mean taking the lock.
impl fmt::Debug for TerminalSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TerminalSession")
            .finish_non_exhaustive()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        // The last line of defence for "leave the terminal clean", so there is
        // nobody left to propagate a failure to. Say so on stderr instead of
        // letting the user wonder why their shell is in raw mode.
        if let Err(error) = self.restore() {
            report_to_stderr(&error);
        }
    }
}

/// Restores a [`TerminalSession`] from outside it. Cloneable and `Send` so the
/// panic hook can hold one.
#[derive(Clone)]
pub struct TerminalRestorer {
    state: Arc<Mutex<SessionState>>,
}

impl fmt::Debug for TerminalRestorer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TerminalRestorer")
            .finish_non_exhaustive()
    }
}

impl TerminalRestorer {
    /// Put every applied mode back, in reverse order. Idempotent, so it is
    /// safe to call this and then let the session's `Drop` run as well.
    pub fn restore(&self) -> io::Result<()> {
        restore(&self.state)
    }
}

impl SessionState {
    fn apply(&mut self, step: Step) -> io::Result<()> {
        match step {
            Step::RawMode => self.raw.enable()?,
            Step::AlternateScreen => {
                self.sink.queue(EnterAlternateScreen)?;
            }
            // crossterm's mouse capture includes any-motion reporting (DEC
            // mode 1003), which is what hover needs (PROMPT.md Technology).
            Step::MouseCapture => {
                self.sink.queue(EnableMouseCapture)?;
            }
            Step::BracketedPaste => {
                self.sink.queue(EnableBracketedPaste)?;
            }
            Step::FocusEvents => {
                self.sink.queue(EnableFocusChange)?;
            }
            Step::HiddenCursor => {
                self.sink.queue(Hide)?;
            }
            Step::KittyKeyboard => {
                self.sink.queue(PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
                ))?;
            }
        }
        // Recorded only once the step has been written in full. A step whose
        // write fails part-way through (crossterm expands mouse capture into
        // five separate sequences) is therefore never undone — deliberately: a
        // sink that fails mid-sequence is a broken file descriptor, so the undo
        // write would fail on it too. Finer-grained tracking would buy nothing.
        self.applied.push(step);
        Ok(())
    }

    fn undo(&mut self, step: Step) -> io::Result<()> {
        match step {
            Step::RawMode => self.raw.disable(),
            Step::AlternateScreen => self.sink.queue(LeaveAlternateScreen).map(drop),
            Step::MouseCapture => self.sink.queue(DisableMouseCapture).map(drop),
            Step::BracketedPaste => self.sink.queue(DisableBracketedPaste).map(drop),
            Step::FocusEvents => self.sink.queue(DisableFocusChange).map(drop),
            Step::HiddenCursor => self.sink.queue(Show).map(drop),
            Step::KittyKeyboard => self.sink.queue(PopKeyboardEnhancementFlags).map(drop),
        }
    }
}

/// Undo every applied step, newest first, and flush.
///
/// A step is taken off the applied list before it is undone, so one failure
/// neither stops the rest nor gets retried: leaving the terminal in raw mode
/// because an escape sequence failed to write would be the worse outcome. The
/// first error is returned once everything else has been tried.
fn restore(state: &Arc<Mutex<SessionState>>) -> io::Result<()> {
    let mut state = lock(state)?;
    let mut first_error = None;
    while let Some(step) = state.applied.pop() {
        if let Err(error) = state.undo(step) {
            first_error.get_or_insert(error);
        }
    }
    if let Err(error) = state.sink.flush() {
        first_error.get_or_insert(error);
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Take the lock without ever blocking or panicking.
///
/// A blocking lock is not an option: the panic hook restores the terminal, and
/// if the panic happened *inside* a restore the hook would deadlock on a lock
/// its own thread already holds — a hang instead of a crash. A poisoned lock,
/// by contrast, is safe to adopt: `applied` only ever records steps that
/// succeeded, so the state is still an accurate picture of the terminal.
fn lock(state: &Mutex<SessionState>) -> io::Result<MutexGuard<'_, SessionState>> {
    match state.try_lock() {
        Ok(guard) => Ok(guard),
        Err(TryLockError::Poisoned(poisoned)) => Ok(PoisonError::into_inner(poisoned)),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "terminal restore already in progress on another thread",
        )),
    }
}

/// Last-resort reporting for a restore that failed where nothing can be
/// returned (a `Drop`, or a rollback inside `new`).
fn report_to_stderr(error: &io::Error) {
    // Ignoring this write is deliberate: stderr is the only channel left, and
    // failing to write to it leaves nothing to fall back on.
    let _ = writeln!(
        io::stderr(),
        "{COMMAND}: could not restore the terminal: {error}"
    );
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    /// Everything the session does, in order, as one readable transcript:
    /// escape sequences as the bytes themselves, raw mode as a marker, so a
    /// single assertion can pin the *interleaving* of the two.
    #[derive(Clone, Default)]
    pub(crate) struct Transcript(Arc<Mutex<Vec<u8>>>);

    impl Transcript {
        pub(crate) fn rendered(&self) -> String {
            let bytes = self.0.lock().unwrap();
            String::from_utf8_lossy(&bytes).replace('\x1b', "<ESC>")
        }

        fn note(&self, marker: &str) {
            self.0.lock().unwrap().extend_from_slice(marker.as_bytes());
        }
    }

    impl Write for Transcript {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A sink that writes everything to the transcript except one chosen
    /// sequence, which fails — a terminal that refuses one specific mode.
    struct PickySink {
        transcript: Transcript,
        fails_on: &'static str,
    }

    impl Write for PickySink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if String::from_utf8_lossy(buf).contains(self.fails_on) {
                return Err(io::Error::other("sink is broken"));
            }
            self.transcript.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Raw mode that only writes to the transcript, so its ordering relative
    /// to the escape sequences is visible.
    pub(crate) struct FakeRawMode {
        transcript: Transcript,
        enable_fails: bool,
    }

    impl RawMode for FakeRawMode {
        fn enable(&mut self) -> io::Result<()> {
            if self.enable_fails {
                return Err(io::Error::other("no controlling terminal"));
            }
            self.transcript.note("<RAW ON>");
            Ok(())
        }

        fn disable(&mut self) -> io::Result<()> {
            self.transcript.note("<RAW OFF>");
            Ok(())
        }
    }

    pub(crate) fn raw_mode(transcript: &Transcript) -> FakeRawMode {
        FakeRawMode {
            transcript: transcript.clone(),
            enable_fails: false,
        }
    }

    pub(crate) const KITTY: SessionOptions = SessionOptions {
        kitty_keyboard: true,
    };

    /// The whole contract in one assertion: which modes are turned on, in
    /// which order, and that teardown is that same list backwards.
    #[test]
    fn teardown_undoes_every_step_in_reverse_order() {
        let transcript = Transcript::default();
        let session =
            TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).unwrap();

        let after_setup = transcript.rendered();
        assert_eq!(
            after_setup,
            concat!(
                "<RAW ON>",
                "<ESC>[?1049h", // alternate screen
                "<ESC>[?1000h<ESC>[?1002h<ESC>[?1003h<ESC>[?1015h<ESC>[?1006h", // mouse, incl. motion
                "<ESC>[?2004h",                                                 // bracketed paste
                "<ESC>[?1004h",                                                 // focus events
                "<ESC>[?25l",                                                   // hide cursor
                "<ESC>[>1u",                                                    // kitty keyboard
            ),
        );

        session.restore().unwrap();
        let teardown = transcript
            .rendered()
            .strip_prefix(&after_setup)
            .map(str::to_owned)
            .unwrap();
        assert_eq!(
            teardown,
            concat!(
                "<ESC>[<1u",                                                    // kitty keyboard
                "<ESC>[?25h",                                                   // show cursor
                "<ESC>[?1004l",                                                 // focus events
                "<ESC>[?2004l",                                                 // bracketed paste
                "<ESC>[?1006l<ESC>[?1015l<ESC>[?1003l<ESC>[?1002l<ESC>[?1000l", // mouse
                "<ESC>[?1049l",                                                 // alternate screen
                "<RAW OFF>",
            ),
        );
    }

    /// `suspend` is the teardown and `resume` is the setup, and between them
    /// the terminal is exactly as the shell left it — which is what `$EDITOR`
    /// and `Ctrl+Z` both need (m3-working-tree-undo-and-stash/05).
    #[test]
    fn suspend_undoes_every_mode_and_resume_puts_the_same_ones_back() {
        let transcript = Transcript::default();
        let session =
            TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).unwrap();
        let after_setup = transcript.rendered();

        let suspended = session.suspend().expect("the terminal is given up");
        let while_outside = transcript
            .rendered()
            .strip_prefix(&after_setup)
            .map(str::to_owned)
            .unwrap();
        assert_eq!(
            while_outside,
            concat!(
                "<ESC>[<1u",
                "<ESC>[?25h",
                "<ESC>[?1004l",
                "<ESC>[?2004l",
                "<ESC>[?1006l<ESC>[?1015l<ESC>[?1003l<ESC>[?1002l<ESC>[?1000l",
                "<ESC>[?1049l",
                "<RAW OFF>",
            ),
            "the alternate screen goes last, so the editor draws over the shell",
        );

        session.resume(suspended).expect("the terminal comes back");
        let after_resume = transcript
            .rendered()
            .strip_prefix(&format!("{after_setup}{while_outside}"))
            .map(str::to_owned)
            .unwrap();
        assert_eq!(
            after_resume, after_setup,
            "resume applies the same modes in the same order setup did",
        );
    }

    /// Whether the kitty flag was on is remembered, not re-derived: the probe
    /// that decided it ran once, at startup, and nobody kept the answer.
    #[test]
    fn resume_does_not_turn_on_a_mode_suspend_did_not_find() {
        let transcript = Transcript::default();
        let session = TerminalSession::new(
            transcript.clone(),
            raw_mode(&transcript),
            SessionOptions::default(),
        )
        .unwrap();
        let suspended = session.suspend().unwrap();
        let before_resume = transcript.rendered();
        session.resume(suspended).unwrap();
        let after = transcript
            .rendered()
            .strip_prefix(&before_resume)
            .map(str::to_owned)
            .unwrap();
        assert!(
            !after.contains("<ESC>[>1u"),
            "kitty keyboard was never on: {after}",
        );
    }

    /// A session already restored has nothing to give up, and resuming that
    /// nothing is not an error — it is what happens when the loop tears down
    /// while an editor is somehow still open.
    #[test]
    fn suspending_an_already_restored_session_is_a_no_op() {
        let transcript = Transcript::default();
        let session =
            TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).unwrap();
        session.restore().unwrap();
        let before = transcript.rendered();

        let suspended = session.suspend().unwrap();
        session.resume(suspended).unwrap();
        assert_eq!(transcript.rendered(), before);
    }

    #[test]
    fn kitty_keyboard_stays_off_unless_asked_for() {
        let transcript = Transcript::default();
        let session = TerminalSession::new(
            transcript.clone(),
            raw_mode(&transcript),
            SessionOptions::default(),
        )
        .unwrap();
        drop(session);

        let rendered = transcript.rendered();
        assert!(!rendered.contains("<ESC>[>"), "pushed flags: {rendered}");
        assert!(!rendered.contains("<ESC>[<1u"), "popped flags: {rendered}");
    }

    #[test]
    fn restoring_twice_emits_nothing_the_second_time() {
        let transcript = Transcript::default();
        let session =
            TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).unwrap();

        session.restore().unwrap();
        let after_first = transcript.rendered();
        session.restore().unwrap();
        assert_eq!(transcript.rendered(), after_first);

        // …and the guard's own `Drop`, the third restore, is silent too. This
        // is what lets the panic hook restore before unwinding reaches `Drop`.
        drop(session);
        assert_eq!(transcript.rendered(), after_first);
    }

    #[test]
    fn a_setup_that_fails_midway_undoes_what_it_managed_to_apply() {
        let transcript = Transcript::default();
        // Raw mode, the alternate screen and mouse capture go on; bracketed
        // paste is the step that breaks.
        let sink = PickySink {
            transcript: transcript.clone(),
            fails_on: "?2004h",
        };

        let error = TerminalSession::new(sink, raw_mode(&transcript), KITTY).unwrap_err();

        assert_eq!(error.to_string(), "sink is broken");
        assert_eq!(
            transcript.rendered(),
            concat!(
                "<RAW ON>",
                "<ESC>[?1049h",
                "<ESC>[?1000h<ESC>[?1002h<ESC>[?1003h<ESC>[?1015h<ESC>[?1006h",
                // …and back out again: only the three steps that took.
                "<ESC>[?1006l<ESC>[?1015l<ESC>[?1003l<ESC>[?1002l<ESC>[?1000l",
                "<ESC>[?1049l",
                "<RAW OFF>",
            ),
        );
    }

    #[test]
    fn a_setup_that_fails_at_the_first_step_leaves_nothing_behind() {
        let transcript = Transcript::default();
        let raw = FakeRawMode {
            transcript: transcript.clone(),
            enable_fails: true,
        };

        let error = TerminalSession::new(transcript.clone(), raw, KITTY).unwrap_err();

        assert_eq!(error.to_string(), "no controlling terminal");
        assert_eq!(transcript.rendered(), "");
    }

    /// Raw mode outlives the process; a missed escape sequence only dirties
    /// the screen. So one failing undo must not abandon the rest.
    #[test]
    fn a_failing_undo_does_not_stop_the_rest_of_the_teardown() {
        let transcript = Transcript::default();
        let sink = PickySink {
            transcript: transcript.clone(),
            fails_on: "?1049l",
        };
        let session = TerminalSession::new(sink, raw_mode(&transcript), KITTY).unwrap();

        let error = session.restore().unwrap_err();

        assert_eq!(error.to_string(), "sink is broken");
        let rendered = transcript.rendered();
        assert!(!rendered.contains("<ESC>[?1049l"), "{rendered}");
        assert!(rendered.ends_with("<RAW OFF>"), "{rendered}");
    }

    #[test]
    fn a_restorer_taken_before_the_panic_restores_instead_of_the_guard() {
        let transcript = Transcript::default();
        let session =
            TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).unwrap();
        let restorer = session.restorer();

        restorer.restore().unwrap();
        let after_restorer = transcript.rendered();
        assert!(after_restorer.ends_with("<RAW OFF>"), "{after_restorer}");

        drop(session);
        assert_eq!(transcript.rendered(), after_restorer);
    }

    /// A panic while the state lock is held must not turn into a deadlock or a
    /// second panic inside the hook — `lock` never blocks, and adopts a
    /// poisoned lock.
    #[test]
    fn a_poisoned_lock_still_restores() {
        let transcript = Transcript::default();
        let session =
            TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).unwrap();
        let restorer = session.restorer();

        let poisoner = std::thread::spawn({
            let restorer = restorer.clone();
            move || {
                let _guard = lock(&restorer.state).unwrap();
                panic!("poison the lock");
            }
        });
        assert!(poisoner.join().is_err());

        restorer.restore().unwrap();
        assert!(transcript.rendered().ends_with("<RAW OFF>"));
    }
}
