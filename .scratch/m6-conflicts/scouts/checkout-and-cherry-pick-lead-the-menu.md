# Scout 71: checkout-and-cherry-pick-lead-the-menu

## What exists and what it holds
- `HistoryVerb { label, op: Operation }`: `crates/graft/src/update/context_menu/verbs.rs:41-46`. It has no disabled field. `op` is not an `Option`.
- Order in `history_verbs`: verbs.rs:55-96. It returns empty if there is no head tip (:67). Then it goes checkout (:72), merge/rebase (:77), `Cherry-pick <sha7>` pushed **always** (:79-86), revert, reset, delete, fix.
- `checkout_verbs` (:101-137):
  - one `Checkout <b>` per non-current local branch;
  - the current branch is skipped;
  - a row with **no local branch** gets `Checkout <sha7>, detached` (:128-135).
  - There is no remote-branch checkout verb.
  - A `HEAD` row with only the current branch gets no checkout at all.
- `OpenMenu { row, menu, verbs }` is built by `open_for_history_row`: `crates/graft/src/update/context_menu.rs:49-75`. `marked` comes from `radar::findings_for` (:63-64).
- `for_row(marked, Vec<SanitizedText>)`: `crates/graft-ui/src/context_menu.rs:136-160`. Order: Explain (if marked), Copy SHA, Compare, Open details, `History(i)` items with `disabled: None` (:147-153), then `always_items`.

## Disabled items, `⏎` and click (already built)
- `MenuItem::disabled` draws dim with ` — ` (`REASON_SEPARATOR`, graft-ui context_menu.rs:115).
- `⏎` on a disabled item: `selected_refusal`, which sets a sticky status and keeps the menu open (`update/context_menu.rs:129-135`).
- A click on a disabled item does the same (`update/mouse.rs:249-253`). **The ticket's `⏎`-on-dim rule already exists.** Only `for_row` hardcodes `disabled: None`.

## Callers and constructors
- `for_row`: `update/context_menu.rs:72`, plus graft-ui tests at context_menu.rs:356, 382, 397, 418, 452, 453, 491, 498, 507.
- `HistoryVerb` is built by `verb()` in verbs.rs:380 and **`drag.rs:343-348`** (the drop menu). Making `op` an `Option` touches the drop menu too, which the ticket puts out of scope.

## Walks that select by position
- The m5 walks select by label: `Step::MenuPick("Cherry-pick")` / `("Checkout …")` at `tests/it/m5_walk/pick_revert_reset.rs:41,239,295`.
- `tests/it/context_menu.rs:443-452` maps by index.
- The `j` loops at context_menu.rs:47 and gestures.rs:24,63 move rows, not menu items.
- I found no `j`-count-after-`.` walk. I only did a light grep, so I could not confirm there are none.

## Captures showing the `.` menu (they contain "Copy SHA")
- `docs/shots/m5-ui-context-menu-verbs-{80x24,160x48}.txt`
- `m5-contact-sheet-2.txt`
- `m6/fix-radar-bookkeeping-120x40.txt`
- `m6/m6-contact-sheet-3.txt`
- Older menus: `m2-context-menu.txt`, `m2-07-foxtrot-merge-dot-*`, `m2-explain-panel.txt`.

## Mismatches between the spec and the code
1. **Remote-only row.** The spec's fallback dim text is `no local branch here`, but today that row offers `Checkout <sha7>, detached` (verbs.rs:128). The ticket's own exception clause applies to that verb in spirit, though it names "remote-branch".
2. **Cherry-pick refusal.** `check_pick` refuses only `NoCommits` and `head_is` (`graft-ops/src/operation/history.rs:466-471`). Nothing refuses "the commit is HEAD", "already on HEAD's branch" or "a merge". The dim reasons the spec lists do not exist in `Operation::check`.
3. The spec says "⏎ on dim: new". It is already implemented (see above).
4. The spec's `HistoryVerb.op: Option` change also reaches `drag.rs:343`.

## Overlap with the preview-refusal builder
- Menu dispatch goes through `update/preview` (the recent commits there are 11, 23, 24).
- The menu code itself does not decide refusals. Slot-2 reasons would need new wording from `graft-ops` or `preview`, which is a possible collision.

## Model
Sonnet is enough for the ordering and table test, provided the conductor rules on mismatch 2 first. If new refusal logic is wanted, use Opus.
