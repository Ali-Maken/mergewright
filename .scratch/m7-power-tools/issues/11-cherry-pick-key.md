# cherry-pick-key — `o` takes the cursor commit onto his branch, previewed; it refuses in words what it can't take

Status: done 2026-09-28 (was ready, amended 2026-09-28 after scout `.scratch/m7-power-tools/scouts/11-build.md`: four spec/code disagreements
decided below under "Scout decisions")
Blocked by: 10-branch-picker (merged); 07b (holds `update/operate.rs`), 30k (may hold `update/preview.rs` for its Minor 6) — conductor, 2026-09-28
Crates: graft (the `o` route, the take title, slot 2's dim and label), graft-ui (`ContextMenu::for_row` carries a dim
per history item). **graft-ops: nothing left** — 11a built the refusals and `sets_aside` is already true for CherryPick.
Files (exact; anything else → stop and report):
- graft: `src/update/preview.rs`, `src/update/keys.rs`, `src/update/context_menu.rs`,
  `src/update/context_menu/{slots,verbs}.rs`, `src/update/context_menu/tests.rs`,
  `src/update/context_menu/verbs/tests.rs`, `src/update/tests.rs`, `src/update/operate.rs` (**only** the `✓ took` line,
  and only if `Done` for a CherryPick words it otherwise today; say which in NOTES), the `m7_walk` test file, `MAP.md`.
- graft-ui: `src/context_menu.rs` (+ its tests), `MAP.md`.
- docs: `TESTING.md`, `docs/shots/m7/*` (new captures + every existing capture showing slot 2's label, listed in NOTES).
- **Not edited:** graft-ops `src/operation/history.rs` (held by 30k), graft-ui `src/preview_box.rs`,
  graft-ui `src/graph_screen/hints/catalogue.rs`, graft `src/update/place/history_keys.rs`.
Source: note "Pass 2a" R5 (`o`); Decision 5; field notes :73; KNOWN_GAPS :806, :854; `docs/research/checkout-cherry-pick.md`
§2, §7.
Build with: Opus. A key onto an existing previewed op, a menu dim through one graft-ui parameter; no new shape.
Scout: done (`scouts/11-build.md`).
Needs: branch-picker (both edit graft; `cherry-pick-other-branch` walks through `C … ⇥`). `PickParams::check_commit`
(11a). Doc rows: the keymap diff (§2.1 rare verbs, §2.3 `o`, §2.3b) applied by the conductor first; "Where I'd
differ" 4 (set-aside for cherry-pick) is approved and built (11a).

## Scout decisions (2026-09-28)
1. **Title `take … onto` is composed app-side; `describe_pick` is not edited.** `graft/src/update/preview.rs:997` builds
   `Preview: {describe}`; for a one-commit `CherryPick` it uses `take_title` instead. Both `o` and slot 2's `⏎` open the
   preview through that one line, so both read `take`. `describe_pick` (graft-ops `operation/history.rs:353`) keeps
   `cherry-pick <sha7> "<subj>"` for the journal, `u`'s words and `wait for …` — the git verb, as the menu label
   `Cherry-pick` and the M6 stop banner already use. No sequencing after 30k needed.
2. **Bar stays `y confirm  esc cancel`.** Every preview reads `y confirm` (H4 consistency; keymap §2.3b "one confirm
   key"; keymap row `o` says "previewed, `y`", not a new bar word). The title already says what `y` does.
   `preview_box.rs:55` untouched.
3. **Slot 2 dims through a per-item dim on `for_row`'s history list** — graft-ui already has `MenuItem::disabled` and
   `for_drop`'s `(label, disabled)` pairs; `for_row` takes the same pairs. A dimmed history item is drawn and acts
   exactly as `dim_first` does (`label — reason`, `DIM`, `ContextAction::Unavailable`, never dispatched). `dim_first`
   and `leading` are unchanged (folding slot 1 into the same carrier is a later cleanup, not this ticket).
   **The dim is decided from facts the app holds, no git read on menu open:** HEAD's own row → `already on <b>`;
   2+ parents → a merge. Both through `PickParams::check_commit(commit, on_head, parents)` (pure, 11a), so graft-ops
   decides and the app only words it. **An ancestor of HEAD (not HEAD itself) stays live in the menu**: `⏎` or `o`
   opens the preview, whose plan refuses it in the same words (11a, one `rev-list`). The conductor files a KNOWN_GAPS
   row (unbuilt, not broken): "slot 2 does not dim on an ancestor of HEAD; `⏎`/`o` refuse it in words".
4. **`sets_aside` stays as built: unconditional for CherryPick** (`operation.rs:1331`; note R5 amended 2026-09-27: every
   changed tracked file set aside, untracked never move). The Signatures line claiming "touched paths only" is dropped.
   The preview shows the set-aside line whenever tracked edits exist.

## Decided
- **`o` on a commit row** (graph focus, where `m`/`r` are live; nowhere else) → `take_key` → `Operation::CherryPick(
  PickParams{commits: vec![oid], …})`, built by the **same** builder slot 2 uses → the existing preview, titled
  `Preview: take <sha7> "<subject>" onto <branch>` (detached: `onto HEAD`), bar `y confirm  esc cancel` → `y`.
  `o` is its own fn, **not** a `HistoryKey` variant (place mode reuses `HistoryKey`; `o` stays unbound there).
- **Refusals** (the preview never opens):
  - HEAD's row, a merge: refused app-side before the preview, from `check_commit`, status = `refusal_status(op, &r)`:
    `<sha7> is already on <branch>` / `<sha7> is a merge · taking a merge isn't in Graft yet`.
  - An ancestor of HEAD: the preview's plan refuses (11a); the box closes, same status words (as every refused preview).
  - The uncommitted row or a stash row: `o takes a commit · this row isn't one` (app-side).
  - Unborn HEAD / moved HEAD: `check_pick`'s words, as today.
- **The `.` menu slot 2** keeps its place. Live label `Cherry-pick <sha7> (o)`. Dim label, as slot 1's shape:
  `Cherry-pick (o) — already on <b>` / `Cherry-pick (o) — a merge · taking a merge isn't in Graft yet` (the sha
  dropped: the row names it, as slot 1 drops the branch). **A stash row's menu** (pre-71 order) carries **no** `(o)`
  if it lists Cherry-pick — `o` refuses there, and `(key)` promises a live key (keymap §2.1).
- **Unsaved edits:** set aside and put back, one entry (ADR-0045 §4 + CherryPick, built in 11a). The preview shows the
  set-aside line. A clash lands on the resolver.
- **Lands** (`land-on-result`): cursor on the new commit, `✓ took "<subject>" onto <branch> · u undoes`. A conflict →
  the resolver (M6). The banner's `c`/`s`/`a` are as today for a cherry-pick.

## Read first
- This ticket's "Scout decisions"; `.scratch/m7-power-tools/scouts/11-build.md`.
- Note `docs/design-notes/m7-power-tools.md` R5 (:661–722), "Journey counts" (:837–838).
- `docs/design/02-keymap.md` §2.1 (:160–167), row `o` (:205), §2.3b.
- `crates/graft/MAP.md`: `update::context_menu` (+ `slots`, `verbs`), `update::operate` (`refusal_status`,
  `operated`). `crates/graft-ui/MAP.md`: `context_menu`, `preview_box`. `crates/graft-ops/MAP.md`: `PickParams`.

## Signatures
```rust
// graft-ui (context_menu.rs): history's element becomes for_drop's pair; collection kind unchanged.
// A Some(reason) item draws `label — reason`, DIM, ContextAction::Unavailable (never dispatched), like dim_first.
ContextMenu::for_row(marked, history: /* [(SanitizedText, Option<SanitizedText>)] */, leading, dim_first) -> ContextMenu

// graft (update::context_menu::verbs)
pub struct HistoryVerb { pub label: SanitizedText, pub op: Operation, pub refused: Option<SanitizedText> }
fn pick_op(id: Oid, subject: &str, head: /* HEAD's Tip, as pick_verb takes it today */) -> Operation; // shared by o + slot 2
fn pick_dim_reason(r: &Refusal) -> Option<SanitizedText>; // AlreadyOnBranch → `already on <b>`, PickMerge → merge words, else None

// graft (update::preview)
pub(crate) fn take_key(state: &mut AppState, next_id: /* as merge/rebase keys take it */) -> Vec<Command>;
fn take_title(params: &PickParams, branch: Option<&str>) -> String; // `take <sha7> "<subject>" onto <b|HEAD>`, sanitised parts
```
Exact parameter types the builder copies from the neighbouring code; the shapes above are the decision.

## Acceptance
- [x] **graft-ops:** (11a, 2026-09-27; words at `check_commit` in `operation/history/tests.rs`, preview shut and
  `apply` refused with nothing written in `tests/it/cherry_pick.rs`)
  - `picking_head_is_refused_already_on_branch` (red first) + `picking_head_is_refused_in_the_preview_and_in_apply`;
  - `picking_an_ancestor_is_refused_already_on_branch` +
    `picking_an_ancestor_is_refused_in_the_preview_and_in_apply`;
  - `picking_a_merge_is_refused` (red first) + `picking_a_merge_is_refused_in_the_preview_and_in_apply`;
  - `picking_over_touched_unsaved_edits_sets_them_aside` (approved; `u`/`U` exact);
  - `a_pick_put_back_that_clashes_stops_as_a_clash` (`u` exact).
- [x] **graft-ui:** (both in `graft-ui src/context_menu.rs` tests; the first was red first: failed to compile on the old signature)
  - `a_dimmed_history_item_draws_label_dash_reason_and_is_unavailable` (red first);
  - `for_row_with_no_dims_draws_as_before` (existing context_menu tests unchanged in output).
- [x] **App:** (each by name in `graft src/update/context_menu/tests.rs`, `slot_two_dims_on_head_and_on_a_merge` also in
  `context_menu/verbs/tests.rs`; `o_is_unbound_in_place_mode` in `src/update/tests.rs`; `y_takes_it_and_lands_on_the_new_commit`
  is a helper in `tests/it/m7_walk/picker.rs` run by `cherry_pick_in_view`. **Not red first**: written after the code — see NOTES)
  - `o_opens_the_take_preview` (red first; title exact, bar `y confirm`);
  - `slot_two_enter_and_o_open_the_same_preview` (same op, same title);
  - `o_on_head_is_refused_without_a_preview`, `o_on_a_merge_is_refused_without_a_preview`;
  - `o_on_an_ancestor_is_refused_in_words` (the plan's `Refused`; box closed);
  - `o_on_the_uncommitted_row_is_refused_in_words`, `o_on_a_stash_row_is_refused_in_words`;
  - `o_is_unbound_in_place_mode`;
  - `slot_two_dims_on_head_and_on_a_merge` (labels exact);
  - `slot_two_dim_behaves_as_slot_one_dim` (`⏎` on each: nothing dispatched, same outcome);
  - `slot_two_label_carries_o`, `a_stash_row_menu_carries_no_o`;
  - `y_takes_it_and_lands_on_the_new_commit` (status exact).
- [x] **Walks** (`m7_walk`, `tests/it/m7_walk/picker.rs`):
  - `cherry_pick_in_view` = `motion o y` = 3; **walked as `o y` = 2** (the graph opens on the wanted commit; no motion);
  - `cherry_pick_other_branch` = `C <type> ⇥ motion o y` = 6.

  Each ≤ its `noted_keys`. The commit's change is on HEAD; `u` removes it.
- [x] **Captures** (colour unverified; taken and asserted by `cherry_pick_in_view`, `take_refused_on_a_merge`,
  `menu_slot_two_dim_on_head` in `m7_walk/picker.rs`. The fixture's branch is `main` and the commit `fix the bug`, not
  `feature-a`/`Fix null session`; bar is the widest rung `y confirm  ←→ choose  ⏎ act  esc cancel`):
  - `docs/shots/m7/take-preview-120x40.txt`. **Right:** title `Preview: take c0ffee1 "Fix null session on logout" onto
    feature-a`; footer `y confirm  esc cancel` (or its widest rung); the set-aside line when edits exist.
  - `docs/shots/m7/take-refused-merge-120x40.txt`. **Right:** status line `<sha7> is a merge · taking a merge isn't in
    Graft yet`; no preview box.
  - `docs/shots/m7/menu-slot-two-dim-120x40.txt`. **Right:** on HEAD's row, slot 1 `Checkout — already on feature-a`
    and slot 2 `Cherry-pick (o) — already on feature-a`, both dim, in places 1 and 2.
  - Every existing capture showing slot 2 re-taken (`Cherry-pick <sha7> (o)`); listed in NOTES.
- [x] Hint-bar tests green (workspace nextest), catalogue untouched (`git status`: not modified) (`o` is reached through `.`'s `(o)` label; the note gives it no hint row).
      MAP.md (graft, graft-ui). TESTING.md rows + captions + contact sheet. NOTES answers KNOWN_GAPS :806's question
      with its line (11a: `history.rs:149-160` via `preview/kinds.rs:133`, `apply.rs:508`; the app: this ticket).

## Touches (check each, report how)
- **`.` menu on commit rows** (stash-badge order, conductor 2026-09-25): order unchanged. Check: context_menu tests +
  every capture whose slot-2 label text changes (list each).
- **Every `for_row` caller** (graft-ui signature change): grep `for_row(`; each passes `None` dims except slot 2.
- **The drop menu** (`for_drop`, `MenuItem::disabled`): shares the carrier, unchanged. Check: its tests green.
- **Absorb's dim on the uncommitted row** (`for_working_tree`): unchanged. Check: its tests.
- **Preview title line** (`preview.rs:997`): only one-commit CherryPick changes. Check: every other preview title test
  green; m5/m6 preview captures unchanged.
- **Resolver `o` (take ours)**: modal pane, claims the alphabet; unaffected. Check: resolver take tests green.
- **Place mode** `history_keys`: `o` unbound there. Check: `o_is_unbound_in_place_mode`.
- **The cherry-pick stop, banner and resolver (M6):** unchanged. Check: the m6 cherry-pick tests.
- **Set-aside (ADR-0045):** no change (11a). Check: set-aside tests green. ADR-0045 amendment rows owed (conductor).
- **Journal/undo words:** keep `cherry-pick <sha7> "…"` from `describe_pick` (decision 1). Check: the `u` walk step's
  status as today.
- **Keymap:** §2.1 rare-verbs line and §2.3 `o` (conductor applies first). `verbs.rs` ~:140 comment "cherry-pick owns
  no letter" → corrected.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).

## NOTES (builder, 2026-09-28)
- **Compacted: yes**, once, after code and tests were green; the remainder (place-mode test, TESTING rows, captions,
  ticks) was done after. Nothing left over.
- **Cause-level build**, no workaround.
- **Deviations from Signatures:**
  - `HistoryVerb` unchanged; the dim rides on `Slots::dimmed: Option<(usize, SanitizedText)>` + `Slots::menu_items()`.
    Why: `update/drag.rs` also builds `HistoryVerb` and is outside the Files line.
  - `pick_op` is `pick_params(id, subject, head) -> PickParams` (`verbs.rs`), shared by `o`, slot 2 (`pick_slot`) and the
    stash row's `pick_verb`.
  - `take_key(state, commands, next_id)` pushes into `commands`, like its neighbours in `keys.rs::write_key`.
  - The take title is set in `preview.rs::preview_box` for every preview state (planning, ready, closing), not only at
    the old :997 line, so the title never flips from `cherry-pick` to `take` as the plan lands.
- **Outside the Files line (forced by the spec'd `(o)` label):** `crates/graft/tests/it/m6_stash_put_back.rs` (labels;
  `the_checkout_slot_on_head_s_row_is_drawn_dim_and_only_it` now asserts slot 2 dim too, Copy SHA live),
  `crates/graft/tests/it/context_menu.rs` (caption text). Conductor: approve or revert.
- **Captures regenerated for the label** (`GRAFT_UPDATE_SHOTS=1`): `docs/shots/m5-ui-context-menu-verbs-160x48.txt`,
  `docs/shots/m5-ui-context-menu-verbs-80x24.txt`, `docs/shots/m6/fix-radar-bookkeeping-120x40.txt`,
  `docs/shots/m6/menu-fixed-slots-120x40.txt` (the last two outside `docs/shots/m7/*`).
- **`operate.rs`:** edited. Before, `done_status` worded a CherryPick as the entry description (`cherry-pick <sha7>
  "…"`, no keys). Now `took "<subject>" onto <b> ✓ · u undoes` (with `your edits are back` when set aside).
- **Tests not red first** for the app: written after the code (only the graft-ui one was red first). Each was
  run green; none was seen failing against the old code.
- **`o_is_unbound_in_place_mode`** filters out `Command::EvilProbe`: every update probes the selected commit
  (`update/detail/ops.rs:193-211`), which the blame fixture has not settled. Scout claim "place mode reuses
  `HistoryKey`" was wrong: `place/history_keys.rs` has no `o` arm and place mode does not route `HistoryKey`.
- **KNOWN_GAPS :806 answer:** graft-ops `src/operation/history.rs:149-160` (`check_commit`), called at
  `preview/kinds.rs:133` and `apply.rs:508` (11a); app side: `update/preview.rs::take_operation` (`o`) and
  `update/context_menu/verbs.rs::pick_slot` (slot 2's dim).
- Ancestor-of-HEAD stays live in the menu (scout decision 3); KNOWN_GAPS row owed by the conductor.
