//! The card's words, spelled from his live keymap.

use graft_ops::tour::TourTask;
use graft_ui::keymap::Keymap;

use super::*;
use crate::update::tour::TourState;

fn shipped() -> Keymap {
    Keymap::shipped().clone()
}

/// Every line, for NOTES and for the eye: each names a key.
#[test]
fn every_task_has_a_line_naming_its_shipped_key() {
    let keys = shipped();
    let lines: Vec<String> = TourTask::ALL.iter().map(|t| line(*t, &keys)).collect();
    assert_eq!(
        lines,
        [
            "\u{23ce} on the top row opens your changes \u{b7} space stages the change under the cursor",
            "c writes a commit from what you staged",
            "u takes back the last thing you did \u{2014} nothing is lost",
            "S opens the stashes \u{b7} s sets your edits aside \u{b7} p brings them back",
            "b makes a branch at the cursor",
            "m on feature shows the merge before it happens \u{b7} y does it",
            "m on clash stops on a conflict \u{b7} \u{23ce} opens it \u{b7} t takes theirs \u{b7} c finishes",
            "P sends main to origin",
        ]
    );
}

/// Ticket A9: `[keys.graph] merge = "M"` → the card says `M`.
#[test]
fn a_remapped_merge_is_the_key_the_card_says() {
    let loaded = crate::config::tests::loaded_from_a_file("[keys.graph]\nmerge = \"M\"\n");
    let (keys, _problems) = crate::config::keymap_from(&loaded);
    let merge = line(TourTask::MergeWithPreview, &keys);
    assert!(merge.starts_with("M on feature"), "{merge}");
    let conflict = line(TourTask::ResolveConflict, &keys);
    assert!(conflict.starts_with("M on clash"), "{conflict}");
}

#[test]
fn the_card_is_on_the_first_task_not_done_and_absent_outside_the_tour() {
    let mut state = crate::update::AppState::default();
    assert_eq!(card(&state), None);
    let mut tour = TourState::default();
    tour.done[0] = true;
    tour.done[1] = true;
    state.tour = Some(Box::new(tour));
    let view = card(&state).expect("in the tour");
    assert_eq!((view.step, view.of), (3, 8));
    assert_eq!(view.title.as_str(), "undo that commit");
    assert_eq!(
        view.ticks.iter().map(|(_, done)| *done).collect::<Vec<_>>(),
        [true, true, false, false, false, false, false, false]
    );
    assert_eq!(
        view.leave.as_ref().map(|w| w.as_str().to_owned()),
        Some("esc leave".to_owned())
    );
}

#[test]
fn the_practice_header_says_it_is_not_his() {
    assert_eq!(
        practice_path("/tmp/x/work"),
        "practice \u{b7} /tmp/x/work \u{b7} a throwaway repo, not yours"
    );
}
