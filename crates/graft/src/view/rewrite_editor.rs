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
use graft_term::{SanitizedText, sanitize_line, sanitize_path_bytes};
use graft_ui::graph_screen::HintContext;
use graft_ui::rewrite_editor::{
    AfterMark, AfterRow, AfterView, ChangesView, EditorRow, FieldView, MessageBlock, NARROW_BELOW,
    RewriteEditorView, RightSide, RowMark,
};
use ratatui::layout::Rect;

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::place::short;
use crate::update::rewrite_editor::{AfterFacts, RowFacts, ViewFacts};

/// `melds into the row below`: a squash or fixup row.
const MELDS: &str = "melds into the row below";

/// Where the editor is drawn: the whole frame but its last row, the bar.
/// `update::rewrite_editor`'s mouse asks the same rectangle.
#[must_use]
pub fn editor_area(state: &AppState) -> Rect {
    Rect::new(
        0,
        0,
        state.size.columns,
        state.size.rows.saturating_sub(1),
    )
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
    let commits = if facts.since == 1 { "commit" } else { "commits" };
    let title = format!(
        "REWRITE {} \u{b7} {} {commits} since it left {} \u{b7} newest first \u{b7} nothing \
         written until y",
        facts.branch, facts.since, facts.base.label
    );
    let mut rows: Vec<EditorRow> = facts
        .rows
        .iter()
        .map(|row| editor_row(row, ascii))
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

/// One of his rows: its mark, and what it says about itself.
fn editor_row(row: &RowFacts<'_>, ascii: bool) -> EditorRow {
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
    if row.by_name {
        said.push("marked by its name".to_owned());
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

/// The right-hand side: `⏎`'s file list, else the AFTER plan.
fn right_side(facts: &ViewFacts<'_>) -> RightSide {
    if let Some((commit, files)) = facts.changes {
        let files = match files {
            Loading::Idle | Loading::Pending(_) => ChangesView::Reading,
            Loading::Failed(why) => ChangesView::Failed(sanitize_line(why)),
            Loading::Ready(list) => ChangesView::Files(
                list.files
                    .iter()
                    .map(|file| {
                        (
                            sanitize_path_bytes(&file.path),
                            file.added.unwrap_or(0),
                            file.removed.unwrap_or(0),
                        )
                    })
                    .collect(),
            ),
        };
        return RightSide::Changes {
            short: short(commit),
            files,
        };
    }
    RightSide::After(match &facts.after {
        AfterFacts::Checking => AfterView::Checking,
        AfterFacts::Failed(why) => {
            AfterView::Failed(sanitize_line(&format!("couldn't plan the rewrite: {why}")))
        }
        AfterFacts::Refused(words) => AfterView::Refused(sanitize_line(words)),
        AfterFacts::Invalid(words) => AfterView::Refused(sanitize_line(words)),
        AfterFacts::Ready(planned) => after_ready(facts, &planned.plan),
    })
}

/// The plan's ghosts, per branch top down (the last dependent first, his
/// branch last), then the base; below them the facts.
fn after_ready(facts: &ViewFacts<'_>, plan: &PreviewPlan) -> AfterView {
    let owner = ghost_owners(plan);
    let rows_by_commit: HashMap<Oid, &RowFacts<'_>> =
        facts.rows.iter().map(|row| (row.commit, row)).collect();
    let stop = match &plan.conflicts {
        ConflictForecast::StopsAt { commit, paths, .. } => plan
            .ghosts
            .iter()
            .position(|ghost| ghost.replays == Some(*commit))
            .map(|at| (at, paths.as_slice())),
        _ => None,
    };
    let his_tip = plan.ref_moves.first().and_then(|moved| match moved.to {
        Some(Target::Ghost(at)) => Some(at),
        _ => None,
    });
    let mut rows = Vec::new();
    for (branch, moved) in plan.ref_moves.iter().enumerate().rev() {
        let mut labelled = false;
        let mine = owner
            .iter()
            .filter(|(_, by)| *by == branch)
            .map(|(at, _)| *at);
        for at in mine.rev() {
            let Some(ghost) = plan.ghosts.get(at) else {
                continue;
            };
            let replayed = ghost.replays.and_then(|commit| rows_by_commit.get(&commit));
            let mut notes = Vec::new();
            if branch > 0 && !labelled {
                notes.push("built on yours \u{b7} follows".to_owned());
            }
            if let Some(row) = replayed {
                notes.extend(group_note(facts, row));
            }
            match stop {
                Some((stops, paths)) if stops == at => notes.push(format!(
                    "! may stop here \u{b7} {}",
                    paths_words(paths)
                )),
                Some((stops, _)) if at > stops => notes.push("not checked past here".to_owned()),
                _ => {}
            }
            rows.push(AfterRow {
                branch: (!labelled).then(|| sanitize_line(&short_ref(&moved.name))),
                mark: if Some(at) == his_tip {
                    AfterMark::NewTip
                } else {
                    AfterMark::New
                },
                subject: sanitize_line(&ghost.subject),
                note: (!notes.is_empty()).then(|| sanitize_line(&notes.join(" \u{b7} "))),
            });
            labelled = true;
        }
    }
    rows.push(AfterRow {
        branch: Some(sanitize_line(&facts.base.label)),
        mark: AfterMark::Unchanged {
            short: short(facts.base.oid),
        },
        subject: sanitize_line(&facts.base.subject),
        note: Some(sanitize_line("unchanged")),
    });
    AfterView::Ready {
        rows,
        facts: after_facts(plan, &owner),
    }
}

/// Which branch (index into `ref_moves`) each ghost is on: from each moved
/// ref's new tip down its first parents, until a real commit or a ghost
/// another branch already holds (his branch's ghosts are claimed first, so
/// a dependent holds only its own picks).
fn ghost_owners(plan: &PreviewPlan) -> Vec<(usize, usize)> {
    let mut owner: Vec<Option<usize>> = vec![None; plan.ghosts.len()];
    for (branch, moved) in plan.ref_moves.iter().enumerate() {
        let mut at = match moved.to {
            Some(Target::Ghost(at)) => Some(at),
            _ => None,
        };
        while let Some(index) = at {
            match owner.get(index) {
                Some(None) => owner[index] = Some(branch),
                _ => break,
            }
            at = match plan.ghosts[index].parents.first() {
                Some(Target::Ghost(below)) => Some(*below),
                _ => None,
            };
        }
    }
    owner
        .into_iter()
        .enumerate()
        .filter_map(|(at, by)| by.map(|by| (at, by)))
        .collect()
}

/// `3 in one` on a kept row with a squash melding in, `fixup melded in`
/// on one with fixups alone.
fn group_note(facts: &ViewFacts<'_>, row: &RowFacts<'_>) -> Option<String> {
    if row.keeps < 2 {
        return None;
    }
    // The melded rows are the ones right above the kept row on screen.
    let at = facts.rows.iter().position(|it| it.commit == row.commit)?;
    let melded = facts.rows.get(at.saturating_sub(row.keeps - 1)..at)?;
    if melded.iter().all(|it| it.action == TodoAction::Fixup) {
        Some("fixup melded in".to_owned())
    } else {
        Some(format!("{} in one", row.keeps))
    }
}

/// The facts under the AFTER rows: who follows, what is on origin, the
/// conflict forecast.
fn after_facts(plan: &PreviewPlan, owner: &[(usize, usize)]) -> Vec<SanitizedText> {
    let mut out = Vec::new();
    let followers: Vec<String> = plan
        .ref_moves
        .iter()
        .enumerate()
        .skip(1)
        .map(|(branch, moved)| {
            let mine: Vec<usize> = owner
                .iter()
                .filter(|(_, by)| *by == branch)
                .map(|(at, _)| *at)
                .collect();
            let count = mine.len();
            let who = mine
                .last()
                .and_then(|&at| plan.ghosts.get(at))
                .map(|ghost| String::from_utf8_lossy(&ghost.author.name).into_owned());
            let commits = if count == 1 { "commit" } else { "commits" };
            match who {
                Some(who) => format!("{} ({who}, {count} {commits})", short_ref(&moved.name)),
                None => format!("{} ({count} {commits})", short_ref(&moved.name)),
            }
        })
        .collect();
    if !followers.is_empty() {
        let verb = if followers.len() == 1 {
            "follows"
        } else {
            "follow"
        };
        out.push(format!("built on yours: {} {verb}", followers.join(", ")));
    }
    if !plan.published.is_empty() {
        let names: Vec<String> = plan
            .published
            .iter()
            .map(|name| String::from_utf8_lossy(name.shorthand()).into_owned())
            .collect();
        out.push(format!(
            "on origin: {} \u{b7} P will need force after",
            names.join(", ")
        ));
    }
    out.push(match &plan.conflicts {
        ConflictForecast::Clean => "no conflicts expected".to_owned(),
        ConflictForecast::Likely { paths } => format!("! may conflict \u{b7} {}", paths_words(paths)),
        ConflictForecast::NotChecked(why) => format!("conflicts: not checked ({why})"),
        ConflictForecast::StopsAt { step, of, .. } => {
            format!("! may stop at step {step} of {of} \u{b7} not checked past it")
        }
    });
    out.into_iter().map(|line| sanitize_line(&line)).collect()
}

/// Up to three paths, then `+N more`.
fn paths_words(paths: &[std::path::PathBuf]) -> String {
    let mut shown: Vec<String> = paths
        .iter()
        .take(3)
        .map(|path| path.display().to_string())
        .collect();
    if paths.len() > 3 {
        shown.push(format!("+{} more", paths.len() - 3));
    }
    shown.join(", ")
}

/// `refs/heads/feature-b` → `feature-b`.
fn short_ref(name: &str) -> String {
    name.strip_prefix("refs/heads/").unwrap_or(name).to_owned()
}
