# G-keymap 2i — preview, remote and rewrite-editor words spell keys from his keymap

Status: done
Build with: **Sonnet** — every site listed, every rule decided (10b W1–W6), each held by a table row. Not Fable.
Blocked by: 10b, 10e (callers' files; test scaffold). **After 09d merges** (`view/rewrite_editor*` holds 09d's
`rewrite_editor::right` call site).
Needs: from 10e — `update/tests/keymap_words.rs` helpers; from 10/10c — `Keymap::{spell_keys, key_verb, after,
translate}`, `AppState::keymap`.
Crate set: graft + graft-ui `tests/no_literal_keys/known_preview.rs` only.
**Files:** graft `update/preview.rs`, `update/preview/{history,line_rows,take,answer}.rs`, `update/remote.rs`,
`update/remote/push_box.rs`, `update/rewrite_editor/{open,words}.rs`, `view/rewrite_editor.rs`,
`update/tests/keymap_words/preview_remote.rs`, `MAP.md` (own entries); graft-ui
`tests/no_literal_keys/known_preview.rs`. Plus any path 10b put in `known_preview.rs`. **Not** `preview/boxes.rs`
(10h). Disjoint from 10f, 10g, 10h except MAP.md.

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `crates/graft/MAP.md` entries for each module in Files; `take_key`
- `crates/graft-ui/MAP.md` `## keymap`; `preview_box` (the `again` slot 10c moved to `ActionId`)

## Scope
In — each site spelled per W1–W6 (from `10b-grep-hits.txt` and 10b's original list; regenerate from `known_preview.rs`):
- `preview.rs:130`; `preview/answer.rs:338`; `preview/history.rs` `a stash isn't a branch · ⏎ opens it`
- `preview/line_rows.rs:228`, `· R to keep them`, `P will need force after`
- `preview/take.rs:83`, `o takes a commit · this row isn't one`, `o is waiting for the refs`
- `remote.rs:285,349` (incl. `c commit  S stash`, `p pull first`), `S s stashes them first` (W5), `m merge it
  r rebase onto it`, `· esc cancels`; `remote/push_box.rs` `f to fetch first`
- `rewrite_editor/open.rs` `R opens it`, `r then R opens it` (W5); `rewrite_editor/words.rs:14`, `p keeps a commit`,
  `r there changes the message`; `view/rewrite_editor.rs` `p keeps it`, `r on "{subject}" changes it` (W4 if drawn
  while the `r` field is open — 09d D5's rule)
- every other `KNOWN: 10i` row
Out: prose rows (`1 commit`, `1 new commit`); graft-ops remote words (10e); 20's items 10, 15, 16 (rewording).

## Signatures
No new public items. Word fns gaining `keys: &Keymap` are named in MAP.md.

## Acceptance (each tick names its test, at tick time)
- [x] (19 sites; red seen on assertion for the stubs, mutation red for `CannotStop`) `update::tests::keymap_words::preview_remote::each_word_spells_the_remapped_key` — one row per site (count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical.
- [x] (18 pins, contexts in its doc comment; green first run — pins, not code) `…::preview_remote::each_word_names_the_row_that_answers_it` — W2 pin per site (ctx stated).
- [x] (+ `an_unbound_take_fetch_or_rewrite_key_drops_its_clause`, `no_unbound_key_draws_a_dash`; red on assertion `Some("m merge it  r rebase onto it")`; `c commit  S stash` / `p pull first` are not in this ticket's files — see NOTES) `…::preview_remote::an_unbound_key_drops_its_clause` — `c commit  S stash` with stash `none` → `c commit`;
      `p pull first` with pull `none` → clause gone; `m merge it  r rebase onto it` with merge `none` →
      `r rebase onto it`; no `—`.
- [x] (`grep -c KNOWN known_preview.rs` → 0; one `article` row) `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_preview.rs` holds no `KNOWN` row.
- [x] (`git diff --stat 370d9c9b -- docs/shots` empty; shipped bytes held by the W6 half of `each_word_spells_the_remapped_key` and the full nextest run) Captures: shipped keymap → `git diff --stat main -- docs/shots` empty, or only W6 normalisations in NOTES.
- [x] (MAP lines `CannotStop`, `busy_text`/`esc_hint`, `Stoppable`, `m8/10i word fns`; checked by grep) MAP.md graft: changed fn signatures.

## Touches
- Callers (compiler); tests asserting these strings (grep each literal); preview captures m5–m8 (shipped: none
  expected).
- The diverged prompt (`m merge it  r rebase onto it`) and the preview box's `again` slot name the same rows: same
  spelling under the sweep keymap (one row in the table).
- Journal/undo: words only — `git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~50k: ~23 sites ~18k, table + pins ~20k, MAP/NOTES ~5k.

## NOTES (builder 10i, 2026-10-07)

No compaction. Scout: none — 10f/10g/10h/10j NOTES served; sites read directly.
- **TDD:** fns added as stubs returning the old literal, tests written, red on assertion (swept spelling, unbound drop), then spelled → green. `CannotStop` was written spelled first; its red seen by mutation (`keys` → `Keymap::shipped()`), reverted.
- **Shape changes:** `HISTORY_CANNOT_STOP`/`PUT_BACK_CANNOT_STOP` are now `CannotStop` values; `Stoppable::No(CannotStop)`; worded at `remote/in_flight.rs` `Esc`. `ESC_HINT`/`OVERWRITE_KEYS`/`DIVERGED_KEYS` consts → `esc_hint`/`overwrite_keys`/`diverged_keys(&Keymap)`; footer uses `ends_with_esc_hint` (false when `app.back` unbound, so the busy line gets its `…`).
- **Judgement (W3):** History line with `operation.abort` unbound keeps `a conflict stops it on the banner` and drops only `, where a aborts` — the fact is not a key clause. Whole-line refusals keep the fact: `waiting for the refs`, `this row isn't one`, `nothing to lease against`, `a stash isn't a branch`.
- **W4:** the editor view (title `until y`, `p keeps it`, `r on … changes it`) spells from `Keymap::shipped()` while the `r` field is open, else his. `until y` (09h) fixed here (`view::rewrite_editor::title`).
- **Not found in Files:** `c commit  S stash`, `p pull first` (latter is graft-ops, 10e). Not spelled.
- **Edge:** with both `graph.merge`/`graph.rebase` unbound the diverged line has no keys, so `forget_stale_divergence` forgets the upstream at once (nothing could press m/r anyway; palette could — question).
- **Outside Files (compiler-forced):** `operate.rs`, `undo.rs` (esc_hint), `remote/in_flight.rs`, `preview/boxes.rs` + `plan_lines.rs` (keys param), `rewrite_editor.rs`/`after.rs`/`keys.rs`/`keys/field.rs`, `view/frame.rs`, `view/frame/bars.rs`; tests `remote/tests.rs`, `preview/tests.rs`, `rewrite_editor/tests.rs` (`view_facts(Keymap::shipped())`, literals, unchanged expectations).
- **Touches:** callers via compiler; string tests: full nextest 5857/5857 green unchanged expectations; captures + journal + graft-ops: `git diff --stat 370d9c9b -- docs/shots crates/graft/src/update/journal crates/graft-ops` empty; keymap table untouched.
