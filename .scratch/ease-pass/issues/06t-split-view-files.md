# 06t — split the four `view/` files 06 grew past 400 (review ease-pass-06 M6, view half)

Status: ready (planner, spec-only pass 2026-10-18: Blocked by settled, paths renamed, sizes from scout `.scratch/ease-pass/scouts/07-graph-rows.md` at e9b08d12). Was: draft.
Blocked by: 06h-graph-mode (fills `Filters.every_branch` in `view::rows`; 06h:107). Built and merged, no longer blocking: 06r, 06b, 06u. 06i dropped: its Scope (06i:47) holds `update::palette` and graft-ui `palette`, no `view/` file.
**Runs alone on these four files:** 07, 07b, 09 and 13 name `view::entries` / `view::rows` / `view::strips` readers; they wait for this ticket. 11b (building now) holds `view/resolver*` only (11b:22) — disjoint. Runs beside 06s (disjoint: `update/`).
Build with: (blank — brownfield; scout first, conductor sets it after). Planner's guess: Sonnet, if the scout finds plain moves; the seams are decided below and the tests prove nothing moved.
Needs: none (moves only).
Review: `docs/reviews/ease-pass-06.md` M6. User, 2026-10-18 (through the conductor): code modular so future features are easy to add — split, don't raise the ratchet.

## Read first
- `docs/reviews/ease-pass-06.md` M6.
- PROMPT.md §6.2. `ARCHITECTURE.md` "Keeping crates deep".
- `~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/codebase-design/SKILL.md`.
- `crates/mergewright/MAP.md`: `view`, `view::rows`, `view::radar`, `view::entries`, `view::detail`, and the 06r node entry (`Node::of`, MAP "A history row's node").
- The scout's file.

## Scout questions (≤1 page)
For each file — `view/rows.rs` (844 at e9b08d12), `view/entries.rs` (428), `view/radar.rs`, `view/detail.rs` (re-measure both after 06h): sections by line range and job; inline tests; items called from outside; any move that would create a cycle or widen visibility. `view/rows/` already holds `context.rs` (`cursor_row_context` :107) and `fold.rs` (`InsideMerge` :27) — say what else.

## Decided here
- **Goal: each file under 400 lines and its row removed from `crates/mergewright/tests/it/over_400.txt`.** No behaviour change; tests pass unedited except `use` paths; nextest count equal; every capture byte-equal with no `MWR_UPDATE_SHOTS`.
- **Seams (builder may differ after the scout; say why):**
  - `view/radar.rs`: by its three jobs (MAP): facts → `Finding`s; finding → the graph row's marker (`marker_for`); finding → the detail pane's radar lines.
  - `view/rows.rs`: `rows/` already holds `context.rs`, `fold.rs`; split further by job — header `Filters` fill, painting, the window cut — per the scout. The two `PlainRow` picks at `rows.rs:308` and `:765` (07b reads them) move with whatever job they belong to; NOTES names their new homes.
  - `view/entries.rs`: by MAP's parts — the selection's thread, the uncommitted row, each row's node/cells/badges (06r's node helper joins the node part if it still sits here). The cells/badges part is the one 07 rewrites: keep it whole in one file.
  - `view/detail.rs`: its radar lines section or the next cohesive section, to get under 400.
- Inline tests to a sibling `tests.rs` allowed in addition, never as the whole split of a file whose non-test code is over 400. No visibility widened. CONTEXT.md names.

## Scope
In: the four files, new modules under `crates/mergewright/src/view/`, `crates/mergewright/tests/it/over_400.txt` (rows removed, none added), `crates/mergewright/MAP.md`.
Out: any behaviour, name or signature change; `update/` (06s); `view::strips` (13); `view/rows/context.rs` and `view/rows/fold.rs` (not over 400; 07b edits `context.rs`).

## Touches (check each, report how)
Every `use` of a moved item; MAP coverage; `no_literal_keys`/sanitisation allow-lists keyed by path; captures (byte-equal, no update flag); clippy size lints. In-flight specs citing lines in these files: 07 (`view::entries` cells and badges), 07b (`view/rows.rs:308,765`), 09, 13 (`view::strips` readers), 14, the review's `entries.rs:300-308,341-346`. NOTES gives each old line's new home.

## Acceptance
Each tick names its test inline.
- [ ] Each of the four files under 400 lines; rows gone from `over_400.txt`; none added or raised. — it `file_size` ratchet test; `wc -l` and `git diff` in NOTES
- [ ] Nextest count equal; only `use` lines changed in tests; every capture test passes without `MWR_UPDATE_SHOTS`. — counts and `git diff --stat` in NOTES
- [ ] No visibility widened. — `git diff | grep '^+.*pub '` reviewed in NOTES
- [ ] MAP.md has each new module. — it `map_coverage`
- [ ] NOTES: seam per file and why; old-line → new-home table for 07, 07b, 09, 13, 14.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k (reads ~2500 lines). Split point: `rows.rs` + `radar.rs`, then the other two.
