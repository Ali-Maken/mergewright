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
mod resolve;
mod table;
mod warning;

#[cfg(test)]
mod tests;

pub use chord::{ChordError, KeyChord, KeyLabel};
pub use graft_core::event::KeyCode;
pub use resolve::{Keymap, Translated};
pub use table::defaults;
pub use warning::KeyWarning;

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

/// An action's name in his config: `<context>.<verb>`, snake_case
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
