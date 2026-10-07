//! Every key binding as data, and the translate that lets him remap them
//! (ADR-0055; `docs/design/02-keymap.md`).
//!
//! - [`defaults()`] is the table: one [`Binding`] per action, its
//!   [`ActionId`] (his config's name for it, never renamed after release),
//!   the place it lives ([`ContextId`]), its default chords and its label.
//! - [`Keymap`] is the table with his overrides applied. The app calls
//!   [`Keymap::translate`] once per key press, before any pane sees it: a
//!   remapped chord becomes the action's default chord, and a default whose
//!   action moved elsewhere becomes [`Translated::Unbound`]. No dispatcher is
//!   rewritten.
//! - Every surface that prints a key spells it with [`Keymap::spell`].

mod chord;
mod clash;
mod resolve;
mod table;
mod warning;

#[cfg(test)]
pub(crate) mod sweep;
#[cfg(test)]
mod tests;

pub use chord::{ChordError, KeyChord, KeyLabel};
pub use graft_core::event::KeyCode;
pub(crate) use resolve::join;
pub use resolve::{Keymap, Translated};
pub use table::defaults;
pub use warning::{Clash, KeyWarning};

/// A place keys are bound in: a pane, an overlay, a box, or `Global`
/// (every place).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextId {
    /// Every place (§2.2).
    Global,
    /// The graph pane (§2.3).
    Graph,
    /// The Map view (§2.3a).
    Map,
    /// The Ghost Preview box (§2.3b).
    Preview,
    /// A confirm box (§2.3c).
    Confirm,
    /// The working-tree changes in the detail pane (§2.4).
    Changes,
    /// A diff, wherever one is shown (§2.5).
    Diff,
    /// The conflict resolver (§2.6).
    Resolver,
    /// The operation banner (§2.6a).
    Banner,
    /// The finish summary (§2.6b).
    Finish,
    /// The interactive rebase (rewrite) editor (§2.7).
    Rewrite,
    /// The branch picker (§2.16), a text field.
    Picker,
    /// Blame (§2.8).
    Blame,
    /// The stash list (§2.9).
    Stash,
    /// The commit message editor (§2.10), a text field.
    CommitEditor,
    /// The undo journal browser (§2.11).
    Journal,
    /// A file's or some lines' history (§2.17).
    History,
    /// The command palette and help (§2.12), a text field.
    Palette,
    /// The screen outside a repository (§2.13).
    EmptyState,
    /// The practice tour (§2.18). It binds no key of its own.
    Tour,
    /// The credential prompt (§2.14), a text field.
    Credential,
    /// The `.` context menu (§2.1): movement, confirm and its own opener.
    Menu,
}

use ContextId as C;

impl ContextId {
    /// Every place, `Global` first.
    pub const ALL: &'static [ContextId] = &[
        C::Global,
        C::Graph,
        C::Map,
        C::Preview,
        C::Confirm,
        C::Changes,
        C::Diff,
        C::Resolver,
        C::Banner,
        C::Finish,
        C::Rewrite,
        C::Picker,
        C::Blame,
        C::Stash,
        C::CommitEditor,
        C::Journal,
        C::History,
        C::Palette,
        C::EmptyState,
        C::Tour,
        C::Credential,
        C::Menu,
    ];

    /// The `[keys.<name>]` table his config writes for this place, and the
    /// prefix of every action id in it.
    #[must_use]
    pub fn config_name(self) -> &'static str {
        match self {
            C::Global => "app",
            C::Graph => "graph",
            C::Map => "map",
            C::Preview => "preview",
            C::Confirm => "confirm",
            C::Changes => "changes",
            C::Diff => "diff",
            C::Resolver => "resolver",
            C::Banner => "operation",
            C::Finish => "finish",
            C::Rewrite => "rewrite",
            C::Picker => "picker",
            C::Blame => "blame",
            C::Stash => "stash",
            C::CommitEditor => "commit",
            C::Journal => "journal",
            C::History => "history",
            C::Palette => "palette",
            C::EmptyState => "empty_state",
            C::Tour => "tour",
            C::Credential => "credential",
            C::Menu => "menu",
        }
    }

    /// Where a key pressed in this place may still land, innermost first,
    /// `Global` last. It may over-include (a default the code ignores here
    /// does nothing, as pressing it does); it must never under-include, or a
    /// remap would fail here. Not for help: `?` lists [`Keymap::in_context`].
    #[must_use]
    pub fn scope(self) -> &'static [ContextId] {
        match self {
            C::Graph => &[C::Graph, C::Diff, C::Global],
            C::Changes => &[C::Changes, C::Diff, C::Graph, C::Global],
            C::Diff => &[C::Diff, C::Graph, C::Global],
            C::Map => &[C::Map, C::Graph, C::Global],
            C::Journal => &[C::Journal, C::Graph, C::Global],
            C::Stash => &[C::Stash, C::Graph, C::Global],
            C::Menu => &[C::Menu, C::Changes, C::Diff, C::Graph, C::Global],
            C::Blame => &[C::Blame, C::Diff, C::Global],
            C::History => &[C::History, C::Diff, C::Global],
            C::Preview => &[C::Preview, C::Global],
            C::Confirm => &[C::Confirm, C::Global],
            C::Resolver => &[C::Resolver, C::Global],
            C::Banner => &[C::Banner, C::Global],
            C::Finish => &[C::Finish, C::Global],
            C::Rewrite => &[C::Rewrite, C::Global],
            C::Picker => &[C::Picker, C::Global],
            C::CommitEditor => &[C::CommitEditor, C::Global],
            C::Credential => &[C::Credential, C::Global],
            C::Palette => &[C::Palette, C::Global],
            C::EmptyState => &[C::EmptyState, C::Global],
            C::Tour | C::Global => &[C::Global],
        }
    }

    /// The rows of other places this place answers itself, ahead of
    /// anything it falls through to: a key resolves in its own rows, then
    /// these, then the rest of [`ContextId::scope`] in order (ticket m8/09c,
    /// D1). Read off the claim chain (`graft::update::keys::key_event`), one
    /// comment per place naming the line that claims them. Empty for
    /// `Graph`, `Global` and text fields.
    #[must_use]
    pub fn borrows(self) -> &'static [ActionId] {
        match self {
            C::Menu => MENU_BORROWS,
            C::Changes | C::Diff => PANE_BORROWS,
            C::Map | C::Stash => LIST_BORROWS,
            C::Journal => JOURNAL_BORROWS,
            C::Blame => BLAME_BORROWS,
            C::History => HISTORY_BORROWS,
            // The one confirm key (§2.3b), each place's own `y`:
            // `update/preview/keys.rs:78` `confirm_key`, `update/finish/
            // keys.rs:98` `files_key`, `update/rewrite_editor/keys.rs:171`,
            // and the init box in graft's `empty_state.rs` `outside_key`.
            C::Preview | C::Finish | C::Rewrite | C::EmptyState => &[ActionId("confirm.yes")],
            C::Global
            | C::Graph
            | C::Confirm
            | C::Resolver
            | C::Banner
            | C::Picker
            | C::CommitEditor
            | C::Palette
            | C::Tour
            | C::Credential => &[],
        }
    }

    /// A text field: typed keys stay as they are. Its rows are fixed — in
    /// the table for `?` and the man page, never remapped — and
    /// [`Keymap::translate`] passes every key here untouched.
    #[must_use]
    pub fn is_text_field(self) -> bool {
        matches!(
            self,
            C::Picker | C::Palette | C::CommitEditor | C::Credential
        )
    }
}

// The rows each place answers itself (`ContextId::borrows`), with the
// line in graft that claims them.

// `context_menu_key` claims j/k, the arrows, ⏎, esc and `.`
// (graft `update/context_menu.rs:133-161`); every other key
// closes the menu and falls through.
const MENU_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("app.menu"),
];

// `detail_key` (graft `update/detail/keys.rs:123-167` ⏎, esc,
// ⇥/⇧⇥; `:285`, `:326` j/k, PgDn/PgUp, g/G in files and diff),
// before `graph_key`.
const PANE_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("app.next_pane"),
    ActionId("app.prev_pane"),
];

// `map_key` (graft `update/map.rs:159-172`): esc, j/k, ⏎.
const LIST_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
];

// `journal::shown_key` (graft `update/journal/keys.rs:83-108`):
// esc, j/k, n/N, ⏎. (`stash::shown_key`, `update/stash.rs:204-218`,
// claims esc, j/k, ⏎ — `LIST_BORROWS`.)
const JOURNAL_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.open"),
    ActionId("app.back"),
    ActionId("app.next_match"),
    ActionId("app.prev_match"),
];

// `place_claim` (graft `update/keys.rs:124-130`) ahead of
// everything; blame's keys `update/place/keys.rs:54-77`.
const BLAME_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.top"),
    ActionId("app.bottom"),
    ActionId("app.find"),
    ActionId("app.next_match"),
    ActionId("app.prev_match"),
    ActionId("app.open"),
    ActionId("app.back"),
];

// `place_claim`, then `update/place/history_keys.rs:29-52`.
const HISTORY_BORROWS: &[ActionId] = &[
    ActionId("app.down"),
    ActionId("app.up"),
    ActionId("app.page_down"),
    ActionId("app.page_up"),
    ActionId("app.next_pane"),
    ActionId("app.open"),
    ActionId("app.back"),
];

/// The config switches that change which chords a row has out of the box
/// (ticket m8/09c, D2). `Default`: every switch off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeySwitches {
    /// `[keys] suspend_on_ctrl_z`: `⌃Z` is `app.suspend`, and undo keeps
    /// only `u`.
    pub suspend_on_ctrl_z: bool,
}

/// An action's name in his config: `<context>.<verb>`, `snake_case`
/// (`graph.merge`). Never renamed after release; a rename keeps the old id
/// as an alias (ADR-0055 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionId(pub &'static str);

/// One action in the table.
pub struct Binding {
    /// His config's name for it.
    pub action: ActionId,
    /// The place it is bound in.
    pub context: ContextId,
    /// The chords it answers out of the box; the first is the one spelled.
    pub default: &'static [KeyChord],
    /// The doc's words for it, as a palette row reads them.
    pub label: &'static str,
    /// Words the palette also finds it by (§2.12: `aside` finds stash).
    pub search: &'static [&'static str],
}
