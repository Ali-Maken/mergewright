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
- [ ] 51's Acceptance boxes all hold (tick them in 51 too).
- [ ] The 6 named tests pass, unmodified in what they assert.
- [ ] 2× numbers recorded here.
- [ ] Gates clean, both clippy invocations.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Heavy/release runs: one at a time, block on them in the foreground. If you compact, say so; if a second compaction is coming, stop at a clean seam with a handoff note.
