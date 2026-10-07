# G-palette 2e — palette review Majors: IN PROGRESS rows run the banner's key; a placeholder cursor reaches any landed row; `update/palette.rs` split; the matcher leaves core; ARCHITECTURE.md matches the code

Status: **partial** — D1, D2 done (commit `3df7417d`); D3, D4 not started, to a split (builder 2026-10-07). Was: ready (conductor 2026-10-07) — was draft (planner spec-only pass 2026-10-07; from `docs/reviews/m8-palette-theme.md` Majors 1–4, fixed hash
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
- [x] **Check first** (before any edit): at base, `palette.rs` :1250–1253 re-presses the chord, :677–687 spells
  IN PROGRESS in `Banner`, `focus_keys.rs:100` guards on `banner_focus`, :1219–1233 `landed` searches `in_group` only;
  and, per group, the `ContextId` its rows are spelled in (NOTES: group → ctx → line). A group other than IN PROGRESS
  spelled in a ctx that is neither `context_of` at open nor `Global` → **stop**, report with the line.
- [x] D1 equivalence, red first on base: at a rebase edit stop (cursor on HEAD), a rebase conflict stop, a cherry-pick
  conflict stop and a merge conflict stop, banner **unfocused**: each IN PROGRESS row run from `?` yields the same
  commands and state as pressing its chord with `banner_focus = true` (focus fields excepted); after it,
  `banner_focus` and `detail.focus` are what they were. Merge lists no Skip row (assert).
  (`update::palette::tests::in_progress_rows_run_as_the_focused_banner_would`)
- [x] D1 the review's case, red first: edit stop, graph focused, `?` → Abort row → `⏎`: no amend command emitted, the
  abort preview is up; Skip row → skip's command. (`update::palette::tests::abort_from_the_palette_never_amends`)
- [x] D1 by the binary: rebase edit stop, graph focused, `?`, cursor to Abort, `⏎` → abort preview drawn; HEAD's oid
  unchanged (`git rev-parse HEAD` before/after). (`it::palette_in_progress::walk_abort_from_the_graph_at_an_edit_stop`)
- [x] D1 no regression: `banner_key`'s own tests green **unedited** (`git diff <base> -- crates/graft/src/update/banner`
  shows only `focus_keys.rs`); J7's `c` walk green unedited. (those tests; the diff command)
- [x] D1 invariant: rows opened from the graph, a focused banner and one place: every `Action` row's ctx is `Banner`,
  `Global` or `context_of` at open. (`update::palette::tests::every_action_row_names_a_context_its_run_reaches`)
- [x] D2, red first: BRANCHES `reading…` with the cursor on it, TAGS answers → cursor on TAGS' first row; BRANCHES then
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
- [x] Captures unchanged: `git diff --stat <base> -- docs/shots` empty. (that command)
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

### Builder 2026-10-07 (Opus) — partial: D1 + D2 landed; D3, D4 to a split

No scout: the ticket says the review is the scout; Check first re-read every cited line at base `f349f135` (below).
No compaction. Stopped after D2 per **Size** ("past ~60k after D2: stop") — context well past that.

**Check first** (base `f349f135`, all as the review says): `palette.rs:1250-1253` `Action(id)` → `dispatch_key(chord)`;
`:677-687` IN PROGRESS from `keymap.in_context(ContextId::Banner)`, spelled `key_in(keymap, ContextId::Banner, b)`;
`focus_keys.rs:100` `if !state.banner_focus || banner_view(state).is_none() { return false; }`; `:1219-1233` `landed`
searches `in_group` only. Group → ctx → line: YOUR CONFIG → none (`Note`) :660-676; IN PROGRESS → `Banner` :683;
HERE / EVERYWHERE → `at = palette.opened_from` :658, :764, :769 (`place_rows`), and `opened_from` is
`super::edge::context_of(state)` at open :336; ACTIONS (`⌃P`) = the same entries (`everything` :730); COMMANDS RUN
RECENTLY → copies of ACTIONS entries (`recent_rows` :715). No group spelled outside {Banner, Global, context_of at
open} — no stop.

**Ticks, each with its test** (all in `crates/graft/src/update/palette/tests.rs` unless said):
- Equivalence: `update::palette::tests::in_progress_rows_run_as_the_focused_banner_would`. Red on base, seen: failed
  `rebase edit stop: Skip this commit: state` (the graph has no `s`). Covers edit stop (`rewrite_stop_tests::edit_stop`),
  rebase / cherry-pick / merge conflict stops (`banner::tests::stopped`, `banner_focus` set false). **Fixture note:**
  the conflict stops set `mergetool = Some("vimdiff")`. Without a tool, `M`'s refusal words are spelled for
  `edge::context_of` (`banner/resume.rs:73-77`): focused → `e edits a.rs  a abort`, from the palette (graph focus
  kept) → no keys. That is correct — the graph's `a` amends — and it is a focus-derived field, so the oracle compares
  the handover command instead.
- Review's case: `update::palette::tests::abort_from_the_palette_never_amends`. Red on base, seen: failed at "nothing
  amends" (`ReadHeadMessage` emitted = the graph's `a`).
- Binary: `it::palette_in_progress::walk_abort_from_the_graph_at_an_edit_stop` (real git, `rebase -i` with
  `sequence.editor` → `edit`). Red on base, seen (base production files checked out over the test): failed at
  "the abort preview" (`palette_in_progress.rs:165`). Green: `Abort the rebase?` drawn, `operating` none, HEAD's oid
  equal after `app.close()`, still rebasing. No capture written (asserts on the frame only).
- No regression: `git diff f349f135 -- crates/graft/src/update/banner` → only `banner/focus_keys.rs` (shown); banner
  tests and `it::palette_in_progress::j7_…` green unedited (full nextest below).
- Invariant: `update::palette::tests::every_action_row_names_a_context_its_run_reaches` (graph at an edit stop, a
  focused banner, the journal; `?` and `⌃P` each; also asserts `opened_from == context_of` at open).
- D2: `update::palette::tests::cursor_on_placeholder_moves_to_a_landed_row_in_another_group` — red on base, seen:
  `left: ("BRANCHES", "reading…") right: ("TAGS", "v2.1.0")`. Covers TAGS landing, then BRANCHES (`v2-fix`) landing
  above it with the cursor kept on the tag. The "every group a placeholder → stays" case is its own test,
  `update::palette::tests::cursor_on_placeholder_stays_while_every_group_is_a_placeholder` (green on base too: a
  guard, not a red). `cursor_on_placeholder_moves_to_first_landed_row` green, unedited.
- Captures unchanged: `git diff --stat f349f135 -- docs/shots` empty.

**Gates at `3df7417d`:** fmt ok; clippy `--all-targets -D warnings` clean; `cargo nextest run --workspace
--test-threads 4` 5882/5882 passed, 10 skipped; `cargo test --doc --workspace` ok.

**Final signatures:**
```rust
pub(crate) enum PaletteTarget { Action { id: ActionId, ctx: ContextId }, App(AppAction), Note, Seek(Oid), File(u32), More(FindGroup) }
pub(crate) fn banner_press(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress,
    next_id: &mut impl FnMut() -> JobId) -> bool;   // banner/focus_keys.rs
```
`run`, `landed`, `first_runnable`, `rows`, `listing`: signatures unchanged. `run` builds `next_id` from
`state.local_jobs` as `keys::banner_key` does (`keys.rs:136-138`).

**RecentRuns can hold a Banner row** — `run` calls `state.palette_recent.ran(target)` for every run
(`palette.rs` `run`, first line), IN PROGRESS rows included. It stores the target with its ctx; `ran` dedupes and
`recent_rows` matches by command (`same_command`: an action by id, ctx ignored), so a recent row is drawn and run as
ACTIONS spells it **now** (`..entry.clone()` keeps the current entry's target). Matching by full target would have
dropped a recent row opened from another place — a behaviour change nobody asked for.

**Deviations — the conductor should know:**
1. **Merge lists no Skip row: was false at base, now made true.** The box asserts it; the code listed every Banner row
   for every kind (`palette.rs:677-687`). Added `Source::skippable` (live and not a merge) and a filter on
   `operation.skip`, matching the banner's hint bar (`graft-ui/src/graph_screen/hints/catalogue.rs:199-206`,
   `skippable: !merge`). Knock-on: `update/tests/palette_live/dead.rs` lost its `(Finish, operation.skip)` row — the
   sweep reported it **fixed** (the merge finish fixture no longer lists it). No capture lists a merge's IN PROGRESS.
2. **`update/banner.rs` edited** (spec: "Not: banner.rs"): one line, `pub(super) use focus_keys::banner_press;` —
   `focus_keys` is a private module, so `palette::run` cannot reach `banner_press` otherwise.
3. **Test files edited for the variant change** (spec: `palette_live` "green, unedited"; `empty_state.rs` "import line
   only"): `Action(id)` patterns cannot compile against `Action { id, ctx }`. Edits are mechanical:
   `palette_live.rs:38` `Action { id, .. }`; `empty_state.rs:177` `Action { id, .. }`; `empty_state/palette_tests.rs`
   expects `ctx: EmptyState`; `keymap_edge/palette.rs` expects `ctx: Graph` (two) and finds by id (one);
   `keymap_edge/palette_find.rs` a helper `on_graph(id)` (8 sites; clippy `too_many_lines` after rustfmt expanded the
   literals). Assertions otherwise unchanged; `palette_live` is still green.

**Touches:** `run` callers (`palette_key` ⏎, `double_clicked`, recent rows via the listing) unchanged in shape;
`grep -rnE 'palette::(run|first_runnable)' crates/graft/src` → none outside `palette.rs` before and after (empty
state's `outside_paste` does not call them at base). `banner_key` callers (`keys.rs:137`) unchanged. Journal/undo,
graft-ui, graft-ops, `update/tests.rs`, `update/mod.rs`, `banner/{pressed,stop}.rs`, `tests/it/palette_find.rs`,
`docs/shots`: `git diff --stat f349f135 -- <each>` empty (run). Keymap / hint bar: untouched (graft-ui diff empty).

**What remains (for the split) — D3 then D4, exactly as Decisions say; nothing of either started:**
- D3: split `update/palette.rs` (now 1509 lines incl. the D1/D2 changes and `#[cfg(test)] mod tests;` at its foot,
  whose file is already at `update/palette/tests.rs`) into `palette/{mod,listing,find,run,input}.rs`; pure move, own
  commit. Gotcha: `same_command` (used by `RecentRuns::ran` and `recent_rows`) and `Source::skippable` are new since
  the spec; `RecentRuns` → mod.rs, `recent_rows` → listing.rs, so `same_command` must be `pub(super)` in whichever
  holds it.
- D4: matcher to graft-ops; sites: `graft-core/src/lib.rs:18`, `graft-ops/src/switch_list.rs:15`,
  `graft/src/jobs/palette.rs:9`, `graft/src/update/palette.rs:23`, and the test
  `graft/src/update/tests/keymap_edge/palette_find.rs` (`graft_core::matcher::fuzzy_score("featu", …)`, :626) —
  a test file, not only import lines. ARCHITECTURE.md and design-note :331 edits; MAP rows (graft-core :80-81,
  graft-ops :394, graft :706).
- MAP box: graft's `banner_press` row and the `update::palette` route are written (this commit); the matcher and
  five-file rows wait for D3/D4.

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
