# set-aside-shows-in-the-preview-and-lands: the preview says the edits go aside; every end lands where the note says

Status: done (builder-opus, 2026-09-26; compacted once mid-ticket, see NOTES)
Blocked by: 73 (merged), 82 (shares files; see 82 Touches). 67 capture delete (Touches :99-100) done by 73.
From: pick 16 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :25): "the preview says so; a clash putting them back
lands on the conflict screen".
Build with: **Opus** (conductor, 2026-09-25, after the scout) → `builder-opus`: words and routing over 73's and 72b's answers.
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
- **Review 72+72b m8** (`docs/reviews/m6-final-fixes-72-72b.md`): the `Operated::Clashed` arms in `banner::resumed`
  (`crates/graft/src/update/banner.rs:745-748`, which also calls `operation_ended`) and `kept_open` (`:884-886`) are
  unreachable until this ticket and untested. Check: this ticket's tests reach **both** arms (a put-back clash after
  `c` finishes a stopped rebase, and whichever end reaches `kept_open`), each named in NOTES; lines re-checked after 82.

## Acceptance (name the verifying test at tick time)
- [x] Every test above, green — `set_aside_row_words`, `set_aside_row_cuts_the_staged_clause_first`
  (graft-ui `preview_box/tests.rs`), `set_aside_roles_name_the_panes_yours_and_set_aside` (`conflict_resolver/names.rs`),
  `a_clean_put_back_says_your_edits_are_back`, `a_put_back_failure_names_the_stash_and_s_p` (`update/operate.rs`),
  `a_put_back_clash_lands_on_the_resolver_with_set_aside_names`, `a_put_back_clash_stop_line_says_your_edits`,
  `a_put_back_clash_done_says_your_edits_are_back` (`update/clash/tests.rs`), `aborting_a_stopped_rebase_says_your_edits_are_back`
  (`banner/tests/resume_words.rs`), `a_stop_with_edits_aside_says_so_and_drops_it_first` (`banner/tests.rs`),
  `relaunch_after_a_kill_names_the_stash_the_edits_are_in` (`update/undo.rs`), `every_key_the_bar_draws_is_answered`
  (with `SetAside` in `update/tests.rs` `in_roles`). **Red not observed separately**: written before the code, but each
  would not have compiled against the old code (new variant/fns/fields), so the first run was a compile failure, not a
  red assertion.
- [x] Walk, real repo (`tests/it/m6_stash_put_back/set_aside.rs`, a child of `m6_stash_put_back.rs`), each asserting the
  key count ≤ the note's row (rows added to the `journey-counts` fence, ids in `JOURNEYS`) and the end state — all six
  green, named below:
  - `a_dirty_pull_rebase_is_three_keys` — `p r r`: rebased; the 3 edited files unchanged in the tree; stash list empty.
  - `a_rejected_push_with_edits_is_five_keys` — `p r r P y`: `git log origin/main` has his commit on the teammate's;
    edits in the tree; stash list empty.
  - `a_dirty_switch_is_three_keys` — `. ⏎ y` on a branch row: on the branch; edits in the tree.
  - `a_dirty_switch_that_clashes_is_five_keys` — `. ⏎ y o ⏎`: on the branch; file has `yours`; stash list empty.
  - `a_stopped_rebase_then_a_clash_is_eight_keys` — `p r r o ⏎ c o ⏎`.
  - `u_after_a_dirty_switch_puts_everything_back` — `. ⏎ y u`: on the old branch; index and tree as before, staged
    split included; stash list as before.
- [x] Captures (rule 6; colour **unverified**, rule 5), written by the walks — `a_dirty_pull_rebase_is_three_keys`
  (preview ×2, done), `a_dirty_switch_that_clashes_is_five_keys` (landing); each asserts its words before `check_shot`:
  - `docs/shots/m6/set-aside-preview-120x40.txt`, `-80x24.txt` — **What right looks like:** the rebase preview box
    with `[ Confirm ]`; under `working tree:` the row `your edits: 3 files set aside, put back after · 1 staged comes
    back unstaged` (at 80 cols the staged clause may be cut, the file count may not).
  - `docs/shots/m6/set-aside-clash-landing-120x40.txt` — resolver panes `yours · main` and `set aside · your edits`;
    notice `your edits clashed in 1 file · esc, u undoes the switch too`; bar `o yours` `t set aside`.
  - `docs/shots/m6/set-aside-done-120x40.txt` — graph on `HEAD`; bottom row ends `· your edits are back`.
- [x] NOTES: scout answers, captures touched, TESTING.md rows for 75 (below; doc, no test).
- [x] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large (~60k): one row, one role, eight wordings, six walks, four captures.

## NOTES (builder, 2026-09-26)

**Compacted once mid-ticket** (context ran out after the unit tests, before the walks). Nothing remains: the walks,
captures, contact sheet, MAP.md, README and gates were all done after the compaction.

**Scout used** (`.scratch/m6-conflicts/scouts/set-aside-shows-in-the-preview-and-lands.md`); built on the code where it
and the ticket differed:
1. Preview rows are app-side (`update/preview.rs` `plan_lines`). The words live in graft-ui
   (`preview_box::set_aside_row(files, staged)` + `fitting_line(wordings, width)`), the app pushes the fitting one under
   `worktree_line` from `state.size.columns`. **Signature deviation:** counts, not `graft_ops::SetAside` — graft-ui does
   not depend on graft-ops (crate rule).
2. `SideRoles::SetAside` (ours `yours`, theirs `set aside`); app arm `resolver/names.rs` before the StashPop arm, keyed on
   `clash::set_aside(state)` (the live clash's `why`), so a set-aside is told from a pop.
3. Done clause goes in `Status::Done::keys` (`what ✓ · your edits are back`) from `clash::edits_back(entry)`: in
   `operate::done_status` (new helper) and `banner::resumed_words`' Finished arm, so 68's aborted wording is extended,
   not replaced. The op's own line for a rebase is `rebase main onto origin/main` (the note's example says `rebased`;
   the existing clean-tree words were kept — Touches says unchanged).
4. Stop ladder: `stop_line_wordings` inserts `<full> · edits aside` as rung 0 when a Pending live entry carries a
   `SetAside` put-back; `stop_line` is now that ladder's first rung. Clash ladder (`clash::stop_wordings`) words the
   set-aside as `your edits clashed` / `your edits clash from last run`, last rung deduped.
5. Interrupted line is `undo::interrupted_notice` (not relaunch): ` · edits in stash@{n}` when the entry is SetAside and
   its stash is still listed.
6. Put-back failure: `Operated::Failed` gains `edits_in: Option<usize>` (jobs `edits_kept_in` finds the stash by commit
   id); `operate::edits_in_status` → sticky `your edits are in stash@{n}` · `S p puts them back`; also in `resumed`.

**Review 72+72b m8 arms:** `banner::resumed`'s `Operated::Clashed` (`banner.rs:742`) is reached by
`a_continue_whose_put_back_clashes_lands_on_the_clash` (unit) **and** by the real walk
`a_stopped_rebase_then_a_clash_is_eight_keys` (observed `Resumed(Clashed { … why: SetAside, paths: ["g.txt"] })` in a
`GRAFT_TEST_TRACE` run). `kept_open`'s Clashed arm (`banner.rs:910`) is **unreachable through `resumed`**:
`kept_open` runs only on `FinishAnswer::Kept`, and `finish::answers` returns `None` for `Clashed` (`answers.rs:175`). It
is tested by a direct call, `kept_open_rereads_everything_after_a_clash`. Question for the reviewer: keep the arm or
make it `unreachable`-by-type.

**Walks** (`tests/it/m6_stash_put_back/set_aside.rs`): `a_dirty_pull_rebase_is_three_keys`,
`a_rejected_push_with_edits_is_five_keys`, `a_dirty_switch_is_three_keys`, `a_dirty_switch_that_clashes_is_five_keys`,
`a_stopped_rebase_then_a_clash_is_eight_keys`, `u_after_a_dirty_switch_puts_everything_back` (`u` counted against the
note's "`u` after any of them: 1", :47 — no row id exists for it, so it is a constant, not a fence row). Plus the
Touches walk `a_stop_with_edits_aside_shows_in_the_stop_line_s_and_j`.
- Staged count (ticket 83 changes `SetAside::staged`): every fixture stages a **modification** of a tracked file only
  (`one.txt` in the rebase walks, `base.txt` in the switch walks), so `1 staged` holds under 83's meaning too. The number
  is asserted in `a_dirty_pull_rebase_is_three_keys` (`diverged_with_three_edits`) and the unit tests.
- The stop-then-clash walk waits for the resolver's re-read of the file before `c` (`resolver::settled`), as a person
  would; pressed early, `c` is refused `still reading the file`.

**Findings (not fixed, for the conductor):**
- At 120 columns the banner's stop line has no room for ` · edits aside` beside its seven keys, so the ladder drops it
  (as the note orders); it shows at 187 (asserted both ways in `a_stop_with_edits_aside_shows_in_the_stop_line_s_and_j`).
  At his usual widths he will rarely see it; the `S` row and the graph's stash row still show the set-aside.
- `S p` pops the **selected** stash, stash@{0} on open. If the kept edits are at stash@{n}, n > 0, the sticky line's
  `S p puts them back` needs a `j` first. Words say `stash@{n}` correctly.
- `Kept::Git` with `back: true` (edits back, only the drop failed) would still produce `edits_in` and read `your edits
  are in stash@{n}` — misleading in that one case. Not reachable in a walk here; unverified.
- The done capture's rebased row reads `futu…`: the rebase commit is dated by the wall clock, the capture by the
  fixtures' fixed clock. Said in its header.

**Touches** (each checked):
- Preview box 80/120: `the_set_aside_row_draws_whole_at_80_and_120` (graft-ui) + preview captures; border sweep green in
  the full run. 187 not rendered separately (the row is whole at 120, so it is whole wider).
- Resolver hints/bar: `every_key_the_bar_draws_is_answered`, `every_planned_hint_is_still_unbound` green (`t set aside`
  added to `keys/bound.rs`); landing capture asserts `o yours` `t set aside` at 120.
- 72b's pop/apply words unchanged: 72b's walks and `clash/tests.rs` green.
- Clean-tree rebase done unchanged: full suite green (edits_back is `None` without a put-back).
- 68's aborted wording: `aborting_a_rebase_does_not_read_as_done` green.
- Readers of `Stop`: `Stop` untouched (diff).
- `J`: the entry reads `rebase main onto origin/main`, no stash words — `a_stop_with_edits_aside_shows_in_the_stop_line_s_and_j`.
- `S` during a stop: `graft: your edits, set aside for …` row — same test.
- Radar (66): full suite green.
- 67's capture: already deleted by 73; `grep -rn fix-rebase-refused-before-preview docs TESTING.md` empty.
- Other rebase/checkout preview captures: `check_shot` green in the full run, none regenerated.
- README.md:133-134 (conductor, from 73's review): reworded to the set-aside row and done clause.
- Contact sheet: 72b's three `stash-clash-*` captures and this ticket's four added to
  `m6-contact-sheet-captions.tsv` (before 82's row); sheet rebuilt, 4 parts, largest 179 772 B.
- Design note fence: five rows added to `docs/design-notes/m6-conflicts.md` `journey-counts` (72b's precedent);
  `journey_counts_match_the_note` green.
- Keymap doc §2.9: not edited (conductor's).
- Gates: fmt, clippy clean; nextest 3435/3436 — the one failure is the known
  `published_repo::nothing_a_commit_would_carry_names_this_machine` on
  `.scratch/m6-conflicts/issues/82-review-fix-72-72b.md:133`, fixed on main; doctests green.

**TESTING.md rows for 75** (what right looks like):
- Dirty pull-rebase: `p r r` with 3 files edited, 1 staged → preview row `your edits: 3 files set aside, put back after ·
  1 staged comes back unstaged`; after, `rebase main onto origin/main ✓ · your edits are back`; files still edited;
  `git stash list` empty.
- Dirty switch: `. ⏎ y` on a branch row → on it, edits in the tree, done line ends `· your edits are back`; `u` → old
  branch, same staged/unstaged split, stash list as before.
- Put-back clash: resolver on the file, panes `yours · <branch>` | `set aside · your edits`, notice `your edits clashed in
  1 file · esc, u undoes the switch too`, bar `o yours  t set aside`; `o ⏎` → `every file done · your edits are back ✓`.
- Stopped rebase with edits aside: `S` lists `graft: your edits, set aside for rebase …`; stop line ends `· edits aside`
  at wide terminals only; `c` after resolving → clean line or the clash landing.

**Captures to judge by eye:** `docs/shots/m6/set-aside-preview-120x40.txt`, `set-aside-preview-80x24.txt`,
`set-aside-clash-landing-120x40.txt`, `set-aside-done-120x40.txt` (colour unverified).
