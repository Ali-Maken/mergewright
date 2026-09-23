# fix-preview-verb-confirms: `y` confirms any Operation preview; in one opened by `m` or `r`, the same key again confirms too; the cursor still rests on Cancel

Status: done (2026-09-23). Was: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: the conductor's keymap edit, below. `02-keymap.md` must carry the `y` row before this is built (CLAUDE.md: a key the doc does not name is not free).
Was: draft · **P1 approved** (user, 2026-09-23: "keep all 5"). The conductor writes the Ghost Preview row into `02-keymap.md` (the note's "Keymap rows owed"). ADR-0038 §1 is kept, not amended.
**Revised after scout (2026-09-23, `.scratch/m6-conflicts/scout-11.md`, plus the user's ruling the same day):**
- (a) The key test is a new helper, `confirm_key(key, opened_by)`. `is_history_key` takes a `KeyPress`, not the `&str` that `opened_by` is.
- (b) **`b` again does not confirm** the name-prompt preview (user: "well not, either pick another key for confirming").
- (c) **`y` confirms every Operation preview**, whoever opened it (`m`, `r`, `.`, the name prompt's `b`). The conductor chose `y` for "yes". `m m` / `r r` stay as the approved P1 shortcuts. A journal-opened `RevertTo` preview is not an Operation preview and gains nothing.
- (d) **No "still predicting" notice.** No channel reaches the screen from inside the open preview: `GraphView::notice` belongs to the renderer tier, and `Status` has no neutral transient. While `planning`, the box already draws `predicting…` in place of its lines, and that answers the swallowed key. The press is **not** queued (M5's promise: you see the picture before you confirm).
- (e) The footer label has a fit ladder for narrow boxes.

**`y` collision check** (`02-keymap.md`, 2026-09-23):
- `y` is bound once, in §2.3 (graph: copy the SHA, line 203).
- The Ghost Preview answers every key itself and swallows the rest (`preview.rs:409`), so the graph's `y` is not live while the preview is open.
- §2.3b binds only `m`/`r`/`⏎`/`Esc`.
- **No collision inside the preview's scope.**

**Keymap rows owed (the conductor writes them before the build):**
- §2.3b, a new row: `` `y` | Confirm, in any merge/rebase/operation preview (opened by `m`, `r`, `.` or the name prompt) once its plan has landed. The cursor still rests on Cancel (ADR-0038 §1). Not in a journal revert preview (user ruling 2026-09-23) ``.
- §2.3b, the `m`/`r` row: "a `.`-opened preview is unchanged" becomes "a preview opened any other way gains only `y`".
- §2.1, the modal-pane list: the **Ghost Preview** joins it. It already answers every key and swallows the rest. With `m`/`r`/`y` it now has letters of its own, and §2.1 says such a pane is added "by amending the list above".

Build with: **Sonnet**. One key rule, one helper and one footer label, fully specified. The pattern exists (`enter`, `FOOTER`). Scout used.
Needs: none.
Crates: graft (`update::preview`, `update::keys`), graft-ui (`preview_box`).

## Read first
- `.scratch/m6-conflicts/scout-11.md` (whole; half a page; every file:line this ticket touches).
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P1, New UI ("preview (P1)").
  - Amendment 2: Journey J1, Keymap rows owed (Ghost Preview).
- `docs/adr/0038-m5-spec-decisions.md` §1: why the cursor never moves by danger. This ticket must not move it.
- `crates/graft/MAP.md` §`update::preview` (`PreviewSubject::Operation { opened_by }`, `preview_key`, `confirm_preview`, `is_history_key`); `crates/graft-ui/MAP.md` §`preview_box` (`PreviewBox`, the footer border text), §`frame_box` (`border_row`, `cells`).
- `docs/design/02-keymap.md` §2.1 (modal panes; `.` closes what `.` opened), §2.3b (the Ghost Preview rows, as the conductor amended them).

## Scope
In:
1. **The helper.** `confirm_key(key, opened_by) -> bool` is true for:
   - a plain `y` (no ctrl/alt), whatever `opened_by` is;
   - or `opened_by` is `"m"` or `"r"`, `is_history_key(key)` is `Some` (so ctrl/alt are refused the same way), and the key's character equals `opened_by`.
   Anything else → false. That includes `b` in a `b`-opened preview, `.` in a `.`-opened one, and `r` in an `m`-opened one.
2. **In `preview_key`**, before the catch-all swallow (`preview.rs:409`): when the subject is `Operation { opened_by, .. }` and `confirm_key(key, opened_by)`, the key is **Confirm**, checked in this order:
   - frame below `PREVIEW_MIN` (`!fits`) → nothing, like `⏎`;
   - `planning()` → swallowed: no command, no status change, the box stays open and keeps drawing `predicting…`;
   - `!acts()` → swallowed (the box already says why it cannot act); **not** closed, unlike `⏎` on Cancel;
   - else → `confirm_preview`, the same path `⏎` on Confirm takes, with the same boxed `Operation` moved across (never rebuilt). The cursor position is irrelevant: `y` or `m m` confirms with the cursor on Cancel.
   A `RevertTo` subject never reaches this check, so `y` stays swallowed there.
3. **Unchanged:**
   - The cursor still opens on Cancel (`PreviewBox::new`).
   - `⏎` still acts on the choice under the cursor.
   - `←`/`→`/`Tab`/`BackTab`/`Esc` are unchanged.
   - `.` keeps closing what it opened.
4. **Footer.** Two new `PreviewBox` fields, set at both build sites (`preview.rs:664` planning, `:690` planned):
   - `confirm_key: true` for every `Operation` subject, `false` for `RevertTo`;
   - `again: Some(("m", "merge"))` / `Some(("r", "rebase"))` from `opened_by`, `None` for every other opener.
   The bottom border draws the widest rung that fits the box's inner width (`cells`). Leave out any part whose field is off.
   1. `y confirm  m merge  ←→ choose  ⏎ act  esc cancel`
   2. `y confirm  m merge  ⏎ act  esc cancel`
   3. `y confirm  ⏎ act  esc cancel`
   4. today's `FOOTER`, drawn as today.
   With `confirm_key: false` and `again: None`, it is always today's `FOOTER`.
5. `update::keys::BOUND_ACTIONS`: add the preview's `y confirm` and `m merge` / `r rebase` rows in the shape the table's existing preview-context rows use. If the table has no preview context today (the scout says rows exist for the graph only), add none. Say so in the ticket's NOTES with the table's line: the hint sweep reads the main footer's `HintContext::Preview`, which this ticket does not change.

Out: any other confirm box; the cursor default; the push and pull boxes; the journal's `RevertTo` preview; the main footer's `HintContext::Preview` list; any notice or status line; `02-keymap.md` (the conductor edits it).

## Signatures
```rust
// graft-ui/src/preview_box.rs
pub struct PreviewBox {
    /* existing */
    pub confirm_key: bool,                                  // draw `y confirm`
    pub again: Option<(SanitizedText, SanitizedText)>,      // ("m", "merge") / ("r", "rebase")
}

// graft/src/update/preview.rs: new, private to the module, beside is_history_key
/// `y` in any Operation preview; `m` in an `m`-opened one, `r` in an `r`-opened one; nothing else.
fn confirm_key(key: KeyPress, opened_by: &str) -> bool;
// preview_key, enter, confirm_preview, is_history_key: signatures unchanged
```

## Acceptance (name the verifying test when you tick)
- [x] `update::preview::tests::the_opening_key_again_confirms` (passes): `m`-opened subject over the unit fixture's op, plan landed, cursor asserted on Cancel, `m` → exactly that `Operation`. Real `m m` end to end: `tests/it/preview.rs::m_again_merges_through_the_real_path` (HEAD moves).
- [x] `…::y_confirms_every_operation_preview` (passes): openers `m`, `r`, `.`, `b`.
- [x] `…::y_does_not_confirm_a_journal_revert` (passes; also asserts the box has no `y`/verb footer fields).
- [x] `…::b_again_does_not_confirm` (passes).
- [x] `…::the_opening_key_waits_for_the_picture` (passes): `m m`, `m y` while `Pending`; status unchanged, `planning` still set on the drawn box.
- [x] `…::the_confirm_key_does_not_close_a_preview_that_cannot_act` (passes; `an_empty_plan`).
- [x] `…::the_cursor_still_opens_on_cancel` (passes).
- [x] `…::the_other_verb_stays_swallowed` (passes).
- [x] `…::confirm_key_refuses_modifiers` (passes).
- [x] graft-ui, all in `crates/graft-ui/src/preview_box/tests.rs`, passing:
  - `preview_box::tests::the_footer_names_both_confirm_keys` (rung 1, `m`-opened, wide box);
  - `…::the_footer_label_shrinks_by_its_ladder` (rungs 2, 3 and 4, each at the width where it first applies);
  - `…::a_dot_opened_footer_names_only_y` (`y confirm  ←→ choose  ⏎ act  esc cancel`);
  - `…::a_revert_preview_draws_the_old_footer`.
- [ ] Journey `a-merge-to-stop` at 2 keys (acceptance ticket).
- [x] Captures under `docs/shots/m6/`: `fix-preview-verb-120x40.txt` (`m`-opened) and `fix-preview-verb-name-prompt-120x40.txt` (`b`-opened). Golden, asserted by `preview::capture_fix_preview_verb_for_docs_shots` and `name_and_drag::capture_fix_preview_verb_name_prompt_for_docs_shots`. Colour unverified; the user judges by eye. **What right looks like:**
  - `m`-opened: the box's bottom border reads `y confirm  m merge  ←→ choose  ⏎ act  esc cancel`.
  - `b`-opened: it reads `y confirm  ←→ choose  ⏎ act  esc cancel`.
  - Both: the buttons row shows `▸[ Cancel ]`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small. One context.

## NOTES (builder, 2026-09-23)
- Scout used (`scout-11.md`). No compaction.
- Scope 5, `BOUND_ACTIONS` (`crates/graft/src/update/keys.rs:754`): it has no preview-context rows (graph, pane, banner and resolver only), so none added, per scope. The main footer's `HintContext::Preview` is unchanged.
- Journey `a-merge-to-stop` left unticked: the acceptance ticket counts it.
- Four m5 golden captures regenerated. Only the box footer line changed (`r`-opened, now `y confirm  r rebase  ←→ choose  ⏎ act  esc cancel`): `docs/shots/m5-ui-preview-{rebase,scrolled}-{80x24,160x48}.txt`.
- The name-prompt capture builds its own dated stash repo (`run_at`), because `a_repo_with_a_stash` stamps the wall clock and the frame drifted between runs.
