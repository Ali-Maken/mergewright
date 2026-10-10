//! What each `.` menu verb offers, and the operation behind it.
//!
//! Scripted: nothing here opens a repository. What a verb *means* is settled
//! here; what the operation *does* is `graft_ops`' and is proved against real
//! git there.

use graft_core::mistake_radar::{MistakeRadarKind, Severity};
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;

use super::*;

/// `n` in the *first* byte, so two fixtures never share a short id and a
/// label that names one can be told from a label that names the other.
fn oid(n: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0] = n;
    Oid::from_sha1(bytes)
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

/// The menu's own fixture builder — one shape of "a row's refs" for both
/// files (m5-08 review, Major 6).
fn built(refs: &[(RefName, Oid)], head: &HeadState) -> RefBadges {
    super::super::tests::ref_badges(refs, head)
}

/// `main` at commit 2 with `HEAD` on it, `feature` and the tag `v1` at
/// commit 3 — the row every table below is read from.
fn main_and_feature() -> RefBadges {
    built(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
            (name("refs/tags/v1"), oid(3)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
    )
}

fn main_tip() -> Tip {
    Tip {
        name: "refs/heads/main".to_owned(),
        oid: oid(2),
    }
}

fn labels(verbs: &[HistoryVerb]) -> Vec<&str> {
    verbs.iter().map(|verb| verb.label.as_str()).collect()
}

fn op_of<'a>(verbs: &'a [HistoryVerb], label: &str) -> &'a Operation {
    let Some(found) = verbs.iter().find(|verb| verb.label.as_str() == label) else {
        panic!("no verb labelled {label:?} in {:?}", labels(verbs))
    };
    &found.op
}

/// `oid(3)`, as every row spells it.
const SHA3: &str = "0300000";

/// The table in the ticket's piece 6, verb by verb: every label the menu
/// offers on a branch row, and the exact `Operation` behind each one.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the ticket's verb table: one assertion per row, not logic"
)]
fn menu_verbs_build_expected_operations() {
    let refs = main_and_feature();
    let verbs = history_verbs(
        oid(3),
        &[oid(2)],
        "third",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );

    assert_eq!(
        labels(&verbs),
        vec![
            "Checkout feature",
            "Cherry-pick 0300000 (o)",
            "Merge feature into main (m)",
            "Rebase main onto feature (r)",
            "Revert 0300000",
            "Reset main to here, mixed",
            "Reset main to here, hard",
            "Delete branch feature",
            "Delete tag v1",
        ]
    );

    assert_eq!(
        op_of(&verbs, "Checkout feature"),
        &Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            on_edits: graft_ops::operation::OnEdits::SetAside,
        })
    );
    assert_eq!(
        op_of(&verbs, "Merge feature into main (m)"),
        &Operation::Merge(MergeParams {
            source: Source::Branch("feature".to_owned()),
            into: main_tip(),
            source_tip: oid(3),
            ff: FastForward::Default,
        })
    );
    assert_eq!(
        op_of(&verbs, "Rebase main onto feature (r)"),
        &Operation::Rebase(RebaseParams::new(
            main_tip(),
            Source::Branch("feature".to_owned()),
            oid(3)
        ))
    );
    assert_eq!(
        op_of(&verbs, "Cherry-pick 0300000 (o)"),
        &Operation::CherryPick(PickParams {
            commits: vec![oid(3)],
            subject: "third".to_owned(),
            head: main_tip(),
        })
    );
    assert_eq!(
        op_of(&verbs, "Revert 0300000"),
        &Operation::Revert(RevertParams {
            commit: oid(3),
            subject: "third".to_owned(),
            mainline: None,
            parents: 1,
            parent: Some(oid(2)),
            head: main_tip(),
        })
    );
    assert_eq!(
        op_of(&verbs, "Reset main to here, mixed"),
        &Operation::Reset(ResetParams {
            target: ResetTarget::Head {
                branch: main_tip(),
                mode: ResetMode::Mixed,
            },
            to: oid(3),
        })
    );
    assert_eq!(
        op_of(&verbs, "Reset main to here, hard"),
        &Operation::Reset(ResetParams {
            target: ResetTarget::Head {
                branch: main_tip(),
                mode: ResetMode::Hard,
            },
            to: oid(3),
        })
    );
    assert_eq!(
        op_of(&verbs, "Delete branch feature"),
        &Operation::BranchDelete(RefDeleteParams {
            name: "refs/heads/feature".to_owned(),
            expected: oid(3),
        })
    );
    assert_eq!(
        op_of(&verbs, "Delete tag v1"),
        &Operation::TagDelete(RefDeleteParams {
            name: "refs/tags/v1".to_owned(),
            expected: oid(3),
        })
    );
}

/// `m` and `r` are the same two operations from the same row: the menu
/// teaches the keyboard, so it must not quietly mean something else.
#[test]
fn the_merge_and_rebase_verbs_are_the_ones_m_and_r_build() {
    let refs = main_and_feature();
    let verbs = history_verbs(
        oid(3),
        &[oid(2)],
        "third",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    let source = source_at(&refs, oid(3));
    let current = current_branch(&refs).expect("HEAD is on main");

    assert_eq!(
        op_of(&verbs, "Merge feature into main (m)"),
        &Operation::Merge(MergeParams {
            source: source.clone(),
            into: current.clone(),
            source_tip: oid(3),
            ff: FastForward::Default,
        })
    );
    assert_eq!(
        op_of(&verbs, "Rebase main onto feature (r)"),
        &Operation::Rebase(RebaseParams::new(current, source, oid(3)))
    );
}

/// A row with no local branch on it is checked out detached, by id.
#[test]
fn a_row_with_no_branch_offers_a_detached_checkout() {
    let refs = main_and_feature();
    let verbs = history_verbs(
        oid(9),
        &[oid(2)],
        "ninth",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    assert_eq!(
        op_of(&verbs, "Checkout 0900000, detached"),
        &Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Detached(oid(9)),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            on_edits: graft_ops::operation::OnEdits::SetAside,
        })
    );
    assert!(
        !labels(&verbs).contains(&"Checkout main"),
        "{:?}",
        labels(&verbs)
    );
}

/// The branch `HEAD` is on is not offered for checkout or deletion: you are
/// already on it, and git will not delete it.
#[test]
fn the_current_branchs_own_row_offers_neither_checkout_nor_delete_nor_merge() {
    let refs = main_and_feature();
    let verbs = history_verbs(
        oid(2),
        &[oid(1)],
        "second",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    let labels = labels(&verbs);
    assert_eq!(
        labels,
        vec![
            "Cherry-pick (o)",
            "Revert 0200000",
            "Reset main to here, mixed",
            "Reset main to here, hard",
        ],
        "only the verbs that still mean something on HEAD's own tip"
    );
}

/// A merge row's revert has to name a side: git refuses a merge without
/// `-m`, so the label says `-m 1` rather than offering a verb that cannot
/// run.
#[test]
fn a_merge_rows_revert_names_its_mainline() {
    let refs = main_and_feature();
    let verbs = history_verbs(
        oid(3),
        &[oid(2), oid(4)],
        "merged",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    assert_eq!(
        op_of(&verbs, "Revert 0300000, -m 1"),
        &Operation::Revert(RevertParams {
            commit: oid(3),
            subject: "merged".to_owned(),
            mainline: Some(1),
            parents: 2,
            parent: Some(oid(2)),
            head: main_tip(),
        })
    );
}

/// Detached `HEAD`: no branch to merge into or rebase, but the verbs that
/// act on `HEAD` itself still do, and say `HEAD` rather than a branch name.
#[test]
fn a_detached_head_keeps_the_verbs_that_do_not_need_a_branch() {
    let refs = built(
        &[(name("refs/heads/feature"), oid(3))],
        &HeadState::Detached(oid(2)),
    );
    let verbs = history_verbs(
        oid(3),
        &[oid(2)],
        "third",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    let labels = labels(&verbs);
    assert!(
        !labels.iter().any(|label| label.starts_with("Merge")),
        "{labels:?}"
    );
    assert!(
        !labels.iter().any(|label| label.starts_with("Rebase")),
        "{labels:?}"
    );
    assert_eq!(
        op_of(&verbs, "Reset HEAD to here, hard"),
        &Operation::Reset(ResetParams {
            target: ResetTarget::Head {
                branch: Tip {
                    name: "HEAD".to_owned(),
                    oid: oid(2),
                },
                mode: ResetMode::Hard,
            },
            to: oid(3),
        })
    );
    assert_eq!(
        op_of(&verbs, "Checkout feature"),
        &Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
            head_before: HeadRef::Detached(oid(2)),
            on_edits: graft_ops::operation::OnEdits::SetAside,
        })
    );
}

/// An unborn branch has no commit to act from, so the menu offers no verb
/// rather than one it cannot pin.
#[test]
fn an_unborn_head_offers_no_history_verb() {
    let refs = built(&[], &HeadState::Unborn(name("refs/heads/main")));
    assert!(
        history_verbs(
            oid(3),
            &[],
            "third",
            &refs,
            &[],
            graft_ui::keymap::Keymap::shipped()
        )
        .is_empty()
    );
}

fn foxtrot(merge: Oid, first_parent: Oid, real_tip: Oid) -> Finding {
    Finding {
        kind: MistakeRadarKind::FoxtrotMerge,
        severity: Severity::Warning,
        at: merge,
        evidence: Evidence::Foxtrot {
            merge,
            first_parent,
            real_tip,
            branch: "main".to_owned(),
        },
    }
}

/// A foxtrot merge `HEAD` is standing on: all three Fix… items, each stated
/// against the branch's real previous tip (`real_tip`, parent 2 here).
#[test]
fn a_foxtrot_merge_head_is_on_offers_all_three_fixes() {
    // `main` is on the merge itself, so `Fix: rebase instead` is offered.
    let refs = built(
        &[(name("refs/heads/main"), oid(2))],
        &HeadState::Branch(name("refs/heads/main")),
    );
    let findings = [foxtrot(oid(2), oid(7), oid(5))];
    let verbs = history_verbs(
        oid(2),
        &[oid(7), oid(5)],
        "Merge branch 'x'",
        &refs,
        &findings,
        graft_ui::keymap::Keymap::shipped(),
    );
    let head = main_tip();

    assert_eq!(
        op_of(&verbs, "Fix: revert the merge, -m 2"),
        &Operation::Revert(RevertParams {
            commit: oid(2),
            subject: "Merge branch 'x'".to_owned(),
            mainline: Some(2),
            parents: 2,
            parent: Some(oid(5)),
            head: head.clone(),
        })
    );
    assert_eq!(
        op_of(&verbs, "Fix: reset main to before the merge"),
        &Operation::Reset(ResetParams {
            target: ResetTarget::Head {
                branch: head.clone(),
                mode: ResetMode::Hard,
            },
            to: oid(5),
        })
    );
    // Onto the branch's own previous tip — the parent that is *not* the
    // first one. Rebasing onto `first_parent` would replay the default
    // branch onto the topic, which is the mistake turned around.
    assert_eq!(
        op_of(&verbs, "Fix: rebase instead"),
        &Operation::Rebase(RebaseParams::new(head, Source::Commit(oid(5)), oid(5)))
    );
}

/// `Fix: rebase instead` moves the branch you are standing on, so it is
/// offered only while you are standing on the merge. The other two fixes
/// still are.
#[test]
fn fix_rebase_is_absent_unless_head_is_on_the_merge() {
    let refs = main_and_feature(); // HEAD is on `main` at commit 2.
    let findings = [foxtrot(oid(3), oid(7), oid(5))];
    let verbs = history_verbs(
        oid(3),
        &[oid(7), oid(5)],
        "Merge branch 'x'",
        &refs,
        &findings,
        graft_ui::keymap::Keymap::shipped(),
    );
    let labels = labels(&verbs);
    assert!(!labels.contains(&"Fix: rebase instead"), "{labels:?}");
    assert!(
        labels.contains(&"Fix: revert the merge, -m 2"),
        "{labels:?}"
    );
    assert!(
        labels.contains(&"Fix: reset main to before the merge"),
        "{labels:?}"
    );
}

/// When the previous tip is not among the merge's parents, nothing here can
/// say which side a fix would keep — so no Fix… item is offered at all,
/// rather than one that guesses.
#[test]
fn no_fix_item_when_the_previous_tip_is_not_a_parent() {
    let refs = main_and_feature();
    let findings = [foxtrot(oid(3), oid(7), oid(99))];
    let verbs = history_verbs(
        oid(3),
        &[oid(7), oid(5)],
        "Merge branch 'x'",
        &refs,
        &findings,
        graft_ui::keymap::Keymap::shipped(),
    );
    let labels = labels(&verbs);
    assert!(
        !labels.iter().any(|label| label.starts_with("Fix:")),
        "{labels:?}"
    );
}

/// A finding about a different commit is not this row's to fix.
#[test]
fn a_foxtrot_on_another_row_adds_nothing_here() {
    let refs = main_and_feature();
    let findings = [foxtrot(oid(8), oid(7), oid(5))];
    let verbs = history_verbs(
        oid(3),
        &[oid(2)],
        "third",
        &refs,
        &findings,
        graft_ui::keymap::Keymap::shipped(),
    );
    assert!(
        !labels(&verbs).iter().any(|label| label.starts_with("Fix:")),
        "{:?}",
        labels(&verbs)
    );
}

/// Every label is `SanitizedText` by the time it leaves here (`DESIGN.md`
/// §8.2). `RefName` already refuses a control byte, so the character that
/// actually gets through is an invisible one: a right-to-left override in a
/// branch name would reorder the menu row it is drawn on. The label is
/// cleaned; the operation still carries the real name, because only what is
/// drawn is cleaned.
#[test]
fn an_invisible_character_in_a_ref_name_is_cleaned_out_of_the_label() {
    const EVIL: &str = "fix\u{202e}gnp.exe";
    let refs = built(
        &[
            (name("refs/heads/main"), oid(2)),
            (name(&format!("refs/heads/{EVIL}")), oid(3)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
    );
    let verbs = history_verbs(
        oid(3),
        &[oid(2)],
        "third",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    for verb in &verbs {
        assert!(
            !verb.label.as_str().contains('\u{202e}'),
            "{:?}",
            verb.label.as_str()
        );
    }
    let checkout = verbs
        .iter()
        .find(|verb| verb.label.as_str().starts_with("Checkout "))
        .expect("the branch row offers a checkout");
    assert_eq!(
        checkout.op,
        Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Branch(format!("refs/heads/{EVIL}")),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            on_edits: graft_ops::operation::OnEdits::SetAside,
        }),
        "the operation keeps the name git will be given"
    );
}

/// The short id the labels use is the seven digits every other row shows.
#[test]
fn a_commit_is_spelled_the_same_seven_digits_everywhere() {
    assert_eq!(short(oid(3)), SHA3);
}

/// The key a label names, read back from the dispatcher that makes the
/// promise true rather than from a hand-kept list (m5/14-22 review, Minor 2:
/// a list someone edits alongside the label it is meant to check is not a
/// check). `update::keys::write_key` sends `m`/`r` straight to
/// `preview::is_history_key`, so what that function answers *is* the set of
/// keys a history verb may put in trailing parens — and which verb each one
/// owns. A hint of more than one character is not a key press at all.
fn key_behind(hint: &str) -> Option<Behind> {
    let mut chars = hint.chars();
    let only = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let press = graft_core::event::KeyPress::plain(graft_core::event::KeyCode::Char(only));
    if crate::update::preview::is_take_key(press) {
        return Some(Behind::Take);
    }
    crate::update::preview::is_history_key(press).map(Behind::History)
}

/// What a label's key runs: `m`/`r`, or `o` (m7-power-tools/11), which
/// `write_key` sends to `preview::is_take_key`.
#[derive(Debug)]
enum Behind {
    History(crate::update::preview::HistoryKey),
    Take,
}

/// Every label `history_verbs` can build, across every shape that changes
/// which verbs are offered.
///
/// `graft-ui`'s `every_label_names_its_key` cannot reach these: it builds
/// the menu with `Vec::new()` for the verbs, so the rule it exists to
/// enforce was unenforced on exactly the labels that broke it (review 08
/// Major 3). `graft-ui` cannot depend on `graft`, so the reach belongs
/// here, next to the labels themselves.
fn every_label_the_app_can_build() -> Vec<HistoryVerb> {
    let keys = graft_ui::keymap::Keymap::shipped();
    let mut all: Vec<HistoryVerb> = Vec::new();
    let mut take = |verbs: Vec<HistoryVerb>| {
        all.extend(verbs);
    };

    let refs = main_and_feature();
    // An ordinary row with a branch on it: checkout, merge, rebase,
    // cherry-pick, revert, reset, delete branch, delete tag.
    take(history_verbs(oid(3), &[oid(2)], "third", &refs, &[], keys));
    // A row with no badge at all: the detached checkout.
    take(history_verbs(oid(9), &[oid(2)], "ninth", &refs, &[], keys));
    // The current branch's own row.
    take(history_verbs(oid(2), &[oid(1)], "second", &refs, &[], keys));
    // A merge row: the revert names its mainline.
    take(history_verbs(
        oid(3),
        &[oid(2), oid(4)],
        "merged",
        &refs,
        &[],
        keys,
    ));
    // A detached `HEAD`.
    take(history_verbs(
        oid(3),
        &[oid(2)],
        "third",
        &built(
            &[(name("refs/heads/feature"), oid(3))],
            &HeadState::Detached(oid(2)),
        ),
        &[],
        keys,
    ));
    // A foxtrot merge `HEAD` is standing on: all three Fix… items.
    take(history_verbs(
        oid(2),
        &[oid(7), oid(5)],
        "Merge branch 'x'",
        &built(
            &[(name("refs/heads/main"), oid(2))],
            &HeadState::Branch(name("refs/heads/main")),
        ),
        &[foxtrot(oid(2), oid(7), oid(5))],
        keys,
    ));

    assert!(
        all.len() > 15,
        "the sweep must actually reach the labels: {all:?}"
    );
    all
}

/// The trailing key-hint rule, over the labels `graft-ui`'s own invariant
/// test never sees. A trailing `(x)` means "that key is live, next time you
/// can skip the menu"; a qualifier that is not a key — a reset mode,
/// `detached`, `-m 1` — goes after a comma.
#[test]
fn a_trailing_parenthesis_on_a_verb_label_is_always_a_live_key() {
    for verb in every_label_the_app_can_build() {
        let label = verb.label.as_str();
        let Some(open) = label.rfind(" (") else {
            continue;
        };
        assert!(
            label.ends_with(')'),
            "a parenthesis that never closes is not a shape this rule can read: {label}"
        );
        let hint = &label[open + 2..label.len() - 1];
        let Some(which) = key_behind(hint) else {
            panic!(
                "{label} wears the key-hint shape, but pressing {hint} on the graph \
                 does nothing - a qualifier goes after a comma"
            )
        };
        // Half the promise is *which* verb the key does: `(m)` on a rebase
        // would teach a shortcut that runs something else.
        let runs = match which {
            Behind::Take => matches!(verb.op, Operation::CherryPick(_)),
            Behind::History(crate::update::preview::HistoryKey::Merge) => {
                matches!(verb.op, Operation::Merge(_))
            }
            Behind::History(crate::update::preview::HistoryKey::Rebase) => {
                matches!(verb.op, Operation::Rebase(_))
            }
        };
        assert!(
            runs,
            "{label} teaches {hint}, but that key runs {which:?} and this row offers {:?}",
            verb.op
        );
    }
}

/// m6-conflicts/71: on every commit row the menu leads with Checkout, then
/// Cherry-pick — the `HEAD` row whose only local branch is the current one
/// leads with a dim `Checkout — already on <b>` instead. One row per shape.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the ticket's table of commit rows: one case per row, not logic"
)]
fn checkout_and_cherry_pick_hold_the_first_two_slots() {
    struct Case<'a> {
        name: &'a str,
        row: Oid,
        parents: Vec<Oid>,
        refs: &'a RefBadges,
        findings: &'a [Finding],
        first: &'a str,
        second: &'a str,
        dim: Option<&'a str>,
    }
    let head_main = HeadState::Branch(name("refs/heads/main"));
    let plain = main_and_feature();
    let two_on_three = built(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
            (name("refs/heads/topic"), oid(3)),
        ],
        &head_main,
    );
    let n7 = built(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/topic"), oid(2)),
        ],
        &head_main,
    );
    let marked = [foxtrot(oid(3), oid(7), oid(5))];
    let case = |name, row, parents, refs, findings, first, second, dim| Case {
        name,
        row,
        parents,
        refs,
        findings,
        first,
        second,
        dim,
    };
    let table = [
        case(
            "branch tip",
            oid(3),
            vec![oid(2)],
            &plain,
            &[][..],
            "Checkout feature",
            "Cherry-pick 0300000 (o)",
            None,
        ),
        case(
            "HEAD, current only",
            oid(2),
            vec![oid(1)],
            &plain,
            &[][..],
            "Cherry-pick (o)",
            "",
            Some("already on main"),
        ),
        case(
            "radar-marked",
            oid(3),
            vec![oid(7), oid(5)],
            &plain,
            &marked[..],
            "Checkout feature",
            "Cherry-pick (o)",
            None,
        ),
        case(
            "two local branches",
            oid(3),
            vec![oid(2)],
            &two_on_three,
            &[][..],
            "Checkout feature",
            "Cherry-pick 0300000 (o)",
            None,
        ),
        case(
            "N7: topic moved onto HEAD",
            oid(2),
            vec![oid(1)],
            &n7,
            &[][..],
            "Checkout topic",
            "Cherry-pick (o)",
            None,
        ),
        case(
            "merge commit",
            oid(3),
            vec![oid(2), oid(4)],
            &plain,
            &[][..],
            "Checkout feature",
            "Cherry-pick (o)",
            None,
        ),
        case(
            "no local branch",
            oid(9),
            vec![oid(2)],
            &plain,
            &[][..],
            "Checkout 0900000, detached",
            "Cherry-pick 0900000 (o)",
            None,
        ),
    ];
    for c in table {
        let slots = history_slots(
            c.row,
            &c.parents,
            "subject",
            c.refs,
            c.findings,
            graft_ui::keymap::Keymap::shipped(),
        );
        let got = labels(&slots.verbs);
        assert_eq!(
            slots
                .dim_first
                .as_ref()
                .map(|(label, reason)| (label.as_str(), reason.as_str())),
            c.dim.map(|reason| ("Checkout", reason)),
            "{}: {got:?}",
            c.name
        );
        if c.dim.is_some() {
            assert_eq!(slots.leading, 1, "{}", c.name);
            assert_eq!(got[0], c.first, "{}: {got:?}", c.name);
        } else {
            assert_eq!(slots.leading, 2, "{}", c.name);
            assert_eq!(&got[..2], &[c.first, c.second], "{}: {got:?}", c.name);
        }
    }
    // Two local branches: the second `Checkout` leads the rest.
    let slots = history_slots(
        oid(3),
        &[oid(2)],
        "subject",
        &two_on_three,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    assert_eq!(labels(&slots.verbs)[2], "Checkout topic");
}

/// m7-power-tools/11: slot 2 dims in place where `o` would refuse, in `o`'s
/// words — `HEAD`'s own row, a merge — decided from the row alone
/// (`PickParams::check_commit`). A plain commit off `HEAD` stays live, and
/// so does a stash row's Cherry-pick, which carries no `(o)`.
#[test]
fn slot_two_dims_on_head_and_on_a_merge() {
    let refs = main_and_feature();
    let dim_of = |row, parents: &[Oid]| {
        let slots = history_slots(
            row,
            parents,
            "subject",
            &refs,
            &[],
            graft_ui::keymap::Keymap::shipped(),
        );
        let (at, reason) = slots.dimmed.clone().expect("slot 2 is dim");
        (
            slots.verbs[at].label.as_str().to_owned(),
            reason.as_str().to_owned(),
        )
    };
    // HEAD's row: `main` only, so slot 1 is the dim Checkout and the pick is
    // the first verb.
    assert_eq!(
        dim_of(oid(2), &[oid(1)]),
        ("Cherry-pick (o)".to_owned(), "already on main".to_owned())
    );
    assert_eq!(
        dim_of(oid(3), &[oid(2), oid(4)]),
        (
            "Cherry-pick (o)".to_owned(),
            "a merge \u{b7} taking a merge isn't in Tributary yet".to_owned()
        )
    );
    // Already on the branch wins over a merge (`check_commit`'s order).
    assert_eq!(dim_of(oid(2), &[oid(1), oid(4)]).1, "already on main");

    let live = history_slots(
        oid(3),
        &[oid(2)],
        "subject",
        &refs,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    assert_eq!(live.dimmed, None);
    assert_eq!(labels(&live.verbs)[1], "Cherry-pick 0300000 (o)");
    let items = live.menu_items();
    assert!(items.iter().all(|(_, dim)| dim.is_none()), "{items:?}");

    // Detached on the row: `already on HEAD`.
    let detached = built(
        &[(name("refs/heads/main"), oid(2))],
        &HeadState::Detached(oid(9)),
    );
    let slots = history_slots(
        oid(9),
        &[oid(2)],
        "subject",
        &detached,
        &[],
        graft_ui::keymap::Keymap::shipped(),
    );
    assert_eq!(
        slots.dimmed.map(|(_, reason)| reason.as_str().to_owned()),
        Some("already on HEAD".to_owned())
    );
}
