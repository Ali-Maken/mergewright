//! Merge and rebase: the ghost equals what git did, for a fast-forward, a
//! merge commit and two replayed commits; a forecast names the file that
//! would conflict; a dirty tree is refused with the stash key named; the
//! drop menu ends in the same preview.

use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::claim::{ghost_equals_git, patch_id};
use super::fixture::{Fixture, conflicting, fast_forwardable, forked, forked_on_main};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::vocabulary::{
    box_says, branch, ch, chain, claim_and_confirm, key, last_entry_committed, plain, redo,
    same_files, status_containing, undo,
};
use crate::repo_truth::check;

pub(super) fn merging() -> Vec<Scenario> {
    vec![
        merge_ff_preview_equals_git(),
        merge_no_ff_ghost_equals_git(),
        merge_forecast_names_the_conflicting_path(),
    ]
}

pub(super) fn rebasing() -> Vec<Scenario> {
    vec![
        rebase_two_commits_ghost_equals_git(),
        rebase_dirty_tree_refused_offers_stash(),
        rebase_via_drop_menu(),
    ]
}

fn merge_ff_preview_equals_git() -> Scenario {
    Scenario {
        name: "merge-ff-preview-equals-git",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: fast_forwardable,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("feature"),
                key(ch('m')),
            ],
            claim_and_confirm("the merge"),
            vec![Step::Mark("merged")],
        ]),
        expect: |fixture, marks| {
            let merged = marks.get("merged")?;
            let claim = marks.claim()?;
            check(claim.plan.plan.ghosts.is_empty(), || {
                "a fast-forward drew a ghost".to_owned()
            })?;
            ghost_equals_git(claim, &marks.before, merged, fixture)?;
            check(
                branch(merged, "main")? == branch(&marks.before, "feature")?,
                || "main is not at feature's tip".to_owned(),
            )?;
            last_entry_committed(merged, OperationKind::Merge)
        },
        capture: None,
    }
}

fn merge_no_ff_ghost_equals_git() -> Scenario {
    Scenario {
        name: "merge-no-ff-ghost-equals-git",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("feature"),
                key(ch('m')),
            ],
            claim_and_confirm("the merge"),
            vec![Step::Mark("merged")],
            undo(),
            vec![Step::Mark("undone")],
            redo(),
            vec![Step::Mark("redone")],
        ]),
        expect: |fixture, marks| {
            let merged = marks.get("merged")?;
            let claim = marks.claim()?;
            check(claim.plan.plan.ghosts.len() == 1, || {
                format!(
                    "{} ghosts drawn for one merge",
                    claim.plan.plan.ghosts.len()
                )
            })?;
            ghost_equals_git(claim, &marks.before, merged, fixture)?;
            let parents = fixture.git(&["log", "-1", "--format=%P", "main"])?;
            let parents: Vec<&str> = parents.split_whitespace().collect();
            check(
                parents
                    == [
                        branch(&marks.before, "main")?.as_str(),
                        branch(&marks.before, "feature")?.as_str(),
                    ],
                || format!("main's parents are {parents:?}"),
            )?;
            last_entry_committed(merged, OperationKind::Merge)?;
            let undone = marks.get("undone")?;
            check(
                branch(undone, "main")? == branch(&marks.before, "main")?,
                || "u did not bring main back".to_owned(),
            )?;
            same_files("u", &marks.before, undone)?;
            let redone = marks.get("redone")?;
            check(branch(redone, "main")? == branch(merged, "main")?, || {
                "U did not bring the merge back".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m5-02-preview-merge",
            headless: vec![],
            keys: plain(&[ch('m')]),
            on_screen: "Preview: merge",
            when: When::Settled,
        }),
    }
}

fn merge_forecast_names_the_conflicting_path() -> Scenario {
    Scenario {
        name: "merge-forecast-names-the-conflicting-path",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: conflicting,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("feature"),
            key(ch('m')),
            Step::WaitFor(Wait::PreviewReady),
            Step::See("the box names shared.txt as conflicting", |state| {
                box_says(state, "may conflict")?;
                box_says(state, "shared.txt")
            }),
            key(graft_core::event::KeyCode::Escape),
            Step::Mark("closed"),
        ],
        expect: |_, marks| {
            let closed = marks.get("closed")?;
            check(closed.repo.refs == marks.before.repo.refs, || {
                "the forecast moved a ref".to_owned()
            })?;
            check(closed.in_progress.is_none(), || {
                format!("git is mid-{:?} after a forecast", closed.in_progress)
            })
        },
        capture: None,
    }
}

fn rebase_two_commits_ghost_equals_git() -> Scenario {
    Scenario {
        name: "rebase-two-commits-ghost-equals-git",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("main"),
                key(ch('r')),
            ],
            claim_and_confirm("the rebase"),
            vec![Step::Mark("rebased")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| rebase_holds(fixture, marks),
        capture: None,
    }
}

/// Two ghosts with patch-ids, `feature` on the newest, its parent chain
/// ending at `main`'s tip; `u` brings the branch and the tree back.
fn rebase_holds(fixture: &Fixture, marks: &super::truth::Marks) -> Result<(), String> {
    let rebased = marks.get("rebased")?;
    let claim = marks.claim()?;
    check(claim.plan.plan.ghosts.len() == 2, || {
        format!(
            "{} ghosts drawn for two commits",
            claim.plan.plan.ghosts.len()
        )
    })?;
    check(
        claim
            .plan
            .plan
            .ghosts
            .iter()
            .all(|ghost| ghost.replays.is_some()),
        || "a rebase ghost replays nothing".to_owned(),
    )?;
    ghost_equals_git(claim, &marks.before, rebased, fixture)?;
    // The rebased branch is named by the **oid the `rebased` mark holds**,
    // never by `feature`: `u` has already put that name back, so every
    // `feature~n` here would read the history the rebase replaced. Objects
    // outlive the ref, so the oid still resolves (m5/32: these three checks
    // were unreachable while the working-tree line failed above, and read
    // the undone history the moment it stopped failing).
    let new_tip = branch(rebased, "feature")?;
    let onto = fixture.git(&["rev-parse", &format!("{new_tip}~2")])?;
    check(onto.trim() == branch(&marks.before, "main")?, || {
        format!("the rebased branch's ~2 is {}, not main's tip", onto.trim())
    })?;
    let old_tip = branch(&marks.before, "feature")?;
    let old_first = fixture.git(&["rev-parse", &format!("{old_tip}~1")])?;
    let new_first = fixture.git(&["rev-parse", &format!("{new_tip}~1")])?;
    for (new, old) in [
        (new_tip.as_str(), old_tip.as_str()),
        (new_first.trim(), old_first.trim()),
    ] {
        check(
            patch_id(fixture, new, false)? == patch_id(fixture, old, false)?,
            || format!("{new}'s patch-id is not {old}'s"),
        )?;
    }
    last_entry_committed(rebased, OperationKind::Rebase)?;
    let undone = marks.get("undone")?;
    check(
        branch(undone, "feature")? == branch(&marks.before, "feature")?,
        || "u did not bring feature back".to_owned(),
    )?;
    same_files("u", &marks.before, undone)
}

fn rebase_dirty_tree_refused_offers_stash() -> Scenario {
    Scenario {
        name: "rebase-dirty-tree-refused-offers-stash",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: |fixture| {
            forked(fixture)?;
            fixture.dirty("base.txt", "edited, not committed\n")
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('r')),
            Step::WaitFor(Wait::PreviewReady),
            Step::Confirm,
            Step::Settle("the refused rebase"),
            Step::See("the status names the clean-tree rule and S", |state| {
                status_containing(state, "rebase needs a clean tree")?;
                status_containing(state, "S stash 1 changed file")
            }),
            Step::Mark("refused"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            check(refused.repo.refs == marks.before.repo.refs, || {
                "the refusal moved a ref".to_owned()
            })?;
            same_files("the refusal", &marks.before, refused)?;
            check(
                fixture.bytes("base.txt")? == b"edited, not committed\n",
                || "the dirty edit is gone".to_owned(),
            )?;
            check(refused.in_progress.is_none(), || {
                "git is mid-rebase after a refusal".to_owned()
            })
        },
        capture: None,
    }
}

fn rebase_via_drop_menu() -> Scenario {
    Scenario {
        name: "rebase-via-drop-menu",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::DragBadgeOnto("main"),
                Step::See("the drop menu has the rebase row", |state| {
                    let labels: Vec<String> = state
                        .context_menu
                        .as_ref()
                        .ok_or_else(|| "no menu".to_owned())?
                        .menu
                        .items
                        .iter()
                        .map(|item| item.label.to_string())
                        .collect();
                    check(
                        labels.iter().any(|label| label.starts_with("Rebase")),
                        || format!("no Rebase row: {labels:?}"),
                    )
                }),
            ],
            vec![Step::MenuPickOpen("Rebase")],
            claim_and_confirm("the rebase"),
            vec![Step::Mark("rebased")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| rebase_holds(fixture, marks),
        capture: None,
    }
}
