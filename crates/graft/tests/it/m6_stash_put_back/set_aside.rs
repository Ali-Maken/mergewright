//! m6-conflicts/74: a rebase or a checkout over uncommitted edits sets them
//! aside and puts them back (ADR-0045) — walked against real repositories,
//! keys counted against the note's rows (`docs/design-notes/m6-conflicts.md`
//! `journey-counts`, from `m6-stash-put-back.md` "Journeys" :40-44).

use std::path::Path;

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;
use graft_fixtures::remote::LocalRemote;

use super::{Keys, cursor_to, main_and_fix, planned, rev};
use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, without_invented_id};
use crate::preview::{FORK_NOW, dated_commit, opened};
use crate::remote_by_key::{confirm, finish, local, ready, run};
use crate::stash_clash::{landed, quiet, stash_list, unmerged};

pub const PULL_REBASE: &str = "pull-rebase-dirty";
pub const REJECTED_PUSH: &str = "rejected-push-dirty-to-pushed";
pub const SWITCH: &str = "switch-dirty";
pub const SWITCH_CLASH: &str = "switch-dirty-clash";
pub const STOP_THEN_CLASH: &str = "pull-rebase-stop-then-clash";

/// `u` after any of them: 1 key (`m6-stash-put-back.md` :47, "`u` after any
/// of them: **1**"). The note gives it no row of its own, so no id.
const UNDO_KEYS: usize = 1;

/// The edits every walk makes: contents written over the tracked files.
const EDIT: &str = "my edit\n";

fn allowed(id: &str) -> usize {
    crate::m5_walk::noted_keys(id).unwrap_or_else(|why| panic!("{id}: {why}"))
}

fn read(repo: &Path, name: &str) -> String {
    std::fs::read_to_string(repo.join(name)).unwrap_or_else(|why| panic!("{name}: {why}"))
}

fn counted(app: &App) -> Keys<'_> {
    Keys { app, count: 0 }
}

/// A teammate pushed, a local commit `ours` adding three files, and those
/// three edited with one of them staged (a staged modification, which is
/// what the preview's staged count counts): the state `p` finds diverged
/// and dirty.
fn diverged_with_three_edits(remote: &LocalRemote) -> (Vec<String>, String) {
    let work = remote.work();
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let files: Vec<String> = ["one.txt", "two.txt", "three.txt"]
        .map(str::to_owned)
        .to_vec();
    for name in &files {
        std::fs::write(work.join(name), "committed\n").expect("the working clone is writable");
    }
    run(
        remote,
        work,
        &["add", "--", "one.txt", "two.txt", "three.txt"],
    );
    run(remote, work, &["commit", "-q", "-m", "ours"]);
    for name in &files {
        std::fs::write(work.join(name), EDIT).expect("the working clone is writable");
    }
    run(remote, work, &["add", "--", &files[0]]);
    (files, theirs)
}

/// `p` refused as diverged — the sticky line `r` answers. One key, counted.
fn pull_refused(keys: &mut Keys<'_>, state: AppState) -> AppState {
    let state = ready(keys.app, state);
    let state = keys.press(&state, KeyCode::Char('p'));
    let state = finish(keys.app, state, "the refused pull");
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("a diverged pull is a sticky line: {:?}", state.status);
    };
    assert!(reason.contains("have diverged"), "{reason}");
    state
}

/// `r` → the rebase preview, its plan landed. One key, counted.
fn rebase_preview(keys: &mut Keys<'_>, state: AppState) -> AppState {
    let state = ready(keys.app, state);
    let state = keys.press(&state, KeyCode::Char('r'));
    planned(keys.app, state)
}

/// `pull-rebase-dirty`: `p r r` over three edited files, one staged —
/// rebased onto the teammate's commit, the edits back, no stash left. Writes
/// the preview captures (120x40, 80x24) and the done capture.
#[test]
fn a_dirty_pull_rebase_is_three_keys() {
    let remote = local();
    let work = remote.work();
    let (files, theirs) = diverged_with_three_edits(&remote);
    let (app, state) = App::open(work);
    let state = app.resize(&state, 120, 40);

    let mut keys = counted(&app);
    let state = pull_refused(&mut keys, state);
    let state = rebase_preview(&mut keys, state);
    preview_shots(&app, &state);
    let state = keys.press(&state, KeyCode::Char('r'));
    let state = app.settle_write(state, "the rebase");
    let state = crate::captures::settled_frame(&app, quiet(&app, ready(&app, state)));

    assert!(
        keys.count <= allowed(PULL_REBASE),
        "{} keys, the note allows {}",
        keys.count,
        allowed(PULL_REBASE)
    );
    assert_eq!(rev(work, "HEAD~1"), theirs, "rebased onto theirs");
    for name in &files {
        assert_eq!(read(work, name), EDIT, "{name} is back as he left it");
    }
    assert!(stash_list(work).is_empty(), "no stash left behind");
    assert!(
        matches!(&state.status, Status::Done { what, also: Some(clause), keys: None }
            // The op's own done line, as a clean-tree rebase words it; the
            // note's example says `rebased`, the op has always said `rebase`.
            if what == "rebase main onto origin/main" && clause == "your edits are back"),
        "{:?}",
        state.status
    );
    done_shot(&state, &rev(work, "HEAD"));
    drop(state);
    app.close();
}

/// `rejected-push-dirty-to-pushed`: from a rejected `P` (not counted: the
/// journey starts at the rejection), `p r r P y` — his commit on the
/// teammate's at `origin/main`, the edits still in the tree.
#[test]
fn a_rejected_push_with_edits_is_five_keys() {
    let remote = local();
    let work = remote.work();
    let (files, theirs) = diverged_with_three_edits(&remote);
    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('P'));
    let state = confirm(&app, &state);
    let state = finish(&app, state, "the rejected push");

    let mut keys = counted(&app);
    let state = pull_refused(&mut keys, state);
    let state = rebase_preview(&mut keys, state);
    let state = keys.press(&state, KeyCode::Char('r'));
    let state = app.settle_write(state, "the rebase");
    let state = ready(&app, state);
    let state = keys.press(&state, KeyCode::Char('P'));
    assert!(state.confirm.is_some(), "P opened the plain box");
    let state = keys.press(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the push");

    assert!(
        keys.count <= allowed(REJECTED_PUSH),
        "{} keys, the note allows {}",
        keys.count,
        allowed(REJECTED_PUSH)
    );
    let pushed = run(&remote, remote.bare(), &["rev-parse", "main"]);
    assert_eq!(pushed, rev(work, "HEAD"), "his commit is on the remote");
    assert_eq!(
        run(&remote, remote.bare(), &["rev-parse", "main~1"]),
        theirs,
        "on top of the teammate's"
    );
    for name in &files {
        assert_eq!(read(work, name), EDIT, "{name} is still edited");
    }
    assert!(stash_list(work).is_empty(), "no stash left behind");
    drop(state);
    app.close();
}

/// A file both `main` and `fix` hold with the same contents, other than
/// `base.txt`: one of the Trivial shape's own.
fn shared_file(path: &Path) -> String {
    let on_fix = git(path, &["ls-tree", "--name-only", "fix"]);
    git(path, &["ls-tree", "--name-only", "main"])
        .lines()
        .find(|name| *name != "base.txt" && on_fix.lines().any(|other| other == *name))
        .unwrap_or_else(|| panic!("main and fix share only base.txt"))
        .to_owned()
}

/// `main_and_fix`, two files both branches hold edited: `base.txt` staged,
/// the other not, so the split is part of what comes back.
fn main_and_fix_dirty() -> (graft_fixtures::Scratch, String) {
    let scratch = main_and_fix();
    let path = scratch.path();
    let other = shared_file(path);
    std::fs::write(path.join("base.txt"), EDIT).expect("the scratch repo is writable");
    std::fs::write(path.join(&other), EDIT).expect("the scratch repo is writable");
    git(path, &["add", "--", "base.txt"]);
    (scratch, other)
}

/// `. ⏎ y` on `fix`'s row, main dirty. Three keys, counted.
fn switch_to_fix(keys: &mut Keys<'_>, path: &Path, state: AppState) -> AppState {
    let state = cursor_to(keys.app, opened(keys.app, state), &rev(path, "fix"));
    let state = keys.press(&state, KeyCode::Char('.'));
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(keys.app, state);
    keys.press(&state, KeyCode::Char('y'))
}

/// `switch-dirty`: `. ⏎ y` on a branch row — on the branch, the edits in
/// the tree.
#[test]
fn a_dirty_switch_is_three_keys() {
    let (scratch, other) = main_and_fix_dirty();
    let path = scratch.path();
    let (app, state) = App::open(path);

    let mut keys = counted(&app);
    let state = switch_to_fix(&mut keys, path, state);
    let state = app.settle_write(state, "the switch");

    assert!(keys.count <= allowed(SWITCH), "{} keys", keys.count);
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(read(path, &other), EDIT);
    assert!(stash_list(path).is_empty(), "no stash left behind");
    assert!(
        matches!(&state.status, Status::Done { also: Some(clause), keys: None, .. } if clause == "your edits are back"),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
}

/// `u_after_a_dirty_switch_puts_everything_back`: `. ⏎ y u` — on main
/// again, the index and the tree as before, staged split included, the
/// stash list as before.
#[test]
fn u_after_a_dirty_switch_puts_everything_back() {
    let (scratch, other) = main_and_fix_dirty();
    let path = scratch.path();
    let staged = git(path, &["diff", "--cached", "--name-only"]);
    let unstaged = git(path, &["diff", "--name-only"]);
    let stashes = stash_list(path);
    let (app, state) = App::open(path);

    let mut keys = counted(&app);
    let state = switch_to_fix(&mut keys, path, state);
    let state = app.settle_write(state, "the switch");
    let state = quiet(&app, state);
    let before_undo = keys.count;
    let state = keys.press(&state, KeyCode::Char('u'));
    assert!(
        state.confirm.is_none(),
        "`u` asks nothing: {:?}",
        state.status
    );
    let state = app.settle_write(state, "the undo");

    assert!(
        keys.count - before_undo <= UNDO_KEYS && keys.count <= allowed(SWITCH) + UNDO_KEYS,
        "{} keys",
        keys.count
    );
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    assert_eq!(git(path, &["diff", "--cached", "--name-only"]), staged);
    assert_eq!(git(path, &["diff", "--name-only"]), unstaged);
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(read(path, &other), EDIT);
    assert_eq!(stash_list(path), stashes, "the stash list as before");
    drop(state);
    app.close();
}

/// `main_and_fix`, then `main` changes `base.txt` and `fix` is checked out
/// with `base.txt` edited: switching to `main` sets the edit aside, and
/// putting it back clashes in `base.txt`, one block.
fn fix_dirty_over_mains_base() -> graft_fixtures::Scratch {
    let scratch = main_and_fix();
    let path = scratch.path();
    dated_commit(
        path,
        "base.txt",
        "main's base\n",
        "main changes the base",
        4,
    );
    git(path, &["switch", "-q", "fix"]);
    std::fs::write(path.join("base.txt"), EDIT).expect("the scratch repo is writable");
    scratch
}

/// `switch-dirty-clash`: `. ⏎ y o ⏎` on main's row — on main, `base.txt`
/// holds yours (main's), the set-aside dropped. Writes the landing capture.
#[test]
fn a_dirty_switch_that_clashes_is_five_keys() {
    let scratch = fix_dirty_over_mains_base();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = cursor_to(&app, opened(&app, state), &rev(path, "main"));

    let mut keys = counted(&app);
    let state = keys.press(&state, KeyCode::Char('.'));
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(&app, state);
    let state = keys.press(&state, KeyCode::Char('y'));
    let state = crate::captures::settled_frame(&app, landed(&app, state));
    assert_eq!(
        state.resolver.as_ref().map(|open| open.path.as_path()),
        Some(Path::new("base.txt")),
        "the put-back's clash lands in the resolver on the clashed file"
    );
    landing_shot(&state);

    let state = keys.press(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage, the drop and their reads", |state| {
        state.resolver.is_none()
            && state.operating.is_none()
            && matches!(&state.status, Status::Done { what, .. } if what.contains("set-aside copy dropped"))
    });

    assert!(keys.count <= allowed(SWITCH_CLASH), "{} keys", keys.count);
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    assert_eq!(read(path, "base.txt"), "main's base\n", "branch taken");
    assert_eq!(unmerged(path), "");
    assert!(stash_list(path).is_empty(), "the set-aside is dropped");
    assert_eq!(
        state.status,
        Status::Done {
            what: "every file done \u{b7} set-aside copy dropped".to_owned(),
            also: None,
            keys: Some("u brings it back".to_owned()),
        }
    );

    // Review m6-final-fixes-74 M3: what `u brings it back` promises — the
    // dropped copy listed again, his edits one `S` away. Not counted: the
    // journey ends at the drop.
    let state = quiet(&app, state);
    let state = app.key(&state, KeyCode::Char('u'));
    assert!(
        state.confirm.is_none(),
        "`u` asks nothing: {:?}",
        state.status
    );
    let state = app.settle_write(state, "the undo of the drop");
    assert_eq!(
        stash_list(path).len(),
        1,
        "`u` brings the set-aside copy back: {:?}",
        stash_list(path)
    );
    drop(state);
    app.close();
}

/// Review m6-final-fixes-74 M3: `. ⏎ y t ⏎` — the same clash, `t yours`
/// keeps his edit: on main, `base.txt` holds his edit, the copy dropped.
/// Same count as taking the branch.
#[test]
fn t_in_a_set_aside_clash_keeps_your_edits() {
    let scratch = fix_dirty_over_mains_base();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = cursor_to(&app, opened(&app, state), &rev(path, "main"));

    let mut keys = counted(&app);
    let state = keys.press(&state, KeyCode::Char('.'));
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(&app, state);
    let state = keys.press(&state, KeyCode::Char('y'));
    let state = crate::captures::settled_frame(&app, landed(&app, state));
    assert_eq!(
        state.resolver.as_ref().map(|open| open.path.as_path()),
        Some(Path::new("base.txt")),
        "the put-back's clash lands in the resolver on the clashed file"
    );
    let state = keys.press(&state, KeyCode::Char('t'));
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage, the drop and their reads", |state| {
        state.resolver.is_none()
            && state.operating.is_none()
            && matches!(&state.status, Status::Done { what, .. } if what.contains("set-aside copy dropped"))
    });

    assert!(keys.count <= allowed(SWITCH_CLASH), "{} keys", keys.count);
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    assert_eq!(read(path, "base.txt"), EDIT, "yours (his edit) taken");
    assert_eq!(unmerged(path), "");
    assert!(stash_list(path).is_empty(), "the set-aside is dropped");
    assert_eq!(
        state.status,
        Status::Done {
            what: "every file done \u{b7} set-aside copy dropped".to_owned(),
            also: None,
            keys: Some("u brings it back".to_owned()),
        }
    );
    drop(state);
    app.close();
}

/// Review m6-final-fixes-74 m1: `. ⏎ y` into the clash, then `esc u` (no
/// take, so no drift box): back on `fix`, the index and the tree as before
/// the switch, staged split included, the stash list as before.
#[test]
fn esc_u_undoes_a_clashed_switch() {
    let scratch = fix_dirty_over_mains_base();
    let path = scratch.path();
    // A staged edit too, so the split is part of what `u` must restore.
    let other = shared_file(path);
    std::fs::write(path.join(&other), EDIT).expect("the scratch repo is writable");
    git(path, &["add", "--", &other]);
    let staged = git(path, &["diff", "--cached", "--name-only"]);
    let unstaged = git(path, &["diff", "--name-only"]);
    let stashes = stash_list(path);
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = cursor_to(&app, opened(&app, state), &rev(path, "main"));

    let state = app.key(&state, KeyCode::Char('.'));
    let state = app.key(&state, KeyCode::Enter);
    let state = planned(&app, state);
    let state = app.key(&state, KeyCode::Char('y'));
    let state = crate::captures::settled_frame(&app, landed(&app, state));
    assert!(state.resolver.is_some(), "landed in the clash");

    let mut keys = counted(&app);
    let state = keys.press(&state, KeyCode::Escape);
    let state = quiet(&app, state);
    let state = keys.press(&state, KeyCode::Char('u'));
    assert!(
        state.confirm.is_none(),
        "`u` asks nothing: {:?}",
        state.status
    );
    let state = app.settle_write(state, "the undo");

    assert_eq!(keys.count, 2, "esc u");
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    assert_eq!(git(path, &["diff", "--cached", "--name-only"]), staged);
    assert_eq!(git(path, &["diff", "--name-only"]), unstaged);
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(read(path, &other), EDIT);
    assert_eq!(unmerged(path), "");
    assert_eq!(stash_list(path), stashes, "the stash list as before");
    drop(state);
    app.close();
}

/// Review m6-final-fixes-74 M2: `. ⏎` on main's row over the clash fixture
/// — the box warns before the switch that putting the edit back may clash.
/// Writes the preview capture.
#[test]
fn the_preview_warns_before_a_switch_that_clashes() {
    let scratch = fix_dirty_over_mains_base();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = cursor_to(&app, opened(&app, state), &rev(path, "main"));
    let state = app.key(&state, KeyCode::Char('.'));
    let state = app.key(&state, KeyCode::Enter);
    let state = planned(&app, state);

    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    let text = drawn.join("\n");
    assert!(
        text.contains("your edits may clash when put back: base.txt"),
        "{text}"
    );
    assert!(
        text.contains("your edits: 1 file set aside, put back after"),
        "{text}"
    );
    assert!(!text.contains("no conflicts expected"), "{text}");
    check_shot(
        "m6/set-aside-clash-preview-120x40.txt",
        &format!("{CLASH_PREVIEW_HEADER}\n{text}\n"),
    );
    drop(state);
    app.close();
}

/// `f.txt` and `g.txt` pushed as `base`; a teammate's commit changing both
/// (its id returned); a local `ours` changing F and adding `ours.txt` (so
/// taking upstream in F leaves the commit something); G edited in the tree. `p r r` stops on F; once the
/// rebase is continued, putting G back clashes with theirs.
fn diverged_on_f_with_g_edited(remote: &LocalRemote) -> (String, String, String) {
    let work = remote.work();
    let (f, g) = ("f.txt".to_owned(), "g.txt".to_owned());
    let write = |name: &str, text: &str| {
        std::fs::write(work.join(name), text).expect("the working clone is writable");
    };
    write(&f, "base\n");
    write(&g, "base\n");
    run(remote, work, &["add", "--", &f, &g]);
    run(remote, work, &["commit", "-q", "-m", "f and g"]);
    run(
        remote,
        work,
        &["push", "-q", "origin", "HEAD:refs/heads/main"],
    );
    write(&f, "theirs\n");
    write(&g, "theirs\n");
    run(remote, work, &["commit", "-q", "-am", "theirs"]);
    run(
        remote,
        work,
        &["push", "-q", "origin", "HEAD:refs/heads/main"],
    );
    let theirs = run(remote, work, &["rev-parse", "HEAD"]);
    run(remote, work, &["reset", "-q", "--hard", "HEAD~1"]);
    write(&f, "ours\n");
    write("ours.txt", "ours\n");
    run(remote, work, &["add", "--", &f, "ours.txt"]);
    run(remote, work, &["commit", "-q", "-m", "ours"]);
    write(&g, EDIT);
    (f, g, theirs)
}

/// The resolver open on a file, its first read in, nothing running.
fn resolver_on(app: &App, state: AppState, name: &str) -> AppState {
    let state = app.pump(state, "the resolver on a file", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some())
    });
    assert_eq!(
        state.resolver.as_ref().map(|open| open.path.as_path()),
        Some(Path::new(name)),
        "{}",
        frame_at(&state, 120, 40, None).join("\n")
    );
    crate::resolver::settled(app, state)
}

/// `pull-rebase-stop-then-clash`: `p r r o ⏎ c o ⏎` — the rebase stops on
/// F, `o ⏎` settles it, `c` continues, putting G back clashes, `o ⏎` takes
/// yours: rebased, G holds theirs, no stash left. `c` reaches
/// `banner::resumed`'s `Operated::Clashed` arm (review 72+72b m8).
#[test]
fn a_stopped_rebase_then_a_clash_is_eight_keys() {
    let remote = local();
    let work = remote.work();
    let (f, g, theirs) = diverged_on_f_with_g_edited(&remote);
    let (app, state) = App::open(work);
    let state = app.resize(&state, 120, 40);

    let mut keys = counted(&app);
    let state = pull_refused(&mut keys, state);
    let state = rebase_preview(&mut keys, state);
    let state = keys.press(&state, KeyCode::Char('r'));
    let state = resolver_on(&app, state, &f);

    let state = keys.press(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage and its reads", |state| {
        state.operating.is_none() && state.status_read.is_none()
    });
    // The stage re-reads the file; `c` waits for that read, as he would.
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Char('c'));
    assert!(
        state.operating.is_some(),
        "`c` continued nothing: {:?}\n{}",
        state.status,
        frame_at(&state, 120, 40, None).join("\n")
    );
    let state = crate::captures::settled_frame(&app, landed(&app, state));
    assert_eq!(
        state.resolver.as_ref().map(|open| open.path.as_path()),
        Some(Path::new(g.as_str())),
        "the put-back's clash lands on G"
    );
    let state = keys.press(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage, the drop and their reads", |state| {
        state.resolver.is_none()
            && state.operating.is_none()
            && matches!(&state.status, Status::Done { what, .. } if what.contains("set-aside copy dropped"))
    });

    assert!(
        keys.count <= allowed(STOP_THEN_CLASH),
        "{} keys",
        keys.count
    );
    assert_eq!(rev(work, "HEAD~1"), theirs, "rebased onto theirs");
    assert!(
        !work.join(".git/rebase-merge").exists(),
        "the rebase is done"
    );
    assert_eq!(read(work, &g), "theirs\n", "yours (HEAD's) taken for G");
    assert_eq!(unmerged(work), "");
    assert!(stash_list(work).is_empty(), "the set-aside is dropped");
    drop(state);
    app.close();
}

/// Review m6-final-fixes-74 m2: `p r r` to the stop on F, then `esc a y` —
/// `esc` leaves the resolver for the banner, `a` asks, `y` aborts. The
/// branch back on its own tip, G's edit in the tree, no stash left, and the
/// line `aborted: rebase … ✓ · your edits are back`.
#[test]
fn aborting_a_stopped_rebase_puts_the_edits_back() {
    let remote = local();
    let work = remote.work();
    let (f, g, _) = diverged_on_f_with_g_edited(&remote);
    let tip = rev(work, "HEAD");
    let (app, state) = App::open(work);
    let state = app.resize(&state, 120, 40);

    let mut keys = counted(&app);
    let state = pull_refused(&mut keys, state);
    let state = rebase_preview(&mut keys, state);
    let state = keys.press(&state, KeyCode::Char('r'));
    let state = resolver_on(&app, state, &f);

    let state = quiet(&app, app.key(&state, KeyCode::Escape));
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(state.confirm.is_some(), "`a` asks: {:?}", state.status);
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the abort and the put-back");

    assert_eq!(rev(work, "HEAD"), tip, "the branch on its own tip again");
    assert!(
        !work.join(".git/rebase-merge").exists(),
        "no rebase left open"
    );
    assert_eq!(read(work, &g), EDIT, "G's edit is back");
    assert_eq!(unmerged(work), "");
    assert!(stash_list(work).is_empty(), "no stash left behind");
    assert_eq!(
        state.status,
        Status::Done {
            what: "aborted: rebase main onto origin/main".to_owned(),
            also: Some("your edits are back".to_owned()),
            keys: None,
        }
    );
    drop(state);
    app.close();
}

/// Touches (ticket 74): a rebase stopped with the edits aside. With the
/// resolver shut the stop line ends `· edits aside`; `S` lists the
/// set-aside as `graft: your edits, set aside for …`; `J` names the entry as
/// the rebase, not as a stash.
#[test]
fn a_stop_with_edits_aside_shows_in_the_stop_line_s_and_j() {
    let remote = local();
    let work = remote.work();
    let (f, _, _) = diverged_on_f_with_g_edited(&remote);
    let (app, state) = App::open(work);
    let state = app.resize(&state, 120, 40);
    let mut keys = counted(&app);
    let state = pull_refused(&mut keys, state);
    let state = rebase_preview(&mut keys, state);
    let state = keys.press(&state, KeyCode::Char('r'));
    let state = resolver_on(&app, state, &f);

    let state = quiet(&app, app.key(&state, KeyCode::Escape));
    // At 120 columns the banner's seven keys leave no room for the part, so
    // the ladder drops it first, as the note orders; at 187 it shows.
    for (columns, rows, shown) in [(187, 50, true), (120, 40, false)] {
        let drawn = frame_at(&state, columns, rows, None);
        let stop = drawn
            .iter()
            .find(|row| row.contains("rebase stopped at step 1 of 1 \u{b7} 1 "))
            .unwrap_or_else(|| panic!("no stop line:\n{}", drawn.join("\n")));
        assert_eq!(
            stop.contains("conflicted \u{b7} edits aside \u{b7}"),
            shown,
            "{columns}: {stop}"
        );
    }

    // `⇥ graph`: the banner has the keys, and `S` is the graph's.
    let state = app.key(&state, KeyCode::Tab);
    let stash = crate::stash_clash::overlay_open(&app, state);
    let drawn = frame_at(&stash, 120, 40, None).join("\n");
    assert!(
        drawn.contains("graft: your edits, set aside for"),
        "{drawn}"
    );

    let state = app.key(&stash, KeyCode::Escape);
    let state = app.key(&state, KeyCode::Char('J'));
    let state = app.pump(state, "the journal browser's list", |state| {
        state.journal.reading.is_none()
    });
    let drawn = frame_at(&state, 120, 40, None);
    // The graph's stash row says `set aside for rebase …` too; the
    // browser's row is the one without it.
    assert!(
        drawn
            .iter()
            .any(|row| row.contains("rebase main onto origin/main")
                && !row.contains("set aside")
                && !row.contains("stash")),
        "{}",
        drawn.join("\n")
    );
    drop(state);
    app.close();
}

/// The rebase preview at 120x40 and 80x24, the row asserted, then the
/// captures.
fn preview_shots(app: &App, state: &AppState) {
    for (columns, rows, name, row) in [
        (
            120,
            40,
            "m6/set-aside-preview-120x40.txt",
            "your edits: 3 files set aside, put back after \u{b7} 1 staged comes back unstaged",
        ),
        // Review m6-final-fixes-74 m5: at 80 the box's 76 inner columns
        // still hold `set_aside_row`'s longest rung (76 cells), so the row is
        // whole there too; the exact line is asserted, not a prefix.
        (
            80,
            24,
            "m6/set-aside-preview-80x24.txt",
            "your edits: 3 files set aside, put back after \u{b7} 1 staged comes back unstaged",
        ),
    ] {
        let sized = app.resize(state, columns, rows);
        let drawn = frame_at(&sized, columns, rows, Some(Clock::Fixed(FORK_NOW)));
        assert!(
            drawn
                .iter()
                .any(|line| line.trim_matches(|c| c == '\u{2502}' || c == ' ') == row),
            "{name}:\n{}",
            drawn.join("\n")
        );
        check_shot(name, &format!("{PREVIEW_HEADER}\n{}\n", drawn.join("\n")));
    }
}

fn landing_shot(state: &AppState) {
    let drawn = frame_at(state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    let text = drawn.join("\n");
    for words in [
        "branch \u{b7} main",
        "yours \u{b7} edits set aside",
        "your edits clashed in 1 file \u{b7} esc, u undoes the switch too",
        "o branch",
        "t yours",
    ] {
        assert!(text.contains(words), "no `{words}`:\n{text}");
    }
    assert!(
        !text.contains("set aside \u{b7}"),
        "`set aside` is no pane's name:\n{text}"
    );
    check_shot(
        "m6/set-aside-clash-landing-120x40.txt",
        &format!("{LANDING_HEADER}\n{text}\n"),
    );
}

fn done_shot(state: &AppState, rebased: &str) {
    let drawn = frame_at(state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    assert!(
        drawn
            .iter()
            .any(|row| row.trim_end().ends_with("\u{b7} your edits are back")),
        "{}",
        drawn.join("\n")
    );
    check_shot(
        "m6/set-aside-done-120x40.txt",
        &format!(
            "{DONE_HEADER}\n{}\n",
            without_invented_id(&drawn.join("\n"), rebased)
        ),
    );
}

const PREVIEW_HEADER: &str = "\
# m6/set-aside-preview-<size>.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/set_aside.rs,
# a_dirty_pull_rebase_is_three_keys, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: `p` refused as diverged, then `r`, three files
# edited and one of them staged.
#  - The rebase preview box, `[ Confirm ]` offered.
#  - Under `working tree:` the row `your edits: 3 files set aside, put back
#    after · 1 staged comes back unstaged`. At 80 columns the staged clause
#    may be cut; the file count may not.
";

const LANDING_HEADER: &str = "\
# m6/set-aside-clash-landing-120x40.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/set_aside.rs,
# a_dirty_switch_that_clashes_is_five_keys, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: `. ⏎ y` on main's row with `base.txt` edited;
# main changed it too, so putting the edit back clashes.
#  - The resolver open on base.txt; panes `branch · main` and
#    `yours · edits set aside`; `set aside ·` is no pane's name.
#  - The notice `your edits clashed in 1 file · esc, u undoes the switch too`.
#  - The bar offers `o branch` and `t yours`; no git error text anywhere.
";

const CLASH_PREVIEW_HEADER: &str = "\
# m6/set-aside-clash-preview-120x40.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/set_aside.rs,
# the_preview_warns_before_a_switch_that_clashes, asserted). Not a pty
# capture: a character grid with no colour (CLAUDE.md rule 5, colour
# unverified).
#
# What right looks like: `. ⏎` on main's row with `base.txt` edited; main
# changed it too, so putting the edit back would clash.
#  - The checkout preview box, `[ Confirm ]` offered.
#  - Under `working tree:` the row `your edits: 1 file set aside, put back
#    after`.
#  - The conflict line `your edits may clash when put back: base.txt`; no
#    `no conflicts expected` anywhere in the box.
";

const DONE_HEADER: &str = "\
# m6/set-aside-done-120x40.txt: 120x40, drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/set_aside.rs,
# a_dirty_pull_rebase_is_three_keys, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
# The rebased commit is made moments before the frame, so its id is new
# every run: it is zeroed to `0000000` here. Nothing else is normalised.
# Its date reads `futu…`: it is dated by the wall clock, the frame by the
# fixtures' fixed clock (so the other rows' ages hold still), which is
# earlier.
#
# What right looks like: `p r r` over three edited files, the rebase clean.
#  - The graph on HEAD, the rebased `ours` on top of the teammate's commit.
#  - The status row reads
#    `rebase main onto origin/main ✓ · your edits are back` (the op's own
#    done line, as a clean-tree rebase words it, then the clause).
";
