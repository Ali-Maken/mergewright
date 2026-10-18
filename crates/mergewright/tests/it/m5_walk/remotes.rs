//! Where history meets the remote: a diverged pull names `m` and `r`, `r`
//! previews the rebase onto the tracking ref, and — the one case where
//! **undo cannot be complete, because the remote already has it** — `u`
//! after a push explains itself before it runs.

use mergewright::update::Status;
use mergewright_core::event::KeyCode;
use mergewright_core::journal::OperationKind;
use mergewright_fixtures::Shape;

use super::claim::ghost_equals_git;
use super::fixture::{Fixture, Remote, ahead_to_merge_and_push, diverged, one_to_push};
use super::steps::{Capture, Scenario, Sees, Step, Wait, When};
use super::truth::{Marks, Truth};
use super::vocabulary::{
    a_status_said, branch, ch, chain, claim_and_confirm, key, plain, same_files, status_containing,
};
use crate::repo_truth::{check, run_git};

pub(super) fn remote_history() -> Vec<Scenario> {
    vec![
        pull_diverged_offers_m_and_r(),
        u_after_push_asks_once(),
        u_after_push_repush_old_tip(),
        u_after_push_cancel_asks_again(),
        push(),
    ]
}

/// The scenarios here that walk a row of the M6 `journey-counts` fence, so
/// `journey_counts_match_the_note` sees them (m7-power-tools/25b).
#[cfg(test)]
pub(super) fn counted_journeys() -> Vec<Scenario> {
    vec![push()]
}

fn pull_diverged_offers_m_and_r() -> Scenario {
    Scenario {
        name: "pull-diverged-offers-m-and-r",
        shape: Shape::Linear,
        remote: Remote::Local,
        size: (80, 24),
        setup: diverged,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                key(ch('p')),
                Step::Settle("the pull"),
                Step::See("the line names both branches and both keys", |state| {
                    status_containing(state, "main and origin/main have diverged")?;
                    status_containing(state, "m merge it")?;
                    status_containing(state, "r rebase onto it")
                }),
                Step::Mark("diverged"),
                Step::WaitFor(Wait::RefsCaughtUp),
                Step::See(
                    "the line's remembered tip is where origin/main is now",
                    |state| {
                        let (tracking, tip) = state
                            .remote
                            .diverged_upstream
                            .as_ref()
                            .ok_or_else(|| "the line remembers no upstream".to_owned())?;
                        let now = state.refs.as_deref().and_then(|refs| {
                            refs.remotes()
                                .chain(refs.branches().map(|(oid, name, _, _)| (oid, name)))
                                .find(|(_, name)| {
                                    format!("refs/remotes/{}", String::from_utf8_lossy(name))
                                        == *tracking
                                })
                                .map(|(oid, _)| oid)
                        });
                        // `remotes()` lists no level a local branch tracks at the same
                        // commit, so a missing badge means the tracking ref is at
                        // some branch's tip — never the stale one a fetch moved.
                        check(now.is_none_or(|now| now == *tip), || {
                            format!(
                                "the line remembers {tracking} at {tip}, the refs have it at {now:?}: \
                             `r` will aim at a tip the pull's own fetch moved"
                            )
                        })
                    },
                ),
                key(ch('r')),
            ],
            claim_and_confirm("the rebase onto origin/main"),
            vec![Step::Mark("rebased")],
        ]),
        expect: pull_diverged_holds,
        capture: Some(Capture {
            stem: "m5-11-diverged-pull-sticky",
            headless: vec![],
            keys: plain(&[ch('p')]),
            on_screen: "have diverged",
            when: When::Appears,
        }),
    }
}

/// The refused pull moved nothing of the user's — only the tracking ref its
/// fetch half brought forward; `r` then rebased `ours locally` onto that
/// tracking ref, the ghost equals git, the bare is untouched.
fn pull_diverged_holds(fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let diverged = marks.get("diverged")?;
    a_status_said(diverged, "have diverged")?;
    // The **local** branches and `HEAD` stayed where they were: the pull
    // refused to fast-forward and wrote nothing of the user's. Its fetch
    // half did run, and `refs/remotes/origin/main` moving is that half
    // doing its job — it is exactly what `r` rebases onto below without a
    // second fetch. Reading `refs` whole called that fetch "a ref moved"
    // (m5/32; unreachable while the `See` step above failed).
    check(heads(diverged) == heads(&marks.before), || {
        "the refused pull moved a local branch".to_owned()
    })?;
    check(diverged.repo.head == marks.before.repo.head, || {
        "the refused pull moved HEAD".to_owned()
    })?;
    let tracking_was = marks.before.repo.refs.get("refs/remotes/origin/main");
    check(
        diverged.repo.refs.get("refs/remotes/origin/main") != tracking_was,
        || "the pull's fetch left origin/main where it was".to_owned(),
    )?;
    let rebased = marks.get("rebased")?;
    let claim = marks.claim()?;
    check(claim.plan.plan.ghosts.len() == 1, || {
        format!(
            "{} ghosts for one local commit",
            claim.plan.plan.ghosts.len()
        )
    })?;
    ghost_equals_git(claim, &marks.before, rebased, fixture)?;
    let onto = fixture.git(&["rev-parse", "main~1"])?;
    let tracking = rebased
        .repo
        .refs
        .get("refs/remotes/origin/main")
        .ok_or_else(|| "no tracking ref".to_owned())?;
    check(onto.trim() == tracking, || {
        format!("main~1 is {}, not origin/main {tracking}", onto.trim())
    })?;
    check(rebased.bare == marks.before.bare, || {
        "the rebase touched the bare".to_owned()
    })
}

// ───────────────────────────── u after a push ────────────────────────────

/// Merge `feature` into `main` (a fast-forward, so the ghost is a ref move
/// and nothing else) and push it. That is the shape `u` after a push is
/// about: a journal entry that moved `refs/heads/main`, and a push of the
/// same branch after it.
pub(super) fn merged_and_pushed() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        Step::CursorTo("feature"),
        key(ch('m')),
        Step::WaitFor(Wait::PreviewReady),
        Step::Confirm,
        Step::Write("the merge"),
        Step::WaitFor(Wait::Ready),
        key(ch('P')),
        Step::Confirm,
        Step::Write("the push"),
        Step::WaitFor(Wait::Ready),
        Step::WaitFor(Wait::RefsCaughtUp),
        // A re-push is a force-with-lease against the tracking tip the app
        // is holding, so that reading has to be the one the push just left.
        Step::WaitFor(Wait::RemotesCaughtUp),
        Step::WaitFor(Wait::JournalHas(OperationKind::Push)),
        Step::WaitFor(Wait::JournalRead),
        Step::Mark("pushed"),
    ]
}

/// The box `u` opens after a push, in the words the design gives it. The
/// third choice is there only when a re-push can be built, and here it can.
fn the_box_explains(state: &mergewright::update::AppState) -> Result<(), String> {
    let view = &state
        .confirm
        .as_ref()
        .ok_or_else(|| format!("u opened no box; status {:?}", state.status))?
        .view;
    check(view.title.as_str() == "Undo after a push", || {
        format!("the box is titled {:?}", view.title.as_str())
    })?;
    let message: Vec<String> = view.message.iter().map(ToString::to_string).collect();
    check(
        message
            .iter()
            .any(|line| line.contains("undo works on your local branch only")),
        || format!("the box does not say what undo can reach: {message:?}"),
    )?;
    let choices: Vec<String> = view.choices.iter().map(ToString::to_string).collect();
    check(
        choices == ["Cancel", "Undo locally", "Re-push old tip"],
        || format!("choices {choices:?}"),
    )?;
    check(view.cursor == 0, || {
        format!("the cursor opens on choice {}", view.cursor)
    })
}

/// The push really sent the merge: the bare's `main` is the local `main`.
fn the_push_landed(marks: &Marks) -> Result<(), String> {
    let pushed = marks.get("pushed")?;
    let tip = branch(pushed, "main")?;
    check(tip != branch(&marks.before, "main")?, || {
        "the merge did not move main".to_owned()
    })?;
    check(bare_main(pushed)? == tip, || {
        format!("the bare's main is {:?}, main is {tip}", bare_main(pushed))
    })
}

/// The bare's `refs/heads/main` in a truth.
fn bare_main(truth: &Truth) -> Result<String, String> {
    truth
        .bare
        .as_ref()
        .ok_or_else(|| "this scenario has no bare".to_owned())?
        .get("refs/heads/main")
        .cloned()
        .ok_or_else(|| "the bare has no main".to_owned())
}

/// `u` on work a push has already sent asks first, and `[ Undo locally ]`
/// then does exactly what the box promised: **the local branch only**. The
/// bare keeps the commit — which is the honest half of this, and the reason
/// the box exists at all. Asked once: the second `u` opens no box.
fn u_after_push_asks_once() -> Scenario {
    Scenario {
        name: "u-after-push-asks-once",
        shape: Shape::Linear,
        remote: Remote::Local,
        size: (80, 24),
        setup: ahead_to_merge_and_push,
        steps: chain(vec![
            merged_and_pushed(),
            vec![
                key(ch('u')),
                Step::See("u explains before it runs", the_box_explains),
                Step::Choose(1),
                Step::Write("the undo"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("undone"),
                key(ch('u')),
                Step::See("the second u does not ask again", |state| {
                    check(state.confirm.is_none(), || {
                        "u asked a second time about the same push".to_owned()
                    })
                }),
                Step::Settle("the second u"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("twice"),
            ],
        ]),
        expect: |_, marks| {
            the_push_landed(marks)?;
            let pushed = marks.get("pushed")?;
            let undone = marks.get("undone")?;
            // What the box promised: the local branch went back.
            check(
                branch(undone, "main")? == branch(&marks.before, "main")?,
                || {
                    format!(
                        "main is {}, not back at {}",
                        branch(undone, "main").unwrap_or_default(),
                        branch(&marks.before, "main").unwrap_or_default()
                    )
                },
            )?;
            // And the half it promised it could not reach: the remote
            // still has the merge. Undo after a push is **not** complete,
            // and the walk says so in the same breath as the box does.
            check(bare_main(undone)? == bare_main(pushed)?, || {
                "the undo moved the bare, which the box said it would not".to_owned()
            })?;
            let twice = marks.get("twice")?;
            check(bare_main(twice)? == bare_main(pushed)?, || {
                "the second u moved the bare".to_owned()
            })
        },
        capture: None,
    }
}

/// `[ Re-push old tip ]` is the one way back that **does** reach the
/// remote, and it asks a second time before it touches it: the lease box.
/// After it, the bare is where it was before the push and the local branch
/// has not moved.
fn u_after_push_repush_old_tip() -> Scenario {
    Scenario {
        name: "u-after-push-repush-old-tip",
        shape: Shape::Linear,
        remote: Remote::Local,
        size: (80, 24),
        setup: ahead_to_merge_and_push,
        steps: chain(vec![
            merged_and_pushed(),
            vec![
                key(ch('u')),
                Step::See("u explains before it runs", the_box_explains),
                Step::Choose(2),
                Step::See("the remote is not touched without a second yes", |state| {
                    let view = &state
                        .confirm
                        .as_ref()
                        .ok_or_else(|| "the lease box did not open".to_owned())?
                        .view;
                    check(view.command.as_str().contains("--force-with-lease"), || {
                        format!("the lease box runs {:?}", view.command.as_str())
                    })?;
                    check(state.operating.is_none(), || {
                        "something ran before the second yes".to_owned()
                    })
                }),
                Step::Confirm,
                Step::Write("the re-push"),
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::RefsCaughtUp),
                Step::Mark("repushed"),
            ],
        ]),
        expect: |_, marks| {
            the_push_landed(marks)?;
            let repushed = marks.get("repushed")?;
            // The bare is back where it was before the push …
            check(bare_main(repushed)? == bare_main(&marks.before)?, || {
                format!(
                    "the bare's main is {:?}, not the tip the push replaced ({:?})",
                    bare_main(repushed),
                    bare_main(&marks.before)
                )
            })?;
            // … and the local branch was not touched: a re-push sends the
            // old tip, it does not undo anything here.
            let pushed = marks.get("pushed")?;
            check(branch(repushed, "main")? == branch(pushed, "main")?, || {
                "the re-push moved the local main".to_owned()
            })?;
            same_files("a re-push", pushed, repushed)
        },
        capture: None,
    }
}

/// `Cancel` runs nothing **and does not use up the question**: the next `u`
/// asks again. A box you dismissed by accident must not silently turn the
/// next keypress into the thing you were being warned about.
fn u_after_push_cancel_asks_again() -> Scenario {
    Scenario {
        name: "u-after-push-cancel-asks-again",
        shape: Shape::Linear,
        remote: Remote::Local,
        size: (80, 24),
        setup: ahead_to_merge_and_push,
        steps: chain(vec![
            merged_and_pushed(),
            vec![
                key(ch('u')),
                Step::See("u explains before it runs", the_box_explains),
                key(KeyCode::Escape),
                Step::See("Esc closed it and ran nothing", |state| {
                    check(state.confirm.is_none(), || {
                        "the box is still open".to_owned()
                    })?;
                    check(state.operating.is_none(), || {
                        format!("something is running: {:?}", state.operating)
                    })
                }),
                Step::Mark("cancelled"),
                key(ch('u')),
                Step::See("u asks again", the_box_explains),
                key(KeyCode::Escape),
                Step::Mark("cancelled-again"),
            ],
        ]),
        expect: |_, marks| {
            the_push_landed(marks)?;
            let pushed = marks.get("pushed")?;
            for mark in ["cancelled", "cancelled-again"] {
                let after = marks.get(mark)?;
                check(after.repo.refs == pushed.repo.refs, || {
                    format!("{mark}: a cancelled u moved a ref")
                })?;
                check(after.bare == pushed.bare, || {
                    format!("{mark}: a cancelled u moved the bare")
                })?;
                check(after.journal.len() == pushed.journal.len(), || {
                    format!("{mark}: a cancelled u journaled something")
                })?;
                same_files("a cancelled u", pushed, after)?;
            }
            Ok(())
        },
        capture: None,
    }
}

/// `push`: plain push, `P y` — 2, the M6 fence's row (P15,
/// `m6-usability-fixes.md:59`). Starts on the graph, the cursor on the
/// current branch's row, clean, one commit ahead of `origin`. Lands with
/// the bare's `main` at the local tip, the push box gone from the frame and
/// no failure on the status line (m7-power-tools/25b).
fn push() -> Scenario {
    Scenario {
        name: "push",
        shape: Shape::Linear,
        remote: Remote::Local,
        size: (120, 40),
        setup: one_to_push,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Route(true),
            key(ch('P')),
            Step::Frame(
                "the push box, drawn",
                &[Sees::Has("Push main → origin/main")],
            ),
            key(ch('y')),
            Step::Write("the push"),
            Step::WaitFor(Wait::Ready),
            Step::Route(false),
            Step::Frame(
                "the push box gone",
                &[Sees::Lacks("Push main → origin/main")],
            ),
            Step::See("no failure on the status line", |state| {
                check(
                    !matches!(state.status, Status::Failed(_) | Status::Sticky { .. }),
                    || format!("status {:?}", state.status),
                )
            }),
            Step::Shell(|fixture| {
                let head = fixture.git(&["rev-parse", "HEAD"])?;
                let bare = fixture.bare().ok_or("this scenario has no bare")?;
                let main = run_git(bare, &["rev-parse", "refs/heads/main"])?;
                check(main.trim() == head.trim(), || {
                    format!("the bare's main is {main:?}, HEAD {head:?}")
                })
            }),
        ],
        expect: |_, _| Ok(()),
        capture: None,
    }
}

/// Only the local branches of a truth, as name/oid pairs: what a refused
/// pull must leave alone, without counting the tracking refs its fetch half
/// is supposed to move.
fn heads(truth: &super::truth::Truth) -> Vec<(&String, &String)> {
    truth
        .repo
        .refs
        .iter()
        .filter(|(name, _)| name.starts_with("refs/heads/"))
        .collect()
}
