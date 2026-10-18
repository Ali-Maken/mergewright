//! Words spelled from his keymap (m8/10e–10i): each site's line under the
//! shipped keymap is byte-identical to the literal it replaced (10b Ruling
//! W6), under the swept keymap it names the remapped key and no default
//! letter of a moved action (W1), and an unbound key drops its clause (W3).
//!
//! One submodule per ticket's sites: `ops` (10e, mergewright-ops' words), then
//! `stops`, `resolver`, `undo_journal`, `preview_remote` for 10f–10i.

use std::sync::Arc;

use mergewright_ops::keys::{KeyNames, OpKey};
use mergewright_ui::keymap::{KeySwitches, Keymap};

use crate::update::op_keys::OpKeys;

mod focus;
mod ops;
mod preview_remote;
mod resolve;
mod resolver;
mod stops;
mod undo_journal;

/// Every movable action on its own free chord: `keymap_edge`'s sweep.
fn swept() -> Arc<Keymap> {
    super::keymap_edge::swept().0
}

/// The shipped keymap with `id` unbound (`"none"`, as his config says it).
fn unbinding(id: &str) -> Keymap {
    let (map, problems) = Keymap::with_overrides(
        &[(id.to_owned(), vec!["none".to_owned()])],
        KeySwitches::default(),
    );
    assert_eq!(problems, [], "{id} = none reads cleanly");
    map
}

/// The letter each key's words carried before 10e — the literals the
/// shipped keymap must still spell, byte for byte (W6).
fn letter(key: OpKey) -> &'static str {
    match key {
        OpKey::Commit | OpKey::Continue => "c",
        OpKey::Amend | OpKey::Abort => "a",
        OpKey::Stage => "space",
        OpKey::Stash => "S",
        OpKey::Switch => "C",
        OpKey::Rewrite => "R",
        OpKey::Fetch => "f",
        OpKey::Pull => "p",
        OpKey::Push => "P",
        OpKey::Merge => "m",
        OpKey::Rebase => "r",
        OpKey::Undo => "u",
        OpKey::Redo => "U",
    }
}

/// His label for `key` under `keymap`, which must bind it.
fn label(keymap: &Keymap, key: OpKey) -> String {
    OpKeys(keymap)
        .label(key)
        .unwrap_or_else(|| panic!("{key:?} is bound"))
}

/// How many times `token` stands alone in `line`: split on spaces, with
/// the punctuation the words put round a key (`(C)`, `c,`) trimmed.
fn whole_tokens(line: &str, token: &str) -> usize {
    line.split_whitespace()
        .map(|word| word.trim_matches(|c: char| "(),.;:".contains(c)))
        .filter(|word| *word == token)
        .count()
}
