# m1-the-graph/10 — The graph screen: scrolling, selection, ancestry, hover

Status: ready
Build with: Opus
Blocked by: 06, 08, 09

Why: the first ticket where the three pieces meet and the app stops being a status screen. After this one, `graft ~/code/api` opens on a real graph — which is what the M1 checkpoint looks at.

## Read first
- DESIGN.md §1.1–§1.3 (layout at three widths), §2.2–§2.3 and §2.15 (keys and mouse), §5.2 (Focus mode)
- DESIGN.md §3.6 (what virtualization means here), §8.3 (data flow)
- KNOWN_GAPS.md M0 rows: "Nothing cancels a job yet", "The not-a-repo path flashes the alternate screen"
- `crates/graft/src/event_loop/` and `crates/graft-ui/src/status_screen/`

## Goal
The graph is the default view: it streams in, scrolls smoothly at any history depth, and selecting a commit lights up its ancestry.

## Scope
In:
- The graph pane replaces the status screen as the default view; the status screen's diagnostics stay reachable (they are the only place the tier and cell size are visible).
- Virtualized scrolling: only rows near the viewport are laid out and drawn. `j`/`k`, arrows, PgUp/PgDn, `g`/`G`, and the mouse wheel — **mouse is first-class** (DESIGN.md §2.15), not an afterthought.
- Streaming: first chunk paints immediately, later chunks arrive between frames, and a cold deep jump (`G` on a fresh repo) shows progress and stays cancellable.
- The provisional-order notice while a no-commit-graph repo has not yet promoted (ticket 06).
- Selection: highlights the selected commit's ancestry path and dims unrelated lanes. **Focus mode (`F`)** is the same mechanism turned up — a paint-time attribute on rows layout already produced, O(visible rows) on selection change, never a new history pass.
- Hover: a tooltip with the full message and parents. Everything in it must be reachable without a mouse (a key opens the same thing). "Contained in" needs M2's containment read — leave the line out rather than faking it, and say so in KNOWN_GAPS.md.
- `Esc` cancels the running layout/walk job; a result whose job id is not the awaited one is discarded (closes the M0 cancel gap for real, through a key press).
- Open the repository **before** entering the alternate screen, or otherwise stop `graft /tmp` flashing it (closes an M0 gap).

Out:
- Search (15), Map view and Overview strip (carried to M2 — see the README).
- Detail pane content, diffs, Mistake Radar (M2).
- Pixel tier (13/14) — this screen must work fully at the Unicode tier first.

## Acceptance
- [ ] `cargo run -- <fixture>` opens on a graph; `q` quits clean; snapshots at the three widths.
- [ ] Scrolling from top to bottom of a large fixture keeps the resident row set bounded (assert it).
- [ ] Selection moves with the keys and the mouse; ancestry highlight and Focus mode snapshot-tested at truecolor and 16 colours.
- [ ] `Esc` during a cold deep jump cancels it, the viewport stays put, and nothing from the cancelled job ever lands on screen.
- [ ] `graft /tmp` prints its error with no alternate-screen flash (re-run M0's pty harness and record the byte stream).
- [ ] Hover tooltip content is reachable from the keyboard; prove it in a test.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
