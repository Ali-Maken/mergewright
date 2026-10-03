//! The rewrite editor's view (m7-power-tools/08, note R1): the editor's
//! facts (`update::rewrite_editor::ViewFacts`) turned into the
//! [`RewriteEditorView`] `graft_ui::rewrite_editor` draws, the area it is
//! drawn in, and its bar's context.
//!
//! Every word the editor shows that is not a refusal is chosen here: the
//! title, each row's note, the message label, the AFTER rows and facts.
//! Refusals arrive in their owners' words (`graft_ops`' `explain`,
//! `operate::refusal_status`) and are drawn as given (one wording, H4).

use std::collections::HashMap;

use graft_core::oid::Oid;
use graft_ops::preview::{ConflictForecast, PreviewPlan, Target};
use graft_ops::rewrite::TodoAction;
use graft_term::{sanitize_line, sanitize_path_bytes};
use graft_ui::graph_screen::HintContext;
use graft_ui::rewrite_editor::{
    AfterMark, AfterRow, AfterView, ChangesView, EditorRow, FieldView, MessageBlock, NARROW_BELOW,
    RewriteEditorView, RightSide, RowMark,
};
use ratatui::layout::Rect;

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::place::short;
use crate::update::rewrite_editor::{AfterFacts, MarkedBy, RowFacts, ViewFacts};

mod right;
use right::right_side;

/// `melds into the row below`: a squash or fixup row.
const MELDS: &str = "melds into the row below";

/// Where the editor is drawn: the whole frame but its last row, the bar.
/// `update::rewrite_editor`'s mouse asks the same rectangle.
#[must_use]
pub fn editor_area(state: &AppState) -> Rect {
    Rect::new(0, 0, state.size.columns, state.size.rows.saturating_sub(1))
}

/// The editor as drawn, `None` while none is up.
#[must_use]
pub fn rewrite_view(state: &AppState, ascii: bool) -> Option<RewriteEditorView> {
    let editor = state.rewrite.editor.as_deref()?;
    Some(view_of(&editor.view_facts(), ascii))
}

/// The editor's bar: its field's keys while `r` has the field open, else
/// its own, `y rewrite` only while `y` would start (never on a refused,
/// failed or invalid plan) and `tab after` below 120 columns.
#[must_use]
pub fn rewrite_context(state: &AppState) -> Option<HintContext> {
    let editor = state.rewrite.editor.as_deref()?;
    let facts = editor.view_facts();
    if facts.field.is_some() {
        return Some(HintContext::RewriteMessage);
    }
    Some(HintContext::RewriteEditor {
        narrow: state.size.columns < NARROW_BELOW,
        can_start: matches!(facts.after, AfterFacts::Checking | AfterFacts::Ready(_)),
    })
}

/// The whole view from the editor's facts.
fn view_of(facts: &ViewFacts<'_>, ascii: bool) -> RewriteEditorView {
    let commits = if facts.since == 1 {
        "commit"
    } else {
        "commits"
    };
    let title = format!(
        "REWRITE {} \u{b7} {} {commits} since it left {} \u{b7} newest first \u{b7} nothing \
         written until y",
        facts.branch, facts.since, facts.base.label
    );
    let mut rows: Vec<EditorRow> = facts
        .rows
        .iter()
        .map(|row| editor_row(row, &facts.base.label, ascii))
        .collect();
    rows.push(EditorRow {
        mark: RowMark::Base {
            label: sanitize_line(&facts.base.label),
        },
        short: short(facts.base.oid),
        subject: sanitize_line(&facts.base.subject),
        note: Some(sanitize_line("stays")),
        moved: false,
    });
    RewriteEditorView {
        title: sanitize_line(&title),
        rows,
        cursor: facts.cursor,
        field: facts.field.map(|(row, text, caret)| FieldView {
            row,
            text: sanitize_line(text),
            caret,
        }),
        message: message_block(facts),
        right: right_side(facts),
        narrow_shows_right: facts.narrow_shows_right,
        // While `$EDITOR` has the terminal nothing here is drawn; a frame
        // that lands before it takes over says where the message went.
        notice: if facts.handing_over {
            Some(sanitize_line("writing the message in $EDITOR\u{2026}"))
        } else {
            facts.notice.map(sanitize_line)
        },
        ascii,
    }
}

/// One of his rows: its mark, and what it says about itself. `base` is the
/// base row's label, which a rebase's suggested drop is already in.
fn editor_row(row: &RowFacts<'_>, base: &str, ascii: bool) -> EditorRow {
    let mark = match row.action {
        TodoAction::Pick => RowMark::Pick,
        TodoAction::Reword => RowMark::Reword,
        TodoAction::Edit => RowMark::Edit,
        TodoAction::Squash => RowMark::Squash,
        TodoAction::Fixup => RowMark::Fixup,
        TodoAction::Drop => RowMark::Drop,
    };
    let mut said: Vec<String> = Vec::new();
    if let Some(problem) = &row.problem {
        let cross = if ascii { "x" } else { "\u{2717}" };
        said.push(format!("{cross} {problem}"));
    } else {
        match row.action {
            TodoAction::Squash | TodoAction::Fixup => said.push(MELDS.to_owned()),
            TodoAction::Reword => said.push("new message".to_owned()),
            TodoAction::Edit => said.push("stops here so you can change files".to_owned()),
            TodoAction::Drop => said.push("left out".to_owned()),
            TodoAction::Pick => {}
        }
    }
    match row.marked {
        Some(MarkedBy::Name) => said.push("marked by its name".to_owned()),
        Some(MarkedBy::AlreadyIn) => said.push(format!("already in {base} \u{b7} p keeps it")),
        None => {}
    }
    EditorRow {
        mark,
        short: short(row.commit),
        subject: sanitize_line(row.subject),
        note: (!said.is_empty()).then(|| sanitize_line(&said.join(" \u{b7} "))),
        moved: row.moved,
    }
}

/// The message block: the one whose kept row is the cursor's, else the
/// newest. `MESSAGE of the squashed commit · r on "<kept subject>"
/// changes it`.
fn message_block(facts: &ViewFacts<'_>) -> Option<MessageBlock> {
    let cursor_commit = facts.rows.get(facts.cursor).map(|row| row.commit);
    let message = facts
        .messages
        .iter()
        .find(|message| Some(message.kept) == cursor_commit)
        .or_else(|| facts.messages.first())?;
    let subject = facts
        .rows
        .iter()
        .find(|row| row.commit == message.kept)
        .map_or(message.kept_subject.as_str(), |row| row.subject);
    let what = if message.squashed {
        "squashed"
    } else {
        "reworded"
    };
    Some(MessageBlock {
        label: sanitize_line(&format!(
            "MESSAGE of the {what} commit \u{b7} r on \"{subject}\" changes it"
        )),
        lines: message.text.lines().map(sanitize_line).collect(),
    })
}
