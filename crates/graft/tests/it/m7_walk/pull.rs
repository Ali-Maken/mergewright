//! `pull_ff_over_edits` (m7-power-tools/03b): a fast-forward pull over an
//! unsaved edit to a file it changes is `p`, one key, no decision — his
//! edit set aside and put back, HEAD fast-forwarded, the cursor on the new
//! HEAD, `pulled origin/main ✓ · your edits are back`. The capture is drawn
//! by `view::view` from the walked state (colour unverified, CLAUDE.md
//! rule 5).
//!
//! The M7 note's `journey-counts` fence has no row for this journey: its
//! count, 1, is 03's "Decided" (`Journey: pull over overlapping edits = p,
//! 1 key`). No best-tool count: the prior-art study has no pull row.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::session::Clock;
use graft::update::Status;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, settled_frame};
use crate::pull_over_edits::{BOTH, MINE, SHARED, his_lines, origin_ahead_over, pulled, selected};
use crate::remote_by_key::{local, ready, rev};
use crate::stash_clash::stash_list;

/// 03 "Decided": `p`, one key.
const NOTED_KEYS: usize = 1;

/// An hour after the fixture's fixed commit clock (2021-01-01T00:00Z), so
/// the capture's ages never drift with real time.
const PULL_NOW: i64 = 1_609_459_200 + 60 * 60;

#[test]
fn pull_ff_over_edits() {
    let remote = local();
    let work = remote.work();
    let theirs = origin_ahead_over(&remote, MINE);
    let mine = his_lines(work);
    let (app, state) = App::open_with_clock(work, Clock::Fixed(PULL_NOW));
    let state = app.resize(&state, 120, 40);
    let state = ready(&app, state);

    let route = ["p"];
    let state = pulled(&app, &state);
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
        body.contains("pulled origin/main \u{2713} \u{b7} your edits are back"),
        "{body}"
    );
    check_shot("m7/pull-over-edits-120x40.txt", &body);
    drop(state);
    app.close();
}
