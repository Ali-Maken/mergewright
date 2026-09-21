//! The hang, progress, undo and journal scenarios: `Esc` on a socket that
//! never answers, progress on the footer, and what `u` never touches.

use graft_core::event::KeyCode;
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;
use graft_ui::journal_screen::RowMark;

use super::fixture::{Fixture, RemoteKind};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{
    aborted_cancelled, bare_ref, box_title_starts, ch, chain, commit_by_key, ctrl, done_containing,
    head, key, last_entry, locks, ours_locally, plain, push_by_key, same_bare, stage_pushed_txt,
    teammate_commits, work_ref,
};
use crate::repo_truth::{check, diff_truth as diff_work};

pub(super) fn hangs_progress_and_undo() -> Vec<Scenario> {
    vec![
        hung_remote_esc_cancels(),
        hung_remote_pull_esc_cancels(),
        hung_remote_push_esc_cancels(),
        progress_reaches_the_footer(),
        u_never_touches_the_remote(),
        journal_rows(),
        no_index_write_on_fetch(),
    ]
}

fn hung_remote_esc_cancels() -> Scenario {
    Scenario {
        name: "hung-remote-esc-cancels",
        shape: Shape::Linear,
        remote: RemoteKind::Hung,
        setup: |_| Ok(()),
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::See("the fetch is running", |state| {
                check(state.operating.is_some(), || "f queued nothing".to_owned())
            }),
            Step::WaitFor(Wait::Connected),
            Step::Cancel("the cancelled fetch"),
            Step::See("the line says cancelled", |state| {
                done_containing(state, "fetch all remotes cancelled")
            }),
            Step::Mark("cancelled"),
        ],
        expect: |fixture, marks| hung_cancel_expect(fixture, marks, OperationKind::Fetch),
        capture: None,
    }
}

/// What every cancelled hang must leave: the last entry is `kind` and
/// `Aborted("cancelled")`, no `*.lock` under `.git`, the hung remote took
/// exactly one connection, the bare has not moved, and nothing in the work
/// clone moved — `HEAD`, the refs, the index (its bytes too) and every file
/// are what they were before the key. A pull is the one that would write
/// the index and the tree, so the same verdict covers all three.
fn hung_cancel_expect(fixture: &Fixture, marks: &Marks, kind: OperationKind) -> Result<(), String> {
    let cancelled = marks.get("cancelled")?;
    let entry = last_entry(cancelled)?;
    check(entry.kind == kind, || {
        format!("the last entry is {:?}, not {kind:?}", entry.kind)
    })?;
    aborted_cancelled(entry)?;
    let left = locks(&fixture.work().join(".git"));
    check(left.is_empty(), || format!("locks left: {left:?}"))?;
    check(cancelled.hung_connections == Some(1), || {
        format!("{:?} connections", cancelled.hung_connections)
    })?;
    same_bare("a cancelled hang", &marks.before, cancelled)?;
    let moved = diff_work(&marks.before.work, &cancelled.work);
    check(moved.is_empty(), || {
        format!("the work clone moved: {}", moved.join(" | "))
    })?;
    check(
        cancelled.work.index_bytes == marks.before.work.index_bytes,
        || ".git/index was rewritten".to_owned(),
    )
}

/// `p` against the socket that never answers: the pull hangs at git's first
/// request, `Esc` ends it, and the branch, index and tree are untouched.
fn hung_remote_pull_esc_cancels() -> Scenario {
    Scenario {
        name: "hung-remote-pull-esc-cancels",
        shape: Shape::Linear,
        remote: RemoteKind::Hung,
        setup: |_| Ok(()),
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::See("the pull is running", |state| {
                check(state.operating.is_some(), || "p queued nothing".to_owned())
            }),
            Step::WaitFor(Wait::Connected),
            Step::Cancel("the cancelled pull"),
            Step::See("the line says cancelled", |state| {
                done_containing(state, "pull origin/main cancelled")
            }),
            Step::Mark("cancelled"),
        ],
        expect: |fixture, marks| hung_cancel_expect(fixture, marks, OperationKind::Pull),
        capture: None,
    }
}

/// `P` `⏎` against the socket that never answers, with a commit of ours to
/// send: the push hangs at git's first request, `Esc` ends it, the bare has
/// not moved and the commit is still ours to push.
fn hung_remote_push_esc_cancels() -> Scenario {
    Scenario {
        name: "hung-remote-push-esc-cancels",
        shape: Shape::Linear,
        remote: RemoteKind::Hung,
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the plain push box", |state| {
                box_title_starts(state, "Push ")
            }),
            Step::Confirm,
            Step::See("the push is running", |state| {
                check(state.operating.is_some(), || "P queued nothing".to_owned())
            }),
            Step::WaitFor(Wait::Connected),
            Step::Cancel("the cancelled push"),
            Step::See("the line says cancelled", |state| {
                done_containing(state, "push main → origin/main cancelled")
            }),
            Step::Mark("cancelled"),
        ],
        expect: |fixture, marks| hung_cancel_expect(fixture, marks, OperationKind::Push),
        capture: None,
    }
}

fn progress_reaches_the_footer() -> Scenario {
    Scenario {
        name: "progress-reaches-the-footer",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| fixture.many_objects_on_remote(400, 4096).map(drop),
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::Mark("fetched"),
        ],
        expect: |_, marks| {
            let fetched = marks.get("fetched")?;
            check(
                fetched.statuses.iter().any(|line| {
                    line.starts_with("fetch all remotes · ") && line.contains("objects")
                }),
                || format!("no progress line among {:?}", fetched.statuses),
            )
        },
        capture: Some(Capture {
            stem: "m4-05-progress-footer",
            headless: Vec::new(),
            keys: plain(&[ch('f')]),
            secret: None,
            on_screen: "objects",
            when: When::Appears,
        }),
    }
}

/// `u-never-touches-the-remote`'s verdict.
fn u_never_touches_the_remote_expect(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let pushed = marks.get("pushed")?;
    let undone = marks.get("undone")?;
    same_bare("undoing everything", pushed, undone)?;
    check(head(undone)? == head(&marks.before)?, || {
        format!(
            "undoing everything left main at {:?}, not where it started",
            undone.work.head
        )
    })?;
    let theirs = bare_ref(&marks.before, &fixture.main_ref())?;
    check(work_ref(undone, &fixture.tracking_ref())? != theirs, || {
        "the tracking ref should have moved to the pushed tip".to_owned()
    })?;
    check(
        work_ref(undone, &fixture.tracking_ref())? == head(pushed)?,
        || "the tracking ref is not the pushed tip".to_owned(),
    )
}

fn u_never_touches_the_remote() -> Scenario {
    Scenario {
        name: "u-never-touches-the-remote",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            teammate_commits(fixture)?;
            stage_pushed_txt(fixture)
        },
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                key(ch('f')),
                Step::Write("the fetch"),
                Step::WaitFor(Wait::Ready),
                key(ch('p')),
                Step::Write("the pull"),
                Step::WaitFor(Wait::Ready),
            ],
            commit_by_key("ours"),
            push_by_key(),
            vec![
                Step::Mark("pushed"),
                Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
                Step::UndoAll,
                Step::Mark("undone"),
            ],
        ]),
        expect: u_never_touches_the_remote_expect,
        capture: Some(Capture {
            stem: "m4-08-undo-after-push",
            headless: chain(vec![
                vec![
                    Step::WaitFor(Wait::Ready),
                    key(ch('p')),
                    Step::Write("the pull"),
                    Step::WaitFor(Wait::Ready),
                ],
                commit_by_key("ours"),
                push_by_key(),
                vec![Step::WaitFor(Wait::JournalHas(OperationKind::Push))],
            ]),
            // `u`, then `→ Enter` on `[ Undo locally ]`: from M5 the key
            // opens the undo-after-push box first (ADR-0038 §4), and this
            // capture's subject is the line the undo leaves afterwards.
            // The box itself is M5's own capture,
            // `m5-ui-undo-after-push-80x24.txt`.
            keys: plain(&[ch('u'), KeyCode::Right, KeyCode::Enter]),
            secret: None,
            on_screen: "still has it",
            when: When::Settled,
        }),
    }
}

/// `f`, `p`, a commit out of band, `P`: fetch and pull are the app's, the
/// commit is not journaled, so the pull is the next undo.
fn fetch_pull_commit_push() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        key(ch('f')),
        Step::Write("the fetch"),
        Step::WaitFor(Wait::Ready),
        key(ch('p')),
        Step::Write("the pull"),
        // No wait here on purpose: `Ctrl+R` lands while the pull's own
        // refresh is still in flight, which is what a person does and what
        // used to wedge the pool (m5-history-and-ghost-preview/05).
        Step::Shell(ours_locally),
        Step::Key(ctrl('r')),
        Step::WaitFor(Wait::Ready),
        key(ch('P')),
        Step::Confirm,
        Step::Write("the push"),
        Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
    ]
}

/// `journal-rows`'s verdict.
fn journal_rows_expect(_fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let kinds: Vec<OperationKind> = marks
        .get("shown")?
        .journal
        .iter()
        .map(|entry| entry.kind)
        .collect();
    check(
        kinds
            == [
                OperationKind::Fetch,
                OperationKind::Pull,
                OperationKind::Push,
            ],
        || format!("the journal holds {kinds:?}"),
    )
}

fn journal_rows() -> Scenario {
    Scenario {
        name: "journal-rows",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: chain(vec![
            fetch_pull_commit_push(),
            vec![
                key(ch('J')),
                Step::See("the journal's remote rows", |state| {
                    check(state.journal.shown, || "J opened nothing".to_owned())?;
                    let now = state.harness.clock.now_seconds();
                    let view = graft::view::journal_view(state, now);
                    let row = |kind: &str| {
                        view.rows
                            .iter()
                            .find(|row| row.description.as_str().starts_with(kind))
                            .ok_or_else(|| {
                                format!(
                                    "no {kind} row among {:?}",
                                    view.rows
                                        .iter()
                                        .map(|row| row.description.as_str())
                                        .collect::<Vec<_>>()
                                )
                            })
                    };
                    let push = row("push")?;
                    check(
                        push.summary
                            .as_str()
                            .contains("not undoable \u{2014} origin has it"),
                        || format!("the push row: {}", push.summary.as_str()),
                    )?;
                    let fetch = row("fetch")?;
                    check(fetch.summary.as_str().contains("not undoable"), || {
                        format!("the fetch row: {}", fetch.summary.as_str())
                    })?;
                    let pull = row("pull")?;
                    check(pull.mark == RowMark::NextUndo, || {
                        format!("the pull row is not marked ← u: {:?}", pull.mark)
                    })
                }),
                Step::Mark("shown"),
            ],
        ]),
        expect: journal_rows_expect,
        capture: Some(Capture {
            stem: "m4-07-journal-remote-rows",
            headless: fetch_pull_commit_push(),
            keys: plain(&[ch('J')]),
            secret: None,
            on_screen: "not undoable",
            when: When::Settled,
        }),
    }
}

fn no_index_write_on_fetch() -> Scenario {
    Scenario {
        name: "no-index-write-on-fetch",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::Mark("fetched"),
        ],
        expect: |_, marks| {
            let fetched = marks.get("fetched")?;
            check(
                fetched.work.index_bytes == marks.before.work.index_bytes,
                || ".git/index was rewritten by a fetch".to_owned(),
            )
        },
        capture: None,
    }
}
