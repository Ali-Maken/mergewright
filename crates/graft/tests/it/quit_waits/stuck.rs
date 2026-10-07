//! A stuck write (m8/13b review fix): a git hook that never ends. Real
//! repository, real job pool, real git and a real hook (`sleep 600`).
//!
//! The walk is the user's: `q y` (Graft waits), the stop offer, `q y`
//! (stop it now). The offer's 10 s are the loop's clock
//! (`event_loop::tests::quit_offer` walks that timeline); here, with no
//! loop, the harness plays the clock by calling `quit::waited_long` itself
//! — the injected short `STOP_OFFER_AFTER`. Then: git gone, the pool down
//! inside `SHUTDOWN_GRACE`, no lock left. A rebase is paused on disk, its
//! entry `Pending`, and the next launch offers continue or abort with his
//! edits named. A merge stopped in `pre-merge-commit` is **not** paused —
//! see `stop_now_mid_merge_stops_git_and_leaves_no_lock`.
//!
//! The hook itself outlives git's stop: the pool signals git's pid, not its
//! process group. Each test kills the leftover by pid (`reap`).

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use graft::jobs::{EXIT_WRITE_WAIT, SHUTDOWN_GRACE, Shutdown};
use graft::update::{AppState, Command, Quit};
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::JournalEntry;
use graft_core::journal::{EntryStatus, OperationKind};

use super::{assert_no_cancel, merging, press};
use crate::app_harness::{App, git};
use crate::preview::{forked, on_main, opened};

/// The design note whose `journey-counts` fence these walks are held to.
const NOTE: &str = "../../docs/design-notes/m8-polish-and-ship.md";

/// Fails when the walk took more keys than the note's fence allows for `id`.
fn within_the_note(id: &str, keys: usize) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    let note =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    let allowed = crate::m5_walk::parse_counts(&note)
        .unwrap_or_else(|why| panic!("the M8 fence does not parse: {why}"))
        .iter()
        .find(|row| row.id == id)
        .and_then(|row| row.keys)
        .unwrap_or_else(|| panic!("the M8 fence has no count for {id}"));
    assert!(
        keys <= allowed,
        "{id}: {keys} keys, the note allows {allowed}"
    );
}

/// How long the hook may take to start once the write is launched.
const HOOK_STARTS: Duration = Duration::from_secs(20);

/// A hook that records git's pid and its own, then sleeps for ten minutes.
fn hang(repo: &Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;
    let hook = repo.join(".git/hooks").join(name);
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho $PPID > .git/git.pid\necho $$ > .git/hook.pid\nexec sleep 600\n",
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// A pid the hook wrote, once it has written it. Polled: the hook runs in
/// git, which runs in the pool, and nothing tells the test when it starts.
fn pid(repo: &Path, file: &str) -> u32 {
    let path = repo.join(".git").join(file);
    let began = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        assert!(began.elapsed() < HOOK_STARTS, "the hook never ran ({file})");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// `kill -0 pid`: is the process still there?
fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .expect("kill runs")
        .success()
}

/// Waits up to `bound` for `pid` to be gone. A killed child is reaped by
/// the pool's job, which may lag the signal by a poll.
fn gone_within(pid: u32, bound: Duration) -> bool {
    let began = Instant::now();
    while alive(pid) {
        if began.elapsed() > bound {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

/// Every `*.lock` under `.git`.
fn locks(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect(".git is readable") {
        let path = entry.expect("a readable entry").path();
        if path.is_dir() {
            found.extend(locks(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}

fn entries(repo: &Path) -> Vec<JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal is readable")
        .entries()
        .to_vec()
}

/// `q y` → waiting; the clock calls the wait long; `q` → the stop box;
/// `y` → `ForceStop`, `Quit::StoppedHalfWay`. Counts the keys pressed.
fn walk_to_the_stop(app: &App, state: &AppState) -> (AppState, usize) {
    let mut keys = 0;
    let mut step = |state: &AppState, key: char| {
        keys += 1;
        press(app, state, KeyPress::plain(KeyCode::Char(key)))
    };
    let (boxed, commands) = step(state, 'q');
    assert_no_cancel(&commands, "`q`");
    let (mut waiting, commands) = step(&boxed, 'y');
    assert_no_cancel(&commands, "`y`");
    assert!(waiting.quit_when_done.is_some(), "waiting to quit");
    graft::update::quit::waited_long(&mut waiting);
    let (stop_box, commands) = step(&waiting, 'q');
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        stop_box
            .confirm
            .as_ref()
            .map(|open| open.view.title.as_str()),
        Some("Quit now?")
    );
    let id = waiting.operating.expect("the write runs");
    let (stopped, commands) = step(&stop_box, 'y');
    assert_eq!(commands, vec![Command::ForceStop(id)]);
    assert!(
        matches!(stopped.quit, Some(Quit::StoppedHalfWay { .. })),
        "{:?}",
        stopped.quit
    );
    (stopped, keys)
}

/// The pool shut down on another thread, joined with a timeout: a hang
/// here fails the test instead of hanging it.
fn close_within(app: App, bound: Duration) -> (Shutdown, Duration) {
    let (sent, got) = mpsc::channel();
    let began = Instant::now();
    std::thread::spawn(move || {
        let shutdown = app.jobs.shutdown(SHUTDOWN_GRACE, EXIT_WRITE_WAIT, &());
        // The receiver is gone only if the test already failed on its
        // timeout; nothing is left to tell.
        sent.send(shutdown).unwrap_or(());
    });
    let shutdown = got
        .recv_timeout(bound)
        .unwrap_or_else(|_| panic!("the pool did not shut down within {bound:?}"));
    (shutdown, began.elapsed())
}

/// The next launch: the paused operation's banner, read as drawn once the
/// operation, the refs and the journal's pending entry are all read.
fn relaunch_banner(repo: &Path) -> String {
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the relaunch's reads", |state| {
        state.in_progress.is_some() && state.refs.is_some() && !state.live_entries.is_empty()
    });
    let frame = crate::captures::frame_at(&state, 100, 30, None).join("\n");
    drop(state);
    app.close();
    frame
}

/// Kills what the force-stop left running (the hook, item 10's question:
/// the pool signals git's pid, not its group). By pid, never by pattern.
fn reap(pid: u32) {
    if alive(pid) {
        let status = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "could not kill the leftover hook {pid}");
    }
}

/// A merge whose `pre-merge-commit` hook hangs, `q y`, the offer, `q y`:
/// git gone, the pool down within `SHUTDOWN_GRACE`, no lock left.
/// `quit-stuck-write 4` counted. What the merge leaves on disk is printed,
/// not asserted: see the comment at the end.
#[test]
fn stop_now_mid_merge_stops_git_and_leaves_no_lock() {
    let scratch = forked();
    let repo = scratch.path();
    hang(repo, "pre-merge-commit");
    let (app, state) = App::open(repo);
    let state = merging(&app, state);
    let git_pid = pid(repo, "git.pid");
    let hook_pid = pid(repo, "hook.pid");

    let (state, keys) = walk_to_the_stop(&app, &state);
    within_the_note("quit-stuck-write", keys);
    let state = app.pump(state, "the stopped merge's answer", |state| {
        state.operating.is_none()
    });
    drop(state);
    assert!(
        gone_within(git_pid, SHUTDOWN_GRACE),
        "git {git_pid} still runs"
    );
    let (shutdown, took) = close_within(app, SHUTDOWN_GRACE + Duration::from_secs(5));
    assert_eq!(shutdown, Shutdown::CLEAN);
    assert!(took <= SHUTDOWN_GRACE, "{took:?}");

    let hook_survived = alive(hook_pid);
    reap(hook_pid);
    eprintln!("item 10 / force-stop: the hook (sleep) survived git's stop: {hook_survived}");

    assert_eq!(locks(&repo.join(".git")), Vec::<PathBuf>::new());
    let merges: Vec<_> = entries(repo)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Merge)
        .collect();
    assert_eq!(merges.len(), 1, "{merges:?}");
    // NOT the ticket's acceptance (13b review fixes, Q1): the ticket expects
    // `MERGE_HEAD` kept, the entry `Pending` and the relaunch offering
    // continue/abort. Observed instead: git writes no `MERGE_HEAD` before
    // `pre-merge-commit` runs, so the stop leaves the merge's result staged
    // on `feature` with no merge in progress, and the entry closes
    // `Aborted { reason: "git was stopped" }`. Raised in the ticket for the
    // conductor; recorded here so the run shows it.
    eprintln!(
        "stopped merge: MERGE_HEAD {} · entry {:?}\n{}",
        repo.join(".git/MERGE_HEAD").exists(),
        merges[0].status,
        git(repo, &["status", "--short"])
    );
}

/// `quit-while-write 2`: `q y` and Graft quits when the merge lands.
#[test]
fn quit_while_write_is_two_keys() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = super::merging(&app, state);
    let mut keys = 0;
    let (state, _) = press(&app, &state, KeyPress::plain(KeyCode::Char('q')));
    keys += 1;
    let (state, _) = press(&app, &state, KeyPress::plain(KeyCode::Char('y')));
    keys += 1;
    let state = app.pump(state, "the merge's answer", |state| state.quit.is_some());
    assert_eq!(state.quit, Some(Quit::Requested));
    within_the_note("quit-while-write", keys);
    drop(state);
    app.close();
}

/// The same walk over a rebase that set his edits aside: its
/// `post-checkout` hook hangs after git has begun. Stopped half-way, the
/// rebase is paused, and the relaunch offers continue or abort, and his
/// edits back.
#[test]
fn stop_now_mid_rebase_with_edits_set_aside() {
    let scratch = forked();
    let repo = scratch.path();
    std::fs::write(repo.join("base.txt"), "base, edited\n").expect("writable");
    let (app, state) = App::open(repo);
    let state = rebasing_into_a_hang(&app, state, repo);
    let git_pid = pid(repo, "git.pid");
    let hook_pid = pid(repo, "hook.pid");

    let (state, keys) = walk_to_the_stop(&app, &state);
    within_the_note("quit-stuck-write", keys);
    let state = app.pump(state, "the stopped rebase's answer", |state| {
        state.operating.is_none()
    });
    drop(state);
    assert!(gone_within(git_pid, SHUTDOWN_GRACE), "git still runs");
    let (shutdown, took) = close_within(app, SHUTDOWN_GRACE + Duration::from_secs(5));
    assert_eq!(shutdown, Shutdown::CLEAN);
    assert!(took <= SHUTDOWN_GRACE, "{took:?}");
    reap(hook_pid);
    std::fs::remove_file(repo.join(".git/hooks/post-checkout")).expect("the hook goes");

    assert!(
        repo.join(".git/rebase-merge").exists() || repo.join(".git/rebase-apply").exists(),
        "the rebase is paused: {}",
        git(repo, &["status"])
    );
    assert_eq!(locks(&repo.join(".git")), Vec::<PathBuf>::new());
    let rebases: Vec<_> = entries(repo)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Rebase)
        .collect();
    assert_eq!(rebases.len(), 1, "{rebases:?}");
    assert_eq!(rebases[0].status, EntryStatus::Pending);
    assert!(rebases[0].put_back.is_some(), "the set-aside is journaled");
    let banner = relaunch_banner(repo);
    eprintln!("relaunch over the stopped rebase:\n{banner}");
    for offered in [
        "from a previous run",
        "edits aside",
        "c continue",
        "a abort",
    ] {
        assert!(banner.contains(offered), "{offered:?} missing: {banner}");
    }
}

/// `feature` checked out with an edit, the cursor on `main`, `r r` with a
/// `post-checkout` hook that hangs: the rebase is running and stuck.
fn rebasing_into_a_hang(
    app: &App,
    state: graft::update::AppState,
    repo: &std::path::Path,
) -> graft::update::AppState {
    let mut state = opened(app, state);
    for _ in 0..20 {
        if on_main(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_main(&state), "never found main's row");
    let state = app.key(&state, KeyCode::Char('r'));
    let state = app.pump(state, "the rebase plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    hang(repo, "post-checkout");
    let state = app.key(&state, KeyCode::Char('r'));
    assert!(
        state.operating.is_some(),
        "the rebase runs: {:?}",
        state.status
    );
    state
}
