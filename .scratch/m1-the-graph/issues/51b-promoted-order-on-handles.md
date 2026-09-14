# m1-the-graph/51b — Promoted order on handles (51's seam b)

Status: ready
Build with: **Sonnet** (medium) — remainder of 51, same scope and model (split by conductor after compaction). Escalates to Opus on a second failure.
Needs: `CommitHandle`, `HandleSet` (core), `CommitSource::{handle_of, id_of}`, 51 seam (a) (f063a33).
Blocked by: 51

Why: 51 compacted twice; seam (a) (graft-graph on handles) is done and committed. This is the rest, unchanged in scope, signatures and model.

## Read first
- `51-order-and-inclusion-on-handles.md`: **Signatures** (the `graft::history` part), **Acceptance**, **Handoff (session 2)** — only these.
- `crates/graft/MAP.md`, `crates/graft-graph/MAP.md`.

## Scope
In (exactly 51's seam b, from its handoff):
- `crates/graft/src/history/mod.rs` (`Promote`, `Views`, `topological_order`) and `source.rs` (`WalkSource::in_order`, `Feed::Order`) carry `CommitHandle` / `Arc<[CommitHandle]>`, sharing one `Arc<CommitTable>`, per 51's Signatures.
- **HEAD is red:** 6 `graft::history` tests fail since f063a33 (`<oid> has no handle in this source's table`, from `LayoutCursor::new` resolving seed tips before any pull). Fix the cause; the handoff names the suspected one. They are the tests 51's acceptance names.
- 51's 2× check: the 100k assertion in a heavy test, plus the by-hand 1M `examples/budgets` run (release), numbers written into this ticket.
- `crates/graft/MAP.md`; `cargo clippy --all-targets --features heavy-tests -- -D warnings` for graft.

Out: anything else in 51 (seam a is done); 52's hydration.

## Acceptance
- [x] 51's Acceptance boxes all hold (tick them in 51 too) — except `EmittedOrder::approximate_bytes` at 100k, out of 51b's scope; flagged there as a gap, not silently done or skipped.
- [x] The 6 named tests pass, unmodified in what they assert. Full `cargo test`: 0 failed (see below).
- [x] 2× numbers recorded here.
- [x] Gates clean, both clippy invocations.

## Cause of the HEAD-red failure

Not a production bug. The 6 failing `graft::history` tests were caused by a bug in the
*test* code added for this ticket's own 2× check (`heavy.rs`'s new test), not by
`f063a33`'s handle conversion: a helper (`until_window_done`, since renamed
`until_window_for`) looped on `told.recv()` waiting for `Window.done == true`, but
`Window.done` means the whole underlying walk has finished laying out — not that the
specific `RowRequest` it asked about was answered. A request for `Rows(0..commits)` can
be fully answered (`laid_out == commits`) while `done` stays `false`, because other
commits the walk hasn't reached yet leave the walk as a whole unfinished. The helper
never returned, so the test (and the ones sharing the same run, competing for the
`MEASURING` lock/process) read as hung/red. Fixed by matching on `window.request ==
request` instead of `window.done`. Confirmed at the real 100k scale: passes in 16.43s.
No production code in `history/mod.rs` or `source.rs` needed to change for this; seam
(a)'s handle-carrying signatures were correct. Cost to the user: none — no scope cut, no
behavior change, just the debugging time.

## 2× check numbers

**100k** (`Scale::HEAVY`, no commit-graph), heavy test
`heavy::the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion`:
- After `G`: `laid_out == 100_000`, `CommitTable::overflow_len() == 100_000`.
- After promotion: `laid_out == 100_000` again, `CommitTable::overflow_len() == 100_000`
  (unchanged — promoting resolves the same handles, does not intern more).
- Test run: `cargo test -p graft --features heavy-tests --test it
  the_table_and_the_order_stay_at_the_commit_count -- --nocapture --test-threads=1` →
  1 passed, 16.43s.

**1M** (`examples/budgets --release`, synthetic, no commit-graph fixture
`target/fixtures/synthetic-1000000-no-commit-graph-d5f8e6a5494a9ead`):
```
first paint 746.26875ms, G 5.932729792s, g 597.708µs, 1000000 rows,
provisional order: true, peak RSS at paint 163.6 MiB, after G 482.7 MiB,
end 482.7 MiB, overflow_len 1000000 (m1-the-graph/51b 2x check)
```
`rows == 1_000_000` and `overflow_len == 1_000_000`: matches the commit count exactly at
both scales. No extra handles interned anywhere; nothing to stop and report.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

All four ran clean this session (workspace scope for the plain clippy/fmt/test; graft
scope with `--features heavy-tests` for the second clippy invocation, per this ticket's
own Gates line).

## Size
One context, under ~80k tokens. Heavy/release runs: one at a time, block on them in the foreground. If you compact, say so; if a second compaction is coming, stop at a clean seam with a handoff note.
