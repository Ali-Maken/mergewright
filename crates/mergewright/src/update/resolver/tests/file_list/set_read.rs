//! The conflict set's answer landing: a failed read, and a merge's
//! `Combined by git` read carried with it (ease-pass/11b). Split from
//! `file_list.rs` to keep it under the size ratchet.

use super::*;
use crate::update::resolver::files::SetRead;

#[test]
fn an_unreadable_set_says_so() {
    let mut state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let read = JobId::new(7_100);
    let open = state.resolver.as_mut().expect("open");
    open.files = Loading::Pending(read);
    open.set_job = Some(read);
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            read,
            Product::ConflictSet(Err("git broke".to_owned()).into()),
        ),
    );
    assert_eq!(
        states(&state),
        vec![
            (
                "conflicted.rs".to_owned(),
                RowState::Open { open: 2, of: 2 }
            ),
            ("later.rs".to_owned(), RowState::Unread),
        ],
        "rows from the status"
    );
    let drawn = super::super::frame_text(&state);
    assert!(drawn.contains("git's list unavailable"), "{drawn}");
}

/// The set's job carries a merge's combined files: they land on the open
/// resolver beside the set; `None` (not a merge) lands as nothing combined.
#[test]
fn the_combined_files_land_with_the_set() {
    for (combined, want) in [
        (
            Some(Ok(vec![PathBuf::from("clean.txt")])),
            Loading::Ready(Arc::new(vec![PathBuf::from("clean.txt")])),
        ),
        (
            Some(Err("locked".to_owned())),
            Loading::Failed("locked".to_owned()),
        ),
        (None, Loading::Ready(Arc::new(Vec::new()))),
    ] {
        let mut state = listing(&["conflicted.rs"], &["conflicted.rs"]);
        let read = JobId::new(7_200);
        let open = state.resolver.as_mut().expect("open");
        assert_eq!(open.combined, Loading::Idle);
        open.set_job = Some(read);
        let set = SetRead {
            set: Ok(set_of(&["conflicted.rs"])),
            combined,
        };
        let (state, _) = update(&state, AppEvent::Produced(read, Product::ConflictSet(set)));
        assert_eq!(state.resolver.as_ref().expect("open").combined, want);
    }
}
