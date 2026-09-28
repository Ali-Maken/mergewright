//! The history verbs behind `.` on a commit row, and the operation each one
//! means (m5-history-and-ghost-preview/08, piece 6).
//!
//! Pure: it reads the row, the refs and the row's Mistake Radar findings and
//! answers with a list. Nothing here touches `AppState`, opens a preview or
//! runs anything — `.` ends in a picture, and only `Confirm` in that picture
//! writes (PROMPT.md §1.7).
//!
//! **This is where the mapping lives, and it lives here on purpose.** The
//! menu widget reports which row of itself the user chose
//! (`ContextAction::History(i)`); `history_verbs()[i].op` is what row `i`
//! meant. `graft-ui` never names a `CheckoutTarget`, a `ResetMode` or an
//! `Oid`-bearing operation, because `ui` never imports `ops`
//! (`ARCHITECTURE.md`) — and because a ref name only reaches a widget as
//! `term::SanitizedText`, which is what the label beside each index is.
//!
//! Every operation is pinned to the tips the row was drawn against, the same
//! way `m` and `r` pin theirs (`update::preview::history_operation`): the
//! menu names a picture, and `graft_ops`' `check` refuses `Moved` rather than
//! silently re-planning if the repository has moved by the time `Confirm` is
//! pressed.

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_ops::mistake_radar::{Evidence, Finding};
use graft_ops::operation::{
    CheckoutParams, CheckoutTarget, FastForward, MergeParams, OnEdits, Operation, PickParams,
    RebaseParams, RefDeleteParams, Refusal, ResetMode, ResetParams, ResetTarget, RevertParams,
    Source, Tip,
};
use graft_term::{SanitizedText, sanitize_line};

#[cfg(test)]
use super::slots::history_slots;
use crate::refs::{RefBadges, RefLabel};
use crate::update::preview::{current_branch, short_name_of, source_at};

/// One history verb the `.` menu offers: what the row says, and what
/// choosing it would run.
///
/// The label is already sanitised — it names refs and commit subjects out of
/// the repository, so this is the boundary `DESIGN.md` §8.2 asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryVerb {
    /// The menu row, e.g. `Merge feature into main`.
    pub label: SanitizedText,
    /// The operation the preview is drawn from, and that `Confirm` runs.
    pub op: Operation,
}

/// The verbs `.` offers on the commit row `id`, in menu order: the flat list
/// of [`history_slots`], which is what the menu itself builds from. Test-only.
///
/// Empty whenever the repository has not told us enough to name an operation
/// honestly: no refs yet, an unborn `HEAD`, or a Foxtrot finding whose real
/// previous tip is not among the merge's parents. An absent item is the
/// honest answer — a verb that cannot say what it would do is worse than no
/// verb at all.
#[cfg(test)]
pub fn history_verbs(
    id: Oid,
    parents: &[Oid],
    subject: &str,
    refs: &RefBadges,
    findings: &[Finding],
) -> Vec<HistoryVerb> {
    history_slots(id, parents, subject, refs, findings).verbs
}

/// `Checkout <b>` for each local branch on the row that `HEAD` is not
/// already on; on a row carrying no local branch at all, `Checkout <sha7>
/// , detached` instead. Answers the current branch when it is on the row and
/// was skipped, so the caller can say `already on <b>`.
pub(super) fn checkout_verbs(
    verbs: &mut Vec<HistoryVerb>,
    id: Oid,
    refs: &RefBadges,
    on: &[RefLabel],
    here: &str,
) -> Option<String> {
    let head_before = head_ref(refs)?;
    let mut any_local = false;
    let mut skipped = None;
    for label in on {
        let RefLabel::Branch { name, current, .. } = label else {
            continue;
        };
        any_local = true;
        let short = String::from_utf8_lossy(name).into_owned();
        if *current {
            skipped = Some(short);
            continue;
        }
        verbs.push(verb(
            &format!("Checkout {short}"),
            // The full name: `check_checkout` looks it up in the facts under
            // exactly this key, and `plan_checkout` shortens it itself.
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Branch(format!("refs/heads/{short}")),
                head_before: head_before.clone(),
                // The previewed form; the switch rule (`update::switch`)
                // launches it with `Refuse` on a clean tree.
                on_edits: OnEdits::SetAside,
            }),
        ));
    }
    if !any_local {
        verbs.push(verb(
            &format!("Checkout {here}, detached"),
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Detached(id),
                head_before,
                on_edits: OnEdits::SetAside,
            }),
        ));
    }
    skipped
}

/// Taking `id` onto `HEAD`: the one builder `o` (`update::preview::take_key`)
/// and slot 2 share, so the key and the menu item open the same preview.
pub(in crate::update) fn pick_params(id: Oid, subject: &str, head: &Tip) -> PickParams {
    PickParams {
        commits: vec![id],
        subject: subject.to_owned(),
        head: head.clone(),
    }
}

/// `Cherry-pick <sha7>` onto `HEAD`, with no key: a stash row's item, which
/// `o` refuses (`(key)` promises a live key, keymap §2.1).
pub(super) fn pick_verb(id: Oid, subject: &str, head: &Tip, here: &str) -> HistoryVerb {
    verb(
        &format!("Cherry-pick {here}"),
        Operation::CherryPick(pick_params(id, subject, head)),
    )
}

/// Slot 2 on a commit row (m7-power-tools/11): `Cherry-pick <sha7> (o)`, or
/// — where `o` would refuse — `Cherry-pick (o)` with `o`'s refusal words as
/// its dim reason (the sha dropped: the row names it, as slot 1 drops the
/// branch).
///
/// Decided from facts the app already holds, no git read on menu open:
/// `HEAD`'s own row is already on the branch, and 2+ parents is a merge,
/// both through [`PickParams::check_commit`] so graft-ops decides and this
/// only words it. An ancestor of `HEAD` stays live here; its preview's plan
/// refuses it in the same words.
pub(super) fn pick_slot(
    id: Oid,
    parents: &[Oid],
    subject: &str,
    head: &Tip,
    here: &str,
) -> (HistoryVerb, Option<SanitizedText>) {
    let params = pick_params(id, subject, head);
    let dim = params
        .check_commit(id, id == head.oid, parents.len())
        .err()
        .and_then(|refusal| pick_dim_reason(&refusal));
    let label = if dim.is_some() {
        "Cherry-pick (o)".to_owned()
    } else {
        format!("Cherry-pick {here} (o)")
    };
    (verb(&label, Operation::CherryPick(params)), dim)
}

/// Slot 2's dim reason for `o`'s refusal: `already on <b>`, or the merge
/// words. `None` for any other refusal, which `check_commit` never gives.
fn pick_dim_reason(refusal: &Refusal) -> Option<SanitizedText> {
    match refusal {
        Refusal::AlreadyOnBranch { branch, .. } => {
            Some(sanitize_line(&format!("already on {branch}")))
        }
        Refusal::PickMerge { .. } => Some(sanitize_line(
            "a merge \u{b7} taking a merge isn't in Graft yet",
        )),
        _ => None,
    }
}

/// `Merge <b> into <cur> (m)` and `Rebase <cur> onto <b> (r)`, the two
/// operations `m` and `r` build from this row. Absent on a detached `HEAD`
/// (neither has a branch to act on — the keys refuse in words there) and on
/// the current branch's own tip, where both are `already on it`.
///
/// A trailing `(key)` in this menu means "next time you can do this without
/// opening it", so a label names a key only where one is live: these two,
/// and slot 2's Cherry-pick `(o)` since M7 ([`pick_slot`]). Checkout,
/// revert, reset and the deletions own no letter — which is why they live
/// behind `.` at all (`docs/design/02-keymap.md` §2.1) — so they carry none.
pub(super) fn merge_and_rebase_verbs(
    verbs: &mut Vec<HistoryVerb>,
    id: Oid,
    refs: &RefBadges,
    current: Option<&Tip>,
    here: &str,
) {
    let Some(current) = current else {
        return;
    };
    let source = source_at(refs, id);
    let cur = short_name_of(&current.name);
    if current.oid == id && matches!(&source, Source::Branch(name) if *name == cur) {
        return;
    }
    let named = match &source {
        Source::Branch(name) => name.clone(),
        Source::Commit(_) => here.to_owned(),
    };
    verbs.push(verb(
        &format!("Merge {named} into {cur} (m)"),
        Operation::Merge(MergeParams {
            source: source.clone(),
            into: current.clone(),
            source_tip: id,
            ff: FastForward::Default,
        }),
    ));
    verbs.push(verb(
        &format!("Rebase {cur} onto {named} (r)"),
        Operation::Rebase(RebaseParams::new(current.clone(), source, id)),
    ));
}

/// `Revert <sha7>`, or `Revert <sha7>, -m 1` on a merge: git refuses a
/// merge without `-m`, so the label says which side is kept rather than
/// offering a verb that cannot run.
pub(super) fn revert_verb(
    id: Oid,
    parents: &[Oid],
    subject: &str,
    head: &Tip,
    here: &str,
) -> HistoryVerb {
    let is_merge = parents.len() > 1;
    let (label, mainline) = if is_merge {
        (format!("Revert {here}, -m 1"), Some(1))
    } else {
        (format!("Revert {here}"), None)
    };
    verb(
        &label,
        Operation::Revert(RevertParams {
            commit: id,
            subject: subject.to_owned(),
            mainline,
            parents: parents.len(),
            // The parent the reverse patch is taken against: `c^` for an
            // ordinary commit, `c^<mainline>` for a merge — both the first
            // parent here. `None` costs the conflict forecast, nothing else.
            parent: parents.first().copied(),
            head: head.clone(),
        }),
    )
}

/// `Reset <cur> to here, mixed` and `, hard`. `--soft` is not offered: the
/// design note lists two.
///
/// The mode is **not** parenthesised. A trailing `(x)` anywhere in this menu
/// means "that letter is live right now, press it and skip the menu"
/// (`docs/design/02-keymap.md` §2.1); `mixed` and `hard` are reset modes, not
/// keys, and wore that shape until m5-history-and-ghost-preview/12.
pub(super) fn reset_verbs(verbs: &mut Vec<HistoryVerb>, id: Oid, head: &Tip) {
    let cur = short_name_of(&head.name);
    for (mode, word) in [(ResetMode::Mixed, "mixed"), (ResetMode::Hard, "hard")] {
        verbs.push(verb(
            &format!("Reset {cur} to here, {word}"),
            Operation::Reset(ResetParams {
                target: ResetTarget::Head {
                    branch: head.clone(),
                    mode,
                },
                to: id,
            }),
        ));
    }
}

/// `Delete branch <b>` for every local branch on the row but the current one
/// — deleting the branch you are standing on is not a thing git will do —
/// and `Delete tag <t>` for every tag. Both pinned by compare-and-swap to
/// the commit the row showed.
pub(super) fn delete_verbs(verbs: &mut Vec<HistoryVerb>, id: Oid, on: &[RefLabel]) {
    for label in on {
        match label {
            RefLabel::Branch {
                name,
                current: false,
                ..
            } => {
                let short = String::from_utf8_lossy(name).into_owned();
                verbs.push(verb(
                    &format!("Delete branch {short}"),
                    Operation::BranchDelete(RefDeleteParams {
                        name: format!("refs/heads/{short}"),
                        expected: id,
                    }),
                ));
            }
            RefLabel::Tag(name) => {
                let short = String::from_utf8_lossy(name).into_owned();
                verbs.push(verb(
                    &format!("Delete tag {short}"),
                    Operation::TagDelete(RefDeleteParams {
                        name: format!("refs/tags/{short}"),
                        expected: id,
                    }),
                ));
            }
            RefLabel::Branch { .. }
            | RefLabel::Remote(_)
            | RefLabel::DetachedHead
            | RefLabel::Stash(_) => {}
        }
    }
}

/// The three Fix… items the Mistake Radar's Foxtrot finding earns this row
/// (PROMPT.md §1.4, design note "`.` menu additions").
///
/// A foxtrot merge is one on the default branch whose *first* parent is not
/// that branch's real previous tip: the branch's own line was rewritten. All
/// three fixes are stated against `real_tip`, the finding's name for that
/// previous tip:
///
/// - revert the merge keeping the previous tip's side (`-m N`, where `N` is
///   that tip's parent number — 2 for a foxtrot);
/// - reset the branch back to it;
/// - rebase onto it, which is the merge the user meant to make. It is the
///   *other* parent in the finding's own terms — the non-first one — and it
///   is the only one that replays the topic commits rather than replaying
///   the default branch onto the topic.
///
/// Absent, all three, when `real_tip` is not among the merge's parents:
/// nothing here would then be able to say which side it keeps.
pub(super) fn fix_verbs(
    verbs: &mut Vec<HistoryVerb>,
    id: Oid,
    parents: &[Oid],
    subject: &str,
    refs: &RefBadges,
    findings: &[Finding],
    head: &Tip,
) {
    let Some(real_tip) = findings.iter().find_map(|finding| match &finding.evidence {
        Evidence::Foxtrot {
            merge, real_tip, ..
        } if *merge == id => Some(*real_tip),
        _ => None,
    }) else {
        return;
    };
    let Some(mainline) = parents
        .iter()
        .position(|parent| *parent == real_tip)
        .and_then(|at| u8::try_from(at + 1).ok())
    else {
        return;
    };
    let cur = short_name_of(&head.name);
    verbs.push(verb(
        &format!("Fix: revert the merge, -m {mainline}"),
        Operation::Revert(RevertParams {
            commit: id,
            subject: subject.to_owned(),
            mainline: Some(mainline),
            parents: parents.len(),
            parent: Some(real_tip),
            head: head.clone(),
        }),
    ));
    verbs.push(verb(
        &format!("Fix: reset {cur} to before the merge"),
        Operation::Reset(ResetParams {
            target: ResetTarget::Head {
                branch: head.clone(),
                mode: ResetMode::Hard,
            },
            to: real_tip,
        }),
    ));
    // Only while `HEAD` is on the merge: a rebase moves the branch you are
    // standing on, so offering it from a row you are not on would name a
    // different branch than the one the label says.
    if refs.head() == Some(id) {
        verbs.push(verb(
            "Fix: rebase instead",
            Operation::Rebase(RebaseParams::new(
                head.clone(),
                Source::Commit(real_tip),
                real_tip,
            )),
        ));
    }
}

/// `HEAD` as a [`Tip`]: its branch when it has one, else `HEAD` itself at the
/// commit it is detached on. `None` on an unborn branch.
pub(in crate::update) fn head_tip(refs: &RefBadges) -> Option<Tip> {
    if let Some(tip) = current_branch(refs) {
        return Some(tip);
    }
    refs.head().map(|oid| Tip {
        name: "HEAD".to_owned(),
        oid,
    })
}

/// `HEAD` as `graft_ops` reads it from the repository, so `check_checkout`
/// can compare the two and refuse when it moved.
pub(in crate::update) fn head_ref(refs: &RefBadges) -> Option<HeadRef> {
    let head = refs.head()?;
    Some(current_branch(refs).map_or(HeadRef::Detached(head), |tip| HeadRef::Branch(tip.name)))
}

/// The first seven hex digits, the way every other row label spells a commit.
pub(super) fn short(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

pub(super) fn verb(label: &str, op: Operation) -> HistoryVerb {
    HistoryVerb {
        label: sanitize_line(label),
        op,
    }
}

#[cfg(test)]
mod tests;
