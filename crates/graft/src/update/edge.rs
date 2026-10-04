//! The edge: where a pressed key meets his `[keys]` remaps (ADR-0055 §3–4).
//!
//! One call, ahead of every dispatcher: a key pressed into a text field goes
//! on as typed; otherwise it is translated in the context the claim chain
//! would hand it to — a moved action's new chord becomes its default chord,
//! a default whose action moved away becomes a key nobody answers — and the
//! dispatchers below only ever see default chords. They are not edited and
//! do not know remapping exists.

use std::fmt;
use std::sync::Arc;

use graft_core::event::KeyPress;
use graft_ui::detail::DetailFocus;
use graft_ui::keymap::{ContextId, KeyChord, Keymap, Translated, defaults};

use super::detail::DetailSpec;
use super::{AppState, Command};

/// The process's one keymap, shared by every screen it opens.
///
/// A wrapper because `AppState` is `Clone + Debug + Eq` and
/// `graft_ui::keymap::Keymap` derives none of them: cloning an `Arc` is what
/// "one keymap per process" means anyway, and two are equal when every
/// action has the same chords.
#[derive(Clone, Default)]
pub struct SharedKeymap(Arc<Keymap>);

impl SharedKeymap {
    /// Share `keymap`.
    #[must_use]
    pub fn new(keymap: Keymap) -> Self {
        Self(Arc::new(keymap))
    }

    /// The keymap itself.
    #[must_use]
    pub fn get(&self) -> &Keymap {
        &self.0
    }
}

impl PartialEq for SharedKeymap {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || defaults()
                .iter()
                .all(|row| self.0.chords(&row.action) == other.0.chords(&row.action))
    }
}

impl Eq for SharedKeymap {}

impl fmt::Debug for SharedKeymap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let moved = defaults()
            .iter()
            .filter(|row| self.0.chords(&row.action) != row.default)
            .count();
        write!(f, "SharedKeymap({moved} moved)")
    }
}

/// A key, from the terminal to the dispatchers (`core_event`'s `Key` arm).
pub(super) fn key(next: &mut AppState, commands: &mut Vec<Command>, pressed: KeyPress) {
    // Typed text is never remapped (§2.1): every field `text_field_open`
    // names takes the press as it came.
    if next.text_field_open() {
        super::dispatch_key(next, commands, pressed);
        return;
    }
    let Some(chord) = KeyChord::from_press(&pressed) else {
        // Command held: no chord can name it, so no remap can move it.
        super::dispatch_key(next, commands, pressed);
        return;
    };
    let context = context_of(next);
    match next.keymap.get().translate(context, chord) {
        Translated::Pass => super::dispatch_key(next, commands, pressed),
        Translated::Default(default) => {
            super::dispatch(next, commands, default.to_press(), pressed);
        }
        Translated::Unbound => unanswered(next, pressed, context),
    }
}

/// What a key no dispatcher answers does (a default whose action was moved):
/// whatever happens to *any* key before it is answered — an unseen failure
/// dismissed, or a `Done` acknowledged — a badge in the air dropped, and the
/// `.` menu closed, which every key it does not claim does
/// (`context_menu_key`). Nothing else, and no status.
fn unanswered(next: &mut AppState, pressed: KeyPress, context: ContextId) {
    if super::keys::before_any_key(next, &pressed) {
        return;
    }
    next.drag = None;
    if context == ContextId::Menu {
        next.context_menu = None;
    }
}

/// The context the claim chain (`core_event`, then `keys::key_event`) would
/// hand a key to, in the order it claims keys.
///
/// A text field over a place is that place: the edge passes the field's keys
/// before this matters. Never `Tour` (no tour yet) or `Palette` (ticket 11's
/// to add); the name prompt has no context — it is a text field.
#[must_use]
pub(crate) fn context_of(state: &AppState) -> ContextId {
    use ContextId as C;
    let detail_focused = state.detail.shown && state.detail.focus != DetailFocus::Graph;
    let graph_focused = !detail_focused;
    let chain: [(bool, ContextId); 16] = [
        (state.credential.is_some(), C::Credential),
        (state.drag.is_some(), C::Graph),
        (state.confirm.is_some(), C::Confirm),
        (state.preview.is_some(), C::Preview),
        (state.rewrite.editor.is_some(), C::Rewrite),
        (state.finish.is_some(), C::Finish),
        (state.resolver.is_some(), C::Resolver),
        (super::picker::shown(state), C::Picker),
        (state.banner_focus, C::Banner),
        (state.trail.on_screen().is_some(), C::Blame),
        (state.trail.history_on_screen().is_some(), C::History),
        (state.context_menu.is_some(), C::Menu),
        (state.commit_editor.is_some(), C::CommitEditor),
        // The overlays take the keys before the pane only while the graph
        // has focus (`keys::pane_and_overlay_key`).
        (graph_focused && state.journal.shown, C::Journal),
        (graph_focused && state.stash.shown, C::Stash),
        (graph_focused && state.map.shown, C::Map),
    ];
    if let Some((_, context)) = chain.iter().find(|(up, _)| *up) {
        return *context;
    }
    if detail_focused {
        return if state.detail.spec == Some(DetailSpec::WorkingTree) {
            C::Changes
        } else {
            C::Diff
        };
    }
    C::Graph
}
