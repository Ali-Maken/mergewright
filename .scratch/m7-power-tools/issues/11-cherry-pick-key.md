# cherry-pick-key — `o` takes the cursor commit onto his branch, previewed; it refuses in words what it can't take

Status: ready
Blocked by: 10-branch-picker
Crates: graft-ops (`check_pick` refusals, `sets_aside` for CherryPick), graft (the `o` route, the `.` slot-2 dim and label).
Source: note "Pass 2a" R5 (`o`); Decision 5; field notes :73; KNOWN_GAPS :806 (question: "Cherry-pick in the `.` menu
is never dimmed, even on the commit you are on or on a merge; `check_pick` refuses only with no commits or a moved
HEAD"), :854; `docs/research/checkout-cherry-pick.md` §2, §7.
Build with: Opus. A key onto an existing previewed op; two refusals; an ADR-0045 extension.
Scout: `scout` first (ADR-0035).
Needs: branch-picker (both edit graft; `cherry-pick-other-branch` walks through `C … Tab`). Doc rows: the keymap diff
(§2.1 rare verbs, §2.3 `o`, §2.3b) and "Where I'd differ" 4 (set-aside for cherry-pick) approved before the build. If 4
isn't approved: unsaved edits the pick touches refuse with `c commit  S stash` words, and nothing is set aside.

## Decided
- **`o` on a commit row** → `Operation::CherryPick(PickParams{commits: vec![oid], …})` → the existing preview, titled
  `Preview: take <sha7> "<subject>" onto <branch>`, bar `y take it  esc cancel` → `y`.
- **Refusals** (`check_pick`; the preview never opens):
  - `Refusal::AlreadyOnBranch{commit, branch}`: `<sha7> is already on <branch>`, for HEAD or any ancestor of HEAD.
  - `Refusal::PickMerge{commit}`: `<sha7> is a merge · taking a merge isn't in Graft yet`.
  - The uncommitted row or a stash row: `o takes a commit · this row isn't one` (app-side).
- **The `.` menu slot 2** keeps its place. Its label carries `(o)`. It dims in place with the same refusal words.
- **Unsaved edits the pick touches:** set aside and put back, one entry (ADR-0045 §4 + CherryPick, if approved). The
  preview shows the set-aside line. A clash lands on the resolver.
- **Lands** (`land-on-result`): the cursor on the new commit, `✓ took "<subject>" onto <branch> · u undoes`. A
  conflict → the resolver (M6). The banner's `c`/`s`/`a` are as today for a cherry-pick.

## Read first
- Note "Pass 2a" R5 (the `o` bullets, `.` menu), "Journey counts" (cherry-pick-*), the keymap diff §2.1/§2.3/§2.3b,
  "Where I'd differ" 4.
- `docs/design/02-keymap.md` §2.1 (the rare-verbs paragraph, `.` slots), §2.3b.
- `docs/adr/0045-…` §4.
- `crates/graft-ops/MAP.md`: `operation` (`CherryPick`, `PickParams`, `sets_aside`), `history` (`check_pick`),
  `preview`. `crates/graft/MAP.md`: `update::context_menu` (+ slots), `update::operate`.

## Scout brief (≤ 1 page)
1. `check_pick` today: what it refuses, and its line. What a pick of HEAD, of an ancestor, and of a merge does today
   (KNOWN_GAPS :806's question). Answer from code, with the line.
2. How slot 2's label and dim state are built; whether dimming carries words today (slot 1 does: `Checkout — already on
   <b>`).
3. What `o` does on the graph today (grep).
4. The set-aside wrap: what adding `CherryPick` requires.

## Signatures
```rust
// graft-ops
// Refusal::AlreadyOnBranch { commit: Oid, branch: RefName }, Refusal::PickMerge { commit: Oid }; Display = words above
// Operation::sets_aside(): CherryPick when touched paths have unsaved edits (if approved)
```

## Acceptance
- [ ] **graft-ops:**
  - `picking_head_is_refused_already_on_branch` (red first);
  - `picking_an_ancestor_is_refused_already_on_branch`;
  - `picking_a_merge_is_refused` (red first);
  - `picking_over_touched_unsaved_edits_sets_them_aside` (if approved);
  - `a_pick_put_back_that_clashes_stops_as_a_clash`.
- [ ] **App:**
  - `o_opens_the_take_preview` (red first);
  - `o_on_the_uncommitted_row_is_refused_in_words`;
  - `slot_two_dims_with_the_same_words`;
  - `slot_two_label_carries_o`;
  - `y_takes_it_and_lands_on_the_new_commit`.
- [ ] **Walks** (`m7_walk`):
  - `cherry_pick_in_view` = motion `o y`;
  - `cherry_pick_other_branch` = `C <type> Tab` motion `o y`.

  Each ≤ its `noted_keys`. The commit's change is on HEAD; `u` removes it.
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/take-preview-120x40.txt`. **Right:** `Preview: take c0ffee1 "Fix null session on logout" onto
    feature-a`; bar `y take it`.
  - `docs/shots/m7/take-refused-merge-120x40.txt`. **Right:** the status line `<sha> is a merge · taking a merge isn't
    in Graft yet`.
  - `docs/shots/m7/menu-slot-two-dim-120x40.txt`. **Right:** slot 2 dim, reading `Cherry-pick (o) — already on
    feature-a`.
- [ ] Hint-bar tests green. MAP.md (graft-ops, graft). TESTING.md rows + captions + contact sheet. NOTES answers
      KNOWN_GAPS :806's question with its line.

## Touches (check each, report how)
- **`.` menu slots on commit rows** (the stash-badge order, conductor 2026-09-25): unchanged order. Check: the
  context_menu tests + captures (label text changes: list each capture).
- **The cherry-pick stop, banner and resolver (M6):** unchanged. Check: the m6 cherry-pick tests.
- **Set-aside (ADR-0045):** a new kind. Check: all set-aside tests green. **ADR-0045 amendment rows owed** (the conductor).
- **Keymap:** the §2.1 rare-verbs line and §2.3 `o` (the conductor applies them first).
- **Journal/undo:** CherryPick entries as today, plus put-back. Check: the `u` walk step.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).
