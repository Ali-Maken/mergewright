# G-keymap 2j — stops remainder: the mergetool fallback's `e`/`a` by place, the seven inline words tested remapped, 10f's MAP box held

Status: ready
Amended: planner spec-only pass 2026-10-07, from 10g's NOTES (merged partial, 3693ddd6): rulings 4–6 added — 10g's two
left `KNOWN` rows (`open_failed`, `finish_refused_notice`) and 10g's re-opened MAP box. Nothing above is changed.
Build with: **Sonnet** — all three rulings below are decided and each is held by a test or a stated command. Mechanical
spelling plus fixtures. Not Fable: wrong words fail a test.
Build with (amended): still **Sonnet** — rulings 4–6 are decided; the one unknown (the place after the resolver closes)
is settled by a pin test, not by the builder's judgement.
Blocked by: **none by file** for source — 09f and 10g hold none of the Files below (see Touches for the checks).
Blocked by (amended): still none. 10g is merged (3693ddd6), so the "wait for 10g" below is already met; the resolver
files added below are no longer held. `update/status_read.rs` is in no other ticket's Files (planner grep of
`.scratch/m8-polish-and-ship/issues/`, 2026-10-07); `banner/pressed.rs` was 10f's, now done.
Shared: `crates/graft/MAP.md` with both (conductor merges hunks). **Scheduling:** two cargo helpers at a time — starts
when 09f or 10g merges. **Stop and wait for 10g** if step 1's grep finds a test asserting the fallback line in a
resolver test file (10g's).
Needs: from 10f — `banner::abort_key(&Keymap)`, `update::tests::keymap_words::stops` (its table, helpers `swept`,
`unbinding`, `label`, `whole_tokens`); `update::edge::context_of(&AppState) -> ContextId`; graft-ui
`Keymap::{spell_keys, key_verb, translate}`, `ContextId`, `ActionId`.
Needs (amended): from 10g — `update::resolver::finish_refused_notice` (today `(&str) -> (String, bool)`), the row
pinned by `update::tests::keymap_words::resolver::a_notice_and_the_footer_spell_mark_done_alike` (`⏎` marks done).
Crate set: graft + graft-ui `tests/no_literal_keys/{known_stops,prose}.rs` only.
Crate set (amended): plus graft-ui `tests/no_literal_keys/known_resolver.rs`.
**Files:** graft `update/banner/resume.rs`, `update/mod.rs` (`mergetool_returned` only),
`update/tests/keymap_words/stops.rs`, `MAP.md` (own entries); only if a remapped test fails, the failing site's file:
`update/operate/{outcome.rs,failure.rs,diff_cursor.rs}`, `update/finish/keys.rs`, `update/banner/pressed.rs`; a
private fixture builder made `pub(in crate::update)` in its own file (named in NOTES). graft-ui
`tests/no_literal_keys/known_stops.rs`, `tests/no_literal_keys/prose.rs` (one row out, ruling 1b).
**Files (amended):** graft `update/resolver/answers.rs` (`open_failed` only), `update/resolver/resolved.rs`
(`finish_refused_notice` only), `update/banner/pressed.rs` (its `finish_refused_notice` call, `:40` — now in, not
only-if), `update/status_read.rs` (its call, `:67`), the file holding `update::resolver::tests::file_list` (only the
expected words of `an_unmerged_open_failure_still_closes`, ruling 4; path in NOTES), `update/banner.rs` or
`update/banner/mod` (a `pub(crate) use` only, if `resume` is not visible from `update::resolver`; NOTES says which).
graft-ui `tests/no_literal_keys/known_resolver.rs` (2 `KNOWN: 10g` rows out).
**Not:** `banner/focus_keys.rs` (09f), `resolver/*` (10g), `detail/keys.rs` (09f), `banner/tests*` (09f),
`tests/no_literal_keys.rs` root (09f).
**Not (amended):** the `resolver/*` above means every resolver file except the three named in Files (amended);
`update/tests/keymap_words/resolver.rs` (10g's table — its count stays 21; the new cases live in `stops.rs`).

## Read first
- This file's **Rulings** (whole); `10f-keymap-words-stops.md` NOTES
- `10g-keymap-words-resolver.md` **NOTES** ("Left — 2 KNOWN rows") — amended
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `docs/design/02-keymap.md` §2.4 rows `e`, `M`, `c`/`a` (:308–312), §2.6 (:327–349, no `a` row), §2.6a (:371–384)
- `crates/graft/MAP.md` `update::banner` (:443–471, `open_mergetool` :470), `update::edge::context_of` (:210),
  `keymap_words` (:341); `crates/graft-ui/MAP.md` `## keymap`
- Amended: `crates/graft/MAP.md` `finish_refused_notice` / `set_notice` (:499), `## update::status_read` (:538–543),
  `Identity` (:908), `whole_file_keys` (:495)
- Source after the maps: `banner/resume.rs` (`open_mergetool`, :75), `update/mod.rs:511`, `keymap_words/stops.rs`.
- Amended, source after the maps: `resolver/answers.rs` `open_failed`, `resolver/resolved.rs`
  `finish_refused_notice`, `banner/pressed.rs:40`, `status_read.rs:67`, `an_unmerged_open_failure_still_closes`.

## Rulings (planner; not the builder's)

**1. `e edits {path}  a abort` — no single row. Each clause names the row that answers it where the line is shown,
read from `context_of(state)` when the words are set.** The refusal leaves him where he pressed `M` (banner, resolver,
changes pane); his next key is translated in that place, so W2's "where he presses it" is the caller's place, not
one of three fixed rows. Pinning `operation.edit` would misname the key in the resolver once he remaps
`resolver.edit` (ux row 4, consistency).
- `e` clause: `ContextId::Banner` → `operation.edit`; `Resolver` → `resolver.edit`; `Changes` → `changes.edit`;
  `Diff` → `changes.edit` **only if** `Keymap::shipped().translate(Diff, e)` resolves to it (pin decides; NOTES
  says); any other place → clause dropped (path goes with it). Rejected: one row for all (wrong after a remap);
  passing an origin enum through the three callers (edits 09f's `focus_keys.rs`/`detail/keys.rs` and 10g-adjacent
  `resolver/handover.rs` for what `context_of` already knows).
- `a` clause: kept **only where the row answering `a` is `operation.abort`** — expected Banner only: §2.6 has no `a`
  (resolver is modal), §2.4 says `a` refuses in words while an operation is live. Spelled by `banner::abort_key`.
  Outside the banner the clause is dropped. This is a W6 byte change in two places (resolver, changes pane): allowed,
  listed in NOTES. If the pin shows `a` **is** `operation.abort` at Resolver or Changes, keep it there — the rule is
  the pin, not my expectation.
- Both sites (`open_mergetool`'s no-`merge.tool` refusal, `resume.rs:75`; `mergetool_returned`, `mod.rs:511`) call one
  fn. `open_mergetool`'s signature is **unchanged** (it already has `&mut AppState`), so its three callers are not
  edited. `mergetool_returned` computes `context_of` after its own state change (NOTES: which arm sets the line, with
  file:line). Nothing left → the line is the reason alone (`no merge.tool configured`), no `·`, no `—`.
- **1b (same clause family, 10f's "not mine, seen"):** `prose.rs`'s row `a copy may still be in S` names the stash
  key — not prose. Spell `S` from `graph.stash` (W2 Graph, as `S p`) in `edits_clause`'s `Back{None}` arm
  (`failure.rs`, only if not already spelled — check first); unbound → that clause dropped (W3). Row out of
  `prose.rs`. `banner.rs` `RESOLVE_KEYS` (`M mergetool  e edit  space marks resolved`) is a **question** (rule 7, no
  line): NOTES answers whether it is drawn (file:line) and why the grep misses it; drawn → spell it here (ctx
  Changes, §2.6a :382–384) if it is one fn, else report; a grep hole → report only (09f holds the root).

**2. The seven inline words — tested by driving the real path, not by extracting fns.** Each site: fixture → the
event that draws it, with `state.keymap = swept()`, read the words from where he sees them (status, notice, box line).
Why: a table row on an extracted fn proves the fn, not that the caller passes `state.keymap` rather than
`Keymap::shipped()` — the failure 10f could not rule out. No visibility widened except fixture builders.
Sites (10f NOTES, ctx as 10f pinned): outcome `c continues` (amend at an edit stop, Banner), outcome `u undoes`
(Graph), MarkersRemain `e edit  M mergetool` (Changes), finish `⇥ to write one` (Finish), abort box `u puts the
merge back` (shell merge, Graph/Banner as pinned), diff_cursor `press v …` (Diff), discard box `` `u` can undo``
(Diff). Seven cases; count stated. A failing case is a defect: fix the site (cause, not the test).

**3. 10f's MAP.md box — held by a command check, not dropped.** MAP drift is real (`MAP.md:470` still reads
`e edit …` against code's `e edits`). The check: the fns whose signature gained `keys: &Keymap` in 10f
(`git diff b1756a50 8e47dec7 -- crates/graft/src | rg '^\+.*keys: &Keymap'`, each mapped to its fn) and here,
each found in `crates/graft/MAP.md` with `keys` in its entry (`rg -n '<fn>' crates/graft/MAP.md`); NOTES lists
fn → MAP line. Conductor: re-open 10f's box as `[ ] → 10j`.

**4 (amended). `open_failed`'s `e edit  M mergetool` — ruling 1's place rule, read after the resolver closes.** Same
family as ruling 1: the line is shown after the resolver has closed (10g NOTES), so W2's place is wherever the close
leaves him, read by `context_of(state)` **after** the close, when the words are set.
- **Rule 7 first.** 10g inferred "neither Banner nor Changes" from a failing test and did not confirm the place. That
  is a stop, not a premise: step 1 is a pin test driving `an_unmerged_open_failure_still_closes`'s flow and asserting
  `context_of` after the close. NOTES names the place and the file:line that sets focus on close. **The ruling below
  does not depend on which place it is.**
- `e` clause: ruling 1's table, verb unchanged (` edit`, no path — no rewording). **One private place → edit-row fn
  serves both `mergetool_fallback` and this line** so the two can never name different keys in one place (ux row 4).
- `M` clause: kept only where shipped `translate(ctx, M)` resolves to a mergetool row — §2.6a :377 (Banner), §2.6
  :348 (Resolver), §2.4 :310 (Changes); Diff only if the pin says so; ids in NOTES. Elsewhere dropped. Graph: `M` is
  `graph.map` (graft-ui `keymap/table.rs:155`, cited by 10g) → dropped there.
- Nothing left → the reason alone, no `·`, no `—`. Existing order (`e` then `M`) kept.
- The existing test `update::resolver::tests::file_list::an_unmerged_open_failure_still_closes`: if the pinned place
  drops a clause, update **only its expected words** (W6 byte change: the old words named keys that do not edit or
  hand over where he lands); its close assertion stays. NOTES quotes old → new. Not disabled, not loosened. If the
  place keeps both clauses, the test is unchanged.
- **Question, not built:** if he lands where neither key reaches the file, the line names no next step (ux
  visibility). Whether an open failure should land him on that file's `CONFLICTS` row is a landing (journey) change —
  NOTES files it for the conductor with the pin's result. Rejected here: re-landing him (behaviour, not words; out of
  scope); keeping the literal (names keys that do nothing there once remapped, and `M` maps the graph today).

**5 (amended). `finish_refused_notice` — `⏎` from the resolver's row, fixed place, one spelling with the footer.**
- Signature as 10g proposed: `(reason: &str, keys: &Keymap) -> (String, bool)`; the `bool` is untouched. Both callers
  (`banner/pressed.rs:40`, `status_read.rs:67`) pass `&state.keymap`.
- Place: **Resolver**, not `context_of`. The words are the open resolver's notice row (`set_notice`, MAP :499), and
  while the resolver is open `context_of` hands keys to it before the focused banner (MAP :210 order). `⏎` is spelled
  from the row `a_notice_and_the_footer_spell_mark_done_alike` pins, so notice and footer name it alike.
- If the driven test shows either caller drawing this string **outside** an open resolver (status line with the
  resolver closed), stop and report with file:line — do not pick a row (rule 7).
- Unbound → `{reason}` alone, no `·`. Shipped → byte-identical `<reason> · ⏎ marks done`.

**6 (amended). 10g's MAP box — ruling 3's check, extended.** The fns whose signature gained `keys: &Keymap` in 10g
(`git diff 3693ddd6^1 3693ddd6 -- crates/graft/src | rg '^\+.*keys: &Keymap'`, each mapped to its fn), each found in
`crates/graft/MAP.md` with `keys` in its entry; `Identity::Done.restart` reads `Option<&str>` in its entry (:908 area);
`finish_refused_notice` at :499 reads the new signature; `edit_or_mergetool` has an entry beside `mergetool_fallback`.
NOTES lists fn → MAP line. Conductor: 10g's re-opened box (`[ ] → 10j`) and 10g's ratchet box tick with this ticket's.

## Scope
In: rulings 1, 1b, 2, 3; delete the 2 `KNOWN: 10f W2 stop` rows from `known_stops.rs`; `MAP.md:470` and the new fn.
In (amended): rulings 4, 5, 6; delete the 2 `KNOWN: 10g` rows from `known_resolver.rs`; MAP entries named in ruling 6.
Out: rewording beyond the drops above (20); graft-ui `ALLOWED`; any grep pattern change (09f); resolver words (10g).
Out (amended): "resolver words (10g)" means every resolver word except the two in rulings 4–5; re-landing an open
failure (ruling 4's question); 10g's `keymap_words/resolver.rs` table.

## Signatures
```rust
// graft::update::banner::resume — m8/10j. Pure in (ctx, keys): table-testable.
/// The fallback clause after `no merge.tool configured · `: `e edits {path}  a abort`, each key from the row
/// answering it at `ctx` (ruling 1); a clause with no row or an unbound key dropped; nothing left → None.
pub(crate) fn mergetool_fallback(ctx: ContextId, path: &Path, keys: &Keymap) -> Option<String>;
// callers: open_mergetool (resume.rs), mergetool_returned (mod.rs) — both pass `edge::context_of(state)`, `&state.keymap`
```
No other signature changes. `open_mergetool`, `mergetool_returned` unchanged.

Amended — plus these two (rulings 4, 5); still no change to `open_mergetool`, `mergetool_returned`, `open_failed`'s
own signature:
```rust
// graft::update::banner::resume — m8/10j ruling 4. Pure in (ctx, keys). Shares the private place → edit-row fn
// with `mergetool_fallback`.
/// `e edit  M mergetool` as said at `ctx`: `e` from the edit row answering it there (ruling 1's table), `M` only where
/// it is a mergetool row; a clause with no row or an unbound key dropped; nothing left → None.
pub(crate) fn edit_or_mergetool(ctx: ContextId, keys: &Keymap) -> Option<String>;
// caller: update::resolver::answers::open_failed — passes `edge::context_of(state)` read after the close, `&state.keymap`

// graft::update::resolver::resolved — m8/10j ruling 5 (was `(&str) -> (String, bool)`).
pub(crate) fn finish_refused_notice(reason: &str, keys: &Keymap) -> (String, bool);
// callers: update::banner::pressed (:40), update::status_read (:67) — both pass `&state.keymap`
```

## Acceptance (each tick names its test, at tick time)
- [ ] Pins: `update::tests::keymap_words::stops::the_mergetool_fallback_names_the_row_where_it_is_shown` — shipped
      `translate(ctx, e)` == `operation.edit`/`resolver.edit`/`changes.edit` at Banner/Resolver/Changes (Diff per
      ruling); `translate(Banner, a)` == `operation.abort`; at Resolver and Changes ≠ `operation.abort`. Red first.
- [ ] Spelling: `…::stops::the_mergetool_fallback_spells_each_place_s_own_key` — the three edit rows remapped to three
      different keys: each place's line carries its own key and no other's; shipped → Banner line byte-identical to
      today's; Resolver/Changes lines = today's minus `  a abort` (NOTES quotes old → new).
- [ ] Unbound: `…::stops::an_unbound_edit_or_abort_drops_its_clause` — `resolver.edit = "none"` at Resolver → line is
      `no merge.tool configured` alone; `operation.abort = "none"` at Banner → `e edits {path}` alone; no `—`, no
      trailing `·`.
- [ ] Both sites, driven: `…::stops::no_merge_tool_words_follow_the_place_he_pressed_m` — `M` with no `merge.tool`
      from the focused banner, the resolver, a CONFLICTS row: each status/notice spelled for that place under the
      sweep keymap; and `mergetool_returned`'s arm (NOTES names it) likewise.
- [ ] Inline seven: `…::stops::each_inline_stop_word_spells_the_remapped_key` — 7 cases (count stated), swept →
      remapped label as a whole token, no default label of a moved action; shipped → byte-identical to the literal
      pinned in the test.
- [ ] 1b: `…::stops::each_stop_word_spells_the_remapped_key` gains the `Back{None}` row (`S` from `graph.stash`);
      `RESOLVE_KEYS` answered in NOTES with its line (and a row here if spelled).
- [ ] Ratchet: `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `rg -c 'KNOWN' crates/graft-ui/tests/no_literal_keys/known_stops.rs` → 0 (output in NOTES).
- [ ] Captures: `git diff --stat <base> -- docs/shots` empty (planner checked 2026-10-07: `rg 'merge\.tool' docs/shots`
      has no hit, so no capture draws the changed line). Check: that command.
- [ ] MAP.md (ruling 3): fn → MAP line list in NOTES for 10f's and this ticket's `keys: &Keymap` fns; `:470` reads
      the new words. Check: the two commands in ruling 3.

Amended (rulings 4–6):
- [ ] Open-failure place pin: `…::stops::an_open_failure_lands_him_where_its_words_are_read` — drives
      `an_unmerged_open_failure_still_closes`'s flow; asserts `context_of` after the close == the place NOTES names,
      and shipped `translate(that place, e)` / `translate(that place, M)` == the rows NOTES names (or not an
      edit/mergetool row). Red first.
- [ ] Open-failure words: `…::stops::the_open_failure_names_only_keys_that_work_where_he_lands` — table on
      `edit_or_mergetool` over Banner/Resolver/Changes/Graph (Diff per pin): shipped, swept (each place's own key, no
      other's), `e`-row unbound, `M`-row unbound, both unbound → None; Graph → None (`M` is `graph.map`). Plus driven:
      the line after the close under the sweep keymap is that place's spelling or the reason alone, no `·`.
- [ ] One edit row: `…::stops::the_fallback_and_the_open_failure_name_the_same_edit_key` — at each place, under the
      sweep keymap, the `e` key in `mergetool_fallback` and in `edit_or_mergetool` is the same label.
- [ ] Existing test held: `update::resolver::tests::file_list::an_unmerged_open_failure_still_closes` green; its close
      assertion unchanged (`git diff <base>` of that test shows only the expected-words line, or nothing); NOTES quotes
      old → new.
- [ ] Finish refusal: `…::stops::the_finish_refusal_spells_mark_done_from_the_resolver_row` — shipped
      `translate(Resolver, ⏎)` == the row `resolver::a_notice_and_the_footer_spell_mark_done_alike` pins; swept →
      remapped label as a whole token, no `⏎`; that row unbound → `{reason}` alone, no `·`; shipped → byte-identical
      `<reason> · ⏎ marks done`; `bool` equal to today's for each case.
- [ ] Both refusal callers, driven: `…::stops::both_finish_refusals_spell_his_key` — banner `c` with the resolver open
      and a `continue_refusal` (`pressed.rs` path); a status read settling to a refusal while the notice says
      `CHECKING` (`status_read.rs` path); under the sweep keymap the resolver's notice row carries the remapped key.
- [ ] Ratchet (10g's): `rg -c 'KNOWN' crates/graft-ui/tests/no_literal_keys/known_resolver.rs` → 0 (output in NOTES);
      `no_literal_keys::no_user_facing_literal_spells_a_default_key` green.
- [ ] Captures (rulings 4–5): `rg -n 'e edit  M mergetool' docs/shots` → no hit (planner checked 2026-10-07); `· ⏎
      marks done` hits only `three.txt · ⏎ marks done` (10g's settled-file notice, not this fn). Shipped bytes of
      `finish_refused_notice` unchanged. Check: `git diff --stat <base> -- docs/shots` empty.
- [ ] MAP.md (ruling 6): NOTES lists fn → MAP line for 10g's `keys: &Keymap` fns, `Identity::Done.restart`,
      `finish_refused_notice`, `edit_or_mergetool`. Check: the commands in ruling 6.

## Touches
- `open_mergetool` callers `banner/focus_keys.rs:115` (09f), `resolver/handover.rs:43`, `detail/keys.rs:234` (09f):
  signature unchanged → `git diff <base> -- <those three>` empty (show it).
- Tests asserting the fallback text: `rg 'merge.tool configured|e edits' crates/graft/src` before editing; a hit in
  `update/resolver/**` or `banner/tests*` → stop, wait for 10g/09f merge.
- `mergetool_returned` callers (event loop message): signature unchanged.
- Hint bar `M`/`e` (`banner_view.conflict`, `hint_context`): not touched — grep shows no use of the new fn there.
- Keymap: no binding added/moved; `keymap_ids.txt` unchanged (empty diff shown).
- Journal/undo: words only — `git diff <base> -- crates/graft/src/update/journal* crates/graft/src/update/undo* crates/graft-ops` empty.
- `prose.rs`: one row removed; 10h/10i may add rows there — conductor merges.
- Captures: none (above). Contact sheet unchanged.
- Amended — `finish_refused_notice` callers: `rg -n 'finish_refused_notice' crates/graft/src` → the def, `pressed.rs:40`,
  `status_read.rs:67`, tests; nothing else (output in NOTES). Compiler holds the rest.
- Amended — `open_failed`: its own signature unchanged; `rg -n 'e edit  M mergetool' crates/graft/src` before and
  after (after: no literal left); tests asserting it named in NOTES (expected: the one in `file_list`).
- Amended — `finish_if_owed` / `set_notice` / `status_read::settle` (same notice row): words only, no state change;
  `cancel_owed_finish` keys unchanged. Full nextest holds.
- Amended — 10g's resolver footer (10c) and `a_notice_and_the_footer_spell_mark_done_alike`: not edited; the new
  refusal test reads the same row.
- Amended — `known_resolver.rs`: 2 rows out; no other `known_*` file changes from rulings 4–5.
- Amended — 10h/10i: `status_read.rs` and `pressed.rs` are in neither's Files (planner grep 2026-10-07); MAP.md hunks
  merged by the conductor.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k: fallback fn + 2 callers ~6k; pins/spelling/unbound/driven ~12k; seven inline fixtures ~18k; 1b + question ~4k;
MAP check + NOTES ~5k. Past ~65k: stop after the seven, hand 1b/`RESOLVE_KEYS` to a split (NOTES says where).
Amended: ~60k with rulings 4–6 (open failure pin + fn + driven ~7k, finish refusal + two callers driven ~6k, MAP ~2k).
Order: 1, 4, 5, 2, 3+6, then 1b. The split rule above still holds: past ~65k, 1b/`RESOLVE_KEYS` go to the split
first; if 4–5 are not done by then, they go with it (NOTES says which).
