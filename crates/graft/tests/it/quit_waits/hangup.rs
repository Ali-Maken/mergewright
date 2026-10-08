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

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use graft::jobs::{EXIT_WRITE_WAIT, FORCED_WAIT};
use graft_core::journal::{EntryStatus, OperationKind};

use super::pty_session::{
    Leftovers, PATIENCE, Session, Stderr, assert_no_crash, hook, wait_for_line,
};
use super::stuck::{
    alive, assert_his_edit_waits_aside, entries, gone_within, listed, locks, pid, set_aside_stash,
};
use crate::app_harness::{App, git};
use crate::pty::{Isolated, Pty};

/// The status a hang-up leaves: the process dies of `SIGHUP`, as the
/// `SIGTERM` path dies of `SIGTERM` (`quit_waits::signal` asserts
/// `status.signal() == Some(15)`; `main` re-raises the signal).
const SIGHUP: i32 = 1;

/// His edit to a file the merge does not touch.
const HIS_EDIT: &str = "base, his edit\n";

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

/// The merge is paused on disk, as the stuck write's `q y` leaves it:
/// `MERGE_HEAD` the source, `HEAD` unmoved, no lock, git and the hook gone,
/// the entry `Pending`, and his edit waiting aside — `S` listed, the tree
/// at `HEAD`'s copy (R6, 19d-a2). Answers `S`, for the abort to see dropped.
fn assert_paused(repo: &Path, merging: &Merging) -> String {
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
    assert_his_edit_waits_aside(repo, &merges[0]);
    set_aside_stash(&merges[0])
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
    let stash = assert_paused(&repo, &merging);
    let text = std::fs::read_to_string(&said).expect("stderr");
    assert!(
        text.contains("the merge is paused \u{b7} graft here offers continue or abort"),
        "{text}"
    );
    assert_no_crash(&isolated);
    std::fs::remove_file(repo.join(".git/hooks/pre-merge-commit")).expect("the hook goes");

    capture_the_relaunch(&repo);
    relaunch_and_abort(&isolated, &repo, &merging.before);
    assert!(!listed(&repo, &stash), "S is dropped");
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
    // `S` stays listed: no resume follows here.
    assert_paused(&repo, &merging);
    assert_no_crash(&isolated);
}
