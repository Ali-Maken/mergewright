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
use crate::graph_column::RowPaint;
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
    /// hover), or — when none of those apply — the ordinary key hints.
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

        match (
            &self.view.problem,
            &self.view.search,
            &self.view.notice,
            &self.view.compare_footer,
        ) {
            (Some(problem), _, _, _) => (format!(" {problem}"), Token::Warning),
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
                    let mut extra = Vec::new();
                    extra.extend(expand_hint(fold, self.view.compaction.expanded));
                    // Only while the pane is on screen: no divider to drag
                    // otherwise (ADR-0028).
                    extra.extend(self.view.detail_shown.then_some("<> resize"));
                    // The keys that write, only on the pane that can write
                    // (m3-working-tree-undo-and-stash/04). They sit ahead of
                    // the movement keys so a narrow terminal keeps them.
                    if self.view.changes_pane {
                        extra.push("space stage");
                        extra.push("d discard");
                    }
                    // The cursor row's own hint, only when it is marked
                    // (`m2-details-and-diffs/17`, design note "Marker
                    // column").
                    extra.extend(cursor_marked(rows).then_some(". explain"));
                    (key_hints(&extra, width), Token::TextMuted)
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

/// Every key the graph screen answers, in the order the footer drops them
/// (PROMPT.md principle 4: the hint bar teaches the tool). Leaving, the
/// status screen and the renderer first, then `+` while it applies
/// ([`LEADING_HINTS`] of these come before it), then search and movement
/// and movement, then the rest of the actions: a narrow terminal drops whole
/// hints from the right, and the hint that comes and goes matters more than
/// the movement keys that are always there. `n`/`N` live on the search line,
/// where they work. `Enter` opens the detail pane (ADR-0023); its hint comes
/// with the pane (`detail-pane-frame`), not before it draws.
pub(super) const HINTS: [&str; 15] = [
    "q quit",
    "F2 status",
    "F3 renderer",
    "u undo",
    "/ search",
    "j/k move",
    "Space fold",
    "1 first-parent",
    "F focus",
    "H stash",
    "J journal",
    "y sha",
    "PgUp/PgDn page",
    "g/G top/bottom",
    "Ctrl+R refresh",
];
/// How many of [`HINTS`] come before the lane-expansion hint. `u undo` is
/// among them: the milestone's promise is that work cannot be lost, and the
/// key that keeps it must survive a narrow terminal dropping hints from the
/// right (m3-working-tree-undo-and-stash/06).
const LEADING_HINTS: usize = 4;

/// Whether the cursor row (the one painted [`RowPaint::Highlighted`]) has a
/// Mistake Radar marker — the `. explain` hint's own condition.
fn cursor_marked(rows: &[GraphEntry<'_>]) -> bool {
    rows.iter()
        .find(|entry| entry.line.paint == RowPaint::Highlighted)
        .is_some_and(|entry| entry.marker.is_some())
}

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
/// fit, dropped from the right, never one cut in half. `extra` (lane-fold,
/// pane-resize) is spliced in right after the leading hints, in the order
/// given.
pub(super) fn key_hints(extra: &[&str], width: usize) -> String {
    let (leading, rest) = HINTS.split_at(LEADING_HINTS);
    let mut line = String::new();
    for hint in leading
        .iter()
        .copied()
        .chain(extra.iter().copied())
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
