//! mergewright-ops' words spelled from his keymap (ticket m8/10e, 10b Ruling).
//!
//! mergewright-ops names a key by what it does ([`OpKey`]) and asks a
//! [`KeyNames`] for its label; this is the one place that answers, from the
//! keymap row that acts where he presses it. Every caller hands mergewright-ops
//! `&OpKeys(&state.keymap)` — a job that words on its own thread holds an
//! `Arc<Keymap>` cloned at dispatch.

use mergewright_ops::keys::{KeyNames, OpKey};
use mergewright_ui::keymap::{ActionId, Keymap};

/// The row that answers `key` where his words send him (10b Ruling W2): the
/// graph for the graph's keys, the banner for continue and abort, the
/// changes pane for stage. No catch-all: a new [`OpKey`] fails to compile
/// here until it has a row.
pub(crate) fn op_action(key: OpKey) -> ActionId {
    ActionId(match key {
        OpKey::Commit => "graph.commit",
        OpKey::Amend => "graph.amend",
        OpKey::Continue => "operation.continue",
        OpKey::Abort => "operation.abort",
        OpKey::Stage => "changes.stage",
        OpKey::Stash => "graph.stash",
        OpKey::Switch => "graph.switch",
        OpKey::Rewrite => "graph.rewrite",
        OpKey::Fetch => "graph.fetch",
        OpKey::Pull => "graph.pull",
        OpKey::Push => "graph.push",
        OpKey::Merge => "graph.merge",
        OpKey::Rebase => "graph.rebase",
        OpKey::Undo => "app.undo",
        OpKey::Redo => "app.redo",
    })
}

/// His keymap, as mergewright-ops asks it.
pub(crate) struct OpKeys<'a>(pub &'a Keymap);

impl KeyNames for OpKeys<'_> {
    fn label(&self, key: OpKey) -> Option<String> {
        self.0.spell_keys(&[op_action(key)])
    }
}

#[cfg(test)]
mod tests {
    use mergewright_ui::keymap::{ContextId, KeyChord, defaults};

    use super::*;

    /// Each key, the place he presses it, its default chord, and the letter
    /// mergewright-ops wrote before it spelled from the keymap.
    const PINS: &[(OpKey, ContextId, char, &str)] = &[
        (OpKey::Commit, ContextId::Graph, 'c', "c"),
        (OpKey::Amend, ContextId::Graph, 'a', "a"),
        (OpKey::Continue, ContextId::Banner, 'c', "c"),
        (OpKey::Abort, ContextId::Banner, 'a', "a"),
        (OpKey::Stage, ContextId::Changes, ' ', "space"),
        (OpKey::Stash, ContextId::Graph, 'S', "S"),
        (OpKey::Switch, ContextId::Graph, 'C', "C"),
        (OpKey::Rewrite, ContextId::Graph, 'R', "R"),
        (OpKey::Fetch, ContextId::Graph, 'f', "f"),
        (OpKey::Pull, ContextId::Graph, 'p', "p"),
        (OpKey::Push, ContextId::Graph, 'P', "P"),
        (OpKey::Merge, ContextId::Graph, 'm', "m"),
        (OpKey::Rebase, ContextId::Graph, 'r', "r"),
        (OpKey::Undo, ContextId::Graph, 'u', "u"),
        (OpKey::Redo, ContextId::Graph, 'U', "U"),
    ];

    #[test]
    fn every_op_key_is_a_row_and_spells_todays_letter() {
        assert_eq!(PINS.len(), OpKey::ALL.len(), "one pin per key");
        for key in OpKey::ALL {
            let (_, ctx, code, letter) = PINS
                .iter()
                .find(|(pinned, ..)| pinned == key)
                .unwrap_or_else(|| panic!("{key:?} has no pin"));
            let id = op_action(*key);
            assert!(
                defaults().iter().any(|row| row.action == id),
                "{key:?}: {} is a row of the table",
                id.0
            );
            // W2: the row is the one his press answers where the words
            // send him.
            assert_eq!(
                Keymap::shipped().acting(*ctx, KeyChord::key(*code)),
                Some(id),
                "{key:?} in {ctx:?}"
            );
            assert_eq!(
                OpKeys(Keymap::shipped()).label(*key).as_deref(),
                Some(*letter),
                "{key:?}"
            );
        }
    }
}
