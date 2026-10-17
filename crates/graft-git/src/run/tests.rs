//! `run`'s own tests. Moved out of `run.rs` when 13c added the stop tests.

use std::sync::atomic::AtomicUsize;

use graft_fixtures::Shape;

use super::*;
use crate::fixture::Fixture;

#[test]
fn a_stopped_command_is_killed_and_reaped() {
    let invocation = GitInvocation::in_directory(&graft_fixtures::borrow(Shape::Linear)).args([
        GitArg::Flag("log"),
        GitArg::Flag("-p"),
        GitArg::Flag("--all"),
        GitArg::Flag("--no-color"),
    ]);
    let mut child = invocation.spawn().expect("git starts");
    // Stop on the very first poll, microseconds after the spawn: git has
    // not even finished starting, so this is a kill, not an exit a stop
    // arrived after. A later poll would race a small repository's `log`
    // to its end.
    let result = invocation.supervise(&mut child, &AtomicBool::new(true));
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(
        child.try_wait().expect("try_wait").is_some(),
        "the child has exited and been reaped"
    );
}

/// A bare remote holding `commits` commits of one new file each (three
/// objects apiece), and an empty repository to fetch it into.
fn remote_with_objects(commits: usize) -> (Fixture, Fixture) {
    let work = Fixture::empty_repo();
    for n in 0..commits {
        std::fs::write(work.root().join(format!("f{n}")), format!("{n}\n"))
            .expect("a fixture file writes");
        work.git(&["add", "."]);
        work.git(&["commit", "--quiet", "-m", &format!("c{n}")]);
    }
    let bare = Fixture::bare_repo();
    let bare_path = bare.root().to_str().expect("UTF-8 fixture path");
    work.git(&["push", "--quiet", bare_path, "main"]);
    (bare, Fixture::empty_repo())
}

#[test]
fn stderr_sink_sees_progress_and_capture_is_whole() {
    let (bare, into) = remote_with_objects(20);
    let seen = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let sink_seen = Arc::clone(&seen);
    let output = GitInvocation::in_directory(into.root())
        .args([
            GitArg::Flag("fetch"),
            GitArg::Flag("--progress"),
            GitArg::Word(bare.root().to_str().expect("UTF-8").to_owned()),
            GitArg::Flag("main"),
        ])
        .on_stderr_line(Box::new(move |segment| {
            sink_seen
                .lock()
                .expect("the test's own lock")
                .push(segment.to_vec());
        }))
        .run(&())
        .expect("the fetch succeeds");

    let seen = seen.lock().expect("the test's own lock").clone();
    assert!(
        seen.iter()
            .any(|segment| crate::remote::parse_progress(segment).is_some()),
        "at least one progress segment: {seen:?}"
    );
    let from_capture: Vec<&[u8]> = output
        .stderr
        .split(|&b| b == b'\r' || b == b'\n')
        .filter(|segment| !segment.is_empty())
        .collect();
    assert_eq!(
        from_capture,
        seen.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        "the sink saw exactly the captured stderr, in order, minus separators"
    );
}

/// `KNOWN_GAPS` row 455: a stderr sink poisoned by an earlier run of the
/// same (cloned) invocation must surface as `RunError::Sink`, not
/// `RunError::Spawn` — the latter reads as "git could not be started",
/// which is not what happened.
#[test]
fn poisoned_sink_is_not_spawn() {
    let repo = Fixture::with_two_commits();
    let invocation = GitInvocation::in_directory(repo.root())
        .args([
            GitArg::Flag("checkout"),
            GitArg::Word("nonesuch-branch".to_owned()),
        ])
        .on_stderr_line(Box::new(|_| panic!("the sink panics on purpose")));

    // git writes to stderr and exits non-zero; the sink panics on the
    // first segment it is handed, poisoning its mutex. However this
    // first run fails is not the point under test.
    let first = invocation.run(&());
    assert!(
        first.is_err(),
        "a panicking sink cannot let this run succeed"
    );

    // Same (cloned) invocation, same poisoned sink, a second run: the
    // failure now is the sink refusing to run again, not a fresh spawn
    // failure — `git` starts and runs exactly as before.
    let second = invocation.run(&());
    assert!(matches!(second, Err(RunError::Sink(_))), "{second:?}");
}

/// Counts the asks; never expects one.
struct Counting(AtomicUsize);

impl askpass::AskPass for Counting {
    fn ask(&self, _prompt: &str) -> Option<askpass::Secret> {
        self.0.fetch_add(1, Ordering::SeqCst);
        None
    }
}

fn askpass_server() -> (Arc<Counting>, askpass::AskPassServer) {
    let handler = Arc::new(Counting(AtomicUsize::new(0)));
    let server = askpass::AskPassServer::spawn(
        std::path::PathBuf::from("/opt/graft helper"),
        handler.clone(),
    )
    .expect("the askpass server starts");
    (handler, server)
}

/// Read back from the child itself: a git alias running `env` shows the
/// environment git hands the programs it starts, askpass included.
#[test]
fn askpass_env_is_set() {
    let (_handler, server) = askpass_server();
    let repo = Fixture::empty_repo();
    let output = GitInvocation::in_directory(repo.root())
        .args([
            GitArg::Flag("-c"),
            GitArg::Flag("alias.showenv=!env"),
            GitArg::Flag("showenv"),
        ])
        .askpass(&server)
        .run(&())
        .expect("git runs the alias");
    let env = String::from_utf8(output.stdout).expect("UTF-8 env");
    let socket = server.socket_path().to_str().expect("UTF-8 socket path");
    for line in [
        "GIT_ASKPASS=/opt/graft helper".to_owned(),
        "SSH_ASKPASS=/opt/graft helper".to_owned(),
        "SSH_ASKPASS_REQUIRE=force".to_owned(),
        format!("{}={socket}", graft_core::on_disk::ASKPASS_SOCKET_VAR),
        "GIT_TERMINAL_PROMPT=0".to_owned(),
    ] {
        assert!(env.lines().any(|seen| seen == line), "{line} in:\n{env}");
    }
    server.close().expect("closes cleanly");
}

/// Answers every ask with one known password.
struct Answers(&'static [u8]);

impl askpass::AskPass for Answers {
    fn ask(&self, _prompt: &str) -> Option<askpass::Secret> {
        Some(askpass::Secret::new(self.0.to_vec()))
    }
}

/// After a real exchange over the socket, the password is nowhere the
/// runner puts things: not the invocation's `Debug`, not its command
/// line, not the environment git hands its children, not git's output,
/// not the server's `Debug`. It went to the helper's `out` and nowhere
/// else. (The full git → helper → app path is proved where the helper
/// lives, in the app's askpass mode.)
#[test]
fn the_password_never_reaches_the_invocation() {
    const PASSWORD: &str = "hunter2-do-not-leak";
    let server = askpass::AskPassServer::spawn(
        std::path::PathBuf::from("/opt/graft"),
        Arc::new(Answers(PASSWORD.as_bytes())),
    )
    .expect("the askpass server starts");
    let mut helper_out = Vec::new();
    askpass::respond(server.socket_path(), "Password:", &mut helper_out)
        .expect("the exchange works");
    assert_eq!(helper_out, format!("{PASSWORD}\n").into_bytes());

    let repo = Fixture::empty_repo();
    let invocation = GitInvocation::in_directory(repo.root())
        .args([
            GitArg::Flag("-c"),
            GitArg::Flag("alias.showenv=!env"),
            GitArg::Flag("showenv"),
        ])
        .askpass(&server)
        .on_stderr_line(Box::new(|_| {}));
    let output = invocation.run(&()).expect("git runs the alias");

    for (place, text) in [
        ("invocation Debug", format!("{invocation:?}")),
        ("command line", invocation.command_line()),
        (
            "child env",
            String::from_utf8_lossy(&output.stdout).into_owned(),
        ),
        (
            "git stderr",
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ),
        ("server Debug", format!("{server:?}")),
    ] {
        assert!(
            !text.contains(PASSWORD),
            "the password leaked into the {place}"
        );
    }
    server.close().expect("closes cleanly");
}

/// A remote that needs no credentials never reaches the helper: askpass
/// only adds a way to answer, it changes nothing else.
#[test]
fn askpass_leaves_a_credential_free_remote_alone() {
    let (handler, server) = askpass_server();
    let (bare, into) = remote_with_objects(1);
    let url = format!("file://{}", bare.root().to_str().expect("UTF-8"));
    let output = GitInvocation::in_directory(into.root())
        .args([GitArg::Flag("ls-remote"), GitArg::Word(url)])
        .askpass(&server)
        .run(&())
        .expect("ls-remote over file:// works");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("refs/heads/main"),
        "{output:?}"
    );
    assert_eq!(handler.0.load(Ordering::SeqCst), 0, "nothing was asked");
    server.close().expect("closes cleanly");
}

#[test]
fn command_line_quotes_what_a_shell_would_split() {
    let line = GitInvocation::in_directory(Path::new("."))
        .args([
            GitArg::Flag("log"),
            GitArg::Word("--format=%H %s".to_owned()),
            GitArg::Path(b"it's.txt".to_vec()),
        ])
        .command_line();
    assert_eq!(line, r"git log '--format=%H %s' -- 'it'\''s.txt'");
}

#[test]
fn quote_word_quotes_one_word_as_a_command_line_would() {
    assert_eq!(
        quote_word(b"/r/.git/mwr/commit-index"),
        "/r/.git/mwr/commit-index"
    );
    assert_eq!(quote_word(b"/my repo/it's"), r"'/my repo/it'\''s'");
    assert_eq!(quote_word(b""), "''");
}

/// m8-polish-and-ship/13c: the process group, and how a stop ends git's hook.
mod stop;
