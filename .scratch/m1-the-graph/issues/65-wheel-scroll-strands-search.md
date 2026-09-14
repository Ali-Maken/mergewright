# m1-the-graph/65 — Review M3: a wheel scroll during a far jump strands the search

Status: ready
Build with: **Sonnet** (medium) — the fix and the test are named by the review; one reducer path.
Blocked by: none

From the M1 review, round 1 (docs/reviews/m1-the-graph.md "M3"). Closes KNOWN_GAPS.md row ~197 ("Wheel scroll during a far search strands the search") and the PROGRESS open question.

## Read first
- docs/reviews/m1-the-graph.md "M3" only.
- `crates/graft/MAP.md`; `crates/graft/src/history/scroll.rs` around `select_at`, `scroll`, `seek` (~270–320).

## Scope
In: a failing test in `history` first. It seeks far, wheel-scrolls before the answer lands, then delivers the answers, and asserts `seeking` clears and the header can't stay "seeking". Then the cause fix: `History::scroll` withdraws the pending jump the way `select_at` does, or a newer answer clears `seeking`. Choose whichever keeps `select_at`/`seek` symmetric, and say which. Close the KNOWN_GAPS row.
Out: anything else in scroll.rs; any file split (70–74).

## Acceptance
- [ ] Test red before the fix, green after.
- [ ] KNOWN_GAPS row closed with the commit.
- [ ] Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
