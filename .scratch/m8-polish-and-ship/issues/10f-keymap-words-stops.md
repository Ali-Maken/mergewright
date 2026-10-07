# G-keymap 2f — stop, outcome and failure words spell keys from his keymap

Status: done (2 sites held at W2 stop, see NOTES)
Build with: **Sonnet** — every site listed, every rule decided (10b W1–W6), each held by a table row. Not Fable.
Blocked by: 10b, 10e (same callers' files; test scaffold).
Needs: from 10e — `update/tests/keymap_words.rs` helpers, `OpKeys`; from 10/10c — `Keymap::{spell_keys, key_verb,
after, translate}`, `AppState::keymap`.
Crate set: graft + graft-ui `tests/no_literal_keys/known_stops.rs` only.
**Files:** graft `update/banner/{focus_keys,pressed,resume}.rs`, `update/commit_editor/refusal.rs`, `update/mod.rs`
(one site), `update/operate/{outcome.rs,outcome/staging_not_kept.rs,failure.rs,diff_cursor.rs}`,
`update/finish/keys.rs`, `view/journal/summary.rs`, `update/tests/keymap_words/stops.rs`, `MAP.md` (own entries);
graft-ui `tests/no_literal_keys/known_stops.rs`. Plus any path 10b put in `known_stops.rs` (conductor amends).
Disjoint from 10g, 10h, 10i except MAP.md.

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** → W1–W6
- `crates/graft/MAP.md` entries for each module in Files; `OperationRefusal`, `operation_refusal`, `tab_keys`
- `crates/graft-ui/MAP.md` `## keymap` (`key_verb`, `after`, `spell_keys`)

## Scope
In — each site spelled per W1–W6 (sites from `10b-grep-hits.txt` at 80dd116b; regenerate from `known_stops.rs`):
- `banner/focus_keys.rs`, `banner/pressed.rs` ×3, `view/journal/summary.rs`: `c continue  a abort`;
  `banner/pressed.rs` `u puts the merge back`
- `banner/resume.rs` `e edits {}  a abort`, `a abort`; `update/mod.rs` `e edits {}  a abort`
- `commit_editor/refusal.rs` `then c finish  a abort`, `then c continue  a abort` (keep `OperationRefusal`'s one-value
  rule: `editor_words()` and `status()` stay from one value)
- `operate/outcome.rs` `c continues`, `u undoes`; `outcome/staging_not_kept.rs` `u undoes it, staging included`
- `operate/failure.rs` `S p puts them back` (W5), `u restores the backup` ×2, `e edit  M mergetool`,
  `S stash {count} changed`
- `operate/diff_cursor.rs:180,297` (from 10b's original list; not grep hits)
- `finish/keys.rs` `⇥ to write one`
- every other `KNOWN: 10f` row in `known_stops.rs`
Out: prose rows; graft-ops words (10e); rewording (20 — its item 17 is in `failure.rs`).

## Signatures
No new public items. Word fns gaining `keys: &Keymap` are named in MAP.md.

## Acceptance (each tick names its test, at tick time)
- [x] `update::tests::keymap_words::stops::each_stop_word_spells_the_remapped_key` — 11 rows (one helper row covers 5 call sites; count
      stated): sweep keymap → remapped label, no default label of a moved action as a whole token; shipped →
      byte-identical to the literal.
- [x] `…::stops::each_word_names_the_row_that_answers_it` (10 pins) — W2 pin per site (ctx stated). A mismatch was a stop.
- [x] `…::stops::an_unbound_key_drops_its_clause` + `…::stops::an_unbound_key_drops_the_failure_clause` — `banner.abort` (or the row W2 pins) `= "none"`: `c continue`
      alone; the failure line with stash unbound drops its `S …` clause; no `—`.
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green — yes; but `known_stops.rs` still holds 2 `KNOWN: 10f W2 stop` rows (NOTES). Not ticked.
- [x] Captures: `git diff --stat b1756a50 -- docs/shots` empty (no reshoot; capture tests green in `cargo nextest run`; main has since moved, so diff vs base), or only W6 normalisations listed in NOTES.
- [ ] → 10j (conductor 2026-10-07: ticked naming no test). MAP.md graft: changed fn signatures (doc, no test).

## Touches
- Callers of each changed word fn (compiler); tests asserting these strings (grep each literal).
- The banner and journal summary draw the same `c continue  a abort` state: both spelled by one helper, say which.
- Journal/undo: words only — `git diff main -- crates/graft/src/update/journal*` empty.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~50k: ~21 sites ~20k, table + pins ~20k, MAP/NOTES ~5k.

## NOTES (builder 10f, 2026-10-07)
No compaction. No scout: no `scouts/` file names 10f; sites were listed in the ticket, read directly.
- **W2 stop (2 sites, not built):** `e edits {path}  a abort` in `update/banner/resume.rs:75` (`open_mergetool`, no merge.tool) and `update/mod.rs:511` (`mergetool_returned`). `open_mergetool` is reached from the banner (`banner/focus_keys.rs:115`), the resolver (`resolver/handover.rs:43`) and the changes/diff pane (`detail/keys.rs:234`) — `e` there is `operation.edit`, `resolver.edit`, `changes.edit`. Two+ candidates → planner ruling. Rows stay `KNOWN: 10f W2 stop` in `known_stops.rs`.
- **Contexts pinned (W2):** banner keys → Banner (also for the journal row and the amend-at-edit-stop `c continues`, which send him there); `S` → Graph, `p` → Stash (W5); `u` → Graph and Banner; markers refusal `e`/`M` → Changes (`operate.rs:386` Space on Unmerged); `v` → Diff; finish `⇥` → Finish (`acting` = `app.next_pane`).
- **One helper:** `banner::continue_abort` spells `c continue  a abort` for focus_keys, pressed ×3 and `view/journal/summary.rs` (`in_progress`). With both unbound the journal row is `in progress`, no dash.
- **Commit editor (W4):** `operation_refusal` takes `keys`; the editor row (`CommitEditor::refusal_before_subject`) passes `Keymap::shipped()` (drawn while its text field is open), pane statuses pass `state.keymap`. Each `OperationRefusal` still makes both words from one value.
- **Also spelled (not in the grep list, same fns):** `then c finishes it` (merge in shell), `— c hands it to git` (rebase/cherry-pick in shell), `diff_cursor.rs` `press v …` and the discard box's `` `u` can undo``.
- **Not table-tested (inline in fns private to their modules):** outcome `c continues`/`u undoes`, MarkersRemain `e edit  M mergetool`, finish `⇥ to write one`, abort box `u puts the merge back`, diff_cursor ×2. Shipped bytes held by existing tests (all 5741 green); W2 pins cover their rows; swept spelling **unverified** by test.
- **Prose row added to `known_stops.rs`:** `a copy is in stash@{{{n}}}` (split from `u restores the backup`; `a` is the article).
- **Outside the Files list (compiler-forced callers):** `banner.rs` (one re-export), `commit_editor.rs`, `amend_into.rs`, `undo.rs` (2 `edits_clause` calls), `view/journal.rs` (1 call), tests in `amend_into/`, `keys/`, `commit_editor/`, `operate/tests/`, `banner/tests/`.
- Not mine, seen: `banner.rs` `RESOLVE_KEYS` ("M mergetool  e edit  space marks resolved") is still a literal (not in known_stops; allowed elsewhere?); `prose.rs` row "a copy may still be in S" names the stash key `S`.
