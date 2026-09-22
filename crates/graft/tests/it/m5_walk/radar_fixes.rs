//! The Mistake Radar's three `Fix:` verbs, on the corpus's own foxtrot
//! merge: revert it keeping the branch's real previous tip, reset back to
//! that tip, or rebase onto it — the merge the user meant to make.
//!
//! A foxtrot merge is one on the default branch whose *first* parent is not
//! that branch's previous tip, so `--first-parent main` walks off `main`.
//! `Shape::FoxtrotMerge` — the shape the radar's own tests are written
//! against — is exactly that, with `main` checked out on the merge, so all
//! three verbs are offered on one row.

use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::claim::ghost_equals_git;
use super::fixture::{Fixture, Remote, foxtrot};
use super::steps::{Scenario, Step, Wait};
use super::truth::{Marks, Truth};
use super::vocabulary::{branch, chain, claim_and_confirm, last_entry_committed, undo};
use crate::repo_truth::check;

pub(super) fn fixing_a_foxtrot() -> Vec<Scenario> {
    vec![
        fix_revert_the_merge(),
        fix_reset_to_before_the_merge(),
        fix_rebase_instead(),
    ]
}

/// Everything up to the `.` menu: the rows, the refs **and the radar's
/// context**, then the cursor on `main`'s tip — the merge.
///
/// `Wait::RadarReady` is the step that makes these three scenarios about
/// the verbs rather than about timing: `.` builds its rows from the
/// findings visible at that moment, and `view::radar::findings_for` answers
/// "none" while the context is still loading, so an early `.` gets a menu
/// with no `Fix:` row in it and nothing on screen saying why.
fn at_the_foxtrot() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        Step::WaitFor(Wait::RadarReady),
        Step::CursorTo("main"),
    ]
}

/// The merge's second parent: `main`'s real previous tip, which all three
/// fixes are stated against. Read by running git off the id the mark holds,
/// never from the plan.
fn real_tip(fixture: &Fixture, before: &Truth) -> Result<String, String> {
    let merge = before
        .repo
        .head
        .as_deref()
        .ok_or_else(|| "no HEAD before".to_owned())?;
    Ok(fixture
        .git(&["rev-parse", &format!("{merge}^2")])?
        .trim()
        .to_owned())
}

/// The merge itself: `HEAD` before anything ran.
fn the_merge(before: &Truth) -> Result<&str, String> {
    before
        .repo
        .head
        .as_deref()
        .ok_or_else(|| "no HEAD before".to_owned())
}

fn fix_revert_the_merge() -> Scenario {
    Scenario {
        name: "fix-revert-the-merge",
        shape: Shape::FoxtrotMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: foxtrot,
        steps: chain(vec![
            at_the_foxtrot(),
            // `-m 2`: the mainline is the real previous tip's parent
            // number, so the revert keeps `main`'s own side.
            vec![Step::MenuPick("Fix: revert the merge, -m 2")],
            claim_and_confirm("the fix revert"),
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
                    .any(|line| line.starts_with("git revert ") && line.contains("-m 2")),
                || format!("the plan's commands are {:?}", claim.plan.plan.commands),
            )?;
            ghost_equals_git(claim, &marks.before, reverted, fixture)?;
            // Keeping `main`'s side means what the feature side brought is
            // gone and what `main` had stays.
            check(
                !reverted
                    .repo
                    .files
                    .contains_key(std::path::Path::new("feature.txt")),
                || "the revert left feature.txt in the tree".to_owned(),
            )?;
            for kept in ["base.txt", "main.txt"] {
                check(
                    reverted.repo.files.contains_key(std::path::Path::new(kept)),
                    || format!("the revert took {kept} out of the tree"),
                )?;
            }
            let parent = fixture.git(&["rev-parse", "main~1"])?;
            check(parent.trim() == the_merge(&marks.before)?, || {
                "the revert did not land on the merge it reverts".to_owned()
            })?;
            last_entry_committed(reverted, OperationKind::Revert)
        },
        capture: None,
    }
}

fn fix_reset_to_before_the_merge() -> Scenario {
    Scenario {
        name: "fix-reset-to-before-the-merge",
        shape: Shape::FoxtrotMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: foxtrot,
        steps: chain(vec![
            at_the_foxtrot(),
            vec![Step::MenuPick("Fix: reset main to before the merge")],
            claim_and_confirm("the fix reset"),
            vec![Step::Mark("reset")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            let reset = marks.get("reset")?;
            ghost_equals_git(marks.claim()?, &marks.before, reset, fixture)?;
            let tip = real_tip(fixture, &marks.before)?;
            check(branch(reset, "main")? == tip, || {
                format!(
                    "main is at {}, not the real previous tip {tip}",
                    branch(reset, "main").unwrap_or_default()
                )
            })?;
            check(
                !reset
                    .repo
                    .files
                    .contains_key(std::path::Path::new("feature.txt")),
                || "the hard reset left feature.txt in the tree".to_owned(),
            )?;
            last_entry_committed(reset, OperationKind::Reset)?;
            let undone = marks.get("undone")?;
            check(branch(undone, "main")? == the_merge(&marks.before)?, || {
                "u did not put main back on the merge".to_owned()
            })
        },
        capture: None,
    }
}

fn fix_rebase_instead() -> Scenario {
    Scenario {
        name: "fix-rebase-instead",
        shape: Shape::FoxtrotMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: foxtrot,
        steps: chain(vec![
            at_the_foxtrot(),
            vec![Step::MenuPick("Fix: rebase instead")],
            claim_and_confirm("the fix rebase"),
            vec![Step::Mark("rebased")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| rebase_instead_holds(fixture, marks),
        capture: None,
    }
}

/// `main` replayed onto its real previous tip: the merge is gone, the work
/// the feature side brought is a plain commit on top of `main`'s own line,
/// and `u` puts the merge back.
///
/// **The claim is judged last, on purpose.** `ghost_equals_git` is the
/// broadest check here — it was the one that failed when m5/38 wrote this
/// scenario (a rebase whose range contains a merge drew a ghost for that
/// merge, which `git rebase` without `--rebase-merges` drops; fixed in
/// m5/43). Put first, it would hide every other check behind itself — the
/// m5/32 lesson, where three assertions sat unreachable under a failing
/// line and read the wrong history the moment they became reachable.
fn rebase_instead_holds(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let rebased = marks.get("rebased")?;
    let new_tip = branch(rebased, "main")?;
    let parents = fixture.git(&["log", "-1", "--format=%P", &new_tip])?;
    let parents: Vec<&str> = parents.split_whitespace().collect();
    let tip = real_tip(fixture, &marks.before)?;
    check(parents == [tip.as_str()], || {
        format!("the replayed tip's parents are {parents:?}, not just {tip}")
    })?;
    // The work is still there, on a line that no longer forks.
    for kept in ["base.txt", "main.txt", "feature.txt"] {
        check(
            rebased.repo.files.contains_key(std::path::Path::new(kept)),
            || format!("the rebase took {kept} out of the tree"),
        )?;
    }
    last_entry_committed(rebased, OperationKind::Rebase)?;
    let undone = marks.get("undone")?;
    check(branch(undone, "main")? == the_merge(&marks.before)?, || {
        "u did not put main back on the merge".to_owned()
    })?;
    ghost_equals_git(marks.claim()?, &marks.before, rebased, fixture)
}
