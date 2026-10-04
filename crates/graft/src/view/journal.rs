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
use graft_ui::journal_screen::{
    JournalFindView, JournalRowView, JournalStatus, JournalView, RowKind, RowMark,
};

use crate::update::AppState;
use crate::update::detail::Loading;

mod summary;
use summary::{failure, short, summary};

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
    let ready_list = match status {
        JournalStatus::Ready => state.journal.list.as_deref(),
        JournalStatus::Loading | JournalStatus::Empty | JournalStatus::Failed(_) => None,
    };
    let enter_words = ready_list.and_then(|list| enter_words(list, state, selected));
    // `b new branch here` only where `b` has a commit to name: the same
    // `row_commit` the key reads (m7/30o Minor 2).
    let branch_here =
        ready_list.is_some_and(|list| crate::update::journal::row_commit(list, selected).is_some());
    JournalView {
        location,
        rows,
        selected,
        status,
        title_suffix: title_suffix(state),
        status_row: None,
        enter_words,
        branch_here,
        box_open: state.preview.is_some(),
        find: find_view(state),
        find_prompt: state.journal.find.typing.as_deref().map(sanitize_line),
    }
}

/// The kept `/` find for the title (m7-power-tools/30o-c): its words and
/// which hit the cursor is at, counted as blame's `FindTitle` is.
fn find_view(state: &AppState) -> Option<JournalFindView> {
    let query = state.journal.find.kept.as_deref()?;
    let (at, of) = crate::update::journal::find::count(&state.journal)?;
    Some(JournalFindView {
        query: sanitize_line(query),
        at: (of > 0).then_some(at),
        of,
    })
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
    // Not read (m7/30o Minor 5): the sha7 alone, no quotes around nothing.
    if let Some(subject) = &commit.subject {
        words.push_str(&format!(" \"{subject}\""));
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
            // A step Graft doesn't undo (a fetch, a push): `⏎` has nothing
            // to go back to there, so no promise (m7/30o Minor 4).
            if crate::update::journal::not_undone_kind(entry).is_some() {
                return None;
            }
            if Some(id) == marks.next_undo {
                "undo this".to_owned()
            } else if Some(id) == marks.next_redo {
                "redo this".to_owned()
            } else if matches!(entry.kind, OperationKind::Undo | OperationKind::Redo) {
                // The time machine refuses an undo or redo row
                // (`update::journal::enter_graft`): no promise.
                return None;
            } else {
                "revert to before this".to_owned()
            }
        }
        ListRow::Outside(step) => {
            if let Some(from) = switched_from(&step.record.subject) {
                format!("switch back to {from}")
            } else {
                let branch = step.record.refname.strip_prefix("refs/heads/")?;
                step.record.old?;
                if step.record.new.is_none() {
                    format!("make {branch} again")
                } else {
                    format!("put {branch} back to before this")
                }
            }
        }
        ListRow::Lost { commit, .. } => match &commit.held_by {
            HeldBy::Branch { name, gone: false } => format!("put {name} back to it"),
            HeldBy::Branch { name, gone: true } => format!("make {name} again"),
            HeldBy::Head => return None,
        },
        ListRow::LostEdit { edit, .. } => {
            if edit.paths.len() > 1 {
                "put your edits back".to_owned()
            } else {
                "put your edit back".to_owned()
            }
        }
    };
    Some(sanitize_line(&words).as_str().to_owned())
}

/// The branch a shell switch left, for the bar: `checkout: moving from
/// main to spike` → `main`; a detached start is its sha7. Worded from
/// `update::journal::switched_from`'s answer, the one parse `⏎` switches
/// by (m7/30o Minor 1), so the bar and the key cannot read it apart.
fn switched_from(subject: &str) -> Option<String> {
    use crate::update::switch::SwitchTarget;
    match crate::update::journal::switched_from(subject)? {
        SwitchTarget::Branch(name) => {
            Some(name.strip_prefix("refs/heads/").unwrap_or(&name).to_owned())
        }
        SwitchTarget::Detached(oid) => Some(short(oid)),
        // The parse makes neither: a reflog subject names no remote to
        // track and no branch to make. Nothing to name, so no hint.
        SwitchTarget::Track { .. } | SwitchTarget::NewBranch { .. } => None,
    }
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

#[cfg(test)]
mod tests;
