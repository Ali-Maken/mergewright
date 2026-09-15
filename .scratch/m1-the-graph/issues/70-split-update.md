# m1-the-graph/70 — Review M1: split update.rs under the 400-line rule

Status: done
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 65, 66, 67, 68, 69

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split into 70–76, one crate area each, after the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft/src/update.rs (1,542 non-test lines).

## Scope
In: split crates/graft/src/update.rs (1,542 non-test lines), e.g. update/{keys,history,search,refs,jobs}.rs. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, list every file in these crates over 400 non-test lines (`wc`-based, tests excluded) and fix the ones this ticket names; list the rest in the report.
Out: behaviour changes, renames of public items, anything outside the named files' crates.

## Acceptance
- [x] Named files split; each new file ≤400 non-test lines.
- [x] No test changed except import paths; all gates green; no snapshot changes.
- [x] MAP.md (and ARCHITECTURE.md if needed) updated.
- [x] Report: before/after line counts; any file still over 400 in these crates.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.

## Report

`crates/graft/src/update.rs` (1,546 non-test lines) split into
`crates/graft/src/update/`:
- `mod.rs` (337) — doc comment, dispatcher (`update`, `answer`, `core_event`,
  `took_fact`, `side_job_ended`), mouse handling.
- `state.rs` (305) — `AppState` and the plain state types (`Screen`,
  `Harness`, `Banner`, `Quit`, `Status`, `Size`).
- `keys.rs` (324) — the graph's own keys, movement, view changes.
- `events.rs` (194) — `AppEvent`, `Progress`, `Task`, `Product`, `GitCheck`,
  `Command`, `CommitGraphOffer`.
- `search.rs` (229) — the `/` prompt, running search, stepping hits.
- `commit_graph.rs` (140) — the commit-graph offer state machine.
- `history.rs` (96) — walk progress and topological-sort promotion.
- `tests.rs` (1,811 lines, unchanged content) — kept as one file: the
  original `mod tests` referenced only two private symbols (`answer`,
  `HEADER_ROWS`, both still defined in `mod.rs`) and otherwise called the
  public `update()`, so no test needed splitting, only its `use` lines
  (glob imports that used to ride in on `update.rs`'s own top-of-file
  `use`s now need to be explicit — this is the "import paths" adjustment).
  One test, `update_cannot_reach_the_world`, reads its own module's source
  to check for forbidden std/terminal imports; updated to `include_str!`
  every `update/*.rs` file except `tests.rs` instead of the one file it
  used to be.

All named files ≤400 non-test lines; public paths (`crate::update::AppState`
etc.) unchanged via `pub use` re-exports in `mod.rs`, so no MAP.md or
ARCHITECTURE.md edit was needed (neither names `update.rs` as a file).

Other files over 400 non-test lines in these crates (out of scope, ticket
names only `update.rs`; ticket order runs 70–74, so most are presumably
picked up by 71–74): `view.rs` (704), `graph_screen/mod.rs` (1,130),
`repo.rs` (805), `jobs/mod.rs` (802), `layout/stream.rs` (777),
`event_loop/mod.rs` (766), `history/mod.rs` (713), `raster.rs` (654),
`layout.rs` (586), `graph_column/mod.rs` (583), `git/history/mod.rs` (516),
`git/history/topological.rs` (502), `history/scroll.rs` (495),
`term/input/decode.rs` (451), `term/kitty.rs` (448), `graph/inclusion.rs`
(436), `config/mod.rs` (431), `term/detect/mod.rs` (413).

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`,
`cargo test` — all pass (110+ unit tests, 27 test binaries green, 0
failures). No snapshot files touched (`git status` shows only the
`update.rs` deletion and the new `update/` directory).
