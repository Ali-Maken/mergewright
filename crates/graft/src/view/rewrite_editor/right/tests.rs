//! The AFTER side's facts, over scripted plans.

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_ops::preview::{Fork, ForkFate, LeftBehind, PreviewPlan, RefMove, Target, WhyStays};

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

/// The same two fork rows as the preview box (m8/14c): the follower by its
/// fork point, not as built on his; the conflicting one a warning (`!`);
/// the backup still left behind; then the forecast.
#[test]
fn the_after_side_names_each_fork() {
    let plan = forked();
    let facts: Vec<String> = after_facts(&plan, &ghost_owners(&plan))
        .iter()
        .map(|fact| fact.as_str().to_owned())
        .collect();
    assert_eq!(
        facts,
        [
            "feature-x follows \u{b7} 2 commits from a1b2c3d",
            "! feature-y stays on b2c3d4e \u{b7} would conflict in src/a.rs",
            "left behind: backup stays on the old c3d4e5f",
            "no conflicts expected",
        ]
    );
}

/// A stays row for any other reason is not a warning.
#[test]
fn a_fork_that_stays_for_a_plain_reason_is_no_warning() {
    let mut plan = forked();
    plan.forks[1].fate = ForkFate::Stays(WhyStays::DefaultBranch);
    let facts: Vec<String> = after_facts(&plan, &ghost_owners(&plan))
        .iter()
        .map(|fact| fact.as_str().to_owned())
        .collect();
    assert!(
        facts.contains(&"feature-y stays on b2c3d4e \u{b7} the default branch".to_owned()),
        "{facts:#?}"
    );
}
