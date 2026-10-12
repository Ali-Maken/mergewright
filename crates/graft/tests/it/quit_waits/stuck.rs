//! A stuck write (m8/13b review fix): a git hook that never ends. Real
//! repository, real job pool, real git and a real hook (`sleep 600`).
//!
//! The walk is the user's: `q y` (Graft waits), the stop offer, `q y`
//! (stop it now). The offer's 10 s are the loop's clock
//! (`event_loop::tests::quit_offer` walks that timeline); here, with no
//! loop, the harness plays the clock by calling `quit::waited_long` itself
//! — the injected short `STOP_OFFER_AFTER`. Then: git and its hook gone,
//! the pool down inside `FORCED_WAIT`, no lock left, and the operation
//! paused on disk, its entry `Pending`: a merge because Graft ended the
//! stuck hook first and git paused the merge itself (13c), a rebase because
//! it is stopped where it stands (13d Q1). The exit line says so, and the
//! next launch offers continue or abort.
//!
//! A leftover hook is killed by pid (`reap`) in case a regression leaves
//! one; the walks assert it is gone first.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use graft::jobs::{EXIT_WRITE_WAIT, FORCED_WAIT, Forced, SHUTDOWN_GRACE, Shutdown};
use graft::update::{AppState, Command, Quit};
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::JournalEntry;
use graft_core::journal::{EntryStatus, OperationKind, PutBack, PutBackWhy};
use graft_git::state::InProgress;
use graft_ops::apply::StoppedAt;

use super::{assert_no_cancel, merging, press};
use crate::app_harness::{App, git};
use crate::preview::{forked, on_main, opened};
use graft_core::on_disk::CURRENT_DIR;

/// Fails when the walk took more keys than the note's fence allows for `id`.
fn within_the_note(id: &str, keys: usize) {
    crate::m8_walk::within_the_note_taking(id, keys, "(keys)");
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
pub(super) fn pid(repo: &Path, file: &str) -> u32 {
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
pub(super) fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .expect("kill runs")
        .success()
}

/// Waits up to `bound` for `pid` to be gone. A killed child is reaped by
/// the pool's job, which may lag the signal by a poll.
pub(super) fn gone_within(pid: u32, bound: Duration) -> bool {
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
pub(super) fn locks(dir: &Path) -> Vec<PathBuf> {
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

pub(super) fn entries(repo: &Path) -> Vec<JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl")))
        .expect("the journal is readable")
        .entries()
        .to_vec()
}

/// `q y` → waiting; the clock calls the wait long; `q` → the stop box;
/// `y` → `ForceStop`, `Quit::StoppedHalfWay`. Counts the keys pressed.
pub(super) fn walk_to_the_stop(app: &App, state: &AppState) -> (AppState, usize) {
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
pub(super) fn close_within(app: App, bound: Duration) -> (Shutdown, Duration) {
    let (sent, got) = mpsc::channel();
    let began = Instant::now();
    std::thread::spawn(move || {
        // The real exit's shutdown (`event_loop::exit_wait`): a forced
        // write's answer is waited for up to `FORCED_WAIT`.
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
/// since 13c the pool stops git's whole process group; this reaps by
/// pidfile only what a failing test leaves behind). By pid, never by pattern.
fn reap(pid: u32) {
    if alive(pid) {
        let status = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "could not kill the leftover hook {pid}");
    }
}

/// The exit line the shell gets for `forced`, as `event_loop::exit_wait`
/// prints it: the binary's name, a fixed wait (the harness has no loop
/// clock; the offer is injected), and undo as the default keymap spells it.
pub(super) fn exit_line_for(forced: &Forced) -> String {
    graft::update::quit::exit_line(graft::cli::COMMAND, &forced.what, OFFERED, forced.at, "u")
}

/// The wait the exit lines here are worded with: the offer's 10 s.
const OFFERED: Duration = Duration::from_secs(10);

/// His edit to a file the merge does not touch.
const HIS_EDIT: &str = "base, his edit\n";

/// A merge stopped in its stuck `pre-merge-commit` hook by the stop box's
/// `y`, and closed as the real exit closes it: no answer pumped, the pool's
/// shutdown waits for it.
pub(super) struct StoppedMerge {
    pub shutdown: Shutdown,
    /// From the stop box's `y` to the pool being down: the quit's wait.
    pub took: Duration,
    pub keys: usize,
    pub git_gone: bool,
    pub hook_gone: bool,
    /// `main`'s tip, the merge's source.
    pub source: String,
    /// `HEAD` before the merge.
    pub head_before: String,
    /// `git status --porcelain` before the merge: his edit alone.
    pub status_before: String,
}

/// `feature` checked out with his edit to `base.txt`, a hanging
/// `pre-merge-commit`, `m m` onto `main`, then `q y`, the offer, `q y`.
/// The hook is removed afterwards, so a relaunch's `c` would not hang.
pub(super) fn stop_a_stuck_merge(repo: &Path) -> StoppedMerge {
    std::fs::write(repo.join("base.txt"), HIS_EDIT).expect("writable");
    let head_before = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let source = git(repo, &["rev-parse", "main"]).trim().to_owned();
    let status_before = git(repo, &["status", "--porcelain"]);
    hang(repo, "pre-merge-commit");
    let (app, state) = App::open(repo);
    let state = merging(&app, state);
    let git_pid = pid(repo, "git.pid");
    let hook_pid = pid(repo, "hook.pid");

    let (state, keys) = walk_to_the_stop(&app, &state);
    // The loop ends at `y`: its answer is never pumped, as in the app.
    drop(state);
    let (shutdown, took) = close_within(app, FORCED_WAIT + Duration::from_secs(5));
    let git_gone = gone_within(git_pid, Duration::from_secs(1));
    let hook_gone = gone_within(hook_pid, Duration::from_secs(1));
    reap(hook_pid);
    std::fs::remove_file(repo.join(".git/hooks/pre-merge-commit")).expect("the hook goes");
    StoppedMerge {
        shutdown,
        took,
        keys,
        git_gone,
        hook_gone,
        source,
        head_before,
        status_before,
    }
}

/// 13c's walk, 13b's NOT MET box: a merge whose `pre-merge-commit` hook
/// hangs, `q y`, the offer, `q y`. Graft ends the hook first, git pauses
/// the merge itself, and the exit line says the merge is paused. Git and
/// the hook gone, the quit inside `FORCED_WAIT`, `MERGE_HEAD` the source's
/// tip, `HEAD` unmoved, no lock, the entry `Pending`, and his edit waiting
/// aside (R6). `quit-stuck-write 4`.
#[test]
fn stop_now_mid_merge_leaves_it_paused() {
    let scratch = forked();
    let repo = scratch.path();
    let stopped = stop_a_stuck_merge(repo);

    within_the_note("quit-stuck-write", stopped.keys);
    assert!(stopped.git_gone, "git still runs");
    assert!(stopped.hook_gone, "the hook's sleep still runs");
    eprintln!(
        "13d NOTES: the stop box's y to the pool down: {:?}",
        stopped.took
    );
    assert!(stopped.took <= FORCED_WAIT, "{:?}", stopped.took);
    let [forced] = stopped.shutdown.forced.as_slice() else {
        panic!("one forced write: {:?}", stopped.shutdown);
    };
    assert_eq!(stopped.shutdown.stalled, 0);
    assert_eq!(forced.what, "merge main into feature");
    assert_eq!(forced.at, Some(StoppedAt::Paused(InProgress::Merge)));
    assert_eq!(
        exit_line_for(forced),
        "mwr: stopped merge main into feature after 10 s \u{b7} the merge is paused \u{b7} \
         mwr here offers continue or abort"
    );

    let merge_head = std::fs::read_to_string(repo.join(".git/MERGE_HEAD"))
        .unwrap_or_else(|why| panic!("the merge is paused: {why}"));
    assert_eq!(merge_head.trim(), stopped.source);
    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]).trim(),
        stopped.head_before
    );
    assert_eq!(locks(&repo.join(".git")), Vec::<PathBuf>::new());
    let merges: Vec<_> = entries(repo)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Merge)
        .collect();
    assert_eq!(merges.len(), 1, "{merges:?}");
    assert_eq!(merges[0].status, EntryStatus::Pending);
    assert_his_edit_waits_aside(repo, &merges[0]);
}

/// R6 (19d-a2): a merge sets his edit aside (ADR-0045 §4), so while it is
/// paused his edit waits in `S`, listed by commit, and the tree holds
/// `HEAD`'s copy; `resume` puts it back.
pub(super) fn assert_his_edit_waits_aside(repo: &Path, merge: &JournalEntry) {
    let stash = set_aside_stash(merge);
    assert!(
        listed(repo, &stash),
        "S is listed: {}",
        git(repo, &["stash", "list"])
    );
    assert_eq!(
        git(repo, &["show", &format!("{stash}:base.txt")]),
        HIS_EDIT,
        "his edit is in S"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("base.txt")).expect("the file"),
        git(repo, &["show", "HEAD:base.txt"]),
        "the tree holds HEAD's copy"
    );
}

/// While the merge is paused with his edit aside, what is staged is the
/// merge's own work and nothing of his (19d-a3, ticket §R7.2): the index's
/// changed paths, read from the index, equal what the source brings since
/// the merge base, read from the commits — `main-one.txt` and
/// `main-two.txt` in [`forked`] — and his `base.txt` is not among them.
pub(super) fn assert_staged_are_the_merges_own(repo: &Path) {
    let staged = git(repo, &["diff", "--cached", "--name-only"]);
    let brought = git(repo, &["diff", "--name-only", "HEAD...MERGE_HEAD"]);
    assert_eq!(staged, brought, "the staged paths are the merge's own");
    assert_eq!(
        staged, "main-one.txt\nmain-two.txt\n",
        "as forked() makes them"
    );
}

/// The stash a set-aside entry recorded (`S`, ADR-0045 §1), as full hex.
pub(super) fn set_aside_stash(entry: &JournalEntry) -> String {
    match &entry.put_back {
        Some(PutBack {
            stash,
            why: PutBackWhy::SetAside,
        }) => stash.to_string(),
        other => panic!("not a set-aside entry: {other:?}"),
    }
}

/// Whether `stash` is in the stash list, found by commit.
pub(super) fn listed(repo: &Path, stash: &str) -> bool {
    git(repo, &["stash", "list", "--format=%H"])
        .lines()
        .any(|line| line == stash)
}

/// The next launch over that stopped merge offers continue or abort — the
/// live merge's own banner words, `c finish  a abort`, with the stop line's
/// ` · edits aside` rung (R6: his edit waits in `S`) — and `a` (then `y`)
/// puts `HEAD` and the tree back as they were, his edit byte-equal and `S`
/// dropped. Recovery leaves the paused entry live, `S` with it.
#[test]
fn relaunch_after_a_stopped_merge_offers_continue_and_abort() {
    let scratch = forked();
    let repo = scratch.path();
    let stopped = stop_a_stuck_merge(repo);
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "{:?}",
        stopped.shutdown
    );

    let (app, state) = App::open(repo);
    let state = app.pump(state, "the relaunch's reads", |state| {
        state.in_progress.is_some() && state.refs.is_some() && !state.live_entries.is_empty()
    });
    // Recovery (`recover_interrupted`, run by the first journal read) left
    // the paused set-aside merge alone: live, still `Pending`, `S` listed,
    // nothing closed as interrupted.
    let merge = entries(repo)
        .into_iter()
        .find(|entry| entry.kind == OperationKind::Merge)
        .expect("the merge's entry");
    assert_eq!(
        state.live_entries,
        vec![merge.id.clone()],
        "the merge is live"
    );
    assert_eq!(merge.status, EntryStatus::Pending);
    assert!(state.interrupted_edits.is_empty(), "nothing interrupted");
    assert_his_edit_waits_aside(repo, &merge);
    let stash = set_aside_stash(&merge);
    let banner = crate::captures::frame_at(&state, 100, 30, None).join("\n");
    // The live merge's own banner, quoted: with nothing unmerged its `c`
    // reads `finish` (the merge's commit is all that is left), and the
    // set-aside's rung says where his edit is.
    assert!(
        banner.contains(
            "merge still in progress from a previous run \u{b7} edits aside \u{b7} c finish  \
             a abort"
        ),
        "{banner}"
    );
    assert!(state.banner_focus, "the banner has the keyboard: {banner}");
    let asked = app.key(&state, KeyCode::Char('a'));
    assert!(asked.confirm.is_some(), "a asks first");
    let aborting = app.key(&asked, KeyCode::Char('y'));
    let aborted = app.pump(aborting, "the abort", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    drop(aborted);
    app.close();

    assert!(!repo.join(".git/MERGE_HEAD").exists(), "the merge is gone");
    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]).trim(),
        stopped.head_before
    );
    assert_eq!(git(repo, &["status", "--porcelain"]), stopped.status_before);
    assert_eq!(
        std::fs::read_to_string(repo.join("base.txt")).expect("his file"),
        HIS_EDIT,
        "his edit, byte-equal"
    );
    assert!(!listed(repo, &stash), "S is dropped");
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
/// `post-checkout` hook hangs after git has begun. Stopped where it stands
/// (13d Q1: not hook-first, which would let git finish it), the rebase is
/// paused with no lock left, the exit line says so, and the relaunch offers
/// continue or abort, and his edits back. Closed as the real exit closes:
/// the answer is the shutdown's, never pumped.
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
    drop(state);
    let (shutdown, took) = close_within(app, FORCED_WAIT + Duration::from_secs(5));
    assert!(gone_within(git_pid, SHUTDOWN_GRACE), "git still runs");
    eprintln!("13d NOTES: rebase, the stop box's y to the pool down: {took:?}");
    assert!(took <= FORCED_WAIT, "{took:?}");
    let [forced] = shutdown.forced.as_slice() else {
        panic!("one forced write: {shutdown:?}");
    };
    assert_eq!(shutdown.stalled, 0);
    assert_eq!(forced.at, Some(StoppedAt::Paused(InProgress::Rebase)));
    assert_eq!(
        exit_line_for(forced),
        format!(
            "mwr: stopped {} after 10 s \u{b7} the rebase is paused \u{b7} mwr here offers \
             continue or abort",
            forced.what
        )
    );
    let hook_gone = gone_within(hook_pid, Duration::from_secs(1));
    reap(hook_pid);
    assert!(hook_gone, "the hook's sleep still runs");
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
