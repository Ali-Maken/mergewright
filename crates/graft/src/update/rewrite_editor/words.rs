//! The editor's words, and what it hands the view (`view_facts`).

use graft_ops::rewrite::{TodoAction, TodoProblem, squash_groups};

use super::{After, Arc, BaseRow, FileList, Loading, MarkedBy, Oid, Planned, RewriteEditor, Right};
use crate::update::state::Status;
use graft_ui::keymap::{ActionId, Keymap};

/// `p s f d e r`, a move, onto the base row (R1).
pub(super) const BASE_STAYS: &str = "the base stays \u{b7} it isn't yours to rewrite";

/// Opened from a rebase whose every row is a suggested drop he has not
/// touched: he pressed `R` to keep one, so the first words say where he is
/// and the next key, not a refusal (30n Q1; ux-principles §3.1, H9).
/// `p` is the editor's keep (`rewrite.pick`); unbound, `nothing kept yet`.
pub(crate) fn nothing_kept_yet(keys: &Keymap) -> String {
    keys.after(
        "nothing kept yet",
        ActionId("rewrite.pick"),
        "keeps a commit",
    )
}

/// An invalid todo's words, on the AFTER side and on `y`: the problem's,
/// except [`nothing_kept_yet`] while every row is still a drop the rebase
/// suggested. Once he marks any of them himself, every drop is his choice
/// and the problem's own words come back.
pub(super) fn invalid(editor: &RewriteEditor, problem: &TodoProblem, keys: &Keymap) -> String {
    let arrived = *problem == TodoProblem::EverythingDropped
        && editor.todo.iter().all(|row| {
            row.action == TodoAction::Drop
                && editor.marked_by.get(&row.commit) == Some(&MarkedBy::AlreadyIn)
        });
    if arrived {
        nothing_kept_yet(keys)
    } else {
        problem.explain()
    }
}

/// `r` on a melded row: its message is the kept row's. `r` is the
/// editor's reword (`rewrite.reword`); unbound, the fact alone.
pub(crate) fn melds_into(kept_subject: &str, keys: &Keymap) -> String {
    keys.after(
        &format!("this row melds into \"{kept_subject}\""),
        ActionId("rewrite.reword"),
        "there changes the message",
    )
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
pub(super) fn view_facts<'a>(editor: &'a RewriteEditor, keys: &Keymap) -> ViewFacts<'a> {
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
            After::Invalid(problem) => AfterFacts::Invalid(invalid(editor, problem, keys)),
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
                marked: editor.marked_by.get(&row.commit).copied(),
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

/// What the view (`view::rewrite_editor`) reads to draw the editor: everything on screen, in the
/// screen's order, and nothing it has to work out again.
#[derive(Debug)]
pub(crate) struct ViewFacts<'a> {
    /// His branch's short name (`feature-a`).
    pub branch: &'a str,
    /// How many commits since it left the base, as opened.
    pub since: usize,
    /// His rows, newest first.
    pub rows: Vec<RowFacts<'a>>,
    /// The row under his.
    pub base: &'a BaseRow,
    /// The cursor: an index into `rows`, or `rows.len()` on the base row.
    pub cursor: usize,
    /// `r`'s field: its screen row, text and caret (a char index).
    pub field: Option<(usize, &'a str, usize)>,
    /// One block per squash group or reword, newest kept row first: what
    /// the rewrite will write.
    pub messages: Vec<MessageFacts>,
    /// The AFTER side.
    pub after: AfterFacts<'a>,
    /// `Some` while `⏎` shows a row's changes instead of AFTER.
    pub changes: Option<(Oid, &'a Loading<Arc<FileList>>)>,
    /// Below 120 columns: the AFTER side is up instead of the rows.
    pub narrow_shows_right: bool,
    /// A refusal in words, above the bar.
    pub notice: Option<&'a str>,
    /// The `$EDITOR` handover is out: the terminal is away.
    pub handing_over: bool,
}

/// One of his rows.
#[derive(Debug)]
pub(crate) struct RowFacts<'a> {
    /// Its action now.
    pub action: TodoAction,
    /// Its commit.
    pub commit: Oid,
    /// Its subject as it stands (a reword's edited subject).
    pub subject: &'a str,
    /// `↕ moved`.
    pub moved: bool,
    /// Why it came up marked, while he has not re-marked it: `marked by its
    /// name`, or `already in <base> · p keeps it`.
    pub marked: Option<MarkedBy>,
    /// Its commit is the kept row of a group: `n in one` counts it and the
    /// melded rows.
    pub keeps: usize,
    /// What is wrong with it (`nothing below to meld into`), drawn `✗`.
    pub problem: Option<String>,
}

/// One message the rewrite will write.
#[derive(Debug)]
pub(crate) struct MessageFacts {
    /// The kept (or reworded) row.
    pub kept: Oid,
    /// Its subject, for the label's `r on "<subject>"`.
    pub kept_subject: String,
    /// A squash group's (`MESSAGE of the squashed commit`) or a reword's.
    pub squashed: bool,
    /// The whole message.
    pub text: String,
}

/// The AFTER side's state.
#[derive(Debug)]
pub(crate) enum AfterFacts<'a> {
    /// `checking…`.
    Checking,
    /// The read failed.
    Failed(&'a str),
    /// The plan refused.
    Refused(&'a str),
    /// The todo cannot run: the problem's words.
    Invalid(String),
    /// The plan: its ghosts, forecast and `published`.
    Ready(&'a Planned),
}
