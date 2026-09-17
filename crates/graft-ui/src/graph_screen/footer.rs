//! The footer: a problem, a search line, a notice, a revealed subject, the
//! commit-graph offer, or — when none of those apply — the key hints; and
//! the commit-graph offer's own line above it.

use graft_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::layout::{Columns, graph_width, text_width};
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
    /// reveal (a truncated subject, named by hover), or — when
    /// none of those apply — the ordinary key hints.
    pub(super) fn footer_line(
        &self,
        body: Rect,
        rows: &[GraphEntry<'_>],
        applied: Option<&Fold>,
        fold: &Fold,
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

        match (&self.view.problem, &self.view.search, &self.view.notice) {
            (Some(problem), _, _) => (format!(" {problem}"), Token::Warning),
            (None, Some(SearchLine::Prompt(typed)), _) => (format!(" /{typed}_"), Token::Info),
            (None, Some(SearchLine::Status { text, failed }), _) => (
                format!(" {text}"),
                if *failed { Token::Warning } else { Token::Info },
            ),
            (None, None, Some(notice)) => (format!(" {notice}"), Token::Info),
            (None, None, None) => match &reveal_subject {
                Some(subject) => (format!(" {subject}"), Token::Text),
                None => (
                    key_hints(expand_hint(fold, self.view.compaction.expanded), width),
                    Token::TextMuted,
                ),
            },
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
            " commit-graph written: {} commits · the next open sorts at once",
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

/// Every key the graph screen answers, in the order the footer drops them
/// (PROMPT.md principle 4: the hint bar teaches the tool). Leaving, the
/// status screen and the renderer first, then `+` while it applies
/// ([`LEADING_HINTS`] of these come before it), then search and movement
/// and movement, then the rest of the actions: a narrow terminal drops whole
/// hints from the right, and the hint that comes and goes matters more than
/// the movement keys that are always there. `n`/`N` live on the search line,
/// where they work. `Enter` opens the detail pane (ADR-0023); its hint comes
/// with the pane (`detail-pane-frame`), not before it draws.
pub(super) const HINTS: [&str; 13] = [
    "q quit",
    "F2 status",
    "F3 renderer",
    "/ search",
    "j/k move",
    "Space fold",
    "1 first-parent",
    "F focus",
    "H stash",
    "y sha",
    "PgUp/PgDn page",
    "g/G top/bottom",
    "Ctrl+R refresh",
];
/// How many of [`HINTS`] come before the lane-expansion hint.
const LEADING_HINTS: usize = 3;

/// The hint for the lane-expansion key, shown only while there is something
/// to expand or collapse.
fn expand_hint(fold: &Fold, expanded: bool) -> Option<&'static str> {
    match (fold.marker().is_some(), expanded) {
        (false, false) => None,
        (_, true) => Some("+ collapse lanes"),
        (true, false) => Some("+ expand lanes"),
    }
}

/// The footer's key hints within `width` columns: as many whole hints as
/// fit, dropped from the right, never one cut in half.
pub(super) fn key_hints(expand: Option<&str>, width: usize) -> String {
    let (leading, rest) = HINTS.split_at(LEADING_HINTS);
    let mut line = String::new();
    for hint in leading
        .iter()
        .copied()
        .chain(expand)
        .chain(rest.iter().copied())
    {
        let separator = if line.is_empty() { " " } else { "  " };
        if !line.is_empty() && text_width(&line) + text_width(separator) + text_width(hint) > width
        {
            break;
        }
        line.push_str(separator);
        line.push_str(hint);
    }
    line
}
