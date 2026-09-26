# switch-rule — one rule for every switch: instant on a clean tree, preview (`y`) with unsaved edits or a detached target

Status: ready
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
  - `✓ on <b> · u goes back`
  - `✓ on <b> (new, tracks <origin/b>) · u goes back`
  - with a set-aside: `✓ on <b> · your edits are back · u goes back` — the existing `clash::edits_back` clause,
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
- [ ] **graft-ops:**
  - `checkout_refuse_refuses_a_dirty_tree` (red first);
  - `checkout_set_aside_is_unchanged` (the m6 set-aside round trips green, named);
  - `switching_create_at_head_over_edits_sets_nothing_aside` (index and files byte-equal after; red first — flips
    today's `DirtyTree` test, named in NOTES);
  - `switching_create_elsewhere_over_edits_sets_them_aside_and_u_restores`;
  - `switching_create_elsewhere_refuse_refuses_dirty`;
  - `switching_create_previews_with_the_put_back_forecast`;
  - `track_creates_the_local_branch_tracking_the_remote` (upstream read back from config);
  - `track_refuses_when_the_local_name_exists`; `track_refuses_when_the_remote_moved`;
  - `track_undo_deletes_the_branch_and_puts_head_back`.
- [ ] **App:**
  - `dot_checkout_on_a_clean_tree_switches_at_once` (red first);
  - `dot_checkout_with_edits_opens_the_set_aside_preview`;
  - `dot_checkout_detached_still_previews`;
  - `b_tab_with_edits_opens_the_preview` (red first);
  - `b_tab_on_a_clean_tree_is_unchanged`;
  - `an_instant_switch_refused_for_edits_lands_on_the_preview`;
  - `switch_words_on_landing` (the three lines above);
  - `a_switching_create_clash_is_labelled_switch`.
- [ ] **Walks:** m5 `checkout_branch_via_menu` (`pick_revert_reset.rs:228`) and `journey_menu_checkout`
      (`every_scenario.rs:143`) now `. ⏎`, no `y`; HEAD on the target, `u` back. `checkout_detached_via_menu`
      unchanged. New in `m7_walk`: `switch_with_edits_via_menu` = `. ⏎ y`; `new_branch_with_edits` = `b <name> ⇥ ⏎ y`
      — edits back after each, then `u`.
- [ ] **Captures** (colour unverified): `docs/shots/m7/switch-menu-clean-landed-120x40.txt`. **Right:** `▸` on the new
      HEAD's row; status `✓ on feature-b · u goes back`; no preview box.
      `docs/shots/m7/new-branch-with-edits-preview-120x40.txt`. **Right:** `Preview:` box naming the new branch, the
      line `your edits stay as they are` (at HEAD), `y` on the box edge.
      Every changed m5/m6 capture re-taken (`GRAFT_UPDATE_SHOTS=1`, then a pass without, all `settled`); NOTES lists
      each with what changed.
- [ ] MAP.md (graft-ops, graft). TESTING.md rows for the new walks and captures.

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
