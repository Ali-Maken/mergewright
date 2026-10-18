//! Where the review's `y` lands (ease-pass/12 M18, decision 9) and what
//! never draws before its read (decision 7), over plain data. The real
//! merge, its fold count and `u` are `tests/it/finish/landing.rs`.

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_ui::graph_screen::hints::RowBar;

use super::answers::committing;
use super::*;
use crate::update::fold_count::FoldCount;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn merge_oid() -> Oid {
    oid(7)
}

/// The refs: `HEAD` on `main` at `tip` (as `tests::head_keep`).
fn refs_on_main(tip: Oid) -> crate::refs::RefBadges {
    let main = graft_core::ref_name::RefName::new("refs/heads/main").expect("a valid name");
    crate::refs::badges(
        &[(main.clone(), tip)],
        &graft_git::repo::HeadState::Branch(main),
        &std::collections::HashMap::new(),
        &[],
        |_, _| unreachable!("no upstreams to count"),
    )
    .expect("nothing to count")
}

/// The review's `y`, answered `Done` with `main` moved onto the merge.
fn landed_done() -> AppState {
    let (state, id) = committing();
    let mut applied = crate::update::banner::tests::done_applied();
    applied.entry.head_after = Some(HeadRef::Branch("refs/heads/main".to_owned()));
    applied
        .entry
        .refs_after
        .insert("refs/heads/main".to_owned(), Some(merge_oid()));
    update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Done(applied))),
        ),
    )
    .0
}

/// Decision 9: `✓ Merged <from> into <into>.` — the done line draws the
/// one `✓`, the words carry none.
#[test]
fn y_lands_with_the_merged_words_and_one_tick() {
    let state = landed_done();
    assert!(state.finish.is_none(), "the review closed");
    let Status::Done { what, also, keys } = &state.status else {
        panic!("a done status: {:?}", state.status);
    };
    assert_eq!(what, "Merged feature into main.");
    let line = crate::view::status::done_line(what, also.as_deref(), keys.as_deref());
    assert!(line.starts_with("✓ Merged feature into main."), "{line}");
    assert_eq!(line.matches('✓').count(), 1, "{line}");
    assert_eq!(state.last_merge, Some(merge_oid()), "the merge just made");
}

/// M18: no `JustMerged` until the count is known, nor off the merge or
/// once it is not `HEAD`.
#[test]
fn just_merged_waits_for_the_count_and_the_cursor() {
    let mut state = landed_done();
    // `HEAD` off the merge (no refs read yet): never.
    state.fold_counts.insert(merge_oid(), FoldCount::Counted(3));
    assert_eq!(just_merged_bar(&state, Some(merge_oid())), None);
    // With `HEAD` on it, as the refs read after the landing says.
    state.refs = Some(Arc::new(refs_on_main(merge_oid())));
    assert_eq!(
        just_merged_bar(&state, Some(merge_oid())),
        Some(RowBar::JustMerged { inside: 3 })
    );
    assert_eq!(just_merged_bar(&state, Some(oid(8))), None);
    state.fold_counts.insert(merge_oid(), FoldCount::Counting);
    assert_eq!(just_merged_bar(&state, Some(merge_oid())), None, "G's bar");
    state.fold_counts.insert(merge_oid(), FoldCount::Failed);
    assert_eq!(just_merged_bar(&state, Some(merge_oid())), None);
}

/// Decision 7: the doors draw nothing until the read lands — no review
/// frame on the screen in between.
#[test]
fn no_review_frame_before_the_read_lands() {
    let state = stopped(InProgress::Merge, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(state.finish.is_none());
    assert!(crate::view::finish::finish_view(&state).is_none());
    let frame = crate::view::tests::draw(&state, 120, 40);
    assert!(!frame.contains("Nothing committed yet"), "{frame}");
    let (state, _) = review::landed((state, commands));
    let frame = crate::view::tests::draw(&state, 120, 40);
    assert!(frame.contains("Nothing committed yet"), "{frame}");
}
