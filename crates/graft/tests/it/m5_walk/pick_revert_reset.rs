//! The `.` menu's verbs: cherry-pick, revert, reset hard and mixed,
//! checkout of a branch and of a commit — each previewed, each judged by
//! `ghost_equals_git`, the destructive one brought back by `u`.

use graft_core::event::KeyCode;
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::claim::ghost_equals_git;
use super::fixture::{Fixture, Remote, forked_on_main, merged_on_main};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{
    box_says, branch, ch, chain, claim_and_confirm, last_entry_committed, plain, undo,
};
use crate::repo_truth::check;

pub(super) fn picking_reverting_resetting() -> Vec<Scenario> {
    vec![
        cherry_pick_via_menu_ghost_equals_git(),
        revert_via_menu_ghost_equals_git(),
        revert_merge_uses_mainline(),
        reset_hard_says_loses_and_u_brings_it_back(),
        reset_mixed_keeps_the_tree(),
        checkout_branch_via_menu(),
        checkout_detached_via_menu(),
    ]
}

fn cherry_pick_via_menu_ghost_equals_git() -> Scenario {
    Scenario {
        name: "cherry-pick-via-menu-ghost-equals-git",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorToSubject("finish the feature"),
                Step::MenuPick("Cherry-pick"),
            ],
            claim_and_confirm("the cherry-pick"),
            vec![Step::Mark("picked")],
        ]),
        expect: cherry_pick_holds,
        capture: None,
    }
}

fn revert_via_menu_ghost_equals_git() -> Scenario {
    Scenario {
        name: "revert-via-menu-ghost-equals-git",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorToSubject("main moves again"),
                Step::MenuPick("Revert"),
            ],
            claim_and_confirm("the revert"),
            vec![Step::Mark("reverted")],
        ]),
        expect: |fixture, marks| {
            let reverted = marks.get("reverted")?;
            let claim = marks.claim()?;
            check(claim.plan.plan.ghosts.len() == 1, || {
                "one ghost for one revert".to_owned()
            })?;
            ghost_equals_git(claim, &marks.before, reverted, fixture)?;
            check(
                !reverted
                    .repo
                    .files
                    .contains_key(std::path::Path::new("main-two.txt")),
                || "the revert left main-two.txt in the tree".to_owned(),
            )?;
            let parent = fixture.git(&["rev-parse", "main~1"])?;
            check(parent.trim() == branch(&marks.before, "main")?, || {
                "the revert did not land on main's old tip".to_owned()
            })?;
            last_entry_committed(reverted, OperationKind::Revert)
        },
        capture: None,
    }
}

/// `.` on a merge commit: git refuses `revert` on one without being told
/// which side to keep, so the menu row carries the mainline it will pass —
/// `Revert <sha7>, -m 1`, the first parent, the branch the merge landed on
/// (`context_menu::verbs::revert_verb`). The ghost's patch is the merge's,
/// reversed against that same parent, which is what `ghost_equals_git`
/// checks for a revert.
fn revert_merge_uses_mainline() -> Scenario {
    Scenario {
        name: "revert-merge-uses-mainline",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: merged_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("main"),
                Step::MenuPick("Revert "),
            ],
            claim_and_confirm("the revert of the merge"),
            vec![Step::Mark("reverted")],
        ]),
        expect: |fixture, marks| {
            let reverted = marks.get("reverted")?;
            let claim = marks.claim()?;
            check(
                claim
                    .plan
                    .plan
                    .commands
                    .iter()
                    .any(|line| line.starts_with("git revert ") && line.contains("-m 1")),
                || format!("the plan's commands are {:?}", claim.plan.plan.commands),
            )?;
            check(claim.plan.plan.ghosts.len() == 1, || {
                "one ghost for one revert".to_owned()
            })?;
            ghost_equals_git(claim, &marks.before, reverted, fixture)?;
            // `-m 1` keeps the first parent's side: what `feature` brought
            // goes, what `main` had stays.
            for gone in ["work-one.txt", "work-two.txt"] {
                check(
                    !reverted.repo.files.contains_key(std::path::Path::new(gone)),
                    || format!("the revert left {gone} in the tree"),
                )?;
            }
            for kept in ["main-one.txt", "main-two.txt", "base.txt"] {
                check(
                    reverted.repo.files.contains_key(std::path::Path::new(kept)),
                    || format!("the revert took {kept} out of the tree"),
                )?;
            }
            let parent = fixture.git(&["rev-parse", "main~1"])?;
            check(parent.trim() == branch(&marks.before, "main")?, || {
                "the revert did not land on the merge it reverts".to_owned()
            })?;
            last_entry_committed(reverted, OperationKind::Revert)
        },
        capture: None,
    }
}

fn reset_hard_says_loses_and_u_brings_it_back() -> Scenario {
    Scenario {
        name: "reset-hard-says-loses-and-u-brings-it-back",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        // An uncommitted edit to a tracked file: what `--hard` throws away.
        setup: |fixture| {
            forked_on_main(fixture)?;
            fixture.dirty("base.txt", "edited, not committed\n")
        },
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorToSubject("main moves on"),
                Step::MenuPick("Reset main to here, hard"),
                Step::WaitFor(Wait::PreviewReady),
                Step::See("the box says what the tree loses", |state| {
                    box_says(state, "working tree: LOSES base.txt (backup saved)")
                }),
            ],
            claim_and_confirm("the reset"),
            vec![Step::Mark("reset")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: reset_hard_holds,
        capture: Some(Capture {
            stem: "m5-03-preview-reset-hard",
            headless: vec![],
            // `main moves on` is the fourth row of the forked history; the
            // menu lists the hard reset after the mixed one.
            // `.` on `main moves on`, then down the menu to `Reset main to
            // here, hard` (the tenth item on a non-HEAD row) and `⏎`.
            keys: reset_hard_keys(),
            on_screen: "LOSES base.txt",
            when: When::Settled,
        }),
    }
}

fn reset_mixed_keeps_the_tree() -> Scenario {
    Scenario {
        name: "reset-mixed-keeps-the-tree",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorToSubject("main moves on"),
                Step::MenuPick("Reset main to here, mixed"),
            ],
            claim_and_confirm("the reset"),
            vec![Step::Mark("reset")],
        ]),
        expect: |fixture, marks| {
            let reset = marks.get("reset")?;
            let claim = marks.claim()?;
            ghost_equals_git(claim, &marks.before, reset, fixture)?;
            check(reset.repo.files == marks.before.repo.files, || {
                "a mixed reset changed the tree".to_owned()
            })?;
            let status = fixture.git(&["status", "--porcelain"])?;
            check(
                status.lines().any(|line| line.ends_with("main-two.txt")),
                || format!("main-two.txt is not left unstaged: {status:?}"),
            )?;
            last_entry_committed(reset, OperationKind::Reset)
        },
        capture: None,
    }
}

fn checkout_branch_via_menu() -> Scenario {
    Scenario {
        name: "checkout-branch-via-menu",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("feature"),
                Step::MenuPick("Checkout feature"),
            ],
            claim_and_confirm("the checkout"),
            vec![Step::Mark("switched")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            let switched = marks.get("switched")?;
            let claim = marks.claim()?;
            check(claim.plan.plan.ref_moves.is_empty(), || {
                "a checkout moved a ref".to_owned()
            })?;
            check(
                switched.head_symbolic.as_deref() == Some("refs/heads/feature"),
                || format!("HEAD is {:?}", switched.head_symbolic),
            )?;
            check(switched.repo.refs == marks.before.repo.refs, || {
                "a checkout moved a ref".to_owned()
            })?;
            check(
                switched
                    .repo
                    .files
                    .get(std::path::Path::new("work-two.txt"))
                    == Some(&b"two\n".to_vec()),
                || {
                    format!(
                        "the tree is not feature's: {:?}; status {:?}",
                        switched.repo.files.keys().collect::<Vec<_>>(),
                        fixture.git(&["status", "--porcelain"])
                    )
                },
            )?;
            last_entry_committed(switched, OperationKind::Checkout)?;
            let undone = marks.get("undone")?;
            check(
                undone.head_symbolic.as_deref() == Some("refs/heads/main"),
                || format!("u left HEAD on {:?}", undone.head_symbolic),
            )
        },
        capture: None,
    }
}

fn checkout_detached_via_menu() -> Scenario {
    Scenario {
        name: "checkout-detached-via-menu",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorToSubject("add the base"),
                Step::MenuPick("Checkout "),
            ],
            claim_and_confirm("the checkout"),
            vec![Step::Mark("detached")],
        ]),
        expect: |fixture, marks| {
            let detached = marks.get("detached")?;
            check(detached.head_symbolic.is_none(), || {
                format!("HEAD is still on {:?}", detached.head_symbolic)
            })?;
            let base = fixture.git(&[
                "rev-parse",
                &format!("{}~2", branch(&marks.before, "main")?),
            ])?;
            check(detached.repo.head.as_deref() == Some(base.trim()), || {
                format!("HEAD is at {:?}, not the base", detached.repo.head)
            })?;
            check(detached.repo.refs == marks.before.repo.refs, || {
                "a detached checkout moved a ref".to_owned()
            })?;
            last_entry_committed(detached, OperationKind::Checkout)
        },
        capture: None,
    }
}

/// The pick's ghost equals git: one new commit on `main` with the picked
/// patch, the tree is the pick's, the entry is a committed `CherryPick`.
fn cherry_pick_holds(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let picked = marks.get("picked")?;
    let claim = marks.claim()?;
    check(
        claim.plan.plan.ghosts.len() == 1 && claim.plan.plan.ghosts[0].replays.is_some(),
        || "one ghost replaying the picked commit".to_owned(),
    )?;
    ghost_equals_git(claim, &marks.before, picked, fixture)?;
    let parent = fixture.git(&["rev-parse", "main~1"])?;
    check(parent.trim() == branch(&marks.before, "main")?, || {
        "the pick did not land on main's old tip".to_owned()
    })?;
    check(
        fixture.bytes("work-two.txt")? == b"two\n"
            && !picked
                .repo
                .files
                .contains_key(std::path::Path::new("work-one.txt")),
        || "the tree is not the pick's: work-two.txt only".to_owned(),
    )?;
    last_entry_committed(picked, OperationKind::CherryPick)
}

/// `.` on `main moves on` (the fourth row of the forked history), then
/// down the menu to `Reset main to here, hard` — the tenth item on a
/// non-HEAD row — and `⏎`.
fn reset_hard_keys() -> Vec<graft_core::event::KeyPress> {
    plain(&[
        ch('j'),
        ch('j'),
        ch('j'),
        ch('.'),
        ch('j'),
        ch('j'),
        ch('j'),
        ch('j'),
        ch('j'),
        ch('j'),
        ch('j'),
        ch('j'),
        ch('j'),
        KeyCode::Enter,
    ])
}

/// The hard reset's ghost equals git, the tree lost what the box said,
/// and `u` brings the branch, the file and the uncommitted edit back.
fn reset_hard_holds(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let reset = marks.get("reset")?;
    let claim = marks.claim()?;
    ghost_equals_git(claim, &marks.before, reset, fixture)?;
    let target = fixture.git(&[
        "rev-parse",
        &format!("{}~1", branch(&marks.before, "main")?),
    ])?;
    check(branch(reset, "main")? == target.trim(), || {
        "main is not at `main moves on`".to_owned()
    })?;
    check(
        !reset
            .repo
            .files
            .contains_key(std::path::Path::new("main-two.txt")),
        || "reset --hard left main-two.txt".to_owned(),
    )?;
    check(
        reset.repo.files.get(std::path::Path::new("base.txt")) == Some(&b"base\n".to_vec()),
        || "reset --hard kept the uncommitted edit".to_owned(),
    )?;
    last_entry_committed(reset, OperationKind::Reset)?;
    let undone = marks.get("undone")?;
    check(
        branch(undone, "main")? == branch(&marks.before, "main")?,
        || "u did not bring main back".to_owned(),
    )?;
    check(
        undone.repo.files.get(std::path::Path::new("main-two.txt"))
            == marks
                .before
                .repo
                .files
                .get(std::path::Path::new("main-two.txt")),
        || "u did not bring main-two.txt's bytes back".to_owned(),
    )?;
    check(
        undone.repo.files.get(std::path::Path::new("base.txt"))
            == Some(&b"edited, not committed\n".to_vec()),
        || "u did not bring the uncommitted edit back".to_owned(),
    )
}
