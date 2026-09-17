# detail-resize-and-persistence: `<`/`>`, divider drag, wheel regions, saved pane size

Status: ready
Blocked by: 10
Build with: Sonnet — same model and scope as `10-detail-keys-and-focus`, split at that ticket's own named seam (group 2). No new decisions; signatures were fixed by the 2026-09-17 spec pass (ADR-0028).
Needs: `detail-keys-and-focus` (`detail_key`, `scroll_body`, the pure `resize_pane`/`set_pane_cells`/`PaneSize`/`stepped` and the `RIGHT_MIN`/`BELOW_MIN`/`GRAPH_MIN_COLUMNS`/`GRAPH_MIN_ROWS`/`RESIZE_STEP` consts, all already in `update/detail.rs` and unused until wired), `detail-pane-frame` (`DetailLayout::compute`'s `override_cells`, `DetailAreas`)

## Scope

Exactly the "Not done — remainder for a new ticket (group 2)" list in `10-detail-keys-and-focus.md`'s NOTES section, which is this ticket's specification. Read it in full before starting; it names every file, precedent and ordering constraint. In short:

- `<`/`>` dispatch in `detail_key` over the existing pure resize methods.
- Divider drag → `set_pane_cells`; mouse wheel over the files/diff regions (hit-testing via `DetailLayout`/`DetailAreas`, mirroring `graph_row_at` in `update/mod.rs`).
- `crates/graft/src/layout.rs`: `load_split`/`save_split`, `<common_dir>/graft/layout`; add the `common_dir()` accessor on `RepoStore` (`crates/graft/src/repos.rs`) first — precedent in `graft-git/src/commit_graph.rs`.
- `pane_size` into `view/frame.rs::render_graph_with_detail`'s `DetailLayout::compute` (two-step compute per ticket 10's Signatures section).
- Save-on-resize `Command` + `event_loop/running.rs` handling (`CopyToClipboard` precedent); startup `layout::load_split(common_dir)` → `DetailState.pane_size`.
- Pane hints (`<> resize` beside `w W v [ ]`) and `?` help.
- Capture `docs/shots/m2-detail-keys-resize.txt` (≤200 KB), user judges by eye.

Out: the Map-view no-op guard for resize (`Screen` has no `Map` variant until ticket 19; 19 or a later revisit adds it) — say so in the ticket and KNOWN_GAPS.

## Acceptance

Ticket 10's acceptance checklist, group-2 rows only (marked there against this split). Per-orientation persistence round-trips: resize in `Right`, switch to `Below`, resize, switch back, both remembered across restart.
