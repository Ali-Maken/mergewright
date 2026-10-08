# G-keymap 0l — `c`'s refusal at a conflict names keys that act where he is: `⏎ resolve` replaces `space marks resolved`

Status: done (builder 2026-10-08) — was ready (conductor 2026-10-08) — was draft (planner spec-only pass 2026-10-08; rules KNOWN_GAPS `M8/resolve-refusal-space-dead`, 10k Part B's stop)
Build with: **Sonnet** — ruled below; one const becomes a pure fn on 10j's existing pattern (`mergetool_fallback`,
`edit_or_mergetool`), held by the 10k pin and a remap test. Not Fable: no signature others build on.
Scout: **none** — 10k NOTES Part B is the scout (both callers, the place, the three rows, with lines). Check first
re-reads them at this base.
Blocked by:
- **12f merged** — the 12e D1 equivalence test (`update/palette/tests.rs`) compares the refusal's status; this edits
  its oracle, and 12f moves that directory.
- **May run beside 11e** (disjoint: 11e holds graft-ui, graft `view/*`, `update/tests.rs`, capture tests).
- Changes **no capture** (no `docs/shots` file holds the words — `rg -l 'space marks resolved' docs/shots` empty at
  planning); not in the serial capture chain. Rebase after any capture-run merge and run full nextest again.
- **Two cargo helpers at a time** (memory).
Needs: 10j's `row_at` / `acting` lookups (as `mergetool_fallback` uses), `edge::context_of`, `Keymap`, `ContextId`;
10k's pin test and widened fixtures; 12e's `banner_press` (the palette path).
**Crate set:** graft only — `update/banner.rs` (the const → fn), `update/banner/pressed.rs` (`continue_refused`),
`update/status_read.rs` (caller, only if its call changes), `update/banner/tests.rs` (:207, :945 expected words),
`update/tests/keymap_words/stops.rs`, `update/palette/tests.rs` (the equivalence oracle, one case),
the M5 walk test that quotes the words if any (`rg`), `MAP.md`; plus `TESTING.md` :2047–2049 (scenario 27's words).
**Not:** graft-ui, hint bar, keymap table, `update/banner/focus_keys.rs`, `docs/shots`.

## Read first
- 10k: Part B (:55–67), Signatures (:69–80), NOTES Part B (:139–145)
- `docs/design/02-keymap.md` §2.6a :361–387 (the banner table, "With focus given back")
- `crates/graft/MAP.md` :462 (`continue_refusal`/`continue_refused`), :476 (`mergetool_fallback` — the pattern to copy)
- 12e NOTES "Fixture note" (:184–188) — how the equivalence test already treats a focus-spelled refusal

## Ruling (planner)
**Name what acts where he stands; don't move him.** Today `1 file still conflicted · M mergetool  e edit  space marks
resolved`: both callers leave him on the focused banner (10k pin), where `space` does nothing (§2.6a: `space` marks
resolved only from the changes pane, after `⇥`). The banner's own first key does what he needs: `⏎` opens the resolver
on the first conflicted file, where `⏎ done` marks it resolved — and the banner's bar already leads with `⏎ resolve`.
New words, same order as that bar: **`1 file still conflicted · ⏎ resolve  M mergetool  e edit`**.
- Rejected — move focus to the CONFLICTS rows: a refusal that moves him is a landing nobody asked for; it breaks P5
  (the first key after a stop goes to what stopped — `c`, `a` would be dead after it) and a second `c` would need `⇥`.
- Rejected — drop the clause: takes away his next step (ux principles §3.1, visibility of the next step).
- Spelled by 10j ruling 1's place rule, as `mergetool_fallback` is: each clause from the row answering it at
  `context_of(state)` when the words are set; unbound or no row there → clause dropped; nothing left → no keys, no `·`.
  From the palette (12e D1: `c` runs via `banner_press` with the graph focused) that gives the reason alone — the same
  as `M`'s refusal there (12e NOTES :184–188). Accepted.
- Within §2.6a ("refused, in words"); m6/31 S-4 kept `space` on the belief he stands where `space` acts — 10k's pin
  showed he does not. A words change on a line he sees, inside the approved design: no user question.

## Scope
In: the fn, its two production callers, expected words, TESTING.md scenario 27, MAP. Out: the no-literal scanner
reading `const` items (10k's question — separate); `⇥ banner`-style keys on the palette path; any focus change.

## Signatures
```rust
// graft::update::banner — replaces `const RESOLVE_KEYS: &str` (banner.rs:347–348). Pure in (ctx, keys).
/// `⏎ resolve  M mergetool  e edit` as said at `ctx`: each key from the row answering it there (10j ruling 1);
/// an unbound key or a clause with no row at `ctx` dropped; nothing left → None.
pub(crate) fn resolve_keys(ctx: ContextId, keys: &Keymap) -> Option<String>;
// caller: banner::pressed::continue_refused — passes `edge::context_of(state)`, `&state.keymap`
```
`continue_refused`: if it already takes `&AppState`, signature unchanged; else it gains `ctx: ContextId, keys: &Keymap`
last. Callers `pressed.rs:36–38` and `status_read.rs:77` pass them. NOTES quotes the final signature.

## Acceptance (each tick names its test, at tick time)
- [x] **Check first**: at base, `acting(Banner, ⏎)` is the row that opens the resolver (NOTES: its `ActionId`, line);
  `M`, `e` as 10k pinned. `⏎` not a resolver row at Banner → **stop**, report the line.
- [x] Pin updated, red first: `update::tests::keymap_words::stops::the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown`
  — both callers: every clause in the words acts at the place he is left (`⏎`, `M`, `e`); `space` no longer named.
  Red on base (it names `space`), green after.
- [x] Spelled: `update::tests::keymap_words::stops::the_resolve_keys_spell_his_keys` — swept keymap → his labels as
  whole tokens; each row unbound → its clause gone; all unbound → no keys, no `·`, no `—`; shipped → exactly
  `⏎ resolve  M mergetool  e edit`.
- [x] Words: `update::banner::tests` :207, :945 expect `1 file still conflicted · ⏎ resolve  M mergetool  e edit`
  (and the plural form); `…::the_refusal_counts_one_file_in_the_singular` green.
- [x] Palette path: `update::palette::tests::in_progress_rows_run_as_the_focused_banner_would` — for the Continue row
  at a conflict stop the oracle compares the refusal **reason** and commands, keys excepted (the 12e `M` treatment);
  a new assert that the palette run's refusal names no key that does not act at `context_of`. Only that case edited.
- [x] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `rg -n 'RESOLVE_KEYS|space marks resolved'
  crates` empty (output in NOTES).
- [x] M5 / M6 walks: any test quoting the old words (`rg` before) updated to the new and green; TESTING.md scenario 27
  (:2047–2049) reads the new words, and its next steps still hold (`Esc`, `space` in the changes pane, `⇥`, `c`).
- [x] Captures unchanged: `git diff --stat <base> -- docs/shots` empty.
- [x] MAP.md graft: `resolve_keys` beside `mergetool_fallback` (:476); :462 `continue_refused` line. Check: `rg -n
  'resolve_keys' crates/graft/MAP.md`.

## Touches
- `continue_refused` callers: `rg -n 'continue_refused|RESOLVE_KEYS|resolve_keys' crates/graft/src` before/after
  (NOTES).
- Hint bar `Banner{conflicted}` (`⏎ resolve` first): unchanged; the refusal now matches it.
- Palette IN PROGRESS rows (12e D1): the one oracle case above.
- Keymap / `keymap_ids.txt`, journal/undo, graft-ui: none (`git diff --stat` empty, shown).
- KNOWN_GAPS `M8/resolve-refusal-space-dead`: closable on this ticket's merge — conductor's.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~25k: check-first 3k; fn + callers 5k; tests 9k; walks/TESTING 4k; NOTES/MAP 4k.

## NOTES (builder, 2026-10-08)
- No compaction. Scout: none (ticket rules 10k NOTES Part B is the scout; re-checked at base).
- Check first: `acting(Banner, ⏎)` = `app.open` (Global row, `graft-ui/src/keymap/table.rs:99`); the banner answers
  Enter by opening the resolver on the first conflicted file (`update/banner/focus_keys.rs:119-121`). `M`, `e` =
  `operation.mergetool`, `operation.edit`. Held by the pin test.
- Red shown: pin and spell test failed on the const (`left: Some("M mergetool  e edit  space marks resolved")`),
  green after.
- Final signature: `continue_refused(reason: String, ctx: ContextId, keys: &Keymap) -> Status`; `resolve_keys` lives
  in `banner/resume.rs` beside `mergetool_fallback`. `⏎` clause only at `ContextId::Banner` (elsewhere `app.open`
  drills into something else, not the resolver).
- **Third caller found, not in spec:** `c` from the changes pane (`update/keys/live_c_tests.rs`
  `c_from_the_pane_and_from_the_banner_emit_the_same`) also reaches `continue_refused`; at Changes it now says
  `M mergetool  e edit` (no `⏎`, which does not resolve there). Test edited: reasons equal, keys each = `resolve_keys`
  at its own `context_of`.
- Palette path: Continue at a conflict stop gives the reason with keys `None` (Graph) — compared keys-excepted via
  `keys_excepted` helper (`palette/tests.rs`).
- `grep -rn 'RESOLVE_KEYS\|space marks resolved' crates` → empty. Callers of `continue_refused(`/`resolve_keys(`:
  pressed.rs, status_read.rs, finish/tests.rs, banner/tests.rs, live_c_tests.rs, palette/tests.rs, stops.rs.
- Walks: `tests/it/conflict_interim.rs:506` and `tests/it/m5_walk/conflicts.rs:248` updated; TESTING.md 27 updated,
  its next steps unchanged.
- Touches: hint bar, keymap table, graft-ui, docs/shots — `git diff --stat 4527016f -- docs/shots crates/graft-ui` empty.
  Captures not re-shot (none hold the words; m5_walk See-label text changed but See labels are assertions, not shots —
  unverified whether any capture prints the walk's status line; 11e's capture run will show).
- Gates: fmt, clippy -D warnings, nextest 5901/5901 (10 skipped), test --doc all green, workspace-wide.
