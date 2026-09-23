//! The footer: one line that is a problem, a search line, a notice, the
//! pinned compare base, a revealed subject, or — when none of those apply —
//! the key hints (`footer_line`). The commit-graph offer is not one of those
//! alternatives: it is drawn by `commit_graph_line` on its own row above.

use graft_graph::fold::Fold;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::hints::{Divider, fit_status_line, hints_for, hints_text, stopped_row};
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
                    // The divider exists while any detail pane is open in
                    // a *split* — a commit's as much as the changes pane's
                    // (ADR-0028), which is why the flag is read here and not
                    // carried inside one context (m5-03 review, Major 4).
                    // The tab layout has no divider, and the flag says so
                    // (m5/33).
                    let divider = if self.view.divider_shown {
                        Divider::Shown
                    } else {
                        Divider::Hidden
                    };
                    let hints = hints_for(&self.view.hint_context, divider, width16);
                    match &self.view.stopped {
                        // A stop is a state, not a problem: it leads the
                        // bar rather than replacing it, and never names
                        // `esc dismiss` (m6-conflicts/12).
                        Some(reason) => (stopped_row(reason, &hints, width16), Token::Text),
                        None => (hints_text(&hints), Token::TextMuted),
                    }
                }
            }
        }
    }
}

/// Cells a wording takes.
pub(super) fn segments_width(segments: &[(String, bool)]) -> usize {
    segments.iter().map(|(text, _)| text.width()).sum()
}

/// The commit-graph line's text at `width`, as segments: `true` marks a key
/// letter, drawn in the accent.
///
/// The wording drawn is the **widest one that fits `width`**, or the
/// narrowest there is when none does — keyed on what fits, not on a width
/// tier.
///
/// The tiers this replaces (120 / 100 / under) were calibrated against the
/// *screen's* width, but this line is drawn inside the graph's area, and the
/// graph gives up 42% of the screen the moment the detail pane opens beside
/// it (`DetailLayout::Right`). So a 160-column terminal handed this function
/// 92 columns, fell under the 100-column tier and drew the 47-cell wording
/// where the 87-cell one had five cells to spare: forty more columns of
/// terminal, less text than at 120 (m5/19, found by the user in
/// `docs/shots/m5-ui-banner-conflict-160x48.txt`). Choosing the widest that
/// fits cannot invert like that, whatever order the wordings are written in
/// and whatever width the caller has — pinned by
/// `no_footer_line_gets_terser_as_the_room_for_it_grows`.
pub(super) fn commit_graph_segments(line: CommitGraphLine, width: usize) -> Vec<(String, bool)> {
    let mut wordings = commit_graph_wordings(line);
    // Nothing fits: draw the narrowest anyway and let `commit_graph_line`
    // cut it. Cutting the shortest wording loses the least.
    // `unwrap_or_default()` here is index 0, not a swallowed failure: the
    // `None` arm is "`wordings` is empty", and `commit_graph_wordings`
    // returns a non-empty `vec![..]` for every arm of its match. Index 0 is
    // then the only index the `swap_remove` below could take in any case, so
    // the default is the answer rather than a stand-in for one (CLAUDE.md
    // rule 3; M5 review Minor 4 — the line reads like a swallow and is not).
    let narrowest = wordings
        .iter()
        .enumerate()
        .min_by_key(|(_, wording)| segments_width(wording))
        .map(|(index, _)| index)
        .unwrap_or_default();
    let index = wordings
        .iter()
        .enumerate()
        .filter(|(_, wording)| segments_width(wording) <= width)
        .max_by_key(|(_, wording)| segments_width(wording))
        .map_or(narrowest, |(index, _)| index);
    wordings.swap_remove(index)
}

/// Every wording the line has, in no particular order:
/// [`commit_graph_segments`] picks between them by what fits.
fn commit_graph_wordings(line: CommitGraphLine) -> Vec<Vec<(String, bool)>> {
    let text = |words: &str| (words.to_owned(), false);
    let key = |letter: &str| (letter.to_owned(), true);
    match line {
        CommitGraphLine::Offered => vec![
            vec![
                text(" no commit-graph file: history sorts on every open · "),
                key("B"),
                // The spec's " never for this repo" made this line 125
                // columns; "never here" keeps it inside the 120 it is
                // drawn at.
                text(" build one (git commit-graph write --reachable) · "),
                key("X"),
                text(" never here"),
            ],
            vec![
                text(" no commit-graph file · "),
                key("B"),
                text(" build one (git commit-graph write --reachable) · "),
                key("X"),
                text(" never here"),
            ],
            vec![
                text(" no commit-graph file · "),
                key("B"),
                text(" build one · "),
                key("X"),
                text(" never here"),
            ],
        ],
        CommitGraphLine::NoGitToBuild => vec![
            vec![text(
                " no commit-graph file, and no git ≥ 2.38 on PATH to build one",
            )],
            vec![text(" no commit-graph file · no git to build one")],
        ],
        CommitGraphLine::Writing => vec![
            vec![text(
                " building the commit-graph… (git commit-graph write --reachable)",
            )],
            vec![text(" building the commit-graph…")],
        ],
        CommitGraphLine::Written { commits } => vec![
            vec![text(&format!(
                " commit-graph written: {} commits · the next open sorts at once · Ctrl+R to use \
                 it",
                grouped(commits)
            ))],
            vec![text(" commit-graph written · next open sorts at once")],
        ],
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

#[cfg(test)]
mod tests {
    //! The stop's rung is the last one (m6-conflicts/12, scope item 3): the
    //! rows above it in `footer_line` still win the whole row.

    use graft_term::sanitize_line;

    use crate::graph_screen::tests::{draw, linear, text, view};
    use crate::graph_screen::{HintContext, ProblemLine, SearchLine, WalkView};

    fn last_row(view: &crate::graph_screen::GraphView<'_>) -> String {
        text(&draw(view, 120, 8)).pop().expect("eight rows")
    }

    #[test]
    fn a_problem_still_wins_over_a_stop() {
        let rows = linear(3);
        let mut shown = view(&rows, WalkView::Complete { rows: 3 }, 0);
        shown.stopped = Some(sanitize_line("merge stopped · 1 file conflicted"));
        // The focused banner's short list, which leaves the reason room.
        shown.hint_context = HintContext::Banner {
            skippable: false,
            conflicted: true,
        };
        assert!(
            last_row(&shown).starts_with(" merge stopped · 1 file conflicted ·"),
            "the control: with nothing above it the stop leads the row: {}",
            last_row(&shown)
        );
        shown.problem = Some(ProblemLine {
            reason: sanitize_line("a hook refused the commit"),
            keys: Some(sanitize_line("a abort")),
        });
        let row = last_row(&shown);
        assert!(
            row.starts_with(" a hook refused the commit · a abort"),
            "{row}"
        );
        assert!(!row.contains("merge stopped"), "{row}");
    }

    #[test]
    fn a_search_line_still_wins_over_a_stop() {
        let rows = linear(3);
        let mut shown = view(&rows, WalkView::Complete { rows: 3 }, 0);
        shown.stopped = Some(sanitize_line("merge stopped · 1 file conflicted"));
        shown.search = Some(SearchLine::Prompt(sanitize_line("fix")));
        let row = last_row(&shown);
        assert!(row.starts_with(" /fix_"), "{row}");
        assert!(!row.contains("merge stopped"), "{row}");
    }
}
