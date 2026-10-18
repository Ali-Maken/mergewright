//! `update` tests: graph toggles. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// Ease-pass 06 box 7: `space` on a folding merge opens it (`▾`, its
/// branch drawn), and again closes it; the cursor stays on the merge.
#[test]
fn space_opens_a_folding_merge_and_closes_it_again() {
    let id = JobId::new(5);
    let (state, merge) = merge_on_screen(id, 1);

    let (opened, commands) = update(&state, key(KeyCode::Char(' ')));
    let view = asked_view(&commands);
    assert_eq!(
        view.inclusion.opened().map(|opened| opened.contains(merge)),
        Some(true),
        "the merge is opened"
    );
    assert_eq!(opened.history.as_ref().map(|h| &h.view), Some(&view));
    assert_eq!(
        opened.history.as_ref().map(|h| h.cursor),
        Some(1),
        "stays put"
    );

    let (closed, commands) = update(&opened, key(KeyCode::Char(' ')));
    assert_eq!(asked_view(&commands), View::folded());
    assert_eq!(closed.history.map(|h| h.view), Some(View::folded()));
}

/// Ease-pass 06 box 7: `space` on a row inside an opened merge closes that
/// merge and puts the cursor on the merge's row.
#[test]
fn space_inside_an_opened_merge_closes_it_and_lands_on_the_merge() {
    let id = JobId::new(5);
    let (state, merge) = merge_on_screen(id, 1);
    let (mut opened, commands) = update(&state, key(KeyCode::Char(' ')));
    let Some(Command::ShowRows(_, request)) = commands
        .iter()
        .find(|command| matches!(command, Command::ShowRows(..)))
    else {
        panic!("opening asks for rows: {commands:?}");
    };
    // The opened view's answer: the merged commit shows under the merge.
    let history = opened.history.as_ref().expect("a walk");
    let mut side = history.row_at(1).expect("the merge").clone();
    side.commit.id = Oid::from_bytes(&[4; Oid::SHA1_BYTES]).expect("an oid");
    side.commit.parents = Vec::new();
    let rows = vec![
        history.row_at(0).expect("row 0").clone(),
        history.row_at(1).expect("the merge").clone(),
        side,
    ];
    let inside = crate::history::RowFold {
        folds: None,
        inside: Some(crate::history::Inside { merge, label: None }),
    };
    let merge_fold = crate::history::RowFold {
        folds: Some(true),
        inside: None,
    };
    let window = Window {
        request: request.id,
        first: 0,
        rows: rows.into(),
        laid_out: 3,
        done: true,
        promoted: false,
        fold: [crate::history::RowFold::default(), merge_fold, inside].into(),
        sought: None,
    };
    (opened, _) = update(
        &opened,
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    opened.history.as_mut().expect("a walk").cursor = 2;

    let (closed, commands) = update(&opened, key(KeyCode::Char(' ')));
    let view = asked_view(&commands);
    assert_eq!(
        view.inclusion.opened().map(|opened| opened.contains(merge)),
        Some(false),
        "the merge is closed"
    );
    let history = closed.history.as_ref().expect("a walk");
    assert_eq!(history.cursor, 1, "the cursor lands on the merge");
    assert_eq!(history.reanchor, None, "not back on the hidden row");
    assert_eq!(history.selected().map(|row| row.commit.id), Some(merge));
}

/// Ease-pass 06 box 7: `space` on a row that neither folds a branch nor
/// sits inside an opened merge does nothing.
#[test]
fn space_off_a_merge_changes_nothing() {
    let id = JobId::new(5);
    let (state, _) = merge_on_screen(id, 0);
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.history, state.history);
}

/// m2-details-and-diffs/21 (m1-the-graph/54): a walk restart — `Ctrl+R`'s
/// among them — puts the cursor back on the commit it was on, the same
/// `reanchor` idiom `change_view` already uses. `update` cannot tell a
/// rebound table (a rebuilt id table on a changed commit-graph) apart from
/// an ordinary M1 refresh — that fact is only known on the job-runner thread
/// (`RepoStore::refresh`'s `Refreshed`) — so every restart reanchors.
#[test]
fn a_walk_restart_reanchors_on_the_previously_selected_commit() {
    let id = JobId::new(7);
    let (state, merge) = merge_on_screen(id, 1);
    assert_eq!(
        state
            .history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge),
        "the merge is selected before the restart"
    );

    let next_id = JobId::new(8);
    let (restarted, _) = walk_started(&state, next_id);

    assert_eq!(
        restarted.history.as_ref().map(|history| history.reanchor),
        Some(Some(merge)),
        "the new walk is told to land back on the commit that was selected"
    );
}

#[test]
fn one_switches_to_first_parent_view_from_the_top_and_back() {
    let id = JobId::new(6);
    let (state, _) = merge_on_screen(id, 1);

    let (straight, commands) = update(&state, key(KeyCode::Char('1')));
    assert_eq!(asked_view(&commands).follow, Follow::FirstParent);
    let history = straight.history.as_ref().expect("a walk");
    assert_eq!((history.top, history.cursor), (0, 0));

    let (full, commands) = update(&straight, key(KeyCode::Char('1')));
    assert_eq!(asked_view(&commands), View::folded());
    assert_eq!(full.history.map(|h| h.view), Some(View::folded()));
}

/// Every event clones the state (`update` returns a new one), so a large
/// immutable part must be shared by that clone, never copied: with 50k
/// refs a copy and its drop cost a millisecond a frame (m1-the-graph/69).
#[test]
fn the_next_state_shares_the_ref_badges_instead_of_copying_them() {
    let (state, _) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(3),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (next, _) = update(&state, key(KeyCode::Char('j')));

    let before = state.refs.as_ref().expect("the refs answer was taken");
    let after = next.refs.as_ref().expect("the badges carried over");
    assert!(Arc::ptr_eq(before, after), "the badges were copied");
}

#[test]
fn capital_h_hides_the_stash_and_shows_it_again() {
    let id = JobId::new(7);
    let (mut state, _) = merge_on_screen(id, 0);
    let (none, commands) = update(&state, key(KeyCode::Char('H')));
    assert!(
        commands.is_empty(),
        "no stash, nothing to hide: {commands:?}"
    );
    assert_eq!(none.history, state.history);

    let stash = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("an oid");
    let older = Oid::from_bytes(&[8; Oid::SHA1_BYTES]).expect("an oid");
    let name = graft_core::ref_name::RefName::new("refs/stash").expect("valid");
    let badges = crate::refs::badges(
        &[(name, stash)],
        &graft_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        &[(stash, 0), (older, 1)],
        |_, _| Err("no upstreams here".to_owned()),
    )
    .expect("badges");
    assert_eq!(badges.stash(), Some(stash));
    state.refs = Some(Arc::new(badges));

    // Both entries, not only the newest (m3-working-tree-undo-and-stash/06).
    let (hidden, commands) = update(&state, key(KeyCode::Char('H')));
    assert_eq!(
        asked_view(&commands).inclusion.hidden_stashes,
        [stash, older].into()
    );
    let (shown, commands) = update(&hidden, key(KeyCode::Char('H')));
    assert_eq!(asked_view(&commands), View::folded());
    assert_eq!(shown.history.map(|h| h.view), Some(View::folded()));
}

/// Refs listing one stash entry (`stash@{0}` at `stash`), `HEAD` detached
/// on it — enough for `refs.stashes()`.
fn refs_with_a_stash(stash: Oid) -> crate::refs::RefBadges {
    let name = graft_core::ref_name::RefName::new("refs/stash").expect("valid");
    crate::refs::badges(
        &[(name, stash)],
        &graft_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        &[(stash, 0)],
        |_, _| Err("no upstreams here".to_owned()),
    )
    .expect("badges")
}

/// m7-power-tools/16: a refs read with no stash asks nothing new — not the
/// stash-only read, not a view change — and drops any marks held.
#[test]
fn a_refs_read_with_no_stash_queues_nothing_new() {
    let mut state = AppState::default();
    state.stash_only.commits =
        Arc::new([(Oid::from_bytes(&[4; Oid::SHA1_BYTES]).expect("an oid"), 0)].into());
    let (next, commands) = update(
        &state,
        AppEvent::Produced(
            JobId::new(3),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadStashOnly(..) | Command::ShowRows(..))),
        "{commands:?}"
    );
    assert_eq!(next.stash_only, crate::update::StashOnlyMarks::default());
}

/// m7-power-tools/16: a refs read listing a stash asks which commits only
/// it reaches, under a fresh id; the view learns the entry.
#[test]
fn a_refs_read_with_a_stash_asks_the_stash_only_read_and_tells_the_view() {
    let id = JobId::new(7);
    let (state, _) = merge_on_screen(id, 0);
    let stash = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("an oid");
    let (next, commands) = update(
        &state,
        AppEvent::Produced(JobId::new(3), Product::Refs(Ok(refs_with_a_stash(stash)))),
    );
    let asked: Vec<_> = commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadStashOnly(job, stashes) => Some((*job, stashes.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(asked.len(), 1, "{commands:?}");
    assert_eq!(asked[0].1, vec![stash]);
    assert_eq!(next.stash_only.job, Some(asked[0].0));
    assert_eq!(asked_view(&commands).inclusion.stashes(), &[stash].into());
}

/// m7-power-tools/16: the stash-only read failing unmarks every row and
/// says so on the problem line (rule 3); an answer to an older read is
/// dropped, whatever it says.
#[test]
fn a_failed_stash_only_read_says_so() {
    let marked = Oid::from_bytes(&[4; Oid::SHA1_BYTES]).expect("an oid");
    let mut state = AppState::default();
    state.stash_only.job = Some(JobId::new(12));
    state.stash_only.commits = Arc::new([(marked, 0)].into());

    let (older_answer, _) = update(
        &state,
        AppEvent::Produced(JobId::new(11), Product::StashOnly(Err("old".to_owned()))),
    );
    assert_eq!(
        older_answer.stash_only, state.stash_only,
        "an older read's answer is dropped"
    );
    assert_eq!(older_answer.status, state.status);

    let (failed, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(12),
            Product::StashOnly(Err("git died".to_owned())),
        ),
    );
    assert!(failed.stash_only.commits.is_empty(), "no row stays marked");
    assert_eq!(failed.stash_only.job, None);
    assert_eq!(
        failed.status,
        Status::Failed("couldn't read which commits only a stash holds: git died".to_owned())
    );
}

#[test]
fn plus_expands_and_collapses_the_lanes_without_touching_the_walk() {
    let id = JobId::new(3);
    let (state, _) = update(&awaiting(id), rows(id, 4));
    let before = state.history.clone();

    let (expanded, commands) = update(&state, key(KeyCode::Char('+')));
    assert!(expanded.lanes_expanded);
    assert!(
        commands.is_empty(),
        "expanding is a view decision: {commands:?}"
    );
    assert_eq!(expanded.history, before, "the rows are not laid out again");

    let (folded, commands) = update(&expanded, key(KeyCode::Char('+')));
    assert!(!folded.lanes_expanded);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(folded.history, before);
}

#[test]
fn capital_f_toggles_focus_and_esc_leaves_it_without_touching_the_walk() {
    let id = JobId::new(3);
    let (state, _) = update(&awaiting(id), rows(id, 4));
    let before = state.history.clone();

    let (focused, commands) = update(&state, key(KeyCode::Char('F')));
    assert!(focused.focus);
    assert!(commands.is_empty(), "focus is paint: {commands:?}");
    assert_eq!(focused.history, before);

    let (left, commands) = update(&focused, key(KeyCode::Escape));
    assert!(!left.focus);
    assert!(commands.is_empty(), "Esc leaves focus first: {commands:?}");
    assert_eq!(left.awaiting, Some(id), "the walk goes on");
    assert_eq!(left.history, before);
}
