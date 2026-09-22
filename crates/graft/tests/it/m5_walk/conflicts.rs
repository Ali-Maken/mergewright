//! The honest interim: a merge that stops on a conflict focuses the banner
//! and lists the file; `a` aborts and restores; the stop survives a
//! relaunch.

use graft_core::journal::{EntryStatus, OperationKind};
use graft_fixtures::Shape;
use graft_git::state::InProgress;

use super::fixture::conflicting;
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{ch, chain, key, last_entry, plain, same_files, status_containing};
use crate::repo_truth::check;

pub(super) fn conflicting_merges() -> Vec<Scenario> {
    vec![
        conflict_focuses_banner_and_lists_files(),
        banner_a_aborts_and_restores(),
        conflict_survives_relaunch(),
    ]
}

/// `m` on `feature` from `main`, confirmed, stopped on `shared.txt`.
fn merge_that_stops() -> Vec<Step> {
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
