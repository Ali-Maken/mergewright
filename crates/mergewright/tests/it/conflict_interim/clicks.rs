//! Clicks while the banner is up: below a hundred columns, and on the
//! detail tab. Moved out of `conflict_interim.rs` to keep it under its row.

use super::*;

/// **Review M1, on the screen.** The rows the tab layout spends — the
/// `[Graph] Detail` strip, and the operation banner's row when one is drawn
/// above it — were unknown to `update::geometry` and to
/// `AppState::graph_rows`, so below 100 columns with a merge stopped every
/// click landed on a row above the one the user was pointing at.
///
/// Driven through the real mouse event, not through the geometry: what is
/// being asserted is that the commit the click picks is the commit whose
/// subject is drawn on that screen row. The sweep
/// (`update::tests::the_geometry_mirror_matches_the_rects_the_frame_hands_out`)
/// is the class test; this is the anchor that ties it to pixels.
#[test]
fn a_click_below_a_hundred_columns_picks_the_row_it_landed_on() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let mut state = stopped_with_the_pane_open(&app, state);
    state.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
    state.size = mergewright::update::Size {
        columns: 80,
        rows: 24,
    };

    for banner_focus in [false, true] {
        let mut on_screen = state.clone();
        on_screen.banner_focus = banner_focus;
        let drawn = frame(&on_screen, 80, 24);
        let at = format!("banner_focus={banner_focus}:\n{}", drawn.join("\n"));
        for subject in ["main's take", "the shared files", "Add a readme"] {
            let row = drawn
                .iter()
                .position(|line| line.contains(subject))
                .unwrap_or_else(|| panic!("`{subject}` is on the frame {at}"));
            let row = u16::try_from(row).expect("24 rows fit a u16");
            let clicked = app.mouse_down(&on_screen, MouseButton::Left, 2, row);
            let picked = clicked
                .history
                .as_ref()
                .and_then(mergewright::history::History::selected)
                .map(|found| String::from_utf8_lossy(found.commit.subject()).into_owned())
                .unwrap_or_default();
            assert_eq!(
                picked, subject,
                "a click on screen row {row} picked `{picked}` {at}"
            );
        }
    }
    drop(state);
    app.close();
}

/// The other half of review M1's click: **below 100 columns with the Detail
/// tab shown the graph is not on screen at all**, so no click can name a
/// graph row. Before the guard in `geometry::graph_row_at`, clicking the
/// file `notes.md` moved the hidden graph's selection to another commit -
/// nothing on screen said so, and the pane the user was reading is what
/// changed next.
#[test]
fn a_click_on_the_detail_tab_does_not_move_the_hidden_graph() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let mut state = stopped_with_the_pane_open(&app, state);
    state.size = mergewright::update::Size {
        columns: 80,
        rows: 24,
    };

    let selected = |state: &AppState| {
        state
            .history
            .as_ref()
            .and_then(mergewright::history::History::selected)
            .map(|found| String::from_utf8_lossy(found.commit.subject()).into_owned())
            .unwrap_or_default()
    };

    for banner_focus in [false, true] {
        let mut on_screen = state.clone();
        on_screen.banner_focus = banner_focus;
        let drawn = frame(&on_screen, 80, 24);
        let at = format!("banner_focus={banner_focus}:\n{}", drawn.join("\n"));
        let row = drawn
            .iter()
            .position(|line| line.contains("notes.md"))
            .expect("the conflicted file is on the frame");
        let row = u16::try_from(row).expect("24 rows fit a u16");
        let clicked = app.mouse_down(&on_screen, MouseButton::Left, 4, row);
        assert_eq!(
            selected(&clicked),
            selected(&on_screen),
            "a click on a file name selected another commit {at}"
        );
        assert!(
            !clicked.on_working_tree || on_screen.on_working_tree,
            "nor the uncommitted row {at}"
        );
    }
    drop(state);
    app.close();
}
