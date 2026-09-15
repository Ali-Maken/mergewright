# m1-the-graph/78 — Review round 2 M1: the update purity guard scans one file of seven

Status: done
Build with: **Sonnet** (medium) — one test rewritten, with a red proof.
Blocked by: none

From docs/reviews/m1-the-graph-round-2.md "Major M1" and "Minor m3".

## Read first
- docs/reviews/m1-the-graph-round-2.md "M1" and "m3" only.
- `crates/graft/src/update/tests.rs` `update_cannot_reach_the_world` (~1296–1311); `crates/graft/src/jobs/runner.rs` ~85–95.

## Scope
In:
- Strip the test section per file, not after joining. The file set must not be hand-kept: either the test asserts its list equals `update/*.rs` minus `tests.rs`, via `std::fs::read_dir` at test time (test code may use fs), or it scans the directory.
- Red proof: temporarily put a forbidden token (e.g. `Instant::now()`) in the last scanned file and show the test fail. Record the output in `target/m1-78/`, then revert.
- m3: fix `runner.rs:91`'s comment. Say why ignoring the join's `Err` is safe (the panic was already reported by `crash::install`), or use an explicit match like 77's.
Out: other guards; other swallow sites.

## Acceptance
- [ ] Guard scans every non-test update file; new files can't be missed.
- [ ] Red proof recorded, reverted.
- [ ] Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens.

## Notes
Done. `update_cannot_reach_the_world` now scans `update/*.rs` via
`std::fs::read_dir` at test time (excluding `tests.rs`), stripping only
`mod.rs`'s own `#[cfg(test)]` tail; every other file is kept whole. A new
`update/*.rs` file is picked up automatically — no hand-kept list. Red proof:
`target/m1-78/red_proof.txt` (an `Instant::now()` added to the end of
`commit_graph.rs` fails the test with "must stay pure, but its module
mentions `std::time`"); reverted, confirmed clean via `git diff --stat`.

`jobs/runner.rs:91`'s comment now says the `Err` case is a worker panic
already reported by `crash::install`'s hook, so ignoring it here is safe
(cause, not a behaviour change).

Cause: the old test joined all seven files into one string and then did a
single `split_once("#[cfg(test)]")`, which only strips right — everything
after the *first* match. Since `mod.rs`'s test marker came first in the
joined string, the split discarded not just its own tests but every file
concatenated after it, silently narrowing the purity guard to 1 of 7 files.
This cost the user: from ticket 70 onward, `keys.rs`/`events.rs`/etc. could
import `std::time`, `std::thread`, `crossterm` etc. with the guard still
green — exactly where M2's Mistake Radar was about to add code.
