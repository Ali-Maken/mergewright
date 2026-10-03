//! The AFTER side's facts, over scripted plans.

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_ops::preview::{Fork, ForkFate, LeftBehind, PreviewPlan, RefMove, Target, WhyStays};
use graft_ui::rewrite_editor::FactTone;

use super::{after_facts, ghost_owners};

fn at(hex7: &str) -> Oid {
    Oid::from_hex(&format!("{hex7}{}", "0".repeat(33))).expect("an id")
}

fn branch(name: &str) -> RefName {
    RefName::new(format!("refs/heads/{name}").as_bytes()).expect("a ref name")
}

/// His branch with one ghost, `feature-x` following off it, `feature-y`
/// staying because it would conflict, and `backup` parked at a tip.
fn forked() -> PreviewPlan {
    let mut plan = crate::view::tests::one_ghost_over(at("0a0a0a0"), at("0b0b0b0"));
    plan.ref_moves.push(RefMove {
        name: "refs/heads/feature-x".to_owned(),
        from: Some(at("0c0c0c0")),
        to: Some(Target::Ghost(0)),
    });
    plan.forks = vec![
        Fork {
            branch: branch("feature-x"),
            at: at("a1b2c3d"),
            commits: 2,
            fate: ForkFate::Follows {
                onto: Target::Ghost(0),
            },
        },
        Fork {
            branch: branch("feature-y"),
            at: at("b2c3d4e"),
            commits: 1,
            fate: ForkFate::Stays(WhyStays::WouldConflict {
                commit: at("0d0d0d0"),
                paths: vec![std::path::PathBuf::from("src/a.rs")],
            }),
        },
    ];
    plan.left_behind = vec![
        LeftBehind {
            branch: branch("feature-y"),
            on: at("b2c3d4e"),
        },
        LeftBehind {
            branch: branch("backup"),
            on: at("c3d4e5f"),
        },
    ];
    plan
}

fn facts_of(plan: &PreviewPlan) -> Vec<(String, FactTone)> {
    after_facts(plan, &ghost_owners(plan))
        .iter()
        .map(|fact| (fact.text.as_str().to_owned(), fact.tone))
        .collect()
}

/// The same fork rows as the preview box (m8/14c): the follower by its
/// fork point, not as built on his; the conflicting one a warning (`!`);
/// the backup on the old tip, muted and last in the stays block, never
/// `left behind:` (R1, R4); then the forecast.
#[test]
fn the_after_side_names_each_fork() {
    assert_eq!(
        facts_of(&forked()),
        [
            (
                "feature-x follows \u{b7} 2 commits from a1b2c3d".to_owned(),
                FactTone::Plain
            ),
            (
                "! feature-y stays on b2c3d4e \u{b7} would conflict in src/a.rs".to_owned(),
                FactTone::Warning
            ),
            (
                "backup stays on c3d4e5f \u{b7} on the old tip".to_owned(),
                FactTone::Muted
            ),
            ("no conflicts expected".to_owned(), FactTone::Plain),
        ]
    );
}

/// A stays row for any other reason is no warning: muted (R4).
#[test]
fn a_fork_that_stays_for_a_plain_reason_is_muted() {
    let mut plan = forked();
    plan.forks[1].fate = ForkFate::Stays(WhyStays::DefaultBranch);
    let facts = facts_of(&plan);
    assert!(
        facts.contains(&(
            "feature-y stays on b2c3d4e \u{b7} the default branch".to_owned(),
            FactTone::Muted
        )),
        "{facts:#?}"
    );
}
