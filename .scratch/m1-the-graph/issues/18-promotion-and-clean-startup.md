# m1-the-graph/18 — Promotion in the app, and a startup that never flashes

Status: ready
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
- Making `full_topological` cheaper (30 measures and decides).

## Acceptance
- [ ] Skewed-dates fixture: notice shown, promotion lands, selected commit id and its on-screen row unchanged (test).
- [ ] A superseded job's result is never applied (test).
- [ ] `Esc` during promotion keeps provisional order and says so.
- [ ] `graft /tmp` under M0's pty harness: record the byte stream; no alternate-screen enter. Close the gap row.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
