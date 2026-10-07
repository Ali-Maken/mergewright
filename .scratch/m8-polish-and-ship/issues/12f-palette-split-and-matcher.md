# G-palette 2f — 12e's remainder: `update/palette.rs` split (D3), the matcher leaves core (D4), ARCHITECTURE.md matches the code

Status: ready (conductor 2026-10-08) — was draft (planner spec-only pass 2026-10-08; replaces 12e's unbuilt D3 and D4 — decisions unchanged, carried here)
Build with: **Sonnet** — both decisions are 12e's, unchanged; D3 is a pure move held by an empty test diff and the full
suite, D4 a file move held by `rg` and unedited tests. The banner route that made 12e Opus is done (D1). Not Fable: no
new signature, every step has an oracle.
Scout: **none** — 12e's Check first and NOTES "What remains" are the scout (base `f349f135`, plus D1/D2 at `3df7417d`).
Check first re-reads them at this base.
Blocked by:
- **12e's D1/D2 commit on main** (`3df7417d`).
- **May run beside 11e** (disjoint: 11e holds graft-ui, graft `view/*`, `update/tests.rs`, capture tests). Shared file:
  `crates/graft/MAP.md`, different rows — conductor merges.
- **10l waits for this** (10l edits `update/palette/tests.rs`, which D3 may move).
- **May run beside 12d or 22c** if their files still hold (12d: `event_loop/places.rs`, `banner/stop.rs`,
  `tests/it/outside_repo.rs`, `tests/it/palette_find.rs`; 22c: `tests/it/look_polish.rs`, search capture tests). Must
  not touch `tests/it/palette_find.rs` or `update/mod.rs`.
- Changes **no capture**, so not in the serial capture chain; but nextest asserts every `docs/shots` file — rebase on
  main after any of 11e/12d/22c merges and run full nextest again.
- **Two cargo helpers at a time** (memory).
Needs: nothing new — moves only.
**Crate set:** graft-core (`src/matcher.rs` removed, `src/lib.rs` mod line, `MAP.md`) + graft-ops (`src/matcher.rs`
new, `src/lib.rs`, `src/switch_list.rs` import line, `MAP.md`) + graft (`src/update/palette.rs` →
`src/update/palette/{mod,listing,find,run,input}.rs`; `src/jobs/palette.rs` import line;
`src/update/tests/keymap_edge/palette_find.rs` the one `graft_core::matcher` path (:626); `MAP.md`) +
`ARCHITECTURE.md` + `docs/design-notes/m8-polish-and-ship.md` :331 (the comment only).
**Not:** `update/mod.rs`, `update/tests.rs`, `update/banner*`, `tests/it/palette_find.rs`, graft-ui, `docs/shots/*`,
`KNOWN_GAPS.md`.

## Read first
- 12e: Decisions **D3, D4** (:67–88) — the rulings, unchanged; NOTES "What remains (for the split)" (:245–257)
- `docs/adr/0007-core-holds-plain-data-no-behaviour.md` (whole, 40 lines)
- `ARCHITECTURE.md` :158–162, :179–190, :228–236
- `crates/graft/MAP.md` :704–707 (`update::palette`); `crates/graft-core/MAP.md` :80–81; `crates/graft-ops/MAP.md` :394
- Source after the maps: `update/palette.rs` (whole, to split it); `event_loop/running/turn.rs` :303–316 (D4 docs only)

## Rulings
12e D3 and D4 stand as written. Two additions from 12e NOTES, not new decisions:
- `same_command` (used by `RecentRuns::ran` in mod.rs and `recent_rows` in listing.rs) goes beside `RecentRuns` in
  `mod.rs`, `pub(super)`. `Source::skippable` goes with the IN PROGRESS spelling in `listing.rs`.
- The test file is already `update/palette/tests.rs`; it stays one file (no per-module split — that is a second move
  inside a move). Its `use` lines may change; its bodies do not.
Order: D3 (own commit), then D4 (own commit).

## Signatures
None new, none changed. `graft_ops::matcher::fuzzy_score(query: &str, text: &str) -> Option<u32>` — moved from
`graft_core::matcher`, body, doc and tests unchanged. Every `crate::update::palette::X` path outside the directory stays
valid through `mod.rs` re-exports. NOTES quotes `run`, `landed`, `first_runnable`, `rows`, `listing` after the move.

## Acceptance (each tick names its test, at tick time)
- [ ] **Check first**: 12e NOTES' four D4 sites still the only `graft_core::matcher` users (`rg -n 'graft_core::matcher'
  crates` — output in NOTES); `rg -n 'matcher' crates/graft-ui/src` empty (a hit → **stop**: ui may not depend on ops).
- [ ] D3 pure move, its own commit: at that commit full nextest green; `git diff <pre-split>..<split> --
  crates/graft/tests` empty; outside `update/palette/` the diff is empty except `update/palette.rs` deleted; inside,
  `update/palette/tests.rs` differs in `use` lines only (`git diff -- crates/graft/src/update/palette/tests.rs | rg
  '^[+-]' | rg -v '^[+-](use |\s*$|[+-]{2})'` empty). `wc -l crates/graft/src/update/palette/*.rs` in NOTES, each
  production file ≤ ~400 (one over → NOTES names it and its seam). (gates; those commands)
- [ ] D3 behaviour held: `update::palette::tests::*` and `update::tests::palette_live` green, unedited bodies; the
  12e D1/D2 tests green (`in_progress_rows_run_as_the_focused_banner_would`, `abort_from_the_palette_never_amends`,
  `cursor_on_placeholder_moves_to_a_landed_row_in_another_group`). (those tests)
- [ ] D4 move: `rg -n 'graft_core::matcher' crates` empty; `graft_ops::matcher::tests::{the_table,
  segment_start_ranks_below_inside}` green, bodies unedited (`git diff -M` shows a rename with only the path changed);
  `switch_list` tests unedited, green; the crate-edge test (ADR-0006 — NOTES names it) green unedited.
- [ ] D4 docs: ARCHITECTURE.md reads "Seven commands", names `Retheme` and the reason the code gives with its line
  (`turn.rs`; quote, do not guess); :183–185 points at `jobs/runner.rs` `not_a_job` as the full list; "**Changing the
  palette**" row (the five `update/palette/` files, `graft-ui/src/palette.rs`, `jobs::palette`, `graft_ops::matcher`);
  crate table `ops` row gains "the matcher pickers and the palette rank by". Design note :331 →
  `// one matcher, graft-ops (12e; ADR-0007)`. Check: `rg -n 'Seven commands|Retheme|Changing the palette|not_a_job'
  ARCHITECTURE.md` (output in NOTES).
- [ ] Captures unchanged: `git diff --stat <base> -- docs/shots` empty. (that command)
- [ ] MAP.md: graft-core `## matcher` gone; graft-ops `## matcher` added, :394 path; graft :705 `graft_ops::matcher`
  and the five files with one line each. Check: `rg -n 'matcher|palette/' crates/*/MAP.md`.

## Touches
- `run` / `first_runnable` callers: `rg -n 'palette::(run|first_runnable)' crates/graft/src` before/after (NOTES).
- graft-ops `switch_list` ranking: unchanged (tests unedited). graft-core: one module fewer. graft-ui: none.
- Banner, journal/undo, keymap, hint bar, `keymap_ids.txt`: none (`git diff --stat <base> -- <each>` empty, shown).
- Records naming `graft_core::matcher` (tickets 11/12/12b, scouts): untouched.
- 10l: will edit `update/palette/tests.rs` after this merges — NOTES says whether the file moved.
- Gap rows that wait on this split (`M8/12b-outside-paste-duplicates-first-runnable`, Minor 6 perf): out — NOTES says
  they are now unblocked.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~40k: check-first 2k; D3 split 22k; D4 move 5k; ARCHITECTURE + note 5k; NOTES/MAP 5k. D3 is the one that must land:
past ~55k after it, stop, D4 to a split.
