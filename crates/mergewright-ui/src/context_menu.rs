//! The `.`/right-click popup: a small list of actions for the row that
//! opened it (`context-actions-menu`, m2-details-and-diffs/12).
//!
//! The widget draws only the item labels and the cursor row's emphasis; it
//! carries no theme (the exact signature this ticket ships), so the
//! `surface-raised` background the design note asks for is painted by the
//! caller directly on the buffer before this renders on top — the same split
//! `graph_screen`'s row emphasis uses (background separately from text).

use mergewright_term::{SanitizedText, sanitize_line};

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
    /// list's own convention, `mergewright_ui::detail::FileListView::move_cursor`
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

mod draw;

#[cfg(test)]
mod tests;
