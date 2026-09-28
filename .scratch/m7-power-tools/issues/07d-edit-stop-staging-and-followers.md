# 07d — remainder of 07c: `space` stages at an edit stop, the followers row, the walks' fence, the captures

Status: ready (planner spec-only pass, 2026-09-28; filed by conductor, after 08-a). Remainder of 07c (its boxes 1, 4, 5 unticked; NOTES rounds 1–3).
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
- [ ] graft-ops, real git, a 3-branch line (his 4, feature-b by Sara 2, feature-c by Lina 1), edit stop on his row 2:
  `progress_names_each_follower_with_author_and_count` — followers == `[(feature-b, Sara, 2), (feature-c, Lina, 1)]`,
  `pending_here().len() == 2`; `no_followers_on_the_top_branch` (stop in feature-c's row); `no_followers_for_a_shell_rebase`
  (shell `rebase -i --update-refs`: `branch` None, followers empty).
- [ ] graft: `pending_view_lists_this_branchs_rows_and_summarises_followers` — no follower's sha7 among the rows; the
  07c box-1 tests still pass (name them at tick time).
- [ ] `space_on_the_stopped_row_stages_every_change`, `space_on_the_uncommitted_row_at_an_edit_stop_stages`
  (each: the command sent equals `Ctrl+A`'s, compared, not re-described); `space_on_a_merge_row_at_an_edit_stop_still_folds`;
  `space_outside_an_edit_stop_is_unchanged` (Uncommitted row and a merge row, no stop); `space_with_the_banner_focused_is_the_banners`.
- [ ] `the_bar_offers_space_stage_exactly_where_space_stages` (sweep: edit stop / conflict stop / no stop × stopped row /
  Uncommitted row / other row × graph / banner focus: bar draws `space stage` iff `space_stages_here`); the existing
  sweeps `every_key_the_bar_draws_is_answered`, `the_bar_offers_c_exactly_where_c_acts`, the m1 fold and M7/spacefold
  tests all green (named at tick time).
- [ ] **Walks** (`m7_walk`, counted, fail over the fence):
  - `edit_stop_amend_continue`: from the landing (cursor on the stopped row), fs write to a tracked file, wait for the
    working-tree read, then `space a c` = 3 ≤ `noted_keys("edit-stop-amend-continue")`. After `space`: `git diff
    --cached` names the file. After `a`: still stopped, HEAD's tree has the edit, same message. After `c`: no
    `rebase-merge/`, his branch's commit has the edit, feature-b on top of his new tip.
  - `abort_from_a_chain_stop`: route `esc a y`, fenced ≤ `noted_keys("abort-from-a-chain-stop")`; every branch at its
    old oid, HEAD on his branch, the status line D3 names.
- [ ] **Captures** (colour unverified), each ends in `docs/shots/m7/` ≤ 200 KB, under its Right line in the sheet:
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
- [ ] TESTING.md "Stops during a rewrite": one entry per walk and capture above, saying what right looks like in the
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
