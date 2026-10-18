//! Where the graph's status line stands (ease-pass/08s M4, mock
//! `mergewright-commit-screen.html:178-179`): in the bar's row, as every
//! status always stood, or on its own row directly above the bar — only a
//! done line over a landing bar, so the words that say what `u` gives back
//! show at the moment they matter instead of after the next key.

use super::{HintContext, RowBar};

impl RowBar {
    /// `JustCommitted` | `JustMerged { .. }`: the done line stands above
    /// this bar rather than in its place.
    #[must_use]
    pub fn is_landing(&self) -> bool {
        matches!(self, Self::JustCommitted | Self::JustMerged { .. })
    }
}

/// The row a status line takes on the graph screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum StatusRow {
    /// In the bar's row, the bar hidden while it stands (every status but
    /// one).
    #[default]
    InBar,
    /// Its own row directly above the bar; the graph gives up that row.
    AboveBar,
}

impl StatusRow {
    /// Rows the line takes from the graph: 1 above the bar, 0 in it.
    #[must_use]
    pub fn rows_taken(self) -> u16 {
        u16::from(self == Self::AboveBar)
    }
}

/// [`StatusRow::AboveBar`] for a done line (`done`) while the bar is a
/// landing bar's ([`RowBar::is_landing`]); [`StatusRow::InBar`] otherwise.
///
/// Looked for through the wrappers that keep the cursor row's own bar and
/// only lead or filter it — a radar marker, a search, `R`, a focused commit
/// pane, a live operation, a place's `esc back` — so the landing words are
/// still on the bar under the done line.
#[must_use]
pub fn status_row(done: bool, context: &HintContext) -> StatusRow {
    if done && landing_bar(context) {
        StatusRow::AboveBar
    } else {
        StatusRow::InBar
    }
}

/// The cursor row's own bar, under any wrapper that keeps it, is a landing.
fn landing_bar(context: &HintContext) -> bool {
    match context {
        HintContext::GraphRow { row, .. } => row.is_landing(),
        HintContext::Marked(inner)
        | HintContext::Searching(inner)
        | HintContext::Rewritable(inner)
        | HintContext::BackToBlame(inner)
        | HintContext::BackToHistory(inner)
        | HintContext::PaneFocused { row: inner, .. }
        | HintContext::OperationLive { row: inner, .. } => landing_bar(inner),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_screen::hints::RowVerbs;
    use mergewright_term::sanitize_line;

    fn row(row: RowBar) -> HintContext {
        HintContext::GraphRow {
            row,
            verbs: RowVerbs::NONE,
        }
    }

    /// Only a landing bar takes the done line above it; every other row's
    /// bar, and every status that is not a done line, keeps today's row.
    #[test]
    fn status_row_is_above_only_for_a_landing_bar() {
        let elsewhere = RowBar::Elsewhere {
            branch: sanitize_line("sara/fix-toc"),
        };
        let cases = [
            (true, row(RowBar::JustCommitted), StatusRow::AboveBar),
            (
                true,
                row(RowBar::JustMerged { inside: 3 }),
                StatusRow::AboveBar,
            ),
            (false, row(RowBar::JustCommitted), StatusRow::InBar),
            (true, row(RowBar::Own), StatusRow::InBar),
            (true, row(elsewhere), StatusRow::InBar),
            (
                true,
                row(RowBar::FoldedMerge { inside: 2 }),
                StatusRow::InBar,
            ),
            (true, row(RowBar::OpenedMerge), StatusRow::InBar),
            (
                true,
                HintContext::PlainRow {
                    verbs: RowVerbs::NONE,
                },
                StatusRow::InBar,
            ),
            (true, HintContext::Preview, StatusRow::InBar),
            (true, HintContext::WorkingTreeRow, StatusRow::InBar),
            // The wrappers that keep the row's own bar (a radar marker, a
            // search, `R`, a focused commit pane, a live operation, a
            // place's `esc back`) keep its landing too.
            (
                true,
                HintContext::Marked(Box::new(row(RowBar::JustCommitted))),
                StatusRow::AboveBar,
            ),
            (
                true,
                HintContext::Searching(Box::new(HintContext::Rewritable(Box::new(row(
                    RowBar::JustCommitted,
                ))))),
                StatusRow::AboveBar,
            ),
            (
                true,
                HintContext::PaneFocused {
                    row: Box::new(row(RowBar::JustCommitted)),
                    opens: true,
                    blamable: false,
                    folds: false,
                    graph_hidden: false,
                },
                StatusRow::AboveBar,
            ),
            (
                true,
                HintContext::BackToHistory(Box::new(row(RowBar::Own))),
                StatusRow::InBar,
            ),
        ];
        for (done, context, want) in cases {
            assert_eq!(status_row(done, &context), want, "{done} {context:?}");
        }
    }
}
