//! The editor table is pure and tested as such; the handover is tested over
//! a scripted terminal session and a real [`InputMux`] on a socket pair, so
//! what it proves is the sequence and the guard, not that a function was
//! called.
//!
//! What no test here can prove is the real terminal: whether a real `$EDITOR`
//! comes up clean and leaves Mergewright's screen intact is the acceptance walk's
//! pty check and the user's own eye.

use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use super::*;
use crate::terminal::SessionOptions;
use crate::terminal::tests::{KITTY, Transcript, raw_mode};
use mergewright_core::on_disk::CURRENT_DIR;

/// `Ctrl+E`'s message file sits directly in the app's own dir — whichever
/// names are in force decided that dir (ADR-0057, Amendment §4) — and what
/// the editor left is read back exactly.
#[test]
fn the_message_file_lives_in_the_app_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app_dir = dir.path().join(CURRENT_DIR);
    let back = edit_message_in(&app_dir, "subject\n", |file| {
        assert_eq!(file, app_dir.join("COMMIT_EDITMSG"));
        std::fs::write(file, "edited\n").map_err(|error| error.to_string())
    })
    .expect("the round trip runs");
    assert_eq!(back, "edited\n");
}

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

/// With no working tree — the path stays as it is given, so the existing
/// table assertions read the way they always did.
fn opened(editor: &str, line: Option<u32>) -> Vec<String> {
    spelled(&editor_command(
        editor,
        &PathBuf::from("src/main.rs"),
        line,
        None,
    ))
}

/// And with one, which is every real run.
fn opened_in(editor: &str, line: Option<u32>, workdir: &str) -> Command {
    editor_command(
        editor,
        &PathBuf::from("src/main.rs"),
        line,
        Some(Path::new(workdir)),
    )
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
        mergewright_core::event::Event::Key(mergewright_core::event::KeyPress::plain(
            mergewright_core::event::KeyCode::Char('a')
        )),
    );
}

/// Major 2 of the m3/05 review. `graft ~/code/api` run from `~` used to open
/// `~/src/main.rs`: an empty new buffer in vim, "file not found" in code and
/// zed, and a stray file the moment the user saved.
#[test]
fn a_porcelain_path_is_joined_onto_the_worktree_not_graft_s_own_directory() {
    let command = opened_in("vim", Some(12), "/code/api");

    assert_eq!(spelled(&command), ["vim", "+12", "/code/api/src/main.rs"]);
    assert_eq!(
        command.get_current_dir(),
        Some(Path::new("/code/api")),
        "and the editor starts where git would start one"
    );
}

#[test]
fn an_absolute_path_is_left_alone() {
    let command = editor_command(
        "vim",
        Path::new("/tmp/graft/COMMIT_EDITMSG"),
        None,
        Some(Path::new("/code/api")),
    );

    assert_eq!(spelled(&command), ["vim", "/tmp/graft/COMMIT_EDITMSG"]);
}

/// Major 5. The common macOS `core.editor`: a quoted path with a space in
/// it. Split on whitespace it became "could not run `'/Applications/Sublime`".
#[test]
fn a_quoted_editor_path_is_run_by_a_shell_the_way_git_runs_it() {
    let editor = "'/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl' -w";
    let command = opened_in(editor, None, "/repo");

    assert_eq!(
        spelled(&command),
        [
            "/bin/sh",
            "-c",
            &format!("{editor} \"$@\""),
            editor,
            "/repo/src/main.rs",
        ],
        "the words the user wrote reach the shell whole"
    );
}

/// A word list with no shell syntax in it is still run directly — a shell
/// per keystroke for `code --wait` would buy nothing.
#[test]
fn a_plain_word_list_is_not_handed_to_a_shell() {
    assert_eq!(
        spelled(&opened_in("code --wait", Some(9), "/repo")),
        ["code", "--wait", "--goto", "/repo/src/main.rs:9"]
    );
    assert_eq!(
        spelled(&opened_in("emacs -nw", None, "/repo")),
        ["emacs", "-nw", "/repo/src/main.rs"]
    );
}

/// The line style still comes off the first word, shell or no shell.
#[test]
fn the_shell_form_keeps_its_editors_line_style() {
    let command = opened_in("code --wait --new-window $EXTRA", Some(3), "/repo");
    let words = spelled(&command);

    assert_eq!(words[0], "/bin/sh");
    assert!(
        words.contains(&"--goto".to_owned()) && words.contains(&"/repo/src/main.rs:3".to_owned()),
        "{words:?}"
    );
}

/// A repository in `workdir` stopped on a **real** merge conflict in
/// `a.txt`, with `merge.tool` set to `tool` running `cmd`.
///
/// The conflict is not decoration: `git mergetool` on a path that is not
/// unmerged prints "No files need merging" and exits 0, so without one the
/// failure these tests are about never happens and they pass for the wrong
/// reason.
///
/// `trustExitCode` so git reports the tool's own status rather than asking
/// on stdin whether the merge went well — which is the question
/// `--no-prompt` exists to avoid.
fn repo_stopped_on_a_conflict(workdir: &Path, tool: &str, cmd: &str) {
    use mergewright_fixtures::git::run;
    let git =
        |args: &[&str]| run(workdir, args).unwrap_or_else(|error| panic!("git {args:?}: {error}"));
    git(&["init", "--quiet", "--initial-branch", "main"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "merge.tool", tool]);
    git(&["config", &format!("mergetool.{tool}.cmd"), cmd]);
    git(&["config", &format!("mergetool.{tool}.trustExitCode"), "true"]);

    std::fs::write(workdir.join("a.txt"), "base\n").expect("the base file");
    git(&["add", "a.txt"]);
    git(&["commit", "--quiet", "-m", "base"]);

    git(&["checkout", "--quiet", "-b", "other"]);
    std::fs::write(workdir.join("a.txt"), "theirs\n").expect("their edit");
    git(&["commit", "--quiet", "-am", "theirs"]);

    git(&["checkout", "--quiet", "main"]);
    std::fs::write(workdir.join("a.txt"), "ours\n").expect("our edit");
    git(&["commit", "--quiet", "-am", "ours"]);

    // Expected to fail: that is the conflict.
    let merged = run(workdir, &["merge", "other"]);
    assert!(merged.is_err(), "the merge was supposed to conflict");
    assert!(
        workdir.join(".git/MERGE_HEAD").exists(),
        "git is standing in the merge"
    );
}

/// What `run_outside_terminal` writes on the way out and the way back in,
/// for a transcript that started at `after_setup`.
///
/// The assertion every restoration test below makes: the bytes after setup
/// are *teardown followed by the same setup again*, byte for byte. That is a
/// statement about the terminal's modes, not about which functions were
/// called — a `mergetool` that skipped the handover writes nothing here and
/// fails this, and one that handed over but never came back ends on the
/// teardown and fails it too.
fn assert_terminal_was_given_up_and_taken_back(transcript: &Transcript, after_setup: &str) {
    let rendered = transcript.rendered();
    let moved = rendered
        .strip_prefix(after_setup)
        .unwrap_or_else(|| panic!("the transcript does not start with setup:\n{rendered}"));
    assert!(
        !moved.is_empty(),
        "the terminal was never handed over at all: the tool ran over Mergewright's own screen, \
         in raw mode, on the alternate screen"
    );
    assert!(
        moved.contains("<ESC>[?1049l"),
        "never left the alternate screen: {moved}"
    );
    assert!(
        moved.ends_with(after_setup),
        "the terminal was not put back the way Mergewright found it — this is the failure that \
         outlives the process:\n{moved}"
    );
}

/// The failure that matters most: `git mergetool` exits non-zero, and the
/// user's terminal is still restored and re-entered.
///
/// `merge.tool` is set to a tool that exits 1, so git itself fails. What is
/// asserted is the transcript — every mode off, then every mode back on, in
/// order — not that a restore function was called.
#[test]
fn a_mergetool_that_exits_non_zero_still_puts_the_terminal_back() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let workdir = scratch.path();
    // The tool exits 1 on a file that really is unmerged, so git exits
    // non-zero too.
    repo_stopped_on_a_conflict(workdir, "failing", "false");

    let transcript = Transcript::default();
    let session =
        TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).expect("a session");
    let (_terminal, mux) = mux();
    let after_setup = transcript.rendered();

    let handover = Handover {
        session: &session,
        mux: &mux,
        git_dir: None,
        app_dir: None,
        workdir: Some(workdir),
    };
    let failed = handover
        .mergetool(Path::new("a.txt"))
        .expect_err("the tool exits non-zero");
    assert!(
        failed.contains("a.txt"),
        "the line names the file: {failed}"
    );

    assert_terminal_was_given_up_and_taken_back(&transcript, &after_setup);
}

/// The other failure shape: `git` cannot run the tool at all (here, a
/// `merge.tool` naming a program that does not exist). Same guarantee.
#[test]
fn a_mergetool_that_cannot_start_still_puts_the_terminal_back() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let workdir = scratch.path();
    repo_stopped_on_a_conflict(workdir, "nosuchtool", "graft-no-such-program-exists");

    let transcript = Transcript::default();
    let session =
        TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).expect("a session");
    let (_terminal, mux) = mux();
    let after_setup = transcript.rendered();

    let handover = Handover {
        session: &session,
        mux: &mux,
        git_dir: None,
        app_dir: None,
        workdir: Some(workdir),
    };
    let failed = handover
        .mergetool(Path::new("a.txt"))
        .expect_err("there is no such tool");
    assert!(!failed.is_empty(), "the reason is never empty");

    assert_terminal_was_given_up_and_taken_back(&transcript, &after_setup);
}

/// Nothing is left paused after a failed mergetool either: a mux still
/// parked would take no more keys, which looks exactly like a hang — and it
/// would happen precisely when the user most needs to press `a`.
#[test]
fn the_reader_is_reading_again_after_a_failed_mergetool() {
    use std::io::Write;

    let scratch = tempfile::tempdir().expect("a scratch directory");
    let workdir = scratch.path();
    repo_stopped_on_a_conflict(workdir, "failing", "false");

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

    let handover = Handover {
        session: &session,
        mux: &mux,
        git_dir: None,
        app_dir: None,
        workdir: Some(workdir),
    };
    let _failed = handover.mergetool(Path::new("a.txt"));

    terminal.write_all(b"a").expect("a key after the mergetool");
    let event = inputs
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the reader is back on the terminal")
        .expect("a key, not a failure");
    assert_eq!(
        event,
        mergewright_core::event::Event::Key(mergewright_core::event::KeyPress::plain(
            mergewright_core::event::KeyCode::Char('a')
        )),
    );
}

/// A bare repository has no working tree to run the tool in, and says so
/// rather than handing the terminal to a `git mergetool` with nowhere to go.
#[test]
fn a_mergetool_without_a_working_tree_refuses_before_handing_over() {
    let transcript = Transcript::default();
    let session =
        TerminalSession::new(transcript.clone(), raw_mode(&transcript), KITTY).expect("a session");
    let (_terminal, mux) = mux();
    let after_setup = transcript.rendered();

    let handover = Handover {
        session: &session,
        mux: &mux,
        git_dir: None,
        app_dir: None,
        workdir: None,
    };
    let failed = handover
        .mergetool(Path::new("a.txt"))
        .expect_err("no working tree");
    assert!(failed.contains("working tree"), "{failed}");
    assert_eq!(
        transcript.rendered(),
        after_setup,
        "nothing was handed over, so nothing was written to the terminal"
    );
}

/// The command git is actually asked to run.
#[test]
fn the_mergetool_command_names_the_file_after_a_double_dash() {
    let command = mergetool_command(Path::new("-weird.rs"), Path::new("/tmp/repo"));
    assert_eq!(
        spelled(&command),
        ["git", "mergetool", "--no-prompt", "--", "-weird.rs"],
        "`--` keeps a file called `-weird.rs` a file and not a flag"
    );
    assert_eq!(
        command.get_current_dir(),
        Some(Path::new("/tmp/repo")),
        "run at the top of the working tree, where git's porcelain paths are relative to"
    );
}
