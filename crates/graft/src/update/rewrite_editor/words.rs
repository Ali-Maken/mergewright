//! The editor's words, and what it hands the view (`view_facts`).

use graft_ops::rewrite::{TodoAction, TodoProblem, squash_groups};

use super::{After, AfterFacts, MarkedBy, MessageFacts, RewriteEditor, Right, RowFacts, ViewFacts};
use crate::update::state::Status;

/// `p s f d e r`, a move, onto the base row (R1).
pub(super) const BASE_STAYS: &str = "the base stays \u{b7} it isn't yours to rewrite";

/// Opened from a rebase whose every row is a suggested drop he has not
/// touched: he pressed `R` to keep one, so the first words say where he is
/// and the next key, not a refusal (30n Q1; ux-principles §3.1, H9).
pub(super) const NOTHING_KEPT_YET: &str = "nothing kept yet \u{b7} p keeps a commit";

/// An invalid todo's words, on the AFTER side and on `y`: the problem's,
/// except [`NOTHING_KEPT_YET`] while every row is still a drop the rebase
/// suggested. Once he marks any of them himself, every drop is his choice
/// and the problem's own words come back.
pub(super) fn invalid(editor: &RewriteEditor, problem: &TodoProblem) -> String {
    let arrived = *problem == TodoProblem::EverythingDropped
        && editor.todo.iter().all(|row| {
            row.action == TodoAction::Drop
                && editor.by_name.get(&row.commit) == Some(&MarkedBy::AlreadyIn)
        });
    if arrived {
        NOTHING_KEPT_YET.to_owned()
    } else {
        problem.explain()
    }
}

/// `r` on a melded row: its message is the kept row's.
pub(super) fn melds_into(kept_subject: &str) -> String {
    format!("this row melds into \"{kept_subject}\" \u{b7} r there changes the message")
}

/// A status line as one line of words: `reason · keys`.
pub(super) fn status_words(status: &Status) -> String {
    match status {
        Status::Sticky {
            reason,
            keys: Some(keys),
        } => format!("{reason} \u{b7} {keys}"),
        Status::Sticky { reason, keys: None } => reason.clone(),
        Status::Failed(words) | Status::Waiting(words) | Status::Busy(words) => words.clone(),
        other => format!("{other:?}"),
    }
}

/// The whole message the row at todo `index` will be written with, as it
/// stands: his edit; else, for the kept row of a group a `squash` melds
/// into, `default_message`; else the message read at open.
/// `None` for an index past the rows.
pub(super) fn message_of(editor: &RewriteEditor, index: usize) -> Option<String> {
    let row = editor.todo.get(index)?;
    if let Some(edit) = editor.edits.get(&row.commit) {
        return Some(edit.clone());
    }
    if let Some(group) = editor.group_of(index)
        && let Some(message) = editor.group_message(&group)
    {
        return Some(message);
    }
    Some(editor.whole_of(row))
}

/// `message` with its first line replaced by `subject`.
pub(super) fn with_subject(message: &str, subject: &str) -> String {
    match message.split_once('\n') {
        Some((_, rest)) => format!("{subject}\n{rest}"),
        None => format!("{subject}\n"),
    }
}

/// Everything the view draws, newest row first.
pub(super) fn view_facts(editor: &RewriteEditor) -> ViewFacts<'_> {
    let count = editor.todo.len();
    let rows = rows(editor);
    let field = editor.field.as_ref().and_then(|field| {
        let index = editor
            .todo
            .iter()
            .position(|row| row.commit == field.commit)?;
        Some((count - 1 - index, field.text.text(), field.text.cursor()))
    });
    ViewFacts {
        branch: editor
            .opened
            .head
            .name
            .strip_prefix("refs/heads/")
            .unwrap_or(&editor.opened.head.name),
        since: editor.opened.todo.len(),
        rows,
        base: &editor.base,
        cursor: editor.cursor,
        field,
        messages: messages(editor),
        after: match &editor.after {
            After::NotAsked | After::Pending { .. } => AfterFacts::Checking,
            After::Ready { planned, .. } => AfterFacts::Ready(planned),
            After::Failed(why) => AfterFacts::Failed(why),
            After::Refused(words) => AfterFacts::Refused(words),
            After::Invalid(problem) => AfterFacts::Invalid(invalid(editor, problem)),
        },
        changes: match &editor.right {
            Right::Changes { commit, files } => Some((*commit, files)),
            Right::After => None,
        },
        narrow_shows_right: editor.narrow_shows_right,
        notice: editor.notice.as_deref(),
        handing_over: editor
            .field
            .as_ref()
            .is_some_and(|field| field.handover.is_some()),
    }
}

/// His rows, newest first, as the view draws them.
fn rows(editor: &RewriteEditor) -> Vec<RowFacts<'_>> {
    let groups = squash_groups(&editor.todo);
    (0..editor.todo.len())
        .rev()
        .map(|index| {
            let row = &editor.todo[index];
            // A reword's subject is his edit's first line (an empty edit
            // is an empty subject, not a missing one).
            let subject = match editor.edits.get(&row.commit) {
                Some(edit) if row.action == TodoAction::Reword => {
                    edit.lines().next().unwrap_or_default()
                }
                _ => row.subject.as_str(),
            };
            let keeps = groups
                .iter()
                .find(|group| group.kept == index)
                .map_or(0, |group| 1 + group.melded.len());
            let problem = (row.action.melds() && editor.melds_into(index).is_none())
                .then(|| TodoProblem::MeldsIntoNothing { row: index }.explain());
            RowFacts {
                action: row.action,
                commit: row.commit,
                subject,
                moved: editor.moved.contains(&row.commit),
                marked: editor.by_name.get(&row.commit).copied(),
                keeps,
                problem,
            }
        })
        .collect()
}

/// One block per message the rewrite writes: each squash group's (when it
/// writes one) and each reword's, newest kept row first.
fn messages(editor: &RewriteEditor) -> Vec<MessageFacts> {
    let groups = squash_groups(&editor.todo);
    let mut out = Vec::new();
    for (index, row) in editor.todo.iter().enumerate().rev() {
        let group = groups.iter().find(|group| group.kept == index);
        let text = match group {
            Some(group) => editor.group_message(group),
            None if row.action == TodoAction::Reword => editor.edits.get(&row.commit).cloned(),
            None => None,
        };
        if let Some(text) = text {
            out.push(MessageFacts {
                kept: row.commit,
                kept_subject: text.lines().next().unwrap_or_default().to_owned(),
                squashed: group.is_some(),
                text,
            });
        }
    }
    out
}
