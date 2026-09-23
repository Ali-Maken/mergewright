# fix-preview-verb-confirms: in a Ghost Preview opened by `m` or `r`, the same key again confirms — the cursor still rests on Cancel

Status: draft · **gated on Proposed change P1** (02-keymap Ghost Preview keys). Do not file until approved. ADR-0038 §1 is kept, not amended.
Build with: set by the conductor after a scout (brownfield); expected **Sonnet** — one key rule and one footer label, fully specified.
Needs: none.
Crates: graft (`update::preview`), graft-ui (`preview_box`).

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Prior art row "Confirm a previewed op", Journey J1, Where I'd differ P1, New UI ("preview (P1)").
- `docs/adr/0038-m5-spec-decisions.md` §1 (why the cursor never moves by danger — this ticket must not move it).
- `crates/graft/MAP.md` §`update::preview` (`PreviewSubject::Operation { opened_by }`, `preview_key`, `confirm_preview`, `is_history_key`); `crates/graft-ui/MAP.md` §`preview_box` (`PreviewBox`, footer border text).
- `docs/design/02-keymap.md` §2.1 (`.` closes what `.` opened), §2.3 (`m`, `r`).
- Scout question: where the preview's footer border string is built, and whether `planning` is visible to `preview_key`.

## Scope
In:
1. In `preview_key`: when the subject is `Operation { opened_by }` and `is_history_key(opened_by)` (`m`, `r` only), that key again **is `Confirm`** — the same `confirm_preview` path, the same `Operation` moved across. While `planning` it is swallowed with the notice `still predicting · wait for the picture`. Below `PREVIEW_MIN` it does nothing, like `⏎`.
2. The cursor still opens on Cancel; `⏎` still acts on the choice under the cursor; `←`/`→`/`Esc` unchanged. A preview opened by `.` (or the journal) gets nothing new — `.` keeps closing what it opened.
3. `PreviewBox.again = Some(("m", "merge"))` / `("r", "rebase")`; the footer border reads `m merge  ←→ choose  ⏎ act  esc cancel`.
4. `update::keys::BOUND_ACTIONS` and the hint sweep know the key.

Out: any other confirm box; the cursor default; push/pull boxes.

## Signatures
```rust
// graft-ui/src/preview_box.rs
pub struct PreviewBox { /* existing */ pub again: Option<(SanitizedText, SanitizedText)> }
// graft/src/update/preview.rs — preview_key's signature unchanged
```

## Acceptance (name the verifying test at tick time)
- [ ] `update::preview::tests::the_opening_key_again_confirms` — `m` on `feature`, plan lands, `m` → one `Command::Operate` whose `Operation` equals `PreviewState.plan`'s.
- [ ] `…::the_opening_key_waits_for_the_picture` — `m m` with the plan `Pending` → no command, notice `still predicting`.
- [ ] `…::the_cursor_still_opens_on_cancel` — `m` then `⏎` → closed, no command (ADR-0038 §1 held).
- [ ] `…::a_dot_opened_preview_gains_no_key`.
- [ ] graft-ui `preview_box::tests::the_footer_names_the_opening_key`.
- [ ] Capture `docs/shots/m6/fix-preview-verb-120x40.txt`. **What right looks like:** the box's bottom border reads `m merge  ←→ choose  ⏎ act  esc cancel`; the buttons row shows `▸[ Cancel ]`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small. One context.
