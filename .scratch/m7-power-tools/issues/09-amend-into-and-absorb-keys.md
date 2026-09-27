# amend-into-and-absorb-keys — `a` on an older commit puts the staged changes into it; absorb from the `.` menu

Status: ready
Blocked by: 05-rewrite-preview, 06-absorb-map, 06e (new refusal and stay variants — planner, 2026-09-27)
Crates: graft (the `a` route, the `.` menu item on the uncommitted row, walks, captures). graft-ui only if the scout
finds the preview box can't draw the absorb rows as `rewrite-preview` left it.
Source: note "Pass 2a" R2; journeys 2, 2b, 7; the keymap diff (§2.3 `a`, §2.3b `y`).
Build with: Opus. It wires existing ops calls into the existing preview.
Scout: `scout` first (ADR-0035). This settles Decision 5's open question: what `a` does today with the cursor off HEAD.
Needs: rewrite-preview (preview of `Operation::Rewrite`), absorb-map (`RewriteRequest::Absorb`).

## Decided
- **`a` on a commit row of his branch that is not HEAD:**
  - `PlanRewrite(AmendInto{target})` → the preview (note R2's first box) → `y`.
  - The box's title is `Preview: put your staged changes into <sha7> "<subject>"`; the bar reads `y put it in  esc
    cancel`.
- **`a` on HEAD or the uncommitted row:**
  - With **no** dependents: exactly today's amend (unchanged path).
  - With dependents: `PlanRewrite(AmendInto{target: HEAD})` → the preview → `y` (journey 7).
  - The dependents check is `rewrite::dependents`, off-thread. While it runs, `a` shows `checking what's built on
    feature-a…` and takes no second key. It must stay under 1 frame on the fixture; measure it.
- **`a` on a commit off his branch:** the status line reads `a puts staged changes into a commit on your branch ·
  <sha7> isn't on it`.
- **Nothing staged, off HEAD:** `nothing staged to put into <sha7> · space stages`.
- **The `.` menu on the uncommitted row** gains `Put staged changes into their commits (absorb)`. Its place is after
  today's items, so their order is unchanged.
  - `⏎` → `PlanRewrite(Absorb)` → the absorb preview (note R2's second box) → `y`.
  - No staged changes → the item is dim, with `nothing staged`.
- **Lands:**
  - The graph, cursor on the new HEAD.
  - The target's new row is tagged `✎ changed` until the next operation. This is the same row-tag mechanism the
    journal's `↺ put back` will use, so `land-on-result` or G-list may already have it; the scout checks.
  - Status: `✓ put into "<subject>" · u undoes`, or for absorb `✓ put 2 hunks into 2 commits · 1 stays staged ·
    u undoes`.
- **During an operation:** `a` keeps P14's refusal, except at an `edit` stop (`rewrite-stops` owns that).

## Read first
- Note "Pass 2a" R2 (both box mocks, the refusal words), "Journey counts" (fix-into-older, absorb-staged,
  amend-with-dependents), the keymap diff §2.3 `a`.
- `docs/design/02-keymap.md` §2.1 (the `.` menu rules), §2.3, §2.3b, §2.6a (P14).
- `crates/graft/MAP.md`: `update::operate` (`claim_write`, `launch`), `update::context_menu` (+ slots, verbs),
  `update::commit_editor` (what `a` opens), `update::preview`.

## Scout brief (≤ 1 page)
1. What `a` does today: on HEAD, on the uncommitted row, and on an older commit (does it amend HEAD regardless?). The
   function and its line.
2. The uncommitted row's `.` menu items, in order, and how an item is added (`slots`, `verbs`).
3. Is there a row-tag mechanism (e.g. `↺`, `○ was here`) in the graph today?
4. The walks and captures that press `a` (`grep -rn "'a'"` over `crates/graft/tests`, `.scratch/*/pty-walk*.py`).

## Signatures
```rust
// graft — no new pub fn required. The `a` route and the `.` item call:
//   Command::PlanRewrite(RewriteRequest::AmendInto { target }) / (RewriteRequest::Absorb)
// and open the preview on Operation::Rewrite(params) via the existing opener. Name the functions touched in NOTES.
```

## Acceptance
- [ ] **App:**
  - `a_on_an_older_commit_opens_the_put_it_in_preview` (red first: today it amends HEAD, or whatever the scout finds);
  - `a_on_head_without_dependents_is_todays_amend`;
  - `a_on_head_with_a_dependent_previews`;
  - `a_off_the_branch_is_refused_in_words`;
  - `a_with_nothing_staged_is_refused_in_words`;
  - `absorb_item_is_dim_with_nothing_staged`;
  - `absorb_opens_its_preview`;
  - `y_launches`.
- [ ] **Walks** (`m7_walk`):
  - `fix_into_older` = motion `a y` ≤ `noted_keys("fix-into-older")`: the target commit's diff now contains the fix,
    and nothing is staged;
  - `absorb_staged` = `.` motion `⏎ y` ≤ `noted_keys("absorb-staged")`: 2 hunks placed, 1 still staged;
  - `amend_with_dependents` = `a y` ≤ `noted_keys("amend-with-dependents")`: feature-b is rebased onto the amended
    feature-a;
  - `u` after each restores refs + index (a walk step).
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/amend-into-preview-187x50.txt`. **Right:** title `Preview: put your staged changes into 3b2a1f0
    "Add OAuth callback"`; the `gains` row; `feature-b … built on yours · follows`; `u undoes all of it`.
  - `docs/shots/m7/absorb-preview-120x40.txt`. **Right:** two `→ <sha>` rows; one `stays staged · no commit on your
    branch changed these lines`.
  - `docs/shots/m7/amend-into-landed-120x40.txt`. **Right:** the cursor on the new HEAD; `✎ changed` on the target
    row; status `✓ put into "Add OAuth callback" · u undoes`.
- [ ] Hint-bar tests green. MAP.md (graft). TESTING.md rows + captions + contact sheet.

## Touches (check each, report how)
- **Today's amend path** (commit editor, `--amend`): unchanged when nothing is built on HEAD. Check: the m3/m5 amend
  tests and captures byte-identical.
- **P14 refusal while an operation is live:** unchanged. Check: the existing refusal test.
- **The `.` menu slots on commit rows:** unchanged (the item is on the uncommitted row only). Check: the context_menu
  tests + captures.
- **Journal/undo:** the engine's entry. Check: the `u` walk steps.
- **Keymap:** §2.3 `a` row per the note's diff (the conductor, before the build).

## Rules / Gates
As rewrite-engine.

## Size
Medium (~55k).

## Carry (conductor, 2026-09-26): words from 06c
06c added `StaysWhy::Submodule` and `StaysWhy::OutsideYourCommits` (a hunk that stays), and the refusals `AllPushed`, `AlreadyPushed` and `BeyondReach`, whose words are in `explain()` (graft-ops `operation.rs`). This ticket renders them. Every new `StaysWhy` needs words; test each one.

## Carry (planner, 2026-09-27): words from 06e's ruling
- `StaysWhy::AlreadyPushed` (new): `stays staged · the commit that changed these lines is already pushed`. Test it in the absorb preview.
- `Refusal::NoCommitsOfItsOwn { branch }` (new): `<branch> has no commits of its own yet · c commits them` — absorb on a branch just cut from `main`'s tip.
- `Refusal::AlreadyPushed` words now: `<short> is already pushed · a only changes older commits you haven't pushed` (a pushed HEAD is amended, not refused).
- `Refusal::BeyondReach` now names the branch: `<short> is more than 10 commits back · on <branch> a reaches the last 10 unpushed commits`.
- `a` on a pushed older commit of a tracked feature branch is now refused `AlreadyPushed` (before 06e it planned). If the scout finds a walk or fixture whose `a` target is a pushed older commit (`fix_into_older`), the fixture's target must be unpushed; say which in NOTES. Test: `a_on_a_pushed_older_commit_is_refused_in_words`.
