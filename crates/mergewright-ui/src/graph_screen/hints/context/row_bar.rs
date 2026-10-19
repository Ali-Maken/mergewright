//! [`RowBar`]: which graph row the cursor is on, with the names and counts
//! its words carry (ticket ease-pass/03b).

use mergewright_term::SanitizedText;

use super::HintContext;

/// Which graph row the cursor is on, for [`HintContext::GraphRow`]: each
/// kind says its own next steps (design note ease-pass, G13 and M1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowBar {
    /// His own branch's row: `⏎ open · P push · c commit`.
    Own,
    /// A teammate's or the trunk's row: `m merge <b> into yours · ⏎ open ·
    /// C switch to <b> · x compare`, `<b>` its [`bar_name`].
    Elsewhere {
        /// The branch the row's badge names, whole; the bar draws its last
        /// path segment.
        branch: SanitizedText,
    },
    /// A folded merge: `space show the N commits inside · ⏎ open · x compare`.
    FoldedMerge {
        /// How many commits the fold hides.
        inside: u32,
    },
    /// The merge row whose fold `space` opened (mock
    /// `mergewright-main-screen.html:206`): `space hide the commits inside ·
    /// ⏎ open · x compare`. Rows inside an opened merge take their own
    /// branch's bar ([`RowBar::Own`] / [`RowBar::Elsewhere`]).
    OpenedMerge,
    /// Just after a commit (C13): `u undo the commit, your edits come back ·
    /// P push · c commit again`.
    JustCommitted,
    /// Just after a merge (M18): `u undo the merge · P push · space show
    /// the N commits inside`.
    JustMerged {
        /// How many commits the merge brought in.
        inside: u32,
    },
}

/// A branch's or a path's bar form: its last path segment
/// (`sara/fix-toc` → `fix-toc`, Q2 a; `docs/notes.md` → `notes.md`).
/// Already sanitized.
#[must_use]
pub fn bar_name(full: &SanitizedText) -> &str {
    let name = full.as_str();
    name.rsplit('/').next().unwrap_or(name)
}

impl HintContext {
    /// An ease-pass row bar: [`HintContext::GraphRow`], bare or under
    /// [`HintContext::Rewritable`] (ease-pass/07b). Both draw the mock's
    /// bar — its own words, `u undo`, the doorway — and no `R rewrite`.
    #[must_use]
    pub(crate) fn is_graph_row_bar(&self) -> bool {
        match self {
            HintContext::GraphRow { .. } => true,
            HintContext::Rewritable(inner) => matches!(**inner, HintContext::GraphRow { .. }),
            _ => false,
        }
    }
}
