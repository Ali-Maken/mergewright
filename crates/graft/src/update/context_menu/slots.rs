//! Where the `.` menu's history verbs sit (m6-conflicts/71): slot 1 is the
//! first Checkout, slot 2 Cherry-pick, so `. ⏎` and `. j ⏎` mean the same
//! thing on every row. The verbs themselves, and what each one runs, are
//! `verbs`'.

use graft_core::oid::Oid;
use graft_ops::mistake_radar::Finding;
use graft_term::{SanitizedText, sanitize_line};

use super::verbs::{
    HistoryVerb, checkout_verbs, delete_verbs, fix_verbs, head_tip, merge_and_rebase_verbs,
    pick_verb, reset_verbs, revert_verb, short,
};
use crate::refs::{RefBadges, RefLabel};
use crate::update::preview::current_branch;

/// The `.` menu's history items and where they sit:
/// `verbs[..leading]` go above `Explain` and the row's own actions, the rest
/// below them. `dim_first` is `(label, reason)` for a slot 1 that cannot
/// act — the `HEAD` row whose only local branch is the current one — and
/// carries no verb, so it dispatches nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Slots {
    pub verbs: Vec<HistoryVerb>,
    pub leading: usize,
    pub dim_first: Option<(SanitizedText, SanitizedText)>,
}

impl Slots {
    pub(crate) fn empty() -> Self {
        Slots {
            verbs: Vec::new(),
            leading: 0,
            dim_first: None,
        }
    }
}

/// The verbs `.` offers on the commit row `id`, in menu order, with their
/// slots: Checkout then Cherry-pick lead (`leading` 2), or a dim
/// `Checkout — already on <b>` then Cherry-pick (`leading` 1).
///
/// Empty whenever the repository has not told us enough to name an operation
/// honestly (see [`super::verbs::history_verbs`]).
///
/// **A stash row keeps the order it had before the slots** (`leading` 0):
/// Checkouts, Merge/Rebase, then Cherry-pick. Checking out or cherry-picking
/// a stash commit is not a daily task, so it does not earn the fixed slots
/// (conductor, 2026-09-25, review m6-final-fixes-66-67-71 Question A).
pub(crate) fn history_slots(
    id: Oid,
    parents: &[Oid],
    subject: &str,
    refs: &RefBadges,
    findings: &[Finding],
) -> Slots {
    // `HEAD` as a `Tip`: `None` only while the branch is unborn, where there
    // is no commit to operate from.
    let Some(head) = head_tip(refs) else {
        return Slots::empty();
    };
    let current = current_branch(refs);
    let here = short(id);
    let on = refs.on(id);

    let mut checkouts = Vec::new();
    let skipped_current = checkout_verbs(&mut checkouts, id, refs, on, &here);
    let pick = pick_verb(id, subject, &head, &here);

    let mut verbs = Vec::new();
    let (leading, dim_first) = if is_stash_row(on) {
        verbs.extend(checkouts);
        merge_and_rebase_verbs(&mut verbs, id, refs, current.as_ref(), &here);
        verbs.push(pick);
        (0, None)
    } else {
        let mut checkouts = checkouts.into_iter();
        let slots = match checkouts.next() {
            Some(first) => {
                verbs.push(first);
                (2, None)
            }
            // No Checkout at all: every local branch here is the current one.
            None => (
                1,
                skipped_current.map(|branch| {
                    (
                        sanitize_line("Checkout"),
                        sanitize_line(&format!("already on {branch}")),
                    )
                }),
            ),
        };
        verbs.push(pick);
        // Any further Checkout items lead the rest.
        verbs.extend(checkouts);
        // `m` and `r`'s own operations, from the row the cursor is on, built
        // by the very functions those keys use — the menu teaches the
        // keyboard, so it must not mean something else.
        merge_and_rebase_verbs(&mut verbs, id, refs, current.as_ref(), &here);
        slots
    };

    verbs.push(revert_verb(id, parents, subject, &head, &here));
    reset_verbs(&mut verbs, id, &head);
    delete_verbs(&mut verbs, id, on);
    fix_verbs(&mut verbs, id, parents, subject, refs, findings, &head);

    Slots {
        verbs,
        leading,
        dim_first,
    }
}

/// A row carrying a stash badge.
fn is_stash_row(on: &[RefLabel]) -> bool {
    on.iter().any(|label| matches!(label, RefLabel::Stash(_)))
}
