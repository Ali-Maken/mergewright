# checkout-and-cherry-pick-lead-the-menu: the `.` menu on a commit row opens with Checkout, then Cherry-pick, always

Status: done
Blocked by: 66 (merged), 67 (runs after it; no shared file, see Crates)
From: pick 17 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :26); review N7 (`docs/reviews/m6-usability-final.md`
:193-208). Interim: their own keys and a branch picker are M7, prototyped first (not here).
Build with: **Sonnet** — ordering and one dim label, no refusal logic, every case in one table test.
Scout: `.scratch/m6-conflicts/scouts/checkout-and-cherry-pick-lead-the-menu.md` (done).
Needs: none.
Crates: graft (`update/context_menu.rs`, `update/context_menu/verbs.rs`, tests, walks), graft-ui (`context_menu.rs`).
**Not** graft-ops, `update/preview.rs`, `update/operate.rs` (ticket 67's), nor `update/drag.rs`.
Group: **G**. After 67, before E and F2 (F2's `. ⏎ y` walks need slot 1).

## Amended 2026-09-25 (planner, after scout 71)
- **Mismatch 1, Cherry-pick reasons.** `check_pick` refuses only no-commits and HEAD-moved (`graft-ops`
  `history.rs:466-471`), and the menu pushes `Cherry-pick <sha7>` on every row today (`verbs.rs:79-86`). Decided: **no new
  refusal logic.** Slot 2 is always `Cherry-pick <sha7>`, enabled, as today. Why: new refusals (the commit is `HEAD`,
  already on `HEAD`'s branch, a merge) are a graft-ops rule with its own words and tests, and M7 redesigns cherry-pick
  anyway (own key, prototyped). This supersedes the design note's "or `Cherry-pick — <why>`, dim". Conductor: a
  KNOWN_GAPS question row — "cherry-pick offered on a commit already on `HEAD`; git's own words when it fails" — for M7.
- **Mismatch 2, a row with no local branch.** Today's `Checkout <sha7>, detached` (`verbs.rs:128-135`) **stands** and
  fills slot 1. Why: it is a working action today; a dim label would take it away for no gain. The only dim slot is
  the `HEAD` row whose sole local branch is the current one (today: no checkout at all).
- **Mismatch 3, `⏎`/click on a dim item.** Already built (`update/context_menu.rs:129-135`, `mouse.rs:249-253`). No new
  code; a test pin below proves the dim slot reaches it.
- **Mismatch 4, `HistoryVerb.op: Option`.** Avoided. `HistoryVerb` is unchanged, so the drop menu (`drag.rs:343-348`) is
  untouched. The dim slot is not a verb: `for_row` takes it as its own argument and draws a `MenuItem` with `disabled`,
  which dispatches nothing.

## Read first
- `.scratch/m6-conflicts/scouts/checkout-and-cherry-pick-lead-the-menu.md` (the whole page).
- `docs/design-notes/m6-stash-put-back.md` "Journeys" (`menu-checkout`, `menu-cherry-pick`), "New UI" (menu bullet;
  Cherry-pick's dim form superseded above).
- `docs/research/checkout-cherry-pick.md` §2.
- `docs/design/02-keymap.md` §2.1 (cherry-pick is a rare verb behind `.`; **no new key**), the `.` row.
- `crates/graft/MAP.md` :165 (`AppState::context_menu`), :205-216 (`update::context_menu`, `verbs`, `HistoryVerb`).
- `crates/graft-ui/MAP.md` `ContextMenu`, `for_row`, `MenuItem::disabled`.
- ADR-0039 (menu contents never depend on width).

## Decided
- **Commit rows only** (those `history_verbs` fills; it returns empty with no head tip, `verbs.rs:67`: unchanged).
- **Slot 1:** the first `checkout_verbs` item — `Checkout <b>` (first non-current local branch) or
  `Checkout <sha7>, detached`. When `checkout_verbs` is empty (the `HEAD` row, current branch only): dim
  `Checkout — already on <b>`.
- **Slot 2:** `Cherry-pick <sha7>`.
- Then `Explain` when marked; then Copy SHA, Compare, Open details; then the rest of today's history verbs in today's
  order (further `Checkout …` items first among them); then `always_items`.
- Uncommitted-changes row, stash rows, the drop menu: unchanged. Same contents at 80 and 120 columns.

## Scope
**In:** reorder in `history_verbs`; the dim slot; `for_row`'s two new arguments; tests; walks; capture.
**Out:** any refusal logic (graft-ops); new keys or a picker (M7); the drop menu; `Explain`'s logic; m9.

## Signatures
```rust
// graft-ui, context_menu.rs — history labels stay `Vec<SanitizedText>` indexed by `History(i)`.
// Order: [dim_first] + history[..leading] + [Explain if marked] + row actions + history[leading..] + always items.
impl ContextMenu {
    pub fn for_row(marked: bool, history: Vec<SanitizedText>, leading: usize,
                   dim_first: Option<(SanitizedText, SanitizedText)>) -> Self;   // (label, reason), drawn disabled
}
// graft, verbs.rs — `HistoryVerb { label, op: Operation }` unchanged. The menu's slots:
pub(crate) struct Slots { pub verbs: Vec<HistoryVerb>, pub leading: usize,
                          pub dim_first: Option<(SanitizedText, SanitizedText)> }
pub(crate) fn history_slots(/* history_verbs' arguments, unchanged */) -> Slots;   // leading: 2, or 1 with dim_first
```

## The failing tests first
- graft `update`: `checkout_and_cherry_pick_hold_the_first_two_slots` — table of commit rows: plain branch tip; `HEAD`
  (current branch only → dim); radar-marked; two local branches; a second branch moved onto the same commit (N7); a
  merge commit; remote-only / no local branch (`Checkout <sha7>, detached`). Slot 1 and 2 asserted per row.
- `explain_comes_after_the_slots_on_a_marked_row`.
- `the_uncommitted_row_menu_is_unchanged`; `a_stash_row_menu_is_unchanged`.
- graft-ui: `for_row_places_dim_first_and_leading_before_explain`; `menu_contents_do_not_depend_on_width` (80 vs 120).

## Touches (each entry, and how the builder checks it)
- **`for_row` callers**: `update/context_menu.rs:72` and graft-ui tests `context_menu.rs:356-507` (scout). Check: build.
- **`HistoryVerb`**: unchanged; `drag.rs` untouched. Check: `git diff --stat` shows no `drag.rs`.
- **`History(i)` indices**: the dim item has none. Check: a test that `⏎` on each enabled slot runs its own verb.
- **Index maps in tests**: `tests/it/context_menu.rs:443-452`. Check: updated, old → new in NOTES.
- **Label-picked walks** (`Step::MenuPick`, `tests/it/m5_walk/pick_revert_reset.rs:41,239,295`): unaffected. Check:
  green.
- **Drop menu**: unchanged. Check: its tests green.
- **Hint bars**: unchanged. Check: `every_key_the_bar_draws_is_answered` green.
- **Radar (66)**: `Explain` placement only. Check: the marked-row test uses a fixture mark.
- **Captures** (scout): `docs/shots/m5-ui-context-menu-verbs-{80x24,160x48}.txt`, `m6/fix-radar-bookkeeping-120x40.txt`,
  `m2-context-menu.txt`, `m2-07-foxtrot-merge-dot-*`, `m2-explain-panel.txt`, and the contact sheets holding them —
  each regenerated by its test or `git diff --exit-code`, listed in NOTES.
- **TESTING.md** rows naming menu positions: `grep -n "item [0-9]\|j×" TESTING.md`, listed in NOTES for the acceptance
  ticket.
- **Keymap doc** §2.1 rare-verbs line: owed by the conductor.
- **Journal/undo:** none. **MAP.md:** graft `history_slots`, order; graft-ui `for_row`.

## Acceptance (name the verifying test at tick time)
- [x] Every test above, red then green. Red = did not compile (new `for_row` arity, no `history_slots`) or failed on
      order: `verbs::tests::checkout_and_cherry_pick_hold_the_first_two_slots`,
      `update::context_menu::tests::explain_comes_after_the_slots_on_a_marked_row`,
      `…::the_uncommitted_row_menu_is_unchanged`, `…::a_stash_row_menu_is_unchanged`,
      graft-ui `context_menu::tests::for_row_places_dim_first_and_leading_before_explain`,
      `…::menu_contents_do_not_depend_on_width` (green-first in substance: red only by arity).
- [x] Pin, green-first as expected: `update::context_menu::tests::enter_on_the_dim_checkout_slot_keeps_the_menu_and_says_why`;
      by click `it::m6_stash_put_back::a_click_on_the_dim_checkout_slot_keeps_the_menu_and_says_why`. Plus
      `…::enter_on_each_enabled_slot_runs_its_own_verb` (History indices).
- [x] Pin: no `.`-then-`j`-count selection exists. Greps: `grep -rnE -A4 "Char\('\.'\)" crates/graft/tests/it crates/graft/src
      | grep -E "Char\('j'\)|Down|move_cursor"` → only this ticket's own walk; `grep -rnE "History\([0-9]+\)"
      crates/graft/tests/it crates/graft/src` outside `context_menu/tests.rs` → none. `tests/it/context_menu.rs:443-452`
      computes the index from the label, so no old → new map needed. `TESTING.md` `grep -n "item [0-9]\|j×"` → no menu
      rows (one unrelated hit, :432).
- [x] Walk `it::m6_stash_put_back::menu_checkout_is_three_keys` (3 keys, HEAD on `fix`).
- [x] Walk `it::m6_stash_put_back::menu_cherry_pick_is_four_keys` (4 keys, both without and with `topic` on the commit;
      `fix.txt` on `main`, `main~1` = old main).
- [x] Capture `docs/shots/m6/menu-fixed-slots-120x40.txt` by `it::m6_stash_put_back::capture_menu_fixed_slots`. Colour
      and dim **unverified** (rule 5). Frame 1 also lists `Explain` third: fix's row carries the radar's ⓘ.
- [x] MAP.md: graft (`history_slots`, `Slots`, order), graft-ui (`for_row`, `Unavailable`). Doc, no test.

## NOTES (builder)
- Scout used: `.scratch/m6-conflicts/scouts/checkout-and-cherry-pick-lead-the-menu.md`.
- **New public item:** `graft_ui::context_menu::ContextAction::Unavailable` — the dim slot's action. The spec said the
  dim item "dispatches nothing" but a `MenuItem` needs an action; a variant is honest where a fake `History` index is
  not. Dispatch never reaches it (disabled); `apply_context_action` maps it to no command + `debug_assert`.
- **Stash rows (question for conductor):** a stash row goes through `open_for_history_row` too and gets history verbs.
  To keep "stash rows: unchanged" literally, `history_verbs` keeps the pre-71 order on a row carrying a stash badge
  (`verbs.rs` `is_stash_row`), `leading` 0. If stash rows should get the slots too, delete that branch.
- Captures regenerated: `m5-ui-context-menu-verbs-{80x24,160x48}.txt` (order + header claim), `m6/fix-radar-bookkeeping-120x40.txt`
  (frame 3 now leads with dim Checkout), new `m6/menu-fixed-slots-120x40.txt`. m2 captures unchanged (`git diff`).
  Captions updated: m5 rows 44-45, m6 ticket-66 row (its "first item is `Copy SHA`" became false) + new ticket-71 row.
  Contact sheets rebuilt: m5 (2 parts), m6 (3 parts), all ≤ 180 000 B.
- Existing tests re-ordered: `verbs::tests::menu_verbs_build_expected_operations`,
  `update::context_menu::tests::a_branch_row_lists_its_history_verbs_between_its_actions_and_the_dropped_keys`.
- Keymap §2.1 rare-verbs line: owed by the conductor. KNOWN_GAPS row (cherry-pick on a commit already on HEAD) owed by
  the conductor.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small–medium (~35k): one reorder, two arguments, a table test, two walks, one capture, index updates.
