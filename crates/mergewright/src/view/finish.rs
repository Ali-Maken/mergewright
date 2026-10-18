//! Turning [`AppState`]'s merge review into the view model
//! [`mergewright_ui::finish_summary::FinishScreen`] draws (ease-pass/12;
//! m6-conflicts/19 before it).
//!
//! The keys and the mouse build the same view to keep the cursor on screen
//! and to hit-test a click, so a click and the draw agree on every row.

use mergewright_git::diff::Origin as GitOrigin;
use mergewright_term::{sanitize_line, sanitize_path};
use mergewright_ui::finish_summary::{ReviewFile, ReviewView, SummaryState};
use mergewright_ui::graph_screen::HintContext;
use mergewright_ui::graph_screen::hints::ReviewBar;
use mergewright_ui::merge_look::{CombinedFile, LookBody, LookLine, LookView, Origin};
use ratatui::layout::Rect;

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::finish::{FinishState, LookRead, how_solved, subject_of};

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
    from: mergewright_term::SanitizedText,
    into: mergewright_term::SanitizedText,
    clashed: Vec<ReviewFile>,
    combined: u32,
    message: mergewright_term::SanitizedText,
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

/// The review's key context: the list's bar (mock
/// `mergewright-merge-screen.html:162`), whichever row the cursor is on, or
/// the look's while it is open (`:177`, ease-pass/12b).
#[must_use]
pub fn finish_context(state: &AppState) -> HintContext {
    let looking = state
        .finish
        .as_ref()
        .is_some_and(|finish| finish.look.is_some());
    HintContext::Review(if looking {
        ReviewBar::Look
    } else {
        ReviewBar::List
    })
}

/// The keys the review offers in an area `width` wide: the list its
/// bottom border draws (`finish_summary::footer_hints`), or only `esc back`
/// below the widget's floor, where `Esc` is the only key that acts
/// (`finish::finish_key`) and the widget's own line says so. What the key
/// sweep presses.
#[cfg(test)]
#[must_use]
pub fn finish_hints(state: &AppState, width: u16) -> Vec<mergewright_ui::graph_screen::Hint> {
    // Below the review's floor the look is not drawn either: only `esc`.
    if crate::update::finish::fits(state) && look_view(state).is_some() {
        return mergewright_ui::merge_look::look_hints(width, &state.keymap);
    }
    if crate::update::finish::fits(state) {
        finish_view(state).map_or_else(Vec::new, |view| {
            mergewright_ui::finish_summary::footer_hints(&view, width)
        })
    } else {
        mergewright_ui::graph_screen::hints_for(
            &finish_context(state),
            mergewright_ui::graph_screen::Divider::Hidden,
            width,
            &state.keymap,
        )
        .into_iter()
        .filter(|hint| {
            hint.is(&[mergewright_ui::graph_screen::HintKey::Key(
                mergewright_ui::keymap::ActionId("app.back"),
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

/// The open look as it draws now (ease-pass/12b), or `None` when no look is
/// open.
#[must_use]
pub fn look_view(state: &AppState) -> Option<LookView> {
    let look = state.finish.as_ref()?.look.as_ref()?;
    let body = match &look.read {
        Loading::Idle | Loading::Pending(_) => LookBody::Reading,
        Loading::Failed(why) => LookBody::Failed(sanitize_line(why)),
        Loading::Ready(read) => match read.as_ref() {
            LookRead::Clashed(text) => LookBody::Clashed {
                lines: text
                    .lines()
                    .zip(1u32..)
                    .map(|(line, n)| LookLine::Plain {
                        n,
                        text: sanitize_line(line),
                    })
                    .collect(),
            },
            LookRead::Combined { files, sample } => LookBody::Combined {
                files: files
                    .iter()
                    .map(|file| CombinedFile {
                        path: sanitize_path(&file.path),
                        yours: file.yours,
                        theirs: file.theirs,
                    })
                    .collect(),
                sample: sample.as_ref().map(|sample| {
                    (
                        sanitize_path(&sample.path),
                        sample.lines.iter().map(sample_line).collect(),
                    )
                }),
            },
        },
    };
    Some(LookView {
        path: look
            .path
            .as_deref()
            .map_or_else(|| sanitize_line(""), sanitize_path),
        body,
        top: 0,
    })
}

/// One sample line, its side mapped to the widget's own.
fn sample_line((n, text, origin): &(u32, String, Option<GitOrigin>)) -> LookLine {
    let text = sanitize_line(text);
    match origin {
        None => LookLine::Plain { n: *n, text },
        Some(origin) => LookLine::From {
            n: *n,
            text,
            origin: match origin {
                GitOrigin::Yours => Origin::Yours,
                GitOrigin::Theirs => Origin::Theirs,
                GitOrigin::Both => Origin::Both,
            },
        },
    }
}
