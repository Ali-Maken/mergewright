# Scout 46: Q1 "replaced" (code at 8c7b46df)

## Where a lost run meets its event
- `build` crates/graft-ops/src/journal_list.rs:275 — delegates to `rows::assemble` (:286).
- Attach: crates/graft-ops/src/journal_list/rows.rs:116-131 — `place(&olds, run.tip)` gives `(event, hint)`; pushes `LostCommit { run, subject, held_by }`. `held_by` from `held_by(hint, …)` rows.rs:249-265. Rows emitted :136-150 (`ListRow::Lost { event: at, commit }`, then `LostEdit` only for `Event::Graft`).
- `Event::Graft { entry: usize, own }` rows.rs:26-29 — `entry` indexes `entries`, so the `JournalEntry` is reachable here.
- `LostCommit` journal_list.rs:94-102 (no fate); `HeldBy` :80-90; `ListRow` :117-.
- `in_force_all` graft-ops/src/journal.rs:208; `undoable` :247; `undoable_kind` :231. Types: `Oid` graft-core/src/oid.rs:22; `JournalEntry` graft-core/src/journal.rs:262 (match the ticket's paths).

## Words and glyph
- `journal_list::words` (words.rs:27) words `$` reflog records only — **not** the lost row.
- Lost row words: graft/src/view/journal.rs:240 `lost_words` ("· was X's tip · no branch holds it" / "· X is gone" / "· was HEAD", "+N under it"); mapped to `RowKind::Lost` at :139-141.
- `RowKind` graft-ui/src/journal_screen/view.rs:50 (Graft, Outside, Lost, LostEdit).
- Glyph: graft-ui/src/journal_screen/pieces.rs:60-66 hardcodes `"└ ✗ lost"`, `Ink::Warning`.

## ASCII glyph for `✗`
- **None found.** grep for ascii/unicode tier in journal_screen*, view/journal.rs: no hits; `✗` drawn identically in both tiers (pieces.rs:66). Ticket assumes an ASCII tier exists for J — could not confirm one.

## `↺` already claimed — contradiction
- `↺ put back` (ASCII `put back`) graph row tag: graft-ui/src/graph_screen/mod.rs:444, 470-471; also graft/src/update/recover.rs:64, update/undo/lands_on.rs:39. Ticket's rule: other meaning → stop and report. `~` grep not done (too common to judge by grep alone).

## Keys (update/journal/keys.rs)
- `n`/`N` :97-98 → `next_lost` :127-143: steps `ListRow::Lost | LostEdit`.
- Also claimed: `J`/Esc close :85, k/j/Up/Down, Space expand, `b` branch_here :108, Enter :109. Any key sets `journal.touched` :83 (kills preselect).
- Preselect: graft/src/update/journal.rs:102. Keymap §2.11 Enter text: docs/design/02-keymap.md:506.

## Legend
- `legend()` graft-ui/src/journal_screen.rs:58-70, one string ending `✗ a commit no branch holds`. Shedding = `cut_at_word(legend, width)` :412 (tail cut by width, not per clause); drawn only if Ready and height ≥4 (:405).

## Tour
- update/tour.rs mentions journal (:4); no `✗`/lost hit in tour.rs — could not confirm whether a tour step reads J rows.
