# 06t — split the four `view/` files 06 grew past 400 (review ease-pass-06 M6, view half)

Status: draft
Blocked by: 06r-fold-fixes (edits `view/entries.rs`), 06b-lanes-on-screen-and-header (`view::rows` `Filters` fill), 06h-graph-mode, 06i-graph-mode-in-help (header/bar readers; conductor confirms against its rewrite). **Runs alone on these four files:** 07 and 09 name `view::entries` / `view::rows`; they wait for this ticket (07's Blocked by gains 06t). Runs beside 06s (disjoint: `update/`).
Build with: (blank — brownfield; scout first, conductor sets it after). Planner's guess: Sonnet, if the scout finds plain moves; the seams are decided below and the tests prove nothing moved.
Needs: none (moves only).
Review: `docs/reviews/ease-pass-06.md` M6. User, 2026-10-18 (through the conductor): code modular so future features are easy to add — split, don't raise the ratchet.

## Read first
- `docs/reviews/ease-pass-06.md` M6.
- PROMPT.md §6.2. `ARCHITECTURE.md` "Keeping crates deep".
- `~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/codebase-design/SKILL.md`.
- `crates/graft/MAP.md`: `view`, `view::rows`, `view::radar`, `view::entries`, `view::detail`.
- The scout's file.

## Scout questions (≤1 page)
For each file (`view/rows.rs` 894, `view/radar.rs` 732, `view/entries.rs` 435, `view/detail.rs` 407): sections by line range and job; inline tests; items called from outside; any move that would create a cycle or widen visibility.

## Decided here
- **Goal: each file under 400 lines and its row removed from `crates/graft/tests/it/over_400.txt`.** No behaviour change; tests pass unedited except `use` paths; nextest count equal; every capture byte-equal with no `MWR_UPDATE_SHOTS`.
- **Seams (builder may differ after the scout; say why):**
  - `view/radar.rs`: by its three jobs (MAP): facts → `Finding`s; finding → the graph row's marker (`marker_for`); finding → the detail pane's radar lines.
  - `view/rows.rs`: `rows/` already holds `context.rs`, `fold.rs`; split further by job — header `Filters` fill, painting, the window cut — per the scout.
  - `view/entries.rs`: by MAP's parts — the selection's thread, the uncommitted row, each row's node/cells/badges (06r's node helper joins the node part).
  - `view/detail.rs`: its radar lines section (`:137-142` region) or the next cohesive section, to get under 400.
- Inline tests to a sibling `tests.rs` allowed in addition, never as the whole split of a file whose non-test code is over 400. No visibility widened. CONTEXT.md names.

## Scope
In: the four files, new modules under `view/`, `over_400.txt` (rows removed, none added), `crates/graft/MAP.md`.
Out: any behaviour, name or signature change; `update/` (06s); `view::strips` (13).

## Touches (check each, report how)
Every `use` of a moved item; MAP coverage; `no_literal_keys`/sanitisation allow-lists keyed by path; captures (byte-equal, no update flag); clippy size lints; in-flight specs citing lines in these files (07 `view::entries`, 09, 13 via `view::strips` readers, the review's `entries.rs:300-308,341-346`): NOTES gives each old line's new home.

## Acceptance
Each tick names its test inline.
- [ ] Each of the four files under 400 lines; rows gone from `over_400.txt`; none added or raised. — it `file_size` ratchet test; `wc -l` and `git diff` in NOTES
- [ ] Nextest count equal; only `use` lines changed in tests; every capture test passes without `MWR_UPDATE_SHOTS`. — counts and `git diff --stat` in NOTES
- [ ] No visibility widened. — `git diff | grep '^+.*pub '` reviewed in NOTES
- [ ] MAP.md has each new module. — it `map_coverage`
- [ ] NOTES: seam per file and why; old-line → new-home table for 07, 09, 13.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k (reads ~2500 lines). Split point: `rows.rs` + `radar.rs`, then the other two.
