# detail-resize-and-persistence: `<`/`>`, divider drag, wheel regions, saved pane size

Status: done — `<`/`>`, drag, wheel, persistence, footer hint all landed (5ae38ac, 1556de9). `?` help deferred to M8 (planner-fable, 2026-09-18: PROMPT §8 owns keybinding help/remapping; DESIGN §1.16 overlay is the §2 keymap table — an M2 hand-typed list would be rewritten). Capture refresh for the hint/drag state left to ticket 23's gallery; KNOWN_GAPS holds both.
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

## NOTES (2026-09-18, continuation 2 — builder-sonnet)

Picked up the remainder above (divider drag, wheel routing, `<> resize` hint). Resumed from a harness-summarized context (the previous continuation of this same session compacted; noted per standing rules).

Done:
- `DetailState::dragging` (bool, not persisted) + `DetailState::scroll_body_by` (wheel-scale wrapper over `scroll_body`, looks up the current diff's content length) in `crates/graft/src/update/detail.rs`.
- `crates/graft/src/update/mod.rs`: `mouse_event` rewritten — `Down(Left)` on the divider starts a drag (swallows the click, no row selection underneath); `Drag(Left)` while dragging calls the new `drag_divider` → `DetailState::set_pane_cells` → `Command::SaveLayout`; `Up` (any button) always clears `dragging`. Wheel is routed by the new `detail_hit`/`wheel` through a `DetailHit` (`Divider`/`Files`/`Body`): `Files` moves the files cursor, `Body` scrolls the diff body, `Divider`/`None` falls through to the existing graph-viewport scroll.
  - **Purity constraint hit and fixed:** `update/` is grepped by an automated test (`update::tests::update_cannot_reach_the_world`) for any mention of the view crate's rendering library, including inside doc comments. The first draft reused `graft_ui::detail_pane::split_area`/`detail_areas` (both `Rect`-based) — failed the purity test. Reimplemented as plain `u16` arithmetic (`Bounds`, `detail_split`, `detail_hit_areas`, `percent_of`) mirroring `graft_ui::detail_pane`'s geometry exactly, same pattern already used for `RIGHT_MIN`/`BELOW_MIN` in `detail.rs`. Also had to reword a doc comment that named the library literally in prose — the grep matches doc comments too.
  - `DetailHit::Divider`'s hit zone is the pane's first column/row (no rendered divider glyph exists anywhere to grab instead) — documented as a deliberate choice, not a rendered seam.
- `graft_ui::detail_pane::split_area` (new pure fn) factored out of `view/frame.rs::split_for_detail`, used by the real frame draw; `update/mod.rs`'s `detail_split` mirrors it rather than calling it (purity, above).
- Footer hint: `graph_screen/footer.rs`'s `key_hints` generalized from `Option<&str>` to `&[&str]` so more than one conditional hint can be spliced in (lane-fold + resize); `GraphView::detail_shown` (new field) gates `<> resize`. All `GraphView` literal construction sites updated (`view/rows.rs` ×2, `graph_screen/tests.rs`, `graft-ui/tests/support/screen.rs`).
- 3 new tests in `update/tests.rs`: full divider drag/drop cycle (`Down`→`Drag`→`Up`, `pane_size`/`SaveLayout`/`dragging` lifecycle, and that further drag motion after `Up` is a no-op); wheel-over-files moves the files cursor and leaves the graph viewport alone; wheel-over-the-pane-body never falls through to the graph scroll. Needed `state.detail.spec = state.selected_spec()` primed in the fixtures — otherwise `selection::follow_selection` (which runs after every event) fires its own `ListFiles`/`ContainedIn` commands and confuses the assertions; not a bug, just how the fixture needs to be built.
- MAP.md updated: `graft-ui/MAP.md` (`split_area`, `GraphView::detail_shown`), `graft/MAP.md` (`DetailState::scroll_body_by`, `DetailState::dragging`).
- KNOWN_GAPS.md's `10b` entry rewritten: drag/wheel/hint gaps closed; also corrected the previous continuation's claim that "no hints footer infrastructure exists" — it did (`key_hints`/`HINTS`/`expand_hint`), just single-slot; generalized it instead of inventing a parallel one.
- Gates: `cargo fmt --check`, `cargo clippy --all-targets --message-format=short -- -D warnings`, `cargo test --quiet --workspace`, `cargo build --quiet --workspace --all-targets` all pass.

Not done (KNOWN_GAPS, `m2-details-and-diffs/10b`):
- `?` help screen: still no `?`-key handler or help-overlay surface anywhere. Confirmed again, independently. Needs its own ticket (docs/design/01-view-mockups.md §1.16).
- `docs/shots/m2-detail-keys-resize.txt` capture not regenerated even though the `<> resize` hint changes what's on screen while the pane is shown — ran out of budget for a pty capture this session.
- Map-view no-op guard: still correctly out of scope (no `Map` screen variant yet, ticket 19).

Used a scout for locating code where reasonable (this continuation reused the previous continuation's file/line map rather than re-reading source cold); read source directly for the exact geometry math and the purity-test fallout, which a scout summary could not have carried the precision needed for (mirroring arithmetic bug-for-bug). Compacted once, mid-ticket, by the harness (noted here per standing rules) — the conductor should treat the remainder (`?` help, capture) as split-off work if it wants it in this milestone, not a new redo of what's above.
