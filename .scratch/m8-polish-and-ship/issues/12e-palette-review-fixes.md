# G-palette 2e — palette review Majors: IN PROGRESS rows run the banner's key; a placeholder cursor reaches any landed row; `update/palette.rs` split; the matcher leaves core; ARCHITECTURE.md matches the code

Status: ready (conductor 2026-10-07) — was draft (planner spec-only pass 2026-10-07; from `docs/reviews/m8-palette-theme.md` Majors 1–4, fixed hash
`81075e6f`)
Build with: **Opus** (expected; conductor confirms) — every fix is decided below, but Major 1 sits in the modal banner
key path at a stop (a wrong route rewrote a commit), and the split moves ~1460 lines where one misplaced line changes
behaviour silently; three crates. Not Fable: no signature other tickets build on; each fix has an equivalence or
red-first oracle.
Scout: **none** — the review is the scout (file:line at `81075e6f` for every claim). The Check-first box re-confirms
those lines at the builder's base; a line that moved or no longer says what the review says → stop.
Blocked by:
- **10k merged.** 10k edits `update/banner.rs`, `banner/pressed.rs` and widens the stop fixtures
  (`banner/tests.rs`, `banner/rewrite_stop_tests.rs`) to `pub(in crate::update)`; this ticket edits
  `banner/focus_keys.rs` beside them and **reuses those fixtures**.
- **May run beside 11d** (after 10k): disjoint files. 11d holds graft-ui `graph_screen/hints*`, graft
  `update/tests.rs`, `tests/it/hint_bar.rs`, and the capture tests it re-shoots. This ticket never edits
  `update/tests.rs` (re-exports keep every `update::palette::…` path it uses); needing to → stop.
- **May run beside 12d or 22c**: disjoint files (12d: `event_loop/places.rs`, `banner/stop.rs`,
  `tests/it/outside_repo.rs`, `tests/it/palette_find.rs`; 22c: `tests/it/look_polish.rs`, search capture tests).
  This ticket changes **no capture** (box 7 proves it), so it is not in the serial capture chain. It must not touch
  `tests/it/palette_find.rs` (12d) or `update/mod.rs`.
- **Two cargo helpers at a time** (memory). Rebase on main after any of 11d/12d/22c merges; full nextest again after
  the rebase (every `docs/shots` file is asserted).
- graft-core and graft-ops: no other drafted/running ticket holds them (specs 10k, 11d, 12d, 22c read); conductor
  confirms against what is running.
Needs: graft-ui `ContextId`, `ActionId`, `KeyPress`; `update::dispatch_key`; `update::edge::context_of`;
`banner::focus_keys::banner_key`; 10k's widened stop fixtures.
**Crate set:** graft-core (`src/matcher.rs` removed, `src/lib.rs` mod line, `MAP.md`) + graft-ops (`src/matcher.rs`
new, `src/lib.rs`, `src/switch_list.rs` import line only, `MAP.md`) + graft (`src/update/palette.rs` →
`src/update/palette/`; `src/update/banner/focus_keys.rs`; import lines only at each `graft_core::matcher` site `rg`
finds; `tests/it/palette_in_progress.rs`; `MAP.md`) + `ARCHITECTURE.md` + `docs/design-notes/m8-polish-and-ship.md`
:331 (the comment only).
**Not:** `update/mod.rs`, `update/tests.rs` (11d), `tests/it/palette_find.rs` (12d), `banner/{pressed,stop}.rs`,
`banner.rs`, graft-ui, `empty_state.rs` beyond an import line, `docs/shots/*`, `KNOWN_GAPS.md`.

## Read first
- `docs/reviews/m8-palette-theme.md` Majors 1–4 (:40–87)
- `docs/design-notes/m8-polish-and-ship.md` :175–180 (Lands rows, palette); :331
- `docs/adr/0007-core-holds-plain-data-no-behaviour.md` (whole, 40 lines)
- `ARCHITECTURE.md` :158–162, :179–190, :228–236 (crate table)
- `crates/graft/MAP.md` :210–212 (`edge`, `context_of`, `dispatch_key`), :464 (`banner_key`), :704–707
  (`update::palette`); `crates/graft-core/MAP.md` :80–81; `crates/graft-ops/MAP.md` :394
- Source after the maps: `update/palette.rs` :660–700 (group spelling), :1205–1265 (`landed`, `run`);
  `banner/focus_keys.rs` :95–120

## Decisions (planner)
**D1 — Major 1: a row runs in the context it was spelled in.** Cause: `run` re-presses the row's chord through
`dispatch_key`, which routes by today's focus; IN PROGRESS rows are spelled in `ContextId::Banner`, which the claim
chain reaches only while `banner_focus` is set. Fix the route, not the focus:
- `PaletteTarget::Action` carries the `ContextId` its row was spelled in.
- `banner_key` splits into its focus guard + modal handling (Tab/BackTab/Esc, `q`, `u`, the swallow) and a new
  `banner_press` holding the action keys (`c`, `s`, `a`, `M`, `e` — exactly what `banner_key` answers with focus).
  `banner_key` calls `banner_press`; its behaviour is unchanged (its tests green, unedited).
- `run`: `ctx == Banner` → `banner_press`; any other ctx → `dispatch_key`, as today.
- **Focus is not changed.** He picked Abort; he lands where the banner's `a` lands him (the abort preview), graph focus
  as it was. Rejected: setting `banner_focus` for the dispatch (leaves the graph's keys dead afterwards — a side
  effect he did not ask for, and the next stop would open banner-focused); dispatch by `ActionId` across every
  context (the right end state, but it rewires every handler — not a review fix).
- If `RecentRuns` can hold a Banner row, it stores the ctx with the id (NOTES says whether it can, with the line).

**D2 — Major 2: the note's rule, :177 — "the cursor moves to the first real row when one lands".** In `landed()`:
anchor on a placeholder → the first runnable row of its own group if any (today's behaviour, its test unedited);
**else the first runnable row in display order, any group**; none anywhere → stays. Once on a real row it keeps its row
(as today). Fix the doc comment (:1210) to say this. Why own-group first: keeps the case already tested and the row he
moved to most nearly kept.

**D3 — Major 3: split by the four seams the review names, pure move, before nothing else changes.**
`update/palette/mod.rs` (state, `PaletteTarget`, `Group`, `open_key`, `palette_key`, `view`/`view_of`/`bar`,
re-exports) · `listing.rs` (`rows`, `listing`, `flat`, filter, spelling, `outside_source`) · `find.rs` (`ask_*`,
`branches_read`, `names_answered`, `refs_landed`, `tracked_read`, `commits_answered`, `landed`, cancel) · `run.rs`
(`run`, `first_runnable`, recent) · `input.rs` (keys, edit, `paste`, mouse) · tests beside each or `palette/tests.rs`.
Every path outside the directory (`update::palette::X`) stays valid through `mod.rs` re-exports. Production files
≤ ~400 lines each. Minors 6–7 (rows rebuilt per key; `outside_paste` duplicating `first_runnable`) sit on this seam
but are **out** — gap rows below.

**D4 — Major 4: move the matcher to graft-ops; doc fix, not an ADR exception.** ADR-0007 is the only guard keeping
core inert, and its own test of membership ("two crates that may not depend on each other both need the shape") fails
here: the callers are `graft_ops::switch_list` and `graft` (palette, `jobs::palette::match_files`), and graft already
depends on graft-ops — no new edge (`crates/graft/Cargo.toml:29`). An amending ADR would make the first algorithm in
core a precedent the next builder copies. graft-ops already owns the picker list it ranks (`switch_list`). Cost:
one file move, import lines, MAP rows. graft-ui never sees it (a future match-highlight is computed by app into the
view model, per ARCHITECTURE :249–254). If `rg` finds a graft-ui caller → **stop** (ui may not depend on ops).
ARCHITECTURE.md: :160 "Six commands" → seven, naming `Retheme` and **why** the loop carries it (read
`event_loop/running/turn.rs:303–316`; state the reason the code gives, cite the line, do not guess); :183–185 says the
six **hand the terminal over** and points at `jobs/runner.rs` `not_a_job` (:1469) as the full list of loop-carried
commands; add "**Changing the palette**" to the where-to-change list (the five `update/palette/` files,
`graft-ui/src/palette.rs`, `jobs::palette`, `graft_ops::matcher`); crate table `ops` row gains "the matcher pickers
and the palette rank by". Design note :331 comment → `// one matcher, graft-ops (12e; ADR-0007)`.

## Signatures
```rust
// graft::update::palette (mod.rs) — today `Action(ActionId)` (palette.rs:1250); other variants unchanged
pub(crate) enum PaletteTarget {
    Action { id: ActionId, ctx: ContextId }, // ctx: the context the row was spelled in
    /* Seek(Oid), File(usize), More(..), Note, theme/app rows: unchanged */
}

// graft::update::banner::focus_keys — new; banner_key = guard + modal keys + this
/// The banner's action keys (`c`, `s`, `a`, `M`, `e`), applied whatever `banner_focus` is; focus unchanged.
/// `false` for any other key. Callers: `banner_key` (focused), `update::palette::run` (an IN PROGRESS row).
pub(crate) fn banner_press(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress /*, the trailing
    parameters `banner_key` takes after `KeyPress`, unchanged — NOTES quotes the final signature */) -> bool;

// graft_ops::matcher — moved from graft_core::matcher; body, doc and tests unchanged
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32>;
```
`run`, `landed`, `first_runnable`, `rows`, `listing`: signatures unchanged (moved; NOTES quotes each).

## Acceptance (each tick names its test, at tick time)
- [ ] **Check first** (before any edit): at base, `palette.rs` :1250–1253 re-presses the chord, :677–687 spells
  IN PROGRESS in `Banner`, `focus_keys.rs:100` guards on `banner_focus`, :1219–1233 `landed` searches `in_group` only;
  and, per group, the `ContextId` its rows are spelled in (NOTES: group → ctx → line). A group other than IN PROGRESS
  spelled in a ctx that is neither `context_of` at open nor `Global` → **stop**, report with the line.
- [ ] D1 equivalence, red first on base: at a rebase edit stop (cursor on HEAD), a rebase conflict stop, a cherry-pick
  conflict stop and a merge conflict stop, banner **unfocused**: each IN PROGRESS row run from `?` yields the same
  commands and state as pressing its chord with `banner_focus = true` (focus fields excepted); after it,
  `banner_focus` and `detail.focus` are what they were. Merge lists no Skip row (assert).
  (`update::palette::tests::in_progress_rows_run_as_the_focused_banner_would`)
- [ ] D1 the review's case, red first: edit stop, graph focused, `?` → Abort row → `⏎`: no amend command emitted, the
  abort preview is up; Skip row → skip's command. (`update::palette::tests::abort_from_the_palette_never_amends`)
- [ ] D1 by the binary: rebase edit stop, graph focused, `?`, cursor to Abort, `⏎` → abort preview drawn; HEAD's oid
  unchanged (`git rev-parse HEAD` before/after). (`it::palette_in_progress::walk_abort_from_the_graph_at_an_edit_stop`)
- [ ] D1 no regression: `banner_key`'s own tests green **unedited** (`git diff <base> -- crates/graft/src/update/banner`
  shows only `focus_keys.rs`); J7's `c` walk green unedited. (those tests; the diff command)
- [ ] D1 invariant: rows opened from the graph, a focused banner and one place: every `Action` row's ctx is `Banner`,
  `Global` or `context_of` at open. (`update::palette::tests::every_action_row_names_a_context_its_run_reaches`)
- [ ] D2, red first: BRANCHES `reading…` with the cursor on it, TAGS answers → cursor on TAGS' first row; BRANCHES then
  answers → cursor stays on that tag row; every group a placeholder → cursor stays.
  (`update::palette::tests::cursor_on_placeholder_moves_to_a_landed_row_in_another_group`);
  `cursor_on_placeholder_moves_to_first_landed_row` green unedited.
- [ ] D3 pure move, its own commit: at that commit full nextest green and `git diff <pre-split>..<split> --
  crates/graft/tests` empty; outside `update/palette/` the diff is empty. `wc -l crates/graft/src/update/palette/*.rs`
  in NOTES, each production file ≤ ~400 (one over → NOTES names it and its seam). (gates; those commands)
- [ ] D4: `rg -n 'graft_core::matcher' crates` empty; `rg -n 'graft_ops' crates/graft-ui` empty;
  `graft_ops::matcher::tests::{the_table, segment_start_ranks_below_inside}` green, bodies unedited; `switch_list`
  tests unedited, green; the crate-edge test (ADR-0006; NOTES names it) green unedited. (those tests; the `rg`s)
- [ ] D4 docs: ARCHITECTURE.md reads "Seven commands", names `Retheme` with its reason and line, :183–185 points at
  `not_a_job`; "Changing the palette" row present; `ops` row amended; design note :331 amended. Check: `rg -n
  'Seven commands|Retheme|Changing the palette|not_a_job' ARCHITECTURE.md` (output in NOTES).
- [ ] Captures unchanged: `git diff --stat <base> -- docs/shots` empty. (that command)
- [ ] MAP.md: graft-core `## matcher` gone; graft-ops `## matcher` added, :394 path; graft :705 `graft_ops::matcher`,
  the `PaletteTarget::Action { id, ctx }` route, the five files; `banner_press` beside `banner_key` (:464). Check:
  `rg -n 'matcher|banner_press|palette/' crates/*/MAP.md`.

## Touches
- `run` callers: palette `⏎`, mouse click on a row, recent rows, empty state's `outside_paste` (`first_runnable` via
  re-export) — `rg -n 'palette::(run|first_runnable)' crates/graft/src` before/after in NOTES.
- Banner: `banner_key` callers (`keys::key_event`) unchanged; banner view's `focused` unchanged (focus not set).
- Journal/undo: abort and skip go through the banner's own path, journaled as with focus; `git diff <base> --
  crates/graft/src/update/{journal,undo}* crates/graft-ops/src/{apply,undo}*` empty.
- `update::tests::palette_live` (keys pressed, not rows run): green, unedited.
- Hint bar, keymap, `keymap_ids.txt`: none (empty diff shown). Captures: none (box above).
- graft-ops `switch_list` ranking and picker order: unchanged (tests unedited). graft-core: one module fewer.
- Docs naming `graft_core::matcher`: design note :331 (amended); tickets 11/12/12b and scouts are records, untouched.
- 11d (`update/tests.rs`), 12d (`palette_find.rs`, `banner/stop.rs`), 22c: no shared file (`git diff --stat` shown).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~60k: check-first 4k; D1 + tests 14k; D2 + test 5k; D3 split 22k; D4 move + docs 9k; NOTES/MAP 6k.
**Order:** D1, D2 (in the old file, red first), then D3 (moves them too), then D4. Past ~60k after D2: stop; D3–D4
go to a split (NOTES says so). D1 is the one that must land.

## NOTES

### For the conductor: review Minors and Questions → KNOWN_GAPS rows (planner proposal; conductor writes them)
| id | row | status | expiry |
|---|---|---|---|
| M8/11-files-over-size | `graft-ui/src/palette.rs` 430, `graft/src/empty_state.rs` 441, `graft/src/session.rs` 510 lines (rule ~400); split on their seams (Minor 1) → 21 | **broken** | M8 |
| M8/11-allow-no-reason | `graft-ui/src/palette.rs:350` `#[allow(clippy::too_many_arguments)]` with no `reason` (Minor 2) → 21 | **broken** | M8 |
| M8/11-rank-segment-change-untested | `switch_list` `rank`: a segment now starts after any non-alphanumeric (was `/`, `-`); picker order for `_`/`.` names changed, unreported, no test (Minor 3) | **unverified** | M8 |
| M8/11b-click-below-short-box | `view/frame/layout.rs:291–303` hit-tests the max `palette_area`, not the drawn box: a click below a short box reads `Inside` (Minor 4; reasoned, not run) | **unverified** | M8 |
| M8/12b-more-row-at-exact-cap | `update/palette.rs:589` `hits.len() >= COMMIT_HITS` with the job capped at `COMMIT_HITS`: exactly 9 hits offers `more` (Minor 5; reasoned, not run) | **unverified** | M8 |
| M8/12b-outside-paste-duplicates-first-runnable | `empty_state.rs` `outside_paste` repeats `first_runnable` (Minor 7); one call once 12e's split lands | **broken** | M8 |
| M8/16-problem-label-spelling | theme problem labelled `[theme.high-contrast]` when he wrote `high_contrast` (`config/theme.rs`; Minor 8; reasoned) | **unverified** | M8 |
| M8/12-walk-journey-numbers | `tests/it/palette_find.rs` header/walk docs say J26–J28, "J30 seek-commit-words", "J31 file-history"; note has J29, J30 (Minor 9; after 12d, which edits that file) | **broken** | M8 |
| M8/11-tests-never-red | 11's tests written after the code (its NOTES :207), no gap row (Minor 10) | **unverified** | M8 |
| M8/12-test-file-size | `tests/it/palette_find.rs` ~900 lines; split by journey (Minor 11; after 12d) → 21 | **broken** | M8 |
| M8/12-seek-under-overlay-focus | Q1: a seek from under Journal/Stash/Map sets `detail.focus = Graph` (`update/palette.rs:1258–1261`) without closing the overlay (`edge.rs:84–86`); does he see the graph focused, as :175 says? Needs a capture | **unverified** | M8 |
| M8/12b-files-row-esc-from-place | Q3: a FILES row from inside a place leaves its trail (`place/history/open.rs`, `trail::leave(…, false)`), so `Esc` lands on the graph. Ruling: compare `F` from the same place — same landing → intended (consistency), close with that test's line; `F` stacks → FILES must too (defect) | **unverified** | M8 |

- Minor 6 (listing rebuilt 3–4× per key): **no new row** — append to `M8/12-palette-perf` ("update rebuilds `rows`/`flat`
  in `edit`, `landed`, `first_runnable` + per frame, ~⅓ of p95 update").
- Q2 (find's step-5 prototype skipped, design note :97–98): not a gap — a pipeline record. Conductor confirms in
  `docs/log/m8.md` or asks the user; until then the reviewer's "unconfirmed" stands.
- Ticket 11 decision 1 ("one matcher, in graft-core") is superseded by D4; the log says so.
- Question for the conductor, not in scope: ADR-0007 is held only by review (it was broken by a planner spec, 11 :35).
  An independent check — e.g. a fast test that graft-core exports no free `pub fn` — would hold it. File or decline.
