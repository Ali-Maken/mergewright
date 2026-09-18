# m1-the-graph/27c — Promotion landing during a jump drops the jump (G, search)

Status: done
Build with: **Opus** (high) — concurrency between the streaming layout and the promotion swap
Blocked by: 27b
Leaves runnable: yes — `G` or a search jump pressed while promotion is swapping still arrives.

Why: found in 27b continuation 1 (see 27b "Fix C" and "Next steps" 1). `graft::history::scroll` `promoted()` clears `jump` unconditionally and never clears `seeking`. If the ADR-0008 promotion swap lands around a `G` press, the jump is lost and nothing re-asks: the app sits at the top for ever (the budget run hung for 300 s). A search jump in flight waits for ever. The race exists today but is unlikely; 27b's faster promotion (fix C) makes it common, so C can't land until this is fixed.

Conductor decision (2026-09-13): this is a latent bug, not a behaviour change. The fix restores what the user asked for: an in-flight jump or seek survives the swap (re-issued against the new layout, or preserved).

## Read first
- Ticket 27b "Continuation 1", Fix C paragraph and "Next steps"
- docs/design/03-graph-layout-algorithm.md §3.6 promotion paragraph; ADR-0008 (decision and consequences)
- `crates/graft/MAP.md`; then, via scout, `graft/src/history/scroll.rs` `promoted()` and `graft/src/update.rs` around `Motion::Bottom` and `seek_landed`

## Scope
In:
- Tests first: "`G` pressed while promotion is swapping reaches the bottom"; "a search jump in flight when promotion lands reaches its match"; plus `g`/PgDn if they share the path.
- Fix in `promoted()` (or where the jump is owned): keep or re-issue the in-flight jump/seek; clear `seeking` correctly.
- Check nothing else is dropped on the swap (selection, pinned tooltip) and note what you found.

Out: fix C's call site, D, the heavy-tier guard (→ 27d). No new public items expected; update MAP.md if one changes.

## Acceptance
- [x] The new tests fail before the fix, pass after (say how you saw both).
- [x] 27b's `next_ids` call site applied **temporarily** reproduces no hang in 3 no-cg budget runs; then leave the call site to 27d (revert) unless it is already measured and clean — say which.
- [x] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`. Heavy tier not needed.

## Size
Under ~80k tokens. Small ticket; stop with a handoff note rather than compacting.

## Result (2026-09-13, builder-opus)

**Two orderings, both fixed in `graft::history`:**
1. *Promotion asked first, key second* (the key's request carries the order): `serve` used to replace every `wanted` with `Promote::around`. Now only `Rows` is recounted; `Last` is kept, `Find` keeps its commit (fallback recounted). `promoted()` lands a matching jump/seek after anchoring, and always clears `jump` **and** `seeking` (so `seek_landed` no longer waits for ever).
2. *Key first, promotion second* (the one budgets hit: `G` = request 2, promotion = request 3 supersedes it): `History::promote` now re-asks a pending jump (`Last`, or `seek` again) instead of plain rows.
3. Found on the same path: PgDn onto rows not on hand re-used a stale anchor, snapping the cursor back on the swap. `ask()` now sets the anchor to the current selection (None when off hand); `around` returns `None` without an anchor and the worker keeps the rows by number.

**Tests** (`history/tests.rs`): `g_pressed_while_promotion_swaps_reaches_the_bottom`, `a_search_jump_in_flight_when_promotion_lands_reaches_its_match`, `promotion_asked_after_g_still_reaches_the_bottom`, `promotion_asked_after_a_search_jump_still_reaches_its_match`, `page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place`. Each seen failing before its fix (cursor 0 / seeking stuck / snapped to (0,0)) and passing after. `g` from row 0 asks nothing, so it has no race.

**Budgets, no cg, fix C's call site applied** (`target/27c/nocg-run*.txt`): the first attempt (ordering 1 fixed only) hung again (300 s, same stuck state), which exposed ordering 2. After both fixes: 3/3 runs clean, G 877–887 ms, **peak RSS 250.2–250.7 MiB** (was 290–294), footprint 162–164 MiB. **Call site kept** (measured and clean); cg not re-measured → 27d.

**Checked for other drops on the swap:** selection anchored (fixed for off-hand moves); `reanchor` cleared (the promoted request is answered around the anchor in its own view, fine). **Pinned tooltip is a row number** and moves to another commit on a swap: logged in KNOWN_GAPS, not fixed. Also logged: a wheel scroll during a far seek strands the search on the plain path.

No public items changed; MAP.md unchanged.
