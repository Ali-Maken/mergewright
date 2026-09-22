# m1-the-graph/09 — The graph screen: open on a graph and scroll it

Status: done — 09a (walk serves row requests, bounded window, scroll keys) and 09b (the graph screen) both committed
Build with: **Opus** (high) — event loop, frames fed by a streaming job, virtualization
Blocked by: 07, 08
Leaves runnable: yes — `graft <repo>` opens on a scrollable graph. The first ticket the user can look at.

Why: after this the app stops being a status screen.

## Read first
- docs/design/01-view-mockups.md §1.1–§1.3 (where the graph pane sits; ignore detail-pane content), docs/design/02-keymap.md §2.2, §2.3 (scroll keys only), docs/design/03-graph-layout-algorithm.md §3.6 (the virtualization paragraphs)
- PROMPT.md §6.3
- KNOWN_GAPS.md M1 rows "`git/git` is 85,649 commits and nothing reads it yet", "Nothing reads `AppState::history`"
- Code: `crates/graft/src/event_loop/`, `crates/graft-ui/src/status_screen/`, 07's streaming module, 08's widget

## Scope
In:
- The graph pane is the default view. The status screen stays reachable from a key (it is the only place tier and cell size show).
- Virtualized scrolling: only rows near the viewport are requested from `StreamingLayout`. `j`/`k`, arrows, PgUp/PgDn, `g`/`G`.
- Chunks that arrive are painted on the next frame. `G` on a cold walk shows progress and stays cancellable with `Esc`.
- A cursor row (a plain index). Real selection semantics are 16.
- One plain subject column beside the graph so the screen is readable; 10 replaces it.

Out:
- Columns and badges (10–12), mouse (17), promotion notice (18), search (26), pixel tier (22–25, 29).

## Acceptance
- [x] `cargo run -- <fixture>` opens on a graph and `q` quits clean (pty, exit code 0); snapshots at 80×24, 120×40, 200×60 (`graft-ui/tests/graph_screen.rs`).
- [x] Scrolling top to bottom of the largest fast-tier fixture keeps resident rows bounded — asserted (`history::tests::scrolling_top_to_bottom_keeps_resident_rows_bounded`).
- [x] `G` then `Esc` on a cold walk: viewport stays put, nothing from the cancelled job lands (`history::tests::a_withdrawn_jump_stops_walking_and_lands_nothing`, `update::tests::esc_withdraws_a_jump_before_it_cancels_the_walk`).
- [x] Open `git/git` once under a pty and record time to first graph frame (an observation, not a budget): **567 ms**, debug build, 120×40, expect timing spawn → first node glyph. This repo (137 commits) at 80×24: 793 ms. One run each; not a benchmark.
- [x] Exact command prepared for the user to try in Zed and VS Code (below).

## Notes (handoff, 2026-09-13)

Split at a clean seam: too big for one context. No compaction.

**Done (09a):**
- Protocol: history job no longer ships every row. `history::stream` takes a `Receiver<RowRequest>` and `serve`s: each `RowRequest { id, wanted: Rows(range) | Last(n) }` is answered with `HistoryUpdate::Window` (only those rows), laying out a chunk at a time as far as the request reaches, reporting `Progress { request, laid_out }` between chunks and switching to a newer request if one queued. Idle between requests (blocks on `recv`); ends when the sender drops.
- Runner: `JobRunner.row_requests: Mutex<HashMap<JobId, Sender<RowRequest>>>`; `Command::ShowRows(JobId, RowRequest)` routes; `CancelJob` and `shutdown` drop senders.
- State: `History` holds one `Window`, `rows` (laid out), `top`, `cursor`, `asked`, `jump`. `history/scroll.rs`: `apply`, `show`, `move_cursor(Motion, height)`, `withdraw_jump`, `visible(height)`. Stale ids dropped. A screen of slack asked each side.
- `update`: `Started` asks the first screen; `j/k`, arrows, PgUp/PgDn, `g`/Home, `G`/End; `Esc` withdraws a pending `G` first, else cancels the walk (old behaviour); resize re-asks; `Ctrl+R` cancels the old walk before starting a new one. `AppState::graph_rows()` = rows − `CHROME_ROWS` (2), min 1.

**Remains (09b), exact next steps:**
1. `graft-ui`: new `graph_screen` module — header line (sanitised path, walk phase / `laying out… N rows` while `History::jump` is set, `Esc cancels`), rows: `GraphColumn` + one plain subject column (`sanitize_line` on `Commit::subject()`, `fit`), cursor row `RowPaint::Highlighted`, footer key hints. Takes a view struct built in `graft::view` from `History::visible(state.graph_rows())`. Must fit `CHROME_ROWS`.
2. `AppState` screen toggle (graph default; status screen from a key — suggest `F2`, since `Tab`/`s`/`i` are taken in docs/design/02-keymap.md §2.2–2.3) and `view` dispatch.
3. insta snapshots at 80×24, 120×40, 200×60 (corpus shape via `graft-ui/tests/support/corpus.rs`).
4. pty capture to `docs/shots/09-graph-screen.txt`; open `git/git` under a pty, record time to first graph frame; write the Zed/VS Code command.
- Gotcha: the status screen's row count now means rows laid out as far as asked (≈ 2 screens at startup), not the whole walk.

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (09b, 2026-09-13)

No compaction.

**Done (09b):**
- `graft-ui::graph_screen`: `GraphView { location, walk: WalkView, problem, rows: Vec<GraphEntry> }`, `GraphScreen::new(view, theme)`, `CHROME_ROWS` (moved here; `graft::update::CHROME_ROWS` re-exports it). Header (path, walk state; `laying out… N rows  Esc cancels` during `G`), graph column capped at half the width, one subject column, cursor row bold on `surface-raised`, footer key hints (quit and F2 first, since a narrow terminal cuts from the right). An unsuperseded `Status::Failed` replaces the hints, so an error is never hidden behind the graph.
- `graft`: `AppState::screen: Screen { Graph (default), Status }`, `F2` swaps; `view::graph_view(state)` builds the view from `History::visible`.
- **Bug found by the pty capture, fixed:** the first `AppState` had a 0×0 size, and a terminal sends no resize at startup, so the walk was asked for one row and the graph drew one row until the window was resized. `run` now seeds `size` from `terminal.size()?`. Test: `event_loop::tests::graft_starts_at_the_terminals_size_not_zero_by_zero`.
- Capture: `docs/shots/09-graph-screen.txt` (this repo at 80×24, git/git at 120×40; plain and colour). **The user judges it by eye.**

**Try in Zed and VS Code** (integrated terminal):
```
cd <home____________>/dev/sandbox/gitter && cargo run --release -- .
cargo run --release -- ~/.cache/graft/git-git.git
```
Keys: `j`/`k` or arrows, `PgUp`/`PgDn`, `g`/`G`, `Esc` backs out of a `G`, `F2` status screen, `q` quits.

**Not done here, on purpose:** layout advancing in the background between frames (docs/design/03-graph-layout-algorithm.md §3.6). This ticket's scope is viewport-driven requests; sizing a background walk on git/git needs ticket 27's numbers. KNOWN_GAPS row kept, owner ticket 27.
