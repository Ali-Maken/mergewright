# detail-resize-and-persistence: `<`/`>`, divider drag, wheel regions, saved pane size

Status: partial (see NOTES) — keyboard resize + persistence done; drag/wheel/hints deferred
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

## NOTES (2026-09-18, partial — builder-sonnet)

Done:
- `<`/`>` dispatch in `detail_key` (first branch), over the existing pure `resize_pane`. Clamped to layout min and graph min; no-op (false, no command) in `Tab`, hidden, at a bound. `+`/`-` untouched (lane unfold only).
- `effective_layout` (two-step compute: kind first with no override, then that kind's `PaneSize` looked up and folded back in) — used by both `detail_key` and `view/frame.rs::render_graph_with_detail`.
- `crates/graft/src/layout.rs`: `load_split`/`save_split` at `<common_dir>/graft/layout`, non-fatal on any I/O failure (module doc explains why). 3 unit tests.
- `RepoStore::common_dir` (`crates/graft/src/repos.rs`), precedented by `commit_graph`'s common-dir use.
- `Command::SaveLayout(PaneSize)`, handled in `event_loop/running.rs` (writes via `crate::layout::save_split`, drops the `Result` — comment explains why safe), rejected in `jobs/runner.rs` (`unreachable!`, mirrors `CopyToClipboard`).
- Startup: `event_loop/startup.rs` loads `common_dir` before `repos` moves into `JobRunner`, and `terminal_and_state` loads the saved split into `state.detail.pane_size` if present.
- Capture: `docs/shots/m2-detail-keys-resize.txt` (873 B), generated via `TestBackend`, no test code left behind (`git status` clean on `detail_pane/tests.rs`).
- 6 new tests in `update/detail.rs` covering resize clamps, no-op cases, `+`/`-` never resizing, `PaneSize::for_layout`/`set_pane_cells` round-trip.
- Gates: `cargo fmt --check`, `cargo clippy --all-targets --message-format=short -- -D warnings`, `cargo test --quiet`, `cargo build --quiet` all pass at workspace level.

Deferred to KNOWN_GAPS (not done, explicit ticket scope items):
- Divider drag → `set_pane_cells` (mouse down+drag on the divider).
- Mouse wheel over the files/diff regions (needs `DetailLayout`/`DetailAreas` hit-testing, mirroring `graph_row_at`).
- Pane hints (`<> resize`) and `?` help screen — investigated: **no hints footer or help-screen infrastructure exists anywhere in the codebase yet** (no `?` handler, no hints module). This is bigger than this ticket's slice; needs its own ticket once a hints/help surface exists.
- Map-view no-op guard: out of scope per this ticket's own "Out" note (`Screen::Map` doesn't exist until ticket 19).

Ran long: read source directly for exact signatures instead of scouting throughout (flagged per standing rules). Did not hit the 40-tool-call/100k-token hard stop, but the session was compaction-summarized once mid-ticket by the harness (context ran out, not a self-triggered compact); resumed via the harness's summary and completed to a clean state without redoing verified work.

Status: partial — mouse drag/wheel and hints/help are the remainder. No further code seam needed for a follow-up ticket beyond what's listed above (drag/wheel hit-testing; hints/help infrastructure design).
