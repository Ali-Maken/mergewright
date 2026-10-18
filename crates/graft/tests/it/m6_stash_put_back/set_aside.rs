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
/// three edited with one of them staged (a staged modification, which the
/// put-back keeps staged, as `git switch` would): the state `p` finds
/// diverged and dirty.
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
    let state = crate::captures::at_rest(&app, state);
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
        matches!(&state.status, Status::Done { what, also: Some(clause), keys: Some(keys) } if what == "on fix" && clause == "your edits are back" && keys == "u goes back"),
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

/// m8-polish-and-ship/19d: `m y` on `fix`'s row over a staged change to a
/// file `fix` doesn't touch — the case git refuses a true merge over. The
/// preview draws the set-aside row and the conflict line with the put-back
/// folded in, in the words a rebase's preview uses; `y` merges, the edit is
/// back staged, the status says `your edits are back`.
#[test]
fn merge_over_staged_edits_previews_and_puts_back() {
    let scratch = main_and_fix();
    let path = scratch.path();
    std::fs::write(path.join("base.txt"), EDIT).expect("the scratch repo is writable");
    git(path, &["add", "--", "base.txt"]);
    let main_before = rev(path, "main");
    let fix = rev(path, "fix");
    let (app, state) = App::open(path);
    let state = cursor_to(&app, opened(&app, state), &fix);

    let state = app.key(&state, KeyCode::Char('m'));
    let state = planned(&app, state);

    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW))).join("\n");
    assert!(drawn.contains("Merge fix into main"), "{drawn}");
    for row in [
        "your edits: 1 file set aside, put back after",
        "You decide    nothing: no file clashes",
    ] {
        assert!(drawn.contains(row), "`{row}`:\n{drawn}");
    }

    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the merge");

    let parents = git(path, &["log", "-1", "--format=%P", "main"]);
    assert_eq!(
        parents.split_whitespace().collect::<Vec<_>>(),
        [main_before.as_str(), fix.as_str()],
        "a merge commit of main and fix"
    );
    assert_eq!(read(path, "base.txt"), EDIT, "the edit is back");
    assert_eq!(
        git(path, &["status", "--porcelain"]).trim_end(),
        "M  base.txt",
        "still staged"
    );
    assert!(stash_list(path).is_empty(), "no stash left behind");
    assert!(
        matches!(&state.status, Status::Done { also: Some(clause), .. } if clause == "your edits are back"),
        "{:?}",
        state.status
    );
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
/// set-aside as `mergewright: your edits, set aside for …`; `J` names the entry as
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
        drawn.contains("mergewright: your edits, set aside for"),
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
            "your edits: 3 files set aside, put back after",
        ),
        // At 80 the box's 76 inner columns hold `set_aside_row`'s longest
        // rung whole; the exact line is asserted, not a prefix. No staging
        // words: the put-back keeps the staging (ADR-0045 amendment b, 30b).
        (
            80,
            24,
            "m6/set-aside-preview-80x24.txt",
            "your edits: 3 files set aside, put back after",
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
#    after`, with no staging words: the staging comes back as he left it.
#    At narrow widths `put back after` may be cut; the file count may not.
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

/// `switch-staging-not-kept` (ticket 30b's Journeys table): `. ⏎ y`, 3
/// keys. Not a `journey-counts` row of the m6 note, so its count is here.
const SWITCH_STAGING_NOT_KEPT_KEYS: usize = 3;

/// Twenty lines, `line N`, with line `n` replaced by `with`.
fn twenty_lines(changed: &[(usize, &str)]) -> String {
    (1..=20)
        .map(|n| {
            changed
                .iter()
                .find(|(at, _)| *at == n)
                .map_or_else(|| format!("line {n}\n"), |(_, with)| format!("{with}\n"))
        })
        .collect()
}

/// Ticket 30 P3's shape: on `main`, a 20-line `f.txt` with line 5 edited
/// and staged; `fix` changed line 7. `git switch fix` refuses; Graft's
/// `stash apply --index` cannot reinstate the index on `fix`'s tree, so the
/// put-back falls back to a plain apply, which lands clean.
fn staging_that_cannot_come_back() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    // The stash the app makes (here and in the killed child, which inherits
    // the test's HOME) is signed by the repo's identity, not his.
    crate::app_harness::pin_identity(path);
    dated_commit(path, "f.txt", &twenty_lines(&[]), "add f", 1);
    git(path, &["branch", "fix"]);
    git(path, &["switch", "-q", "fix"]);
    dated_commit(
        path,
        "f.txt",
        &twenty_lines(&[(7, "seven on fix")]),
        "fix changes line 7",
        2,
    );
    git(path, &["switch", "-q", "main"]);
    std::fs::write(path.join("f.txt"), twenty_lines(&[(5, "five mine")]))
        .expect("the scratch repo is writable");
    git(path, &["add", "--", "f.txt"]);
    scratch
}

/// Ticket 30b: the preview says nothing about staging; after `y` the
/// sticky line names the file and `u`, never `could not confirm`; it
/// survives a key; `u` puts the split back exactly. Walked at 187x50 and
/// 80x24, each writing its capture.
#[test]
fn a_switch_whose_staging_cannot_come_back_says_so() {
    for (columns, rows) in [(187u16, 50u16), (80, 24)] {
        let scratch = staging_that_cannot_come_back();
        let path = scratch.path();
        let before = SplitState::of(path);
        assert_eq!(before.status, "M  f.txt\n");
        let (app, state) = App::open(path);
        let state = app.resize(&state, columns, rows);

        let mut keys = counted(&app);
        let state = switched_over_the_fallback(&mut keys, path, state, (columns, rows));
        assert_eq!(keys.count, SWITCH_STAGING_NOT_KEPT_KEYS, "{columns}");
        let line = fallback_line_shot(&state, columns, rows);

        // Sticky: one more key, and the line is still there.
        let state = app.key(&state, KeyCode::Char('j'));
        assert_eq!(staging_line(&state, columns, rows), line, "{columns}");

        // Git: on `fix`, both lines in, unstaged, no stash left.
        assert_eq!(
            git(path, &["symbolic-ref", "HEAD"]).trim(),
            "refs/heads/fix"
        );
        assert_eq!(git(path, &["status", "--porcelain"]), " M f.txt\n");
        assert_eq!(
            read(path, "f.txt"),
            twenty_lines(&[(5, "five mine"), (7, "seven on fix")])
        );
        assert!(stash_list(path).is_empty(), "no stash left behind");

        // `u`, one key: back on `main`, the split exactly as before.
        let before_undo = keys.count;
        let state = keys.press(&state, KeyCode::Char('u'));
        let state = quiet(&app, app.settle_write(state, "the undo"));
        assert_eq!(keys.count - before_undo, UNDO_KEYS);
        assert_eq!(
            git(path, &["symbolic-ref", "HEAD"]).trim(),
            "refs/heads/main"
        );
        assert_eq!(SplitState::of(path), before, "index and tree as before");
        assert!(stash_list(path).is_empty(), "the stash list as before");
        drop(state);
        app.close();
    }
}

/// The index and the tree, byte for byte, as `u` must restore them.
#[derive(Debug, PartialEq, Eq)]
struct SplitState {
    status: String,
    cached: String,
    unstaged: String,
    index: String,
    bytes: String,
}

impl SplitState {
    fn of(path: &Path) -> Self {
        Self {
            status: git(path, &["status", "--porcelain"]),
            cached: git(path, &["diff", "--cached"]),
            unstaged: git(path, &["diff"]),
            index: git(path, &["ls-files", "-s"]),
            bytes: read(path, "f.txt"),
        }
    }
}

/// `. ⏎ y` on `fix`, counted: the preview's row has no staging words, and
/// the answer is in.
fn switched_over_the_fallback(
    keys: &mut Keys<'_>,
    path: &Path,
    state: AppState,
    (columns, rows): (u16, u16),
) -> AppState {
    let app = keys.app;
    let state = cursor_to(app, opened(app, state), &rev(path, "fix"));
    let state = keys.press(&state, KeyCode::Char('.'));
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(app, state);
    let preview = frame_at(&state, columns, rows, None).join("\n");
    assert!(
        preview.contains("your edits: 1 file set aside, put back after"),
        "{columns}: {preview}"
    );
    assert!(
        !preview.contains("staged") && !preview.contains("staging"),
        "{columns}: no staging words in the preview:\n{preview}"
    );
    let state = keys.press(&state, KeyCode::Char('y'));
    // Not `settle_write`: the answer is a sticky line, not a `Done`.
    let state = app.pump(state, "the switch", |state| state.operating.is_none());
    quiet(app, state)
}

/// The fallback's line: rung 1 at 187, rung 2 or 3 at 80, whole, never
/// `could not confirm`. Writes the capture; returns the line.
fn fallback_line_shot(state: &AppState, columns: u16, rows: u16) -> String {
    let line = staging_line(state, columns, rows);
    let rungs = [
        "on fix \u{2713} \u{b7} your edits are back \u{b7} staging not kept: f.txt \u{b7} u undoes it, staging included",
        "on fix \u{2713} \u{b7} staging not kept: f.txt \u{b7} u undoes it, staging included",
        "staging not kept: f.txt \u{b7} u undoes it, staging included",
    ];
    if columns >= 187 {
        assert_eq!(line, rungs[0], "{columns}");
    } else {
        assert!(rungs[1..].contains(&line.as_str()), "{columns}: {line:?}");
    }
    let drawn = frame_at(state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n");
    assert!(!drawn.contains("could not confirm"), "{drawn}");
    check_shot(
        &format!("m7/30b-staging-not-kept-{columns}x{rows}.txt"),
        &format!(
            "{}\n{drawn}\n",
            STAGING_NOT_KEPT_HEADER.replace("<size>", &format!("{columns}x{rows}"))
        ),
    );
    line
}

/// The bottom row holding the fallback's words, trimmed.
fn staging_line(state: &AppState, columns: u16, rows: u16) -> String {
    let drawn = frame_at(state, columns, rows, None);
    drawn
        .iter()
        .find(|row| row.contains("staging not kept"))
        .map_or_else(
            || panic!("{columns}: no staging line:\n{}", drawn.join("\n")),
            |row| row.trim().to_owned(),
        )
}

const STAGING_NOT_KEPT_HEADER: &str = "\
# m7/30b-staging-not-kept-<size>.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/set_aside.rs,
# a_switch_whose_staging_cannot_come_back_says_so, asserted). Not a pty
# capture: a character grid with no colour (CLAUDE.md rule 5, colour
# unverified).
#
# What right looks like: `. ⏎ y` on `fix` from `main`, `f.txt` line 5
# edited and staged, `fix` changed line 7. git could not put the staging
# back on `fix`'s tree, so the edits came back unstaged.
#  - The cursor on `fix`, now HEAD.
#  - The bottom row: `on fix ✓ · your edits are back · staging not
#    kept: f.txt · u undoes it, staging included` at 187; at 80 `your
#    edits are back` goes first, the file and `u` stay whole.
#  - Nowhere `could not confirm`.
";

/// The kill seam, the same script graft-ops' `set_aside_kill` runs: a `git`
/// first on a child's `PATH` that SIGKILLs the child on `stash drop`.
#[cfg(unix)]
const SHIM: &str = include_str!("../../../../graft-ops/tests/it/git_shim.sh");

#[cfg(unix)]
const KILL_CHILD_REPO: &str = "MWR_K4_CHILD_REPO";

/// K4's child: `git switch fix` from `main` over the staged edit, run the
/// way the app's job runs it (`jobs/operate.rs` `in_context`), under the
/// shim, which kills this process on `stash drop` — after the plain apply
/// brought the edits back unstaged. Does nothing when run on its own.
#[cfg(unix)]
#[test]
#[ignore = "the child of K4; runs only under killed_before_the_drop"]
fn child_switches_until_the_drop() {
    let Some(repo) = std::env::var_os(KILL_CHILD_REPO) else {
        return;
    };
    let handle = graft_git::repo::RepoHandle::open(Path::new(&repo)).expect("the repo opens");
    let journal_file = graft_ops::journal::journal_path(&handle).expect("the journal path");
    let mut journal = graft_ops::journal::Journal::open(&journal_file).expect("the journal opens");
    let git_version = graft_git::version::system_git()
        .expect("git is on PATH")
        .version();
    let mut ctx = graft_ops::apply::Context::open(
        &handle,
        git_version,
        &mut journal,
        FORK_NOW * 1000,
        [7; 10],
        &(),
    )
    .expect("the context opens");
    let op = graft_ops::operation::Operation::Checkout(graft_ops::operation::CheckoutParams {
        to: graft_ops::operation::CheckoutTarget::Branch("refs/heads/fix".to_owned()),
        head_before: graft_core::journal::HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    let ran = graft_ops::apply::apply(&mut ctx, &op);
    panic!("the shim should have killed this process: {ran:?}");
}

/// `staging_that_cannot_come_back`, switched to `fix` in a child that is
/// killed on `stash drop`: the edits back unstaged, `S` still listed, the
/// entry interrupted (graft-ops K2b's state).
#[cfg(unix)]
fn killed_before_the_drop(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::ExitStatusExt;

    let bin = path.join(".git/shim-bin");
    std::fs::create_dir_all(&bin).expect("the shim dir is made");
    let script = bin.join("git");
    std::fs::write(&script, SHIM).expect("the shim writes");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the shim is executable");
    let inherited = std::env::var_os("PATH").expect("PATH is set");
    let real_git = std::env::split_paths(&inherited)
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git is on PATH");
    let child_path =
        std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&inherited)))
            .expect("PATH joins");
    // The stash's dates are fixed so the capture's rows hold still.
    let when = format!("@{} +0000", crate::preview::FORK_EPOCH + 3 * 60);
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "m6_stash_put_back::set_aside::child_switches_until_the_drop",
            "--ignored",
            "--test-threads=1",
        ])
        .env("PATH", child_path)
        .env("MWR_SHIM_REAL_GIT", real_git)
        .env("MWR_SHIM_ON", "drop")
        .env("MWR_SHIM_DO", "kill")
        .env("GIT_AUTHOR_DATE", &when)
        .env("GIT_COMMITTER_DATE", &when)
        .env(KILL_CHILD_REPO, path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the child runs");
    assert_eq!(
        status.signal(),
        Some(9),
        "the shim killed the child: {status:?}"
    );
    std::fs::remove_dir_all(path.join(".git/shim-bin")).expect("the shim goes");
}

/// K4 (30c, review m7-30-06d Question 1): killed after the fallback put the
/// edits back unstaged and before the drop. The relaunch recovers the entry
/// and its line says where the edits are — back, with a copy listed — in
/// the status line's words, not ` · edits in stash@{0}` (which reads as
/// "they are only there"). `u`, one key, puts the switch, the split and the
/// stash list back. Walked at 120x40 and 80x24, each writing its capture.
#[cfg(unix)]
#[test]
fn a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back() {
    for (columns, rows) in [(120u16, 40u16), (80, 24)] {
        let scratch = staging_that_cannot_come_back();
        let path = scratch.path();
        let before = SplitState::of(path);
        killed_before_the_drop(path);
        assert_eq!(
            git(path, &["symbolic-ref", "HEAD"]).trim(),
            "refs/heads/fix"
        );
        assert_eq!(git(path, &["status", "--porcelain"]), " M f.txt\n");
        assert_eq!(stash_list(path).len(), 1, "S still listed");

        let (app, state) = App::open(path);
        let state = app.resize(&state, columns, rows);
        let state = crate::conflict_interim::launched(&app, state);
        let state = app.pump(state, "the refs the relaunch reads", |state: &AppState| {
            state.refs.is_some()
        });
        let state = crate::captures::settled_frame(&app, state);
        // The row marks (`ⓘ`) are the radar's own job's: wait for it too.
        let state = crate::captures::radar_answered(&app, state);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n");
        // 30d: the footer row is the ladder's rung for the width, whole —
        // rung 1 at 120, rung 4 at 80 (the row indents one cell).
        let footer = if columns >= 120 {
            " an operation was interrupted: checkout fix \u{b7} your edits are back \u{b7} a copy stays in stash@{0} \u{b7} J to inspect"
        } else {
            " interrupted \u{b7} your edits are back \u{b7} a copy stays in stash@{0} \u{b7} J to inspect"
        };
        let bottom = drawn.lines().last().expect("a bottom row");
        assert_eq!(bottom.trim_end(), footer, "{columns}x{rows}:\n{drawn}");
        assert!(
            !drawn.contains("edits in stash"),
            "{columns}x{rows}:\n{drawn}"
        );
        check_shot(
            &format!("m7/30c-relaunch-edits-back-{columns}x{rows}.txt"),
            &format!(
                "{}\n{drawn}\n",
                RELAUNCH_HEADER.replace("<size>", &format!("{columns}x{rows}"))
            ),
        );

        let mut keys = counted(&app);
        let state = keys.press(&state, KeyCode::Char('u'));
        let state = quiet(&app, app.settle_write(state, "the undo"));
        assert_eq!(keys.count, UNDO_KEYS);
        assert_eq!(
            git(path, &["symbolic-ref", "HEAD"]).trim(),
            "refs/heads/main"
        );
        assert_eq!(SplitState::of(path), before, "index and tree as before");
        assert!(stash_list(path).is_empty(), "the stash list as before");
        drop(state);
        app.close();
    }
}

/// K4, review m7-30c-to-10c Majors 1 and 2 (30f). At 160x50 with the detail
/// pane open (`⏎`) the footer is the graph half's row, not the frame's: the
/// notice is fitted to it and reads whole, `J to inspect` included. Then he
/// drops the copy in the shell and presses `Ctrl+R`: the notice no longer
/// names a stash that is gone.
#[cfg(unix)]
#[test]
fn the_relaunch_notice_fits_the_pane_and_follows_the_stash_list() {
    let (columns, rows) = (160u16, 50u16);
    let scratch = staging_that_cannot_come_back();
    let path = scratch.path();
    killed_before_the_drop(path);

    let (app, state) = App::open(path);
    let state = app.resize(&state, columns, rows);
    let state = crate::conflict_interim::launched(&app, state);
    let state = app.pump(state, "the refs the relaunch reads", |state: &AppState| {
        state.refs.is_some()
    });
    let state = crate::captures::settled_frame(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.detail.shown, "⏎ opens the detail pane");
    let state = crate::captures::settled_frame(&app, state);
    let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n");
    let bottom = drawn.lines().last().expect("a bottom row");
    assert!(
        bottom.contains("your edits are back \u{b7} a copy stays in stash@{0} \u{b7} J to inspect"),
        "the never-cut clause and J, whole, at {columns}x{rows} with the pane open:\n{drawn}"
    );
    assert!(!bottom.contains('\u{2026}'), "nothing cut:\n{drawn}");

    git(path, &["stash", "drop"]);
    let state = app.press(
        &state,
        graft_core::event::KeyPress {
            code: KeyCode::Char('r'),
            modifiers: graft_core::event::Modifiers::CONTROL,
        },
    );
    let state = app.pump(state, "the stash list re-read", |state: &AppState| {
        state
            .refs
            .as_deref()
            .is_some_and(|refs| refs.stashes().is_empty())
    });
    let state = crate::captures::settled_frame(&app, state);
    let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n");
    let bottom = drawn.lines().last().expect("a bottom row");
    assert!(
        bottom.contains("your edits are back \u{b7} J to inspect"),
        "{drawn}"
    );
    assert!(!drawn.contains("stash@"), "no stash that is gone:\n{drawn}");
    drop(state);
    app.close();
}

/// 30f: every width from 80 to 187, the detail pane shown and hidden, the
/// relaunch notice's never-cut clause and `J to inspect` are whole on the
/// bottom row — whichever row the footer is drawn in.
#[cfg(unix)]
#[test]
fn the_relaunch_notice_is_whole_at_every_width_pane_or_not() {
    const WHOLE: &str = "your edits are back \u{b7} a copy stays in stash@{0} \u{b7} J to inspect";
    let scratch = staging_that_cannot_come_back();
    let path = scratch.path();
    killed_before_the_drop(path);
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = crate::conflict_interim::launched(&app, state);
    let state = app.pump(state, "the refs the relaunch reads", |state: &AppState| {
        state.refs.is_some()
    });
    let hidden = crate::captures::settled_frame(&app, state);
    let shown = app.key(&hidden, KeyCode::Enter);
    assert!(shown.detail.shown, "⏎ opens the detail pane");
    let shown = crate::captures::settled_frame(&app, shown);
    for (pane, state) in [("hidden", &hidden), ("shown", &shown)] {
        for columns in 80u16..=187 {
            let drawn = frame_at(state, columns, 50, Some(Clock::Fixed(FORK_NOW)));
            // The footer is the graph area's last row: the frame's with
            // the pane beside or hidden, above the pane when it is below.
            let footer = drawn
                .iter()
                .find(|row| row.contains("interrupted"))
                .unwrap_or_else(|| panic!("{columns}, pane {pane}: no notice\n{drawn:#?}"));
            assert!(
                footer.contains(WHOLE),
                "{columns} columns, pane {pane}: {footer:?}"
            );
        }
    }
    drop((hidden, shown));
    app.close();
}

#[cfg(unix)]
const RELAUNCH_HEADER: &str = "\
# m7/30c-relaunch-edits-back-<size>.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/set_aside.rs,
# a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back,
# asserted). Not a pty capture: a character grid with no colour (CLAUDE.md
# rule 5, colour unverified).
#
# What right looks like: `f.txt` line 5 edited and staged on `main`, a
# switch to `fix` killed after the edits came back (unstaged) and before
# the stash was dropped; Graft launched again.
#  - HEAD on `fix`.
#  - At 120 the bottom row reads `an operation was interrupted: checkout
#    fix · your edits are back · a copy stays in stash@{0} · J to inspect`.
#    At 80 it reads, whole, `interrupted · your edits are back · a copy
#    stays in stash@{0} · J to inspect` (30d's ladder: the framing and
#    the description go, the clause and `J` stay).
#  - Nowhere `edits in stash@{0}` alone, which would say the edits are
#    only in the stash.
";
