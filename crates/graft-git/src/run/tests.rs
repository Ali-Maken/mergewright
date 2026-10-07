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
        format!("GRAFT_ASKPASS_SOCKET={socket}"),
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

/// m8-polish-and-ship/13c: the process group, and how a stop ends git's hook.
mod stop {
    use std::path::PathBuf;
    use std::time::Instant;

    use nix::sys::signal::kill as signal;
    use nix::unistd::{Pid, getpgid, getpgrp};

    use super::*;
    use crate::run::exec::{self, Graces, StopOrder};
    use crate::run::group;

    /// `main` and `feature` each one commit past a shared base, his
    /// unrelated `notes.txt` left dirty, and `.git/hooks/<hook>` running
    /// `body` (which may use `$PIDFILE`).
    struct Merging {
        repo: Fixture,
        source: String,
        pidfile: PathBuf,
        notes: Vec<u8>,
    }

    fn merging(hook: &str, body: &str) -> Merging {
        let repo = Fixture::empty_repo();
        let write = |name: &str, text: &str| {
            std::fs::write(repo.root().join(name), text).expect("a fixture file writes");
        };
        write("a.txt", "base\n");
        write("notes.txt", "notes\n");
        repo.git(&["add", "."]);
        repo.git(&["commit", "-m", "base"]);
        repo.git(&["checkout", "-b", "feature"]);
        write("b.txt", "feature\n");
        repo.git(&["add", "b.txt"]);
        repo.git(&["commit", "-m", "feature"]);
        repo.git(&["checkout", "main"]);
        write("c.txt", "main\n");
        repo.git(&["add", "c.txt"]);
        repo.git(&["commit", "-m", "main side"]);
        write("notes.txt", "notes\nhis edit\n");
        let script = repo.root().join(".git/hooks").join(hook);
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).expect("the hook writes");
        let mut mode = std::fs::metadata(&script)
            .expect("hook metadata")
            .permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        std::fs::set_permissions(&script, mode).expect("the hook is executable");
        let source = repo.rev_parse("feature");
        let pidfile = repo.root().join(".git/hook-pid");
        let notes = std::fs::read(repo.root().join("notes.txt")).expect("notes reads");
        Merging {
            repo,
            source,
            pidfile,
            notes,
        }
    }

    /// `git merge --no-ff feature`, hermetic: no global or system config
    /// (a user's `core.hooksPath` or `commit.gpgsign` would change the run).
    fn merge(m: &Merging) -> GitInvocation {
        GitInvocation::in_directory(m.repo.root())
            .args([
                GitArg::Flag("merge"),
                GitArg::Flag("--no-ff"),
                GitArg::Flag("feature"),
            ])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Graft Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Graft Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .env("PIDFILE", &m.pidfile)
    }

    /// The pid the hook wrote, once it has.
    fn hook_pid(pidfile: &Path) -> Pid {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(text) = std::fs::read_to_string(pidfile)
                && let Ok(pid) = text.trim().parse::<i32>()
            {
                return Pid::from_raw(pid);
            }
            assert!(Instant::now() < deadline, "the hook never wrote its pid");
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// Runs `invocation` on a thread, stops it once the hook has written its
    /// pid, and says what `run` returned, how long the stop took, and the
    /// hook's pid and process group.
    fn stop_at_the_hook(
        invocation: GitInvocation,
        pidfile: &Path,
    ) -> (Result<GitOutput, RunError>, Duration, Pid, Pid) {
        let stop = Arc::new(AtomicBool::new(false));
        let asked = Arc::clone(&stop);
        let running = thread::spawn(move || invocation.run(&*asked));
        let hook = hook_pid(pidfile);
        let group = getpgid(Some(hook)).expect("the hook is alive");
        let at = Instant::now();
        stop.store(true, Ordering::Relaxed);
        let result = running.join().expect("the run thread ends");
        (result, at.elapsed(), hook, group)
    }

    /// `kill -0` fails: the process is gone. Polled briefly, because a hook
    /// orphaned by git's end is reaped by init a moment later, and a zombie
    /// still answers `kill -0`.
    fn gone(pid: Pid) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while signal(pid, None).is_ok() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(5));
        }
        true
    }

    /// No member of `group` is left (polled, as [`gone`]).
    fn group_empty(group: Pid) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let left = group::members(group).expect("the table reads");
            if left.is_empty() {
                return true;
            }
            if Instant::now() >= deadline {
                eprintln!("left in group {group}: {left:?}");
                return false;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn locks_under(dir: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(dir).expect("a readable directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                found.extend(locks_under(&path));
            } else if path.extension().is_some_and(|ext| ext == "lock") {
                found.push(path);
            }
        }
        found
    }

    fn assert_paused(m: &Merging) {
        let git_dir = m.repo.root().join(".git");
        let merge_head = std::fs::read_to_string(git_dir.join("MERGE_HEAD"))
            .expect("git paused the merge: MERGE_HEAD is there");
        assert_eq!(merge_head.trim(), m.source, "MERGE_HEAD is the source tip");
        assert!(git_dir.join("MERGE_MSG").exists(), "MERGE_MSG is there");
        assert_eq!(locks_under(&git_dir), Vec::<PathBuf>::new(), "no lock left");
        assert_eq!(
            std::fs::read(m.repo.root().join("notes.txt")).expect("notes reads"),
            m.notes,
            "his dirty file is byte-equal"
        );
    }

    #[test]
    fn git_runs_in_its_own_process_group() {
        let repo = Fixture::with_two_commits();
        let invocation = GitInvocation::in_directory(repo.root())
            .args([GitArg::Flag("cat-file"), GitArg::Flag("--batch")])
            .stdin(Vec::new());
        let mut child = invocation.spawn().expect("git starts");
        // stdin stays open, so git waits on it and is certainly alive.
        let pid = Pid::from_raw(i32::try_from(child.id()).expect("a pid fits"));
        let group = getpgid(Some(pid));
        drop(child.stdin.take());
        child.wait().expect("git ends on EOF");
        let group = group.expect("git's group reads");
        assert_eq!(group, pid, "git leads its own group");
        assert_ne!(group, getpgrp(), "and it is not ours");
    }

    #[test]
    fn a_stop_leaves_no_process_of_the_group() {
        let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
        let (result, took, hook, group) = stop_at_the_hook(merge(&m), &m.pidfile);
        assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
        assert!(took <= STOP_WITHIN, "stopped in {took:?}");
        assert!(gone(hook), "the hook is gone");
        assert!(group_empty(group), "the hook's sleep is gone too");
    }

    #[test]
    fn hooks_first_lets_git_pause_a_merge() {
        let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
        let (result, took, hook, group) =
            stop_at_the_hook(merge(&m).stop_hooks_first(), &m.pidfile);
        assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
        assert!(took <= STOP_WITHIN, "stopped in {took:?}");
        assert!(
            gone(hook) && group_empty(group),
            "nothing of the group left"
        );
        assert_paused(&m);

        // git itself was never signalled: the stop `wait` calls reports
        // git's own exit, 1 ("Not committing merge").
        let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
        let mut child = merge(&m).spawn().expect("git starts");
        hook_pid(&m.pidfile);
        let end = exec::stop(
            &mut child,
            StopOrder::HooksFirst,
            &group::members,
            Graces::REAL,
        )
        .expect("stopped and reaped");
        assert!(!end.git_signalled, "{end:?}");
        assert_eq!(end.status.code(), Some(1), "git's own exit: {end:?}");
        assert_paused(&m);
    }

    #[test]
    fn hooks_first_kills_a_hook_that_ignores_term() {
        let m = merging(
            "pre-merge-commit",
            r#"trap '' TERM; echo $$ > "$PIDFILE"; sleep 600"#,
        );
        let (result, took, hook, group) =
            stop_at_the_hook(merge(&m).stop_hooks_first(), &m.pidfile);
        assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
        assert!(took <= STOP_WITHIN, "stopped in {took:?}");
        assert!(
            took >= HELPER_TERM_GRACE_FOR_TESTS,
            "the hook was given its grace first: {took:?}"
        );
        assert!(
            gone(hook) && group_empty(group),
            "nothing of the group left"
        );
        assert_paused(&m);
    }

    const HELPER_TERM_GRACE_FOR_TESTS: Duration = exec::HELPER_TERM_GRACE;

    #[test]
    fn a_stopped_hook_is_continued_and_ended() {
        let m = merging(
            "pre-merge-commit",
            r#"echo $$ > "$PIDFILE"; kill -STOP $$; sleep 600"#,
        );
        let stop = Arc::new(AtomicBool::new(false));
        let asked = Arc::clone(&stop);
        let invocation = merge(&m).stop_hooks_first();
        let running = thread::spawn(move || invocation.run(&*asked));
        let hook = hook_pid(&m.pidfile);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !is_stopped(hook) {
            assert!(Instant::now() < deadline, "the hook never stopped itself");
            thread::sleep(Duration::from_millis(5));
        }
        stop.store(true, Ordering::Relaxed);
        let at = Instant::now();
        let result = running.join().expect("the run thread ends");
        assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
        assert!(at.elapsed() <= STOP_WITHIN, "stopped in {:?}", at.elapsed());
        assert!(gone(hook), "the stopped hook was continued and ended");
        assert_paused(&m);
    }

    /// `ps -o stat=` starts with `T`: stopped by a signal.
    fn is_stopped(pid: Pid) -> bool {
        let stat = std::process::Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .expect("ps runs");
        String::from_utf8_lossy(&stat.stdout)
            .trim()
            .starts_with('T')
    }

    #[test]
    fn hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed() {
        let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
        let mut child = merge(&m).spawn().expect("git starts");
        let hook = hook_pid(&m.pidfile);
        let group = getpgid(Some(hook)).expect("the hook is alive");
        let unreadable = |_: Pid| -> io::Result<Vec<Pid>> { Err(io::Error::other("injected")) };
        let end = exec::stop(&mut child, StopOrder::HooksFirst, &unreadable, Graces::REAL)
            .expect("stopped and reaped");
        assert!(end.git_signalled, "the group stop signalled git: {end:?}");
        assert!(
            gone(hook) && group_empty(group),
            "nothing of the group left"
        );
    }

    #[test]
    fn an_unstopped_run_leaves_a_hooks_background_job_alone() {
        let m = merging(
            "post-merge",
            r#"sleep 600 </dev/null >/dev/null 2>&1 & echo $! > "$PIDFILE""#,
        );
        merge(&m).run(&()).expect("the merge lands");
        let job = hook_pid(&m.pidfile);
        let alive = signal(job, None).is_ok();
        // Our own test started it; end it before asserting.
        signal(job, nix::sys::signal::Signal::SIGKILL).expect("our own job can be ended");
        assert!(alive, "the hook's background job was left running");
    }
}
