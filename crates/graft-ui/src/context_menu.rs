//! The `.`/right-click popup: a small list of actions for the row that
//! opened it (`context-actions-menu`, m2-details-and-diffs/12).
//!
//! The widget draws only the item labels and the cursor row's emphasis; it
//! carries no theme (the exact signature this ticket ships), so the
//! `surface-raised` background the design note asks for is painted by the
//! caller directly on the buffer before this renders on top — the same split
//! `graph_screen`'s row emphasis uses (background separately from text).

use graft_term::{SanitizedText, sanitize_line};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::keymap::{ActionId, Keymap};

/// An action a context-menu item dispatches. `apply_context_action` (in the
/// app crate) carries each one out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAction {
    /// Open the explain panel for a marked row (only ever offered there).
    Explain,
    /// Copy the row's full SHA — the same command `y` already sends.
    CopySha,
    /// Pin or complete a compare against this row (`x`'s three branches).
    CompareWithThis,
    /// Open the detail pane on this row, or the working tree.
    OpenDetails,
    /// Fetch, the same as `f`.
    Fetch,
    /// Pull, the same as `p`.
    Pull,
    /// Push, the same as `P` (its confirm box included).
    Push,
    /// Undo the last operation, the same as `u`.
    Undo,
    /// Put each staged hunk into the commit on his branch that last changed
    /// its lines — absorb (m7/09). Only on the uncommitted row's menu.
    Absorb,
    /// One of the history verbs the app offered on this row — checkout,
    /// merge, rebase, cherry-pick, revert, reset, a ref deletion or one of
    /// the Mistake Radar's Fix items — named by its position in the list
    /// the app passed to [`ContextMenu::for_row`].
    ///
    /// The index is all the widget knows. Which git operation the verb
    /// means, and against which tips it is pinned, is the app's to decide,
    /// at the same boundary it assembles every other view model: `ui` never
    /// imports `git` or `ops` (`ARCHITECTURE.md`), and repo-derived text
    /// reaches a widget only as [`SanitizedText`] (`DESIGN.md` §8.2), which
    /// is what the label beside this index is.
    History(usize),
    /// A slot shown only to say why it is empty — the `HEAD` row's dim
    /// `Checkout — already on main` (m6-conflicts/71), or a history verb the
    /// app dimmed (`Cherry-pick (o) — a merge · …`, m7/11). Always drawn
    /// disabled, so `Enter` and a click answer with its reason and never
    /// dispatch it; the app maps it to no command.
    Unavailable,
}

/// One row of the menu.
///
/// The label is [`SanitizedText`] because a history verb names refs and
/// commit subjects out of the repository (`Merge feature into main (m)`), and
/// repo-derived text crosses into a widget only through that newtype.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// What choosing this item does.
    pub action: ContextAction,
    /// Its label.
    pub label: SanitizedText,
    /// Why this item cannot act right now, or `None` when it can.
    ///
    /// A disabled row is **shown, dimmed, with its reason beside it** rather
    /// than hidden: `Rebase feature onto main — check out feature first`
    /// teaches what is missing, and a row that quietly vanished would teach
    /// nothing at all. `Enter` on one puts the reason on the status line and
    /// leaves the menu open, so the user can pick another row without
    /// reopening it (m5/10).
    pub disabled: Option<SanitizedText>,
}

/// The popup itself: its items, an optional heading, and which item the
/// cursor is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenu {
    /// A heading drawn above the items, on a row of its own — the drop
    /// menu's `feature → main`, which says what the drag was before the
    /// verbs say what could be done with it. `None` for the `.` menu, whose
    /// row is already the selected one.
    pub title: Option<SanitizedText>,
    /// The items, in display order (top to bottom).
    pub items: Vec<MenuItem>,
    /// The highlighted item.
    pub cursor: usize,
}

/// The hints the width rule drops first and the user reaches for most: the
/// remote verbs and undo, in the footer's own order. Every menu lists them,
/// since none depends on which row the cursor is on.
/// (A function rather than a `const`: [`SanitizedText`] owns its string, and
/// only [`sanitize_line`] can make one.)
fn always_items(keys: &Keymap) -> [MenuItem; 4] {
    [
        keyed_item(ContextAction::Fetch, "Fetch", "graph.fetch", keys),
        keyed_item(ContextAction::Pull, "Pull", "graph.pull", keys),
        keyed_item(ContextAction::Push, "Push", "graph.push", keys),
        keyed_item(ContextAction::Undo, "Undo", "app.undo", keys),
    ]
}

/// `label (key)`, the key spelled from `keys` (ticket m8/10): the menu
/// teaches the key his keymap gives `action`. An action he left unbound
/// keeps its row — it acts by `⏎` or a click — and loses the suffix, since
/// a `(key)` naming nothing would teach a key that does nothing.
#[must_use]
pub fn with_key(label: &str, action: ActionId, keys: &Keymap) -> String {
    match keys.spell_keys(&[action]) {
        Some(key) => format!("{label} ({key})"),
        None => label.to_owned(),
    }
}

/// [`item`], its label carrying `action`'s key ([`with_key`]).
fn keyed_item(action: ContextAction, label: &str, key: &'static str, keys: &Keymap) -> MenuItem {
    item(action, &with_key(label, ActionId(key), keys))
}

/// A fixed label this crate wrote itself. It still goes through
/// [`sanitize_line`] so that `MenuItem::label` has one constructor rather
/// than two, and no caller has to know which labels are safe by provenance.
fn item(action: ContextAction, label: &str) -> MenuItem {
    MenuItem {
        action,
        label: sanitize_line(label),
        disabled: None,
    }
}

/// The uncommitted row's absorb item (keymap §2.1, `02-keymap.md:167`).
const ABSORB_LABEL: &str = "Put staged changes into their commits (absorb)";

/// What a disabled row's reason is written after: ` — check out main first`.
const REASON_SEPARATOR: &str = " — ";

impl ContextMenu {
    /// A commit row's menu. Order (m6-conflicts/71, fixed so the first two
    /// items never move): `dim_first` when given, drawn disabled with its
    /// reason; then `history[..leading]` — Checkout, then Cherry-pick —
    /// then `Explain` when the row is marked; then the row's own actions;
    /// then the rest of `history`; then the keys the hint bar drops at 80
    /// and 120 columns.
    ///
    /// `. more` is a promise: at 80 columns the bar shows three keys and
    /// says the rest are behind `.`, so the rest have to *be* behind `.`
    /// (ADR-0037 §4; m5-03 review, Major 1). Each label carries its key, so
    /// choosing an item once teaches the key for next time.
    ///
    /// `history` is the history verbs the app offers on this row, in the
    /// order it wants them listed, each `(label, dim)` as [`for_drop`]
    /// takes them; each becomes a [`ContextAction::History`] carrying its
    /// index in `history`, and the app maps that index back to the
    /// operation it means. One with a `Some(reason)` dim (slot 2's
    /// `Cherry-pick (o) — already on main`, m7-power-tools/11) is drawn and
    /// acts as `dim_first` does: `label — reason`, `DIM`,
    /// [`ContextAction::Unavailable`], never dispatched. `leading` is
    /// clamped to `history.len()`. `dim_first` is `(label, reason)` and
    /// carries no index: it is [`ContextAction::Unavailable`].
    ///
    /// It takes no width, so what the menu holds cannot depend on the
    /// terminal's (ADR-0039): the signature is the guarantee.
    ///
    /// Each `(key)` is spelled from `keys` ([`with_key`]).
    ///
    /// [`for_drop`]: ContextMenu::for_drop
    #[must_use]
    pub fn for_row(
        marked: bool,
        history: Vec<(SanitizedText, Option<SanitizedText>)>,
        leading: usize,
        dim_first: Option<(SanitizedText, SanitizedText)>,
        keys: &Keymap,
    ) -> ContextMenu {
        let leading = leading.min(history.len());
        let mut items = Vec::with_capacity(9 + history.len());
        if let Some((label, reason)) = dim_first {
            items.push(MenuItem {
                action: ContextAction::Unavailable,
                label,
                disabled: Some(reason),
            });
        }
        let mut history = history
            .into_iter()
            .enumerate()
            .map(|(index, (label, disabled))| MenuItem {
                // A dimmed verb keeps its label and reason but gives up its
                // index: `Enter` answers with the reason before dispatch, and
                // `Unavailable` makes reaching dispatch anyway a routing bug.
                action: if disabled.is_some() {
                    ContextAction::Unavailable
                } else {
                    ContextAction::History(index)
                },
                label,
                disabled,
            });
        items.extend(history.by_ref().take(leading));
        if marked {
            items.push(item(ContextAction::Explain, "Explain"));
        }
        items.push(keyed_item(
            ContextAction::CopySha,
            "Copy SHA",
            "graph.copy_sha",
            keys,
        ));
        items.push(keyed_item(
            ContextAction::CompareWithThis,
            "Compare with this",
            "graph.compare",
            keys,
        ));
        items.push(keyed_item(
            ContextAction::OpenDetails,
            "Open details",
            "app.open",
            keys,
        ));
        items.extend(history);
        items.extend(always_items(keys));
        ContextMenu {
            title: None,
            items,
            cursor: 0,
        }
    }

    /// The uncommitted-changes row's menu: absorb (m7/09) first — the row's
    /// only verb with no key of its own, so it leads as Checkout leads a
    /// commit row (keymap §2.1, 09b) — then `Open details` (design note,
    /// "Working tree row selected"), then the same dropped keys every menu
    /// carries. With nothing `staged` absorb keeps its place, drawn dim,
    /// reading `nothing staged`; the cursor opens on it either way, as a
    /// commit row's opens on its dim Checkout.
    #[must_use]
    pub fn for_working_tree(staged: bool, keys: &Keymap) -> ContextMenu {
        let mut absorb = item(ContextAction::Absorb, ABSORB_LABEL);
        if !staged {
            absorb.disabled = Some(sanitize_line("nothing staged"));
        }
        let mut items = vec![
            absorb,
            keyed_item(ContextAction::OpenDetails, "Open details", "app.open", keys),
        ];
        items.extend(always_items(keys));
        ContextMenu {
            title: None,
            items,
            cursor: 0,
        }
    }

    /// The menu a dropped branch badge opens over the row it was dropped on
    /// (`docs/design/02-keymap.md` §2.15).
    ///
    /// `title` says what was dropped where (`feature → main`); each of
    /// `verbs` is one of the app's own history verbs, paired with the reason
    /// it cannot run right now — `None` for the ones that can. The actions
    /// are [`ContextAction::History`] indices for the same reason the `.`
    /// menu's are: which git operation a row means is the app's to say, and
    /// `ui` imports neither `git` nor `ops` (`ARCHITECTURE.md`).
    ///
    /// The cursor starts on the first row that can act — a menu opening on a
    /// row `Enter` only refuses on would make the commonest gesture (drop,
    /// press `Enter`) a no-op. All four disabled means it starts at the top,
    /// where the first reason is.
    #[must_use]
    pub fn for_drop(
        title: SanitizedText,
        verbs: Vec<(SanitizedText, Option<SanitizedText>)>,
    ) -> ContextMenu {
        let items: Vec<MenuItem> = verbs
            .into_iter()
            .enumerate()
            .map(|(index, (label, disabled))| MenuItem {
                action: ContextAction::History(index),
                label,
                disabled,
            })
            .collect();
        let cursor = items
            .iter()
            .position(|item| item.disabled.is_none())
            .unwrap_or(0);
        ContextMenu {
            title: Some(title),
            items,
            cursor,
        }
    }

    /// Rows the popup draws: its items, plus the title's own row when it has
    /// one. What the caller sizes the rectangle by.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.items.len() + usize::from(self.title.is_some())
    }

    /// The item `offset` rows down from the popup's top edge, or `None` when
    /// that row is the title (which is not selectable) or past the end.
    /// The one place a screen row is turned back into an item index, so a
    /// click and the cursor cannot count the title differently.
    #[must_use]
    pub fn item_at(&self, offset: usize) -> Option<usize> {
        let index = offset.checked_sub(usize::from(self.title.is_some()))?;
        (index < self.items.len()).then_some(index)
    }

    /// Why the item under the cursor cannot act, or `None` when it can.
    ///
    /// `.get`, not indexing: every constructor today guarantees a non-empty
    /// `items` (`for_drop` is only reached with non-empty `verbs`, `drop_menu`
    /// returns `None` otherwise — this widget crate has no way to check
    /// that from here), so an out-of-range `cursor` is not reachable. `None`
    /// on a miss reads exactly as "no refusal", the same as a real item with
    /// nothing disabled — the call site already handles that, so this
    /// costs nothing and removes a panic path from a widget crate rather
    /// than turning an out-of-range cursor into a silent wrong answer
    /// (review m5-10-30-31 m8).
    #[must_use]
    pub fn selected_refusal(&self) -> Option<&SanitizedText> {
        self.items
            .get(self.cursor)
            .and_then(|item| item.disabled.as_ref())
    }

    /// Move the cursor by `delta` items, clamped to the ends (the files
    /// list's own convention, `graft_ui::detail::FileListView::move_cursor`
    /// — no wrap).
    pub fn move_cursor(&mut self, delta: isize) {
        let last = self.items.len().saturating_sub(1);
        self.cursor = self.cursor.saturating_add_signed(delta).min(last);
    }

    /// The action the cursor is on. Never panics: both constructors always
    /// list at least one item.
    #[must_use]
    pub fn selected(&self) -> ContextAction {
        self.items[self.cursor].action
    }

    /// The longest label's cell width, plus 4 (design note, "Context
    /// actions").
    #[must_use]
    pub fn width(&self) -> u16 {
        let longest = self
            .items
            .iter()
            .map(|item| row_text(item).chars().count())
            .chain(
                self.title
                    .iter()
                    .map(|title| title.as_str().chars().count()),
            )
            .max()
            .unwrap_or(0);
        u16::try_from(longest).unwrap_or(u16::MAX).saturating_add(4)
    }
}

/// One item's whole row, its refusal included: `Rebase f onto main — check
/// out f first`. The one place the two halves are joined, so [`width`] and
/// the draw measure the same string.
///
/// [`width`]: ContextMenu::width
fn row_text(item: &MenuItem) -> String {
    match &item.disabled {
        Some(reason) => format!(
            "{}{REASON_SEPARATOR}{}",
            item.label.as_str(),
            reason.as_str()
        ),
        None => item.label.as_str().to_owned(),
    }
}

impl Widget for &ContextMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let mut offset = 0_u16;
        if let Some(title) = &self.title {
            buf.set_stringn(
                area.x,
                area.y,
                fit(&format!(" {}", title.as_str()), width),
                width,
                Style::new().add_modifier(Modifier::BOLD),
            );
            offset = 1;
        }
        for item in &self.items {
            if offset >= area.height {
                break;
            }
            let index = usize::from(offset) - usize::from(self.title.is_some());
            let mut style = if item.disabled.is_some() {
                // Dim: the row is there to be read, not chosen. `Enter` on it
                // says why in words rather than doing nothing.
                Style::new().add_modifier(Modifier::DIM)
            } else {
                Style::default()
            };
            if index == self.cursor {
                // The accent cursor row: reversed so it reads over whatever
                // background the caller painted underneath. A disabled row
                // still takes it — the cursor must be visible wherever it is.
                style = style.add_modifier(Modifier::REVERSED);
            }
            let text = fit(&format!(" {}", row_text(item)), width);
            let y = area.y.saturating_add(offset);
            buf.set_stringn(area.x, y, text, width, style);
            offset = offset.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
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
    /// the graph pane it draws in (`graft::update::geometry::
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
}
