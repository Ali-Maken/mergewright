# 07d — remainder of 07c: `space` stages at an edit stop, the followers row, the walks' fence, the captures

Status: done (2026-09-28, builder; see NOTES — session compacted once). Remainder of 07c.
Build with: **Opus** (provisional; the conductor confirms after the scout, ADR-0035). Why: three crates, a key whose
meaning changes on two rows of the daily graph, and a bar sweep whose knock-ons need judgement; no signature here is
hard to reverse (no Fable test met).
Needs: 07c merged (`still_to_apply.rs`, `PendingView.followers`, `banner::pending_view`, `m7_walk/rewrite_stops.rs`);
07a `edit_stop` / `EditStop { stopped, head_on_it }`; graft-ops `rewrite::progress` / `RewriteProgress`.
Blocked by: **08-a** (holds `update/keys.rs` and `view/rows.rs`, both needed here — see Files).
Scout: **yes, first** (brownfield): ≤1 page — `graph_key`'s `space` arm (`keys.rs:314`) and its return type; the
`Ctrl+A` stage-all route (`update::operate`); which file emits the edit-stop hint bar and why `space` is drawn bare
(07c NOTES round 2); every literal `RewriteProgress { … }` (grep, all crates' tests included).

## Read first
- `07-rewrite-stops.md` lines 1-118 (Decided, Signatures).
- `07c-still-to-apply-and-walks.md` NOTES rounds 2–3 (what exists, the probes).
- `docs/design/02-keymap.md` §2.2 `Space` (:189), §2.3 `Space` (:224), §2.4 `Space`/`Ctrl+A` (:257-259), §2.6a (:309-351).
- `docs/design-notes/m7-power-tools.md` R4 (:601-651), journey fence (:819-845).
- `crates/graft-ops/MAP.md` `## rewrite` (:269-291); `crates/graft/MAP.md` `## update::operate` (:262-269).

## Decided

### D1. `space` at an edit stop stages every change — on the stopped row or the Uncommitted row; the fold elsewhere
- **Where it acts:** graph has the keyboard, `Stop.rewrite` is `Ok` with `edit: Some(_)`, and the cursor is on the
  stopped commit's row (`HEAD · stopped here`) **or** the `Uncommitted changes` row. Those are exactly the rows where
  `a` amends at an edit stop (07 Decided), so `space a c` works from where the stop lands with no motion (the note's
  mock-up :616-624 draws `space stage` with `▸` on HEAD).
- **What it does:** exactly what §2.4 `Ctrl+A` does — the same `Operation`, the same `Command::Operate`, the same
  journal entry, the same status words (MAP `update::operate` :263). Not a toggle: at a stop he may have staged part of
  a file in the changes pane on purpose; a whole-tree toggle would silently unstage it. Nothing unstaged → whatever
  `Ctrl+A` says with nothing to stage (unchanged words).
- **Partial staging** stays as today: `⏎` on the Uncommitted row → changes pane → §2.4 `space` per file/hunk/line.
- **The fold stays reachable:** every other row keeps §2.3 `space` (fold on a merge), byte for byte. Neither staging
  row is a merge: the Uncommitted row is no commit, and git's `edit` stops on a picked commit (Graft refuses merges
  in a rewrite's range, `MergeInRange`). Planner's belief for shell-begun `--rebase-merges` todos: **unverified**;
  the predicate keys on the row, so if it were ever a merge, stage wins there.
- **Outside an edit stop:** unchanged everywhere (the Uncommitted row's `space` included). Generalising `space` =
  stage-all on the Uncommitted row to every day is a separate keymap question, not taken here.
- **One predicate, two callers** (the `pane_c_acts` pattern): the key arm and the bar both ask `space_stages_here`.
  At the edit stop the bar draws `space stage` (was bare `space`).
- **Prior art:** git CLI `git add -A` → 1 command of 3; lazygit `a` stage-all in its files panel (research
  `interactive-rebase.md` §1.1: stops show "YOU ARE HERE" at HEAD; the amend is `A` in the commits panel, ~5 keys with
  panel switches **(?)**). **Take** lazygit's stage-all-in-one-key; **reject** its panel switch (the graph already has
  the keyboard).
- **Keymap:** the code is fixed to match §2.6a. The line is underspecified, not wrong. **For the conductor** (not the
  builder), a clarifying amendment to append to §2.6a :320 after "`space` stages":
  `— on the stopped commit or the Uncommitted-changes row, every change, as §2.4's Ctrl+A; on any other row §2.3's fold`.

### D2. Followers row: extend `RewriteProgress` (no scope cut)
- He works on dependent branches with teammates (memory; note :25, weekly); "is Sara's branch still to come" is R4's
  stated problem (:603-604). Cutting it cuts the point of the block.
- `progress` already reads `git-rebase-todo`'s `update-ref` lines (for `branch.i`) and the saved plan (`plan.json`
  `line: [[name, tip]]`). The followers come from the same two reads plus **one** author spawn.
- **Follower** = each branch of the line **above** the current one (`branch.i + 1 ..= n`), bottom up. Commits = the
  pick-like todo rows between the previous branch's boundary and its `update-ref` line (the top branch, `head-name`, has
  no `update-ref`: its rows run to the end of the todo). Tip author = `%an` of its tip in `plan.json` (the tip before
  the rewrite, the teammate's commit), all tips in one `git log --no-walk --format=%H%x00%an` spawn.
- Empty when `branch` is `None` (a line of 1, or shell-begun: no chain words there either, 07 Decided).
- **The block's pick rows become this branch's rows only** (`pending_here`): today `pending_view` lists every pending
  row, so a follower's picks would be drawn twice (as rows and as `N commits follow`). The mock-up (:616-621,
  `step 3 of 7` = 3 + 2 his + 2 Sara's) lists only his.
- Read failure: `progress` already returns `Err` → 07's "failed progress read" path (rule 3). No partial followers.

### D3. `abort-from-a-chain-stop` fence row: 3 (`esc a y`)
- Landing is the resolver (§2.6, modal, no `a`); `esc` there lands on the banner (§2.6a :317); `a` → box; `y`.
- Best tool: lazygit lands on a `Conflicts!` menu whose `Abort` is `a` (research `conflict-resolution.md` :105-106):
  1 key + a confirm **(?)**, but that menu must be left to resolve anything; from its Files panel, `m` (rebase
  options), `a`, confirm = 3 **(?: confirm not documented)**. git: `git rebase --abort`, 1 command, no confirm.
- **Keep 3; reject** an `a` in the resolver: saves 1 key on a rare destructive path, and puts abort on `a` one pane
  away from where `a` amends — the note's own reason for edit-stop focus (:1004). ux-principles: error prevention
  over keystroke economy for destructive actions.
- Lands: graph, HEAD on his branch at its old commit, every branch back, `✓ rewrite aborted · every branch is as before`.
- Row to add to the fence (`m7-power-tools.md`, after `edit-stop-amend-continue` :832), verbatim:
  `abort-from-a-chain-stop       3     # lazygit 3 from files (m a ⏎), git 1 cmd · esc a y`

## Scope
- In: D1 (key arm, predicate, bar label); D2 (graft-ops read + graft `pending_view`); D3 (fence row, walk route fixed to
  `esc a y` and fenced); walk `edit_stop_amend_continue`; three captures re-taken; TESTING.md; contact sheet; MAP.md.
- Out: generalising `space` outside edit stops; the 120-wide bar fitting that drops `c`'s verb (07c NOTES, not this
  rule); any keymap edit (conductor applies D1's line); graft-ui widget changes (it already draws followers, tested).

## Signatures
```rust
// graft-ops — rewrite.rs, beside RewriteProgress
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Follower { pub branch: String, pub tip_author: String, pub commits: usize }
// RewriteProgress gains:
pub followers: Vec<Follower>,      // branches above the current one, bottom up; empty when `branch` is None
impl RewriteProgress {
    /// `pending` minus the followers' rows: this branch's rows still to apply, oldest first.
    pub fn pending_here(&self) -> &[TodoLine];
}
// filled in rewrite/progress.rs `progress` (signature unchanged)

// graft — update/keys/edit_stop.rs (new)
/// True iff the graph has the keyboard, the stop is an edit stop, and the cursor is on the stopped
/// commit's row or the Uncommitted-changes row. Callers: `graph_key`'s `space` arm; the edit-stop hint bar.
pub(crate) fn space_stages_here(state: &AppState) -> bool;

// graft — update/keys.rs :314, one arm ahead of the fold:
//   space if edit_stop::space_stages_here(state) => <the Ctrl+A stage-all route, update::operate>,
// graft — update/banner.rs: `pending_view` rows from `progress.pending_here()`; followers from `progress.followers`
//   → PendingView.followers: Vec<(branch, tip_author, commits)> (07's type, unchanged)
```

## Files
- graft-ops: `src/rewrite.rs` (`Follower`, field, `pending_here`), `src/rewrite/progress.rs` (the read), its tests
  (the file `progress` is tested in today; scout names it), `MAP.md` (`## rewrite` `progress` line).
- graft: `src/update/keys.rs` (**08-a's**: one arm), `src/update/keys/edit_stop.rs` (**new**) +
  `src/update/keys/edit_stop_tests.rs` (**new**), `src/update/keys.rs` `mod` line, the edit-stop hint-bar source
  (scout names it; likely `src/view/rows.rs`, **08-a's**), `src/update/banner.rs` (`pending_view`),
  `src/update/banner/tests.rs`, `tests/it/m7_walk/rewrite_stops.rs` (+ fixtures for the captures), every literal
  `RewriteProgress { … }` the scout finds (if one is in `update/tests.rs`, ticket 11's: one line, ask the conductor).
- Docs: `docs/design-notes/m7-power-tools.md` (D3's fence row only), `TESTING.md` "Stops during a rewrite",
  `docs/shots/m7/` (three captures), `m7-contact-sheet-captions.tsv`, contact sheet, `crates/graft/MAP.md`.
- **08-a overlap:** `update/keys.rs` and (likely) `view/rows.rs` → run after 08-a. None of `jobs/runner.rs`,
  `update/events*`, `update/mod.rs`.

## Acceptance
- [x] graft-ops, real git, a 3-branch line (his 4, feature-b by Sara 2, feature-c by Lina 1), edit stop on his row 2:
  `progress_names_each_follower_with_author_and_count` — followers == `[(feature-b, Sara, 2), (feature-c, Lina, 1)]`,
  `pending_here().len() == 2`; `no_followers_on_the_top_branch` (stop in feature-c's row); `no_followers_for_a_shell_rebase`
  (shell `rebase -i --update-refs`: `branch` None, followers empty).
- [x] graft: `pending_view_lists_this_branchs_rows_and_summarises_followers` — no follower's sha7 among the rows; the
  07c box-1 tests still pass (name them at tick time).
- [x] `space_on_the_stopped_row_stages_every_change`, `space_on_the_uncommitted_row_at_an_edit_stop_stages`
  (each: the command sent equals `Ctrl+A`'s, compared, not re-described); `space_on_a_merge_row_at_an_edit_stop_still_folds`;
  `space_outside_an_edit_stop_is_unchanged` (Uncommitted row and a merge row, no stop); `space_with_the_banner_focused_is_the_banners`.
- [x] `the_bar_offers_space_stage_exactly_where_space_stages` (sweep: edit stop / conflict stop / no stop × stopped row /
  Uncommitted row / other row × graph / banner focus: bar draws `space stage` iff `space_stages_here`); the existing
  sweeps `every_key_the_bar_draws_is_answered`, `the_bar_offers_c_exactly_where_c_acts`, the m1 fold and M7/spacefold
  tests all green (named at tick time).
- [x] **Walks** (`m7_walk`, counted, fail over the fence):
  - `edit_stop_amend_continue`: from the landing (cursor on the stopped row), fs write to a tracked file, wait for the
    working-tree read, then `space a c` = 3 ≤ `noted_keys("edit-stop-amend-continue")`. After `space`: `git diff
    --cached` names the file. After `a`: still stopped, HEAD's tree has the edit, same message. After `c`: no
    `rebase-merge/`, his branch's commit has the edit, feature-b on top of his new tip.
  - `abort_from_a_chain_stop`: route `esc a y`, fenced ≤ `noted_keys("abort-from-a-chain-stop")`; every branch at its
    old oid, HEAD on his branch, the status line D3 names.
- [x] **Captures** (colour unverified), each ends in `docs/shots/m7/` ≤ 200 KB, under its Right line in the sheet:
  - `rewrite-edit-stop-187x50.txt` — **Right:** `still to apply · not commits yet` boxed; his two `pick` rows only;
    `feature-b (Sara): 2 commits follow, on top`; `▸`/`▏` on `HEAD · stopped here`; bar contains `a amend it`,
    `c continue`, `space stage`.
  - `rewrite-edit-stop-80x24.txt` — new fixture: his 10 rows + feature-b (Sara) 2, edit on his row 3. **Right:**
    `REBASING  step 3 of 12` whole; no `change files` hint; the block ends its pick rows with `+N more`, N = 7 − picks
    drawn (the builder writes N and the test asserts it); follower row drawn if the widget's height allows — write
    which.
  - `rewrite-chain-stop-187x50.txt` — re-check only: banner `REBASING  step 3 of 5 · pick <sha> <subject> · feature-b,
    branch 2 of 3`, resolver below; byte-identical or each change named in NOTES. `rewrite-edit-stop-120x40.txt`
    regenerated if the pending rows changed.
- [x] TESTING.md "Stops during a rewrite": one entry per walk and capture above, saying what right looks like in the
  words of the Right lines.

## Touches (each checked, reported on the builder's Touches line)
- `RewriteProgress` consumers: banner chain words, relaunch (`a_relaunch_mid_rewrite_shows_the_chain`), `RewriteStop`,
  every literal (grep). Step count unchanged (`step` still counts every pick-like row).
- 07c's widget and `pending_view` tests; 07c's `edit_stop_lands` (its 3 picks were one branch: unchanged — check).
- Graph `space` on every row outside the two, and outside edit stops: fold tests byte-identical.
- Bar: the sweeps above; the 120x40 capture.
- Journal/undo: the stage is `Ctrl+A`'s entry, nothing new; `u` still refused while live.
- Keymap: none by the builder; D1's line is the conductor's.

## Gates / Size
Workspace-wide gates (fmt, clippy `-D warnings`, nextest, doc tests). Captures to files; report paths only.
Size: ~55k.

## Conductor note (2026-09-28, after scout `scouts/07d.md`)

- The scout's file list governs, not the Files line above. It adds graft-ui `graph_screen/hints.rs` (:403-409), `hints/lists.rs` (:126, :353), `hints/context.rs` and `hints/tests.rs`: drawing `space stage` is this ticket's own acceptance, so they are granted. Colour unverified.
- `keys.rs:314` is not a `space` arm. `graph_key` (:290) passes `space` to `view_change` (:315), and the fold is `keys/view_change.rs:87`. Branch before that call.
- `stage_all` (`operate.rs:676`) is reached today only from the changes pane (`detail/keys.rs:269`). Reuse it; add no second path.
- Keymap §2.6a D1 clause: applied (4df7fa65).
- 08 also holds graft-ui `hints/context.rs` and `hints/lists.rs`, so 07d is dispatched after 08-a merges.
- The 07c claim that `k` can't reach Uncommitted was withdrawn by 07c itself (round 3: "`k` does reach it").

## Conductor note (2026-09-28, review `docs/reviews/m7-10-to-07c.md`: Majors 1, 2, Minors 1, 7 are yours; same files)

Each is decided in the review. Red test first for each, and tick each as its own box in NOTES.
- **Major 1:** `Tab` in a shown picker at a live stop goes to the banner (`keys.rs:150` `banner_key` runs before `:166` `picker_key`; `banner.rs:333-352` `take_focus` ignores the picker). Route the shown picker ahead of `banner_key`, or have `take_focus` return false while `modal_owns_keys()`. Test: `Tab` closes the picker with the cursor on the tip, `banner_focus` untouched, `c`/`s`/`a` typed into the query.
- **Major 2:** the still-to-apply block isn't in the app's layout mirrors (`update/geometry.rs:23-27` `rows_above_graph`; `update/state.rs:473-489` `graph_rows`; `update/picker.rs:256` reads the same). Write one function that mirrors `screen_rows`' pending height, used by both, like `preview_rows_taken`. Test: a click on a drawn row under the block selects that row. Granted: `geometry.rs`, `state.rs`, `update/picker.rs`.
- **Minor 1:** the stale comment at `keys.rs:343-344` (`o` and place mode).
- **Minor 7:** the edit-stop bar draws `c` bare and spends width on `r`/`m`, which refuse during a live op (`preview.rs:303-304`). Rank `c continue` above the graph's write keys while an operation is live. Re-take the edit-stop captures.

## NOTES (builder, 2026-09-28)

**Compacted once mid-ticket.** Nothing remains: every box below is done and the workspace gates were run after the compaction. Scout used: `scouts/07d.md`.

Ticks, each naming its test (all green in the final workspace run, 4741/4741 nextest + doc tests):
- [x] Box 1 (graft-ops): `rewrite_followers::progress_names_each_follower_with_author_and_count` (red first: `followers` empty), `no_followers_on_the_top_branch`, `no_followers_for_a_shell_rebase` (`crates/graft-ops/tests/it/rewrite_followers.rs`, real git).
- [x] Box 2: `update::banner::rewrite_stop_tests::pending_view_lists_this_branchs_rows_and_summarises_followers` (red first). 07c box-1 tests still green: graft-ui `still_to_apply::tests::{still_to_apply_is_newest_first_with_followers, still_to_apply_caps_at_six_then_more, no_block_when_nothing_is_pending}`, `graph_screen::tests::no_ghost_rows_in_the_graph`.
- [x] Box 3: `update::keys::edit_stop_tests::{space_on_the_stopped_row_stages_every_change, space_on_the_uncommitted_row_at_an_edit_stop_stages, space_on_a_merge_row_at_an_edit_stop_still_folds, space_outside_an_edit_stop_is_unchanged, space_with_the_banner_focused_is_the_banners}`. The two staging tests compare the command against `operate::stage_all` run on a copy, not re-described.
- [x] Box 4: `edit_stop_tests::the_bar_offers_space_stage_exactly_where_space_stages` (18 states; `space stage` drawn on 2: stopped row and Uncommitted row, graph focused, edit stop). Written after the bar change, not red first. Existing sweeps green: `update::tests::every_key_the_bar_draws_is_answered`, `every_swept_state_is_the_context_it_claims`, `keys::live_c_tests::the_bar_offers_c_exactly_where_c_acts`, `update::tests::space_folds_the_merge_under_the_cursor_and_unfolds_it`.
- [x] Box 5 walks: `m7_walk::rewrite_stops::edit_stop_amend_continue` (`space a c`, 3, fenced by `within`); `abort_from_a_chain_stop` (`esc a y`, 3, fenced). Fence row added at `m7-power-tools.md` after `edit-stop-amend-continue`, verbatim.
- [x] Box 6 captures: `capture_edit_stop_sizes` (187x50, 80x24) and the 120x40 one; `rewrite_stop_to_first_file` for the chain stop. `rewrite-chain-stop-187x50.txt` byte-identical (not in the diff). 80x24: 7 of his rows pending, 6 drawn, `+1 more`; follower row **drawn**. All ≤ 4 KB. Colour unverified.
- [x] Box 7: TESTING.md "M7: stops during a rewrite" rewritten, one entry per walk and capture.
- [x] Review **Major 1** (cause): a shown picker now claims keys before `banner_key` (`update/keys.rs` modal chain). `update::picker::tests::tab_in_the_picker_at_a_live_stop_is_the_pickers` (red first), `letters_in_the_picker_at_a_live_stop_are_typed`.
- [x] Review **Major 2** (cause): `banner::pending_rows_taken` mirrors `view::frame`'s pending carve; `geometry::rows_above_graph` and `AppState::graph_rows` add it (`update/picker.rs` reads `rows_above_graph`, so needed no edit). `m7_walk::rewrite_stops::a_click_under_the_block_selects_the_row_drawn_there` at 120x40 and 80x24 (red first).
- [x] Review **Minor 1**: `o` comment in `write_key` fixed. A comment: no test possible.
- [x] Review **Minor 7** (cause): `lists::said_live_on_the_graph` ranks `c` right after `⇥ banner` (and `a amend it`), ahead of the write keys. `graft-ui hints::tests::live_c_is_ranked_ahead_of_the_rows_write_keys` — written after the change, not run red; the old order is on record in the pre-change m6 capture bars (`⇥ banner  m merge  r rebase … c finish`). m5 scenario `hints_at_80_show_three_and_more` expectation updated to `⇥ banner  c continue  ⏎ changes`.

Findings:
- The ticket's Touches line said `edit_stop_lands`' 3 picks were one branch. They were three branches (a2, b1, c1); the test now asserts rows `["a2"]` + followers `feature-b 1`, `feature-c 1`.
- **Question (rule 7):** abort's status is `aborted: rebase feature-a onto main` (`update/banner.rs` `closing`, ~:1005), not the note's `✓ rewrite aborted · every branch is as before` (D3). The walk asserts today's words; not changed here.
- **Textual defect, open:** graft-ui `still_to_apply.rs` `follower_line` says `1 commit follow` (120x40 capture). Out of scope (no widget changes).
- `space` with nothing stageable does what `Ctrl+A` does: nothing, silently.
- At 120 wide the bar draws `space` and `⏎` bare (width ladder); `space stage` shows at 187.
- Harness: `App::focus_gained` added (`tests/it/app_harness.rs`) — the harness has no watcher, so the walk sends focus-gained after his edit, as the terminal would.
- `graph_key` hit clippy's cognitive-complexity cap; the view arms moved into `view_key` (same order: edit-stop `space` first, then `view_change`).
- m6 contact sheets regenerated (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`): three m6 captures changed bar order; sheets 3 and 4 were already stale from earlier tickets' capture changes and are current now.

Touches: `RewriteProgress` literals (grep, all updated: `update/tests.rs`, `rewrite_stop_tests.rs`, `view/tests.rs`); relaunch chain words (`a_relaunch_mid_rewrite_shows_the_chain` green); graph `space` elsewhere (fold tests green); bar (sweeps green, captures re-taken m6 ×3, m7 ×3); picker geometry (reads `rows_above_graph`; picker tests green); journal/undo (stage is `stage_all`'s entry, no new path); keymap (none edited); held files `operate.rs`, `preview.rs`, `running.rs`, graft-ops `preview.rs` not touched (diff).
