//! The ease pass's bar states for [`every_context`](super::every_context)
//! (ticket ease-pass/03b): each [`RowBar`] once, each [`ClashAt`] for a
//! merge and for a rebase. Sample values `sara/fix-toc` and `3`.
//!
//! No wrapped forms (marked, searching, live, pane-focused): the choice site
//! that picks these (ticket G) adds them with the states it reaches.

use graft_term::sanitize;

use super::super::{ClashAt, ClashBar, ClashOp, HintContext, RowBar, RowVerbs};
use crate::conflict_resolver::SideRoles;

/// Every ease-pass form.
pub(super) fn ease_forms() -> Vec<HintContext> {
    let rows = [
        RowBar::Own,
        RowBar::Elsewhere {
            branch: sanitize("sara/fix-toc"),
        },
        RowBar::FoldedMerge { inside: 3 },
        RowBar::Opened,
        RowBar::JustCommitted,
        RowBar::JustMerged { inside: 3 },
    ];
    let mut forms: Vec<HintContext> = rows
        .into_iter()
        .map(|row| HintContext::GraphRow {
            row,
            verbs: RowVerbs::NONE,
        })
        .collect();
    let ats = [
        ClashAt::Open,
        ClashAt::Picked,
        ClashAt::FileSolved {
            next: sanitize("src/report.rs"),
        },
        ClashAt::AllSolved,
    ];
    for at in ats {
        for op in [ClashOp::Merge, ClashOp::Other(SideRoles::Rebase)] {
            forms.push(HintContext::Clash(ClashBar { at: at.clone(), op }));
        }
    }
    forms
}
