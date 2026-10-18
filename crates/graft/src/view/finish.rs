//! Turning [`AppState`]'s merge review into the view model
//! [`graft_ui::finish_summary::FinishScreen`] draws (ease-pass/12;
//! m6-conflicts/19 before it).
//!
//! The keys and the mouse build the same view to keep the cursor on screen
//! and to hit-test a click, so a click and the draw agree on every row.

use graft_term::{sanitize_line, sanitize_path};
use graft_ui::finish_summary::{ExpandedDiff, ReviewFile, ReviewView, SummaryState};
use graft_ui::graph_screen::HintContext;
use graft_ui::graph_screen::hints::ReviewBar;
use ratatui::layout::Rect;

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::finish::{FinishState, how_solved, subject_of};

/// The review as it draws now, or `None` when none is open.
#[must_use]
pub fn finish_view(state: &AppState) -> Option<ReviewView<'_>> {
    let finish = state.finish.as_ref()?;
    let parts = review_parts(state, finish);
    Some(ReviewView {
        keys: &state.keymap,
        from: parts.from,
        into: parts.into,
        clashes: parts.clashed.iter().map(|file| file.how.clashes()).sum(),
        incoming: finish.incoming,
        clashed: parts.clashed,
        combined: parts.combined,
        message: parts.message,
        cursor: finish.cursor,
        expanded: finish.expanded.as_ref().map(|(row, diff)| {
            let shown = match diff {
                Loading::Ready(diff) => ExpandedDiff::Ready(std::sync::Arc::clone(diff)),
                Loading::Failed(why) => ExpandedDiff::Failed(sanitize_line(why)),
                Loading::Idle | Loading::Pending(_) => ExpandedDiff::Loading,
            };
            (*row, shown)
        }),
        // A live box claims the next key before the review does.
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

/// What the review's read decides: the branch names, the clashed rows, how
/// many files git combined, the message's subject and the read's state.
struct ReviewParts {
    from: graft_term::SanitizedText,
    into: graft_term::SanitizedText,
    clashed: Vec<ReviewFile>,
    combined: u32,
    message: graft_term::SanitizedText,
    state: SummaryState,
}

fn review_parts(state: &AppState, finish: &FinishState) -> ReviewParts {
    let unread = |shown: SummaryState| ReviewParts {
        // Nothing names the branches before the read lands: no invented names.
        from: sanitize_line(""),
        into: sanitize_line(""),
        clashed: Vec::new(),
        combined: 0,
        message: sanitize_line(""),
        state: shown,
    };
    let summary = match &finish.summary {
        Loading::Ready(summary) => summary,
        Loading::Failed(why) => return unread(SummaryState::Unavailable(sanitize_line(why))),
        Loading::Idle | Loading::Pending(_) => return unread(SummaryState::Loading),
    };
    ReviewParts {
        from: sanitize_line(&String::from_utf8_lossy(&summary.from)),
        into: sanitize_line(&String::from_utf8_lossy(&summary.into)),
        clashed: summary
            .by_you
            .iter()
            .map(|file| ReviewFile {
                path: sanitize_path(&file.path),
                how: how_solved(state.choices.get(&file.path), file.result),
            })
            .collect(),
        combined: u32::try_from(summary.by_git.len()).unwrap_or(u32::MAX),
        message: sanitize_line(&subject_of(&summary.message)),
        state: SummaryState::Ready,
    }
}

/// Where the review draws: the whole frame under the banner line (one row
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

/// The review's key context: its one bar (mock
/// `mergewright-merge-screen.html:161`), whichever row the cursor is on.
#[must_use]
pub fn finish_context(_state: &AppState) -> HintContext {
    HintContext::Review(ReviewBar::List)
}

/// The keys the review offers in an area `width` wide: the list its
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
            &state.keymap,
        )
        .into_iter()
        .filter(|hint| {
            hint.is(&[graft_ui::graph_screen::HintKey::Key(
                graft_ui::keymap::ActionId("app.back"),
            )])
        })
        .collect()
    }
}

/// The app's bottom row under the review: `committing…` while the commit
/// runs, else nothing. The keys live in the review's own frame
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
