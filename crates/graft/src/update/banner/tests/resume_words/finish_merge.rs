//! Finishing a merge keeps the edits-are-back words.
//! Moved out of `resume_words.rs` to keep that file under its size row.

use super::*;

/// 21b-4 5h (19d-c hand-back): a paused merge finished from the finish
/// summary keeps `your edits are back` after its own words — the summary's
/// `merged … into …` replaces `what`, never `also`.
#[test]
fn finishing_a_merge_keeps_your_edits_are_back() {
    let mut state = stopped(InProgress::Merge, &[]);
    let mut done = applied(
        OperationKind::Merge,
        "merge feature into main",
        EntryStatus::Committed,
    );
    done.entry = set_aside_around(done.entry);
    let mut n = 0;
    let mut ids = move || {
        n += 1;
        JobId::new(n)
    };
    crate::update::banner::resume::resumed_done(
        &mut state,
        &Resume::Continue { solved: Vec::new() },
        &done,
        Some(crate::update::finish::FinishAnswer::Merged(Some(
            "merged feature into main".to_owned(),
        ))),
        &mut ids,
    );
    assert_eq!(state.status, edits_back("merged feature into main"));
}
