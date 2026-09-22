//! The honest interim: a merge that stops on a conflict focuses the banner
//! and lists the file; `a` aborts and restores; the stop survives a
//! relaunch; `c` refuses until the tree is resolved; and a rebase that
//! stops twice is still **one** journal entry that **one** `u` takes back.

use graft_core::event::KeyCode;
use graft_core::journal::{EntryStatus, OperationKind};
use graft_fixtures::Shape;
use graft_git::state::InProgress;

use super::fixture::{conflicting, twice_conflicting};
use super::steps::{Capture, Fix, Scenario, Step, Wait, When};
use super::truth::{Marks, Truth};
use super::vocabulary::{
    a_status_said, branch, ch, chain, entries_of, file_was, key, last_entry, plain, same_files,
    status_containing, undo,
};
use crate::repo_truth::check;

pub(super) fn conflicting_merges() -> Vec<Scenario> {
    vec![
        conflict_focuses_banner_and_lists_files(),
        banner_a_aborts_and_restores(),
        conflict_survives_relaunch(),
        continue_refuses_until_resolved_then_one_entry_undoes(),
        rebase_stops_twice_same_entry(),
    ]
}

/// `m` on `feature` from `main`, confirmed, stopped on `shared.txt`.
pub(super) fn merge_that_stops() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        Step::CursorTo("feature"),
        key(ch('m')),
        Step::WaitFor(Wait::PreviewReady),
        Step::Confirm,
        Step::Settle("the merge"),
        Step::WaitFor(Wait::BannerFocused),
    ]
}

/// `MERGE_HEAD` present, the journal's merge entry still `Pending`.
fn stopped(marks: &Marks, mark: &str) -> Result<(), String> {
    let truth = marks.get(mark)?;
    check(truth.in_progress.as_deref() == Some("merge"), || {
        format!("git says {:?}, not a merge", truth.in_progress)
    })?;
    let entry = last_entry(truth)?;
    check(
        entry.kind == OperationKind::Merge && entry.status == EntryStatus::Pending,
        || format!("the entry is {:?} {:?}", entry.kind, entry.status),
    )
}

fn conflict_focuses_banner_and_lists_files() -> Scenario {
    Scenario {
        name: "conflict-focuses-banner-and-lists-files",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: conflicting,
        steps: chain(vec![
            merge_that_stops(),
            vec![
                Step::See("the banner has the keyboard and names the file", |state| {
                    check(
                        state
                            .in_progress
                            .as_ref()
                            .is_some_and(|detail| detail.kind == InProgress::Merge),
                        || "the app does not know it is merging".to_owned(),
                    )?;
                    let paths = graft::update::banner::unmerged_paths(state);
                    check(
                        paths.len() == 1 && paths[0].as_os_str() == "shared.txt",
                        || format!("unmerged: {paths:?}"),
                    )?;
                    check(state.banner_focus, || {
                        "the banner is not focused".to_owned()
                    })
                }),
                Step::Mark("stopped"),
            ],
        ]),
        expect: |fixture, marks| {
            stopped(marks, "stopped")?;
            let body = fixture.bytes("shared.txt")?;
            check(body.starts_with(b"<<<<<<<"), || {
                "shared.txt carries no conflict markers".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m5-06-banner-conflicts",
            headless: vec![],
            keys: plain(&[
                ch('m'),
                graft_core::event::KeyCode::Right,
                graft_core::event::KeyCode::Enter,
            ]),
            // The banner shares the header row: `MERGING` at its end. `CONFLICTS
            // (1)` is the Detail tab's, not this frame's.
            on_screen: "MERGING",
            when: When::Settled,
        }),
    }
}

fn banner_a_aborts_and_restores() -> Scenario {
    Scenario {
        name: "banner-a-aborts-and-restores",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: conflicting,
        steps: chain(vec![
            merge_that_stops(),
            vec![
                Step::Mark("stopped"),
                key(ch('a')),
                Step::See("a asks first, Abort second", |state| {
                    let view = &state
                        .confirm
                        .as_ref()
                        .ok_or_else(|| "a opened no box".to_owned())?
                        .view;
                    let choices: Vec<String> =
                        view.choices.iter().map(ToString::to_string).collect();
                    check(choices == ["Cancel", "Abort"], || {
                        format!("choices {choices:?}")
                    })
                }),
                Step::Confirm,
                Step::Write("the abort"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("aborted"),
            ],
        ]),
        expect: |_, marks| {
            stopped(marks, "stopped")?;
            let aborted = marks.get("aborted")?;
            check(aborted.in_progress.is_none(), || {
                format!("git still says {:?}", aborted.in_progress)
            })?;
            same_files("the abort", &marks.before, aborted)?;
            check(aborted.repo.refs == marks.before.repo.refs, || {
                "the abort moved a ref".to_owned()
            })?;
            let entry = last_entry(aborted)?;
            check(
                entry.kind == OperationKind::Merge
                    && entry.status
                        == (EntryStatus::Aborted {
                            reason: "aborted".to_owned(),
                        }),
                || format!("the entry is {:?} {:?}", entry.kind, entry.status),
            )
        },
        capture: None,
    }
}

fn conflict_survives_relaunch() -> Scenario {
    Scenario {
        name: "conflict-survives-relaunch",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: conflicting,
        steps: chain(vec![
            merge_that_stops(),
            vec![
                Step::Mark("stopped"),
                Step::Relaunch,
                Step::WaitFor(Wait::BannerFocused),
                Step::See("the line says whose merge it is", |state| {
                    status_containing(state, "still in progress from a previous run")
                }),
                Step::Mark("relaunched"),
                key(ch('a')),
                Step::Confirm,
                Step::Write("the abort"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("aborted"),
            ],
        ]),
        expect: |_, marks| {
            stopped(marks, "stopped")?;
            stopped(marks, "relaunched")?;
            let aborted = marks.get("aborted")?;
            check(aborted.in_progress.is_none(), || {
                format!("git still says {:?}", aborted.in_progress)
            })?;
            same_files("the abort after a relaunch", &marks.before, aborted)
        },
        capture: None,
    }
}

// ───────────────────────── finishing what stopped ────────────────────────

/// What a person does to a conflicted `shared.txt`: write the resolution
/// with an editor, then `space` on it in the changes pane — the banner's
/// own `space marks resolved`.
///
/// The banner is modal while focused, so `Esc` hands the keyboard back
/// before the pane opens and `⇥` takes it again afterwards; without that
/// the pane's keys are swallowed silently. The wait between is not
/// decoration: `c` decides from the **last working-tree read**
/// (`banner::continue_refusal`), so pressing it before the read after the
/// stage lands would be refused for a file that is already resolved.
fn resolve(write: Fix) -> Vec<Step> {
    vec![
        key(KeyCode::Escape),
        Step::Shell(write),
        Step::Stage("shared.txt"),
        Step::WaitFor(Wait::Resolved),
        key(KeyCode::Tab),
        Step::See("the banner has the keyboard again", |state| {
            check(state.banner_focus, || {
                "⇥ did not take the banner's focus back".to_owned()
            })
        }),
    ]
}

/// `c` with something still unmerged: the words, and the keys that get out
/// of it.
fn c_is_refused() -> Vec<Step> {
    vec![
        key(ch('c')),
        Step::See("c refuses and names the keys that resolve it", |state| {
            status_containing(state, "1 file still conflicted")?;
            status_containing(state, "space marks resolved")
        }),
        Step::Mark("refused"),
    ]
}

/// The one entry of `kind` in the journal, or why there is not exactly one.
fn the_one_entry(
    truth: &Truth,
    kind: OperationKind,
) -> Result<&graft_core::journal::JournalEntry, String> {
    let count = entries_of(truth, kind);
    check(count == 1, || {
        format!(
            "{count} {kind:?} entries, not one: {:?}",
            truth
                .journal
                .iter()
                .map(|entry| (entry.kind, entry.status.clone()))
                .collect::<Vec<_>>()
        )
    })?;
    truth
        .journal
        .iter()
        .find(|entry| entry.kind == kind)
        .ok_or_else(|| format!("no {kind:?} entry"))
}

fn resolve_shared(fixture: &super::fixture::Fixture) -> Result<(), String> {
    fixture.dirty("shared.txt", "resolved by hand\n")
}

fn continue_refuses_until_resolved_then_one_entry_undoes() -> Scenario {
    Scenario {
        name: "continue-refuses-until-resolved-then-one-entry-undoes",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: conflicting,
        steps: chain(vec![
            merge_that_stops(),
            vec![Step::Mark("stopped")],
            c_is_refused(),
            resolve(resolve_shared),
            vec![
                key(ch('c')),
                Step::Write("the continue"),
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::NotInProgress),
                Step::Mark("merged"),
            ],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            stopped(marks, "stopped")?;
            // The refusal ran nothing: git is still mid-merge and the index
            // still holds unmerged entries, which is what `write-tree`
            // refuses to write.
            let refused = marks.get("refused")?;
            check(refused.in_progress.as_deref() == Some("merge"), || {
                format!("git says {:?}, not a merge", refused.in_progress)
            })?;
            check(refused.index_tree.is_none(), || {
                "the index has no unmerged entry left, so `c` refused for nothing".to_owned()
            })?;
            a_status_said(refused, "1 file still conflicted")?;
            // The merge finished as **one** entry, the one the stop left
            // pending.
            let merged = marks.get("merged")?;
            check(merged.in_progress.is_none(), || {
                format!("git still says {:?}", merged.in_progress)
            })?;
            let entry = the_one_entry(merged, OperationKind::Merge)?;
            check(entry.status == EntryStatus::Committed, || {
                format!("the merge entry is {:?}", entry.status)
            })?;
            // Named by the **oid the `merged` mark holds**, never by
            // `main`: `u` has already moved that name back by the time
            // `expect` runs, and the objects outlive the ref.
            let merge_commit = branch(merged, "main")?;
            let parents = fixture.git(&["log", "-1", "--format=%P", &merge_commit])?;
            let parents: Vec<&str> = parents.split_whitespace().collect();
            check(
                parents
                    == [
                        branch(&marks.before, "main")?.as_str(),
                        branch(&marks.before, "feature")?.as_str(),
                    ],
                || format!("main's parents are {parents:?}"),
            )?;
            file_was(merged, "shared.txt", b"resolved by hand\n")?;
            // One `u` takes the whole thing back: the branch and the file.
            let undone = marks.get("undone")?;
            check(
                branch(undone, "main")? == branch(&marks.before, "main")?,
                || "u did not bring main back".to_owned(),
            )?;
            file_was(undone, "shared.txt", b"main's line\n")
        },
        capture: None,
    }
}

fn resolve_once(fixture: &super::fixture::Fixture) -> Result<(), String> {
    fixture.dirty("shared.txt", "resolved once\n")
}

fn resolve_twice(fixture: &super::fixture::Fixture) -> Result<(), String> {
    fixture.dirty("shared.txt", "resolved twice\n")
}

/// `feature` rebased onto `main` over two commits that both rewrite
/// `shared.txt`: git stops once per replayed commit, and the second stop
/// must not open a second journal entry. One `u` afterwards takes the whole
/// rebase back — M3's promise holding inside M5's.
fn rebase_stops_twice_same_entry() -> Scenario {
    Scenario {
        name: "rebase-stops-twice-same-entry",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: twice_conflicting,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorTo("main"),
                key(ch('r')),
                Step::WaitFor(Wait::PreviewReady),
                Step::Confirm,
                Step::Settle("the rebase"),
                Step::WaitFor(Wait::BannerFocused),
                Step::Mark("first-stop"),
            ],
            resolve(resolve_once),
            vec![
                key(ch('c')),
                Step::Settle("the first continue"),
                Step::WaitFor(Wait::BannerFocused),
                Step::Mark("second-stop"),
            ],
            resolve(resolve_twice),
            vec![
                key(ch('c')),
                Step::Write("the second continue"),
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::NotInProgress),
                Step::Mark("rebased"),
            ],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: two_stops_one_entry,
        capture: None,
    }
}

/// Both stops are the same entry, the replay landed, and one `u` takes it
/// all back.
fn two_stops_one_entry(fixture: &super::fixture::Fixture, marks: &Marks) -> Result<(), String> {
    for mark in ["first-stop", "second-stop"] {
        let truth = marks.get(mark)?;
        check(truth.in_progress.as_deref() == Some("rebase"), || {
            format!("{mark}: git says {:?}, not a rebase", truth.in_progress)
        })?;
        let entry = the_one_entry(truth, OperationKind::Rebase)?;
        check(entry.status == EntryStatus::Pending, || {
            format!("{mark}: the rebase entry is {:?}", entry.status)
        })?;
    }
    // Two distinct stops, not the same one twice: the first stop is
    // replaying `feature writes`, the second `feature writes again`.
    let first = marks.get("first-stop")?;
    let second = marks.get("second-stop")?;
    check(first.repo.head != second.repo.head, || {
        "both stops are at the same HEAD, so the rebase stopped once".to_owned()
    })?;
    let rebased = marks.get("rebased")?;
    check(rebased.in_progress.is_none(), || {
        format!("git still says {:?}", rebased.in_progress)
    })?;
    let entry = the_one_entry(rebased, OperationKind::Rebase)?;
    check(entry.status == EntryStatus::Committed, || {
        format!("the rebase entry is {:?}", entry.status)
    })?;
    // The replayed branch sits on `main`'s tip and carries both
    // resolutions, newest first.
    let new_tip = branch(rebased, "feature")?;
    let onto = fixture.git(&["rev-parse", &format!("{new_tip}~2")])?;
    check(onto.trim() == branch(&marks.before, "main")?, || {
        format!("the rebased branch's ~2 is {}, not main's tip", onto.trim())
    })?;
    file_was(rebased, "shared.txt", b"resolved twice\n")?;
    // **One** `u`.
    let undone = marks.get("undone")?;
    check(
        branch(undone, "feature")? == branch(&marks.before, "feature")?,
        || "one u did not bring feature back".to_owned(),
    )?;
    same_files("one u after two stops", &marks.before, undone)
}
