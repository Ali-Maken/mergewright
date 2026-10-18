//! The ease pass's bar states for [`every_context`](super::every_context)
//! (ticket ease-pass/03b): each [`RowBar`] once, each [`ClashAt`] for a
//! merge and for a rebase, and each [`ReviewBar`] (ease-pass/12). Sample values `sara/fix-toc` and `3`.
//!
//! No wrapped forms (marked, searching, live, pane-focused): the choice site
//! that picks these (ticket G) adds them with the states it reaches.

use mergewright_term::sanitize;

use super::super::{ClashAt, ClashBar, HintContext, ReviewBar, RowBar, RowVerbs};
use crate::conflict_resolver::SideRoles;

/// Every bar form.
pub(super) fn bar_forms() -> Vec<HintContext> {
    let rows = [
        RowBar::Own,
        RowBar::Elsewhere {
            branch: sanitize("sara/fix-toc"),
        },
        RowBar::FoldedMerge { inside: 3 },
        RowBar::OpenedMerge,
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
        for roles in [SideRoles::Merge, SideRoles::Rebase] {
            forms.push(HintContext::Clash(ClashBar {
                at: at.clone(),
                roles,
            }));
        }
    }
    // The merge review's list (ease-pass/12) and its look (12b).
    forms.push(HintContext::Review(ReviewBar::List));
    forms.push(HintContext::Review(ReviewBar::Look));
    forms
}
