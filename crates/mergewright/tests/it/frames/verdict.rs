//! What a scroll's numbers mean: the budget verdict, and the lines
//! recorded beside it.

use std::time::Duration;

use mergewright_core::capabilities::RenderingTier;

use super::run::Run;
use super::{IDLE, INPUT_TO_FRAME, Pane, Refs, idle_wakes};
use crate::frame_trace::Gate;

/// The p99 budget a scroll is judged against. 16 ms everywhere except the
/// pixel tier on the 100k fixture: pp-final (2026-11-12, release, load ~3-5)
/// measured p99 17.7 ms there (11 of 420 keys over, worst 21 ms, own frame
/// 16-21 ms with 1.1-2.6 MB of kitty bytes each), because the cursor row grew
/// a second line in ease-pass and every strip now carries more to send.
/// Ruled (user, 2026-11-12): raise the bound, keep the check. 24 ms is the
/// measured worst key (21 ms) plus ~15 %.
pub(super) fn budget(tier: RenderingTier, refs: Refs) -> Duration {
    match (tier, refs) {
        (RenderingTier::Pixel, Refs::Fixture) => Duration::from_millis(24),
        _ => INPUT_TO_FRAME,
    }
}

/// Release enforces the input-to-frame budget on the p99, debug records it
/// (ADR-0021). Every key over the budget is printed either way, and the
/// max is on the summary line (m7-power-tools/04: one outlier in 420 keys
/// no longer fails the run).
///
/// `summary` is the scroll's `frames:` p99 line. In release it prints only
/// once every other assertion has held, so a run that fails one never leaves
/// a p99 line behind that reads as a pass (review 04 Major 1); the gate's
/// own assertion comes after it, so a p99 over budget still prints its line.
pub(super) fn judge(
    tier: RenderingTier,
    refs: Refs,
    pane: Pane,
    run: &Run,
    gate: &Gate,
    summary: &str,
) {
    let limit = budget(tier, refs);
    let total = run.latencies.len() - idle_wakes().len();
    let late: Vec<_> = gate
        .over
        .iter()
        .map(|&key| {
            format!(
                "key {key}: {:?} ({})",
                run.latencies[key], run.breakdown[key]
            )
        })
        .collect();
    if cfg!(debug_assertions) {
        // ADR-0021: debug records the budget, release enforces it. Rows on
        // hand depend on pace too: whether layout stays ahead of a held
        // PageDown at 60 Hz is a question of how fast the worker runs, and
        // debug code runs it 2-5x slower.
        println!("{summary}");
        println!(
            "frames: tier={tier:?} refs={refs:?} pane={pane:?} debug build, budget recorded not asserted: {} of {} keys at or over {limit:?}: {}",
            late.len(),
            total,
            late.join(", ")
        );
        println!(
            "frames: tier={tier:?} refs={refs:?} pane={pane:?} debug build, rows on hand recorded not asserted: {} of {} keys drawn before their rows arrived: {}",
            run.waited.len(),
            run.latencies.len(),
            run.waited.join(", ")
        );
        return;
    }
    // PROMPT.md's third principle: a key draws what is on hand, never waits;
    // in release, what is on hand must already be the whole viewport.
    assert!(
        run.waited.is_empty(),
        "{} keys were drawn before their rows arrived (layout fell behind the keys): {}",
        run.waited.len(),
        run.waited.join(", ")
    );
    println!("{summary}");
    println!(
        "frames: tier={tier:?} refs={refs:?} pane={pane:?} keys over {limit:?} (recorded; the gate is the p99): {} of {}: {}",
        late.len(),
        total,
        late.join(", ")
    );
    assert!(
        gate.holds(limit),
        "p99 {:?} is over the {limit:?} input-to-frame budget at {tier:?} with {refs:?} refs ({} of {} keys over): {}",
        gate.p99,
        late.len(),
        total,
        late.join(", ")
    );
}

/// Prints what each kind of wake cost and the idle-wake keys: recorded,
/// not asserted.
pub(super) fn print_details(tier: RenderingTier, refs: Refs, run: &Run, idle: &[Duration]) {
    for (trigger, frames, sum) in &run.by_trigger {
        println!(
            "frames: tier={tier:?} refs={refs:?} trigger={trigger} frames={frames} totals: bytes={} transmits={} transmit_bytes={} placements={} deletes={} frees={}",
            sum.bytes, sum.transmits, sum.transmit_bytes, sum.placements, sum.deletes, sum.frees,
        );
    }

    let mut prefetch = run.prefetch.clone();
    prefetch.sort();
    let over = |limit: Duration| prefetch.iter().filter(|spent| **spent >= limit).count();
    println!(
        "frames: tier={tier:?} refs={refs:?} idle_prefetch_us (per frame; recorded, not asserted): frames={} p50={} p99={} max={} total={} over_1ms={} over_2ms={}",
        prefetch.len(),
        prefetch
            .get(prefetch.len() / 2)
            .map_or(0, Duration::as_micros),
        prefetch
            .get((prefetch.len().saturating_sub(1)) * 99 / 100)
            .map_or(0, Duration::as_micros),
        prefetch.last().map_or(0, Duration::as_micros),
        prefetch.iter().sum::<Duration>().as_micros(),
        over(Duration::from_millis(1)),
        over(Duration::from_millis(2)),
    );

    let idle: Vec<_> = idle_wakes()
        .iter()
        .zip(idle)
        .map(|(key, latency)| format!("{key:?}={}", latency.as_micros()))
        .collect();
    println!(
        "frames: tier={tier:?} refs={refs:?} idle_wake_us (first key after {IDLE:?} idle; recorded, not asserted): {}",
        idle.join(" ")
    );
}
