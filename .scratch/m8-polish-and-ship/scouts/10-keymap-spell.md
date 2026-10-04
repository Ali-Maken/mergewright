# Scout 10 — keymap spell from the table (main @ a6f9f12e)

## What reads graft_ui::keymap today
- graft-ui: only `keymap/*` itself. `hints/lists.rs` mentions "Keymap §…" in comments only — no hint, menu, box or footer reads the table.
- graft: `update/edge.rs` (translate, `SharedKeymap`), `update/{mod,state,confirm,undo}.rs`, `update/keys/table_check.rs`, `config/keys.rs`, `empty_state.rs`, `event_loop/{startup,places}.rs`, tests `update/tests/keymap_edge.rs`.
- API present: `Keymap::spell(&ActionId) -> KeyLabel` (`keymap/resolve.rs:129`, first chord or `—`), `chords` (:137), `KeyChord::label` (`chord.rs:107`), `ActionId(pub &'static str)` (`keymap.rs:184`, derives Copy/Eq/Hash).
- `Keymap` derives only `Debug, PartialEq, Eq` (`resolve.rs:7`) — no `Clone`. 09b NOTES propose deriving `Clone, Debug, PartialEq, Eq` and dropping graft's `SharedKeymap` Arc wrapper (`update/edge.rs`).

## Key spellings written as literals
Hints (structural, via `hint("key", verb)`): `hints/lists.rs` ~145 calls, `lists/resolver.rs` ~41, `hints.rs` ~18 (incl. `hint("esc",…)` :334, `BACK_TO_BLAME` :539, `BACK_TO_HISTORY` :543), `fitting.rs` 3. `Hint.key: &'static str` at `hints.rs:77-79`; `hints_for` `hints.rs:324`; `every_hint` `catalogue.rs:538`.

**Concatenated keys a single `spell` can't reach** (20 in hint lists): `j/k`, `n/N`, `←/→`, `⏎/space`, `O/T`, `⌃O/⌃T` (`lists/resolver.rs:78-79`), plus `J/esc close` (`journal_screen.rs:221`), `M/Esc back`, `↑↓ select` (`map_screen/render.rs:131`), `focus (F/Esc off)` (`graph_screen/mod.rs:310`), `←→ choose` (`preview_box.rs:42`), `⌃⏎/Commit commit` (`commit_editor.rs:176`).

`.` menu `(key)` labels: `context_menu.rs:105-108,192,195,197,220`; graft `update/context_menu/verbs.rs:161,163,249,258` (Cherry-pick/Merge/Rebase `(o)/(m)/(r)`).

Footers/boxes (graft-ui): `frame_box.rs:307,317`; `preview_box.rs:42,46,85-89,99`; `stash_screen.rs:25,181,184,186`; `commit_editor.rs:172,176,227`; `picker.rs:257,259`; `journal_screen.rs:46` (LEGEND `u`),221; `finish_summary.rs:182`, `finish_summary/list.rs:99`; `blame.rs:242,272`; `history.rs:347`; `conflict_resolver.rs:105,107`; `empty_state.rs:99,101` (tuple form).

Status/refusal words (graft): `update/remote.rs:285,349`; `quit.rs:103`; `clash.rs:297-298`; `resolver.rs:399`; `operate/diff_cursor.rs:180,297`; `preview.rs:130`; `preview/answer.rs:338`; `preview/take.rs:83`; `preview/line_rows.rs:228`; `place/answer.rs:181,184`; `rewrite_editor/words.rs:14`; `undo.rs:756`; `undo/consequence.rs:415` (`U`; :81 already takes `{key}`).

**graft-ops (needs label passed in):** `operation.rs:908,920(generic "press the key"),924,1047,1055,1154`; `apply/remote.rs:330`; `apply/error/words.rs:38`.

Count: ~55 non-hint literal sites + ~200 `hint(` calls — over the ticket's ~60 split threshold if counted literally.

## Exemptions from 09
`release`, `type`, `tab` in `table_check.rs:27`; `tab` source `keys/bound.rs:201` (`("tab","after")`) vs `⇥` elsewhere; `stash_screen.rs:181` also writes `tab`.

## Tests
`every_key_the_bar_draws_is_answered`, `NEVER_HINTED`, `bound_actions`: `update/tests.rs`, `update/keys/bound.rs`. Menu label asserts `context_menu.rs:455-812`. 398 files in `docs/shots`; not individually checked which assert spelled keys.

## Contradictions / unconfirmed
- Ticket: `Hint` one `action: ActionId` — 20 hints spell two keys (`j/k`, `⌃O/⌃T`); one ActionId can't draw them.
- Ticket expects "graft-ops words take the label as a parameter" — `operation.rs:920` names no key at all ("press the key again").
- Could not confirm whether docs (`02-keymap.md`, man) are in scope; ticket names none.
