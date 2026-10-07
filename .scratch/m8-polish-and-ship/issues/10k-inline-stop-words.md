# G-keymap 2k — the seven inline stop words, tested remapped by driving the real path; `RESOLVE_KEYS` spelled or stopped

Status: done (Part A built; Part B stopped per spec)
Build with: **Sonnet** — every rule decided below (10j ruling 2 stands; how to reach the fixtures decided here); each
case is a full oracle (swept keymap → remapped label, shipped → byte-identical). Part B has a pin with a stop, so no
judgement is left to the builder. Not Fable.
Blocked by: **none by file.** 09f is merged, so its Not-list (`banner/tests.rs`, `banner/tests*`) no longer binds; 10j
is merged. **Two cargo helpers at a time** (memory): runs as the second helper beside 17c, or after it.
Not concurrently with **11c** only if a fixture turns out to live in `update/tests.rs` (11c's) — then stop and wait.
Needs: from 10f — `update::tests::keymap_words::stops` (`swept`, `unbinding`, `label`, `whole_tokens`); from 10j —
`banner::resume::row_at` (private; the place → row rule), `edge::context_of(&AppState) -> ContextId`; graft-ui
`Keymap::{shipped, acting, translate, spell_keys}`, `ContextId`, `ActionId`.
Crate set: graft only.
**Files:** `update/tests/keymap_words/stops.rs`; the `mod` line of each test module holding a fixture builder this
ticket reaches (`update/banner.rs` or `update/banner/mod`, `update/finish.rs`, the operate/detail parent as found);
the fixture files themselves, **visibility only** (`banner/tests.rs`, `banner/rewrite_stop_tests.rs`,
`finish/tests.rs`, and the files NOTES names for the other four sites). Part B: `update/banner.rs` (`RESOLVE_KEYS`
:347), `update/banner/pressed.rs` (`continue_refused` :156 and its call :38), `update/status_read.rs` (its call :77),
`banner/tests.rs` :207, :945 (expected words only). Only if a case fails: the failing site's file
(`update/operate/{outcome,failure,diff_cursor}.rs`, `update/finish/keys.rs`, `update/banner/*`). `MAP.md` (Part B
entry only).
**Not:** `update/mod.rs` (17c), `view/*` (17c), `event_loop/*` (17c), `update/tests.rs` (11c),
`update/tests/keymap_words/{resolver,preview_remote,undo_journal}.rs` (other tables; counts unchanged), graft-ui.

## Read first
- `10j-keymap-words-stops-remainder.md` **Rulings** 1 and 2 (:55–88) and **NOTES** (:252–298: fixture lines, 1b,
  `RESOLVE_KEYS`)
- `10f-keymap-words-stops.md` NOTES (the seven sites and the place each was pinned at)
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `docs/design/02-keymap.md` §2.6a (:371–384, CONFLICTS rows: `M`, `e`, `space`)
- `crates/graft/MAP.md` `keymap_words` (:341), `update::banner` (:443–471), `update::edge::context_of` (:210),
  `## update::status_read` (:538–543)
- Source after the maps: `keymap_words/stops.rs` (helpers), then each fixture builder at the lines 10j NOTES cites.

## Decision (planner): how the seven are reached
**Widen the fixture builders, test-only, and keep one table in `stops.rs`.** Each fixture builder a case needs becomes
`pub(in crate::update)`, and its module's `mod` line `#[cfg(test)] pub(in crate::update) mod …;`. No production item
changes visibility.
- Why: 10j ruling 2 already allowed "no visibility widened except fixture builders"; the stop was only that the files
  were 09f's. One table keeps the count stated and reuses `swept`/`whole_tokens`, so all seven are judged by one rule.
- Rejected: a test beside each fixture in its own file (seven tests in five files, each re-importing the helpers; the
  count is no longer one place's); new fixtures in `stops.rs` (duplicates a rebase-at-edit stop, a shell merge and a
  finish box — three setups that already exist and are kept right by their own tests); moving the builders to a shared
  `update/tests/fixtures` module (churns 09f's files for no gain).
- If a site already has a driven remapped test (10h/10i touched `operate/outcome.rs`, `failure.rs`): cite it in the
  table's doc comment and keep the row anyway (one place, count 7) — grep first: `rg -n '<literal>' crates/graft/src/update/tests/keymap_words`.

**The seven** (10j ruling 2; ctx as 10f pinned): outcome `c continues` (Banner; fixture `banner/rewrite_stop_tests.rs`
:363), outcome `u undoes` (Graph), MarkersRemain `e edit  M mergetool` (Changes), finish `⇥ to write one` (Finish;
`finish/tests.rs` :249), abort box `u puts the merge back` (shell merge; `banner/tests.rs` :437), diff_cursor
`press v …` (Diff), discard box `` `u` can undo`` (Diff). Each: fixture → the event that draws it, with
`state.keymap = swept()`, words read where he sees them (status, notice, box line). A failing case is a defect: fix the
site (cause), not the test.

## Part B — `RESOLVE_KEYS` (10j 1b question, answered in 10j NOTES)
`banner.rs:347` `M mergetool  e edit  space marks resolved` is drawn (`pressed.rs:156` `continue_refused`, callers
`pressed.rs:38`, `status_read.rs:77`) and not spelled: the scanner reads no `const`. Rule: 10j ruling 1's place rule —
each clause from the row answering it at `context_of(state)` when the words are set.
- **Pin first** (red-first not needed: it is a fact check): at the place each caller leaves him, shipped
  `acting(place, M)` / `(e)` / `(space)` → a mergetool / edit / mark-resolved row?
- **All three act there** → spell each from its row (`row_at`), unbound → clause dropped, nothing left → `None`;
  shipped bytes identical, so `banner/tests.rs:207,945` stay unchanged.
- **Any one does not act there** → **stop on Part B** (do not drop the clause): dropping `space marks resolved` takes
  away his next step, which is a landing question (should the refusal focus his CONFLICTS rows?), not a words fix.
  NOTES: the pin output, file:line of the place, the question for the conductor. Const stays; Part A ships.
- Out: making the no-literal scanner read `const` items (09f's root pattern) — NOTES files it as a question with
  `banner.rs:347` as the line: other consts may hide the same way.

## Signatures
Part A: none (test-only visibility).
Part B, only on the "all three act" branch:
```rust
// graft::update::banner — replaces `const RESOLVE_KEYS: &str` (banner.rs:347). Pure in (ctx, keys).
/// `M mergetool  e edit  space marks resolved` as said at `ctx`: each key from the row answering it there
/// (10j ruling 1's `row_at`); an unbound key or a clause with no row dropped; nothing left → None.
pub(crate) fn resolve_keys(ctx: ContextId, keys: &Keymap) -> Option<String>;
// caller: banner::pressed::continue_refused — passes `edge::context_of(state)`, `&state.keymap`
```
`continue_refused`: if it already takes `&AppState`, its signature is unchanged; else it gains `keys: &Keymap` and
`ctx: ContextId` last (NOTES says which). Its two callers pass `&state.keymap` / `context_of(state)`.

## Acceptance (each tick names its test, at tick time)
- [x] Inline seven: `update::tests::keymap_words::stops::each_inline_stop_word_spells_the_remapped_key` — 7 cases
      (count asserted in the test), driven through the event that draws each; swept → remapped label as a whole token,
      no default label of a moved action; shipped → byte-identical to the literal pinned in the test. Red seen once by
      mutation (one site's `keys` → `Keymap::shipped()`), reverted; NOTES names the site.
- [x] Visibility only: `git diff <base> -- crates/graft/src | rg '^\+.*pub\(in crate::update\)'` hits only
      `#[cfg(test)]` modules and fixture fns (NOTES lists each); `git diff <base> --stat` shows no production file
      outside Files. Check: that command.
- [x] Part B pin: `…::stops::the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown` — both callers driven
      (banner `c` with unmerged files; a status read settling to the refusal); asserts the place and the three shipped
      rows. Then either:
      - [ ] n/a (stop branch) spelled: `…::stops::the_resolve_keys_spell_his_keys` — swept → each place's own labels; each row unbound
            → its clause gone, all unbound → no keys, no `·`, no `—`; shipped → byte-identical; `banner/tests.rs`
            :207, :945 green unchanged; or
      - [x] stopped: NOTES holds the pin output and the question — `…::stops::the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown`.
- [x] Ratchet: `no_literal_keys::no_user_facing_literal_spells_a_default_key` green.
- [x] Captures: `git diff --stat <base> -- docs/shots` empty (shipped bytes held). Check: that command.
- [ ] n/a (Part B stopped) MAP.md graft (Part B spelled only): `resolve_keys` beside `mergetool_fallback` (:471). Check: `rg -n
      'resolve_keys' crates/graft/MAP.md`.

## Touches
- Fixture files (09f's, 10f's): visibility lines only — `git diff <base>` of each shows only `pub(in crate::update)`
  edits (shown). Their own tests unchanged, green.
- `continue_refused` callers (`pressed.rs:38`, `status_read.rs:77`): `rg -n 'continue_refused|RESOLVE_KEYS'
  crates/graft/src` before and after (output in NOTES). Hint bar `banner_view.conflict` / `hint_context`: not
  touched (grep).
- Other keymap_words tables: counts unchanged (`resolver.rs` 21, others as merged) — their files untouched.
- Keymap: no binding added/moved; `keymap_ids.txt` unchanged (empty diff shown).
- Journal/undo: words only — `git diff <base> -- crates/graft/src/update/journal* crates/graft/src/update/undo*
  crates/graft-ops` empty.
- 17c: no shared file (17c: `view/*`, `event_loop/graphics.rs`, `update/mod.rs`); `git diff --stat` shows none.
- 11c: `update/tests.rs` not edited (shown).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~40k: locate + widen seven fixtures ~10k; table of seven, driven ~16k; Part B pin + branch ~8k; NOTES/MAP ~4k.
Order: A, then B. Past ~65k: stop after A; B goes to a split (NOTES says so).

## NOTES (builder 10k, 2026-10-07)

No compaction. Scout: 10j NOTES (named by the conductor); fixtures read directly.

- **Inline seven** (`stops::each_inline_stop_word_spells_the_remapped_key`, count asserted 7): amend at edit stop
  (`rewrite_stop_tests::{edit_stop,a_amends,amended}`, keymap swapped before the amend answers — the words are set then);
  one-commit cherry-pick `u undoes` (`operate::launch` + `update(Produced(Operated::Done))`); MarkersRemain (Stage
  launched, answered `Failed{refused: MarkersRemain}`); finish empty subject (`finish::tests::opened`, `y` via
  `dispatch_key`); abort box (`banner::tests::merge_read`, `a` via `dispatch_key`); side-by-side `press v`
  (`stage_at_diff_cursor`); discard box backup line (`request_discard_at_diff_cursor`). Presses go through
  `dispatch_key` with the default chord (what the edge hands on after translating his chord). All 7 green, no site fix
  needed. **Red by mutation:** `operate/outcome.rs` cherry-pick `state.keymap` → `Keymap::shipped()` → assertion fail
  (`left: "u undoes"`, `right: "alt+& undoes"`); reverted. Red per test before code: n/a — test-only, sites existed.
- **Visibility** (`git diff ee79f0e6 -- crates/graft/src | rg '^\+.*pub\(in crate::update\)'`): `banner.rs` mod lines
  `rewrite_stop_tests`, `tests`; `operate.rs` mod `tests`; fns `edit_stop`, `a_amends`, `amended`, `banner::tests::
  {stopped, done_applied, merge_read}`, `operate::tests::{one_pair_diff, state_on_diff}`. All inside `#[cfg(test)]`
  modules. No production item changed.
- **Part B — STOPPED.** Pin output: both callers (banner `c`, `pressed.rs:36`; `status_read::settle`, `status_read.rs:77`)
  leave him at **Banner** (`context_of`). Shipped there: `M` → `operation.mergetool`, `e` → `operation.edit`,
  `space` → **None** (space acts on Changes, asserted as control). §2.6a (02-keymap.md :382–384): space marks resolved
  only on CONFLICTS rows. Const `banner.rs:348` stays.
  **Question for conductor (landing, not words):** `c`'s refusal names `space marks resolved`, which does nothing on the
  focused banner where he is left. Should the refusal hand focus to the CONFLICTS rows (where all three act), or drop
  the clause? Also filed: the no-literal scanner reads no `const` items (`banner.rs:348`) — other consts may hide.
- **Seen, not mine:** `operate/stage_discard.rs:132` literal "`u` can undo it afterwards" (file-row discard box) — not
  spelled from keymap; not in the seven.
- **Touches:** `git diff --stat ee79f0e6 -- docs/shots crates/graft-ui crates/graft-ops update/journal* update/undo*
  update/tests.rs update/mod.rs view event_loop` → empty. `continue_refused|RESOLVE_KEYS` grep unchanged (no edit).
  Other keymap_words tables untouched. Keymap untouched.
- Gates: fmt ok, clippy -D warnings ok, nextest 5876/5876, doctests ok.
