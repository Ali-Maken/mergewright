//! The refs-and-names group: `b` and `t` from the graph, a bad or taken
//! name refused in the box before `Enter` does anything, `u` on a branch or
//! tag delete, `b` from the stash overlay, and moving a branch that is not
//! checked out by dragging its badge.
//!
//! A create moves no history (`update::name_prompt`, "Why this exists"), so
//! `b` and `t` go straight to `Command::Operate` with no preview — those
//! two, and the refusal scenario (which writes nothing at all), take no
//! `Step::Claim` and assert refs themselves. A delete, `stash branch` and
//! moving a branch all move history, so they get the Ghost Preview like
//! every other write and `ghost_equals_git` judges the ref move.

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::journal::OperationKind;
use mergewright_fixtures::Shape;

use super::claim::ghost_equals_git;
use super::fixture::{Fixture, Remote, forked, forked_on_main};
use super::steps::{Scenario, Step, Wait};
use super::truth::Marks;
use super::vocabulary::{
    branch, ch, chain, claim_and_confirm, key, last_entry_committed, same_files, same_refs, undo,
};

pub(super) fn refs_and_names() -> Vec<Scenario> {
    vec![
        b_creates_and_checks_out(),
        b_refuses_bad_and_existing_names(),
        t_creates_a_tag(),
        delete_branch_then_u_restores(),
        delete_tag_then_u_restores(),
        stash_b_branches_from_stash(),
        move_a_branch_not_checked_out(),
    ]
}

/// [`forked`], then a lightweight tag `v1` on `main`'s tip — for the
/// scenario that deletes a tag.
fn forked_with_tag(fixture: &Fixture) -> Result<(), String> {
    forked(fixture)?;
    fixture.git(&["tag", "v1", "main"]).map(drop)
}

/// One commit and a tracked file dirtied and stashed — `git stash list`
/// holds one entry, `wip`.
fn with_a_stash(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("sample.txt", "one\n", "add sample", 1)?;
    fixture.dirty("sample.txt", "one\ntwo\n")?;
    fixture.git(&["stash", "push", "-m", "wip"]).map(drop)
}

/// Nothing moved between setup and `mark`: refs and files alike.
fn nothing_ran(marks: &Marks, mark: &str) -> Result<(), String> {
    let after = marks.get(mark)?;
    same_refs("nothing ran", &marks.before, after)?;
    same_files("nothing ran", &marks.before, after)
}

fn b_creates_and_checks_out() -> Scenario {
    Scenario {
        name: "b-creates-and-checks-out",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('b')),
            Step::WaitFor(Wait::Prompt),
            Step::Type("release"),
            // Asks for the checkout as well.
            key(KeyCode::Tab),
            key(KeyCode::Enter),
            Step::Write("the branch create"),
            Step::WaitFor(Wait::Ready),
            Step::Mark("created"),
        ],
        expect: |_, marks| {
            let created = marks.get("created")?;
            crate::repo_truth::check(
                branch(created, "release")? == branch(&marks.before, "main")?,
                || "release is not at main's commit".to_owned(),
            )?;
            crate::repo_truth::check(
                created.head_symbolic.as_deref() == Some("refs/heads/release"),
                || format!("HEAD is not on release: {:?}", created.head_symbolic),
            )?;
            crate::repo_truth::check(
                created.journal.len() == marks.before.journal.len() + 1,
                || {
                    format!(
                        "journal grew by {}, not 1",
                        created.journal.len() - marks.before.journal.len()
                    )
                },
            )?;
            last_entry_committed(created, OperationKind::BranchCreate)?;
            crate::repo_truth::check(
                created
                    .journal
                    .last()
                    .is_some_and(|entry| entry.description.contains("release")),
                || {
                    format!(
                        "the entry does not name release: {:?}",
                        created.journal.last()
                    )
                },
            )
        },
        capture: None,
    }
}

fn b_refuses_bad_and_existing_names() -> Scenario {
    Scenario {
        name: "b-refuses-bad-and-existing-names",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('b')),
            Step::WaitFor(Wait::Prompt),
            Step::Type("bad name"),
            Step::See("a space is not a valid ref name", |state| {
                let refusal = mergewright::update::name_prompt::name_refusal(state);
                crate::repo_truth::check(
                    refusal
                        .as_deref()
                        .is_some_and(|why| why.starts_with("bad name:")),
                    || format!("refusal is {refusal:?}"),
                )
            }),
            Step::Key(KeyPress::control(KeyCode::Char('u'))),
            Step::Type("main"),
            Step::See("main is already taken", |state| {
                let refusal = mergewright::update::name_prompt::name_refusal(state);
                crate::repo_truth::check(refusal.as_deref() == Some("exists"), || {
                    format!("refusal is {refusal:?}")
                })
            }),
            key(KeyCode::Escape),
            Step::See("Esc closed the prompt", |state| {
                crate::repo_truth::check(state.name_prompt.is_none(), || {
                    "the prompt is still open".to_owned()
                })
            }),
            Step::Mark("closed"),
        ],
        expect: |_, marks| nothing_ran(marks, "closed"),
        capture: None,
    }
}

fn t_creates_a_tag() -> Scenario {
    Scenario {
        name: "t-creates-a-tag",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("feature"),
            key(ch('t')),
            Step::WaitFor(Wait::Prompt),
            Step::Type("v2.0.0"),
            key(KeyCode::Enter),
            Step::Write("the tag create"),
            Step::WaitFor(Wait::Ready),
            Step::Mark("created"),
        ],
        expect: |fixture, marks| {
            let created = marks.get("created")?;
            let tagged = created
                .repo
                .refs
                .get("refs/tags/v2.0.0")
                .cloned()
                .ok_or_else(|| format!("no refs/tags/v2.0.0: {:?}", created.repo.refs))?;
            crate::repo_truth::check(tagged == branch(&marks.before, "feature")?, || {
                "v2.0.0 is not at feature's commit".to_owned()
            })?;
            let kind = fixture.git(&["cat-file", "-t", "refs/tags/v2.0.0"])?;
            crate::repo_truth::check(kind.trim() == "commit", || {
                format!(
                    "a lightweight tag should name a commit, names {}",
                    kind.trim()
                )
            })?;
            crate::repo_truth::check(
                created.journal.len() == marks.before.journal.len() + 1,
                || "journal did not grow by one".to_owned(),
            )?;
            last_entry_committed(created, OperationKind::TagCreate)
        },
        capture: None,
    }
}

fn delete_branch_then_u_restores() -> Scenario {
    Scenario {
        name: "delete-branch-then-u-restores",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("main"),
                Step::MenuPick("Delete branch main"),
            ],
            claim_and_confirm("the branch delete"),
            vec![Step::Mark("deleted")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            let claim = marks.claim()?;
            let deleted = marks.get("deleted")?;
            ghost_equals_git(claim, &marks.before, deleted, fixture)?;
            crate::repo_truth::check(!deleted.repo.refs.contains_key("refs/heads/main"), || {
                "main is still there after the delete".to_owned()
            })?;
            last_entry_committed(deleted, OperationKind::BranchDelete)?;
            let undone = marks.get("undone")?;
            crate::repo_truth::check(
                undone.repo.refs.get("refs/heads/main")
                    == marks.before.repo.refs.get("refs/heads/main"),
                || "u did not restore main to its old commit".to_owned(),
            )
        },
        capture: None,
    }
}

fn delete_tag_then_u_restores() -> Scenario {
    Scenario {
        name: "delete-tag-then-u-restores",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_with_tag,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("v1"),
                Step::MenuPick("Delete tag v1"),
            ],
            claim_and_confirm("the tag delete"),
            vec![Step::Mark("deleted")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            let claim = marks.claim()?;
            let deleted = marks.get("deleted")?;
            ghost_equals_git(claim, &marks.before, deleted, fixture)?;
            crate::repo_truth::check(!deleted.repo.refs.contains_key("refs/tags/v1"), || {
                "v1 is still there after the delete".to_owned()
            })?;
            last_entry_committed(deleted, OperationKind::TagDelete)?;
            let undone = marks.get("undone")?;
            crate::repo_truth::check(
                undone.repo.refs.get("refs/tags/v1") == marks.before.repo.refs.get("refs/tags/v1"),
                || "u did not restore v1 to its old commit".to_owned(),
            )
        },
        capture: None,
    }
}

fn stash_b_branches_from_stash() -> Scenario {
    Scenario {
        name: "stash-b-branches-from-stash",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: with_a_stash,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                key(ch('S')),
                Step::WaitFor(Wait::StashReady),
                key(ch('b')),
                Step::WaitFor(Wait::Prompt),
                Step::Type("rescued"),
                key(KeyCode::Enter),
            ],
            claim_and_confirm("the stash branch"),
            vec![Step::Mark("branched")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            let claim = marks.claim()?;
            let branched = marks.get("branched")?;
            ghost_equals_git(claim, &marks.before, branched, fixture)?;
            crate::repo_truth::check(
                branched.head_symbolic.as_deref() == Some("refs/heads/rescued"),
                || format!("HEAD is not on rescued: {:?}", branched.head_symbolic),
            )?;
            crate::repo_truth::check(branched.repo.stash.is_empty(), || {
                format!("the stash entry is still there: {:?}", branched.repo.stash)
            })?;
            last_entry_committed(branched, OperationKind::BranchFromStash)?;
            let undone = marks.get("undone")?;
            crate::repo_truth::check(
                undone.repo.stash.iter().any(|line| line.contains("wip")),
                || format!("u did not put the stash back: {:?}", undone.repo.stash),
            )
        },
        capture: None,
    }
}

fn move_a_branch_not_checked_out() -> Scenario {
    Scenario {
        name: "move-a-branch-not-checked-out",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: forked_on_main,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready), Step::DragBadgeOnto("main")],
            vec![
                Step::See("the drop menu offers Move, not Reset", |state| {
                    let labels: Vec<String> = state
                        .context_menu
                        .as_ref()
                        .ok_or_else(|| "no menu".to_owned())?
                        .menu
                        .items
                        .iter()
                        .map(|item| item.label.to_string())
                        .collect();
                    crate::repo_truth::check(
                        labels.iter().any(|label| label.starts_with("Move feature")),
                        || format!("no Move row: {labels:?}"),
                    )
                }),
                Step::MenuPickOpen("Move"),
            ],
            claim_and_confirm("the move"),
            vec![Step::Mark("moved")],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            let claim = marks.claim()?;
            let moved = marks.get("moved")?;
            ghost_equals_git(claim, &marks.before, moved, fixture)?;
            crate::repo_truth::check(
                branch(moved, "feature")? == branch(&marks.before, "main")?,
                || "feature did not move onto main's commit".to_owned(),
            )?;
            crate::repo_truth::check(
                moved.head_symbolic.as_deref() == Some("refs/heads/main"),
                || format!("HEAD moved off main: {:?}", moved.head_symbolic),
            )?;
            same_files("moving a branch not checked out", &marks.before, moved)?;
            last_entry_committed(moved, OperationKind::Reset)?;
            let undone = marks.get("undone")?;
            crate::repo_truth::check(
                branch(undone, "feature")? == branch(&marks.before, "feature")?,
                || "u did not restore feature".to_owned(),
            )
        },
        capture: None,
    }
}
