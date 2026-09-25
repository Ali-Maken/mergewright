//! Turning [`AppState`]'s finish summary into the view model
//! [`graft_ui::finish_summary::FinishScreen`] draws (m6-conflicts/19).
//!
//! The keys and the mouse build the same view to keep the cursor on screen
//! and to hit-test a click, so a click and the draw agree on every row.

use graft_term::{sanitize_line, sanitize_path};
use graft_ui::commit_editor::{BODY_LIMIT, Field, SUBJECT_LIMIT};
use graft_ui::finish_summary::{ExpandedDiff, FinishView, SummaryFocus, SummaryRow, SummaryState};
use graft_ui::graph_screen::HintContext;
use graft_ui::text_input::{AreaView, FieldView};
use ratatui::layout::Rect;

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::finish::{FinishFocus, FinishState, by_you_words};

/// The summary as it draws now, or `None` when none is open.
#[must_use]
pub fn finish_view(state: &AppState) -> Option<FinishView<'static>> {
    let finish = state.finish.as_ref()?;
    let parts = summary_parts(state, finish);
    Some(FinishView {
        names: parts.names,
        by_you: parts.by_you,
        by_git: parts.by_git,
        recorded: parts.recorded,
        cursor: finish.cursor,
        expanded: finish.expanded.as_ref().map(|(row, diff)| {
            let shown = match diff {
                Loading::Ready(diff) => ExpandedDiff::Ready(std::sync::Arc::clone(diff)),
                Loading::Failed(why) => ExpandedDiff::Failed(sanitize_line(why)),
                Loading::Idle | Loading::Pending(_) => ExpandedDiff::Loading,
            };
            (*row, shown)
        }),
        subject: subject_view(finish),
        body: body_view(finish),
        focus: summary_focus(finish.focus),
        expandable: on_a_git_row(finish),
        // A live box claims the next key before the summary does.
        focused: crate::view::rows::box_context(state).is_none(),
        notice: finish
            .notice
            .as_ref()
            .map(|(words, problem)| (sanitize_line(words), *problem)),
        busy: finish.committing.is_some(),
        state: parts.state,
        top: finish.top,
    })
}

/// What the summary's read decides: the branch names, the two file lists,
/// whether git recorded the resolutions, and the read's own state.
struct SummaryParts {
    names: Option<(graft_term::SanitizedText, graft_term::SanitizedText)>,
    by_you: Vec<SummaryRow>,
    by_git: Vec<SummaryRow>,
    recorded: bool,
    state: SummaryState,
}

fn summary_parts(state: &AppState, finish: &FinishState) -> SummaryParts {
    let unread = |shown: SummaryState| SummaryParts {
        // Nothing names the sides before the read lands: no invented names.
        names: None,
        by_you: Vec::new(),
        by_git: Vec::new(),
        recorded: true,
        state: shown,
    };
    let summary = match &finish.summary {
        Loading::Ready(summary) => summary,
        Loading::Failed(why) => return unread(SummaryState::Unavailable(sanitize_line(why))),
        Loading::Idle | Loading::Pending(_) => return unread(SummaryState::Loading),
    };
    let names = crate::update::resolver::side_names(state);
    let from = String::from_utf8_lossy(&summary.from);
    let into = String::from_utf8_lossy(&summary.into);
    SummaryParts {
        names: Some((sanitize_line(&from), sanitize_line(&into))),
        by_you: summary
            .by_you
            .iter()
            .map(|file| SummaryRow {
                path: sanitize_path(&file.path),
                what: sanitize_line(&by_you_words(
                    state.choices.get(&file.path),
                    file.result,
                    &names,
                )),
            })
            .collect(),
        by_git: summary
            .by_git
            .iter()
            .map(|file| SummaryRow {
                path: sanitize_path(&file.path),
                what: sanitize_line(&format!(
                    "{}  +{} \u{2212}{}",
                    change_words(&file.change),
                    file.added,
                    file.removed
                )),
            })
            .collect(),
        recorded: summary.recorded,
        state: SummaryState::Ready,
    }
}

fn subject_view(finish: &FinishState) -> FieldView<'static> {
    let focused = finish.focus == FinishFocus::Subject;
    FieldView {
        label: Field::Subject.label(),
        text: sanitize_line(finish.subject.text()),
        cursor: focused.then(|| finish.subject.cursor()),
        guide_at: u16::try_from(SUBJECT_LIMIT).ok(),
        counter: Some((finish.subject.len(), SUBJECT_LIMIT)),
        focused,
        secret: None,
    }
}

fn body_view(finish: &FinishState) -> AreaView<'static> {
    let focused = finish.focus == FinishFocus::Body;
    AreaView {
        label: Field::Body.label(),
        lines: finish
            .body
            .lines()
            .iter()
            .map(|line| sanitize_line(line))
            .collect(),
        cursor: focused.then(|| finish.body.cursor()),
        guide_at: Some(BODY_LIMIT),
        focused,
    }
}

/// A git-combined file's change, in words.
fn change_words(change: &graft_ops::finish::Change) -> String {
    use graft_ops::finish::Change;
    match change {
        Change::Added => "added".to_owned(),
        Change::Modified => "modified".to_owned(),
        Change::Deleted => "deleted".to_owned(),
        Change::Renamed { from } => format!("renamed from {}", sanitize_path(from).as_str()),
        Change::TypeChanged => "type changed".to_owned(),
    }
}

fn summary_focus(focus: FinishFocus) -> SummaryFocus {
    match focus {
        FinishFocus::Files => SummaryFocus::Files,
        FinishFocus::Subject => SummaryFocus::Subject,
        FinishFocus::Body => SummaryFocus::Body,
    }
}

/// Where the summary draws: the whole frame under the banner line (one row
/// while an operation is live) and above the bar. The keys, the mouse and
/// the frame all measure from here.
#[must_use]
pub fn summary_area(state: &AppState) -> Rect {
    let banner = crate::update::resolver::banner_rows(state);
    Rect {
        x: 0,
        y: banner,
        width: state.size.columns,
        height: state.size.rows.saturating_sub(banner).saturating_sub(1),
    }
}

/// The summary's key context: what `y`, `⏎` and `⇥` do from the focused
/// part. `⏎ diff` is offered on a row git combined on its own, the rows the
/// user has never seen the change of (`HintContext::Finish`'s own meaning);
/// on a row they resolved `⏎`/`space` works too, unadvertised.
#[must_use]
pub fn finish_context(state: &AppState) -> HintContext {
    let finish = state.finish.as_ref();
    HintContext::Finish {
        focus: summary_focus(finish.map_or(FinishFocus::Files, |open| open.focus)),
        expandable: finish.is_some_and(on_a_git_row),
    }
}

/// Does the cursor stand on a row git combined on its own?
fn on_a_git_row(finish: &FinishState) -> bool {
    match &finish.summary {
        Loading::Ready(summary) => {
            finish.cursor >= summary.by_you.len()
                && finish.cursor < summary.by_you.len() + summary.by_git.len()
        }
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => false,
    }
}

/// The keys the summary offers in an area `width` wide: the list its
/// bottom border draws (`finish_summary::footer_hints`), or only `esc back`
/// below the widget's floor, where `Esc` is the only key that acts
/// (`finish::finish_key`) and the widget's own line says so. What the key
/// sweep presses.
#[cfg(test)]
#[must_use]
pub fn finish_hints(state: &AppState, width: u16) -> Vec<graft_ui::graph_screen::Hint> {
    if crate::update::finish::fits(state) {
        finish_view(state).map_or_else(Vec::new, |view| {
            graft_ui::finish_summary::footer_hints(&view, width)
        })
    } else {
        graft_ui::graph_screen::hints_for(
            &finish_context(state),
            graft_ui::graph_screen::Divider::Hidden,
            width,
        )
        .into_iter()
        .filter(|hint| hint.key == "esc")
        .collect()
    }
}

/// The app's bottom row under the summary: `committing…` while the commit
/// runs, else nothing. The keys live in the summary's own frame
/// (m6-conflicts/59).
#[must_use]
pub fn finish_bar(state: &AppState) -> String {
    let busy = state
        .finish
        .as_ref()
        .is_some_and(|open| open.committing.is_some());
    if busy {
        " committing…".to_owned()
    } else {
        String::new()
    }
}
