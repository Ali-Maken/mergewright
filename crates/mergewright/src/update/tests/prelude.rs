//! `update` tests: prelude. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// m2-details-and-diffs/23: a harness prelude waits for the first window and
/// the refs answer, then feeds one key per settled state: its `Enter` opens
/// the pane on row 0, not on nothing, and the `j` after it waits for the
/// file list that `Enter` asked for.
#[test]
fn the_prelude_waits_for_the_window_and_the_refs_and_then_opens_row_0() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![
        KeyPress::plain(KeyCode::Enter),
        KeyPress::plain(KeyCode::Char('j')),
    ];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    assert!(!state.detail.shown, "no window yet: nothing to open on");
    assert_eq!(state.harness.prelude.len(), 2, "both keys still waiting");

    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    assert!(
        !state.detail.shown,
        "the window is here but the working-tree status is not: still waiting"
    );
    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert!(state.detail.shown, "Enter was fed once the window landed");
    assert!(state.detail.spec.is_some(), "the pane is on a real row");
    assert!(
        matches!(state.detail.files, detail::Loading::Pending(_)),
        "Enter asked for row 0's files"
    );
    assert_eq!(state.harness.prelude.len(), 1, "j waits for the file list");
}

/// A clean working-tree answer, the way the status job reports one.
pub(super) fn working_tree_read(id: JobId) -> AppEvent {
    AppEvent::Produced(
        id,
        Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
            rewrite: None,
            status: std::sync::Arc::new(mergewright_git::status::WorkingTreeStatus::default()),
            in_progress: None,
            mergetool: None,
        }))),
    )
}

/// m3-working-tree-undo-and-stash/07: a prelude key waits for a write in
/// flight. `u` queued behind `Space` must undo the stage, not race it — the
/// acceptance captures press exactly such sequences.
#[test]
fn the_prelude_holds_a_key_while_a_write_is_in_flight() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Char('J'))];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    state.operating = Some(JobId::new(77));

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "a write is in flight: J waits"
    );
    assert!(!state.journal.shown);

    let mut state = state;
    state.operating = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "the write landed: J was fed"
    );
    assert!(state.journal.shown, "and it opened the journal browser");
}

/// m7-power-tools/03: a finished write that moved `HEAD` owes the cursor a
/// landing until its refresh's refs and walk are in. A key fed before then
/// acts on the row the cursor was on, not the new `HEAD`, and a capture
/// would show the cursor mid-way.
#[test]
fn settled_waits_for_an_owed_landing() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Char('J'))];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    state.land_on_head = true;
    state.head_landing = crate::update::HeadLanding {
        refs: Some(JobId::new(50)),
        walk: Some(JobId::new(51)),
        ..crate::update::HeadLanding::default()
    };

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(state.harness.prelude.len(), 1, "a landing is owed: J waits");

    let mut state = state;
    state.land_on_head = false;
    state.head_landing = crate::update::HeadLanding::default();
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "the landing was spent: J was fed"
    );
}

/// m5-history-and-ghost-preview/11: a preview whose plan is still being
/// worked out is a job an earlier key started. The acceptance captures
/// press `m → ⏎` — `→` and `⏎` on a box with no plan yet would close it
/// and the pty would record a graph with no box, which proves nothing.
#[test]
fn the_prelude_holds_a_key_while_a_preview_plan_is_pending() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Right)];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let planning = JobId::new(78);
    state.preview = Some(super::super::preview::PreviewState {
        subject: super::super::preview::PreviewSubject::RevertTo(
            mergewright_core::journal::EntryId::new(0, [0; 10]),
        ),
        resident: Vec::new(),
        merges: std::collections::HashSet::new(),
        slice_top: 0,
        job: planning,
        plan: Loading::Pending(planning),
        rows: Vec::new(),
        view_start: 0,
        selected_row: None,
        ghost_index: std::collections::HashMap::new(),
        clipped: 0,
        undrawable: None,
        cursor: 0,
        return_to_journal: false,
        refused: false,
        showing: super::super::preview::Showing::Box,
    });

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "the plan is in flight: → waits"
    );

    let mut state = state;
    state.preview = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "the box is gone: → was fed"
    );
}
