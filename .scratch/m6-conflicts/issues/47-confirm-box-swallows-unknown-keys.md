# confirm-box-swallows-unknown-keys: a key the confirm box does not bind leaves it open, never cancels it in silence

Status: done
From: `docs/reviews/m6-usability.md` M1 (:65-75), the part that does not depend on P12; review m1 (:137-140) for the box.
Blocked by: none. It holds whether P12 and P15 are approved or refused: whether or not `y` gets a meaning later, a key
the box does not bind must never cancel it without a word.
Build with: builder-sonnet. One match arm plus a pass-through, with every box site and caller already listed below.
Needs: none. It calls `graft::update::confirm::confirm_key`, as merged.
Crates: graft.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: "Decided here" 1, and the Journeys row "Push".
- `.scratch/m6-conflicts/scouts/47.md` (the code facts this spec is amended from).
- `docs/reviews/m6-usability.md` :65-75, :137-140.
- `docs/adr/0038-m5-spec-decisions.md` §1 (the cursor rests on Cancel in every box).
- `docs/design/02-keymap.md` §2.1 :59 (a modal answers every key, `Ctrl+C` excepted) and :164 (`q`, `Ctrl+C` quit).
- `crates/graft/MAP.md` §`update::confirm` (`PendingConfirm`, `ConfirmThen`, `confirm_key`).
- Source, only after the MAP (planner re-read 2026-09-24): `crates/graft/src/update/confirm.rs:88-176` and its `tests`
  module; `update/preview.rs:386-390` (the preview's `Ctrl+C` pass-through, the pattern to copy); `update/keys.rs:45-56`,
  `:116-130` (`boxes_key` in the claim order), `:257-259` (`graph_key` → `is_quit`).

## Scope
**In:**
- `confirm_key` (`confirm.rs:94-176`):
  - **`Ctrl+C` passes through:** return `false` before the `match`, exactly as `preview_key` does
    (`preview.rs:386-390`). Keymap §2.1 :59 already says every modal excepts it, and :164 says it quits. Today it
    closes the box instead (the catch-all at `:173`), which breaks §2.1. The box is left in state; the app quits.
  - **`Esc`** gets its own arm, `next.confirm = None`, as today.
  - **Every other key the box does not bind is swallowed:** `_ => {}`, with a comment saying why. The box stays open,
    its cursor does not move, `status` is untouched, and `confirm_key` returns `true`, so the key reaches nothing
    behind the box.
  - The doc comment at `:88-93` is reworded to the new rule.
- The fix is that arm. It is the cause: the box treats "unknown" as "cancel".

**Out:**
- `y` acting in the box: `confirm-box-takes-y` (blocked on P12 and P15).
- The `.` menu's pass-through (keymap §2.1, deliberate).
- The preview's own rule, which already swallows.
- The box's drawing and the hint bar's entries.
- Mouse handling: `mouse.rs` has no `confirm` path; `drag.rs:154` only reads `confirm.is_some()` (scout).
- Paste: it never reaches `confirm_key` (scout: `update/mod.rs:856-866`).

## The failing test first
`crates/graft/src/update/confirm.rs` tests: `a_key_the_box_does_not_bind_leaves_it_open`.
- Setup: open a push box (`ConfirmThen::Operate(Operation::Push(..))`), then press each of `y`, `q`, `j`, `P` and `Space`
  in turn.
- After each key, assert all three:
  - `confirm.is_some()`;
  - `view.selected() == 0`;
  - `status` is equal to what it was before the key.
- Red on today's code at the first key (`y` closes the box).

## Signatures
```rust
// graft/src/update/confirm.rs: unchanged signature
pub fn confirm_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress,
                   next_id: &mut impl FnMut() -> JobId) -> bool;
//   before the match: `Ctrl+C` → return false (preview.rs:388's test, same modifiers check)
//   KeyCode::Escape => next.confirm = None,   // explicit arm (today it is inside `_`)
//   _ => {}                                   // swallowed: the box stays; commented why
```

## Touches (each entry, and how the builder checks it)
- **Every box that `confirm_key` answers.** Eleven `confirm = Some` sites (planner grep of
  `crates/graft/src/update`, tests excluded):
  - `operate.rs:515` discard a patch; `operate.rs:657` discard paths;
  - `stash.rs:482` stash drop;
  - `banner.rs:491` resume skip; `banner.rs:507` resume abort. There is **no third resume box**: `Resume` is
    `Continue | ContinueWith | Skip | Abort` (`banner.rs:540-542`), and `Continue` launches without a box
    (`banner.rs:460`). "Start over" is the resolver's restart box, below;
  - `undo.rs:39` undo-after-push (`after_push.rs:206`); `undo.rs:300` the undo drift guard; `undo.rs:392` the
    revert-to drift guard;
  - `resolver/restart.rs:58` restart conflict (`d` in the resolver);
  - `remote/push_box.rs:24` push (`P`); **`remote/after_push.rs:247`** the re-push lease box (`repush_chosen`, reached
    from `[ Re-push old tip ]`).
  - Check: re-run `grep -rn "confirm = Some" crates/graft/src/update --include='*.rs'` and paste it in NOTES. Each
    non-test hit gets one row in `a_key_the_box_does_not_bind_leaves_it_open_in_every_box`: open the box, press `y`,
    and it is still open. A site the grep finds that is not listed above is added as a row and named in NOTES.
- **What reaches `confirm_key`, and in what order.** Only caller: `boxes_key` (`keys.rs:47`), second in `key_event`'s
  claims (`keys.rs:123-124`). Before `key_event`: the credential prompt, the name prompt and the search gate
  (`mod.rs:795-814`).
  - Check: `ctrl_c_with_a_box_open_quits`, through `update`, one row per box site above: `quit ==
    Some(Quit::Requested)` (or, with an operation live, whatever keymap :164's "confirms if an operation is in
    progress" path gives today; name its line), and no `Command::Operate`. If any row's `Ctrl+C` is claimed by a
    handler between `boxes_key` and `graph_key` (`keys.rs:125-257`, for example the journal browser under the
    revert-to box), **stop and report** the handler and line.
- **Tests and comments that relied on the catch-all.**
  - `update/tests.rs:3726-3734` (the key sweep exempts `confirm.is_some()`: "every key it has no use for cancels it")
    and `:3923-3926` (`settled()`, the same). Reword both comments. Check: the sweep is green, and does not now stop
    on an open box; if it does, say which key in NOTES.
  - `keys/bound.rs:131-133`: "`confirm_key`'s catch-all" becomes "`confirm_key`'s `Esc` arm". Comment only.
  - `undo.rs:730, 940, 951, 973, 1136` press `Enter` only: unaffected.
  - Check: `grep -rn "confirm" crates/graft/src crates/graft/tests` for tests that press a letter after opening a box.
    Each one moves to `Esc`; NOTES lists them, or says none with the grep.
- **Walks.** The pty scripts drive the Rust walk scenarios' keys (`pty-walk-m4.py:4-9` runs
  `m4_walk --capture <scenario>`), and none names a box (planner grep of `.scratch/*/pty-walk*.py` for `Cancel`,
  `box` and every box title: no hit). So a box closed by a letter would be in a Rust scenario.
  - Check: the gates run the fast-tier walks. For any walk file the gates do not run (`m3_walk.rs`, `m4_walk/`,
    `m5_walk/`, heavy tier), grep its steps for a box-opening key (`P`, `d`, `S`-drop, `u`, the resume keys) and read
    the next key sent. A non-`Esc`, non-`Enter`, non-arrow, non-`Tab` key moves to `Esc`. NOTES gives the files read
    and the hits, or "none".
- **Captures.** The box does not look any different.
  - Check: after the gates, `git status docs/shots` shows only this ticket's new capture. Any other change is a stop.
- **Other screens that draw the same state.** The confirm box draws no footer of its own (`view/frame/boxes.rs:245` →
  `graft_ui::confirm`). Its keys are in the hint bar: `esc cancel`, `←/→ move`, `⏎ choose` (`keys/bound.rs:130-135`).
  All stay true. (`←→ choose  ⏎ act  esc cancel` is the **preview**'s footer, `graft-ui/src/preview_box.rs:42`: not
  touched.)
  - Check: no diff under `crates/graft-ui` and no diff to `bound.rs` beyond the comment.
- **Journal and undo.** None: no `Enter` arm changes.
  - Check: `git diff` touches only `confirm_key` (the `Ctrl+C` pass, the `Esc` arm, the fall-through, the doc comment),
    the comments above, and tests.
- **Keymap.** No line exists for confirm boxes today. The conductor adds a row (text below); the builder does not edit
  the doc.
- **TESTING.md.** None known.
  - Check: `grep -n "any key\|closes the box\|other key" TESTING.md`. Hits go in NOTES for the conductor.
- **MAP.** graft §`update::confirm` `confirm_key`: replace "everything else (`Esc` included) closes without acting"
  with the new rule, `Ctrl+C` included.

## Acceptance (name the verifying test at tick time)
- [x] `a_key_the_box_does_not_bind_leaves_it_open` (confirm.rs tests; red on `y`, the first key: "closed the box";
  green after): red before the change (on which key), green after.
- [x] `a_key_the_box_does_not_bind_leaves_it_open_in_every_box` (confirm.rs tests, 11 rows; red before, green after): one row per site the grep found. NOTES gives the row
  count and the grep.
- [x] `esc_still_closes_every_box_without_acting` (confirm.rs tests, 11 rows, also asserts no write started): no `Command` emitted, `confirm.is_none()`.
- [x] `enter_on_the_acting_choice_still_acts` → existing `confirm::tests::enter_on_confirm_dispatches_the_operation`, green: the existing test (`enter_on_confirm_dispatches_the_operation`, cited
  at `update/tests.rs:3731`; find it), or a new one; name it.
- [x] `ctrl_c_with_a_box_open_quits` (confirm.rs tests, through `update`, 11 rows; red before at the quit assert): red on today's code (the box closes and nothing quits), green after.
- [x] Capture, test `two_boxes_over_content::capture_box_keeps_on_y_for_docs_shots` (asserts frame before == after,
  box and status unchanged, and the six strings) (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-box-keeps-on-y-120x40.txt`.
  - Scene: the push box open, `y` pressed.
  - **What right looks like:** the frame after `y` is the frame before it, byte for byte (the capture test asserts
    it). The box is titled `Push main → origin/main`, with `[ Cancel ]` under the cursor. The status line reads what it
    read before `y`. The hint bar holds `←/→ move`, `⏎ choose` and `esc cancel`.
- [x] MAP.md updated (doc, no test): graft §`update::confirm` `confirm_key`.
- [x] NOTES (below): the site grep, the tests and walk steps moved to `Esc`, the `Ctrl+C` path's lines, any TESTING.md hits.

## For the conductor (exact text; not the builder's to edit)
Add to `docs/design/02-keymap.md`, after §2.3b, as **§2.3c Confirm box**: `←`/`→` choose (clamped, Cancel first,
ADR-0038 §1) · `Tab` cycles · `Enter` the choice under the cursor · `Esc` cancels · `Ctrl+C` passes through (quits,
§2.1) · **any other key is swallowed: the box stays** (usability walk M1).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small. One context.

## NOTES

Built 2026-09-24 (Opus builder). Scout used: `.scratch/m6-conflicts/scouts/47.md`. No compaction.

- **Cause, not symptom:** `confirm_key`'s catch-all `_ => next.confirm = None` treated every unbound key as cancel.
  Now `Ctrl+C` returns `false` before the match (same check as `preview.rs:388`), `Esc` has its own arm, `_ => {}`
  swallows (commented). `Enter`'s dispatch moved verbatim into private `choose()` (clippy `too_many_lines`).
- **Site grep** (`grep -rn "confirm = Some" crates/graft/src/update --include='*.rs' | grep -v tests`): 11 hits, 11 rows,
  none unlisted: stash.rs:482, banner.rs:491, :507, undo.rs:39, :300, :392, operate.rs:515, :657,
  resolver/restart.rs:58, remote/push_box.rs:24, remote/after_push.rs:247. Rows are built from the `ConfirmThen` each
  site sets, not by driving each opener: `confirm_key` reads `then` only on `Enter`, so the rows prove the key rule per
  variant, not each opener's box shape.
- **Ctrl+C path:** `update` → `boxes_key` (`keys.rs:47`) → `confirm_key` returns `false` → `graph_key` → `is_quit`
  (`keys.rs:258`). No handler between claimed it in any row (default state, no operation live; keymap :164's
  "operation in progress" path not exercised).
- **Test that pinned the old rule, changed:** `update/remote/tests.rs` `box_is_modal` asserted `u U Ctrl+Z f p P`
  close the push box; now asserts the state is unchanged. No other test closed a box with a letter (full nextest green).
- **Comments reworded:** `update/tests.rs` sweep and `settled()`; `keys/bound.rs` "catch-all" → "`Esc` arm".
- **Walks:** m3_walk.rs, m4_walk/, m5_walk/ live under `tests/it` and ran in the gates, green. Their box answers read:
  `confirm_discard` (d, Right, Enter), drift box (Enter; Right Enter), `Step::Confirm` (Right Enter), undo loop
  (Right Enter), `cancel` (Esc on a running write). No letter closes a box; none moved.
- **TESTING.md:** grep hit only :650 "Any other key while peeking" (peek, not the box). Nothing for the conductor.
- **Captures:** new `docs/shots/m6/fix-box-keeps-on-y-120x40.txt` (5.4 KB) plus one row in
  `m6-contact-sheet-captions.tsv` (required by `contact_sheet_captions_cover_every_m6_capture`). Assembled contact
  sheet not regenerated. Colour unverified.
- **Touches:** 11 boxes (rows above); only caller `boxes_key`; Ctrl+C through `update` (test); `box_is_modal`, sweep
  comments (nextest); walks (read + ran); hint bar / graft-ui (no diff under `crates/graft-ui`, bound.rs comment only);
  journal/undo (Enter logic moved verbatim, Enter tests and walks green); keymap doc untouched (conductor's); MAP.
- **Gates:** fmt, clippy -D warnings, nextest 3054 passed / 1 skipped, doctests: all workspace-wide, green.
