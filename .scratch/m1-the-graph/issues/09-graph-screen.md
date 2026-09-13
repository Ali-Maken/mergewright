# m1-the-graph/09 — The graph screen: open on a graph and scroll it

Status: ready
Build with: **Opus** (high) — event loop, frames fed by a streaming job, virtualization
Blocked by: 07, 08
Leaves runnable: yes — `graft <repo>` opens on a scrollable graph. The first ticket the user can look at.

Why: after this the app stops being a status screen.

## Read first
- DESIGN.md §1.1–§1.3 (where the graph pane sits; ignore detail-pane content), §2.2, §2.3 (scroll keys only), §3.6 (the virtualization paragraphs)
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
- Columns and badges (10–12), mouse (17), promotion notice (18), search (27, 28), pixel tier (22–26).

## Acceptance
- [ ] `cargo run -- <fixture>` opens on a graph and `q` quits clean; snapshots at 80×24, 120×40, 200×60.
- [ ] Scrolling top to bottom of the largest fast-tier fixture keeps resident rows bounded — asserted.
- [ ] `G` then `Esc` on a cold walk: viewport stays put, nothing from the cancelled job lands.
- [ ] Open `git/git` once under a pty and record time to first graph frame (an observation, not a budget).
- [ ] Exact command prepared for the user to try in Zed and VS Code.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
