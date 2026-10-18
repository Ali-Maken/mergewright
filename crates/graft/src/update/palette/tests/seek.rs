//! A commit row's `⏎` lands as the picker's `⇥` does — through the walk
//! when the commit is not laid out — and on a commit the default fold
//! hides, the merge opens first (ease-pass 06c). Its own file so
//! `palette/tests.rs` stays under its size row.

use super::*;
use crate::search::CommitHit;
use crate::update::tests::reveal::{find_for, oid_of, on, revealed};

#[test]
fn a_commit_row_on_a_folded_commit_opens_its_merge_and_lands_on_it() {
    let walk = JobId::new(4242);
    let mut state = with_a_tag();
    state.history = Some(crate::history::History::started());
    state.awaiting = Some(walk);
    let (open, _) = press(&state, KeyPress::control(KeyCode::Char('p')));
    let (typing, _) = typed(&open, "login");
    let commits = typing
        .palette
        .as_ref()
        .and_then(|p| p.find.commits.pending.as_ref())
        .map(|(id, _)| *id)
        .expect("a commits job in flight");
    let (merge, hit) = (oid_of(0x20), oid_of(0x11));
    let found: Arc<[CommitHit]> = Arc::from([CommitHit {
        id: hit,
        subject: b"fix login".to_vec().into(),
    }]);
    let (landed, _) = update(
        &typing,
        AppEvent::Produced(commits, Product::PaletteCommits(Ok(found))),
    );
    let landed = cursor_on(landed, "COMMITS", "1111111  fix login");
    let (state, commands) = press(&landed, plain(KeyCode::Enter));
    let find = find_for(&commands, hit);

    let shown: &[u8] = &[0x30, 0x20, 0x10];
    let opened: &[u8] = &[0x30, 0x20, 0x11, 0x10];
    let (state, _) = revealed(&state, walk, find, (merge, hit), (shown, opened));

    assert_eq!(on(&state), Some(hit), "the cursor on the commit");
    assert!(state.palette.is_none(), "the palette closed");
}
