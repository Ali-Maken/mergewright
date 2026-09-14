# m1-the-graph/65 — Review M3: a wheel scroll during a far jump strands the search

Status: done
Build with: **Sonnet** (medium) — the fix and the test are named by the review; one reducer path.
Blocked by: none

From the M1 review, round 1 (docs/reviews/m1-the-graph.md "M3"). Closes KNOWN_GAPS.md row ~197 ("Wheel scroll during a far search strands the search") and the PROGRESS open question.

## Report
Cause fix. `History::scroll` withdrew rows without touching `jump`/`seeking`
(unlike `select_at`/`seek`), so once the wheel's own request superseded a
pending `Find`, the `Find`'s answer never arrives (it is superseded, not
delivered) and nothing ever clears `seeking` — "seeking" sticks in the header
until `Esc`. Cost to the user: `/` far-jump plus an idle wheel flick leaves
the header permanently wrong and the only way out is `Esc`, silently losing
the search state.

Fix: `scroll` now withdraws a pending jump itself (`self.jump.take()`,
`self.seeking = None`), symmetric with `select_at`, and forces a fresh ask
when it did — the same pattern `select_at`/`seek` already use. Chose this
over "clear `seeking` when a newer answer lands" because the newer answer
never carries the old request id at all (it supersedes it in-flight), so
that path can't observe the jump was dropped; withdrawing at the wheel event
is the only point that knows.

Regression test: `history::tests::a_wheel_scroll_during_a_far_seek_withdraws_the_seek`,
red before the fix (asserted `(None, None)`, got `(Some(id), Some(far))`), green
after. Closes KNOWN_GAPS.md row ~197.

## Read first
- docs/reviews/m1-the-graph.md "M3" only.
- `crates/graft/MAP.md`; `crates/graft/src/history/scroll.rs` around `select_at`, `scroll`, `seek` (~270–320).

## Scope
In: a failing test in `history` first. It seeks far, wheel-scrolls before the answer lands, then delivers the answers, and asserts `seeking` clears and the header can't stay "seeking". Then the cause fix: `History::scroll` withdraws the pending jump the way `select_at` does, or a newer answer clears `seeking`. Choose whichever keeps `select_at`/`seek` symmetric, and say which. Close the KNOWN_GAPS row.
Out: anything else in scroll.rs; any file split (70–74).

## Acceptance
- [x] Test red before the fix, green after.
- [x] KNOWN_GAPS row closed with the commit.
- [x] Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
