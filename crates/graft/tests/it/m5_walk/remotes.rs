//! Where history meets the remote: a diverged pull names `m` and `r`, and
//! `r` previews the rebase onto the tracking ref.

use graft_fixtures::Shape;

use super::claim::ghost_equals_git;
use super::fixture::{Fixture, diverged};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{
    a_status_said, ch, chain, claim_and_confirm, key, plain, status_containing,
};
use crate::repo_truth::check;

pub(super) fn remote_history() -> Vec<Scenario> {
    vec![pull_diverged_offers_m_and_r()]
}

fn pull_diverged_offers_m_and_r() -> Scenario {
    Scenario {
        name: "pull-diverged-offers-m-and-r",
        shape: Shape::Linear,
        remote: true,
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
