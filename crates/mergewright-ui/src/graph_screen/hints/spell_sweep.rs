//! The spell sweep (ticket m8/10; ADR-0055's consequence, second half):
//! with every movable action on a chord of its own, every bar and menu
//! spells the chord his keymap gives — never the default it replaced.
//!
//! 09's sweep (`mergewright::update::tests::keymap_edge`) proves the *press*
//! follows a remap; this proves the *words* do. Asserted on [`Hint`]s and
//! menu labels, not on frame text, so a hint the width rule drops cannot
//! hide a wrong spelling and a right one cannot pass by coincidence.

use std::collections::HashMap;

use mergewright_term::sanitize_line;

use super::{Divider, HintKey, every_context, hints_for};
use crate::context_menu::ContextMenu;
use crate::keymap::sweep::swept;
use crate::keymap::{ActionId, Keymap};

/// The widths the ticket names: the narrowest a bar is drawn at, the two
/// width-rule rungs, and a bar wide enough to hold everything.
const WIDTHS: [u16; 4] = [40, 80, 120, 200];

/// What one hint key must read under the sweep: a moved action its new
/// chord, a text field's key its default (a field passes every key as
/// typed, ADR-0055), a word itself.
fn expected(key: &HintKey, moved: &HashMap<&'static str, String>) -> String {
    let shipped = |action: &ActionId| {
        Keymap::shipped()
            .live_labels(action)
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("{} has a default", action.0))
    };
    match key {
        HintKey::Key(action) | HintKey::EveryChord(action) => moved
            .get(action.0)
            .cloned()
            .unwrap_or_else(|| shipped(action)),
        HintKey::Fixed(action) => shipped(action),
        HintKey::Word(word) => (*word).to_owned(),
    }
}

/// (a) every drawn hint's keys are the remapped spelling, and (d) no bar
/// draws one chord twice — over every context, both dividers, every width.
#[test]
fn every_bar_spells_the_remapped_chords() {
    let (keys, moved) = swept();
    let mut wrong = Vec::new();
    let mut bars = 0;
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in WIDTHS {
                bars += 1;
                let mut drawn: Vec<String> = Vec::new();
                for hint in hints_for(&context, divider, columns, &keys) {
                    // Joined as the bar joins them (R2: a repeated
                    // `alt+` is written once — the sweep's chords all
                    // carry one).
                    let want =
                        crate::keymap::join(hint.keys.iter().map(|key| expected(key, &moved)))
                            .unwrap_or_default();
                    let got = hint.key_text(&keys).unwrap_or_default();
                    if got != want {
                        wrong.push(format!(
                            "{context:?} {divider:?} {columns}: `{got} {}`, want `{want}`",
                            hint.verb
                        ));
                    }
                    for key in hint.keys {
                        let label = expected(key, &moved);
                        if !matches!(key, HintKey::Word(_)) && drawn.contains(&label) {
                            wrong.push(format!(
                                "{context:?} {divider:?} {columns}: `{label}` drawn twice"
                            ));
                        }
                        drawn.push(label);
                    }
                }
            }
        }
    }
    assert!(bars > 200, "only {bars} bars swept");
    assert!(
        wrong.is_empty(),
        "{} wrong:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// (b) every `.` menu's `(key)` suffix is the remapped chord.
#[test]
fn every_menu_suffix_spells_the_remapped_chord() {
    let (keys, moved) = swept();
    let history = vec![(sanitize_line("Merge feature into main"), None)];
    let menus = [
        ContextMenu::for_row(false, Vec::new(), 0, None, &keys),
        ContextMenu::for_row(true, history, 0, None, &keys),
        ContextMenu::for_working_tree(false, &keys),
        ContextMenu::for_working_tree(true, &keys),
    ];
    let suffix = |action: &str| format!("({})", moved[action]);
    for menu in &menus {
        for item in &menu.items {
            let label = item.label.as_str();
            let want = match label.split(" (").next().unwrap_or(label) {
                "Copy SHA" => Some(suffix("graph.copy_sha")),
                "Compare with this" => Some(suffix("graph.compare")),
                "Open details" => Some(suffix("app.open")),
                "Fetch" => Some(suffix("graph.fetch")),
                "Pull" => Some(suffix("graph.pull")),
                "Push" => Some(suffix("graph.push")),
                "Undo" => Some(suffix("app.undo")),
                // The caller's own label (mergewright's verbs spell theirs, held
                // by mergewright's menu tests), Explain and absorb: no key here.
                _ => None,
            };
            if let Some(want) = want {
                assert!(label.ends_with(&want), "{label:?}: want {want}");
            }
        }
    }
}
