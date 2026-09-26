# ADR-0048 — One switch rule; blame `n`/`N` find; `F` follows the focus; `┆` marks Graft's commands; History is a place

Date: 2026-09-26
Status: **accepted** (user, 2026-09-26, "all four yes": `docs/field-notes.md:79`).
Design note: `docs/design-notes/m7-power-tools.md`, pass 2a Proposed change 3 and pass 2b Proposed changes 7–11,
and the 2b keymap diff.

## Decision

1. **One switch rule.** `.` → Checkout and the name prompt's `⇥` (switch after creating,
   `crates/graft/src/update/name_prompt.rs:30`) switch instantly on a clean tree, like the picker's `⏎`; a preview
   (`y`) only with unsaved edits or a detached target. `u` goes back. Keymap §2.1, §2.3b.
2. **Blame `n`/`N`** = next/previous find match, as the graph's search; `/` finds. Older line is `^`, back is `Esc`,
   the line's list is `L`. Keymap §2.8, mock-up §1.12.
3. **`F` follows the focus.** Graph: Focus mode, unchanged. Detail pane on a file (row, diff line, changes pane): that
   file's history. `B`/`F` join §2.5. Keymap §2.3, §2.5.
4. **Command log:** Graft's commands are drawn `┆ git …` under `commands Graft ran`; `$` means only "outside Graft".
   `CONTEXT.md` Command Log.
5. **Journal (§2.11):** `⏎` per row kind (`●` `$` `✗`), `b`, `/`, `n`/`N` (with no find: next/previous `✗`), cursor
   preselects the newest `✗` newer than every `●`. Mock-up §1.10.
6. **History (§2.17, mock-up §1.17)** and blame are **places**: full width, stacked, `Esc` pops one, `⏎` to the graph
   hides the stack. `CONTEXT.md` Place, File history.

## Why

- 1: one answer to "does switching ask?" across picker, menu and prompt (UX §3.5, resting on `u`); `. ⏎ y` → `. ⏎`.
- 2: `n`/`N` means "next match" on every other screen (H4).
- 3: a verb applies to the cursor (§2.1); a new letter would spend a thin bank, moving Focus mode would break M1's key.
- 4: `$` read as "a shell ran this"; now the glyph separates Graft's work from the shell's.
- 5–6: as walked by the user in prototype rev 2 (`docs/field-notes.md:78`).

## Cost

- Checkout's M5/M6 preview captures go stale where they switch on a clean tree; ticket 10 (branch picker) carries the
  rule.
- `$ git` captures go stale: found in `docs/shots/m3-journal-expanded.txt` and `docs/shots/m3-contact-sheet.txt`
  (grep, 2026-09-26; no M5/M6 file matched). Ticket 24 (`journal-screen`) redraws the expanded row
  (`.scratch/m7-power-tools/issues/24-journal-screen.md:54,160`); it does not name the M3 files — open question for
  the conductor whether 24 regenerates them.
- If `F` in the detail pane falls through to Focus mode today, that stops (the `history-place` scout names the line).
