//! ease-pass/24 timing probe, copied in by scripts/24-hints-fitting.sh and
//! removed again; never part of the build.
use super::*;

#[test]
fn bench_hints_for() {
    let contexts = every_context();
    let mut calls = 0u32;
    let mut cells = 0usize;
    let start = std::time::Instant::now();
    for _ in 0..3 {
        for context in &contexts {
            for divider in [Divider::Shown, Divider::Hidden] {
                for columns in 40..=200u16 {
                    cells += hints_for(context, divider, columns, Keymap::shipped()).len();
                    calls += 1;
                }
            }
        }
    }
    let ns = start.elapsed().as_nanos() / u128::from(calls);
    println!("BENCH calls={calls} hints_drawn={cells} ns_per_call={ns}");
}
