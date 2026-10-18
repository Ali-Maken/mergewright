//! Where a finished `u`/`U` lands (m8-polish-and-ship/13, ruling 2; UX
//! §3.1): as a `--continue` does, read from what the journal entry it wrote
//! says moved — an undo has no [`Operation`](mergewright_ops::operation::Operation)
//! to decide it from at launch, so its own entry is the only witness.

use mergewright_core::journal::{HeadRef, JournalEntry};
use mergewright_core::oid::Oid;

use crate::update::operate::Landing;

/// The landing an undo or redo whose entry is `entry` owes:
/// 1. `HEAD` moved (a different `HEAD`, or the branch it is on moved) →
///    on `HEAD`;
/// 2. else the first branch or tag (in ref-name order) the entry moved
///    and left existing → on that ref's commit (a deleted branch come back
///    lands on its row);
/// 3. else a ref with no branch or tag row moved (`refs/stash`: its row
///    went or came, and the edits it held are on `HEAD`'s working tree) →
///    on `HEAD`;
/// 4. else nothing: an undone stage or discard leaves the cursor, and the
///    pane he is in, where they were; so does a branch deleted again (its
///    row is gone, and no other row is the result).
pub(super) fn of(entry: &JournalEntry) -> Option<Landing> {
    if head_moved(entry) {
        return Some(Landing::Head);
    }
    if let Some(at) = moved(entry)
        .filter(|(name, _)| has_a_row(name))
        .find_map(|(_, after)| after)
    {
        return Some(Landing::Commit(at));
    }
    moved(entry)
        .any(|(name, _)| !has_a_row(name))
        .then_some(Landing::Head)
}

/// The commits a branch or tag came back on: the refs the entry made exist
/// again. Tagged `↺ put back`, as a put-back from the journal tags what it
/// restored (`recover::tag_rows`).
pub(super) fn came_back(entry: &JournalEntry) -> Vec<Oid> {
    entry
        .refs_after
        .iter()
        .filter(|(name, after)| {
            has_a_row(name)
                && after.is_some()
                && entry.refs_before.get(*name).is_none_or(Option::is_none)
        })
        .filter_map(|(_, after)| *after)
        .collect()
}

/// Whether `HEAD` is somewhere else after the entry: another branch or
/// commit, or the same branch at another commit. An entry with no recorded
/// `head_after` has said nothing of `HEAD` itself; its refs still can.
fn head_moved(entry: &JournalEntry) -> bool {
    if entry
        .head_after
        .as_ref()
        .is_some_and(|after| *after != entry.head_before)
    {
        return true;
    }
    let on = entry.head_after.as_ref().unwrap_or(&entry.head_before);
    match on {
        HeadRef::Branch(name) | HeadRef::Unborn(name) => {
            entry.refs_before.get(name) != entry.refs_after.get(name)
        }
        HeadRef::Detached(_) => false,
    }
}

/// Every ref whose value differs before and after, in name order, with
/// its value after (`None`: it is gone).
fn moved(entry: &JournalEntry) -> impl Iterator<Item = (&str, Option<Oid>)> {
    entry.refs_after.iter().filter_map(|(name, after)| {
        let before = entry.refs_before.get(name).copied().flatten();
        (before != *after).then_some((name.as_str(), *after))
    })
}

/// A ref the graph draws a row badge for: a branch or a tag.
fn has_a_row(name: &str) -> bool {
    name.starts_with("refs/heads/") || name.starts_with("refs/tags/")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mergewright_core::journal::{EntryId, EntryStatus, OperationKind, Touches};

    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; 20]).expect("20 bytes")
    }

    const MAIN: &str = "refs/heads/main";

    /// An undo's entry: `HEAD` on `main` before and after, with these refs
    /// before and after.
    fn entry(before: &[(&str, Option<Oid>)], after: &[(&str, Option<Oid>)]) -> JournalEntry {
        let refs = |pairs: &[(&str, Option<Oid>)]| -> BTreeMap<String, Option<Oid>> {
            pairs
                .iter()
                .map(|(name, at)| ((*name).to_owned(), *at))
                .collect()
        };
        JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Undo,
            description: "undo: commit".to_owned(),
            status: EntryStatus::Committed,
            touches: Touches::default(),
            head_before: HeadRef::Branch(MAIN.to_owned()),
            head_after: Some(HeadRef::Branch(MAIN.to_owned())),
            refs_before: refs(before),
            refs_after: refs(after),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            worktree_raw_before: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: BTreeMap::new(),
            config_after: BTreeMap::new(),
            put_back: None,
            fetched_refs: None,
        }
    }

    /// `u` after a commit (and `U` after that): `main` moves, and `HEAD`
    /// with it.
    #[test]
    fn the_branch_head_is_on_moving_lands_on_head() {
        let undone = entry(&[(MAIN, Some(oid(2)))], &[(MAIN, Some(oid(1)))]);
        assert_eq!(of(&undone), Some(Landing::Head));
        let redone = entry(&[(MAIN, Some(oid(1)))], &[(MAIN, Some(oid(2)))]);
        assert_eq!(of(&redone), Some(Landing::Head));
    }

    /// `u` after a switch: `HEAD` is on another branch.
    #[test]
    fn a_different_head_lands_on_head() {
        let mut switched = entry(&[], &[]);
        switched.head_after = Some(HeadRef::Branch("refs/heads/feature".to_owned()));
        assert_eq!(of(&switched), Some(Landing::Head));
        let mut detached = entry(&[], &[]);
        detached.head_after = Some(HeadRef::Detached(oid(3)));
        assert_eq!(of(&detached), Some(Landing::Head));
    }

    /// `u` after a branch delete: the branch is back, on its row, and the
    /// row is tagged.
    #[test]
    fn a_branch_come_back_lands_on_its_row_and_is_tagged() {
        let back = entry(
            &[("refs/heads/spike", None)],
            &[("refs/heads/spike", Some(oid(7)))],
        );
        assert_eq!(of(&back), Some(Landing::Commit(oid(7))));
        assert_eq!(came_back(&back), vec![oid(7)]);
    }

    /// `U` after that: the branch is deleted again; nothing to land on.
    #[test]
    fn a_branch_gone_again_leaves_the_cursor() {
        let gone = entry(
            &[("refs/heads/spike", Some(oid(7)))],
            &[("refs/heads/spike", None)],
        );
        assert_eq!(of(&gone), None);
        assert!(came_back(&gone).is_empty());
    }

    /// `u` after a stash: `refs/stash` moved, and the edits are back on
    /// `HEAD`'s working tree — not left on a stash row.
    #[test]
    fn a_stash_undone_lands_on_head() {
        let unstashed = entry(&[("refs/stash", Some(oid(9)))], &[("refs/stash", None)]);
        assert_eq!(of(&unstashed), Some(Landing::Head));
        assert!(came_back(&unstashed).is_empty(), "no branch came back");
    }

    /// `u` after a stage: no ref moved; the cursor and the pane stay.
    #[test]
    fn nothing_moved_leaves_the_cursor() {
        let staged = entry(&[(MAIN, Some(oid(1)))], &[(MAIN, Some(oid(1)))]);
        assert_eq!(of(&staged), None);
    }

    /// A moved branch the cursor is not on lands on it, ahead of any other
    /// ref that moved.
    #[test]
    fn a_branch_beats_the_stash() {
        let both = entry(
            &[("refs/heads/topic", Some(oid(4))), ("refs/stash", None)],
            &[
                ("refs/heads/topic", Some(oid(5))),
                ("refs/stash", Some(oid(6))),
            ],
        );
        assert_eq!(of(&both), Some(Landing::Commit(oid(5))));
        assert!(
            came_back(&both).is_empty(),
            "topic moved, it did not come back"
        );
    }
}
