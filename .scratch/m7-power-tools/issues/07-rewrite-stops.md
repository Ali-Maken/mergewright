# rewrite-stops — a stop says where he is and what's left; an `edit` stop hands him the graph; `Esc` never aborts

Status: ready
Blocked by: 02-rewrite-engine, 02b-rewrite-engine-b, 03-land-on-result
Crates:
- graft (`update/banner` + `stop.rs`, the focus rule, the shell-begun `c` hand-over);
- graft-ui (`still_to_apply.rs` new, the banner's chain words);
- graft-ops (`check` for Amend at an `edit` stop only).

Source: note "Pass 2a" R4; journeys 4, 5; the keymap diff §2.6a; KNOWN_GAPS :549; keymap §2.6a "shell-begun rebase and
cherry-pick stay M7".
Build with: Opus. It extends M6's banner and stop landing with ops reads that `rewrite-engine` provides.
Scout: `scout` first (ADR-0035).
Needs: rewrite-engine (`progress`, `RewriteProgress`), land-on-result (the finish/abort landing). Doc rows: the
conductor applies the §2.6a diff first.

## Decided (note R4)
- **The banner text:** `REBASING  step i of n · <action> <sha7> <subject>`. Then `· <branch>, branch k of m` only when
  `progress().branch` is `Some`. The ladder at narrow widths drops the subject first, then the chain words, then the
  sha. `REBASING step i of n` is never cut.
- **A conflict stop:** unchanged M6 landing (the resolver, first file). Only the banner words change.
- **An `edit` stop** (`current.action == Edit`, nothing unmerged):
  - The keyboard goes to **the graph**; the banner is drawn, unfocused; `⇥` focuses it.
  - The cursor is on HEAD (the stopped commit), tagged `HEAD · stopped here`.
  - The banner reads `… · change files, space stages, a amends it, c continues · ⇥ banner`.
- **The "still to apply" block** (graft-ui `draw_still_to_apply`):
  - Boxed above the graph, titled `still to apply · not commits yet`.
  - One row per `pending` line, **newest first**, as the graph: action word, sha7, subject.
  - Then `<branch> (<author of its tip>): N commits follow, on top` for each dependent still to come.
  - At most 6 rows, then `+N more`. Drawn at any stop while `pending` is non-empty.
  - Never ghost rows in the graph (field notes :38).
- **`a` at an `edit` stop** amends the stopped commit with what's staged (`commit --amend --no-edit`). This is P14's
  exception, in graft-ops `check`: allowed iff `progress().current.action == Edit` and HEAD == `current.commit`'s
  rewritten oid (git's `rebase-merge/amend` file names it; the scout checks). Any other live operation: P14 stands.
- **`c` in a shell-begun rebase or cherry-pick:** hands the terminal to `git <kind> --continue`, like `M`'s mergetool
  hand-over. git's own editor may ask for a message. On return: the relaunch/refresh path, landing as after any
  continue.
- **`Esc`** on the banner or the resolver during a live operation gives the keyboard back and **never aborts**
  (KNOWN_GAPS :549). The abort is only `a` → box → `y`.
- **Abort landing:** as `land-on-result` defines. Status `✓ rewrite aborted · every branch is as before`.

## Read first
- Note "Pass 2a" R4 (the mocks and words); "Journey counts" (rewrite-stop-to-first-file, edit-stop-lands,
  edit-stop-amend-continue); the keymap diff §2.6a.
- `docs/design/02-keymap.md` §2.6, §2.6a (whole, including the `u` rule and P14).
- `docs/shots/m7-proto-1b-edit-stop.txt`.
- `crates/graft/MAP.md`: `update::banner` (+ `stop.rs`: `stop_reason`, `stop_wordings`, `land_on_head`,
  `continue_pressed`, `resume`, `resumed`), `update::relaunch`, the `M` hand-over. `crates/graft-ops/MAP.md` `state`,
  `apply::resume`, `check` for Amend.

## Scout brief (≤ 1 page)
1. The focus rule on a stop: where "nothing unmerged → focus the banner" is decided (function + line).
2. `stop_wordings`: the ladder and its tests; where the banner text is built.
3. What `Esc` does on the banner and in the resolver during a live operation today (KNOWN_GAPS :549 says it aborts).
   Name the line.
4. The `M` mergetool hand-over: how the terminal is suspended and restored; whether a git command can be handed over
   the same way.
5. How the relaunch path recognises a shell-begun rebase today, and what `c` does on it now (refused? words?).
6. P14's amend refusal: the function and line.

## Signatures
```rust
// graft-ui
pub fn draw_still_to_apply(frame: &mut Frame, area: Rect, view: &PendingView);
pub struct PendingView { pub rows: Vec<PendingRowView>, pub more: usize, pub followers: Vec<(String, String, usize)> }
// graft: the banner reads graft_ops::rewrite::progress (off-thread, with the stop facts); no new pub fn required. Name
// the functions changed in NOTES.
```

## Acceptance
- [ ] **App:**
  - `a_chain_stop_names_the_branch_in_the_banner` (red first);
  - `a_single_branch_stop_has_no_chain_words`;
  - `an_edit_stop_gives_the_keyboard_to_the_graph` (red first: today it focuses the banner);
  - `tab_focuses_the_banner_at_an_edit_stop`;
  - `a_at_an_edit_stop_amends_the_stopped_commit`;
  - `a_at_a_conflict_stop_is_still_refused`;
  - `esc_never_aborts` (red first, KNOWN_GAPS :549);
  - `c_in_a_shell_rebase_hands_over_to_git`, using a fake hand-over as the `M` tests do.
- [ ] **graft-ops:** `amend_is_allowed_only_at_an_edit_stop_on_the_stopped_commit`.
- [ ] **Ladder:** `stop_wordings_keep_step_i_of_n_at_80_columns`.
- [ ] **Walks** (`m7_walk`):
  - `rewrite_stop_to_first_file`: a rewrite forecast to stop; after `y`, 0 keys and the resolver is open on the first
    file. ≤ `noted_keys("rewrite-stop-to-first-file")`.
  - `edit_stop_lands`: 0 keys to the graph at the stopped commit, with the block drawn.
  - `edit_stop_amend_continue`: a file edit (fs write), then `space a c`. ≤ the fence: the commit contains the edit,
    and the rewrite finished.
  - `abort_from_a_chain_stop`: `⇥ a y`. Every branch is back, and HEAD is on his branch.
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/rewrite-chain-stop-187x50.txt`. **Right:** `REBASING  step 3 of 5 · pick <sha> <subject> ·
    feature-b, branch 2 of 3`; the resolver below.
  - `docs/shots/m7/rewrite-edit-stop-187x50.txt`. **Right:**
    - `still to apply · not commits yet` boxed;
    - two `pick` rows;
    - `feature-b (Sara): 2 commits follow, on top`;
    - `▸` on `HEAD · stopped here`;
    - bar `a amend it  c continue  space stage`.
  - `docs/shots/m7/rewrite-edit-stop-80x24.txt`. **Right:** `REBASING step 3 of 7` whole; the block cut to `+N more`.
- [ ] Hint-bar tests green. MAP.md (graft, graft-ui, graft-ops). TESTING.md section "Stops during a rewrite" + captions
      + contact sheet.

## Touches (check each, report how)
- **M6 banner and resolver for merge, pull and stash clashes:** the words and focus are unchanged for every
  non-rewrite stop. Check: every m6 banner/resolver test and capture byte-identical (`check_shot`), or each change
  listed in NOTES.
- **The `u`/`U` refusal while live:** unchanged. Check: its test.
- **P14 for merge/cherry-pick/revert:** unchanged. Check: the existing amend-refusal tests.
- **The relaunch after kill -9:** reads the saved plan through `progress`. Check: `a_relaunch_mid_rewrite_shows_the_chain`
  (add).
- **Keymap:** §2.6a per the diff; `Esc` meaning is now "never aborts". Check: the hint-bar tests.
- **Journal/undo:** none new.

## Rules / Gates
As rewrite-engine.

## Size
Large (~70k). Split if needed:
- **-a:** banner words, the focus rule, `Esc`, P14 exception;
- **-b:** the still-to-apply block, the shell hand-over, walks, captures.
