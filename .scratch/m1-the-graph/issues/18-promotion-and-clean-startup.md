# m1-the-graph/18 — Promotion in the app, and a startup that never flashes

Status: done
Build with: **Opus** (high) — concurrency: swapping a live layout under the user
Blocked by: 16
Leaves runnable: yes — repos without a commit-graph file settle into true topological order without the view jumping.

Why: ADR-0008's promotion exists in `graph` only. Doing the swap wrong moves the user's selection out from under them.

## Read first
- ADR-0008
- DESIGN.md §3.6 (the promotion paragraphs only)
- KNOWN_GAPS.md rows "ADR-0008's promotion is built in `graph`, not in the app", "Promotion hands the app a swap to get right", "The not-a-repo path flashes the alternate screen", "The corpus cannot tell date order and topological order apart"
- Code: `from_order` in `crates/graft-graph/src/layout/stream.rs`, 07's streaming module, the app's session entry point in `crates/graft/src/`

## Scope
In:
- On a repo without a commit-graph file, run `Walk::full_topological` as a background job; show a provisional-order notice until it lands.
- Swap: advance the promoted layout to the anchor before swapping; selection and viewport stay on the same commit.
- A result from a superseded job id is discarded.
- `Esc` cancels promotion; the notice then says order stays provisional.
- Open the repository before entering the alternate screen, so `graft /tmp` never flashes.
- Use (or add) a corpus shape with skewed committer dates so date and topological order differ.

Out:
- Making `full_topological` cheaper (27 measures and decides).

## Acceptance
- [x] Skewed-dates fixture: notice shown, promotion lands, selected commit id and its on-screen row unchanged (test).
- [x] A superseded job's result is never applied (test).
- [x] `Esc` during promotion keeps provisional order and says so.
- [x] `graft /tmp` under M0's pty harness: record the byte stream; no alternate-screen enter. Close the gap row.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (builder, Opus)
- **Compacted once mid-ticket.** Finished in the second context.
- **Deviation from "Read first":** the promotion does not use `StreamingLayout::from_order`, which bypasses `Included`'s filtering. `WalkSource::in_order` streams the sorted ids under `Included` instead, so one order serves every view (full, first-parent, folded).
- **Swap protocol:** every `RowRequest` carries `Promote { order, anchor, height }` until a `Window { promoted: true }` answers the newest request. `Views::promote` lays the new layout out to the anchor before it swaps, and a repeat of the same order is a no-op.
- **Tests:**
  - `history::tests::promotion_keeps_the_selected_commit_where_it_is_on_screen` (skewed-dates, through `serve`). Selected id and `cursor - top` are unchanged; the row number moves 12 to 11.
  - `update::tests::a_superseded_sort_is_never_applied`.
  - `esc_while_sorting_keeps_the_provisional_order_and_says_so`.
  - `a_failed_sort_leaves_the_order_provisional`.
  - `tests/it/not_a_repo.rs`.
- **Captures:**
  - `docs/shots/18-not-a-repo.txt`: byte stream, exit 1, no `ESC[?1049h`.
  - `docs/shots/18-promotion-skewed-dates.txt`: the `date order, sorting…` frame, then the promoted order.
- **Not closed:** `1` beyond 2 screens still resets the selection (KNOWN_GAPS row unchanged).
- **New gaps:** the sort doesn't poll stop, the walk is blocked while it reaches the anchor, a long path truncates the notice, and `Esc` hasn't been seen in a pty.
