//! The sweep keymap and its checks, shared by every spelling test (ticket
//! m8/10's spell sweep; 10c's sweep (c) over footers and boxes).
//!
//! [`swept`] puts every movable action on a chord no default uses, so a
//! line that still draws a default — a literal left behind, or the wrong
//! action's key — cannot pass by coincidence.

use std::collections::HashMap;

use mergewright_core::event::KeyCode;

use super::{ActionId, KeyChord, Keymap, defaults};

/// 09's sweep set: chords no default uses anywhere.
fn free_chords() -> Vec<KeyChord> {
    let mut out: Vec<KeyChord> = ('!'..='~')
        .map(|c| KeyChord::alt(KeyCode::Char(c)))
        .collect();
    out.extend((1..=12).map(|n| KeyChord::alt(KeyCode::Function(n))));
    out.extend(('a'..='z').map(|c| KeyChord {
        ctrl: true,
        ..KeyChord::alt(KeyCode::Char(c))
    }));
    out
}

/// Every movable action — every row but a text field's and the quit chord —
/// on its own free chord: the keymap, and each moved action's new label.
pub(crate) fn swept() -> (Keymap, HashMap<&'static str, String>) {
    let rows: Vec<&'static str> = defaults()
        .iter()
        .filter(|b| {
            // Not a text field's, not `⌃C`, not a row whose keys follow
            // another's, not one a switch keeps off (m8/09c).
            !b.context.is_text_field()
                && !matches!(
                    b.action.0,
                    "app.quit_now"
                        | "app.suspend"
                        | "preview.confirm_merge"
                        | "preview.confirm_rebase"
                )
        })
        .map(|b| b.action.0)
        .collect();
    let free = free_chords();
    assert!(
        free.len() >= rows.len(),
        "{} free for {}",
        free.len(),
        rows.len()
    );
    let moved: HashMap<&'static str, String> = rows
        .into_iter()
        .zip(free)
        .map(|(action, chord)| (action, chord.label().0))
        .collect();
    let lines: Vec<(String, Vec<String>)> = moved
        .iter()
        .map(|(action, label)| ((*action).to_owned(), vec![label.clone()]))
        .collect();
    let (map, warnings) = Keymap::with_overrides(&lines, crate::keymap::KeySwitches::default());
    assert_eq!(warnings, [], "every chord is free, so nothing collides");
    (map, moved)
}

/// `line` names each group's keys as `keys` spells them, and no shipped
/// label of those actions as a whole token. A group is one part's actions
/// (`["journal.close", "app.back"]` for `J/esc close`). Under [`swept`]
/// each group must have moved, or the check proves nothing.
pub(crate) fn assert_spelled(line: &str, keys: &Keymap, groups: &[&[&'static str]]) {
    let mut rest = line.to_owned();
    for group in groups {
        let ids: Vec<ActionId> = group.iter().map(|id| ActionId(id)).collect();
        let want = keys
            .spell_keys(&ids)
            .unwrap_or_else(|| panic!("{group:?} unbound"));
        assert_ne!(
            Some(&want),
            Keymap::shipped().spell_keys(&ids).as_ref(),
            "{group:?} did not move"
        );
        assert!(
            line.contains(&want),
            "`{line}`: want `{want}` for {group:?}"
        );
        rest = rest.replace(&want, " ");
    }
    for group in groups {
        for id in *group {
            for default in Keymap::shipped().live_labels(&ActionId(id)) {
                assert!(
                    !has_token(&rest, default),
                    "`{line}`: still draws {id}'s default `{default}`"
                );
            }
        }
    }
}

/// `line` names each group's keys as the shipped table spells them: a text
/// field's footer, whatever his keymap says (ADR-0055).
pub(crate) fn assert_shipped(line: &str, groups: &[&[&'static str]]) {
    for group in groups {
        let ids: Vec<ActionId> = group.iter().map(|id| ActionId(id)).collect();
        let want = Keymap::shipped()
            .spell_keys(&ids)
            .unwrap_or_else(|| panic!("{group:?} unbound"));
        assert!(
            line.contains(&want),
            "`{line}`: want `{want}` for {group:?}"
        );
    }
}

/// `label` stands alone in `text`: a whitespace-separated word, or one side
/// of a `/` pair (`J/esc`), brackets and separators trimmed.
fn has_token(text: &str, label: &str) -> bool {
    text.split_whitespace()
        .map(|word| word.trim_matches(|c| matches!(c, '(' | ')' | ',' | '·' | ':')))
        .any(|word| word == label || word.split('/').any(|side| side == label))
}
