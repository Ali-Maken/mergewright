//! Handing the terminal to something else and taking it back
//! (m3-working-tree-undo-and-stash/05).
//!
//! Two things need this: `Ctrl+E`, which runs the user's `$EDITOR` on the
//! commit message, and `Ctrl+Z` under `keys.suspend_on_ctrl_z`, which stops
//! Graft the way any other terminal program stops.
//!
//! Both are the same problem. Graft holds the terminal in a state no other
//! program expects — raw mode, the alternate screen, mouse reporting,
//! bracketed paste, focus events, a hidden cursor, the kitty keyboard
//! protocol — and it holds stdin through a reader thread that never stops
//! reading. Handing the terminal over means putting every one of those back
//! and getting the reader out of the way, in that order, and then doing the
//! reverse. PROMPT.md principle 8: a wrong restore leaves the user with a
//! broken shell, which is worse than never having run.
//!
//! The restore itself is [`graft_term::TerminalSession`]'s, not this
//! module's: it already knows what it turned on and undoes it newest-first,
//! and it is idempotent. This module only says *when*, and guarantees the
//! way back even if what ran in between panicked.

use std::io;
use std::path::Path;
use std::process::Command;

use graft_term::input::InputMux;

use crate::terminal::{Suspended, TerminalSession};

/// Why the terminal could not be handed over, or taken back.
#[derive(Debug)]
pub enum OutsideError {
    /// The terminal could not be put back the way the user's shell expects.
    /// Fatal on the way out: running an editor over a half-restored terminal
    /// is how a shell ends up unusable.
    Restore(io::Error),
    /// Graft could not re-enter the terminal it had just left. The user's
    /// shell is fine — Graft is not.
    Reenter(io::Error),
}

impl std::fmt::Display for OutsideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restore(error) => write!(f, "could not restore the terminal: {error}"),
            Self::Reenter(error) => write!(f, "could not re-enter the terminal: {error}"),
        }
    }
}

impl std::error::Error for OutsideError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Restore(error) | Self::Reenter(error) => Some(error),
        }
    }
}

/// Put the terminal back the way Graft found it, run `f` with it, then take
/// it back.
///
/// The order matters and is the reverse of setup: every mode Graft turned on
/// goes off (the alternate screen last, so the shell's scrollback is what the
/// editor draws over), the input reader is paused so it is not racing the
/// editor for keystrokes, `f` runs, and then everything goes back on.
///
/// The way back is a [`Reenter`] guard rather than a straight line, so a
/// panic inside `f` still re-enters the terminal before it unwinds — without
/// it a panicking editor path would leave the alternate screen off and the
/// panic hook drawing into the user's scrollback.
///
/// # Errors
/// [`OutsideError::Restore`] when the terminal could not be given up, in
/// which case `f` never runs; [`OutsideError::Reenter`] when it could not be
/// taken back, in which case `f` has already run and its value is lost — the
/// caller is about to exit anyway.
pub fn run_outside_terminal<T>(
    session: &TerminalSession,
    mux: &InputMux,
    f: impl FnOnce() -> T,
) -> Result<T, OutsideError> {
    // The reader first: if it cannot be taken off the terminal there is no
    // point restoring the modes, because Graft would still be eating the
    // editor's keystrokes. Nothing has changed yet, so this is a clean
    // refusal.
    mux.pause().map_err(OutsideError::Restore)?;
    let suspended = match session.suspend() {
        Ok(suspended) => suspended,
        Err(error) => {
            // `suspend` already put back what it could; the reader goes back
            // on the terminal too, or Graft would take no more keys.
            mux.resume();
            return Err(OutsideError::Restore(error));
        }
    };

    let guard = Reenter {
        session,
        mux,
        suspended: Some(suspended),
        done: false,
    };
    let value = f();
    guard.finish().map(|()| value)
}

/// Re-enters the terminal when it is dropped, panic or no panic.
struct Reenter<'a> {
    session: &'a TerminalSession,
    mux: &'a InputMux,
    suspended: Option<Suspended>,
    /// [`Self::finish`] already re-entered, so [`Drop`] has nothing to do.
    done: bool,
}

impl Reenter<'_> {
    /// Re-enter now, reporting a failure the caller can act on. `Drop` does
    /// the same thing silently, for the panicking path where there is no
    /// caller left to tell.
    fn finish(mut self) -> Result<(), OutsideError> {
        let result = self.reenter();
        self.done = true;
        result.map_err(OutsideError::Reenter)
    }

    /// The modes back on, the reader reading again, and a full redraw asked
    /// for — the editor has scribbled over every cell Graft thought it knew.
    fn reenter(&mut self) -> io::Result<()> {
        self.mux.resume();
        match self.suspended.take() {
            Some(suspended) => self.session.resume(suspended),
            // Only reachable if `finish` ran twice, which it cannot: it
            // takes `self` by value.
            None => Ok(()),
        }
    }
}

impl Drop for Reenter<'_> {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        // Unwinding: there is no caller to return an error to, and the panic
        // hook is about to print. Re-entering is still the right thing —
        // failing to is not swallowed, it is printed, because a terminal
        // left half-restored is the one thing the user must know about.
        if let Err(error) = self.reenter() {
            eprintln!("graft: could not re-enter the terminal after a panic: {error}");
        }
    }
}

/// The command that opens `path` in `git_editor`, at `line` where the editor
/// takes a line and Graft knows how to ask for it.
///
/// `git_editor` is what `git var GIT_EDITOR` printed, which is a **shell
/// word list**, not a program name: `emacs -nw` and `code --wait` are both
/// normal answers. The first word is the program and the rest are arguments
/// that must survive, so they are kept in front of anything added here.
///
/// The line flag is a per-editor table because there is no convention: the
/// `+N` family is the vi lineage, VS Code wants `--goto file:line`, Zed takes
/// `file:line` bare. An editor not in the table gets the file and no line,
/// which is right — a wrong flag would be passed through as a filename and
/// the user would be editing a file called `+12`.
#[must_use]
pub fn editor_command(git_editor: &str, path: &Path, line: Option<u32>) -> Command {
    let mut words = git_editor.split_whitespace();
    let program = words.next().unwrap_or(git_editor);
    let mut command = Command::new(program);
    command.args(words);

    match (line, line_style(program)) {
        (Some(line), LineStyle::Plus) => {
            command.arg(format!("+{line}"));
            command.arg(path);
        }
        (Some(line), LineStyle::Goto) => {
            command.arg("--goto");
            command.arg(suffixed(path, line));
        }
        (Some(line), LineStyle::Suffix) => {
            command.arg(suffixed(path, line));
        }
        (Some(_), LineStyle::None) | (None, _) => {
            command.arg(path);
        }
    }
    command
}

/// `path:line`, as VS Code and Zed want it. Lossy on a path that is not
/// UTF-8, which is unavoidable: the argument is one string with a `:` in it,
/// so there is nowhere for the raw bytes to go. A repository path that is
/// not UTF-8 therefore opens at the top of the file rather than not at all.
fn suffixed(path: &Path, line: u32) -> String {
    format!("{}:{line}", path.to_string_lossy())
}

/// How an editor is told which line to open at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineStyle {
    /// `editor +12 path` — the vi lineage, and most of the rest.
    Plus,
    /// `editor --goto path:12` — VS Code and its forks.
    Goto,
    /// `editor path:12` — Zed.
    Suffix,
    /// This editor is not in the table: open the file, say nothing about the
    /// line.
    None,
}

/// The table. Matched on the program's file name, so `/usr/bin/vim` and
/// `vim` are the same editor, and `vim.exe` too.
fn line_style(program: &str) -> LineStyle {
    let name = Path::new(program)
        .file_stem()
        .map_or(program, |stem| stem.to_str().unwrap_or(program));
    match name {
        "vim" | "nvim" | "vi" | "nano" | "emacs" | "emacsclient" | "micro" | "hx" | "helix" => {
            LineStyle::Plus
        }
        "code" | "code-insiders" | "codium" | "cursor" | "windsurf" => LineStyle::Goto,
        "zed" | "zeditor" => LineStyle::Suffix,
        _ => LineStyle::None,
    }
}

/// Stop Graft the way `Ctrl+Z` stops any other terminal program, by sending
/// this process `SIGTSTP`.
///
/// Through `kill(1)` rather than `libc::raise`: neither `libc` nor `nix` is a
/// dependency of this crate, and adding one for a single signal — on a path
/// the user has to turn on in config — costs more than the spawn does
/// (the spec says so too).
///
/// The shell puts Graft back in the foreground with `fg`, and the call
/// returns then; the terminal has been restored around it by
/// [`run_outside_terminal`], so `fg` lands the user back in a redrawn Graft.
///
/// # Errors
/// `kill` could not be run, or refused.
pub fn stop_this_process() -> io::Result<()> {
    let status = Command::new("kill")
        .args(["-TSTP", &std::process::id().to_string()])
        .status()?;
    if status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!("kill -TSTP: {status}")))
}

#[cfg(test)]
mod tests;
