//! The undo journal browser's view model (`J`,
//! m3-working-tree-undo-and-stash/06).
//!
//! Pure: [`JournalState`](crate::update::journal::JournalState) plus the
//! journal as it was last read, turned into rows. Which entry `u` would take
//! back is `graft_ops`' answer, never one worked out here — the row marks
//! come from `next_undo`/`next_redo`/`in_force_all` so the browser and the
//! key can never disagree.

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_ops::journal_list::{HeldBy, JournalList, ListRow, LostCommit, LostEdit, LostScan};
use graft_term::sanitize::{sanitize, sanitize_line};
use graft_ui::graph_screen::columns::relative_date;
use graft_ui::journal_screen::{JournalRowView, JournalStatus, JournalView, RowKind, RowMark};

use crate::update::AppState;
use crate::update::detail::Loading;

/// Milliseconds in a second, for the entries' `at_unix_ms`.
const MILLIS: i64 = 1_000;

/// [`JournalView`] for whatever the journal read and J's two later stages
/// have brought back (m7-power-tools/24: the list is
/// `graft_ops::journal_list::build`'s, built by `update::journal::rebuild`).
///
/// J's status row is left `None` here: which refusal or notice shows is the
/// bottom row's ladder (`view::frame`), asked at the width it is drawn at.
#[must_use]
pub fn journal_view(state: &AppState, now_seconds: i64) -> JournalView {
    let location = sanitize(&state.repo_path.display().to_string());
    let (rows, status) = match (&state.journal.entries, &state.journal.list) {
        (Loading::Idle | Loading::Pending(_), _) => (Vec::new(), JournalStatus::Loading),
        (Loading::Failed(why), _) => (Vec::new(), JournalStatus::Failed(sanitize(why))),
        (Loading::Ready(_), Some(list)) if !list.rows.is_empty() => {
            (rows_for(list, state, now_seconds), JournalStatus::Ready)
        }
        (Loading::Ready(_), _) => (Vec::new(), JournalStatus::Empty),
    };
    let selected = state.journal.selected().min(rows.len().saturating_sub(1));
    let enter_words = match status {
        JournalStatus::Ready => state
            .journal
            .list
            .as_deref()
            .and_then(|list| enter_words(list, state, selected)),
        JournalStatus::Loading | JournalStatus::Empty | JournalStatus::Failed(_) => None,
    };
    JournalView {
        location,
        rows,
        selected,
        status,
        title_suffix: title_suffix(state),
        status_row: None,
        enter_words,
        box_open: state.preview.is_some(),
    }
}

/// J1's title words for the lost-commit scan: while it runs, capped, or
/// failed; nothing once done (the status rule).
fn title_suffix(state: &AppState) -> Option<String> {
    if state.journal.finding_lost() {
        return Some("finding lost commits…".to_owned());
    }
    let failed = |why: &str| {
        Some(format!(
            "couldn't look for lost commits: {}",
            sanitize_line(why).as_str()
        ))
    };
    match &state.journal.lost {
        Loading::Failed(why) => failed(why),
        Loading::Ready(found) => match &found.scan {
            LostScan::Capped { checked } => Some(format!("looked at the newest {checked} steps")),
            LostScan::Failed(why) => failed(why),
            LostScan::Running | LostScan::Done => None,
        },
        Loading::Idle | Loading::Pending(_) => None,
    }
}

/// The journal's entries as last read, oldest first (the order the marks
/// are worked out in).
fn entries(state: &AppState) -> &[JournalEntry] {
    match &state.journal.entries {
        Loading::Ready(read) => &read.entries,
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => &[],
    }
}

/// The list's rows, newest first, `✗` rows right after their event.
fn rows_for(list: &JournalList, state: &AppState, now_seconds: i64) -> Vec<JournalRowView> {
    let entries = entries(state);
    let marks = Marks::of(entries);
    list.rows
        .iter()
        .map(|row| match row {
            // The list is built from these very entries
            // (`update::journal::rebuild`), so the id is always there; a
            // blank row would be the honest drawing if it ever were not.
            ListRow::Graft(id) => entries
                .iter()
                .position(|entry| &entry.id == id)
                .map_or_else(JournalRowView::default, |index| {
                    graft_row(entries, index, &marks, state, now_seconds)
                }),
            ListRow::Outside(step) => JournalRowView {
                age: sanitize(&age(step.record.at.saturating_mul(MILLIS), now_seconds)),
                description: sanitize_line(&step.words),
                kind: RowKind::Outside,
                git_subject: Some(sanitize_line(&step.record.subject)),
                ..JournalRowView::default()
            },
            ListRow::Lost { event, commit } => JournalRowView {
                description: sanitize_line(&lost_words(commit)),
                kind: RowKind::Lost,
                under: Some(*event),
                ..JournalRowView::default()
            },
            ListRow::LostEdit { event, edit } => JournalRowView {
                description: sanitize_line(&lost_edit_words(edit, entries)),
                kind: RowKind::LostEdit,
                under: Some(*event),
                ..JournalRowView::default()
            },
        })
        .collect()
}

/// Which entry `u` and `U` would take, and which are in force — asked of
/// `graft_ops`, never worked out here, so the browser and the key can never
/// disagree.
struct Marks<'a> {
    next_undo: Option<&'a graft_core::journal::EntryId>,
    next_redo: Option<&'a graft_core::journal::EntryId>,
    in_force: Vec<bool>,
}

impl<'a> Marks<'a> {
    fn of(entries: &'a [JournalEntry]) -> Marks<'a> {
        Marks {
            next_undo: graft_ops::journal::next_undo(entries).map(|entry| &entry.id),
            next_redo: graft_ops::journal::next_redo(entries).map(|entry| &entry.id),
            in_force: graft_ops::journal::in_force_all(entries),
        }
    }
}

/// A `●` row: entry `index` of `entries` (oldest first).
fn graft_row(
    entries: &[JournalEntry],
    index: usize,
    marks: &Marks<'_>,
    state: &AppState,
    now_seconds: i64,
) -> JournalRowView {
    let entry = &entries[index];
    // Live first, and instead of every other mark: git is part-way
    // through this very entry. `interrupted` would be a lie about a
    // process that is alive, and `← u` would offer to undo an
    // operation that has not finished happening (m5/09 seam A).
    let is_live = state.live_entries.contains(&entry.id);
    let mark = if is_live {
        RowMark::None
    } else if Some(&entry.id) == marks.next_undo {
        RowMark::NextUndo
    } else if Some(&entry.id) == marks.next_redo {
        RowMark::NextRedo
    } else if entry.is_interrupted() {
        RowMark::Interrupted
    } else if marks.in_force.get(index) == Some(&false) {
        RowMark::Undone
    } else {
        RowMark::None
    };
    JournalRowView {
        age: sanitize(&age(entry.at_unix_ms, now_seconds)),
        description: sanitize(&entry.description),
        summary: sanitize(&summary(entry, is_live)),
        mark,
        commands: entry.commands.iter().map(|line| sanitize(line)).collect(),
        failure: failure(entry, is_live).map(|line| sanitize(&line)),
        expanded: state.journal.is_expanded(&entry.id),
        kind: RowKind::Graft,
        git_subject: None,
        under: None,
    }
}

/// A lost commit's line: `9a8b7c6 "Add rate limiter" · was main's tip · no
/// branch holds it`, `· spike is gone`, or `· was HEAD`; `+N under it` when
/// it is a run. No subject known → the sha7 alone, never an empty `""`.
fn lost_words(commit: &LostCommit) -> String {
    let mut words = short(commit.run.tip);
    if !commit.subject.is_empty() {
        words.push_str(&format!(" \"{}\"", commit.subject));
    }
    match &commit.held_by {
        HeldBy::Branch { name, gone: false } => {
            words.push_str(&format!(" · was {name}'s tip · no branch holds it"));
        }
        HeldBy::Branch { name, gone: true } => {
            words.push_str(&format!(" · was {name}'s tip · {name} is gone"));
        }
        HeldBy::Head => words.push_str(" · was HEAD"),
    }
    if commit.run.under > 0 {
        words.push_str(&format!(" · +{} under it", commit.run.under));
    }
    words
}

/// A lost edit's line: `your edit to src/a.rs · replaced by this undo ·
/// kept in its backup` — no hash: it is an edit, not a commit (J1).
fn lost_edit_words(edit: &LostEdit, entries: &[JournalEntry]) -> String {
    let by = match entries
        .iter()
        .find(|entry| entry.id == edit.entry)
        .map(|entry| entry.kind)
    {
        Some(OperationKind::Redo) => "this redo",
        _ => "this undo",
    };
    let first = edit
        .paths
        .first()
        .map_or_else(String::new, |path| path.display().to_string());
    let what = match edit.paths.len() {
        0 | 1 => format!("your edit to {first}"),
        many => format!("your edits to {first} +{} more", many - 1),
    };
    format!("{what} · replaced by {by} · kept in its backup")
}

/// What `⏎` does on row `index`, from the row alone (J1, keymap §2.11) —
/// or `None` where the row cannot name it (UX checklist 16: the bar never
/// promises what the row can't).
fn enter_words(list: &JournalList, state: &AppState, index: usize) -> Option<String> {
    let words = match list.rows.get(index)? {
        ListRow::Graft(id) => {
            let entries = entries(state);
            let marks = Marks::of(entries);
            let entry = entries.iter().find(|entry| &entry.id == id)?;
            if state.live_entries.contains(id) {
                return None;
            }
            if Some(id) == marks.next_undo {
                "⏎ undo this".to_owned()
            } else if Some(id) == marks.next_redo {
                "⏎ redo this".to_owned()
            } else if matches!(entry.kind, OperationKind::Undo | OperationKind::Redo) {
                // The time machine refuses an undo or redo row
                // (`update::journal::enter_graft`): no promise.
                return None;
            } else {
                "⏎ revert to before this".to_owned()
            }
        }
        ListRow::Outside(step) => {
            if let Some(from) = switched_from(&step.record.subject) {
                format!("⏎ switch back to {from}")
            } else {
                let branch = step.record.refname.strip_prefix("refs/heads/")?;
                step.record.old?;
                if step.record.new.is_none() {
                    format!("⏎ make {branch} again")
                } else {
                    format!("⏎ put {branch} back to before this")
                }
            }
        }
        ListRow::Lost { commit, .. } => match &commit.held_by {
            HeldBy::Branch { name, gone: false } => format!("⏎ put {name} back to it"),
            HeldBy::Branch { name, gone: true } => format!("⏎ make {name} again"),
            HeldBy::Head => return None,
        },
        ListRow::LostEdit { edit, .. } => {
            if edit.paths.len() > 1 {
                "⏎ put your edits back".to_owned()
            } else {
                "⏎ put your edit back".to_owned()
            }
        }
    };
    Some(sanitize_line(&words).as_str().to_owned())
}

/// The branch a shell switch left, for the bar: `checkout: moving from
/// main to spike` → `main`; a detached start is its sha7. The same parse
/// `update::journal` switches by.
fn switched_from(subject: &str) -> Option<String> {
    let (from, _) = subject
        .strip_prefix("checkout: moving from ")?
        .split_once(" to ")?;
    let hex = (from.len() == 40 || from.len() == 64) && from.bytes().all(|b| b.is_ascii_hexdigit());
    Some(if hex {
        from.chars().take(7).collect()
    } else {
        from.to_owned()
    })
}

/// `2m ago`, from the same cascade the graph's date column uses, so two
/// views of the same moment never disagree.
fn age(at_unix_ms: i64, now_seconds: i64) -> String {
    let then = at_unix_ms.div_euclid(MILLIS);
    let relative = relative_date(then, now_seconds);
    // "future" and "now" are already whole answers; "now ago" is not English,
    // and the journal browser opened straight after a write is full of
    // entries less than a minute old.
    if relative == "future" || relative == "now" {
        relative
    } else {
        format!("{relative} ago")
    }
}

/// The reason an entry closed by his own `--abort` carries
/// (`graft_ops::apply::resume`, `went_through`).
const ABORTED: &str = "aborted";

/// What such a row says instead of `failed: aborted` (M6/68).
const ABORTED_BY_YOU: &str = "aborted by you · nothing changed";

/// What a live entry's row says instead of a verdict: the operation has not
/// finished happening, and these are the two keys that end it either way.
const IN_PROGRESS: &str = "in progress \u{2014} c continue  a abort";

/// What the entry moved, and whether a backup was kept — the two facts that
/// decide whether `u` can put it back.
///
/// `live` is git standing part-way through this very entry
/// ([`crate::update::AppState::live_entries`]); it outranks every verdict,
/// because there is not one yet.
fn summary(entry: &JournalEntry, live: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    // A push or a fetch is journaled but `u` passes over it (design note,
    // choice 1): the row says what moved and that it stays moved, instead of
    // the ref count and backup that mean "undoable" everywhere else. The
    // verdict comes first and the tracking ref is not repeated (the
    // description names it): at 80 columns the summary has some 40 cells,
    // and it is the verdict that must survive (m4 acceptance review M2).
    match entry.kind {
        OperationKind::Push => {
            let tracking = crate::update::remote::pushed_to(entry);
            if let Some(name) = entry.touches.refs.first()
                && let Some(Some(after)) = entry.refs_after.get(name)
            {
                let before = entry
                    .refs_before
                    .get(name)
                    .copied()
                    .flatten()
                    .map_or_else(|| "new".to_owned(), short);
                parts.push(format!("{before} → {}", short(*after)));
            }
            let remote = tracking.split('/').next().unwrap_or("the remote");
            parts.insert(0, format!("not undoable \u{2014} {remote} has it"));
            return with_failure(entry, live, &parts);
        }
        OperationKind::Fetch => {
            parts.push("remote-tracking refs".to_owned());
            parts.push("not undoable".to_owned());
            return with_failure(entry, live, &parts);
        }
        _ => {}
    }
    match entry.touches.refs.len() {
        0 => {}
        1 => parts.push("1 ref".to_owned()),
        many => parts.push(format!("{many} refs")),
    }
    if entry.touches.index {
        parts.push("the index".to_owned());
    }
    if entry.touches.worktree {
        parts.push("the working tree".to_owned());
    }
    // Interrupted first, and *instead of* `failed:` — recovery closes a torn
    // entry as `Aborted { reason: "interrupted" }`, so the two arms overlap
    // and "failed: interrupted" is the wrong word for a process that died
    // (m3/06 review, Major 2).
    if live {
        parts.insert(0, IN_PROGRESS.to_owned());
    } else if entry.is_interrupted() {
        parts.insert(0, "interrupted".to_owned());
    } else if let EntryStatus::Aborted { reason } = &entry.status {
        // He stopped it himself (`--abort`): not a failure, and git put
        // everything back (M6/68).
        if reason == ABORTED {
            return ABORTED_BY_YOU.to_owned();
        }
        // The row carries the failure in short form; the whole reason is
        // under `Space`, which is where a multi-line stderr belongs.
        parts.insert(0, format!("failed: {}", first_line(reason)));
    }
    if entry.backup_ref.is_some() {
        parts.push("backup saved".to_owned());
    }
    parts.join(" · ")
}

/// A push or fetch row that failed says so first, and nothing else: an
/// aborted push sent nothing, so "origin has it" would be false.
fn with_failure(entry: &JournalEntry, live: bool, parts: &[String]) -> String {
    if live {
        return IN_PROGRESS.to_owned();
    }
    if entry.is_interrupted() {
        return "interrupted".to_owned();
    }
    match &entry.status {
        EntryStatus::Aborted { reason } if reason == ABORTED => ABORTED_BY_YOU.to_owned(),
        EntryStatus::Aborted { reason } => format!("failed: {}", first_line(reason)),
        EntryStatus::Pending | EntryStatus::Committed => parts.join(" · "),
    }
}

/// Seven hex digits, as every row writes a commit.
fn short(oid: graft_core::oid::Oid) -> String {
    let mut hex = oid.to_string();
    hex.truncate(7);
    hex
}

/// The whole reason an `Aborted` entry gives, for the expanded row.
///
/// A live entry has none: nothing failed, and the operation has not finished
/// happening. Saying "a previous run stopped between this entry's two
/// journal lines" about a merge running right now is the very lie this
/// ticket exists to stop.
fn failure(entry: &JournalEntry, live: bool) -> Option<String> {
    if live {
        return None;
    }
    if entry.is_interrupted() {
        return Some(
            "interrupted: a previous run stopped between this entry's two journal lines".to_owned(),
        );
    }
    match &entry.status {
        EntryStatus::Aborted { reason } if reason == ABORTED => Some(ABORTED_BY_YOU.to_owned()),
        EntryStatus::Aborted { reason } => Some(format!("failed: {reason}")),
        // Unreachable: `is_interrupted` has taken every `Pending` entry
        // above. Kept as the honest arm of the match rather than a
        // catch-all that would silently call a torn entry a clean one.
        EntryStatus::Pending | EntryStatus::Committed => None,
    }
}

/// git's first stderr line, which is the one that says what went wrong.
fn first_line(reason: &str) -> &str {
    reason.lines().next().unwrap_or(reason)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use graft_core::journal::{EntryId, HeadRef, IndexState, OperationKind, Touches};

    use super::*;
    use crate::jobs::undo::JournalRead;

    /// A distinct, valid oid: the journal only ever compares them.
    fn oid(byte: u8) -> graft_core::oid::Oid {
        graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
    }

    fn entry(id: i64, kind: OperationKind, description: &str) -> JournalEntry {
        JournalEntry {
            id: EntryId::new(id, [0; 10]),
            at_unix_ms: id,
            kind,
            description: description.to_owned(),
            status: EntryStatus::Committed,
            touches: Touches {
                refs: Vec::new(),
                index: true,
                worktree: false,
            },
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: Some(IndexState::Merged(oid(1))),
            index_after: Some(IndexState::Merged(oid(2))),
            worktree_before: None,
            worktree_after: None,
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: vec!["git add -- src/a.rs".to_owned()],
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: std::collections::BTreeMap::new(),
            config_after: std::collections::BTreeMap::new(),
            put_back: None,
        }
    }

    fn state_with(entries: Vec<JournalEntry>) -> AppState {
        let mut state = AppState::default();
        state.journal.entries = Loading::Ready(Arc::new(JournalRead {
            entries,
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        }));
        crate::update::journal::rebuild(&mut state);
        state
    }

    #[test]
    fn rows_are_newest_first_and_the_next_undo_is_marked() {
        let state = state_with(vec![
            entry(1, OperationKind::Stage, "stage src/a.rs"),
            entry(2, OperationKind::Commit, "commit \"x\""),
        ]);
        let view = journal_view(&state, 10);
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].description.as_str(), "commit \"x\"");
        assert_eq!(view.rows[0].mark, RowMark::NextUndo);
        assert_eq!(view.rows[1].mark, RowMark::None);
    }

    #[test]
    fn an_undo_entry_is_the_redo_target_and_what_it_reversed_reads_undone() {
        let mut undo = entry(3, OperationKind::Undo, "undo: commit \"x\"");
        undo.reverses = Some(EntryId::new(2, [0; 10]));
        let state = state_with(vec![
            entry(1, OperationKind::Stage, "stage src/a.rs"),
            entry(2, OperationKind::Commit, "commit \"x\""),
            undo,
        ]);
        let view = journal_view(&state, 10);
        assert_eq!(view.rows[0].mark, RowMark::NextRedo, "the undo itself");
        assert_eq!(view.rows[1].mark, RowMark::Undone, "what it took back");
        assert_eq!(view.rows[2].mark, RowMark::NextUndo, "what `u` reaches now");
    }

    #[test]
    fn an_interrupted_entry_says_so_in_the_row_and_in_full_when_expanded() {
        let mut interrupted = entry(2, OperationKind::Commit, "commit \"x\"");
        interrupted.status = EntryStatus::Pending;
        let state = state_with(vec![interrupted]);
        let view = journal_view(&state, 10);
        assert_eq!(view.rows[0].mark, RowMark::Interrupted);
        assert!(view.rows[0].summary.as_str().starts_with("interrupted"));
        assert!(
            view.rows[0]
                .failure
                .as_ref()
                .is_some_and(|line| line.as_str().starts_with("interrupted:")),
        );
    }

    /// `J` after a stash clash (m6-conflicts/72, 72b): the pop that clashed
    /// is `Committed` — git kept the stash and the tree is unmerged, which
    /// is the pop's own result, not a failure — so its row reads as any
    /// other, never `failed:`. The drop the last file done launched is a
    /// `StashDrop` row named by its position, as every stash is, and it is what `u`
    /// reaches next.
    #[test]
    fn a_clashed_pop_reads_as_a_normal_row_and_its_drop_as_a_stash_drop() {
        let mut pop = entry(1, OperationKind::StashPop, "pop stash@{0}");
        pop.touches.worktree = true;
        pop.put_back = Some(graft_core::journal::PutBack {
            stash: oid(7),
            why: graft_core::journal::PutBackWhy::Pop,
        });
        let stage = entry(2, OperationKind::Stage, "stage notes.md");
        let mut drop = entry(3, OperationKind::StashDrop, "drop stash@{0}");
        drop.touches = Touches {
            refs: vec!["refs/stash".to_owned()],
            index: false,
            worktree: false,
        };
        let app = state_with(vec![pop, stage, drop]);
        let view = journal_view(&app, 10);

        let drop_row = &view.rows[0];
        assert_eq!(drop_row.description.as_str(), "drop stash@{0}");
        assert_eq!(
            drop_row.mark,
            RowMark::NextUndo,
            "`u` brings the stash back"
        );
        assert!(drop_row.failure.is_none());

        let pop_row = &view.rows[2];
        assert_eq!(pop_row.description.as_str(), "pop stash@{0}");
        assert_eq!(pop_row.mark, RowMark::None);
        assert_eq!(
            pop_row.summary.as_str(),
            "the index · the working tree · backup saved"
        );
        assert!(pop_row.failure.is_none(), "a clash is not a failure");
    }

    /// A remote entry: `refs` touched, no index, no tree, no backup.
    fn remote_entry(
        n: i64,
        kind: OperationKind,
        description: &str,
        refs: Vec<String>,
    ) -> JournalEntry {
        let mut entry = entry(n, kind, description);
        entry.touches = Touches {
            refs,
            index: false,
            worktree: false,
        };
        entry.backup_ref = None;
        entry
    }

    /// Push, Fetch and Pull rows in the design note's words (New UI,
    /// "Journal browser rows"). `← u` falls on the pull — the newest
    /// undoable entry — never on the push or fetch newer than it; that mark
    /// is `graft_ops::journal::next_undo`'s, asserted here, not restated.
    #[test]
    fn remote_rows() {
        let tracking = "refs/remotes/origin/main".to_owned();
        let mut pull = entry(1, OperationKind::Pull, "pull origin/main");
        pull.touches = Touches {
            refs: vec!["refs/heads/main".to_owned()],
            index: true,
            worktree: true,
        };
        let mut push = remote_entry(
            2,
            OperationKind::Push,
            "push main → origin/main",
            vec![tracking.clone()],
        );
        let a1 = graft_core::oid::Oid::from_hex("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678")
            .expect("40 hex digits");
        let nine = graft_core::oid::Oid::from_hex("9f8e7d6c5b4a39281706f5e4d3c2b1a098765432")
            .expect("40 hex digits");
        push.refs_before.insert(tracking.clone(), Some(a1));
        push.refs_after.insert(tracking, Some(nine));
        let unrecorded = remote_entry(
            3,
            OperationKind::Push,
            "push topic → origin/topic (set upstream)",
            Vec::new(),
        );
        let fetch = remote_entry(4, OperationKind::Fetch, "fetch all remotes", Vec::new());
        let state = state_with(vec![pull, push, unrecorded, fetch]);
        let view = journal_view(&state, 10);
        let rows: Vec<(&str, &str, RowMark)> = view
            .rows
            .iter()
            .map(|row| (row.description.as_str(), row.summary.as_str(), row.mark))
            .collect();
        assert_eq!(
            rows,
            [
                (
                    "fetch all remotes",
                    "remote-tracking refs · not undoable",
                    RowMark::None
                ),
                (
                    "push topic → origin/topic (set upstream)",
                    "not undoable — origin has it",
                    RowMark::None
                ),
                (
                    "push main → origin/main",
                    "not undoable — origin has it · a1b2c3d → 9f8e7d6",
                    RowMark::None
                ),
                (
                    "pull origin/main",
                    "1 ref · the index · the working tree · backup saved",
                    RowMark::NextUndo
                ),
            ]
        );
    }

    /// What recovery actually leaves behind, and therefore the only shape
    /// the browser ever really sees: `Aborted { reason: "interrupted" }`.
    /// Keyed on `Pending` alone this row was unreachable and read
    /// `failed: interrupted` (m3/06 review, Major 2).
    #[test]
    fn an_entry_recovery_closed_is_marked_interrupted_not_failed() {
        let mut recovered = entry(2, OperationKind::Commit, "commit \"x\"");
        // No before-tree: the process died before one was taken, so `u`
        // cannot reach it and the row is not the `← u` target. That is the
        // shape the warning mark exists for.
        recovered.worktree_before = None;
        recovered.status = EntryStatus::Aborted {
            reason: graft_core::journal::INTERRUPTED.to_owned(),
        };
        let state = state_with(vec![recovered]);
        let view = journal_view(&state, 10);
        assert_eq!(view.rows[0].mark, RowMark::Interrupted);
        let summary = view.rows[0].summary.as_str();
        assert!(summary.starts_with("interrupted"), "{summary}");
        assert!(
            !summary.contains("failed"),
            "a process that died: {summary}"
        );
        assert!(
            view.rows[0]
                .failure
                .as_ref()
                .is_some_and(|line| line.as_str().starts_with("interrupted:")),
        );
    }

    #[test]
    fn an_aborted_entry_leads_with_gits_first_line_and_keeps_the_rest_for_space() {
        let mut aborted = entry(2, OperationKind::Commit, "commit \"x\"");
        // `in_force` counts an Aborted entry only with a backup of the tree
        // it changed; without one there is nothing for `u` to restore.
        aborted.worktree_before = Some(oid(3));
        aborted.status = EntryStatus::Aborted {
            reason: "hook rejected the commit\nsee .git/hooks/pre-commit".to_owned(),
        };
        let state = state_with(vec![aborted]);
        let view = journal_view(&state, 10);
        let summary = view.rows[0].summary.as_str();
        assert!(
            summary.starts_with("failed: hook rejected the commit"),
            "{summary}"
        );
        assert!(!summary.contains("pre-commit"), "{summary}");
        let failure = view.rows[0]
            .failure
            .as_ref()
            .expect("an aborted entry has a reason");
        assert!(failure.as_str().contains("pre-commit"), "{failure:?}");
        // It kept its backup and changed the index, so it is still what `u`
        // reaches: a write that failed part-way is undoable, and the row
        // must say so rather than look inert.
        assert!(view.rows[0].summary.as_str().ends_with("backup saved"));
        assert_eq!(view.rows[0].mark, RowMark::NextUndo);
    }

    #[test]
    fn an_empty_journal_is_empty_not_ready_and_a_failed_read_carries_its_reason() {
        let state = state_with(Vec::new());
        assert_eq!(journal_view(&state, 10).status, JournalStatus::Empty);

        let mut state = AppState::default();
        state.journal.entries = Loading::Failed("line 3: torn".to_owned());
        match journal_view(&state, 10).status {
            JournalStatus::Failed(why) => assert_eq!(why.as_str(), "line 3: torn"),
            other => panic!("a failed read is not {other:?}"),
        }

        let mut state = AppState::default();
        state.journal.entries = Loading::Idle;
        assert_eq!(journal_view(&state, 10).status, JournalStatus::Loading);
    }
    /// M6/68: an entry the user aborted (`--abort`, `Aborted("aborted")`)
    /// is not a failure: the row says who stopped it and what it left.
    #[test]
    fn an_aborted_entry_reads_aborted_by_you() {
        let mut aborted = entry(2, OperationKind::Merge, "merge feature");
        aborted.status = EntryStatus::Aborted {
            reason: "aborted".to_owned(),
        };
        let state = state_with(vec![aborted]);
        let view = journal_view(&state, 10);
        let summary = view.rows[0].summary.as_str();
        assert_eq!(summary, "aborted by you · nothing changed");
        assert!(!summary.contains("failed"), "{summary}");
    }
}
