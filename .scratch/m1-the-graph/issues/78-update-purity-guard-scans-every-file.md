# m1-the-graph/78 — Review round 2 M1: the update purity guard scans one file of seven

Status: ready
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
