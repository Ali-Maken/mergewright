//! The fetch and pull scenarios: what `f` and `p` move, refuse and undo.

use std::path::Path;

use mergewright_core::journal::{EntryStatus, OperationKind};
use mergewright_fixtures::Shape;

use super::fixture::{Fixture, RemoteKind};
use super::run::status_text;
use super::steps::{Scenario, Step, Wait};
use super::truth::{Marks, diff_truth};
use super::vocabulary::{
    bare_ref, ch, done_containing, head, key, last_entry, ours_locally, same_bare,
    sticky_containing, teammate_commits, work_ref,
};
use crate::repo_truth::{check, diff_truth as diff_work, run_git};

pub(super) fn fetching() -> Vec<Scenario> {
    vec![fetch_brings_new_commits(), fetch_prunes_deleted_branch()]
}

fn fetch_brings_new_commits() -> Scenario {
    Scenario {
        name: "fetch-brings-new-commits",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::See("f says fetched", |state| done_containing(state, "fetched")),
            Step::Mark("fetched"),
            key(ch('u')),
            Step::Write("the undo"),
            Step::See("u says nothing to undo", |state| {
                done_containing(state, "nothing to undo")
            }),
            Step::Mark("after-u"),
        ],
        expect: |fixture, marks| {
            let fetched = marks.get("fetched")?;
            let theirs = bare_ref(&marks.before, &fixture.main_ref())?;
            check(
                work_ref(fetched, &fixture.tracking_ref())? == theirs,
                || {
                    format!(
                        "the tracking ref is {:?}, the bare's tip {theirs}",
                        fetched.work.refs.get(&fixture.tracking_ref())
                    )
                },
            )?;
            check(head(fetched)? == head(&marks.before)?, || {
                "a fetch moved HEAD".to_owned()
            })?;
            let entry = last_entry(fetched)?;
            check(
                entry.kind == OperationKind::Fetch && entry.status == EntryStatus::Committed,
                || format!("the last entry is {:?} {:?}", entry.kind, entry.status),
            )?;
            let after = marks.get("after-u")?;
            check(after.work.refs == fetched.work.refs, || {
                format!("u moved a ref: {:?}", diff_work(&fetched.work, &after.work))
            })?;
            same_bare("u after a fetch", fetched, after)
        },
        capture: None,
    }
}

fn fetch_prunes_deleted_branch() -> Scenario {
    Scenario {
        name: "fetch-prunes-deleted-branch",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture.git(fixture.bare(), &["branch", "gone"])?;
            fixture.git(fixture.work(), &["fetch", "-q", "origin"])?;
            fixture.git(fixture.bare(), &["branch", "-D", "gone"])?;
            Ok(())
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('f')),
            Step::Write("the fetch"),
            Step::Mark("fetched"),
        ],
        expect: |_, marks| {
            check(
                marks
                    .before
                    .work
                    .refs
                    .contains_key("refs/remotes/origin/gone"),
                || "setup: the work clone never had origin/gone".to_owned(),
            )?;
            let fetched = marks.get("fetched")?;
            check(
                !fetched.work.refs.contains_key("refs/remotes/origin/gone"),
                || "origin/gone survived the fetch: not pruned".to_owned(),
            )
        },
        capture: None,
    }
}

pub(super) fn pulling() -> Vec<Scenario> {
    vec![
        pull_fast_forward(),
        pull_diverged_refused(),
        pull_dirty_file_sets_it_aside(),
        pull_no_upstream_refused(),
    ]
}

/// `pull-fast-forward`'s verdict.
fn pull_fast_forward_expect(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let before = &marks.before;
    let theirs = bare_ref(before, &fixture.main_ref())?.to_owned();
    let old = head(before)?.to_owned();
    let pulled = marks.get("pulled")?;
    check(head(pulled)? == theirs, || {
        format!("main is {:?}, not the bare's {theirs}", pulled.work.head)
    })?;
    let changed = run_git(fixture.bare(), &["diff", "--name-only", &old, &theirs])?;
    let file = changed
        .lines()
        .next()
        .ok_or("the teammate's commit changed no file")?
        .to_owned();
    let their_bytes = run_git(fixture.bare(), &["show", &format!("{theirs}:{file}")])?;
    check(
        pulled.work.files.get(Path::new(&file)).map(Vec::as_slice) == Some(their_bytes.as_bytes()),
        || format!("{file}'s bytes are not the teammate's"),
    )?;
    let undone = marks.get("undone")?;
    check(head(undone)? == old, || {
        format!("u left main at {:?}, not {old}", undone.work.head)
    })?;
    check(undone.work.files == before.work.files, || {
        format!(
            "u did not restore the files: {:?}",
            diff_work(&before.work, &undone.work)
        )
    })?;
    check(work_ref(undone, &fixture.tracking_ref())? == theirs, || {
        "u moved the tracking ref".to_owned()
    })?;
    same_bare("u after a pull", pulled, undone)?;
    let redone = marks.get("redone")?;
    check(head(redone)? == theirs, || {
        format!("U left main at {:?}, not {theirs}", redone.work.head)
    })
}

fn pull_fast_forward() -> Scenario {
    Scenario {
        name: "pull-fast-forward",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: teammate_commits,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::Write("the pull"),
            Step::See("p says pulled", |state| {
                done_containing(state, "pulled origin/")
            }),
            Step::Mark("pulled"),
            key(ch('u')),
            Step::Write("the undo"),
            Step::Mark("undone"),
            key(ch('U')),
            Step::Write("the redo"),
            Step::Mark("redone"),
        ],
        expect: pull_fast_forward_expect,
        capture: None,
    }
}

fn pull_diverged_refused() -> Scenario {
    Scenario {
        name: "pull-diverged-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            teammate_commits(fixture)?;
            ours_locally(fixture)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::Settle("the refused pull"),
            // M5 replaced M4's line here. M4 could only send the user to
            // the shell (`pull stopped: … merge or rebase (M5); shell: git
            // pull --rebase`); M5 has both verbs, so the refusal names the
            // two keys that do it inside Mergewright and remembers the upstream
            // they aim at (m5-history-and-ghost-preview/08, piece 9).
            Step::See("the line names the two keys that fix it", |state| {
                sticky_containing(state, &["diverged", "m merge it", "r rebase onto it"])
            }),
            Step::Mark("refused"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            check(head(refused)? == head(&marks.before)?, || {
                "main moved".to_owned()
            })?;
            let theirs = bare_ref(&marks.before, &fixture.main_ref())?;
            check(
                work_ref(refused, &fixture.tracking_ref())? == theirs,
                || "the tracking ref did not advance to the teammate's tip".to_owned(),
            )?;
            same_bare("a refused pull", &marks.before, refused)
        },
        capture: None,
    }
}

/// Was `pull-dirty-file-refused` (m6-conflicts/85 D). Since
/// m7-power-tools/03b a fast-forward pull over his edit to a file it brings
/// sets the edit aside and puts it back (ADR-0045 §4): here both sides
/// rewrote the one line, so the put-back clashes and he lands on the
/// resolver with `HEAD` at the teammate's commit and his bytes kept in the
/// file's clash.
fn pull_dirty_file_sets_it_aside() -> Scenario {
    Scenario {
        name: "pull-dirty-file-sets-it-aside",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            let file = fixture.a_tracked_file()?;
            fixture.change_on_remote(&file, "theirs\n")?;
            std::fs::write(fixture.work().join(&file), "mine, uncommitted\n")
                .map_err(|why| format!("{file}: {why}"))?;
            Ok(())
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::Settle("the pull over his edit"),
            Step::See("the resolver is open", |state| {
                check(state.resolver.is_some(), || {
                    format!("no resolver; status: {}", status_text(state))
                })
            }),
            Step::Mark("clashed"),
        ],
        expect: |fixture, marks| {
            let clashed = marks.get("clashed")?;
            let file = fixture.a_tracked_file()?;
            let bytes = clashed
                .work
                .files
                .get(Path::new(&file))
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .ok_or_else(|| format!("{file} is gone from the tree"))?;
            check(bytes.contains("mine, uncommitted"), || {
                format!("{file} lost his edit: {bytes:?}")
            })?;
            check(bytes.contains("theirs"), || {
                format!("{file} lacks the teammate's line: {bytes:?}")
            })?;
            check(head(clashed)? != head(&marks.before)?, || {
                "main did not move: the pull did not go through".to_owned()
            })
        },
        capture: None,
    }
}

fn pull_no_upstream_refused() -> Scenario {
    Scenario {
        name: "pull-no-upstream-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture
                .git(fixture.work(), &["branch", "--unset-upstream"])
                .map(drop)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('p')),
            Step::See("p is refused in words", |state| {
                check(status_text(state).contains("has no upstream"), || {
                    format!("the line: {:?}", state.status)
                })?;
                check(state.operating.is_none(), || {
                    "a write was queued".to_owned()
                })
            }),
            Step::Mark("refused"),
        ],
        expect: |_, marks| {
            let refused = marks.get("refused")?;
            let moved = diff_truth(&marks.before, refused);
            check(moved.is_empty(), || format!("something changed: {moved:?}"))?;
            check(refused.journal.is_empty(), || {
                "a refusal was journaled".to_owned()
            })
        },
        capture: None,
    }
}
