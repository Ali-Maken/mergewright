//! 50k refs read from a real repository (ticket m1-the-graph/30d).
//!
//! m1-the-graph/30 timed only the pure `refs::badges` on refs made in
//! memory. These tests open a generated repository holding 25k branches,
//! each with a `[branch]` section tracking one of 25k remote-tracking refs,
//! all in `packed-refs`, and time what Graft really does with it: the
//! `refs::read` job (one upstream lookup per branch, ahead/behind, badges)
//! and open → first paint with badges on screen.
//!
//! Heavy tier only (`heavy-tests` feature, ADR-0018).

use std::sync::PoisonError;
use std::time::{Duration, Instant};

use graft::refs::{self, AheadBehindCache};
use graft::repos::RepoStore;
use graft::update::Status;
use graft_fixtures::synthetic::{self, Scale};

use crate::budget_run;
use crate::heavy::MEASURING;

/// PROMPT.md §6.3's 50k refs, on the 100k fixture with its commit-graph.
pub const FIFTY_THOUSAND_REFS: Scale = Scale::HEAVY.with_commit_graph().with_refs(50_000);

/// The `refs::read` job on 50k refs, from a fresh ahead/behind cache (what
/// opening the repository pays). A regression guard for the debug build the
/// heavy tier runs, not PROMPT.md's budget: 30d measured 0.9 s debug and
/// 165 ms release, down from 2.9 s debug when a list lookup made badges
/// quadratic and every in-sync branch opened the commit graph. Either cause
/// coming back breaks it. The graph's own first paint does not wait for this
/// job; `heavy.rs` holds that to 150 ms on this fixture.
const READ_LIMIT: Duration = Duration::from_millis(1_500);

/// Open → graph and badges both on screen, same debug guard as
/// [`READ_LIMIT`] (30d: 0.9 s debug, 0.2 s release).
const FIRST_PAINT_LIMIT: Duration = Duration::from_millis(1_500);

#[test]
fn reading_50k_real_refs_is_within_budget() {
    let repo = synthetic::borrow(FIFTY_THOUSAND_REFS).unwrap_or_else(|why| panic!("{why}"));
    // Poisoned only when another measuring test panicked; the lock guards
    // no data, so this one may still measure.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let store = RepoStore::new(repo.clone(), None);
    // Opening is its own job at startup; take it out of the read's time.
    store.shared().unwrap_or_else(|why| panic!("{why}"));

    let cache = AheadBehindCache::default();
    let started = Instant::now();
    let badges = refs::read(&store, &cache).unwrap_or_else(|why| panic!("{why}"));
    let read = started.elapsed();
    drop(turn);

    println!("refs::read on 50k refs: {read:?}");
    let head = badges
        .head()
        .unwrap_or_else(|| panic!("HEAD has no commit"));
    assert!(!badges.on(head).is_empty(), "HEAD's commit has badges");
    assert!(
        read <= READ_LIMIT,
        "refs::read took {read:?}, over the {READ_LIMIT:?} budget"
    );
}

#[test]
fn first_paint_with_50k_real_refs_is_within_budget() {
    let repo = synthetic::borrow(FIFTY_THOUSAND_REFS).unwrap_or_else(|why| panic!("{why}"));
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);

    let started = Instant::now();
    let (jobs, events, state) = budget_run::open(&repo).unwrap_or_else(|why| panic!("{why}"));
    let painted = |state: &graft::update::AppState| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
            || matches!(state.status, Status::Failed(_))
    };
    let state = budget_run::pump_until(&jobs, &events, state, painted)
        .unwrap_or_else(|why| panic!("{why}"));
    let first_paint = started.elapsed();
    drop(turn);

    println!("open -> first paint with 50k refs: {first_paint:?}");
    assert!(
        matches!(state.status, Status::Ready),
        "a job failed: {:?}",
        state.status
    );
    assert!(
        first_paint <= FIRST_PAINT_LIMIT,
        "first paint with badges took {first_paint:?}, over the {FIRST_PAINT_LIMIT:?} budget"
    );
}

/// Reading the walk's tips from 50k refs, the step the graph's first paint
/// waits on. A debug-build guard: 30d measured 424 ms debug (114 ms release,
/// three quarters of the history job's start) when every ref read and
/// decoded its commit, though the 50k refs name only 25.5k commits.
const TIPS_LIMIT: Duration = Duration::from_millis(200);

#[test]
fn reading_tips_from_50k_real_refs_is_within_budget() {
    let repo = synthetic::borrow(FIFTY_THOUSAND_REFS).unwrap_or_else(|why| panic!("{why}"));
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let store = RepoStore::new(repo.clone(), None);
    let shared = store.shared().unwrap_or_else(|why| panic!("{why}"));
    let handle = shared.handle();

    let started = Instant::now();
    let tips = handle.commit_tips().unwrap_or_else(|why| panic!("{why}"));
    let read = started.elapsed();
    drop(turn);

    println!("commit_tips on 50k refs: {read:?} ({} tips)", tips.len());
    assert!(tips.len() > 25_000, "every distinct tip is found");
    assert!(
        read <= TIPS_LIMIT,
        "commit_tips took {read:?}, over the {TIPS_LIMIT:?} budget"
    );
}
