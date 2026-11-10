//! The git side of "lost work", with a stand-in for Mergewright's three steps
//! (mergewright-ops' `tests/it/lost_work.rs` runs the real ones).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::Path;

use super::{DISCARDED_EDIT, EDITED, END, LOST_EDIT, LostWork, Mergewright, START, at, date};
use crate::git;

/// Records each call, and does the plain-git equivalent of what Mergewright's
/// operation leaves on disk.
#[derive(Default)]
struct StandIn {
    calls: Vec<(&'static str, i64)>,
}

impl Mergewright for StandIn {
    fn fetch(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        self.calls.push(("fetch", at_ms));
        git::run_at(work, &["fetch", "-q", "origin"], &date(at_ms / 1000))
            .map(|_| ())
            .map_err(|why| why.to_string())
    }

    fn discard(&mut self, work: &Path, path: &str, at_ms: i64) -> Result<(), String> {
        assert_eq!(std::fs::read(work.join(path)).unwrap(), DISCARDED_EDIT);
        self.calls.push(("discard", at_ms));
        git::run(work, &["checkout", "--", path])
            .map(|_| ())
            .map_err(|why| why.to_string())
    }

    fn undo_forced(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        assert_eq!(std::fs::read(work.join(EDITED)).unwrap(), LOST_EDIT);
        self.calls.push(("undo", at_ms));
        std::fs::write(work.join(EDITED), DISCARDED_EDIT).map_err(|why| why.to_string())
    }
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = git::run(dir, args).expect("git runs");
    String::from_utf8_lossy(&out).trim_end().to_owned()
}

#[test]
fn start_is_10_12_and_end_is_10_44_utc() {
    assert_eq!(START % 86_400, 10 * 3600 + 12 * 60);
    assert_eq!(END - START, 32 * 60);
    assert_eq!(at(10, 12, 0), START);
}

#[test]
fn mergewright_discards_and_undoes_before_the_spike_and_fetches_last() {
    let mut app = StandIn::default();

    LostWork::build(&mut app).expect("the fixture builds");

    assert_eq!(
        app.calls,
        [
            ("discard", at(10, 16, 0) * 1000),
            ("undo", at(10, 16, 40) * 1000),
            ("fetch", END * 1000),
        ]
    );
}

#[test]
fn d_and_e_are_held_by_nothing_and_s_by_the_stash() {
    let fixture = LostWork::build(&mut StandIn::default()).expect("the fixture builds");
    let work = fixture.work();
    let commits = fixture.commits();

    for lost in [&commits.d, &commits.e] {
        let holders = git_out(work, &["for-each-ref", "--contains", lost]);
        assert_eq!(holders, "", "{lost} is held");
    }
    assert_eq!(git_out(work, &["rev-parse", "main"]), commits.c);
    // Off C, not D: two lost runs, not one run with D folded under E.
    assert_eq!(
        git_out(work, &["rev-parse", &format!("{}^", commits.e)]),
        commits.c
    );
    assert_eq!(
        git_out(work, &["rev-parse", &format!("{}^", commits.d)]),
        commits.c
    );
    assert_eq!(git_out(work, &["rev-parse", "refs/stash"]), commits.s);
    assert_eq!(git_out(work, &["rev-parse", "refs/stash^"]), commits.c);
    assert_eq!(git_out(work, &["branch", "--list", "spike"]), "");
    assert_eq!(
        git_out(work, &["rev-parse", "origin/main"]),
        commits.fetched
    );
    assert_eq!(
        git_out(work, &["log", "--format=%s", "main..origin/main"]),
        "Add an ops runbook\nDocument the api"
    );
    assert_eq!(
        git_out(work, &["log", "-1", "--format=%s", &commits.d]),
        "Add rate limiter"
    );
    assert_eq!(
        git_out(work, &["log", "-1", "--format=%s", &commits.e]),
        "Try redis cache"
    );
}

/// HEAD's log is what the list reads for the shell's rows: every record at
/// its fixed second, inside 10:12–10:44.
#[test]
fn heads_log_is_the_shells_steps_at_fixed_times() {
    let fixture = LostWork::build(&mut StandIn::default()).expect("the fixture builds");

    let log = git_out(
        fixture.work(),
        &["reflog", "show", "--format=%gd %gs", "--date=unix", "HEAD"],
    );

    let expected = [
        // 10:42: a pathspec stash (`-- src/metrics.rs`) writes no HEAD record.
        (at(10, 41, 0), "reset: moving to HEAD~1"),
        (at(10, 37, 0), "checkout: moving from spike to main"),
        (at(10, 21, 0), "commit: Try redis cache"),
        (at(10, 18, 0), "checkout: moving from main to spike"),
        (at(10, 15, 0), "commit: Add rate limiter"),
        (at(10, 14, 0), "commit: Add metrics endpoint"),
        (at(10, 13, 0), "commit: Add request logging"),
        (at(10, 12, 0), "commit (initial): Start the api"),
    ]
    .map(|(at, subject)| format!("HEAD@{{{at}}} {subject}"))
    .join("\n");
    assert_eq!(log, expected);
}

/// 22c: the shell's spike steps run after Mergewright's undo and leave his lost
/// edit showing: the file on disk is the one the undo put back, and neither
/// E nor the stash carries it.
#[test]
fn the_spike_steps_leave_the_undone_edit_on_disk_and_out_of_e() {
    let fixture = LostWork::build(&mut StandIn::default()).expect("the fixture builds");
    let work = fixture.work();
    let commits = fixture.commits();

    assert_eq!(std::fs::read(work.join(EDITED)).unwrap(), DISCARDED_EDIT);
    assert_ne!(DISCARDED_EDIT, LOST_EDIT);
    let e_parent = format!("{}^", commits.e);
    assert_eq!(
        git_out(work, &["diff", "--name-only", &e_parent, &commits.e]),
        "src/cache.rs"
    );
    assert_eq!(
        git_out(work, &["diff", "--name-only", "refs/stash^", "refs/stash"]),
        "src/metrics.rs"
    );
}

#[test]
fn the_ids_are_the_same_every_build() {
    let first = LostWork::build(&mut StandIn::default()).expect("the fixture builds");
    let second = LostWork::build(&mut StandIn::default()).expect("the fixture builds");

    assert_eq!(first.commits(), second.commits());
}
