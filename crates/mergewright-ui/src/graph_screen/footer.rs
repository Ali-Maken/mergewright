//! The footer: one line that is a problem, a search line, a notice, the
//! pinned compare base, a revealed subject, or — when none of those apply —
//! the key hints (`footer_line`). The commit-graph offer is not one of those
//! alternatives: it is drawn by `commit_graph_line` on its own row above.

use mergewright_graph::fold::Fold;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::hints::{Divider, hints_for, hints_text, stopped_row, stopped_row_before_keys};
use super::lanes::graph_width;
use super::layout::ScreenRows;
use super::row_columns::Columns;
use super::row_text::subject_truncated;
use super::{CommitGraphLine, GraphEntry, GraphScreen, HintContext, SearchLine};
use crate::fit::fit;
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::Token;

impl GraphScreen<'_> {
    /// The commit-graph line, styled: info text, key letters in the accent.
    /// Cut to one info-coloured run when even the breakpoint's wording does
    /// not fit.
    pub(super) fn commit_graph_line(&self, line: CommitGraphLine, width: usize) -> Line<'static> {
        let segments = commit_graph_segments(line, width, self.view.keys);
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

    /// The bottom rows: the bar's, and — while the carve gave it one — the
    /// done line's own row directly above it (ease-pass/08s M4). The notice
    /// is then off the bar's row, so the bar shows the landing's keys.
    pub(super) fn bottom_rows(
        &self,
        chrome: &ScreenRows,
        drawn: (Rect, &[GraphEntry<'_>], &Fold),
        width: usize,
        screen: Rect,
        buf: &mut Buffer,
    ) {
        let x = screen.x;
        let (body, rows, fold) = drawn;
        let above = chrome.status.height > 0;
        if let Some(notice) = self.view.notice.as_ref().filter(|_| above) {
            let line = Line::styled(
                fit(&format!(" {notice}"), width),
                foreground(self.theme, Token::Info),
            );
            buf.set_line(x, chrome.status.y, &line, chrome.status.width);
        }
        if chrome.footer.height > 0
            && let Some(problem) = &self.view.problem
        {
            // A problem wins the row; the mark is the renderer's, in its own
            // ink, and the words are cut before it (`failure::status_line`).
            let line = crate::failure::status_line(
                problem,
                self.view.ascii,
                u16::try_from(width).unwrap_or(u16::MAX),
                self.theme,
                foreground(self.theme, Token::Warning),
            );
            buf.set_line(x, chrome.footer.y, &line, chrome.footer.width);
        } else if chrome.footer.height > 0 {
            let (text, token) = self.footer_line((body, screen.width), rows, fold, width, above);
            let footer = Line::styled(fit(&text, width), foreground(self.theme, token));
            buf.set_line(x, chrome.footer.y, &footer, chrome.footer.width);
        }
    }

    /// The footer's text and token: a problem, a search line, a notice (not
    /// when `notice_above` drew it on its own row), the pinned compare
    /// base's line, the reveal (a truncated subject, named by hover), or —
    /// when none of those apply — the context's key hints
    /// (`hints::hints_for`, `hint-bar-redesign`).
    fn footer_line(
        &self,
        (body, screen): (Rect, u16),
        rows: &[GraphEntry<'_>],
        fold: &Fold,
        width: usize,
        notice_above: bool,
    ) -> (String, Token) {
        // Same columns the body just drew through: the reveal must agree
        // with the body on which subjects the `…` actually landed on.
        let columns = Columns::compute(body, graph_width(body, fold), screen);
        let reveal_subject = self
            .view
            .reveal
            .and_then(|index| rows.get(index))
            .filter(|entry| {
                subject_truncated(
                    entry,
                    (self.view.icons, self.view.keys),
                    self.view.ascii,
                    &columns,
                )
            })
            .map(|entry| entry.subject.as_str().to_owned());

        let notice = self.view.notice.as_ref().filter(|_| !notice_above);
        match (&self.view.search, notice, &self.view.compare_footer) {
            (Some(SearchLine::Prompt(typed)), _, _) => (format!(" /{typed}_"), Token::Info),
            (None, Some(notice), _) => (format!(" {notice}"), Token::Info),
            (None, None, Some(compare)) => (format!(" {compare}"), Token::Accent),
            (None, None, None) => {
                if let Some(subject) = &reveal_subject {
                    (format!(" {subject}"), Token::Text)
                } else {
                    self.key_line(width)
                }
            }
        }
    }

    /// The context's key hints, led by a stop's reason when one stands.
    fn key_line(&self, width: usize) -> (String, Token) {
        let width16 = u16::try_from(width).unwrap_or(u16::MAX);
        // The divider exists while any detail pane is open in a *split* — a
        // commit's as much as the changes pane's (ADR-0028), which is why the
        // flag is read here and not carried inside one context (m5-03
        // review, Major 4). The tab layout has no divider, and the flag says
        // so (m5/33).
        let divider = if self.view.divider_shown {
            Divider::Shown
        } else {
            Divider::Hidden
        };
        let hints = hints_for(&self.view.hint_context, divider, width16, self.view.keys);
        match &self.view.stopped {
            // A stop is a state, not a problem: it leads the bar rather than
            // replacing it, and never names `esc dismiss` (m6-conflicts/12).
            // A stash clash has no banner: its reason outranks the bar's
            // tail keys (review 72+72b M1).
            Some(reason) if reason.is_before_keys() => (
                stopped_row_before_keys(
                    reason,
                    &self.view.hint_context,
                    divider,
                    width16,
                    self.view.keys,
                ),
                Token::Text,
            ),
            // Only the focused banner's short bar gives keys for the count;
            // a laddered row bar (after `Esc`) stays as accepted (ease-28 Q3).
            Some(reason) => {
                let banner = matches!(self.view.hint_context, HintContext::Banner { .. });
                let row = if banner {
                    stopped_row(reason, &hints, width16, self.view.keys)
                } else {
                    stopped_row(
                        &reason.clone().without_keys_yield(),
                        &hints,
                        width16,
                        self.view.keys,
                    )
                };
                (row, Token::Text)
            }
            None => (hints_text(&hints, self.view.keys), Token::TextMuted),
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
pub(super) fn commit_graph_segments(
    line: CommitGraphLine,
    width: usize,
    keys: &Keymap,
) -> Vec<(String, bool)> {
    let mut wordings = commit_graph_wordings(line, keys);
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

/// `<lead>` then `B build … · X never here`, each clause only while its key
/// is bound (`graph.write_commit_graph`, `graph.decline_commit_graph`); the
/// lead alone, separator dropped, when neither is.
fn offer_segments(lead: &str, build: &str, keys: &Keymap) -> Vec<(String, bool)> {
    let text = |words: &str| (words.to_owned(), false);
    let clauses: Vec<(String, &str)> = [
        ("graph.write_commit_graph", build),
        ("graph.decline_commit_graph", " never here"),
    ]
    .into_iter()
    .filter_map(|(id, words)| keys.spell_keys(&[ActionId(id)]).map(|key| (key, words)))
    .collect();
    if clauses.is_empty() {
        return vec![text(lead)];
    }
    let mut segments = vec![text(&format!("{lead} · "))];
    for (at, (key, words)) in clauses.into_iter().enumerate() {
        if at > 0 {
            segments.push(text(" · "));
        }
        segments.push((key, true));
        segments.push(text(words));
    }
    segments
}

/// Every wording the line has, in no particular order:
/// [`commit_graph_segments`] picks between them by what fits. Keys are
/// spelled from `keys` (ticket m8/10); an unbound one's clause is left out.
fn commit_graph_wordings(line: CommitGraphLine, keys: &Keymap) -> Vec<Vec<(String, bool)>> {
    let text = |words: &str| (words.to_owned(), false);
    let spell = |id: &'static str| keys.spell_keys(&[ActionId(id)]);
    let offer = |lead: &str, build: &str| offer_segments(lead, build, keys);
    match line {
        // Off the graph `B` is blame (keymap §2.5): the fact stays, the
        // keys go (m7-power-tools/36, M3).
        CommitGraphLine::Offered { keys: false } => vec![
            vec![text(" no commit-graph file: history sorts on every open")],
            vec![text(" no commit-graph file")],
        ],
        // The spec's " never for this repo" made the widest line 125
        // columns; "never here" keeps it inside the 120 it is drawn at.
        CommitGraphLine::Offered { keys: true } => vec![
            offer(
                " no commit-graph file: history sorts on every open",
                " build one (git commit-graph write --reachable)",
            ),
            offer(
                " no commit-graph file",
                " build one (git commit-graph write --reachable)",
            ),
            offer(" no commit-graph file", " build one"),
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
            vec![text(&match spell("app.refresh") {
                Some(refresh) => format!(
                    " commit-graph written: {} commits · the next open sorts at once · {refresh} \
                     to use it",
                    grouped(commits)
                ),
                None => format!(
                    " commit-graph written: {} commits · the next open sorts at once",
                    grouped(commits)
                ),
            })],
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
mod tests;
