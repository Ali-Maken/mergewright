//! The editor table is pure and tested as such; the handover is tested over
//! a scripted terminal session and a real [`InputMux`] on a socket pair, so
//! what it proves is the sequence and the guard, not that a function was
//! called.
//!
//! What no test here can prove is the real terminal: whether a real `$EDITOR`
//! comes up clean and leaves Graft's screen intact is the acceptance walk's
//! pty check and the user's own eye.

use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use super::*;
use crate::terminal::SessionOptions;
use crate::terminal::tests::{KITTY, Transcript, raw_mode};

/// The command's program and arguments as plain strings, which is all these
/// tests care about.
fn spelled(command: &Command) -> Vec<String> {
    let mut words = vec![command.get_program().to_string_lossy().into_owned()];
    words.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );
    words
}

fn opened(editor: &str, line: Option<u32>) -> Vec<String> {
    spelled(&editor_command(editor, &PathBuf::from("src/main.rs"), line))
}

#[test]
fn the_vi_lineage_takes_a_plus_line() {
    assert_eq!(opened("vim", Some(12)), ["vim", "+12", "src/main.rs"]);
    assert_eq!(opened("nvim", Some(1)), ["nvim", "+1", "src/main.rs"]);
    assert_eq!(opened("nano", Some(7)), ["nano", "+7", "src/main.rs"]);
    assert_eq!(opened("hx", Some(7)), ["hx", "+7", "src/main.rs"]);
}

#[test]
fn vs_code_wants_goto_and_zed_wants_a_suffix() {
    assert_eq!(
        opened("code", Some(12)),
        ["code", "--goto", "src/main.rs:12"]
    );
    assert_eq!(opened("zed", Some(12)), ["zed", "src/main.rs:12"]);
}

/// The important half of the table: guessing wrong would have the user
/// editing a file called `+12`.
#[test]
fn an_editor_nobody_knows_gets_the_file_and_nothing_else() {
    assert_eq!(
        opened("unknown-editor", Some(12)),
        ["unknown-editor", "src/main.rs"]
    );
}

#[test]
fn no_line_asked_for_means_no_line_argument() {
    assert_eq!(opened("vim", None), ["vim", "src/main.rs"]);
    assert_eq!(opened("code", None), ["code", "src/main.rs"]);
}

/// `git var GIT_EDITOR` answers with a command line, not a program name.
#[test]
fn an_editor_with_arguments_keeps_them_in_front() {
    assert_eq!(
        opened("emacs -nw", Some(12)),
        ["emacs", "-nw", "+12", "src/main.rs"]
    );
    assert_eq!(
        opened("code --wait", Some(12)),
        ["code", "--wait", "--goto", "src/main.rs:12"]
    );
}

/// A path is the same editor whether it is spelled in full or not.
#[test]
fn the_table_matches_the_programs_name_not_its_path() {
    assert_eq!(
        opened("/opt/homebrew/bin/nvim", Some(3)),
        ["/opt/homebrew/bin/nvim", "+3", "src/main.rs"]
    );
}

/// A mux on a socket pair, and the far end to type into.
fn mux() -> (UnixStream, InputMux) {
    let (terminal, ours) = UnixStream::pair().expect("a socket pair");
    let (mux, _inputs) = InputMux::start(OwnedFd::from(ours)).expect("a reader");
    (terminal, mux)
}

/// The whole contract in one transcript: out of the alternate screen, `f`,
/// back in — with the modes in the right order on both sides.
#[test]
fn the_terminal_is_given_up_before_f_runs_and_taken_back_after() {
    let transcript = Transcript::default();
    let session =
        TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).expect("a session");
    let (_terminal, mux) = mux();
    let after_setup = transcript.rendered();

    let ran = run_outside_terminal(&session, &mux, || {
        // What the editor sees: every mode already off.
        transcript
            .rendered()
            .strip_prefix(&after_setup)
            .map(str::to_owned)
            .expect("only teardown so far")
    })
    .expect("the handover works");

    assert!(ran.ends_with("<RAW OFF>"), "{ran}");
    assert!(
        ran.contains("<ESC>[?1049l"),
        "left the alternate screen: {ran}"
    );

    let after = transcript
        .rendered()
        .strip_prefix(&format!("{after_setup}{ran}"))
        .map(str::to_owned)
        .expect("setup, teardown, then setup again");
    assert_eq!(after, after_setup, "the same modes come back, in order");
}

/// The reason the way back is a guard and not a straight line: a panicking
/// editor path must not leave the user in a terminal with no cursor and no
/// echo. The panic still propagates — it is not swallowed.
#[test]
fn a_panic_inside_f_still_re_enters_the_terminal() {
    let transcript = Transcript::default();
    let session =
        TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).expect("a session");
    let (_terminal, mux) = mux();
    let after_setup = transcript.rendered();

    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_outside_terminal(&session, &mux, || panic!("the editor blew up")).map(drop)
    }));
    assert!(panicked.is_err(), "the panic reaches the caller");

    let rendered = transcript.rendered();
    let after_teardown = rendered
        .strip_prefix(&after_setup)
        .expect("teardown came first");
    assert!(
        after_teardown.ends_with(after_setup.as_str()),
        "the terminal was put back before the unwind: {after_teardown}",
    );
}

/// Nothing is left paused: a mux that stayed paused would take no more keys
/// for the rest of the session, which looks exactly like a hang.
#[test]
fn the_input_reader_is_reading_again_afterwards() {
    use std::io::Write;

    let transcript = Transcript::default();
    let session = TerminalSession::new(
        transcript.clone(),
        raw_mode(&transcript),
        SessionOptions::default(),
    )
    .expect("a session");
    let (terminal, ours) = UnixStream::pair().expect("a socket pair");
    let (mux, inputs) = InputMux::start(OwnedFd::from(ours)).expect("a reader");
    let mut terminal = terminal;

    run_outside_terminal(&session, &mux, || ()).expect("the handover works");

    terminal.write_all(b"a").expect("a key after the editor");
    let event = inputs
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the reader is back on the terminal")
        .expect("a key, not a failure");
    assert_eq!(
        event,
        graft_core::event::Event::Key(graft_core::event::KeyPress::plain(
            graft_core::event::KeyCode::Char('a')
        )),
    );
}
