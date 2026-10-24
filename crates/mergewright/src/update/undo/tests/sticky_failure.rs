//! ease-pass 28 A: the capture `docs/shots/ease-pass/28-sticky-failure.txt`.

use mergewright_core::capabilities::RenderingTier;

use super::*;

fn state_at(ascii: bool) -> AppState {
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    };
    if ascii {
        state.renderer_override = Some(RenderingTier::Ascii);
    }
    state
}

/// The bottom row of the frame with `status` showing, quotes and padding off.
fn bar(mut state: AppState, status: Status) -> String {
    state.status = status;
    let drawn = crate::view::tests::draw(&state, 120, 40);
    let last = drawn.lines().last().expect("a bottom row");
    last.trim_matches(['"', ' ']).to_owned()
}

fn failed_undo(ascii: bool) -> String {
    let mut state = state_at(ascii);
    state.operating = Some(JobId::new(2));
    let failed = Undone::Failed {
        error: "refs/heads/main is not where it was".to_owned(),
        refresh: None,
    };
    undone(&mut state, JobId::new(2), failed, &mut ids());
    let status = state.status.clone();
    bar(state, status)
}

fn rejected_push(ascii: bool) -> String {
    let status = Status::sticky_failure_keys(
        "push rejected: origin/main has commits you don't",
        Some("p pull · P force".to_owned()),
    );
    bar(state_at(ascii), status)
}

/// The failed-undo bar and the rejected-push bar at 120x40, Unicode then
/// ASCII tier. Bottom row only.
#[test]
fn capture_ease_28_sticky_failure() {
    let (undo_u, undo_a) = (failed_undo(false), failed_undo(true));
    let (push_u, push_a) = (rejected_push(false), rejected_push(true));
    assert!(
        undo_u.starts_with("⊗ refs/heads/main is not where it was"),
        "{undo_u}"
    );
    assert!(
        push_u.starts_with("⊗ push rejected") && push_u.contains("p pull · P force"),
        "{push_u}"
    );
    assert!(
        undo_a.starts_with("(x) refs/heads/main") && !undo_a.contains('⊗'),
        "{undo_a}"
    );
    assert!(
        push_a.starts_with("(x) push rejected") && push_a.contains("p pull · P force"),
        "{push_a}"
    );
    let capture = format!(
        "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_ease_28_sticky_failure in crates/mergewright/src/update/undo/tests/sticky_failure.rs
# (asserted). Bottom row of the frame, 120x40. Colour (the mark in red) is
# unverified: a capture cannot show it.
#
=== Failed undo, Unicode. Right: `⊗ ` first on the bottom row ===
{undo_u}

=== Rejected push, Unicode. Right: `⊗ ` first, `p pull · P force` whole ===
{push_u}

=== Failed undo, ASCII. Right: `(x) ` first, no `⊗` ===
{undo_a}

=== Rejected push, ASCII. Right: `(x) ` first, `p pull · P force` whole ===
{push_a}
"
    );
    mergewright_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/ease-pass/28-sticky-failure.txt"
        ),
        capture,
    );
}
