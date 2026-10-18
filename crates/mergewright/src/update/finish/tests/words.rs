//! The review's words over plain data (ease-pass/12 decision 4): how a
//! clashed file was solved, and the message row.

use mergewright_ui::finish_summary::{HowSolved, PickCounts};

use super::*;

/// Decision 4: first match wins — a whole-file choice, then an editor,
/// then the picks by kind; nothing recorded is `solved in the file`.
#[test]
fn how_solved_first_match_wins() {
    let whole = Choices {
        whole: Some(Side::Ours),
        edited: true,
        ..Choices::default()
    };
    let word = |how: HowSolved| how.words();
    assert_eq!(
        word(how_solved(Some(&whole), ResultIs::Deleted)),
        "you chose: delete the file"
    );
    assert_eq!(
        word(how_solved(Some(&whole), ResultIs::Yours)),
        "you chose: ◀ yours, whole file"
    );
    let edited = Choices {
        edited: true,
        takes: vec![Some(Take::Ours)],
        ..Choices::default()
    };
    assert_eq!(
        how_solved(Some(&edited), ResultIs::Combined),
        HowSolved::ByHand
    );
    let mixed = Choices {
        takes: vec![Some(Take::Ours), Some(Take::Theirs), None],
        ..Choices::default()
    };
    assert_eq!(
        how_solved(Some(&mixed), ResultIs::Combined),
        HowSolved::Picks(PickCounts {
            yours: 1,
            theirs: 1,
            both: 0
        })
    );
    assert_eq!(how_solved(None, ResultIs::Yours), HowSolved::ByHand);
    assert_eq!(
        how_solved(Some(&Choices::default()), ResultIs::Yours),
        HowSolved::ByHand
    );
}

#[test]
fn the_message_row_is_the_first_line() {
    assert_eq!(
        subject_of(b"  Merge branch 'x'  \n\nbody\n"),
        "Merge branch 'x'"
    );
    assert_eq!(subject_of(b""), "");
}
