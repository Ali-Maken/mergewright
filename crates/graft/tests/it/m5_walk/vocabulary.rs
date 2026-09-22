//! The words the scenarios are written in: key presses, and the facts a
//! step or a verdict checks — on the UI state, on a truth, on the journal.

use graft::update::AppState;
use graft::update::preview::preview_box;
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};

use super::run::status_text;
use super::steps::{Step, Wait};
use super::truth::{Truth, diff_truth};
use crate::repo_truth::check;

pub(super) fn key(code: KeyCode) -> Step {
    Step::Key(KeyPress::plain(code))
}

pub(super) fn ch(c: char) -> KeyCode {
    KeyCode::Char(c)
}

pub(super) fn plain(codes: &[KeyCode]) -> Vec<KeyPress> {
    codes.iter().map(|code| KeyPress::plain(*code)).collect()
}

pub(super) fn chain(parts: Vec<Vec<Step>>) -> Vec<Step> {
    parts.into_iter().flatten().collect()
}

/// The preview, its plan, the claim, `Confirm`, the write, then everything
/// re-read: what every ghost-equals-git scenario does once the preview key
/// is pressed.
pub(super) fn claim_and_confirm(what: &'static str) -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::PreviewReady),
        Step::Claim,
        Step::Confirm,
        Step::Write(what),
        Step::WaitFor(Wait::Ready),
    ]
}

/// `u`, the write, everything re-read.
pub(super) fn undo() -> Vec<Step> {
    vec![
        key(ch('u')),
        Step::Write("the undo"),
        Step::WaitFor(Wait::Ready),
    ]
}

/// `U`, the write, everything re-read.
pub(super) fn redo() -> Vec<Step> {
    vec![
        key(ch('U')),
        Step::Write("the redo"),
        Step::WaitFor(Wait::Ready),
    ]
}

// ───────────────────────────── facts on the UI state ─────────────────────────

/// The preview box's title, or why there is none.
pub(super) fn box_title(state: &AppState) -> Result<String, String> {
    preview_box(state)
        .map(|view| view.title.to_string())
        .ok_or_else(|| format!("no preview box is drawn; status {:?}", state.status))
}

/// The preview box's lines, as drawn.
pub(super) fn box_lines(state: &AppState) -> Result<Vec<String>, String> {
    preview_box(state)
        .map(|view| view.lines.iter().map(ToString::to_string).collect())
        .ok_or_else(|| format!("no preview box is drawn; status {:?}", state.status))
}

/// The title starts with `Preview: ` and names `word`.
pub(super) fn title_names(state: &AppState, word: &str) -> Result<(), String> {
    let title = box_title(state)?;
    check(
        title.starts_with("Preview: ") && title.contains(word),
        || format!("the box is titled {title:?}"),
    )
}

/// Some drawn line of the box contains `text`.
pub(super) fn box_says(state: &AppState, text: &str) -> Result<(), String> {
    let lines = box_lines(state)?;
    check(lines.iter().any(|line| line.contains(text)), || {
        format!("no box line has {text:?}: {lines:?}")
    })
}

/// The preview drew at least one ghost row.
pub(super) fn ghosts_drawn(state: &AppState) -> Result<(), String> {
    let preview = state
        .preview
        .as_ref()
        .ok_or_else(|| "no preview is open".to_owned())?;
    check(!preview.ghost_index.is_empty(), || {
        "the preview drew no ghost row".to_owned()
    })
}

/// No preview and no box is open.
pub(super) fn nothing_open(state: &AppState) -> Result<(), String> {
    check(state.preview.is_none() && state.confirm.is_none(), || {
        format!("something is still open; status {:?}", state.status)
    })
}

/// The status line contains `text`.
pub(super) fn status_containing(state: &AppState, text: &str) -> Result<(), String> {
    let status = status_text(state);
    check(status.contains(text), || {
        format!("the status is {status:?}, not about {text:?}")
    })
}

// ───────────────────────────── facts on a truth ──────────────────────────────

/// `refs/heads/<name>`'s object in a truth.
pub(super) fn branch(truth: &Truth, name: &str) -> Result<String, String> {
    let full = format!("refs/heads/{name}");
    truth.repo.refs.get(&full).cloned().ok_or_else(|| {
        format!(
            "no {full}: {:?}",
            truth.repo.refs.keys().collect::<Vec<_>>()
        )
    })
}

/// The refs of two truths are the same.
pub(super) fn same_refs(what: &str, a: &Truth, b: &Truth) -> Result<(), String> {
    check(a.repo.refs == b.repo.refs, || {
        format!("{what}: refs differ: {:?}", diff_truth(a, b))
    })
}

/// The files of two truths are byte-identical.
pub(super) fn same_files(what: &str, a: &Truth, b: &Truth) -> Result<(), String> {
    check(a.repo.files == b.repo.files, || {
        format!("{what}: files differ: {:?}", diff_truth(a, b))
    })
}

/// Some status line the loop drew before this truth was taken contains
/// `text` — the words reached the footer, not only the state.
pub(super) fn a_status_said(truth: &Truth, text: &str) -> Result<(), String> {
    check(
        truth.statuses.iter().any(|line| line.contains(text)),
        || format!("no status line said {text:?}: {:?}", truth.statuses),
    )
}

/// The last journal entry, or why there is none.
pub(super) fn last_entry(truth: &Truth) -> Result<&JournalEntry, String> {
    truth
        .journal
        .last()
        .ok_or_else(|| "the journal is empty".to_owned())
}

/// The last entry is of `kind` and committed.
pub(super) fn last_entry_committed(truth: &Truth, kind: OperationKind) -> Result<(), String> {
    let entry = last_entry(truth)?;
    check(
        entry.kind == kind && entry.status == EntryStatus::Committed,
        || format!("the last entry is {:?} {:?}", entry.kind, entry.status),
    )
}
