//! `pull_ff_over_edits` (m7-power-tools/03b): a fast-forward pull over an
//! unsaved edit to a file it changes is `p`, one key, no decision — his
//! edit set aside and put back, HEAD fast-forwarded, the cursor on the new
//! HEAD, `✓ pulled origin/main · your edits are back`. The capture is drawn
//! by `view::view` from the walked state (colour unverified, CLAUDE.md
//! rule 5).
//!
//! Its count, 1, is 03's "Decided" (`Journey: pull over overlapping edits =
//! p, 1 key`); the fence has no row for the over-edits variant.
//!
//! `pull_current_branch` (m7-acceptance) walks the fence's own row,
//! `pull-current-branch` (journey 17: upstream set, clean tree, `p`), read
//! from the note.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::history::Phase;
use graft::session::Clock;
use graft::update::Status;
use graft_fixtures::Shape;
use graft_fixtures::remote::LocalRemote;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, settled_frame};
use crate::pull_over_edits::{
    BOTH, MINE, SHARED, TEN, his_lines, origin_ahead_over, pulled, selected,
};
use crate::remote_by_key::{local, ready, rev, run};
use crate::stash_clash::stash_list;

/// 03 "Decided": `p`, one key.
const NOTED_KEYS: usize = 1;

/// An hour after the fixture's fixed commit clock (2021-01-01T00:00Z), so
/// the capture's ages never drift with real time.
const PULL_NOW: i64 = 1_609_459_200 + 60 * 60;

/// [`local`]'s remote and config over `Shape::Trivial`, a history the walk's
/// first answer holds whole (m8/42-flake-pull-ff).
///
/// Over `Linear`'s 200 commits the capture's header was a race. The pull's
/// reload asks for the first rows; the landing asks a `Find` for `HEAD` once
/// the refs read answers. A `Find` sent after the worker has the first rows
/// but before the screen has them is answered after the worker laid out
/// ahead between requests (`history/serve.rs`, `next_request`), and the
/// header takes that answer's count (`history/scroll/apply.rs`,
/// `self.rows = window.laid_out`): `140 rows so far` in most runs, `146`…
/// `202 commits` in the rest. A history that fits the first answer is
/// `Complete` whichever comes first.
fn whole_in_one_answer() -> LocalRemote {
    let remote =
        LocalRemote::new(Shape::Trivial).expect("the Trivial shape clones into a local remote");
    for args in [
        ["config", "core.hooksPath", ".git/hooks"],
        ["config", "user.name", "Graft Test"],
        ["config", "user.email", "test@example.invalid"],
    ] {
        run(&remote, remote.work(), &args);
    }
    remote
}

#[test]
fn pull_ff_over_edits() {
    let remote = whole_in_one_answer();
    let work = remote.work();
    let theirs = origin_ahead_over(&remote, MINE);
    let mine = his_lines(work);
    let (app, state) = App::open_with_clock(work, Clock::Fixed(PULL_NOW));
    let state = app.resize(&state, 120, 40);
    let state = ready(&app, state);

    let route = ["p"];
    let state = pulled(&app, &state);
    assert_eq!(
        state.history.as_ref().map(|history| &history.phase),
        Some(&Phase::Complete),
        "the whole history fits the walk's first answer, so the header the \
         capture holds cannot depend on when the landing's read answers"
    );
    assert!(
        route.len() <= NOTED_KEYS,
        "pull_ff_over_edits took {} keys {route:?}, 03 allows {NOTED_KEYS}",
        route.len()
    );

    assert_eq!(rev(work, "HEAD"), theirs, "HEAD fast-forwarded");
    assert_eq!(
        std::fs::read_to_string(work.join(SHARED)).expect("the file"),
        BOTH
    );
    assert_eq!(his_lines(work), mine, "his edit back byte-equal");
    assert_eq!(
        git(work, &["diff", "--cached", "--name-only"]).trim(),
        "",
        "unstaged, as it was"
    );
    assert!(stash_list(work).is_empty(), "no stash left");
    assert_eq!(
        selected(&state).as_deref(),
        Some(theirs.as_str()),
        "the cursor on the new HEAD"
    );
    assert!(
        matches!(&state.status, Status::Done { what, also: Some(also), .. }
            if what == "pulled origin/main" && also == "your edits are back"),
        "{:?}",
        state.status
    );

    let state = settled_frame(&app, state);
    let frame = frame_at(&state, 120, 40, Some(Clock::Fixed(PULL_NOW)));
    let body = format!("{}\n", frame.join("\n"));
    assert!(
        body.contains("\u{2713} pulled origin/main \u{b7} your edits are back"),
        "{body}"
    );
    check_shot("m7/pull-over-edits-120x40.txt", &body);
    drop(state);
    app.close();
}

/// `pull-current-branch` (the M7 note's 2a fence; journey 17): the graph,
/// cursor on HEAD, upstream set, a clean tree, origin one ahead. `p`, within
/// the note's count. Lands (note "Finish" row, 03): HEAD at origin's tip,
/// the cursor on it, `pulled origin/main` on the status line, nothing set
/// aside.
#[test]
fn pull_current_branch() {
    let remote = local();
    let work = remote.work();
    // The edit written back is the committed text: the tree is clean.
    let theirs = origin_ahead_over(&remote, TEN);
    assert_eq!(git(work, &["status", "--porcelain"]).trim(), "", "clean");
    let (app, state) = App::open_with_clock(work, Clock::Fixed(PULL_NOW));
    let state = app.resize(&state, 120, 40);
    let state = ready(&app, state);

    let route = ["p"];
    let state = pulled(&app, &state);
    let allowed = super::noted_keys("pull-current-branch");
    assert!(
        route.len() <= allowed,
        "pull-current-branch took {} keys {route:?}, the note allows {allowed}",
        route.len()
    );

    assert_eq!(rev(work, "HEAD"), theirs, "HEAD fast-forwarded");
    assert_eq!(git(work, &["status", "--porcelain"]).trim(), "", "clean");
    assert!(stash_list(work).is_empty(), "nothing set aside");
    assert_eq!(
        selected(&state).as_deref(),
        Some(theirs.as_str()),
        "the cursor on the new HEAD"
    );
    assert!(
        matches!(&state.status, Status::Done { what, also: None, .. }
            if what == "pulled origin/main"),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
}
