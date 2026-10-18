//! `⏎` on the branch he is on (m8 usability Q2): closes, as `Tab` does.
//! Its own file so `picker/tests.rs` stays under its size row.

use super::*;

fn selected_oid(state: &AppState) -> Option<Oid> {
    state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id)
}

/// Closed, graph focused, cursor on HEAD's row (main, 0x02), nothing
/// written, nothing said. Replaces `enter_on_here_writes_nothing`, whose
/// `you're on main` note is the behaviour this removes.
#[test]
fn enter_on_the_current_branch_closes() {
    let state = on_row(&opened(&fixture()), "main");
    let before = state.status.clone();
    let (state, commands) = press(&state, KeyCode::Enter);
    assert!(state.picker.is_none(), "the picker closed");
    assert_eq!(
        state.detail.focus,
        mergewright_ui::detail::DetailFocus::Graph,
        "the graph has the keys"
    );
    assert_eq!(selected_oid(&state), Some(oid(2)), "HEAD's row");
    assert!(operated(&commands).is_none(), "no switch: {commands:?}");
    assert!(state.preview.is_none());
    assert_eq!(state.status, before, "no status line");
}

#[test]
fn enter_on_the_current_branch_lands_as_tab_does() {
    let state = on_row(&opened(&fixture()), "main");
    let (by_enter, enter_commands) = press(&state, KeyCode::Enter);
    let (by_tab, tab_commands) = press(&state, KeyCode::Tab);
    assert_eq!(format!("{enter_commands:?}"), format!("{tab_commands:?}"));
    assert_eq!(selected_oid(&by_enter), selected_oid(&by_tab));
    assert_eq!(by_enter.detail.focus, by_tab.detail.focus);
    assert_eq!(by_enter.picker.is_none(), by_tab.picker.is_none());
    assert_eq!(by_enter.status, by_tab.status);
}

#[test]
fn enter_on_the_current_branch_with_a_dirty_tree_closes_too() {
    let state = on_row(&opened(&with_edits(fixture())), "main");
    let (state, commands) = press(&state, KeyCode::Enter);
    assert!(state.preview.is_none(), "no set-aside preview");
    assert!(state.picker.is_none());
    assert!(operated(&commands).is_none());
    assert_eq!(selected_oid(&state), Some(oid(2)));
}

/// Rule 6 capture (his eye; `TestBackend`, colour unverified, rule 5): the
/// picker open with its cursor on `main`, the branch he is on — the bar
/// offers `⏎ show it on the graph` (`⇥`'s words, its act), the box's
/// bottom line carries no message.
#[test]
fn capture_the_picker_on_the_current_row() {
    let mut state = on_row(&opened(&fixture()), "main");
    state.size = crate::update::Size {
        columns: 120,
        rows: 36,
    };
    let drawn = crate::view::tests::draw_pinned(&state, 120, 36);
    assert!(drawn.contains("⏎ show it on the graph"), "{drawn}");
    assert!(!drawn.contains("switch to it"), "{drawn}");
    assert!(picker(&state).note.is_none());
    mergewright_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-q2-picker-current-row-ui-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        drawn,
    );
}

#[test]
fn enter_on_another_branch_still_switches() {
    let state = on_row(&opened(&fixture()), "catalog");
    let (state, commands) = press(&state, KeyCode::Enter);
    assert_eq!(
        checkout_to(operated(&commands).expect("launched at once")),
        &CheckoutTarget::Branch("refs/heads/catalog".to_owned())
    );
    assert!(state.picker.is_none());
}

/// Detached HEAD: no row is current, so `⏎` on `main` switches to it.
#[test]
fn detached_head_enter_still_switches() {
    let (state, commands) = press(&fixture(), KeyCode::Char('C'));
    let mut rows = list();
    for row in &mut rows.rows {
        row.current = false;
    }
    let (state, _) = update(
        &state,
        AppEvent::Produced(read_id(&commands), Product::SwitchList(Ok(Arc::new(rows)))),
    );
    let state = on_row(&state, "main");
    let (_, commands) = press(&state, KeyCode::Enter);
    assert_eq!(
        checkout_to(operated(&commands).expect("launched at once")),
        &CheckoutTarget::Branch("refs/heads/main".to_owned())
    );
}

#[test]
fn double_click_on_the_current_row_closes() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "main");
    let (state, _) = click(&state, column, row);
    assert!(state.picker.is_some(), "one click only selects");
    let (state, commands) = click(&state, column, row);
    assert!(operated(&commands).is_none(), "no switch");
    assert!(state.picker.is_none(), "closed, as ⏎ on it does");
    assert_eq!(selected_oid(&state), Some(oid(2)));
}
