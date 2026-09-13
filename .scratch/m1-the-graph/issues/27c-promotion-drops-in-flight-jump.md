# m1-the-graph/27c — Promotion landing during a jump drops the jump (G, search)

Status: ready
Build with: **Opus** (high) — concurrency between the streaming layout and the promotion swap
Blocked by: 27b
Leaves runnable: yes — `G` or a search jump pressed while promotion is swapping still arrives.

Why: found in 27b continuation 1 (see 27b "Fix C" and "Next steps" 1). `graft::history::scroll` `promoted()` clears `jump` unconditionally and never clears `seeking`. If the ADR-0008 promotion swap lands around a `G` press, the jump is lost and nothing re-asks: the app sits at the top for ever (the budget run hung for 300 s). A search jump in flight waits for ever. The race exists today but is unlikely; 27b's faster promotion (fix C) makes it common, so C can't land until this is fixed.

Conductor decision (2026-09-13): this is a latent bug, not a behaviour change. The fix restores what the user asked for: an in-flight jump or seek survives the swap (re-issued against the new layout, or preserved).

## Read first
- Ticket 27b "Continuation 1", Fix C paragraph and "Next steps"
- DESIGN.md §3.6 promotion paragraph; ADR-0008 (decision and consequences)
- `crates/graft/MAP.md`; then, via scout, `graft/src/history/scroll.rs` `promoted()` and `graft/src/update.rs` around `Motion::Bottom` and `seek_landed`

## Scope
In:
- Tests first: "`G` pressed while promotion is swapping reaches the bottom"; "a search jump in flight when promotion lands reaches its match"; plus `g`/PgDn if they share the path.
- Fix in `promoted()` (or where the jump is owned): keep or re-issue the in-flight jump/seek; clear `seeking` correctly.
- Check nothing else is dropped on the swap (selection, pinned tooltip) and note what you found.

Out: fix C's call site, D, the heavy-tier guard (→ 27d). No new public items expected; update MAP.md if one changes.

## Acceptance
- [ ] The new tests fail before the fix, pass after (say how you saw both).
- [ ] 27b's `next_ids` call site applied **temporarily** reproduces no hang in 3 no-cg budget runs; then leave the call site to 27d (revert) unless it is already measured and clean — say which.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`. Heavy tier not needed.

## Size
Under ~80k tokens. Small ticket; stop with a handoff note rather than compacting.
