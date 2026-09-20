//! The push scenarios: the box, the upstream, the rejection and the lease.

use graft_core::event::KeyCode;
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::fixture::{Fixture, RemoteKind};
use super::run::status_text;
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{
    amend_locally, bare_ref, box_message_contains, box_title_starts, ch, chain, commit_by_key,
    done_containing, head, key, no_box, ours_locally, plain, same_bare, stage_pushed_txt,
    sticky_containing, teammate_commits,
};
use crate::repo_truth::{check, run_git};

pub(super) fn pushing() -> Vec<Scenario> {
    vec![push_plain(), push_sets_upstream(), push_detached_refused()]
}

/// `push-plain`'s verdict.
fn push_plain_expect(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let before = &marks.before;
    let old = head(before)?.to_owned();
    let pushed = marks.get("pushed")?;
    let committed = head(pushed)?.to_owned();
    check(committed != old, || "the commit did not land".to_owned())?;
    check(bare_ref(pushed, &fixture.main_ref())? == committed, || {
        format!(
            "the bare's main is {:?}, not {committed}",
            pushed.bare.get(&fixture.main_ref())
        )
    })?;
    let entry = pushed
        .journal
        .iter()
        .find(|entry| entry.kind == OperationKind::Push)
        .ok_or("no Push entry in the journal")?;
    let tracking = fixture.tracking_ref();
    let before_ref = entry.refs_before.get(&tracking).copied().flatten();
    let after_ref = entry.refs_after.get(&tracking).copied().flatten();
    check(
        before_ref.map(|oid| oid.to_string()) == Some(old.clone())
            && after_ref.map(|oid| oid.to_string()) == Some(committed.clone()),
        || format!("the Push entry's {tracking}: {before_ref:?} → {after_ref:?}"),
    )?;
    let undone = marks.get("undone")?;
    check(head(undone)? == old, || {
        format!("u left main at {:?}, not {old}", undone.work.head)
    })?;
    same_bare("u after a push", pushed, undone)?;
    let redone = marks.get("redone")?;
    check(head(redone)? == committed, || {
        format!("U left main at {:?}, not {committed}", redone.work.head)
    })
}

fn push_plain() -> Scenario {
    Scenario {
        name: "push-plain",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: stage_pushed_txt,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready)],
            commit_by_key("to push"),
            vec![
                key(ch('P')),
                Step::See("the plain push box", |state| {
                    box_title_starts(state, "Push ")?;
                    box_message_contains(state, &["1 commit", "can't be undone from here"])
                }),
                Step::Confirm,
                Step::Write("the push"),
                Step::See("P says pushed", |state| done_containing(state, "pushed ")),
                Step::Mark("pushed"),
                Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
                key(ch('u')),
                Step::Write("the undo"),
                Step::See("u says origin still has it", |state| {
                    done_containing(state, " still has it")
                }),
                Step::Mark("undone"),
                key(ch('U')),
                Step::Write("the redo"),
                Step::Mark("redone"),
            ],
        ]),
        expect: push_plain_expect,
        capture: Some(Capture {
            stem: "m4-01-push-box-plain",
            headless: chain(vec![
                vec![Step::WaitFor(Wait::Ready)],
                commit_by_key("to push"),
            ]),
            keys: plain(&[ch('P')]),
            secret: None,
            on_screen: "→ origin/",
            when: When::Settled,
        }),
    }
}

fn push_sets_upstream() -> Scenario {
    Scenario {
        name: "push-sets-upstream",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture
                .git(fixture.work(), &["checkout", "-q", "-b", "feature"])
                .map(drop)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the box says it sets the upstream", |state| {
                box_message_contains(state, &["sets upstream"])
            }),
            Step::Confirm,
            Step::Write("the push"),
            Step::Mark("pushed"),
        ],
        expect: |fixture, marks| {
            let pushed = marks.get("pushed")?;
            let tip = head(pushed)?;
            check(bare_ref(pushed, "refs/heads/feature")? == tip, || {
                "the bare's feature is not the local tip".to_owned()
            })?;
            let remote = run_git(fixture.work(), &["config", "branch.feature.remote"])?;
            let merge = run_git(fixture.work(), &["config", "branch.feature.merge"])?;
            check(
                remote.trim() == "origin" && merge.trim() == "refs/heads/feature",
                || format!("branch.feature.remote={remote:?} merge={merge:?}"),
            )
        },
        capture: None,
    }
}

fn push_detached_refused() -> Scenario {
    Scenario {
        name: "push-detached-refused",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            fixture
                .git(fixture.work(), &["checkout", "-q", "--detach"])
                .map(drop)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("P is refused in words, no box", |state| {
                no_box(state)?;
                check(status_text(state).contains("detached"), || {
                    format!("the line: {:?}", state.status)
                })
            }),
            Step::Mark("refused"),
        ],
        expect: |_, marks| {
            let refused = marks.get("refused")?;
            same_bare("a refused push", &marks.before, refused)?;
            check(refused.journal.is_empty(), || {
                "a refusal was journaled".to_owned()
            })
        },
        capture: None,
    }
}

pub(super) fn rejections() -> Vec<Scenario> {
    vec![push_rejected_then_lease(), push_lease_refused_stale()]
}

fn push_rejected_then_lease() -> Scenario {
    Scenario {
        name: "push-rejected-then-lease",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: amend_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::Settle("the rejected push"),
            Step::See("the rejection stays, with the hint", |state| {
                sticky_containing(
                    state,
                    &[
                        "push rejected: origin/",
                        "has moved",
                        "p pull first · P force with lease",
                    ],
                )
            }),
            Step::Mark("rejected"),
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the lease box", |state| {
                box_title_starts(state, "Force-push ")?;
                box_message_contains(
                    state,
                    &[
                        "refused if origin moved since",
                        "rewrites history for everyone who has",
                    ],
                )
            }),
            Step::Confirm,
            Step::Write("the lease push"),
            Step::Mark("forced"),
        ],
        expect: |fixture, marks| {
            let rejected = marks.get("rejected")?;
            same_bare("a rejected push", &marks.before, rejected)?;
            let amended = head(&marks.before)?;
            let forced = marks.get("forced")?;
            check(bare_ref(forced, &fixture.main_ref())? == amended, || {
                "the lease push did not land".to_owned()
            })
        },
        // The rejection `P` remembers lives in the running app, so the
        // captured run must be rejected itself: `P → ⏎` (rejected), then
        // `P` again opens the lease box.
        capture: Some(Capture {
            stem: "m4-02-push-box-lease",
            headless: Vec::new(),
            keys: plain(&[ch('P'), KeyCode::Right, KeyCode::Enter, ch('P')]),
            secret: None,
            on_screen: "Force-push",
            when: When::Settled,
        }),
    }
}

fn push_lease_refused_stale() -> Scenario {
    Scenario {
        name: "push-lease-refused-stale",
        shape: Shape::Linear,
        remote: RemoteKind::Local,
        setup: |fixture| {
            teammate_commits(fixture)?;
            ours_locally(fixture)
        },
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::Settle("the rejected push"),
            Step::See("rejected", |state| {
                sticky_containing(state, &["push rejected"])
            }),
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::See("the lease box", |state| {
                box_title_starts(state, "Force-push ")
            }),
            Step::Confirm,
            Step::Settle("the stale lease push"),
            Step::See("the line names the stale lease", |state| {
                sticky_containing(state, &["push refused:", "stale lease", "f to fetch"])
            }),
            Step::Mark("refused"),
        ],
        expect: |_, marks| same_bare("a stale lease", &marks.before, marks.get("refused")?),
        capture: Some(Capture {
            stem: "m4-06-rejected-sticky",
            headless: Vec::new(),
            keys: plain(&[ch('P'), KeyCode::Right, KeyCode::Enter]),
            secret: None,
            on_screen: "push rejected",
            when: When::Settled,
        }),
    }
}
