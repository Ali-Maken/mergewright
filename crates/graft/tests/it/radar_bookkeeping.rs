//! m6-conflicts/66: the Mistake Radar ignores Graft's and git's own
//! bookkeeping. After a pull that rebases, the pre-rebase commits live on in
//! `refs/tributary/backup/**` (for `u`) and under a stash made on the old tip;
//! neither makes the rebased commits "duplicates", and the stash commit is
//! never called an evil merge.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::update::AppState;
use graft::update::detail::Loading;
use graft_fixtures::remote::LocalRemote;
use graft_ui::context_menu::ContextAction;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame, without_invented_id};
use crate::remote_by_key::{
    confirm, finish, local, plain, ready, rejected_then_diverged, rev, run,
};

/// The duplicate probe has answered for the repository as it is now.
fn duplicates_landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the duplicate probe", |state| {
        matches!(state.radar.duplicates, Loading::Ready(_))
    })
}

/// Moves the cursor with `step` (`j` or `k`) until it rests on `target`.
fn cursor_to(app: &App, mut state: AppState, step: char, target: &str) -> AppState {
    for _ in 0..40 {
        let on = state
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .map(|row| row.commit.id.to_string());
        if on.as_deref() == Some(target) {
            return state;
        }
        state = app.press(&state, plain(step));
    }
    panic!(
        "the cursor never reached {target}:\n{}",
        frame(&state, 120, 40).join("\n")
    );
}

/// A teammate pushed; a local commit `ours` and a stash made on it; then
/// `p` (refused as diverged), `r` `r` rebased through the app. Returns the
/// app, the stash's commit and the rebased tip.
fn stash_then_rebase(remote: &LocalRemote) -> (App, AppState, String, String) {
    let work = remote.work();
    remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let ours = remote.commit_locally("ours").expect("a local commit");
    // A stash on the pre-rebase tip: its base commit is `ours` before the
    // rebase, which `git log --all` walks.
    let tracked = git(work, &["ls-files"])
        .lines()
        .next()
        .expect("the fixture tracks a file")
        .to_owned();
    std::fs::write(work.join(&tracked), "work in progress\n").expect("a dirty edit");
    run(remote, work, &["stash", "push", "-m", "wip"]);
    let stash = rev(work, "refs/stash");
    let branch = remote.default_branch();

    let (app, state) = App::open(work);
    let state = rejected_then_diverged(&app, state, branch);
    let state = ready(&app, state);
    let state = app.press(&state, plain('r'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !matches!(preview.plan, Loading::Pending(_) | Loading::Idle))
    });
    let state = app.press(&state, plain('r'));
    let state = app.settle_write(state, "the rebase");
    let rebased = rev(work, "HEAD");
    assert_ne!(rebased, ours, "the rebase rewrote ours");
    // What the old `--all` walk found: the pre-rebase commit is still
    // reachable, but only from bookkeeping — no branch, remote or tag.
    let holders = git(
        work,
        &["for-each-ref", "--contains", &ours, "--format=%(refname)"],
    );
    assert!(
        holders
            .lines()
            .any(|name| name.starts_with("refs/tributary/backup/"))
            && holders.lines().any(|name| name == "refs/stash")
            && holders
                .lines()
                .all(|name| name.starts_with("refs/tributary/") || name == "refs/stash"),
        "ours lives on only in Graft's backups and the stash: {holders}"
    );
    (app, state, stash, rebased)
}

/// Regenerates `docs/shots/m6/fix-radar-bookkeeping-120x40.txt`.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it radar_bookkeeping`.
#[test]
fn a_pull_rebase_marks_no_duplicates() {
    let remote = local();
    let (app, state, stash, rebased) = stash_then_rebase(&remote);

    // `P`: the rebased work goes up, as it would next. Without it `main`'s
    // tip carries the radar's true "unpushed on the default branch" `ⓘ`,
    // which is not this ticket's.
    let state = ready(&app, state);
    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    let state = finish(&app, state, "the push");

    // No manual refresh: a history or remote write refreshes everything,
    // which puts the duplicate probe back to idle, so it is asked again.
    let state = ready(&app, state);
    let state = crate::context_menu::settled(&app, state);
    let state = duplicates_landed(&app, state);
    let state = crate::captures::at_rest(&app, state);
    assert!(
        state.radar.duplicate_findings.is_empty(),
        "no duplicate findings after a pull-rebase: {:?}",
        state.radar.duplicate_findings.keys().collect::<Vec<_>>()
    );
    let after_pull = frame(&state, 120, 40);
    assert!(
        !after_pull.iter().any(|line| line.contains('\u{24d8}')),
        "no \u{24d8} anywhere: ref findings {:?}, force-push {:?}\n{}",
        state.radar.ref_findings,
        state.radar.force_push_findings,
        after_pull.join("\n")
    );

    // The stash row: no evil probe, no `⚠`.
    let on_stash = cursor_to(&app, state, 'j', &stash);
    assert_eq!(on_stash.detail.evil_at, None, "a stash is never probed");
    let stash_frame = frame(&on_stash, 120, 40);
    assert!(
        !stash_frame.iter().any(|line| line.contains('\u{26a0}')),
        "no \u{26a0} on the stash:\n{}",
        stash_frame.join("\n")
    );

    // `.` on main's tip: the menu does not lead with `Explain`.
    let on_tip = cursor_to(&app, on_stash, 'k', &rebased);
    let menu = app.press(&on_tip, plain('.'));
    let open = menu.context_menu.as_ref().expect("`.` opened the menu");
    assert!(
        !matches!(
            open.menu.items.first().map(|item| &item.action),
            Some(ContextAction::Explain)
        ),
        "main's tip leads with Explain"
    );
    let menu_frame = frame(&menu, 120, 40);

    let body = format!(
        "{HEADER}\n== 1. After `p r r` and `P`: no \u{24d8} on any row ==\n\n{}\n\n\
         == 2. Cursor on stash@{{0}}: no \u{26a0} ==\n\n{}\n\n\
         == 3. `.` on main's tip: the menu does not lead with Explain ==\n\n{}\n",
        after_pull.join("\n"),
        stash_frame.join("\n"),
        menu_frame.join("\n"),
    );
    let body = without_invented_id(&body, &rebased);
    check_shot("m6/fix-radar-bookkeeping-120x40.txt", &body);
    drop(menu);
    app.close();
}

const HEADER: &str = "\
# m6/fix-radar-bookkeeping-120x40.txt: 120x40, drawn by `view::view` from a
# real AppState (crates/graft/tests/it/radar_bookkeeping.rs,
# a_pull_rebase_marks_no_duplicates, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# The scene: a teammate pushed, you had a local commit and a stash made on
# it; `p` named the divergence, `r r` rebased onto their work, `P` pushed
# it (so the true \"unpushed on main\" ⓘ is not in the picture).
# Your old commit still lives in Graft's undo backup and under the stash.
#
# What right looks like:
#  - Frame 1: no \u{24d8} in the marker column of any `main` row. The old
#    commit is bookkeeping, not a copy on a branch you could merge.
#  - Frame 2, cursor on stash@{0}: no \u{26a0} on that row. A stash is not a
#    merge you made, so it is never called an evil merge.
#  - Frame 3, `.` on main's tip: the first item is not `Explain`.
#  - The rebased tip's id is zeroed: it is new every run.
";
