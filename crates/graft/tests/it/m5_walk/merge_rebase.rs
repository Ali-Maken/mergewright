//! Merge and rebase: the ghost equals what git did, for a fast-forward, a
//! merge commit and two replayed commits; a forecast names the file that
//! would conflict; a dirty tree opens the preview (its edits are set aside,
//! m6-conflicts/73); the drop menu ends in the same preview.

use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::claim::{ghost_equals_git, patch_id};
use super::fixture::{
    Fixture, Remote, already_up_to_date, conflicting, fast_forwardable, forked, forked_on_main,
};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::vocabulary::{
    box_choices, box_says, branch, ch, chain, claim_and_confirm, key, last_entry_committed,
    merge_clashes, nothing_open, plain, redo, same_files, same_refs, status_containing,
    title_names, undo,
};
use crate::repo_truth::check;

pub(super) fn merging() -> Vec<Scenario> {
    vec![
        merge_ff_preview_equals_git(),
        merge_no_ff_ghost_equals_git(),
        merge_forecast_names_the_conflicting_path(),
        merge_already_up_to_date_offers_close(),
    ]
}

pub(super) fn rebasing() -> Vec<Scenario> {
    vec![
        rebase_two_commits_ghost_equals_git(),
        rebase_dirty_tree_opens_the_preview(),
        rebase_via_drop_menu(),
    ]
}

fn merge_ff_preview_equals_git() -> Scenario {
    Scenario {
        name: "merge-ff-preview-equals-git",
        shape: Shape::Trivial,
        remote: Remote::None,
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
        remote: Remote::None,
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
            on_screen: "Merge feature into main",
            when: When::Settled,
        }),
    }
}

fn merge_forecast_names_the_conflicting_path() -> Scenario {
    Scenario {
        name: "merge-forecast-names-the-conflicting-path",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: conflicting,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("feature"),
            key(ch('m')),
            Step::WaitFor(Wait::PreviewReady),
            Step::See("the box names shared.txt as a clash", |state| {
                merge_clashes(state, "shared.txt")
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

/// `m` on a branch that is already an ancestor: the box says `nothing to
/// do` and offers **only** `Close`. An enabled `Confirm` that does nothing
/// is how a user learns to stop reading the box
/// (`update::preview::nothing_to_do`).
///
/// This is the one merge scenario `ghost_equals_git` cannot judge: the plan
/// moves no ref, and that function's first line refuses a plan with nothing
/// to judge. The refs are asserted here instead.
fn merge_already_up_to_date_offers_close() -> Scenario {
    Scenario {
        name: "merge-already-up-to-date-offers-close",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: already_up_to_date,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("feature"),
            key(ch('m')),
            Step::WaitFor(Wait::PreviewReady),
            Step::See("the box says there is nothing to do", |state| {
                box_says(state, "nothing to do")
            }),
            Step::See("the only button is Close", |state| {
                let choices = box_choices(state)?;
                check(choices == ["Close"], || {
                    format!("the buttons are {choices:?}")
                })
            }),
            Step::Claim,
            // `→` then `⏎`: on a one-button box `→` has nowhere to go, and
            // `⏎` closes rather than confirming.
            Step::Confirm,
            Step::Settle("Enter on the Close box"),
            Step::See("the box is gone and nothing ran", nothing_open),
            Step::Mark("closed"),
        ],
        expect: |_, marks| {
            check(marks.claim()?.plan.plan.ref_moves.is_empty(), || {
                "an up-to-date merge planned a ref move".to_owned()
            })?;
            check(marks.claim()?.plan.plan.ghosts.is_empty(), || {
                "an up-to-date merge drew a ghost".to_owned()
            })?;
            let closed = marks.get("closed")?;
            same_refs("the up-to-date merge", &marks.before, closed)?;
            same_files("the up-to-date merge", &marks.before, closed)?;
            check(closed.in_progress.is_none(), || {
                format!(
                    "git says {:?} after a box that ran nothing",
                    closed.in_progress
                )
            })?;
            check(closed.journal.is_empty(), || {
                format!("the box journaled {} entries", closed.journal.len())
            })
        },
        capture: None,
    }
}

fn rebase_two_commits_ghost_equals_git() -> Scenario {
    Scenario {
        name: "rebase-two-commits-ghost-equals-git",
        shape: Shape::Trivial,
        remote: Remote::None,
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
            patch_id(fixture, new, None)? == patch_id(fixture, old, None)?,
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

/// `r` over a changed tracked file opens the preview: since
/// m6-conflicts/73 a rebase sets the edits aside and puts them back, so
/// there is no clean-tree refusal (ADR-0045). What the box says about the
/// set-aside is 74's; this only asserts the box opens, and `Esc` leaves
/// everything as it was.
fn rebase_dirty_tree_opens_the_preview() -> Scenario {
    Scenario {
        name: "rebase-dirty-tree-opens-the-preview",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: |fixture| {
            forked(fixture)?;
            fixture.dirty("base.txt", "edited, not committed\n")
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('r')),
            // Answered either way, so a refusal fails on the box below with
            // the status in the message, not on a pump timeout.
            Step::WaitFor(Wait::PreviewAnswered),
            Step::See("the rebase preview is open, with Confirm", |state| {
                title_names(state, "rebase")?;
                let choices = box_choices(state)?;
                check(choices.iter().any(|choice| choice == "Confirm"), || {
                    format!("the box offers {choices:?}")
                })
            }),
            Step::See("no clean-tree refusal", |state| {
                check(status_containing(state, "clean tree").is_err(), || {
                    format!("the status still refuses: {:?}", state.status)
                })
            }),
            key(graft_core::event::KeyCode::Escape),
            Step::Mark("closed"),
        ],
        expect: |fixture, marks| {
            let closed = marks.get("closed")?;
            check(closed.repo.refs == marks.before.repo.refs, || {
                "the preview moved a ref".to_owned()
            })?;
            same_files("the preview", &marks.before, closed)?;
            check(
                fixture.bytes("base.txt")? == b"edited, not committed\n",
                || "the dirty edit is gone".to_owned(),
            )?;
            check(closed.in_progress.is_none(), || {
                "git is mid-rebase after a preview".to_owned()
            })
        },
        capture: None,
    }
}

fn rebase_via_drop_menu() -> Scenario {
    Scenario {
        name: "rebase-via-drop-menu",
        shape: Shape::Trivial,
        remote: Remote::None,
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
