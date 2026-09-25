# set-aside-shows-in-the-preview-and-lands: the preview says the edits go aside; every end lands where the note says

Status: ready
Blocked by: 73
From: pick 16 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :25): "the preview says so; a clash putting them back
lands on the conflict screen".
Build with: (conductor, after the scout; planner suggests Opus — words and routing over 73's and 72b's answers)
Scout: `.scratch/m6-conflicts/scouts/set-aside-shows-in-the-preview-and-lands.md` — brief below.
Needs: 73 (`SetAside`, `PreviewPlan::set_aside`, `Clashed { why: SetAside }`, `put_back` on interrupted entries);
72b (`ClashState`, `AppState::clash`, `refresh_clash`, `land_on_clash`, the `close_after_last` clash branch, `side_names`
reading `AppState::clash`); 71 (`Checkout` in the `.` menu's first slot, for `. ⏎ y`).
Crates: graft-ui (`preview_box` row, `SideRoles::SetAside`, resolver hints), graft (preview view, operate/banner words,
resolver names and close words, relaunch words, walks, captures).
Group: **F2**. Last before the acceptance ticket (75).

## Amended 2026-09-25 (planner, after scout 72)
- A clash is `AppState::clash: Option<ClashState>` (72b), not a `Stop`. This ticket words the `SetAside` case of each
  72b surface: `side_names` (→ `SideRoles::SetAside`), the landing notice, the close words, the clash stop line. The
  rebase-stopped-with-edits-aside line stays a `Stop` (git has a rebase in progress) and reads the Pending entry's
  `put_back` through the journal state 72b already reads.

## Read first
- `docs/design-notes/m6-stash-put-back.md` "Journeys", "Where each event lands him", "New UI".
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §5 (what he is told), §6 (undo words).
- `crates/graft-ui/MAP.md` preview box, `SideRoles`, resolver hints.
- `crates/graft/MAP.md` :218-229 (`operate`), :289-317 (`banner`, `resume`, `stop_wordings`), :321-335 (`resolver`),
  :380-397 (`update::preview`), :436-441 (`relaunch`), :674-677 (`resolver_context`, `done_line`).
- 73's and 72b's NOTES and MAP.md lines.
- `docs/design/02-keymap.md` §2.6, §2.9 (`p`, `r`, `y`): no key changes here.

## Decided (design note — not open here)
- **Preview row**, rebase or checkout with `set_aside: Some`: one row under `working tree:`:
  `your edits: 3 files set aside, put back after` + ` · 1 staged comes back unstaged` when `staged > 0`. Cut from the
  right at narrow widths (the staged clause goes first). Singular `1 file`.
- **Clean put-back**: the op's own done line + ` · your edits are back` (e.g. `rebased main onto origin/main ✓ · your
  edits are back`); graph on `HEAD`.
- **Put-back clash** (`AppState::clash` with `why: SetAside`): 72b's `land_on_clash`. Panes
  `yours · <branch>` | `set aside · your edits`; bar `o yours  t set aside`. Notice
  `your edits clashed in 2 files · esc, u undoes the rebase too` (`the switch too` for a checkout). Stop line with the
  resolver shut: `your edits clashed · 1 file conflicted` (ladder: count, then all but `your edits clashed`). Done:
  `every file done · your edits are back ✓` (72b's close drops S with `DropPutBack`; S is not named: he never named it).
- **Rebase stops with edits aside**: P5 as today; the stop line gains ` · edits aside` when the stop's Pending entry has
  `put_back`, dropped first in the ladder. `c` that finishes → clean line or clash landing. `a` → aborted, edits back:
  `aborted: rebase … ✓ · your edits are back` (68's aborted wording + the clause).
- **Other put-back failure**: sticky `your edits are in stash@{0} · S p puts them back`.
- **Killed mid-way, relaunch**: the interrupted line + ` · edits in stash@{0}` when the entry has `put_back` and S is
  still listed.

## Scout brief (≤1 page, code not intent)
1. The preview box's row list for Rebase and Checkout (`graft-ui` `preview_box`, graft `view` preview): where the
   `working tree:` row is built and how rows are cut at 80 columns.
2. `SideRoles` and 72b's `side_names` clash arm, so `SetAside` slots beside `StashPop`.
3. Where the done line of a Rebase/Checkout `Committed` answer is worded (`operate.rs`), and 68's aborted wording
   (`banner.rs`), so the ` · your edits are back` clause has one place.
4. `stop_wordings`' ladder and 72b's clash wording: where a part is added and in which order parts drop.
5. The relaunch interrupted line (`update::relaunch`): its words and what it reads from the entry.
6. Existing walks from a rejected push (`p r r P`, m4) and 67's capture test, to extend rather than copy.
7. Every capture and TESTING.md row showing a rebase or checkout preview (`grep -rln "working tree:" docs/shots`).
8. How the app finds the Pending entry of a live `Stop` (`live_entries` → the journal state) to read its `put_back`.

## Scope
**In:** the preview row; `SideRoles::SetAside`; the done/abort/stop/clash/relaunch/failure words; walks; captures;
replacing 67's capture.
**Out:** the write itself (73); the clash state and landing mechanics (72b); key changes (none); a new screen (none).

## Signatures
```rust
// graft-ui
pub enum SideRoles { /* … */ SetAside }   // ours "yours", theirs "set aside"; hint `t set aside`
// graft-ui, preview box — the row's words from the plan's counts (one function, used at every width):
pub fn set_aside_row(set_aside: graft_ops::SetAside) -> Vec<String>;   // wordings, longest first, for the cut
// graft — app-side names from 72b and the scout; the words above are fixed.
```

## The failing tests first
- graft-ui: `set_aside_row_words` (1 file, 3 files, 3 files 1 staged); `set_aside_row_cuts_the_staged_clause_first`;
  `set_aside_roles_name_the_panes_yours_and_set_aside`.
- graft `update`: `a_clean_put_back_says_your_edits_are_back`; `a_put_back_clash_lands_on_the_resolver_with_set_aside_names`;
  `a_put_back_clash_stop_line_says_your_edits`; `a_put_back_clash_done_says_your_edits_are_back`;
  `aborting_a_stopped_rebase_says_your_edits_are_back`; `a_stop_with_edits_aside_says_so_and_drops_it_first`;
  `a_put_back_failure_names_the_stash_and_s_p`; `relaunch_after_a_kill_names_the_stash_the_edits_are_in`.
- `every_key_the_bar_draws_is_answered` with `SetAside` roles.

## Touches (each entry, and how the builder checks it)
- **Preview box** at 80/120/187: the new row costs one row. Check: a render test per width; the box's border sweep
  (ticket 30) green.
- **Resolver hints** (`o yours  t set aside`) and **`resolver_context`**: Check: bar tests at 80 and 120.
- **72b's surfaces** (names, notice, close words, clash stop line): the `Pop`/`Apply` words unchanged. Check: 72b's
  tests green.
- **Done lines** for every Rebase/Checkout: the clause only when `put_back` was set. Check: an existing clean-tree
  rebase done test unchanged.
- **68's aborted wording**: extended, not replaced. Check: 68's tests green.
- **Stop line ladder** (72b's clash wording, 61's short counts): Check: the ladder test at 80 cols.
- **Readers of `Stop`**: unchanged (the ` · edits aside` part is added in `stop_wordings`, not on `Stop`). Check: diff.
- **Journal browser `J`**: a set-aside entry's description reads as the op (`rebase main onto origin/main`), not a
  stash. Check: a `journal_view` test.
- **Stash overlay `S`** during a stop: the `graft: your edits, set aside for …` row shows. Check: a stash view test.
- **Radar (ticket 66)**: no change. Check: 66's walk green.
- **67's capture** `docs/shots/m6/fix-rebase-refused-before-preview-120x40.txt` and its TESTING.md row: deleted; the
  set-aside preview capture below takes its place. Check: `grep -rn "fix-rebase-refused-before-preview" docs TESTING.md`
  empty after; the contact sheet rebuilt.
- **Captures** of rebase/checkout previews (scout Q7): each on a clean tree → `git diff --exit-code`; else regenerated.
- **Keymap doc** §2.9 `p` row and the `DirtyTree` doc line: owed by the conductor; the builder does not edit
  `docs/design/`.
- **MAP.md**: graft-ui `SetAside`, `set_aside_row`; graft words and landings.

## Acceptance (name the verifying test at tick time)
- [ ] Every test above, red then green.
- [ ] Walk, real repo (`tests/it/m6_stash_put_back.rs`), each asserting the key count ≤ the note's and the end state:
  - `a_dirty_pull_rebase_is_three_keys` — `p r r`: rebased; the 3 edited files unchanged in the tree; stash list empty.
  - `a_rejected_push_with_edits_is_five_keys` — `p r r P y`: `git log origin/main` has his commit on the teammate's;
    edits in the tree; stash list empty.
  - `a_dirty_switch_is_three_keys` — `. ⏎ y` on a branch row: on the branch; edits in the tree.
  - `a_dirty_switch_that_clashes_is_five_keys` — `. ⏎ y o ⏎`: on the branch; file has `yours`; stash list empty.
  - `a_stopped_rebase_then_a_clash_is_eight_keys` — `p r r o ⏎ c o ⏎`.
  - `u_after_a_dirty_switch_puts_everything_back` — `. ⏎ y u`: on the old branch; index and tree as before, staged
    split included; stash list as before.
- [ ] Captures (rule 6; colour **unverified**, rule 5), written by the walks:
  - `docs/shots/m6/set-aside-preview-120x40.txt`, `-80x24.txt` — **What right looks like:** the rebase preview box
    with `[ Confirm ]`; under `working tree:` the row `your edits: 3 files set aside, put back after · 1 staged comes
    back unstaged` (at 80 cols the staged clause may be cut, the file count may not).
  - `docs/shots/m6/set-aside-clash-landing-120x40.txt` — resolver panes `yours · main` and `set aside · your edits`;
    notice `your edits clashed in 1 file · esc, u undoes the switch too`; bar `o yours` `t set aside`.
  - `docs/shots/m6/set-aside-done-120x40.txt` — graph on `HEAD`; bottom row ends `· your edits are back`.
- [ ] NOTES: scout answers, captures touched, TESTING.md rows for 75.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large (~60k): one row, one role, eight wordings, six walks, four captures.
