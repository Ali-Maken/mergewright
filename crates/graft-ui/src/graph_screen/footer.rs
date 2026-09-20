//! The footer: a problem, a search line, a notice, a revealed subject, the
//! commit-graph offer, or — when none of those apply — the key hints; and
//! the commit-graph offer's own line above it.

use graft_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::hints::{Divider, fit_status_line, hints_for, hints_text};
use super::layout::{Columns, graph_width};
use super::render::subject_truncated;
use super::{CommitGraphLine, GraphEntry, GraphScreen, SearchLine};
use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::Token;

impl GraphScreen<'_> {
    /// The commit-graph line, styled: info text, key letters in the accent.
    /// Cut to one info-coloured run when even the breakpoint's wording does
    /// not fit.
    pub(super) fn commit_graph_line(&self, line: CommitGraphLine, width: usize) -> Line<'static> {
        let segments = commit_graph_segments(line, width);
        let whole: String = segments.iter().map(|(text, _)| text.as_str()).collect();
        if whole.width() > width {
            return Line::styled(fit(&whole, width), foreground(self.theme, Token::Info));
        }
        Line::from(
            segments
                .into_iter()
                .map(|(text, key)| {
                    let token = if key { Token::Accent } else { Token::Info };
                    Span::styled(text, foreground(self.theme, token))
                })
                .collect::<Vec<_>>(),
        )
    }

    /// The footer's text and token: a problem, a search line, a notice, the
    /// pinned compare base's line, the reveal (a truncated subject, named by
    /// hover), or — when none of those apply — the context's key hints
    /// (`hints::hints_for`, `hint-bar-redesign`).
    pub(super) fn footer_line(
        &self,
        body: Rect,
        rows: &[GraphEntry<'_>],
        applied: Option<&Fold>,
        width: usize,
    ) -> (String, Token) {
        // Same columns the body just drew through: the reveal must agree
        // with the body on which subjects the `…` actually landed on.
        let columns = Columns::compute(body, graph_width(body, rows, applied));
        let reveal_subject = self
            .view
            .reveal
            .and_then(|index| rows.get(index))
            .filter(|entry| subject_truncated(entry, self.view.icons, &columns))
            .map(|entry| entry.subject.as_str().to_owned());

        match (
            &self.view.problem,
            &self.view.search,
            &self.view.notice,
            &self.view.compare_footer,
        ) {
            (Some(problem), _, _, _) => {
                let width16 = u16::try_from(width.saturating_sub(1)).unwrap_or(u16::MAX);
                let line = fit_status_line(
                    problem.reason.as_str(),
                    problem.keys.as_ref().map(graft_term::SanitizedText::as_str),
                    width16,
                );
                (format!(" {line}"), Token::Warning)
            }
            (None, Some(SearchLine::Prompt(typed)), _, _) => (format!(" /{typed}_"), Token::Info),
            (None, Some(SearchLine::Status { text, failed }), _, _) => (
                format!(" {text}"),
                if *failed { Token::Warning } else { Token::Info },
            ),
            (None, None, Some(notice), _) => (format!(" {notice}"), Token::Info),
            (None, None, None, Some(compare)) => (format!(" {compare}"), Token::Accent),
            (None, None, None, None) => {
                if let Some(subject) = &reveal_subject {
                    (format!(" {subject}"), Token::Text)
                } else {
                    let width16 = u16::try_from(width).unwrap_or(u16::MAX);
                    // The divider exists while any detail pane is open — a
                    // commit's as much as the changes pane's (ADR-0028), which
                    // is why the flag is read here and not carried inside one
                    // context (m5-03 review, Major 4).
                    let divider = if self.view.detail_shown {
                        Divider::Shown
                    } else {
                        Divider::Hidden
                    };
                    let hints = hints_for(&self.view.hint_context, divider, width16);
                    (hints_text(&hints), Token::TextMuted)
                }
            }
        }
    }
}

/// Wide enough for the commit-graph line's longest wording.
const COMMIT_GRAPH_WIDE: usize = 120;
/// Wide enough for its middle wording; below it, the shortest.
const COMMIT_GRAPH_MEDIUM: usize = 100;

/// The commit-graph line's text at `width`, as segments: `true` marks a key
/// letter, drawn in the accent.
pub(super) fn commit_graph_segments(line: CommitGraphLine, width: usize) -> Vec<(String, bool)> {
    let text = |words: &str| (words.to_owned(), false);
    let key = |letter: &str| (letter.to_owned(), true);
    let wide = width >= COMMIT_GRAPH_WIDE;
    let narrow = width < COMMIT_GRAPH_MEDIUM;
    match line {
        CommitGraphLine::Offered if wide => vec![
            text(" no commit-graph file: history sorts on every open · "),
            key("B"),
            // The spec's " never for this repo" made this line 125 columns;
            // "never here" keeps it inside the 120 it is drawn at.
            text(" build one (git commit-graph write --reachable) · "),
            key("X"),
            text(" never here"),
        ],
        CommitGraphLine::Offered if narrow => vec![
            text(" no commit-graph file · "),
            key("B"),
            text(" build one · "),
            key("X"),
            text(" never here"),
        ],
        CommitGraphLine::Offered => vec![
            text(" no commit-graph file · "),
            key("B"),
            text(" build one (git commit-graph write --reachable) · "),
            key("X"),
            text(" never here"),
        ],
        CommitGraphLine::NoGitToBuild if narrow => {
            vec![text(" no commit-graph file · no git to build one")]
        }
        CommitGraphLine::NoGitToBuild => vec![text(
            " no commit-graph file, and no git ≥ 2.38 on PATH to build one",
        )],
        CommitGraphLine::Writing if narrow => vec![text(" building the commit-graph…")],
        CommitGraphLine::Writing => vec![text(
            " building the commit-graph… (git commit-graph write --reachable)",
        )],
        CommitGraphLine::Written { .. } if narrow => {
            vec![text(" commit-graph written · next open sorts at once")]
        }
        CommitGraphLine::Written { commits } => vec![text(&format!(
            " commit-graph written: {} commits · the next open sorts at once · Ctrl+R to use it",
            grouped(commits)
        ))],
    }
}

/// `1000000` as `1,000,000`.
pub(super) fn grouped(number: u32) -> String {
    let digits = number.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
