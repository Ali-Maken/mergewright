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
    CheckoutParams, CheckoutTarget, FastForward, MergeParams, Operation, PickParams, RebaseParams,
    RefDeleteParams, ResetMode, ResetParams, ResetTarget, RevertParams, Source, Tip,
};
use graft_term::{SanitizedText, sanitize_line};

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

/// The verbs `.` offers on the commit row `id`, in menu order.
///
/// Empty whenever the repository has not told us enough to name an operation
/// honestly: no refs yet, an unborn `HEAD`, or a Foxtrot finding whose real
/// previous tip is not among the merge's parents. An absent item is the
/// honest answer — a verb that cannot say what it would do is worse than no
/// verb at all.
pub fn history_verbs(
    id: Oid,
    parents: &[Oid],
    subject: &str,
    refs: &RefBadges,
    findings: &[Finding],
) -> Vec<HistoryVerb> {
    let mut verbs = Vec::new();
    // `HEAD` as a `Tip`: the branch it is on, or `HEAD` itself when
    // detached — the spelling `graft_ops::operation::history::tip_holds`
    // recognises. `None` only while the branch is unborn, where there is no
    // commit to operate from.
    let Some(head) = head_tip(refs) else {
        return verbs;
    };
    let current = current_branch(refs);
    let here = short(id);
    let on = refs.on(id);

    checkout_verbs(&mut verbs, id, refs, on, &here);

    // `m` and `r`'s own operations, from the row the cursor is on, built by
    // the very functions those keys use — the menu teaches the keyboard, so
    // it must not mean something else.
    merge_and_rebase_verbs(&mut verbs, id, refs, current.as_ref(), &here);

    verbs.push(verb(
        &format!("Cherry-pick {here}"),
        Operation::CherryPick(PickParams {
            commits: vec![id],
            subject: subject.to_owned(),
            head: head.clone(),
        }),
    ));
    verbs.push(revert_verb(id, parents, subject, &head, &here));
    reset_verbs(&mut verbs, id, &head);
    delete_verbs(&mut verbs, id, on);
    fix_verbs(&mut verbs, id, parents, subject, refs, findings, &head);

    verbs
}

/// `Checkout <b>` for each local branch on the row that `HEAD` is not
/// already on; on a row carrying no local branch at all, `Checkout <sha7>
/// , detached` instead.
fn checkout_verbs(
    verbs: &mut Vec<HistoryVerb>,
    id: Oid,
    refs: &RefBadges,
    on: &[RefLabel],
    here: &str,
) {
    let Some(head_before) = head_ref(refs) else {
        return;
    };
    let mut any_local = false;
    for label in on {
        let RefLabel::Branch { name, current, .. } = label else {
            continue;
        };
        any_local = true;
        if *current {
            continue;
        }
        let short = String::from_utf8_lossy(name).into_owned();
        verbs.push(verb(
            &format!("Checkout {short}"),
            // The full name: `check_checkout` looks it up in the facts under
            // exactly this key, and `plan_checkout` shortens it itself.
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Branch(format!("refs/heads/{short}")),
                head_before: head_before.clone(),
            }),
        ));
    }
    if !any_local {
        verbs.push(verb(
            &format!("Checkout {here}, detached"),
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Detached(id),
                head_before,
            }),
        ));
    }
}

/// `Merge <b> into <cur> (m)` and `Rebase <cur> onto <b> (r)`, the two
/// operations `m` and `r` build from this row. Absent on a detached `HEAD`
/// (neither has a branch to act on — the keys refuse in words there) and on
/// the current branch's own tip, where both are `already on it`.
///
/// These two are the only history verbs whose labels name a key, because they
/// are the only ones that have one: a trailing `(key)` in this menu means
/// "next time you can do this without opening it". Checkout, cherry-pick,
/// revert, reset and the deletions own no letter — which is why they live
/// behind `.` at all (`docs/design/02-keymap.md` §2.1) — so they carry none.
fn merge_and_rebase_verbs(
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
        Operation::Rebase(RebaseParams {
            branch: current.clone(),
            onto: source,
            onto_tip: id,
        }),
    ));
}

/// `Revert <sha7>`, or `Revert <sha7>, -m 1` on a merge: git refuses a
/// merge without `-m`, so the label says which side is kept rather than
/// offering a verb that cannot run.
fn revert_verb(id: Oid, parents: &[Oid], subject: &str, head: &Tip, here: &str) -> HistoryVerb {
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
fn reset_verbs(verbs: &mut Vec<HistoryVerb>, id: Oid, head: &Tip) {
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
fn delete_verbs(verbs: &mut Vec<HistoryVerb>, id: Oid, on: &[RefLabel]) {
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
fn fix_verbs(
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
        &format!("Fix: revert the merge (-m {mainline})"),
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
            Operation::Rebase(RebaseParams {
                branch: head.clone(),
                onto: Source::Commit(real_tip),
                onto_tip: real_tip,
            }),
        ));
    }
}

/// `HEAD` as a [`Tip`]: its branch when it has one, else `HEAD` itself at the
/// commit it is detached on. `None` on an unborn branch.
fn head_tip(refs: &RefBadges) -> Option<Tip> {
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
fn head_ref(refs: &RefBadges) -> Option<HeadRef> {
    let head = refs.head()?;
    Some(current_branch(refs).map_or(HeadRef::Detached(head), |tip| HeadRef::Branch(tip.name)))
}

/// The first seven hex digits, the way every other row label spells a commit.
fn short(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

fn verb(label: &str, op: Operation) -> HistoryVerb {
    HistoryVerb {
        label: sanitize_line(label),
        op,
    }
}

#[cfg(test)]
mod tests;
