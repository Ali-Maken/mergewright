//! m6-conflicts/78: a status that arrives under a box is held, not lost,
//! and the name prompt refuses a second write inside itself — captured
//! against a real repository and a real remote.
//!
//! Regenerate on purpose: `TRIB_UPDATE_SHOTS=1 cargo test -p graft --test it status_under_a_box`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::history::Phase;
use graft::update::detail::Loading;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_fixtures::remote::LocalRemote;

use crate::app_harness::App;
use crate::captures::{check_shot, frame, radar_has_answered};
use crate::remote_by_key::{is_ready, local, plain, run};

const SIZES: [(&str, u16, u16); 2] = [("120x40", 120, 40), ("80x24", 80, 24)];

/// Where the first fetch goes: nowhere. Not a machine path — the failure
/// line quotes it, and the capture may be published.
const NOWHERE: &str = "file:///graft-no-such-remote";

fn header(size: &str) -> String {
    format!(
        "\
# TestBackend capture (not a pty), {size}, no colour recorded. Fixture:
# capture_fix_status_under_a_box in crates/graft/tests/it/status_under_a_box.rs
# (asserted). Six frames, one sitting: a fetch fails under the name prompt,
# then a second fetch runs while the prompt is up again.
#
# What right looks like, the bottom row of each frame:
# 1. `f` (to a remote that is not there), `b`, and the fetch fails while the
#    prompt is up: the prompt's keys — the failure is held, not drawn over
#    the box and not lost.
# 2. `topic ⏎`: the prompt closes and the row is the held fetch failure,
#    `… · esc dismiss` — not the create's busy line, which would hide it.
# 3. `esc`: the failure dismissed, the create's own line. The create is not
#    cancelled: `esc` dismissed a drawn failure first.
# 4. `f` again (the remote back), `b`, `other ⏎` while that fetch runs: the
#    prompt stays open with `other` kept, and the row reads `wait for fetch …
#    to finish` — refused inside the prompt, no second write started.
# 5. The fetch answers: the row is the prompt's keys again (the refusal went
#    with its cause); one `⏎` would now create.
# 6. `esc`: the prompt closes and the fetch's finished line shows, with its
#    tick — it arrived under the box and was not acknowledged unseen.
# - No `ⓘ` on any row: `topic` has no upstream, which is no finding
#   (m7-power-tools/15).
# - Colour is unverified (CLAUDE.md rule 5).
#
"
    )
}

const TITLES: [&str; 6] = [
    "`f` to nowhere, `b`, the fetch fails under the prompt",
    "`topic ⏎`, the held failure",
    "`esc`, the create's line",
    "`f`, `b`, `other ⏎` while the fetch runs",
    "the fetch answers under the prompt",
    "`esc`, the fetch's line",
];

fn bottom(drawn: &[String]) -> String {
    drawn
        .last()
        .map(|row| row.trim().to_owned())
        .unwrap_or_default()
}

/// Nothing a frame draws is still being read: `remote_by_key::ready`'s
/// facts, plus a **complete** walk, `git --version` (the footer's
/// commit-graph line) and the radar's answer.
///
/// `ready` alone is what `f`/`b` decide from, not what a frame shows. A
/// frame taken there caught the walk mid-sort (`N rows so far … sorting…`)
/// and the radar either answered or not, whichever the job pool reached
/// first: `topic`'s no-upstream `ⓘ` on main's row in frame 4 under load,
/// none otherwise (`KNOWN_GAPS` M7/ⓘ; the same cause as closed row M7/04b,
/// in `remote_by_key::ready` instead of `two_boxes_over_content::ready`).
fn at_rest(state: &AppState) -> bool {
    is_ready(state)
        && state.system_git.is_some()
        && state
            .history
            .as_ref()
            .is_some_and(|history| history.phase == Phase::Complete && history.promotion.is_none())
        && radar_has_answered(state)
}

/// Pumps until [`at_rest`].
fn rested(app: &App, state: AppState) -> AppState {
    app.pump(state, "a frame with nothing still being read", at_rest)
}

fn type_in(app: &App, state: AppState, text: &str) -> AppState {
    text.chars()
        .fold(state, |state, c| app.press(&state, plain(c)))
}

/// Everything the six frames are drawn from, and what their rows are held
/// to.
struct Sitting {
    /// Frames 1–6. Frame 3's state is the create still running; it is
    /// settled (from a clone) before frame 4.
    states: Vec<AppState>,
    prompt_keys: String,
    failure_words: String,
    fetched_words: String,
}

/// Frames 1–3: `f` to nowhere fails under the prompt; `topic ⏎`; `esc`.
/// Returns the three states and the failure's words.
fn a_failure_under_the_prompt(
    app: &App,
    remote: &LocalRemote,
    state: &AppState,
) -> ((AppState, AppState, AppState), String) {
    run(
        remote,
        remote.work(),
        &["remote", "set-url", "origin", NOWHERE],
    );
    let fetching = app.press(state, plain('f'));
    let fetch = fetching.operating.expect("`f` launched the fetch");
    let prompt = app.press(&fetching, plain('b'));
    assert!(prompt.name_prompt.is_some(), "b opened over the fetch");
    let failed = app.pump(prompt, "the fetch to nowhere", |state| {
        state.operating.is_none()
    });
    let failure_words = match &failed.status {
        Status::Failed(reason) | Status::Sticky { reason, .. } => reason.clone(),
        other => panic!("the fetch did not fail: {other:?}"),
    };
    assert!(failed.name_prompt.is_some(), "the prompt is still up");

    let typed = type_in(app, failed.clone(), "topic");
    let created = app.key(&typed, KeyCode::Enter);
    assert!(
        created.name_prompt.is_none(),
        "the create closed the prompt"
    );
    let create = created.operating.expect("the create launched");
    assert_ne!(create, fetch);

    let dismissed = app.key(&created, KeyCode::Escape);
    assert_eq!(
        dismissed.operating,
        Some(create),
        "esc did not cancel the create"
    );
    ((failed, created, dismissed), failure_words)
}

/// Frames 4–6: the remote back, `f`, `b`, `other ⏎` refused; the fetch
/// answers; `esc`.
fn a_second_write_refused(
    app: &App,
    remote: &LocalRemote,
    after: AppState,
) -> (AppState, AppState, AppState) {
    let work = remote.work();
    let settled = app.settle_write(after, "the create");
    run(
        remote,
        work,
        &["remote", "set-url", "origin", &remote.url()],
    );
    let settled = rested(app, settled);
    let fetching = app.press(&settled, plain('f'));
    let second = fetching.operating.expect("the second fetch launched");
    let prompt = app.press(&fetching, plain('b'));
    let prompt = type_in(app, prompt, "other");
    let refused = app.key(&prompt, KeyCode::Enter);
    assert!(
        matches!(refused.radar.context, Loading::Ready(_)),
        "frame 4 would be drawn before the radar answered: {:?}",
        refused.radar.context
    );
    assert!(
        refused
            .history
            .as_ref()
            .is_some_and(|history| history.phase == Phase::Complete),
        "frame 4 would be drawn from a walk still sorting"
    );
    assert!(refused.name_prompt.is_some(), "refused inside the prompt");
    assert_eq!(refused.operating, Some(second), "no second write started");

    let answered = app.pump(refused.clone(), "the second fetch", |state| {
        state.operating.is_none()
    });
    assert!(answered.name_prompt.is_some(), "the prompt is still up");
    let closed = app.key(&answered, KeyCode::Escape);
    assert!(closed.name_prompt.is_none());
    assert_eq!(
        closed.status, answered.status,
        "the done was not acknowledged unseen"
    );
    (refused, answered, closed)
}

fn sit() -> Sitting {
    let remote = local();
    let (app, state) = App::open_with_clock(
        remote.work(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = rested(&app, state);
    let prompt_keys = {
        let open = app.press(&state, plain('b'));
        assert!(open.name_prompt.is_some(), "b opened the name prompt");
        bottom(&frame(&open, 120, 40))
    };
    let ((failed, created, dismissed), failure_words) =
        a_failure_under_the_prompt(&app, &remote, &state);
    let (refused, answered, closed) = a_second_write_refused(&app, &remote, dismissed.clone());
    app.close();
    let Status::Done {
        what: fetched_words,
        ..
    } = &answered.status
    else {
        panic!("the second fetch did not finish: {:?}", answered.status)
    };
    let fetched_words = fetched_words.clone();
    Sitting {
        states: vec![failed, created, dismissed, refused, answered, closed],
        prompt_keys,
        failure_words,
        fetched_words,
    }
}

/// Each frame's bottom row, held to the header's list.
fn check_rows(sitting: &Sitting, size: &str, rows: &[String]) {
    let at = |n: usize| format!("{size}, frame {}: {rows:?}", n + 1);
    assert_eq!(rows[0], sitting.prompt_keys, "{}", at(0));
    let head: String = sitting.failure_words.chars().take(20).collect();
    assert!(
        rows[1].starts_with(&head) && rows[1].ends_with("esc dismiss"),
        "{}",
        at(1)
    );
    assert!(
        !rows[2].contains(&head) && rows[2].contains("topic"),
        "{}",
        at(2)
    );
    assert!(
        rows[3].starts_with("wait for fetch") && rows[3].ends_with("to finish"),
        "{}",
        at(3)
    );
    assert_eq!(rows[4], sitting.prompt_keys, "{}", at(4));
    assert!(
        rows[5].contains(sitting.fetched_words.as_str()) && rows[5].contains('\u{2713}'),
        "{}",
        at(5)
    );
}

/// `KNOWN_GAPS` M7/ⓘ, both orderings of the race set by hand on a real state:
/// the radar's answer, or the walk's end, not landed yet is not at rest, so
/// no frame is taken in either gap; the state [`rested`] hands back is.
#[test]
fn a_frame_waits_for_the_radar_and_the_whole_walk() {
    let remote = local();
    let (app, state) = App::open_with_clock(
        remote.work(),
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = rested(&app, state);
    app.close();
    assert!(at_rest(&state));

    let mut radar_late = state.clone();
    radar_late.radar.context = Loading::Pending(JobId::new(u64::MAX));
    assert!(
        is_ready(&radar_late) && !at_rest(&radar_late),
        "`ready` holds before the radar answers; a frame must not"
    );

    let mut still_sorting = state;
    if let Some(history) = still_sorting.history.as_mut() {
        history.phase = Phase::Streaming;
    }
    assert!(
        is_ready(&still_sorting) && !at_rest(&still_sorting),
        "`ready` holds mid-sort; a frame must not"
    );
}

#[test]
fn capture_fix_status_under_a_box() {
    let sitting = sit();
    for (size, columns, rows) in SIZES {
        let frames: Vec<Vec<String>> = sitting
            .states
            .iter()
            .map(|state| frame(state, columns, rows))
            .collect();
        let bottoms: Vec<String> = frames.iter().map(|drawn| bottom(drawn)).collect();
        check_rows(&sitting, size, &bottoms);
        let mut body = header(size);
        for (index, (title, drawn)) in TITLES.iter().zip(&frames).enumerate() {
            body.push_str(&format!(
                "=== frame {}: {title} ===\n{}\n",
                index + 1,
                drawn.join("\n")
            ));
            if index + 1 < TITLES.len() {
                body.push('\n');
            }
        }
        check_shot(&format!("m6/fix-status-under-a-box-{size}.txt"), &body);
    }
}
