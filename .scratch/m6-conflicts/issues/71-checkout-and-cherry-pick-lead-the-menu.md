# checkout-and-cherry-pick-lead-the-menu: the `.` menu on a commit row opens with Checkout, then Cherry-pick, always

Status: ready
Blocked by: 66 (merged)
From: pick 17 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :26); review N7 (`docs/reviews/m6-usability-final.md`
:193-208). Interim: their own keys and a branch picker are M7, prototyped first (not here).
Build with: (conductor, after the scout; planner suggests Sonnet — ordering and words, every case in one table test)
Scout: `.scratch/m6-conflicts/scouts/checkout-and-cherry-pick-lead-the-menu.md` — brief below.
Needs: none (66 merged: it changes which rows lead with `Explain`).
Crates: graft (`update/context_menu`, `update/context_menu/verbs.rs`, tests, walks), graft-ui (`context_menu.rs`).
Group: **G**. After 67, before E and F2 (F2's `. ⏎ y` walks need slot 1).

## Read first
- `docs/design-notes/m6-stash-put-back.md` "Prior art" (menu row), "Journeys" (`menu-checkout`, `menu-cherry-pick`),
  "New UI" (the menu bullet), landing "⏎ on a disabled menu slot".
- `docs/research/checkout-cherry-pick.md` §2 (cherry-pick is a top-level item in JetBrains, Sublime Merge).
- `docs/design/02-keymap.md` §2.1 (cherry-pick is a rare verb behind `.`; **no new key**), the `.` row.
- `crates/graft/MAP.md` :165 (`AppState::context_menu`), :205-216 (`update::context_menu`, `verbs`, `HistoryVerb`),
  :462-469 (`drop_menu`, disabled items with a reason).
- `crates/graft-ui/MAP.md` `ContextMenu`, `for_row`, `MenuItem::disabled`.
- ADR-0039 (menu contents never depend on width).

## Decided (design note — not open here)
- **Commit rows only.** Slot 1: `Checkout <b>` — the first non-current local branch on the row (today's first
  `Checkout` verb). When there is none: dim `Checkout — already on <b>` if the row carries `HEAD`'s branch, else dim
  `Checkout — no local branch here` (unless the scout finds today's menu offers a remote-branch checkout on such a row:
  then that verb fills slot 1). Slot 2: `Cherry-pick <sha7>`, or dim `Cherry-pick — <why>` in `Operation::check`'s
  refusal words (the commit is `HEAD`, already on `HEAD`'s branch, a merge…).
- Then `Explain` when the row is marked; then today's row actions; then today's other history verbs, further
  `Checkout <b2>` first among them.
- `⏎` (or a click) on a dim slot: the menu stays open; the status shows its reason (the drop menu's rule).
- The uncommitted-changes row, stash rows and anything that is not a commit: menus unchanged.
- Same contents at 80 and 120 columns.

## Scout brief (≤1 page, code not intent)
1. `verbs.rs:111-128` (review N7): how history verbs are built per row; when Checkout and Cherry-pick are absent today
   (HEAD row, remote-only row, merge commit, stash commit); whether a remote-branch checkout verb exists.
2. `graft-ui` `context_menu.rs:139`: how `for_row` places `Explain`, row actions and `History(i)`; how `MenuItem::disabled`
   draws and what `⏎` on it does in the drop menu (`drop_menu`, `update/drag`).
3. Every caller of `ContextMenu::for_row` and every constructor of `HistoryVerb` (grep, list).
4. Every test and walk that selects a `.` menu item by position (`j` counts after `.`, `History(i)` indices, mouse row
   clicks): grep `tests/it` and `crates/graft/src` — each will move.
5. Captures showing a `.` menu: `grep -rln "Cherry-pick\|Checkout\|Explain" docs/shots`.
6. Where `Cherry-pick`'s refusal is decided (`Operation::check` for Pick) and which refusals are possible with no dirty
   tree rule (scout 67 :11: pick is not refused dirty).

## Scope
**In:** the two fixed slots, their disabled forms and reasons, the `⏎`-on-dim rule, the reorder, tests, walks, capture.
**Out:** new keys for checkout/cherry-pick and a branch picker (M7, prototype first); the drop menu; `Explain`'s own
logic; m9 (upstream-less `ⓘ`).

## Signatures
```rust
// graft, update/context_menu/verbs.rs
pub struct HistoryVerb {
    pub label: SanitizedText,
    pub op: Option<graft_ops::operation::Operation>,   // None ⇔ disabled
    pub disabled: Option<SanitizedText>,               // the reason, drawn dim after `—`
}
// graft-ui, context_menu.rs — `leading` history verbs go before `Explain` (2 on a commit row, 0 elsewhere)
impl ContextMenu {
    pub fn for_row(marked: bool, history: &[(SanitizedText, Option<SanitizedText>)], leading: usize) -> Self;
}
```

## The failing tests first
- graft `update`: `checkout_and_cherry_pick_hold_the_first_two_slots` — a table of commit rows: plain; `HEAD`;
  radar-marked; two local branches; a branch and a second branch moved onto the same commit (N7's case); a merge
  commit; remote-only. Slot 1 and 2 labels (or dim reasons) asserted per row.
- `explain_comes_third_on_a_marked_row`.
- `enter_on_a_dim_slot_keeps_the_menu_and_says_why`.
- `the_uncommitted_row_menu_is_unchanged`; `a_stash_row_menu_is_unchanged`.
- graft-ui: `for_row_places_leading_verbs_before_explain`; `menu_contents_do_not_depend_on_width` (80 vs 120).

## Touches (each entry, and how the builder checks it)
- **`for_row` callers** (scout Q3): each passes `leading`. Check: build; grep.
- **`HistoryVerb` constructors**: `op` becomes `Option`. Check: build; no `unwrap` on `op` added (grep the diff).
- **Walks and tests selecting by position** (scout Q4): each updated to the new index, listed in NOTES with old → new.
  A walk that becomes shorter is fine; one that becomes longer is a finding in NOTES.
- **Mouse**: a click on a dim slot behaves as `⏎` on it. Check: a mouse test.
- **Drop menu** (`drop_menu`): unchanged. Check: its tests green.
- **Hint bars**: the menu's bar unchanged. Check: `every_key_the_bar_draws_is_answered` green.
- **Radar (ticket 66)**: fewer marked rows since 66, so `Explain` leads less. Check: the marked-row test uses a real
  mark from a fixture.
- **Captures** (scout Q5): each regenerated by its test or `git diff --exit-code`, listed in NOTES.
- **TESTING.md** rows naming menu positions (`grep -n "item [0-9]\|j×" TESTING.md`): listed in NOTES for H.
- **Keymap doc** §2.1 rare-verbs line: owed by the conductor; the builder does not edit `docs/design/`.
- **Journal/undo:** none (order only). **MAP.md:** graft `HistoryVerb`, verbs order; graft-ui `for_row`.

## Acceptance (name the verifying test at tick time)
- [ ] Every test above, red then green (width test may be green-first; say so).
- [ ] Walk, real repo (`tests/it/m6_stash_put_back.rs`): `menu_checkout_is_three_keys` — cursor on `fix`'s row, `. ⏎ y`:
      on `fix`; keys ≤ **3**.
- [ ] Walk: `menu_cherry_pick_is_four_keys` — cursor on a commit of `fix`, `main` checked out, `. j ⏎ y`: the commit's
      patch on `main`; keys ≤ **4**. Repeat with `topic` also on that commit (N7's case): the same 4 keys.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/menu-fixed-slots-120x40.txt`, written by the walk.
  - **What right looks like:** frame 1, `.` on `fix`'s row: item 1 `Checkout fix`, item 2 `Cherry-pick <sha7>`, then
    (row marked) `Explain`. Frame 2, `.` on `main`'s tip (checked out): item 1 `Checkout — already on main` (dim: not
    provable in a capture, rule 5), item 2 `Cherry-pick — <reason>`.
- [ ] NOTES: position changes in walks and captures; scout answers.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small–medium (~40k): one struct field, one argument, a table test, two walks, one capture, index updates.
