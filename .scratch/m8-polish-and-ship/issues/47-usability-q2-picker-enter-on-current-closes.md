# Usability Q2: `⏎` on the branch he is on closes the branch picker

Status: ready
Blocked by: none
Build with: **Sonnet**. One key's effect on one row, held by state tests and a pty walk; no git write.
Brownfield: a **scout first** (ADR-0035), half a page: where the picker's `⏎` goes through `switch`, where "you're on
<b>" is produced and drawn, which bar clause the row shows, and whether a mouse click on a row is the same action.
Needs: none. Beside: `usability-q1-replaced-when-undoable` (disjoint files except `docs/design/02-keymap.md`, a
different section).

## Read first
- `docs/reviews/m8-usability.md` Q2 (:159-161); `docs/field-notes.md:97` ("close the picker")
- `docs/design/02-keymap.md` §2.16 (picker), :174 (the switch rule, `already on <b>`)
- `docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md` (Journeys: `picker-enter-current`; Lands)
- `crates/graft/MAP.md` `## update::picker`, `## update::switch`, `## view::picker`; `crates/graft-ui/MAP.md` `## picker`
- The scout's file. Then source, only the files it names.

## Decisions
- **`⏎` on the current branch's row** (HEAD is that branch; not detached): the picker closes, the graph has focus,
  the cursor lands on HEAD's row, the detail pane follows. No command, no journal entry, no status line (nothing
  happened, and the header already names the branch; CLAUDE.md: a status must not read as a problem when nothing is
  wrong). Dirty tree or not: nothing is switched, so no preview.
- **The bar on that row** offers `⏎ close`, not `⏎ switch to it`. The "you're on <b>" message the picker shows today
  is removed with it (the row's own current-branch mark stays).
- **Mouse:** if a click or double-click on a row acts as `⏎` today, it follows the same rule; if not, unchanged.
- **Unchanged:** every other row; `Tab`; `Esc`; detached HEAD (no row is "current"); `.` Checkout's
  `already on <b>` (keymap :174) — a different surface.
- **Keymap §2.16 `⏎` row** (this ticket edits it; the user decided): append "; on the branch you are on, closes the
  picker, cursor on HEAD, nothing written (amended 2026-10-11, usability Q2, user: 'close the picker')".

## Scope
In: graft `update::picker` (the `⏎` branch), `view::picker` (bar clause, message), graft-ui `picker` if the message is
drawn there; keymap §2.16 row; TestBackend captures of the picker. Out: switch rule; other surfaces.

## Signatures
None public. Internal shape for the scout to confirm:
```rust
// graft::update::picker — the one check, pure over state
fn is_current_branch(state: &AppState, row: &PickerRow) -> bool;
```

## Acceptance (each box: the test named at tick time)
- [ ] `update::picker::tests::enter_on_the_current_branch_closes`: picker closed, focus graph, cursor on HEAD's row,
      no `Command` returned.
- [ ] `…::enter_on_the_current_branch_with_a_dirty_tree_closes_too` (no preview opened).
- [ ] `…::enter_on_another_branch_still_switches` (unchanged path).
- [ ] `view::picker::tests::current_row_offers_close`: bar clause `⏎ close` on the current row, `⏎ switch to it` on
      another; no "you're on" text.
- [ ] pty walk `m8_walk::picker_enter_current` (on `main`: `C`, type `main`, `⏎` = 3 keys asserted; then the screen
      has no picker box, `▸` on HEAD's row; `git reflog -1` unchanged, truth from git). Capture
      `docs/shots/m8-q2-picker-enter-current-120x36.txt` (rule 6; colour unverified, rule 5).

## Touches
- **Callers:** `switch` (not called on this row any more: test above); mouse row click (scout); palette's "switch
  branch" action if it opens the same picker.
- **Screens:** the picker; the graph it lands on.
- **Keymap:** §2.16 text; no key added or moved. `?` help in the picker if it describes `⏎`.
- **Journal / undo:** nothing written: asserted by the reflog check.
- **Captures:** picker captures (TestBackend regenerate; drift summary `.scratch/m8-polish-and-ship/q2-shot-drift.txt`).
- **Docs:** keymap §2.16; graft MAP `update::picker` line.
- **Crates:** graft, graft-ui.
- Each is checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide.

## Size
One context, ~25k tokens after the scout.
