# 06s — split the four `update/` files 06 grew past 400 (review ease-pass-06 M6, update half)

Status: done (builder 2026-10-18). Was: ready (planner spec-only pass 2026-10-18, second: fitted to scout `.scratch/ease-pass/scouts/06s-split-update-files.md` at 2352d87f; its 7 contradictions ruled in Decided). Was: draft.
Blocked by: none open — 06h merged (2352d87f; `graph_mode` field at `state.rs:431`). **Runs alone on these four files.** Conductor checks before starting:
- **11b** and **12b** may append to `update/state.rs` (11b:10, 12b:14); the scout could not confirm. If either one's merged diff touches `update/state.rs`, this ticket starts after that merge.
- **06i** cites `update/state.rs:1187` (`Status`) and `:1281` (`Status::done`). Don't run beside 06i; either order works, and the later ticket maps its lines through the other's NOTES.
- **07a (path B), 07, 07b, 09** wait for this ticket.
- Runs beside 06t (disjoint: `view/`).
Build with: Sonnet (builder-sonnet) — with the struct split ruled out below, every change is a move that `mod.rs` re-exports cover, pinned by an equal nextest count. That matches the scout's call for keys, selection and preview.
Needs: none (moves only).
Review: `docs/reviews/ease-pass-06.md` M6. User, 2026-10-18 (through the conductor): "expects the code modular, so future features (e.g. new shortcuts) are easy to add" — split, don't raise the ratchet.

## Read first
- `docs/reviews/ease-pass-06.md` M6.
- PROMPT.md §6.2 (files under about 400 lines). `ARCHITECTURE.md` "Keeping crates deep" (default `pub(crate)`).
- `~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/codebase-design/SKILL.md` (allowed: moving modules).
- `crates/mergewright/MAP.md`: `update`, `update::keys`, `update::state`, `update::selection`, `update::preview`.
- The scout's file (line map at 2352d87f).

## Decided here (the scout's contradictions)
- **C1 — `AppState` stays one struct; `state.rs` is lowered, not taken under 400.** The struct by itself is 492 lines (:31–522, 100+ fields). Getting under 400 would mean splitting it into sub-structs (`state.graph.x`, …), which rewrites every `state.x` reader across the crate (206 files name `state::`) and every in-flight ticket that cites a field. That is a structural redesign, not a move, and doesn't belong in a moves-only ticket. **Listed for the conductor:** an "AppState by area" ticket (Opus at least; the area boundaries are the hard-to-reverse choice) for the next architecture pass (`improve-codebase-architecture` after M7). It would run after 07/07b/09, which add fields.
  - `update/state.rs` keeps: the `AppState` struct, its imports, and the re-exports of the types moved below. Expected ≈530 lines. Its `over_400.txt` row is **lowered to the new count** (from 1318), not removed. NOTES gives the exact number.
  - Moved out, by what they hold, into children of `update/state/`. The builder may regroup after reading; never `types.rs`/`misc.rs`.
    - `impl AppState` (:545–807, :938–1048), split by job (construction vs the queries the update arms ask).
    - `Status` + `impl Status` (:1187–1312) → `state/status.rs`.
    - `Screen`, `GraphOverlay`, `Size`, `CHROME_ROWS` → `state/screen.rs`.
    - `RefsRead`, `IndexRead` → `state/reads.rs`.
    - `Stop`, `Quit`, `Banner`, `ClashState`, `HeadLanding`, `StashOnlyMarks` → by what they hold.
    - `Harness` (:834–884) → `state/harness.rs`.
  - `update/mod.rs:84–87`'s `pub use state::{…}` stays word for word: `state.rs` re-exports each moved type, so no caller's `use` changes.
- **C2 — `facts_pending` (:68) and `fold_counts` (:502) are fields;** they stay in the struct. Nothing moves to `update::facts`/`update::fold_count`.
- **C3 — no `update::status`.** `Status` goes to `update/state/status.rs`. It does not move to `view/status.rs`: that is the drawing layer.
- **C4 — `preview.rs` (429):** `resident_slice` doesn't exist, and `preview/slice.rs` (431, already in `over_400.txt`) gets nothing. `PreviewSubject`, `PreviewRequest`, `PreviewFailure`, `Planned`, `PreviewState` (:155–324) → `preview/state.rs`; `preview.rs` keeps `CannotStop`, the helpers and `open_preview`.
- **C5 — `keys/bound.rs` is `#[cfg(test)]`**, a claim table the tests check against the hint tables. It is never a home for dispatch code.
  - **The rule for a new shortcut:** one arm in `key_event`, one handler in its family's file under `keys/`, and one claim row in `bound.rs`. Toggles' handlers live in `keys/actions.rs`, beside `copy_selected_sha` (:45). 07's `#` follows exactly this.
  - `keys.rs` (474) keeps `before_any_key` and `key_event` (:167–305). The per-family fns (:65–162, :306–435) go to their families' files: existing `acknowledge`, `commit`, `edit_stop`, `escape`, `refresh`, `actions`, `predicates`, or a new one named for the family. `apply_view_change` (:436) → `keys/view_change.rs`. `remote_key` (:450) → the remote family. `move_cursor` (:463) → `keys/movement.rs`.
- **C6 — `selection.rs`:** its non-test code is 256 lines, so moving `mod tests` (:257–639) to `selection/tests.rs` is the whole split. The rule allows it because non-test is under 400. No by-job seam.
- **C7 — line drift:** the anchors above are at 2352d87f.
- **Visibility:** a moved fn the old file still calls may become `pub(super)`. That reaches only the scope that saw it before, so it is not a widening. Nothing becomes `pub(crate)` or `pub` that was narrower. Re-exports keep each type's current visibility. Module names use CONTEXT.md terms.
- Moving inline tests to a sibling `tests.rs` is allowed **in addition**, and as the whole split only where non-test code is under 400.

## Scope
In: the four files, new modules under `crates/mergewright/src/update/{state,preview,keys,selection}/`, `crates/mergewright/tests/it/over_400.txt` (`state.rs` row lowered; the other three rows removed; none added or raised), `crates/mergewright/MAP.md` (one line per new module).
Out: any behaviour, name, field or signature change; splitting `AppState` (C1); `view/` (06t); `update/palette/**` (06i; 03r's Minor 5 continuation); `preview/slice.rs`; test files over 400.

## Touches (check each, report how)
- Every `use` of a moved item: grep; the `mod.rs:84–87` re-export unchanged.
- `update::prelude`; MAP coverage (`map_coverage`).
- `no_literal_keys` allow-lists keyed by file path: a moved string's row follows it.
- `update::tests::purity`; the `file_size` ratchet; clippy `too_many_lines`/`cognitive_complexity`.
- Comments that cite `update/keys.rs` lines: `palette.rs:347`, `focus_keys.rs:210`, `finish/choices.rs:331`, `journal_put_back.rs:120`. Update each to the new home, since a stale cite misleads.
- In-flight specs citing these files: 06i (`state.rs:1187,1281`), 07 (`keys/bound.rs:78`, `keys/actions.rs:45`), 07a path B and 09 (append to `AppState`), 07b (`update/geometry.rs:311` — not moved, confirm), 12b, 14. NOTES gives each old line's new home.

## Acceptance
Each tick names its test inline.
- [x] `keys.rs` 283, `selection.rs` 258, `preview.rs` 242, rows removed; `state.rs` 531 (AppState, imports, mods, re-exports), row lowered 1318 → 531; none added or raised. — it `file_size` (passes); `wc -l` at commit.
- [x] Nextest count equal: `cargo nextest list --workspace | wc -l` = 6955 at c34ad595 and after. Test-file diff (`git diff c34ad595 --stat -- '*tests*'`): `selection/tests.rs` moved whole (de-indented 4), `preview/tests.rs` one `use` line (`Commit` added — `preview.rs` no longer imports it), two doc-comment cites (`keymap_edge/palette.rs:347`, `palette_live/dead.rs:18`). No test body edited.
- [x] No visibility widened: each `+.*pub` line of `git diff c34ad595` read — moved items keep their visibility (`commit_graph_actionable` stays `pub(in crate::update)`, `IndexRead` re-exported `pub(crate)`); new `pub(super)` only on fns the parent still calls: `keys/modal.rs` (7), `panes::{pane_and_overlay_key, editor_key}`, `remote_key`, `move_cursor`, `apply_view_change`, `PreviewState::{planned, planning, acts}`. `panes::{journal_key, stash_key}` stay private.
- [x] `update/mod.rs` unchanged — `git diff c34ad595 -- crates/mergewright/src/update/mod.rs` empty.
- [x] MAP.md has each new module — it `map_coverage` (passes).
- [x] NOTES below.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k (reads ~2900 lines; three of the four are plain moves). Split point if it runs over: `state.rs` alone, then the other three.

## NOTES (builder, 2026-10-18)
No compaction. Scout used; re-measured at c34ad595 (after 11b, 12b): state.rs still 1318, scout anchors hold.

Seams:
- `state.rs`: struct stays (C1). `state/screen.rs` (Screen, GraphOverlay, Size, CHROME_ROWS), `state/queries.rs` (first `impl AppState`: tier, rows, Uncommitted row, focus, overlay), `state/key_owner.rs` (second impl: should_quit, seed_keys, text field, modal key/bottom-row ownership, shown_status), `state/harness.rs` (Harness, Banner — what a harness pins), `state/quit.rs`, `state/stop.rs` (Stop, ClashState), `state/reads.rs` (RefsRead, IndexRead, HeadLanding), `state/stash_marks.rs`, `state/status.rs`. All re-exported from `state.rs`.
- `keys.rs` keeps `before_any_key`, `key_event`, `graph_key`, `view_key`, `write_key` (the dispatch body). New `keys/modal.rs` (box/rewrite/finish/resolver/place/banner claims, `undo_key_anywhere`), `keys/panes.rs`, `keys/remote.rs`, `keys/movement.rs`; `apply_view_change` → `keys/view_change.rs`.
- `selection.rs`: tests → `selection/tests.rs` (C6).
- `preview.rs`: five state types + `outcome_verb` (used only by `outcome_label`) → `preview/state.rs`.

Old line → new home:
- 06i: `state.rs:1187` Status → `state/status.rs:9`; `:1281` done → `state/status.rs:103`; done_with_keys → `:112`.
- 07: `keys/bound.rs:78`, `keys/actions.rs:45` — files untouched.
- 07a path B, 09: append to `AppState` in `state.rs` (struct ends :531; `graph_mode` :440).
- 07b: `update/geometry.rs:311` not moved.
- 12b, 14: `state.rs` impl lines → `state/queries.rs` / `state/key_owner.rs` by fn name; keys.rs family fns → files above.

Stale cite left: `tests/keymap_edge/sweep_swallowed.rs:26` string `"keys.rs:124-129 place_claim"` is test data (no test body edits) — now `keys/modal.rs`. `banner/focus_keys.rs:210`, `stash.rs:391` cite `keys.rs` by file and still hold. `finish/choices.rs:331`, `journal_put_back.rs:120`: grep found no `keys.rs` cite.

Pre-existing failure, not this ticket: `finish::captures::summary_captures` fails at c34ad595 too (checked out and run): `docs/shots/ease-pass/merge-look-clashed-120.txt does not match what the code draws` — likely 12b's capture.

Conductor add-on: `update/geometry.rs:485` comment now cites `view::entries::cells::row_entry` (checked on 06t's branch: `view/entries/cells.rs:20`).

Gates: fmt, clippy clean; nextest full run 6955 run, 6953 passed, 2 failed — `summary_captures` (pre-existing, above; main 2b0b777e fixes it) and `map_coverage` (my MAP bullets; fixed, rerun 18/18); `cargo test --doc` no failures.

For the conductor: "AppState by area" — split the 492-line struct into sub-structs by area (Opus+, after 07/07b/09); state.rs cannot go under 400 without it.
