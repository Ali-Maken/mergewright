# confirm-box-swallows-unknown-keys: a key the confirm box does not bind leaves it open, never cancels it in silence

Status: ready
From: `docs/reviews/m6-usability.md` M1 (:65-75), the part that does not depend on P12; review m1 (:137-140) for the box.
Blocked by: none. It holds whether P12 and P15 are approved or refused: whether or not `y` gets a meaning later, a key
the box does not bind must never cancel it without a word.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: list every site that sets `AppState::confirm` and every path that reaches `confirm_key` (key, paste,
mouse, `Ctrl+C`), plus every test, walk and pty script that dismisses a box with a key other than `Esc`.
Needs: none. It calls `graft::update::confirm::confirm_key`, as merged.
Crates: graft.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: "Decided here" 1, and the Journeys row "Push".
- `docs/reviews/m6-usability.md` :65-75, :137-140.
- `docs/adr/0038-m5-spec-decisions.md` §1 (the cursor rests on Cancel in every box).
- `crates/graft/MAP.md` §`update::confirm` (`PendingConfirm`, `ConfirmThen`, `confirm_key`).
- Source, only after the MAP: `crates/graft/src/update/confirm.rs:88-176` and its `tests` module.

## Scope
**In:**
- `confirm_key`'s fall-through arm (`confirm.rs:173`, `_ => next.confirm = None`).
- The rule after the change:
  - `Esc` closes the box without acting, as today.
  - Every key the box does not bind is swallowed: the box stays open, its cursor does not move, and `status` is untouched.
  - `confirm_key` still returns `true`, so the key reaches nothing behind the box.
- The fix is in that arm only. It is the cause: the box treats "unknown" as "cancel".

**Out:**
- `y` acting in the box: `confirm-box-takes-y` (blocked on P12 and P15).
- The `.` menu's pass-through (keymap §2.1, deliberate).
- The preview's own rule, which already swallows.
- The box's drawing and footer.
- Mouse handling: a click is not a key.

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
// graft/src/update/confirm.rs: unchanged signature; only the fall-through arm changes
pub fn confirm_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress,
                   next_id: &mut impl FnMut() -> JobId) -> bool;
//   KeyCode::Esc => next.confirm = None,   // explicit arm (today it is inside `_`)
//   _ => {}                                // swallowed: the box stays, and the reason is commented
```

## Touches (each entry, and how the builder checks it)
- **Every box that `confirm_key` answers.** `grep -rn "confirm = Some\|PendingConfirm {" crates/graft/src` lists them.
  Expect at least: discard (`operate::request_discard`), the undo drift guard, revert-to, resume
  (abort / skip / start over), undo-after-push, restart-conflict (`d` in the resolver), push (`push_box.rs:24`), and
  stash drop (`stash.rs:458-487`).
  - Each site gets one row in `a_key_the_box_does_not_bind_leaves_it_open_in_every_box`, a table test: open the box,
    press `y`, and it is still open.
  - A box that some path other than `confirm_key` answers goes in NOTES, not fixed here.
- **What reaches `confirm_key`, and in what order** (`update/keys.rs` dispatch).
  - Check: `ctrl_c_with_a_box_open_is_unchanged`, which pins today's behaviour whatever it is.
  - Check: a paste while a box is open (the paste path) goes in NOTES, with the line that decides it.
- **Tests that dismissed a box with a key other than `Esc`.**
  - Check: `grep -rn "confirm" crates/graft/src crates/graft/tests` for tests that press a letter after opening a box.
    Each one moves to `Esc`, and NOTES lists them.
- **Walks.**
  - Check: `grep -n "confirm\|box" .scratch/*/pty-walk*.py`, and the `crates/graft/tests/it/*walk*` files. A step
    that closes a box with a non-`Esc` key moves to `Esc`, listed in NOTES.
- **Captures.** The box does not look any different.
  - Check: after the gates, `git status docs/shots` is clean. Any capture that changed is a stop: report it.
- **Other screens that draw the same state.** The box's footer `←→ choose  ⏎ act  esc cancel` (graft-ui `confirm`)
  stays true.
  - Check: `grep -rn "esc cancel" crates/graft-ui/src/confirm*`, no edit.
- **Journal and undo.** None: no `Enter` arm changes.
  - Check: `git diff` touches only the fall-through arm, a new `Esc` arm, and tests.
- **Keymap.** No line exists for confirm boxes today. The conductor adds a row (text below); the builder does not edit
  the doc.
- **TESTING.md.** None known.
  - Check: `grep -n "any key\|closes the box\|other key" TESTING.md`. Hits go in NOTES for the conductor.
- **MAP.** graft §`update::confirm` `confirm_key`: replace "everything else (`Esc` included) closes without acting"
  with the new rule.

## Acceptance (name the verifying test at tick time)
- [ ] `a_key_the_box_does_not_bind_leaves_it_open`: red before the change (on which key), green after.
- [ ] `a_key_the_box_does_not_bind_leaves_it_open_in_every_box`: one row per site the grep found. NOTES gives the row
  count and the grep command.
- [ ] `esc_still_closes_every_box_without_acting`: no `Command` emitted, `confirm.is_none()`.
- [ ] `enter_on_the_acting_choice_still_acts`: the existing test, or a new one if none exists; name it.
- [ ] `ctrl_c_with_a_box_open_is_unchanged`.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-box-keeps-on-y-120x40.txt`.
  - Scene: the push box open, `y` pressed.
  - **What right looks like:** the box is still drawn, titled `Push main → origin/main`, with `[ Cancel ]` under the
    cursor. The status line reads what it read before `y`. The footer reads `←→ choose  ⏎ act  esc cancel`.
- [ ] MAP.md updated (doc, no test).
- [ ] NOTES: the site list, the tests and walk steps moved to `Esc`, and any TESTING.md hits.

## For the conductor (exact text; not the builder's to edit)
Add to `docs/design/02-keymap.md`, after §2.3b, as **§2.3c Confirm box**: `←`/`→` choose (clamped, Cancel first,
ADR-0038 §1) · `Tab` cycles · `Enter` the choice under the cursor · `Esc` cancels · **any other key is swallowed: the
box stays** (usability walk M1).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small. One context.

## NOTES
