//! The same merge box whatever opened it (ease-pass 10): the `.` menu and
//! a drag-and-drop, each against `m`'s.

use graft::update::AppState;
use graft_core::event::KeyCode;

use super::{cursor_on, fix_toc, merge_preview, open, planned};

#[test]
fn the_merge_box_is_the_same_from_the_dot_menu() {
    let scratch = fix_toc(true);
    let (app, state) = open(scratch.path());
    let by_m = merge_preview(&app, state, "sara/fix-toc");
    app.close();
    // A fresh session: the first one's replies went to `by_m`'s state.
    let (app, state) = open(scratch.path());
    let on_row = cursor_on(&app, state, "sara/fix-toc");
    let menu = app.key(&on_row, KeyCode::Char('.'));
    assert!(menu.context_menu.is_some(), "`.` opens the menu");
    let by_menu = planned(&app, pick_merge(&app, menu));
    let opened_by = |state: &AppState| match &state.preview.as_ref().expect("a preview").subject {
        graft::update::preview::PreviewSubject::Operation { opened_by, .. } => *opened_by,
        graft::update::preview::PreviewSubject::RevertTo(_) => "",
    };
    assert_eq!(opened_by(&by_m), "m");
    assert_eq!(opened_by(&by_menu), ".");
    let boxed = |state: &AppState| graft::update::preview::preview_box(state).expect("a box");
    assert_eq!(boxed(&by_menu).merge, boxed(&by_m).merge);
    assert!(boxed(&by_menu).merge.is_some());
    app.close();
}

#[test]
fn the_merge_box_is_the_same_from_a_drag_and_drop() {
    use graft_core::event::{MouseButton, MouseKind};
    let scratch = fix_toc(true);
    let (app, state) = open(scratch.path());
    let by_m = merge_preview(&app, state, "sara/fix-toc");
    app.close();
    // A fresh session: the first one's replies went to `by_m`'s state.
    let (app, state) = open(scratch.path());
    let state = crate::captures::settled_frame(&app, state);
    let into_row = |state: &AppState| {
        let window = state.history.as_ref().and_then(|it| it.window.as_ref());
        let refs = state.refs.as_deref().expect("refs");
        window
            .expect("rows")
            .rows
            .iter()
            .position(|row| {
                refs.on(row.commit.id).iter().any(|label| {
                    matches!(label, graft::refs::RefLabel::Branch { name, .. } if name == b"wallet-release")
                })
            })
            .expect("wallet-release's row")
    };
    let mut grabbed = None;
    'search: for y in 1..12_u16 {
        for x in 0..120 {
            let down = app.mouse(&state, MouseKind::Down(MouseButton::Left), x, y);
            if down
                .drag
                .as_ref()
                .is_some_and(|drag| drag.source.short == "sara/fix-toc")
            {
                grabbed = Some((down, x, y));
                break 'search;
            }
        }
    }
    let (state, x, y) = grabbed.expect("sara/fix-toc's badge can be picked up");
    let from = state.drag.as_ref().expect("in the air").from_row;
    let onto = into_row(&state);
    let onto_y = u16::try_from(
        i64::from(y) + i64::try_from(onto).expect("small") - i64::try_from(from).expect("small"),
    )
    .expect("on screen");
    let state = app.mouse(&state, MouseKind::Drag(MouseButton::Left), x, onto_y);
    let state = app.mouse(&state, MouseKind::Up(MouseButton::Left), x, onto_y);
    let by_drag = planned(&app, pick_merge(&app, state));
    let boxed = |state: &AppState| graft::update::preview::preview_box(state).expect("a box");
    assert!(boxed(&by_drag).merge.is_some(), "the merge box");
    assert_eq!(boxed(&by_drag).merge, boxed(&by_m).merge);
    app.close();
}

/// The open menu's `Merge …` item, walked to with `j` and taken with `⏎`
/// (the item itself, not the `m` key's own route).
fn pick_merge(app: &crate::app_harness::App, state: AppState) -> AppState {
    let menu = state.context_menu.as_ref().expect("a menu");
    let at = menu
        .menu
        .items
        .iter()
        .position(|item| item.label.as_str().starts_with("Merge"))
        .expect("a merge verb");
    let state = (0..at).fold(state, |state, _| app.key(&state, KeyCode::Char('j')));
    app.key(&state, KeyCode::Enter)
}
