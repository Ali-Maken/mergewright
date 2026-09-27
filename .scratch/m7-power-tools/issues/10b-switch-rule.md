# switch-rule — one rule for every switch: instant on a clean tree, preview (`y`) with unsaved edits or a detached target

Status: blocked — needs 4 unlisted test files (NOTES "Blocked"); everything else done
Blocked by: 03-land-on-result (the landing on the new HEAD; 03 also holds graft-ops `sets_aside`)
Crates: graft-ops (`CheckoutParams`, `CheckoutTarget`, `BranchCreateParams`, `check`/`plan`/`touches`/`sets_aside`,
`preview::plan` for a switching branch-create); graft (`update/switch.rs` new; `context_menu` Checkout route;
`name_prompt` `⇥`; `operate::operated`'s refused arm; `clash` label).
Source: field notes :79 (approved 2026-09-26: `.` Checkout and `b ⇥` switch instantly on a clean tree, like `C`;
preview only with unsaved edits); note R5 "Switch rule"; keymap §2.1 (Checkout line), §2.3b `y`; ADR-0045 §4, §8;
scout 10 (`name_prompt.rs:311-320`, `history.rs:507-514`, `:435-440`, `:178-183`, `context_menu.rs:307-329`,
`verbs.rs:73-112`, `clash.rs:240`).
Build with: Opus. One rule reaching three entry points and the set-aside path; the knock-on list is long.
Scout: scout 10 covers the routes. **Supplementary scout** (≤ ½ page) before building, questions below.
Needs: `land_on_result` (03). Exposes `switch_to` and `CheckoutTarget::Track` for `10-branch-picker`.

## Decided
- **The rule** (one function, `switch_to`; every switch goes through it — picker `⏎`, `.` Checkout, `b ⇥`):
  1. Detached target → preview, then `y` (today's path).
  2. The app's last status shows changed tracked files → preview with the set-aside line (ADR-0045 §4, §8), then `y`.
     Launched with `OnEdits::SetAside`.
  3. Otherwise → launch at once with `OnEdits::Refuse`. No preview.
  - Untracked files alone are not "unsaved edits" (ADR-0045 §4 leaves them; git refuses if one is in the way, and
    that refusal's words stand).
- **A stale "clean" never sets aside unseen.** The instant path carries `OnEdits::Refuse`; `check` refuses
  `Refusal::DirtyTree { count }` if the facts show changed tracked files. The app does not show those words: it opens
  the preview for the same target (rule 2). He lands on the preview; one `y` more, as with edits.
- **`b ⇥` over edits:** today refused (`history.rs:510`). Now: the prompt closes and the preview opens (`opened_by`
  `"b"`). `Esc` on it: back to the graph, nothing written.
  - `at` == HEAD's commit: git's `switch -c` leaves index and files untouched, so **nothing is set aside** (the
    staged split is kept). The preview's files line: `your edits stay as they are`. `sets_aside` → `None`.
  - `at` elsewhere: set aside and put back (ADR-0045 §4), the preview's set-aside line and forecast (§8, the target
    is `at`).
  - `check` never refuses `DirtyTree` for a switching create with `OnEdits::SetAside`; with `Refuse` it refuses only
    when `at` ≠ HEAD.
- **`.` Checkout on a branch row, clean tree:** instant (`. ⏎`, was `. ⏎ y`). Slot 1's label and dim are unchanged.
- **Remote-only target (for the picker):** `CheckoutTarget::Track { remote, local }`. Plan
  `switch --create <local> --track <refs/remotes/…>` (explicit `--track`, so `branch.autoSetupMerge` cannot change it;
  the builder measures the exact form, git 2.54, and records it in MAP.md). `check`: `RefExists` when `local` exists;
  `Moved` when `remote` moved since the list; `NoSuchRef` when it's gone. `touches` names `refs/heads/<local>` (so
  `u` deletes it) plus `refs/stash` when set aside.
  - `u` leaves `branch.<local>.remote/merge` in config. Harmless to git; **file it in NOTES as a gap** (the conductor
    rows it in KNOWN_GAPS). Redo re-creates the ref and tracking comes back.
- **Words on landing** (status line, the cursor lands via `land_on_result`):
  - `on <b> ✓ · u goes back`
  - `on <b> (new, tracks <origin/b>) ✓ · u goes back`
  - with a set-aside: `on <b> ✓ · your edits are back · u goes back`
  - (Word order amended 2026-09-27, conductor ruling: every `Done` draws `<what> ✓ · …`,
    `update/state.rs:1021-1041`; consistency, ux-principles.) — the existing `clash::edits_back` clause,
    **not** the note's `your N edited files put back`: the same words the rebase uses (UX consistency; one wording).
  - A detached checkout keeps today's words. A put-back clash lands on the resolver (ADR-0045), label `switch` for
    Checkout and for a switching create.

## Read first
- This spec. Note "Pass 2a" R5 "Switch rule" only. Keymap §2.1 (the Checkout sentence), §2.3b (`y` row).
- `docs/adr/0045-…` §4, §8 (the forecast), §6.
- `crates/graft-ops/MAP.md` `operation` (`CheckoutParams`, `CheckoutTarget`, `BranchCreateParams`, `SetAside`,
  `sets_aside`, `Refusal::DirtyTree`, `plan` history kinds, `touches`), `preview` (`plan`, `NotPreviewable`).
- `crates/graft/MAP.md`: `update::preview` (`PreviewSubject`, `open_preview`, `preview_refusal`),
  `update::context_menu` (`apply_context_action`), `update::name_prompt`, `update::clash` (`edits_back`),
  `update::operate` (`operated`).

## Supplementary scout (≤ ½ page, before building)
1. Does `preview::plan` accept `BranchCreate { checkout: true }` today, or refuse `NotPreviewable`? Line.
2. `operate::operated`'s refused arm: where a `Refusal` from a launched op becomes a status line. Line.
3. Where the Checkout `Done` status words are formed today, and every test/capture asserting them.
4. Every construction site of `CheckoutParams` and `BranchCreateParams` (scout 10: `verbs.rs:97,106`,
   `name_prompt.rs:311-320`); is either serialized into the journal? (If so, `on_edits` needs a serde default.)
5. Every graft-ops test asserting `DirtyTree` for a branch-create or a checkout.

## Signatures
```rust
// graft-ops
pub enum OnEdits { Refuse, SetAside }
pub struct CheckoutParams { pub to: CheckoutTarget, pub head_before: HeadRef, pub on_edits: OnEdits }
pub enum CheckoutTarget { Branch(String), Detached(Oid), Track { remote: Tip, local: String } }
pub struct BranchCreateParams { pub name: String, pub at: Oid, pub checkout: bool, pub on_edits: OnEdits }

// graft — update/switch.rs
pub enum SwitchTarget {
    Branch(String),
    Track { remote: Tip, local: String },
    Detached(Oid),
    NewBranch { name: String, at: Oid },
}
/// The one switch rule. `opened_by` is the key the preview names if one opens.
pub fn switch_to(state: &mut AppState, target: SwitchTarget, opened_by: &'static str, next_id: &mut dyn FnMut() -> u64)
    -> Vec<Command>;
/// operate's refused arm: an instant switch refused `DirtyTree` opens the preview instead; `None` = not ours.
pub(crate) fn instant_refused(state: &mut AppState, op: &Operation, refusal: &Refusal,
    next_id: &mut dyn FnMut() -> u64) -> Option<Vec<Command>>;
```
(`next_id`: whatever id source `open_preview` takes today — match it; the scout names it.)

## Acceptance
- [x] **graft-ops** (`crates/graft-ops/src/operation/history/tests.rs` unit, `tests/it/switch_rule.rs` against git):
  - `checkout_refuse_refuses_a_dirty_tree` (red first: `left: Ok(()) right: Err(DirtyTree { count: 1 })`);
  - `checkout_set_aside_is_unchanged` → the m6 round trips, green unchanged: `tests/it/set_aside.rs::checkout_round_trips_{an_rm_cached, index_only_content, a_staged_mode_only_change, staged_unstaged_untracked, a_staged_new_file, a_deletion, a_rename}` (their checkouts now carry `OnEdits::SetAside`);
  - `switch_rule::switching_create_at_head_over_edits_sets_nothing_aside` (red first: worktree `Updated { paths: [], .. }` vs `EditsStay`; unit flip below);
  - `switch_rule::switching_create_elsewhere_over_edits_sets_them_aside_and_u_restores`;
  - `history::tests::switching_create_elsewhere_refuse_refuses_dirty`;
  - `switch_rule::switching_create_previews_with_the_put_back_forecast`;
  - `switch_rule::track_creates_the_local_branch_tracking_the_remote` (config `remote`/`merge` and `@{upstream}` read back, `branch.autoSetupMerge=false`);
  - `history::tests::track_refuses_when_the_local_name_exists`; `history::tests::track_refuses_when_the_remote_moved` (+ `NoSuchRef` when gone);
  - `switch_rule::track_undo_deletes_the_branch_and_puts_head_back`.
  - also: `history::tests::switching_create_at_head_sets_nothing_aside_and_never_refuses`, `track_plans_an_explicit_tracking_create_and_names_its_branch`, `track_refuse_refuses_a_dirty_tree_and_set_aside_touches_the_stash`.
- [x] **App** (`crates/graft/src/update/switch/tests.rs` unless named):
  - `dot_checkout_on_a_clean_tree_switches_at_once` (red first, rule forced to always-preview: panicked at `switch/tests.rs:160` "no preview");
  - `dot_checkout_with_edits_opens_the_set_aside_preview`;
  - `dot_checkout_detached_still_previews`;
  - `b_tab_with_edits_opens_the_preview` (red first, rule without the edits branch: `nothing ran: [Operate(.., BranchCreate { .., on_edits: Refuse })]`);
  - `b_tab_on_a_clean_tree_is_unchanged`;
  - `an_instant_switch_refused_for_edits_lands_on_the_preview` (+ `an_instant_switch_refused_otherwise_keeps_the_refusal`);
  - `switch_words_on_landing` (the three lines, plus detached = the entry's description);
  - `clash::tests::a_switching_create_clash_is_labelled_switch` (red first with the old label: `… u undoes the rebase too`).
  - also `dot_checkout_over_untracked_files_only_switches_at_once`.
- [ ] **Walks:** m5 `checkout_branch_via_menu` (`pick_revert_reset.rs`) now `. ⏎`, no `y`, HEAD on the target, `u` back — **done, green** (`m5_walk::every_scenario::checkout_branch_via_menu`). `checkout_detached_via_menu` unchanged, green. New `m7_walk::switch::{switch_clean_via_menu_is_two_keys, switch_with_edits_via_menu, new_branch_with_edits, new_branch_at_head_with_edits_previews_edits_stay}` — green, edits back and `u` in each. **Not done:** `journey_menu_checkout` — its scenario lives in `m5_walk/stash_journeys.rs:859` (not `every_scenario.rs`, which only names it), unlisted → blocked.
- [x] **Captures** (colour unverified): `docs/shots/m7/switch-menu-clean-landed-120x40.txt` (`switch_clean_via_menu_is_two_keys`): no box, cursor on `feature-b` (asserted from state; the row draws `◆ ▏`, there is no `▸` glyph on a graph row — the spec's `▸` is wrong), `on feature-b ✓ · u goes back`. `docs/shots/m7/new-branch-with-edits-preview-120x40.txt` (`new_branch_at_head_with_edits_previews_edits_stay`): `Preview: branch feature-c at 37926a7`, `your edits stay as they are`, `y confirm` on the edge. Re-taken: `docs/shots/m7/30b-staging-not-kept-{80x24,187x50}.txt` (`checkout fix ✓` → `on fix ✓`, header and line). Caption rows added for the two new ones (the tsv had no 30b rows). m5/m6 captures: `m6/fix-undo-drift-way-back-*` is drawn by `undo_over_drift.rs` (blocked) — re-take owed once unblocked.
- [x] MAP.md (graft-ops: `OnEdits`, `CheckoutTarget::Track` + measured git form, `BranchCreateParams.on_edits`, `sets_aside`, `WorktreeEffect::EditsStay`; graft: `update::switch` section, context menu, name prompt). TESTING.md: "M7: one switch rule" table; 30b row's words.

## Touches (check each, report how)
- **`.` Checkout's callers:** `verbs.rs:73-112` (built), `context_menu.rs:307-329` (`preview_history_verb` → now
  `switch_to` for Checkout only; Cherry-pick and the rest unchanged). Check: `the_menu_runs_what_the_dropped_keys_run`,
  `every_verb_the_menu_advertises_is_live_while_it_is_open` green.
- **`b ⇥`'s refusal tests and captures:** `m5_walk/refs_and_names.rs:72`, `tests/it/name_and_drag.rs:90`, graft-ops
  dirty-tree tests (scout Q5), `docs/shots/m5-ui-name-prompt*.txt`. Flipped tests named in NOTES, before/after.
- **The preview for a switch:** `preview::plan` for a switching create (scout Q1); ADR-0045 §8 forecast for its
  target; `PreviewSubject::Operation { opened_by }` for `"b"`. Check: the m6 set-aside preview tests green.
- **Keymap rows:** §2.1 Checkout sentence and §2.3b `y` already say this (amended 2026-09-26). Check: read both
  against the build; a mismatch is reported to the conductor, not edited. The name prompt's hint bar is unchanged.
- **Journal/undo:** Checkout and BranchCreate entries as today; set-aside entry per ADR-0045 §6. Check: `u` in every
  walk above. **ADR-0045 §4's kinds** gain the switching create — amendment owed (the conductor).
- **Other `CheckoutParams`/`BranchCreateParams` builders:** scout Q4; each passes an explicit `OnEdits`.
- **Resolver/clash** label (`clash.rs:240`). Check: the clash test above.
- **Sidebar `Enter` checkout (§2.3):** no code exists (scout 10). Not built here.

## Proposed change (not decided; the conductor takes it to the user)
- `b ⇥` at HEAD over edits: nothing happens to his edits, so the preview shows nothing he must judge. Proposal: instant
  there too (`b <name> ⇥ ⏎`, 3 not 4). This spec follows the approved rule (preview) until he says otherwise.

## Rules / Gates
Workspace-wide gates. Throwaway repos only.

## Size
Large (~65k). Split if needed: **-a** graft-ops (OnEdits, Track, create set-aside, preview plan); **-b** graft (the
rule, routes, words, walks, captures).

## NOTES (builder, 2026-09-27)
- **Compacted once mid-ticket.** Remaining after this report: only the blocked list below.
- Scouts used: `scouts/10.md`, `scouts/10b.md`.
- **Conductor ruling recorded:** landing words `on <b> ✓ · u goes back` (tick after `what`, as every `Done`); acceptance amended above.
- **Blocked — needs these unlisted files** (each fails only because a clean-tree `. ⏎` no longer opens a preview; the fix is semantic, so not taken under the literal/arm grant):
  1. `crates/graft/src/update/context_menu/tests.rs` — `every_history_verb_the_menu_lists_opens_a_preview` (:560) and `enter_on_each_enabled_slot_runs_its_own_verb` (:830): Checkout now launches on the clean fixture. Fix: expect `Command::Operate(Checkout{on_edits: Refuse})` for the Checkout verb, a preview for the rest (or give the fixture an edit).
  2. `crates/graft/tests/it/undo_over_drift.rs:57-64` `checkout_through_menu`: drop `planned` + `y` (`. ⏎` then `settle_write`). Fails 8 tests there and `box_owns_the_bar::capture_fix_box_owns_the_bar` (uses the same helper). Re-take `docs/shots/m6/fix-undo-drift-way-back-*` and `box-owns-the-bar` captures after, if they differ.
  3. `crates/graft/tests/it/m6_stash_put_back.rs:133-160` `menu_checkout_is_three_keys`: drop `planned` + `y`; now 2 keys (note's `menu-checkout` row says 3 — an upper bound, still holds; the row could drop to 2).
  4. `crates/graft/tests/it/m5_walk/stash_journeys.rs:859-895` `menu_checkout`: drop `WaitFor(PreviewReady)` + `key(y)`; doc `. ⏎`.
- Literal-only `on_edits` additions outside the Files line: `crates/graft-ops/tests/it/rewrite_preview.rs` (one `CheckoutParams` literal). Compiler-forced arm: `crates/graft/tests/it/m5_walk/claim.rs:187` (`WorktreeEffect::EditsStay` with `Untouched | IndexOnly`: the tree must not change).
- New file under my new module: `crates/graft/src/update/switch/tests.rs`.
- Flipped graft-ops test: `a_dirty_tracked_tree_refuses_a_new_branch_switch_but_not_rebase_or_checkout` → `a_dirty_tracked_tree_refuses_a_stash_branch_but_not_rebase_checkout_or_create_at_head` (`branch_create(true)` at HEAD over edits: was `Err(DirtyTree{count:2})`, now `Ok`). `b ⇥` refusal tests named in the spec (`m5_walk/refs_and_names.rs:72`, `name_and_drag.rs:90`) are clean-tree creates: unchanged, green.
- Done-word grep (before changing): `describe_checkout` unchanged except `Track`, so undo/redo/interrupted words (`undo/tests/set_aside.rs:264-351`, `operate/tests/staging_not_kept.rs`, `jobs/operate/tests.rs:59,165`, `undo_over_drift` `undo: checkout fix ✓`, `set_aside.rs:1194` relaunch notice) keep `checkout fix`. Changed on screen: the Done line after a branch checkout and the 30b staging-not-kept line (`m6_stash_put_back/set_aside.rs` `a_dirty_switch_is_three_keys`, `a_switch_whose_staging_cannot_come_back_says_so`, both updated, captures re-taken).
- **Gap (for KNOWN_GAPS):** `u` after a `Track` checkout deletes the branch but leaves `branch.<local>.remote/.merge` in config. Harmless to git; redo re-creates the ref and tracking comes back (`switch_rule::track_undo_deletes_the_branch_and_puts_head_back` docs it).
- **ADR-0045 §4 amendment owed** (conductor): the set-aside kinds gain the switching create away from HEAD.
- **Question:** a switching create's landing keeps today's words (`branch nb at <sha7> ✓`, + `your edits are back`), not `on nb ✓ · u goes back`. The spec names words for checkouts only; `landed_words` could take `BranchCreate { checkout: true }` in one arm if wanted.
- `instant_refused`'s `opened_by` falls back to `.` (checkout) / `b` (create) when the launch recorded none; a checkout launch records it (`launch_unstoppable(.., Some(opened_by))`), a create's `launch` does not, so `b` is the fallback in practice.
- `REFS_PENDING` wording duplicated in `switch.rs` (the push box's constant is private to `remote/push_box.rs`, 16b's file).
- Walk fixtures wait for the first status read before pressing keys, so they take the edits branch; the stale-clean path (instant launch refused `DirtyTree` → preview) is covered by the unit test, not a walk.
- Refactors for clippy's 60-line limit (behaviour unchanged): `operate::stopped` (the `Conflicted` arm, moved out of `operated`); `operation_failed` now takes the launched op and does the `instant_refused` check first; `context_menu::run_history_verb`; `name_prompt::switch_to_new`; `Operation::and_stash_if_set_aside`.
- Proposed change (not decided) stands as written above.
- Gates: fmt, clippy, doctests pass. nextest 4368/4382: the 13 blocked above, plus `graft-ops::it rewrite_preview::plan_as_run_leaves_every_other_kind_alone` failed once in the second full run, passed in the first and 3/3 alone. Intermittent under load; cause not investigated (the only change there is one `on_edits` literal).
