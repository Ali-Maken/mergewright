//! The terminal closing under a running write (m8/13g): the real binary as
//! a pty **session leader**, the pty its controlling terminal, so closing
//! the master is a real hang-up — the kernel sends `SIGHUP` and every later
//! read of the terminal fails — not a `kill -HUP` alone.
//!
//! A hang-up is "Quit from outside" (keymap §2.2), the same as `SIGTERM`: the
//! write gets up to `EXIT_WRITE_WAIT` to land; past that, or on a second
//! signal, it is stopped hook first, so git pauses it. Never exit 101 (13d's
//! observation: ratatui's `Terminal::drop` `eprintln!`ed to the dead pty and
//! the failed print panicked), never git left running.
//!
//! Every pid a test learns (Graft's, git's, the hook's) is held by a
//! [`Leftovers`] guard that kills it on drop, by pid, never by pattern.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use graft::jobs::{EXIT_WRITE_WAIT, FORCED_WAIT};
use graft_core::journal::{EntryStatus, OperationKind};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::pty::{Winsize, openpty};

use super::stuck::{alive, entries, gone_within, locks, pid};
use crate::app_harness::{App, git};
use crate::pty::{Isolated, Pty, Screen};

/// How long a wait for the screen may take.
const PATIENCE: Duration = Duration::from_secs(20);

/// The status a hang-up leaves: the process dies of `SIGHUP`, as the
/// `SIGTERM` path dies of `SIGTERM` (`quit_waits::signal` asserts
/// `status.signal() == Some(15)`; `main` re-raises the signal).
const SIGHUP: i32 = 1;

/// His edit to a file the merge does not touch.
const HIS_EDIT: &str = "base, his edit\n";

/// Pids to kill when the test ends, whatever it did: a regression must not
/// leave `sleep 600` (or git, or Graft) behind.
#[derive(Default)]
struct Leftovers(Vec<u32>);

impl Leftovers {
    fn hold(&mut self, pid: u32) {
        self.0.push(pid);
    }
}

impl Drop for Leftovers {
    fn drop(&mut self) {
        for pid in &self.0 {
            if alive(*pid) {
                // A pid that ended between the check and the kill makes
                // `kill` fail, which is what we wanted; only a `kill` that
                // could not run at all is worth a word.
                if let Err(error) = Command::new("kill").args(["-9", &pid.to_string()]).status() {
                    eprintln!("could not kill leftover {pid}: {error}");
                }
            }
        }
    }
}

/// Where Graft's stderr goes.
enum Stderr {
    /// On the pty, as a shell runs it: dead once the terminal closes.
    Pty,
    /// A file, so a test can read the exit's lines after the hang-up.
    File(PathBuf),
}

/// `graft` as the session leader of a fresh pty.
struct Session {
    master: Option<File>,
    child: Child,
    output: Arc<Mutex<Vec<u8>>>,
    reading: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    size: (u16, u16),
}

impl Session {
    fn spawn(isolated: &Isolated, repo: &Path, size: (u16, u16), stderr: &Stderr) -> Self {
        std::fs::create_dir_all(isolated.config()).expect("config home");
        std::fs::create_dir_all(isolated.home()).expect("home");
        let pty = openpty(
            Some(&Winsize {
                ws_row: size.1,
                ws_col: size.0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .expect("a pty");
        let slave: OwnedFd = pty.slave;
        let master_fd = pty.master.as_raw_fd();
        let mut command = Command::new(env!("CARGO_BIN_EXE_graft"));
        command
            .arg(repo)
            .env("XDG_CONFIG_HOME", isolated.config())
            .env("HOME", isolated.home())
            .env("GIT_CEILING_DIRECTORIES", repo)
            .env("TERM", "xterm-256color")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("NO_COLOR")
            .env_remove("GRAFT_TRACE")
            .stdin(Stdio::from(slave.try_clone().expect("slave fd")))
            .stdout(Stdio::from(slave.try_clone().expect("slave fd")));
        match stderr {
            Stderr::Pty => command.stderr(Stdio::from(slave)),
            Stderr::File(path) => {
                command.stderr(Stdio::from(File::create(path).expect("a stderr file")))
            }
        };
        session_leader(&mut command, master_fd);
        let child = command.spawn().expect("graft starts");
        let master = File::from(pty.master);
        let (output, reading, reader) = read_into_memory(&master);
        Self {
            master: Some(master),
            child,
            output,
            reading,
            reader: Some(reader),
            size,
        }
    }
}

/// Start `command` in a new session with its stdin, the pty's slave, as its
/// controlling terminal — as a shell in a terminal window runs it, so
/// closing `master_fd` is the window closing.
fn session_leader(command: &mut Command, master_fd: std::os::fd::RawFd) {
    #[allow(
        unsafe_code,
        reason = "a controlling terminal is only set from pre_exec: close, setsid and \
                  TIOCSCTTY, each async-signal-safe, touching only the child"
    )]
    // SAFETY: the closure makes three async-signal-safe syscalls on the
    // child's own descriptors and allocates nothing.
    unsafe {
        command.pre_exec(move || {
            // The master is not close-on-exec: left open in Graft, it would
            // keep the terminal alive after the test closes it.
            if nix::libc::close(master_fd) < 0
                || nix::libc::setsid() < 0
                || nix::libc::ioctl(0, nix::libc::TIOCSCTTY.into(), 0) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

type Reader = (Arc<Mutex<Vec<u8>>>, Arc<AtomicBool>, JoinHandle<()>);

/// Everything Graft writes to the pty, gathered on a thread until the
/// flag is cleared or the pty closes.
fn read_into_memory(master: &File) -> Reader {
    let output = Arc::new(Mutex::new(Vec::new()));
    let reading = Arc::new(AtomicBool::new(true));
    let mut source = master.try_clone().expect("master fd");
    let sink = Arc::clone(&output);
    let still = Arc::clone(&reading);
    // Polled, not blocked, so it lets go of its copy of the master when
    // told: `hang_up` must close the last one.
    let reader = std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        while still.load(Ordering::Acquire) {
            let mut fds = [PollFd::new(source.as_fd(), PollFlags::POLLIN)];
            match poll(&mut fds, PollTimeout::from(20_u16)) {
                Ok(0) => {}
                Ok(_) => match source.read(&mut buffer) {
                    Ok(read @ 1..) => sink
                        .lock()
                        .expect("output")
                        .extend_from_slice(&buffer[..read]),
                    // EOF or EIO: the last slave closed; Graft has gone.
                    Ok(0) | Err(_) => return,
                },
                Err(error) => panic!("polling the pty: {error}"),
            }
        }
    });
    (output, reading, reader)
}

impl Session {
    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn send(&mut self, bytes: &[u8]) {
        let master = self.master.as_mut().expect("the terminal is open");
        master.write_all(bytes).expect("typed");
        master.flush().expect("flushed");
    }

    fn wait_for(&self, what: &str, done: impl Fn(&Screen) -> bool) {
        let start = Instant::now();
        loop {
            let screen = Screen::replay(
                &self.output.lock().expect("output"),
                self.size.0,
                self.size.1,
            );
            if done(&screen) {
                return;
            }
            assert!(
                start.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for {what}; the screen:\n{}",
                screen.text()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Close the terminal: every master fd this side holds.
    fn hang_up(&mut self) {
        self.reading.store(false, Ordering::Release);
        if let Some(reader) = self.reader.take() {
            reader.join().expect("the reader ends");
        }
        drop(self.master.take());
    }

    /// `kill -HUP` Graft's own pid: the signal again.
    fn hang_up_again(&self) {
        let status = Command::new("kill")
            .args(["-HUP", &self.pid().to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "SIGHUP not delivered");
    }

    /// Wait up to `bound` for Graft to exit; `None` past it.
    fn exit_within(&mut self, bound: Duration) -> Option<(ExitStatus, Duration)> {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("waitable") {
                return Some((status, start.elapsed()));
            }
            if start.elapsed() > bound {
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            if let Err(error) = self.child.kill() {
                eprintln!("could not stop graft: {error}");
            }
            if let Err(error) = self.child.wait() {
                eprintln!("could not reap graft: {error}");
            }
        }
    }
}

/// `pre-merge-commit`: git's pid, its own, then `body`.
fn hook(repo: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let hook = repo.join(".git/hooks/pre-merge-commit");
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        format!("#!/bin/sh\necho $PPID > .git/git.pid\necho $$ > .git/hook.pid\n{body}\n"),
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// The repository before the merge.
struct Before {
    head: String,
    /// `main`'s tip: the merge's source.
    source: String,
    /// `git status --porcelain`: his edit alone.
    status: String,
}

/// A merge of `main` into `feature` started in the app with `hook_body`
/// as its `pre-merge-commit`, his edit to `base.txt` on disk: `j j m` (the
/// preview), `y`, and the hook running. Graft, git and the hook are held by
/// `leftovers`.
struct Merging {
    session: Session,
    git: u32,
    hook: u32,
    before: Before,
}

fn merging(
    isolated: &Isolated,
    repo: &Path,
    hook_body: &str,
    stderr: &Stderr,
    leftovers: &mut Leftovers,
) -> Merging {
    std::fs::write(repo.join("base.txt"), HIS_EDIT).expect("writable");
    let before = Before {
        head: git(repo, &["rev-parse", "HEAD"]).trim().to_owned(),
        source: git(repo, &["rev-parse", "main"]).trim().to_owned(),
        status: git(repo, &["status", "--porcelain"]),
    };
    hook(repo, hook_body);
    let mut session = Session::spawn(isolated, repo, (100, 30), stderr);
    leftovers.hold(session.pid());
    session.wait_for("the graph", |s| s.has("finish the feature"));
    session.send(b"jj");
    session.wait_for("main's row", |s| {
        s.lines()
            .iter()
            .any(|line| line.contains('\u{25b8}') && line.contains("main moves again"))
    });
    session.send(b"m");
    session.wait_for("the merge preview's plan", |s| {
        s.has("no conflicts expected")
    });
    session.send(b"y");
    let git = pid(repo, "git.pid");
    let hook = pid(repo, "hook.pid");
    leftovers.hold(git);
    leftovers.hold(hook);
    Merging {
        session,
        git,
        hook,
        before,
    }
}

/// Wait until `file` holds `needle`: the exit has reached that line.
fn wait_for_line(file: &Path, needle: &str) -> String {
    let began = Instant::now();
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if text.contains(needle) {
            return text;
        }
        assert!(
            began.elapsed() < PATIENCE,
            "never saw {needle:?}; stderr so far:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Graft's crash reports in its isolated state directory: none, or the
/// panic is named.
fn assert_no_crash(isolated: &Isolated) {
    let dir = isolated.home().join(".local/state/graft");
    let reports: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .map(|entry| {
                    let path = entry.expect("a readable entry").path();
                    std::fs::read_to_string(&path).expect("a readable report")
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        reports.is_empty(),
        "graft panicked:\n{}",
        reports.join("\n")
    );
}

/// The merge is paused on disk, as the stuck write's `q y` leaves it:
/// `MERGE_HEAD` the source, `HEAD` unmoved, no lock, git and the hook gone,
/// the entry `Pending`.
fn assert_paused(repo: &Path, merging: &Merging) {
    let merge_head = std::fs::read_to_string(repo.join(".git/MERGE_HEAD"))
        .unwrap_or_else(|why| panic!("the merge is paused: {why}"));
    assert_eq!(merge_head.trim(), merging.before.source);
    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]).trim(),
        merging.before.head
    );
    assert_eq!(locks(&repo.join(".git")), Vec::<PathBuf>::new());
    assert!(
        gone_within(merging.git, Duration::from_secs(1)),
        "git still runs"
    );
    assert!(
        gone_within(merging.hook, Duration::from_secs(1)),
        "the hook's sleep still runs"
    );
    let merges: Vec<_> = entries(repo)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Merge)
        .collect();
    assert_eq!(merges.len(), 1, "{merges:?}");
    assert_eq!(merges[0].status, EntryStatus::Pending);
}

/// 13d's observation, red before 13g with exit 101 after ~0.05 s: stderr on
/// the pty, the merge's hook still running, the master closed. Graft waits
/// for the write instead; once the hook lets the merge land, Graft dies of
/// `SIGHUP` as the `SIGTERM` path dies of `SIGTERM`. No crash report.
///
/// The hook holds the merge until the test lets it go, rather than hanging
/// for good and being ended by a second `SIGHUP`. "The signal again" counts
/// from the exit's waiting line (`exit_wait::Pool::stop_to`); a second
/// signal that comes before the exit reaches that line is not counted. Here
/// stderr is the dead pty, so the test cannot see the line and has no moment
/// at which a second signal is sure to count. A fixed 2 s pause failed when
/// the machine was loaded (13g NOTES, debugger). The second signal is tested
/// by `a_hangup_then_a_second_hangup_leaves_the_merge_paused`, which reads
/// the line first. The stuck hook stopped with stderr dead is tested by the
/// heavy-tier `a_hangup_with_a_stuck_hook_is_stopped_after_the_wait_and_left_paused`.
#[test]
fn a_hangup_mid_write_never_exits_101() {
    let isolated = Isolated::new();
    let scratch = crate::preview::forked();
    let repo = scratch.path().canonicalize().expect("the repo resolves");
    let mut leftovers = Leftovers::default();
    let release = repo.join(".git/release");
    let mut merging = merging(
        &isolated,
        &repo,
        "while [ ! -e .git/release ]; do sleep 0.05; done\nexit 0",
        &Stderr::Pty,
        &mut leftovers,
    );

    merging.session.hang_up();
    let early = merging.session.exit_within(Duration::from_secs(2));
    assert_eq!(early, None, "the hang-up waits for the write");
    assert!(alive(merging.git), "git runs on through the wait");
    std::fs::write(&release, "").expect("the hook is let go");
    let (status, _) = merging
        .session
        .exit_within(EXIT_WRITE_WAIT)
        .expect("the merge lands and Graft exits within the wait");

    assert_ne!(status.code(), Some(101), "{status:?}");
    assert_eq!(status.signal(), Some(SIGHUP), "{status:?}");
    assert_no_crash(&isolated);
    assert!(
        gone_within(merging.git, Duration::from_secs(1)),
        "git ended"
    );
    assert!(
        gone_within(merging.hook, Duration::from_secs(1)),
        "the hook ended"
    );
}

/// The hook takes 2 s: the master closed while it runs, the merge lands
/// whole inside `EXIT_WRITE_WAIT`, and Graft then dies of the hang-up.
#[test]
fn a_hangup_mid_merge_lets_a_quick_hook_land() {
    let isolated = Isolated::new();
    let scratch = crate::preview::forked();
    let repo = scratch.path().canonicalize().expect("the repo resolves");
    let mut leftovers = Leftovers::default();
    let mut merging = merging(
        &isolated,
        &repo,
        "sleep 2\nexit 0",
        &Stderr::Pty,
        &mut leftovers,
    );

    merging.session.hang_up();
    let (status, took) = merging
        .session
        .exit_within(EXIT_WRITE_WAIT)
        .expect("the merge lands and Graft exits within the wait");
    eprintln!("13g NOTES: quick hook, hang-up to exit: {took:?}");

    assert_eq!(status.signal(), Some(SIGHUP), "{status:?}");
    assert_eq!(
        git(&repo, &["rev-parse", "HEAD^2"]).trim(),
        merging.before.source
    );
    assert!(!repo.join(".git/MERGE_HEAD").exists(), "the merge is done");
    assert_eq!(locks(&repo.join(".git")), Vec::<PathBuf>::new());
    assert!(
        gone_within(merging.git, Duration::from_secs(1)),
        "git ended"
    );
    assert!(
        gone_within(merging.hook, Duration::from_secs(1)),
        "the hook ended"
    );
    let merges: Vec<_> = entries(&repo)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Merge)
        .collect();
    assert_eq!(merges.len(), 1, "{merges:?}");
    assert_eq!(merges[0].status, EntryStatus::Committed);
    assert_no_crash(&isolated);
}

/// The hook never ends: the master closed, then `SIGHUP` again once the
/// exit says it is waiting. Graft stops git (hook first, so git pauses the
/// merge) and exits within `FORCED_WAIT`. The next launch, in a new 120x40
/// pty, offers `c finish  a abort`, and `a` `y` gives back `HEAD`, `git
/// status` and his file as they were. The capture is that launch at 120x40.
#[test]
fn a_hangup_then_a_second_hangup_leaves_the_merge_paused() {
    let isolated = Isolated::new();
    let scratch = crate::preview::forked();
    let repo = scratch.path().canonicalize().expect("the repo resolves");
    let said = isolated.root.path().join("stderr.txt");
    let mut leftovers = Leftovers::default();
    let mut merging = merging(
        &isolated,
        &repo,
        "exec sleep 600",
        &Stderr::File(said.clone()),
        &mut leftovers,
    );

    merging.session.hang_up();
    wait_for_line(&said, "send the signal again to stop it half-way");
    merging.session.hang_up_again();
    let (status, took) = merging
        .session
        .exit_within(FORCED_WAIT + Duration::from_secs(5))
        .expect("the second hang-up ends it");
    eprintln!("13g NOTES: second hang-up to exit: {took:?}");

    assert!(took <= FORCED_WAIT, "{took:?}");
    assert_eq!(status.signal(), Some(SIGHUP), "{status:?}");
    assert_paused(&repo, &merging);
    let text = std::fs::read_to_string(&said).expect("stderr");
    assert!(
        text.contains("the merge is paused \u{b7} graft here offers continue or abort"),
        "{text}"
    );
    assert_no_crash(&isolated);
    std::fs::remove_file(repo.join(".git/hooks/pre-merge-commit")).expect("the hook goes");

    capture_the_relaunch(&repo);
    relaunch_and_abort(&isolated, &repo, &merging.before);
}

/// The next launch at 120x40, through the harness at a fixed clock and a
/// fixed path so the capture is the same run to run.
///
/// What right looks like (`m8-hangup-relaunch-120x40.txt`, 13g): nothing in
/// the shell — the terminal had closed. The next launch: `MERGING` at the
/// top, the banner `merge still in progress from a previous run · c finish
/// a abort`. Colour unverified; looks-right is the user's.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo nextest run -p graft
/// --test it quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused`.
fn capture_the_relaunch(repo: &Path) {
    use crate::preview::FORK_NOW;
    use graft::session::Clock;

    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, 40);
    let mut state = app.pump(state, "the relaunch's reads", |state| {
        use graft::update::detail::Loading;
        state.in_progress.is_some()
            && state.refs.is_some()
            && !state.live_entries.is_empty()
            && state.working_tree.is_some()
            && state.operating.is_none()
            && !matches!(state.detail.files, Loading::Pending(_))
            && !matches!(state.detail.diff, Loading::Pending(_))
    });
    if let Some(facts) = state.repository.as_mut() {
        facts.path = PathBuf::from("/code/api");
    }
    let body = crate::captures::frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW))).join("\n");
    let banner = body
        .lines()
        .find(|line| line.contains("c finish  a abort"))
        .unwrap_or_else(|| panic!("no banner row:\n{body}"));
    eprintln!("13g NOTES: banner row: {}", banner.trim_end());
    assert!(banner.contains("from a previous run"), "{banner}");
    crate::captures::check_shot(
        "m8-hangup-relaunch-120x40.txt",
        &format!(
            "=== the terminal closed mid-merge (hook stuck), then SIGHUP again: nothing printed, \
             the terminal was gone ===\n\
             === the next launch ===\n{body}\n"
        ),
    );
    drop(state);
    app.close();
}

/// The real binary in a new 120x40 pty: the banner, `a`, `y`, and the
/// repository as it was before the merge.
fn relaunch_and_abort(isolated: &Isolated, repo: &Path, before: &Before) {
    let mut pty = Pty::spawn(isolated, repo, repo, (120, 40));
    pty.wait_for("the paused merge's banner", |s| s.has("c finish  a abort"));
    pty.send(b"a");
    pty.wait_for("the abort's question", |s| s.has("abort"));
    pty.send(b"y");
    let began = Instant::now();
    while repo.join(".git/MERGE_HEAD").exists() {
        assert!(began.elapsed() < PATIENCE, "the abort never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    pty.wait_for("the banner gone", |s| !s.has("c finish  a abort"));
    pty.send(b"q");
    // The abort's job may still be answering: then `q` boxes (`Quit?`, the
    // write is still running) and `y` quits once it is done — his own way
    // out. Otherwise `q` has already quit.
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(2) {
        if pty.screen().has("quit waits for it to finish") {
            pty.send(b"y");
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "{stderr}");

    assert_eq!(git(repo, &["rev-parse", "HEAD"]).trim(), before.head);
    assert_eq!(git(repo, &["status", "--porcelain"]), before.status);
    assert_eq!(
        std::fs::read_to_string(repo.join("base.txt")).expect("his file"),
        HIS_EDIT,
        "his edit, byte-equal"
    );
}

/// The hook never ends and no second signal comes: the write gets its
/// `EXIT_WRITE_WAIT`, then is stopped as `q y` stops it and left paused.
/// Stderr is the dead pty, as a shell runs it: the forced stop's lines go
/// nowhere and must not end the process. About 60 s, so heavy tier only.
#[cfg(feature = "heavy-tests")]
#[test]
fn a_hangup_with_a_stuck_hook_is_stopped_after_the_wait_and_left_paused() {
    let isolated = Isolated::new();
    let scratch = crate::preview::forked();
    let repo = scratch.path().canonicalize().expect("the repo resolves");
    let mut leftovers = Leftovers::default();
    let mut merging = merging(
        &isolated,
        &repo,
        "exec sleep 600",
        &Stderr::Pty,
        &mut leftovers,
    );

    merging.session.hang_up();
    let (status, took) = merging
        .session
        .exit_within(EXIT_WRITE_WAIT + FORCED_WAIT + Duration::from_secs(5))
        .expect("the wait ends and the write is stopped");
    eprintln!("13g NOTES: stuck hook, hang-up to exit: {took:?}");

    assert!(took <= EXIT_WRITE_WAIT + FORCED_WAIT, "{took:?}");
    assert_eq!(status.signal(), Some(SIGHUP), "{status:?}");
    assert_paused(&repo, &merging);
    assert_no_crash(&isolated);
}
