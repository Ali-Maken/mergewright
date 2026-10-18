# 06s — split the four `update/` files 06 grew past 400 (review ease-pass-06 M6, update half)

Status: ready (planner, spec-only pass 2026-10-18: Blocked by settled, paths renamed, sizes from scout `.scratch/ease-pass/scouts/07-graph-rows.md` at e9b08d12). Was: draft.
Blocked by: 06h-graph-mode (edits `update/keys.rs` `apply_view_change` :447, `update/state.rs` `History::started` :337 and `AppState.graph_mode`). Built and merged, no longer blocking: 06r (its open boxes are questions Q1/Q2 — an answer that needs code in these files lands after this, through NOTES' table), 06c, 08, 08s, 10 (05s:4).
**Runs alone on these four files.** Conductor checks, not blockers on paper:
- **11b** (building now) and **12b** may append to `update/state.rs` (11b:10, 12b:14). If 11b's merged diff touches `update/state.rs`, this starts after 11b merges; 12b likewise.
- **06i** cites `update/state.rs:1188` (`Status`), `:1282` (`Status::done`) (06i:6). Not beside 06i; either order — the later maps its lines through the other's NOTES.
- **07, 07b, 09** wait for this ticket (07's Blocked by names it; 09 owns `update/state.rs` in wave 5).
- Runs beside 06t (disjoint: `view/`).
Build with: (blank — brownfield; scout first, conductor sets it after). Planner's guess: Sonnet, if the scout finds plain moves with no cycles; the seams are decided below and the tests prove nothing moved.
Needs: none (moves only).
Review: `docs/reviews/ease-pass-06.md` M6. User, 2026-10-18 (through the conductor): "expects the code modular, so future features (e.g. new shortcuts) are easy to add" — split, don't raise the ratchet.

## Read first
- `docs/reviews/ease-pass-06.md` M6.
- PROMPT.md §6.2 (files under about 400 lines). `ARCHITECTURE.md` "Keeping crates deep" (default `pub(crate)`).
- `~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/codebase-design/SKILL.md` (allowed: moving modules).
- `crates/mergewright/MAP.md`: `update`, `update::keys`, `update::state`, `update::selection`, `update::preview`, and the modules each file's types belong to (`update::facts`, `update::fold_count`, …).
- The scout's file.

## Scout questions (≤1 page)
For each file — `update/state.rs` (1318 at e9b08d12), `update/keys.rs` (487), `update/selection.rs`, `update/preview.rs` (re-measure both after 06h): its sections by line range and what each does; which are inline tests; which types or fns have callers outside the file; any seam where a move would make a cycle or widen visibility beyond `pub(crate)`. `update/keys/` already holds `bound.rs`, `actions.rs`, `view_change.rs` (07's scout: `bound.rs:78`, `actions.rs:45`) — say what each holds now.

## Decided here
- **Goal: each file under 400 lines and its row removed from `crates/mergewright/tests/it/over_400.txt`.** No behaviour change: every test passes unedited except `use` paths; the nextest count is equal before and after.
- **Seams (the builder may differ after the scout; say why in NOTES):**
  - `update/keys.rs`: one module per key family, so a new shortcut is one arm in the dispatch and one handler in its family's file. `apply_view_change` and its landing → `keys/view_change.rs` (exists); movement, toggles and the rest by family beside it (`bound.rs`/`actions.rs` already hold the bound-action family: a toggle like 07's `#` goes there). `keys.rs` keeps the dispatch only.
  - `update/state.rs`: `AppState` stays; the small types it is built from move to the module that owns their behaviour (`facts_pending` → `update::facts`, `fold_counts` → `update::fold_count`, `Status` and `Status::done` → `update::status` or the module that already draws its words — per the scout's list). A type with no owning module gets one named for what it holds, never `types.rs`/`misc.rs`. New `AppState` fields from later tickets still append at the end of the struct.
  - `update/selection.rs`: by job — the one-bit selection itself vs the evil-merge probe vs whatever else the scout finds.
  - `update/preview.rs`: `resident_slice` → `preview/slice.rs` (exists); `PreviewState` and its small types → `preview/state.rs`; key handling is already `preview/keys.rs`.
- Moving inline tests to a sibling `tests.rs` is allowed **in addition**, never as the whole split of a file whose non-test code is over 400.
- Visibility: nothing becomes `pub` that was `pub(crate)` or narrower. Module names use CONTEXT.md terms.

## Scope
In: the four files, the new modules under `crates/mergewright/src/update/`, `crates/mergewright/tests/it/over_400.txt` (rows removed, none added), `crates/mergewright/MAP.md` (an entry per new module, one line each).
Out: any behaviour, name or signature change; `view/` (06t); `update/palette/**` (06i holds it; 03r's Minor 5 continuation edits `update/palette/editor_tests.rs`); test files over 400 (not grown by 06).

## Touches (check each, report how)
Every `use` of a moved item (grep); `update::prelude`; MAP coverage test (`map_coverage`); `no_literal_keys` allow-lists keyed by file path (a moved string's row follows it); the `file_size` ratchet; clippy `too_many_lines`/`cognitive_complexity`; `update::tests::purity` (greps `update/` for the rendering library — a moved file stays clean). In-flight specs citing lines in these files: 07 and 07b (`update/keys/bound.rs:78`, `actions.rs:45`, `update/geometry.rs:311` — geometry is not moved, confirm), 06i (`state.rs:1188`, `:1282`), 09 (`update/state.rs` wave 5), 12b (state.rs append), 14. NOTES gives each old line's new home, for the conductor to fix those specs.

## Acceptance
Each tick names its test inline.
- [ ] Each of the four files under 400 lines; their rows gone from `over_400.txt`; no row added or raised. — it `file_size` ratchet test; `wc -l` and `git diff` in NOTES
- [ ] Nextest count equal before and after; no test body edited (only `use` lines). — both counts and `git diff --stat -- '*tests*'` in NOTES
- [ ] No visibility widened. — `git diff | grep '^+.*pub '` reviewed line by line in NOTES
- [ ] MAP.md has each new module. — it `map_coverage`
- [ ] NOTES: the seam per file and why, the old-line → new-home table for 06i, 07, 07b, 09, 12b, 14.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k (reads ~2900 lines). Split point if it runs over: `state.rs` alone, then the other three.
