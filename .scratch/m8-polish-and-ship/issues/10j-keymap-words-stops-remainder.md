# G-keymap 2j — stops remainder: the mergetool fallback's `e`/`a` by place, the seven inline words tested remapped, 10f's MAP box held

Status: draft (planner spec-only pass 2026-10-07, from 10f's NOTES: W2 stop ×2, inline words unverified, MAP box untested)
Build with: **Sonnet** — all three rulings below are decided and each is held by a test or a stated command. Mechanical
spelling plus fixtures. Not Fable: wrong words fail a test.
Blocked by: **none by file** for source — 09f and 10g hold none of the Files below (see Touches for the checks).
Shared: `crates/graft/MAP.md` with both (conductor merges hunks). **Scheduling:** two cargo helpers at a time — starts
when 09f or 10g merges. **Stop and wait for 10g** if step 1's grep finds a test asserting the fallback line in a
resolver test file (10g's).
Needs: from 10f — `banner::abort_key(&Keymap)`, `update::tests::keymap_words::stops` (its table, helpers `swept`,
`unbinding`, `label`, `whole_tokens`); `update::edge::context_of(&AppState) -> ContextId`; graft-ui
`Keymap::{spell_keys, key_verb, translate}`, `ContextId`, `ActionId`.
Crate set: graft + graft-ui `tests/no_literal_keys/{known_stops,prose}.rs` only.
**Files:** graft `update/banner/resume.rs`, `update/mod.rs` (`mergetool_returned` only),
`update/tests/keymap_words/stops.rs`, `MAP.md` (own entries); only if a remapped test fails, the failing site's file:
`update/operate/{outcome.rs,failure.rs,diff_cursor.rs}`, `update/finish/keys.rs`, `update/banner/pressed.rs`; a
private fixture builder made `pub(in crate::update)` in its own file (named in NOTES). graft-ui
`tests/no_literal_keys/known_stops.rs`, `tests/no_literal_keys/prose.rs` (one row out, ruling 1b).
**Not:** `banner/focus_keys.rs` (09f), `resolver/*` (10g), `detail/keys.rs` (09f), `banner/tests*` (09f),
`tests/no_literal_keys.rs` root (09f).

## Read first
- This file's **Rulings** (whole); `10f-keymap-words-stops.md` NOTES
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `docs/design/02-keymap.md` §2.4 rows `e`, `M`, `c`/`a` (:308–312), §2.6 (:327–349, no `a` row), §2.6a (:371–384)
- `crates/graft/MAP.md` `update::banner` (:443–471, `open_mergetool` :470), `update::edge::context_of` (:210),
  `keymap_words` (:341); `crates/graft-ui/MAP.md` `## keymap`
- Source after the maps: `banner/resume.rs` (`open_mergetool`, :75), `update/mod.rs:511`, `keymap_words/stops.rs`.

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

## Scope
In: rulings 1, 1b, 2, 3; delete the 2 `KNOWN: 10f W2 stop` rows from `known_stops.rs`; `MAP.md:470` and the new fn.
Out: rewording beyond the drops above (20); graft-ui `ALLOWED`; any grep pattern change (09f); resolver words (10g).

## Signatures
```rust
// graft::update::banner::resume — m8/10j. Pure in (ctx, keys): table-testable.
/// The fallback clause after `no merge.tool configured · `: `e edits {path}  a abort`, each key from the row
/// answering it at `ctx` (ruling 1); a clause with no row or an unbound key dropped; nothing left → None.
pub(crate) fn mergetool_fallback(ctx: ContextId, path: &Path, keys: &Keymap) -> Option<String>;
// callers: open_mergetool (resume.rs), mergetool_returned (mod.rs) — both pass `edge::context_of(state)`, `&state.keymap`
```
No other signature changes. `open_mergetool`, `mergetool_returned` unchanged.

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

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k: fallback fn + 2 callers ~6k; pins/spelling/unbound/driven ~12k; seven inline fixtures ~18k; 1b + question ~4k;
MAP check + NOTES ~5k. Past ~65k: stop after the seven, hand 1b/`RESOLVE_KEYS` to a split (NOTES says where).
