//! m6-conflicts/69: while a box is open, the bottom row is the box's keys,
//! never a status from before it — captured against real repositories.
//!
//! Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it box_owns_the_bar`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::{AppState, Status};
use mergewright_core::event::KeyCode;

use crate::app_harness::App;
use crate::captures::{check_shot, frame, frame_at};
use crate::m6_stash_put_back::main_and_fix;
use crate::preview::FORK_NOW;
use crate::remote_by_key::{confirm, finish, local, plain, ready};
use crate::undo_over_drift::drift_box_after_a_checkout;

const SIZES: [(&str, u16, u16); 2] = [("120x40", 120, 40), ("80x24", 80, 24)];

fn header(size: &str) -> String {
    format!(
        "\
# TestBackend capture (not a pty), {size}, no colour recorded. Fixture:
# capture_fix_box_owns_the_bar in crates/mergewright/tests/it/box_owns_the_bar.rs
# (asserted). Four frames, each a box open over a status that was standing.
#
# What right looks like, the bottom row of each frame:
# 1. A teammate pushed; `P` `→` `⏎` was rejected, then `P` opened the lease
#    box: `←/→ move  ⏎ choose  esc cancel` — the box's keys, not the
#    rejection line (which named `p`, a key the box swallows).
# 2. `esc` on that box: the rejection is back, `push rejected: … · p pull,
#    then m or r · P force with lease` — hidden under the box, never cleared.
# 3. Another repository: a checkout, an edit, then `u` over the drift box:
#    `y undo  ←/→ move  ⏎ choose  esc cancel` — not `undo · esc cancels`,
#    the finished job's busy line.
# 4. Back on the first: `b`, then `⏎` on the empty name: the prompt stays
#    open, and the row reads `a branch needs a name` — the prompt's own
#    refusal, written to the prompt, not the status line.
# - Colour is unverified (CLAUDE.md rule 5).
#
"
    )
}

fn bottom(drawn: &[String]) -> String {
    drawn
        .last()
        .map(|row| row.trim().to_owned())
        .unwrap_or_default()
}

/// Frames 1, 2 and 4: the lease box over a rejection, `esc`, and the empty
/// name refused inside the prompt. Each at [`SIZES`].
fn over_a_rejection() -> [Vec<Vec<String>>; 3] {
    let remote = local();
    let work = remote.work();
    remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    remote.commit_locally("ours").expect("a local commit");

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    let state = finish(&app, state, "the rejected push");
    assert!(
        matches!(&state.status, Status::Sticky { reason, .. } if reason.starts_with("push rejected")),
        "{:?}",
        state.status
    );
    let state = ready(&app, state);
    let state = crate::captures::at_rest(&app, state);
    let lease = app.press(&state, plain('P'));
    assert!(lease.confirm.is_some(), "P opened the lease box");
    let escaped = app.key(&lease, KeyCode::Escape);
    assert!(escaped.confirm.is_none());
    let prompt = app.press(&escaped, plain('b'));
    assert!(prompt.name_prompt.is_some(), "b opened the name prompt");
    let refused = app.key(&prompt, KeyCode::Enter);
    assert!(refused.name_prompt.is_some(), "the empty name kept it open");
    assert_eq!(refused.status, escaped.status, "the status was not written");
    app.close();

    let at = |state: &AppState| -> Vec<Vec<String>> {
        SIZES
            .iter()
            .map(|(_, columns, rows)| frame(state, *columns, *rows))
            .collect()
    };
    [at(&lease), at(&escaped), at(&refused)]
}

/// Frame 3: a checkout through the menu, an edit, `u`: the drift box.
fn drift_box() -> Vec<Vec<String>> {
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open_with_clock(path, Clock::Fixed(FORK_NOW));
    let state = drift_box_after_a_checkout(&app, path, state);
    app.close();
    SIZES
        .iter()
        .map(|(_, columns, rows)| frame_at(&state, *columns, *rows, Some(Clock::Fixed(FORK_NOW))))
        .collect()
}

#[test]
fn capture_fix_box_owns_the_bar() {
    let [lease, escaped, refused] = over_a_rejection();
    let drift = drift_box();
    for (index, (size, _, _)) in SIZES.iter().enumerate() {
        assert_eq!(
            bottom(&lease[index]),
            "←/→ move  ⏎ choose  esc cancel",
            "{size}"
        );
        let back = bottom(&escaped[index]);
        assert!(
            back.starts_with("push rejected") && back.ends_with("P force with lease"),
            "{size}: {back}"
        );
        assert_eq!(
            bottom(&drift[index]),
            "y undo  ←/→ move  ⏎ choose  esc cancel",
            "{size}"
        );
        assert_eq!(bottom(&refused[index]), "a branch needs a name", "{size}");
        check_shot(
            &format!("m6/fix-box-owns-the-bar-{size}.txt"),
            &format!(
                "{}=== frame 1: the lease box over a rejected push ===\n{}\n\n\
                 === frame 2: `esc`, the rejection back ===\n{}\n\n\
                 === frame 3: the drift box `u` opened ===\n{}\n\n\
                 === frame 4: `b`, `⏎` on an empty name ===\n{}\n",
                header(size),
                lease[index].join("\n"),
                escaped[index].join("\n"),
                drift[index].join("\n"),
                refused[index].join("\n"),
            ),
        );
    }
}
