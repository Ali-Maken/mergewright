use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;

use super::*;

fn labels(menu: &ContextMenu) -> Vec<&str> {
    menu.items.iter().map(|item| item.label.as_str()).collect()
}

/// The row's own actions, then the keys the bar drops
/// ([`always_items`]): `. more` has to lead somewhere (m5-03 review,
/// Major 1).
#[test]
fn for_row_omits_explain_when_not_marked() {
    let menu = ContextMenu::for_row(false, Vec::new(), 0, None, Keymap::shipped());
    assert!(
        !menu
            .items
            .iter()
            .any(|item| item.action == ContextAction::Explain)
    );
    assert_eq!(
        menu.items
            .iter()
            .map(|item| item.action)
            .collect::<Vec<_>>(),
        vec![
            ContextAction::CopySha,
            ContextAction::CompareWithThis,
            ContextAction::OpenDetails,
            ContextAction::Fetch,
            ContextAction::Pull,
            ContextAction::Push,
            ContextAction::Undo,
        ]
    );
}

#[test]
fn for_row_lists_explain_first_when_marked() {
    let menu = ContextMenu::for_row(true, Vec::new(), 0, None, Keymap::shipped());
    assert_eq!(menu.items[0].action, ContextAction::Explain);
    assert_eq!(menu.items.len(), 8);
}

/// Every label but `Explain`'s names the key that does the same thing,
/// so the menu teaches the keyboard rather than replacing it. A history
/// verb the caller passes in follows the same rule: it names its key when
/// it has one (`Merge … (m)`, `Rebase … (r)`) and carries none when it
/// does not — owning no letter is why the rest of them live behind `.` at
/// all (`docs/design/02-keymap.md` §2.1, "Rare verbs live behind `.`").
/// So a trailing `(key)` anywhere in this menu means the same thing:
/// next time, you can skip the menu.
#[test]
fn every_label_names_its_key() {
    let menu = ContextMenu::for_row(true, Vec::new(), 0, None, Keymap::shipped());
    assert_eq!(
        labels(&menu),
        vec![
            "Explain",
            "Copy SHA (y)",
            "Compare with this (x)",
            "Open details (⏎)",
            "Fetch (f)",
            "Pull (p)",
            "Push (P)",
            "Undo (u)",
        ]
    );
}

/// The app's history verbs go between the row's own actions and the
/// dropped keys, each carrying the index the app will map back to an
/// operation.
#[test]
fn history_verbs_sit_between_the_rows_actions_and_the_dropped_keys() {
    let menu = ContextMenu::for_row(
        false,
        vec![
            (sanitize_line("Checkout feature"), None),
            (sanitize_line("Merge feature into main (m)"), None),
        ],
        0,
        None,
        Keymap::shipped(),
    );
    assert_eq!(
        menu.items
            .iter()
            .map(|item| item.action)
            .collect::<Vec<_>>(),
        vec![
            ContextAction::CopySha,
            ContextAction::CompareWithThis,
            ContextAction::OpenDetails,
            ContextAction::History(0),
            ContextAction::History(1),
            ContextAction::Fetch,
            ContextAction::Pull,
            ContextAction::Push,
            ContextAction::Undo,
        ]
    );
    assert_eq!(menu.items[3].label.as_str(), "Checkout feature");
    assert_eq!(menu.items[4].label.as_str(), "Merge feature into main (m)");
}

/// The index the widget reports is the position in the list the app
/// passed, not the position in the menu: `Explain` shifting every row
/// down must not change which operation a verb means.
#[test]
fn a_history_index_names_the_apps_own_list_not_the_menu_row() {
    let history = vec![(sanitize_line("Merge feature into main (m)"), None)];
    let plain = ContextMenu::for_row(false, history.clone(), 0, None, Keymap::shipped());
    let marked = ContextMenu::for_row(true, history, 0, None, Keymap::shipped());
    let of = |menu: &ContextMenu| {
        menu.items
            .iter()
            .find_map(|item| match item.action {
                ContextAction::History(index) => Some(index),
                _ => None,
            })
            .expect("the verb is listed")
    };
    assert_eq!(of(&plain), 0);
    assert_eq!(of(&marked), 0);
}

/// 09b ruling B (keymap §2.1): absorb is the uncommitted row's only row
/// verb, so it leads, as Checkout leads a commit row; the menu opens on
/// it.
#[test]
fn absorb_is_the_first_item_on_the_uncommitted_row() {
    let menu = ContextMenu::for_working_tree(true, Keymap::shipped());
    let absorb = &menu.items[0];
    assert_eq!(absorb.action, ContextAction::Absorb);
    assert_eq!(
        absorb.label.as_str(),
        "Put staged changes into their commits (absorb)"
    );
    assert_eq!(absorb.disabled, None, "something is staged: it acts");
    assert_eq!(menu.cursor, 0);
}

/// With nothing staged absorb keeps its first place, dim, reading
/// `nothing staged` — the cursor still opens on it, as a commit row's
/// opens on its dim Checkout, and `Enter` answers with the reason.
#[test]
fn absorb_dims_in_first_place_with_nothing_staged() {
    let menu = ContextMenu::for_working_tree(false, Keymap::shipped());
    assert_eq!(menu.items[0].action, ContextAction::Absorb);
    assert_eq!(menu.cursor, 0);
    assert_eq!(
        menu.selected_refusal().map(SanitizedText::as_str),
        Some("nothing staged")
    );
}

/// The five items the row had before absorb, in their old order, one
/// place down, staged or not.
#[test]
fn for_working_tree_keeps_its_old_order_below_absorb() {
    for staged in [true, false] {
        let menu = ContextMenu::for_working_tree(staged, Keymap::shipped());
        assert_eq!(
            menu.items[1..]
                .iter()
                .map(|item| item.action)
                .collect::<Vec<_>>(),
            vec![
                ContextAction::OpenDetails,
                ContextAction::Fetch,
                ContextAction::Pull,
                ContextAction::Push,
                ContextAction::Undo,
            ]
        );
    }
}

/// `cursor` is a public field, so nothing in this crate stops a caller
/// setting it out of range; `selected_refusal` must not panic if one
/// does (review m5-10-30-31 m8) — `.get` over indexing, and `None`
/// reads as "no refusal", same as a real item with nothing disabled.
#[test]
fn selected_refusal_on_an_out_of_range_cursor_is_none_not_a_panic() {
    let mut menu = ContextMenu::for_row(true, Vec::new(), 0, None, Keymap::shipped());
    menu.cursor = menu.items.len() + 5;
    assert_eq!(menu.selected_refusal(), None);
}

#[test]
fn move_cursor_clamps_at_both_ends() {
    let mut menu = ContextMenu::for_row(true, Vec::new(), 0, None, Keymap::shipped());
    menu.move_cursor(-5);
    assert_eq!(menu.cursor, 0);
    menu.move_cursor(100);
    assert_eq!(menu.cursor, menu.items.len() - 1);
}

#[test]
fn selected_follows_cursor() {
    let mut menu = ContextMenu::for_row(true, Vec::new(), 0, None, Keymap::shipped());
    menu.move_cursor(1);
    assert_eq!(menu.selected(), ContextAction::CopySha);
}

#[test]
fn width_is_longest_label_plus_four() {
    let menu = ContextMenu::for_row(true, Vec::new(), 0, None, Keymap::shipped());
    // "Compare with this (x)" is 21 cells, the longest label.
    assert_eq!(menu.width(), 21 + 4);
}

/// A history verb's label is usually the longest thing in the menu, so
/// the box has to grow to it rather than clip it.
#[test]
fn width_grows_to_a_history_verbs_label() {
    let menu = ContextMenu::for_row(
        false,
        vec![(sanitize_line("Fix: reset main to before the merge"), None)],
        0,
        None,
        Keymap::shipped(),
    );
    assert_eq!(menu.width(), 35 + 4);
}

/// The label a repository supplies is cleaned before it can be drawn:
/// `MenuItem::label` is a [`SanitizedText`], so a branch named with an
/// escape sequence cannot reach the terminal (`DESIGN.md` §8.2).
#[test]
fn a_ref_name_carrying_an_escape_cannot_reach_the_buffer() {
    let menu = ContextMenu::for_row(
        false,
        vec![(sanitize_line("Checkout \u{1b}[31mevil"), None)],
        0,
        None,
        Keymap::shipped(),
    );
    let label = menu.items[3].label.as_str();
    assert!(!label.contains('\u{1b}'), "{label:?}");
}

/// **The key hint is the first thing a narrow pane drops.** `width()`
/// asks for the longest label plus four, but the caller clamps that to
/// the graph pane it draws in (`mergewright::update::geometry::
/// context_menu_bounds`), and `fit` cuts from the right — where the
/// `(key)` is. A long ref name in a narrow pane therefore loses the very
/// thing the label was teaching, and loses it silently: the `…` says
/// something was cut, not what.
///
/// Pinned here rather than fixed: it is the hint bar's problem over again
/// (ADR-0039), and the answer belongs with that one, not in this widget.
#[test]
fn a_pane_too_narrow_for_the_label_cuts_the_key_hint_off_the_end() {
    let menu = ContextMenu::for_row(
        false,
        vec![(
            sanitize_line("Merge release/2026-q1-hotfix into main (m)"),
            None,
        )],
        0,
        None,
        Keymap::shipped(),
    );
    assert_eq!(menu.width(), 42 + 4, "it asks for the whole label");

    let area = Rect {
        x: 0,
        y: 0,
        width: 24,
        height: u16::try_from(menu.items.len()).expect("a short menu"),
    };
    let mut buffer = Buffer::empty(area);
    (&menu).render(area, &mut buffer);
    let row: String = (0..area.width).map(|x| buffer[(x, 3)].symbol()).collect();

    assert!(row.starts_with(" Merge release/"), "{row:?}");
    assert!(!row.contains("(m)"), "the key hint is gone: {row:?}");
    assert!(
        row.ends_with('\u{2026}'),
        "and only an ellipsis says so: {row:?}"
    );
}

#[test]
fn for_working_tree_width_is_its_own_longest_label_plus_four() {
    let menu = ContextMenu::for_working_tree(true, Keymap::shipped());
    // "Put staged changes into their commits (absorb)" is 46 characters.
    assert_eq!(menu.width(), 46 + 4);
    // Dim, it carries its reason: ` — nothing staged` is 17 more.
    assert_eq!(
        ContextMenu::for_working_tree(false, Keymap::shipped()).width(),
        46 + 17 + 4
    );
}

/// m6-conflicts/71: the dim slot, then the leading verbs, then
/// `Explain`, then the row's actions, then the rest of the verbs.
#[test]
fn for_row_places_dim_first_and_leading_before_explain() {
    let history = vec![
        (sanitize_line("Cherry-pick abc1234"), None),
        (sanitize_line("Revert abc1234"), None),
    ];
    let menu = ContextMenu::for_row(
        true,
        history,
        1,
        Some((sanitize_line("Checkout"), sanitize_line("already on main"))),
        Keymap::shipped(),
    );
    assert_eq!(
        menu.items
            .iter()
            .map(|item| item.action)
            .collect::<Vec<_>>(),
        vec![
            ContextAction::Unavailable,
            ContextAction::History(0),
            ContextAction::Explain,
            ContextAction::CopySha,
            ContextAction::CompareWithThis,
            ContextAction::OpenDetails,
            ContextAction::History(1),
            ContextAction::Fetch,
            ContextAction::Pull,
            ContextAction::Push,
            ContextAction::Undo,
        ]
    );
    assert_eq!(
        menu.items[0].disabled.as_ref().map(SanitizedText::as_str),
        Some("already on main")
    );
    assert!(menu.items[1..].iter().all(|item| item.disabled.is_none()));
    assert_eq!(
        menu.selected_refusal().map(SanitizedText::as_str),
        Some("already on main")
    );
}

/// m7-power-tools/11: a history item the app dims (`Cherry-pick (o) —
/// already on main`) is drawn and acts exactly as `dim_first` does: its
/// row reads `label — reason`, `DIM`, and it is
/// [`ContextAction::Unavailable`], so it is never dispatched. The items
/// after it keep their indices into the app's list.
#[test]
fn a_dimmed_history_item_draws_label_dash_reason_and_is_unavailable() {
    let menu = ContextMenu::for_row(
        false,
        vec![
            (sanitize_line("Checkout feature"), None),
            (
                sanitize_line("Cherry-pick (o)"),
                Some(sanitize_line("already on main")),
            ),
            (sanitize_line("Revert abc1234"), None),
        ],
        2,
        None,
        Keymap::shipped(),
    );
    assert_eq!(
        menu.items
            .iter()
            .map(|item| item.action)
            .collect::<Vec<_>>(),
        vec![
            ContextAction::History(0),
            ContextAction::Unavailable,
            ContextAction::CopySha,
            ContextAction::CompareWithThis,
            ContextAction::OpenDetails,
            ContextAction::History(2),
            ContextAction::Fetch,
            ContextAction::Pull,
            ContextAction::Push,
            ContextAction::Undo,
        ]
    );
    let mut moved = menu.clone();
    moved.move_cursor(1);
    assert_eq!(
        moved.selected_refusal().map(SanitizedText::as_str),
        Some("already on main")
    );

    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: u16::try_from(menu.items.len()).expect("a short menu"),
    };
    let mut buffer = Buffer::empty(area);
    (&menu).render(area, &mut buffer);
    let row: String = (0..area.width).map(|x| buffer[(x, 1)].symbol()).collect();
    assert_eq!(row.trim_end(), " Cherry-pick (o) \u{2014} already on main");
    assert!(buffer[(1, 1)].modifier.contains(Modifier::DIM));
    assert!(!buffer[(1, 2)].modifier.contains(Modifier::DIM));
}

/// No dims: the same items, actions and labels a plain list gave.
#[test]
fn for_row_with_no_dims_draws_as_before() {
    let menu = ContextMenu::for_row(
        true,
        vec![
            (sanitize_line("Checkout feature"), None),
            (sanitize_line("Cherry-pick abc1234 (o)"), None),
        ],
        2,
        None,
        Keymap::shipped(),
    );
    assert_eq!(
        labels(&menu),
        [
            "Checkout feature",
            "Cherry-pick abc1234 (o)",
            "Explain",
            "Copy SHA (y)",
            "Compare with this (x)",
            "Open details (\u{23ce})",
            "Fetch (f)",
            "Pull (p)",
            "Push (P)",
            "Undo (u)",
        ]
    );
    assert!(menu.items.iter().all(|item| item.disabled.is_none()));
    assert!(
        !menu
            .items
            .iter()
            .any(|item| item.action == ContextAction::Unavailable)
    );
}
