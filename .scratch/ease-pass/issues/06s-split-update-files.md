# 06s — split the four `update/` files 06 grew past 400 (review ease-pass-06 M6, update half)

Status: draft
Blocked by: 06r-fold-fixes (edits `update/preview.rs`), 06c-fold-seek-reveal (update::search/detail state, likely `update/state.rs`), 06h-graph-mode, 06i-graph-mode-in-help (`update::keys::view_change`; conductor confirms against its rewrite). **Runs alone on these four files:** no in-flight ticket may hold them. 07, 08, 09, 10 name them in Scope (`update/state.rs`, `update/keys.rs`, `update/preview.rs`): whichever starts later waits; the conductor sets their Blocked by lines. Runs beside 06t (disjoint: `view/`).; conductor 2026-10-18: also 08-commit-screen, 10-merge-preview (both grow update/; split once, after them)
Build with: (blank — brownfield; scout first, conductor sets it after). Planner's guess: Sonnet, if the scout finds plain moves with no cycles; the seams are decided below and the tests prove nothing moved.
Needs: none (moves only).
Review: `docs/reviews/ease-pass-06.md` M6. User, 2026-10-18 (through the conductor): "expects the code modular, so future features (e.g. new shortcuts) are easy to add" — split, don't raise the ratchet.

## Read first
- `docs/reviews/ease-pass-06.md` M6.
- PROMPT.md §6.2 (files under about 400 lines). `ARCHITECTURE.md` "Keeping crates deep" (default `pub(crate)`).
- `~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/codebase-design/SKILL.md` (allowed: moving modules).
- `crates/graft/MAP.md`: `update`, `update::keys`, `update::state`, `update::selection`, `update::preview`, and the modules each file's types belong to (`update::facts`, `update::fold_count`, …).
- The scout's file.

## Scout questions (≤1 page)
For each file (`update/state.rs` 1312, `update/selection.rs` 639, `update/keys.rs` 487, `update/preview.rs` 470): its sections by line range and what each does; which are inline tests; which types or fns have callers outside the file; any seam where a move would make a cycle or widen visibility beyond `pub(crate)`.

## Decided here
- **Goal: each file under 400 lines and its row removed from `crates/graft/tests/it/over_400.txt`.** No behaviour change: every test passes unedited except `use` paths; the nextest count is equal before and after.
- **Seams (the builder may differ after the scout; say why in NOTES):**
  - `update/keys.rs`: one module per key family, so a new shortcut is one arm in the dispatch and one handler in its family's file. `apply_view_change` and its landing → `keys/view_change.rs` (exists); movement, toggles and the rest by family beside it. `keys.rs` keeps the dispatch only.
  - `update/state.rs`: `AppState` stays; the small types it is built from move to the module that owns their behaviour (`facts_pending` → `update::facts`, `fold_counts` → `update::fold_count`, and so on, per the scout's list). A type with no owning module gets one named for what it holds, never `types.rs`/`misc.rs`.
  - `update/selection.rs`: by job — the one-bit selection itself vs the evil-merge probe (`selection.rs:208`) vs whatever else the scout finds.
  - `update/preview.rs`: `resident_slice` → `preview/slice.rs` (exists); `PreviewState` and its small types → `preview/state.rs`; key handling is already `preview/keys.rs`.
- Moving inline tests to a sibling `tests.rs` is allowed **in addition**, never as the whole split of a file whose non-test code is over 400.
- Visibility: nothing becomes `pub` that was `pub(crate)` or narrower. Module names use CONTEXT.md terms.

## Scope
In: the four files, the new modules under `update/`, `over_400.txt` (rows removed, none added), `crates/graft/MAP.md` (an entry per new module, one line each).
Out: any behaviour, name or signature change; `view/` (06t); test files over 400 (not grown by 06).

## Touches (check each, report how)
Every `use` of a moved item (grep); `update::prelude`; MAP coverage test (`map_coverage`); `no_literal_keys` allow-lists keyed by file path (a moved string's row follows it); the `file_size` ratchet; clippy `too_many_lines`/`cognitive_complexity`; in-flight tickets' line refs into these files (07, 08, 09, 10 cite `state.rs:96`, `:252`, `update/preview.rs:238`, `keys.rs:78`): NOTES gives each old line's new home, for the conductor to fix those specs.

## Acceptance
Each tick names its test inline.
- [ ] Each of the four files under 400 lines; their rows gone from `over_400.txt`; no row added or raised. — it `file_size` ratchet test; `wc -l` and `git diff` in NOTES
- [ ] Nextest count equal before and after; no test body edited (only `use` lines). — both counts and `git diff --stat -- '*tests*'` in NOTES
- [ ] No visibility widened. — `git diff | grep '^+.*pub '` reviewed line by line in NOTES
- [ ] MAP.md has each new module. — it `map_coverage`
- [ ] NOTES: the seam per file and why, the old-line → new-home table for 07/08/09/10.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k (reads ~2900 lines). Split point if it runs over: `state.rs` alone, then the other three.
