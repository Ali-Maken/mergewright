//! `update` tests: bar over overlay. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// **The bar the Detail tab draws with an overlay behind it answers every
/// key on it** (m5/42, the review's Q1 as m5/36 confirmed it).
///
/// The sweep above now covers this state too, through
/// `HintContext::PaneOverOverlay` — this test is the same claim made where
/// the defect was, so a failure names the overlay and the focus rather than
/// a context enum. Before the fix it reported, for each of the three
/// overlays and both focuses, a bar of the graph's verbs with every one of
/// them dead: `b branch`, `r rebase`, `t tag`, `c commit`, `y sha`,
/// `m merge`, `q quit`, `F2 status`, `F3 renderer`, `x compare`,
/// `+ lanes`, `? keys` — the overlay's `shown_key` claims the keyboard
/// before `graph_key` ever runs.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one sweep over overlays × panes; splitting it hides which cell failed"
)]
fn the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows() {
    use mergewright_ui::graph_screen::{Divider, hints_for};

    use mergewright_ui::graph_screen::TabWord;

    let mut dead: Vec<String> = Vec::new();
    // A commit's pane, `⏎` opening or not; and (m8/09e, 11b row 553) the
    // changes pane from its file list and from its diff, each over a
    // conflicted file or not — `opens` names the pane in the messages.
    let panes = |opens: bool| -> Vec<(String, AppState)> {
        let walk = JobId::new(701);
        let mut panes = vec![(
            format!("commit pane (opens {opens})"),
            commit_pane(walk, opens),
        )];
        for tab in [TabWord::Diff, TabWord::Graph] {
            panes.push((
                format!("changes pane (⇥ {tab:?}, unmerged {opens})"),
                changes_pane_screen(walk, opens, None, tab),
            ));
        }
        panes
    };
    for overlay in EVERY_OVERLAY {
        for (opens, (pane, state)) in [true, false]
            .into_iter()
            .flat_map(|opens| panes(opens).into_iter().map(move |p| (opens, p)))
        {
            let state = settled(overlay_over(state, overlay));
            // The bar the frame really draws over this state, at the width
            // where the Detail tab owns the screen (`view::frame`'s `Tab`
            // layout, the only layout that draws a bar under an overlay).
            let state = AppState {
                size: Size {
                    columns: 80,
                    rows: 48,
                },
                ..state
            };
            assert_eq!(
                crate::update::detail::tab_hint_rows(&state),
                1,
                "{overlay:?} {pane}: the Detail tab reserves its hint row"
            );
            let context = crate::view::rows::pane_hint_context(&state);
            assert_eq!(
                matches!(
                    context,
                    HintContext::ChangesPane {
                        over_overlay: true,
                        ..
                    }
                ),
                state.on_changes_pane(),
                "{overlay:?} {pane}: the changes pane draws its own bar, a commit's the short one \
                 ({context:?}; opens {opens})"
            );
            let idle = nothing_happened(&state);
            for hint in hints_for(
                &context,
                Divider::Hidden,
                80,
                mergewright_ui::keymap::Keymap::shipped(),
            ) {
                if !answered(&state, &idle, &hint) {
                    dead.push(format!(
                        "{overlay:?} {pane} draws `{} {}`",
                        hint.key(),
                        hint.verb
                    ));
                }
            }
        }
    }
    assert!(
        dead.is_empty(),
        "the bar behind an overlay draws {} key(s) nothing answers:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}

/// **Every key on a bar over an overlay changes what he sees** (m8/09e,
/// 11b row 553). The swallow test above counts a key as answered when the
/// state moves — which an invisible prompt does. This one asks the frame:
/// for the commit pane's short bar and the changes pane's own bar, over
/// each of the Map, journal and stash list, each drawn key either changes
/// the drawn frame or emits a command the idle press does not (`e` hands
/// over to `$EDITOR`).
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one sweep over overlays × panes; splitting it hides which cell failed"
)]
fn every_key_on_a_bar_over_an_overlay_changes_what_he_sees() {
    use mergewright_ui::graph_screen::{Divider, TabWord, hints_for};

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 48;
    let walk = JobId::new(701);
    let mut panes: Vec<(String, AppState)> = Vec::new();
    for opens in [true, false] {
        panes.push((
            format!("commit pane (opens {opens})"),
            commit_pane(walk, opens),
        ));
    }
    for tab in [TabWord::Diff, TabWord::Graph] {
        for unmerged in [false, true] {
            panes.push((
                format!("changes pane (⇥ {tab:?}, unmerged {unmerged})"),
                changes_pane_screen(walk, unmerged, None, tab),
            ));
        }
    }
    let mut unseen: Vec<String> = Vec::new();
    let mut pressed = 0;
    for overlay in EVERY_OVERLAY {
        for (pane, state) in &panes {
            let state = AppState {
                size: Size {
                    columns: COLUMNS,
                    rows: ROWS,
                },
                ..settled(overlay_over(state.clone(), overlay))
            };
            let context = crate::view::rows::pane_hint_context(&state);
            let idle = nothing_happened(&state);
            let idle_frame = crate::view::tests::draw(&idle.0, COLUMNS, ROWS);
            for hint in hints_for(
                &context,
                Divider::Hidden,
                COLUMNS,
                mergewright_ui::keymap::Keymap::shipped(),
            ) {
                pressed += 1;
                let seen = presses_for(&hint).into_iter().any(|press| {
                    let (after, commands) = outcome(&state, press);
                    crate::view::tests::draw(&after, COLUMNS, ROWS) != idle_frame
                        || commands.iter().any(|command| !idle.1.contains(command))
                });
                if !seen {
                    unseen.push(format!(
                        "{overlay:?} {pane}: `{} {}`",
                        hint.key(),
                        hint.verb
                    ));
                }
            }
        }
    }
    assert!(pressed > 0, "the bars drew keys");
    assert!(
        unseen.is_empty(),
        "{} key(s) on a bar over an overlay change nothing he can see:\n  {}",
        unseen.len(),
        unseen.join("\n  ")
    );
}
