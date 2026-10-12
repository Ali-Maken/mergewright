//! Review 28 m5: what the in-the-way read costs a switch on a big
//! repository. A measurement, not a check: ignored, and run only by
//! `scripts/m7-28-perf.sh`, which writes `docs/perf/m7-28.txt`.
//!
//! Two repositories, because the read has two costs: a long history (the
//! heavy tier's 100k-commit fixture, whose tree is three files) and a wide
//! tree (`MWR_M728_FILES` tracked files, default 20 000, where `ls-files`
//! lists them all).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::Path;
use std::time::{Duration, Instant};

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_fixtures::synthetic::{self, Scale};
use graft_git::repo::RepoHandle;
use graft_ops::facts::read_facts;
use graft_ops::in_the_way::blocking;
use graft_ops::operation::{CheckoutParams, CheckoutTarget, Operation};

use super::{TempRepo, switch_to};
use crate::support::{Never, git_version};

const RUNS: usize = 7;

/// The median of `RUNS` reads; each must find nothing in the way.
fn time_blocking(dir: &Path, op: &Operation) -> Duration {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let facts =
        read_facts(&repo, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    let mut runs: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            let found = blocking(&repo, op, &facts, &Never).expect("the read");
            let took = started.elapsed();
            assert!(found.is_empty(), "nothing is in the way: {found:?}");
            took
        })
        .collect();
    runs.sort();
    runs[RUNS / 2]
}

fn millis(took: Duration) -> String {
    format!("{:.1} ms", took.as_secs_f64() * 1000.0)
}

#[test]
#[ignore = "measurement: scripts/m7-28-perf.sh writes docs/perf/m7-28.txt"]
fn blocking_for_a_switch_on_big_repositories() {
    let long = synthetic::borrow(Scale::HEAVY).expect("the 100k fixture");
    let far = Oid::from_hex(
        std::str::from_utf8(
            &std::process::Command::new("git")
                .arg("-C")
                .arg(&long)
                .args(["rev-parse", "HEAD~50000"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim(),
    )
    .unwrap();
    let back = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Detached(far),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    let history = time_blocking(&long, &back);
    println!(
        "m7-28 100k commits, switch to HEAD~50000: {}",
        millis(history)
    );

    let files: usize = std::env::var("MWR_M728_FILES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(20_000);
    let wide = TempRepo::init();
    for n in 0..files {
        wide.write(&format!("src/d{}/f{n}.txt", n % 100), b"one\n");
    }
    wide.commit_all("wide");
    wide.git(&["switch", "-q", "-c", "other"]);
    for n in 0..100 {
        wide.write(&format!("src/d{n}/f{n}.txt"), b"changed\n");
    }
    wide.commit_all("100 files changed");
    wide.git(&["switch", "-q", "main"]);
    let tree = time_blocking(wide.path(), &switch_to("other"));
    println!(
        "m7-28 {files} tracked files, switch changing 100: {}",
        millis(tree)
    );
    println!(
        "m7-28 summary: blocking for a switch, median of {RUNS}: {} (100k commits), {} ({files} files)",
        millis(history),
        millis(tree)
    );
}
