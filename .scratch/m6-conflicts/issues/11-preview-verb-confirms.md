# fix-preview-verb-confirms: in a Ghost Preview opened by `m` or `r`, the same key again confirms; the cursor still rests on Cancel

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: none
Was: draft · **P1 approved** (user, 2026-09-23: "keep all 5"). The conductor writes the Ghost Preview row into `02-keymap.md` (the note's "Keymap rows owed"). ADR-0038 §1 is kept, not amended.
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Sonnet**: one key rule and one footer label, fully specified.
Needs: none.
Crates: graft (`update::preview`), graft-ui (`preview_box`).

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P1, New UI ("preview (P1)").
  - Amendment 2: Journey J1, Keymap rows owed (Ghost Preview).
- `docs/adr/0038-m5-spec-decisions.md` §1: why the cursor never moves by danger. This ticket must not move it.
- `crates/graft/MAP.md` §`update::preview` (`PreviewSubject::Operation { opened_by }`, `preview_key`, `confirm_preview`, `is_history_key`); `crates/graft-ui/MAP.md` §`preview_box` (`PreviewBox`, the footer border text).
- `docs/design/02-keymap.md` §2.1 (`.` closes what `.` opened), §2.3 (`m`, `r`).
- **Scout question:** where the preview's footer border string is built, and whether `planning` is visible to `preview_key`.

## Scope
In:
1. In `preview_key`: when the subject is `Operation { opened_by }` and `is_history_key(opened_by)` (`m` or `r` only), that key again **is `Confirm`**. It takes the same `confirm_preview` path, with the same `Operation` moved across.
   - While `planning`, it is swallowed with the notice `still predicting · wait for the picture`.
   - Below `PREVIEW_MIN` it does nothing, like `⏎`.
2. **Unchanged:**
   - The cursor still opens on Cancel.
   - `⏎` still acts on the choice under the cursor.
   - `←`/`→`/`Esc` are unchanged.
   - A preview opened by `.` (or by the journal) gets nothing new; `.` keeps closing what it opened.
3. `PreviewBox.again = Some(("m", "merge"))` or `("r", "rebase")`. The footer border reads `m merge  ←→ choose  ⏎ act  esc cancel`.
4. `update::keys::BOUND_ACTIONS` and the hint sweep know the key.

Out: any other confirm box; the cursor default; the push and pull boxes.

## Signatures
```rust
// graft-ui/src/preview_box.rs
pub struct PreviewBox { /* existing */ pub again: Option<(SanitizedText, SanitizedText)> }
// graft/src/update/preview.rs — preview_key's signature unchanged
```

## Acceptance (name the verifying test when you tick)
- [ ] `update::preview::tests::the_opening_key_again_confirms`: `m` on `feature`, the plan lands, `m` → one `Command::Operate` whose `Operation` equals `PreviewState.plan`'s.
- [ ] `…::the_opening_key_waits_for_the_picture`: `m m` with the plan `Pending` → no command, and the notice `still predicting`.
- [ ] `…::the_cursor_still_opens_on_cancel`: `m` then `⏎` → the box closes with no command (ADR-0038 §1 held).
- [ ] `…::a_dot_opened_preview_gains_no_key`.
- [ ] graft-ui `preview_box::tests::the_footer_names_the_opening_key`.
- [ ] Journey `a-merge-to-stop` at 2 keys (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-preview-verb-120x40.txt`. **What right looks like:**
  - The box's bottom border reads `m merge  ←→ choose  ⏎ act  esc cancel`.
  - The buttons row shows `▸[ Cancel ]`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small. One context.
