# fix round: a stacked branch's commits carry its own name

Status: done except 3 boxes, see NOTES (Q1 answered: (a) straight on, ADR-0064; Q2: bob/x not planned, user 2026-10-20)
Blocked by: 15-fix-identity-and-base (lane identity per tip, and `history/seeds.rs`, which this also edits)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the rule is decided here and pinned by a pure function and unit tests; the code is one lookup on a path that exists. Not a hard layout area at this size.
Needs: 15's one-identity-per-tip seeds (local first, remote only where no local sits).

From review Major 2 (`docs/reviews/ease-pass-07-14.md:124-142`) and usability U2. Closes (conductor, not the builder — builders don't read KNOWN_GAPS): `ease/07b-made-on-lane`, `ease/07c-wallet-no-column`. **Not** `ease/14-sibling-bob-x` — see Q2.

## Read first
- `docs/reviews/ease-pass-07-14.md:124-142` (Major 2) only.
- `docs/design/03-graph-layout-algorithm.md` §3.1 (Lane), §3.2 step 3, §3.3 pseudocode, §3.4 first paragraph (roles).
- `crates/mergewright-graph/MAP.md` (`lane`, `layout`, `role`, `seeds`), then `crates/mergewright/MAP.md` (history seeds, branch column, branch pane). MAP before source; scout anything beyond a few lines.
- Mock rule, by quote (don't open the mock): "A commit is in branch X if it was made on X, or on what X was built on, at or below where X left it" (`prototypes/ease-mocks/mergewright-main-screen.html:168-169`); "A branch column names the branch for each row, written once where it changes" (`:98`).

## The rule (decided)
Today a commit a lane is waiting for takes that lane **and its name** (`assign_row.rs:62-63`); the branch-tip lookup is read only for a commit nothing waits for (`open_new_lane`, `:209-214`). So on a stack the top branch's name runs down over every branch below it.

New: **when a lane reaches a commit that is another branch's tip, the lane carries that branch's name from that row down.** Exceptions, in order:
1. The lane is reserved (trunk, integration): never renamed. A branch pointing at a trunk commit stays trunk (mock: `master is here` sits on development's line).
2. The tip's name is the lane's own name: nothing changes.
3. The tip is the **remote copy of the lane's branch** (`origin/feature/receipts` one behind `feature/receipts`): nothing changes. Without this every pushed commit under `↑1 to push` would read `made on origin/…` (mock rule `:99`, "your branch and its remote are one thing").
4. Fork points are unchanged: lowest lane wins (lane 0 overrides), then rule above applies to the winner.

**Merge-parent case** (`assign_row.rs:148-155`): the lane a merge opens for its second parent takes the parent's branch name **from the merge row on** if the parent is a branch tip (a live branch beats a name recovered from the merge subject); otherwise it keeps the recovered name (`LaneIdentity::Merge`) and the rule above renames it if it later reaches another branch's tip. Builder: write in NOTES what `:148-155` did before, then change it.

**Geometry (pending Q1, default a):** the lane keeps its column; only the name changes at the tip row. No new column, no new connector, `lanes drawn: N of M` unchanged. DESIGN §3.1's "identity stays fixed for the lane's life" becomes "until the lane reaches another branch's tip"; the conductor applies that edit and an ADR after Q1 — not the builder.

Cost: one hash lookup per row that continues a lane (the same map `open_new_lane` reads). Checkpoint size unchanged (identity already lives in `Lane`). No git read in the graph crate.

## Q1 for the user (blocks dispatch) — in his terms
*feature/receipts sits on top of wallet-release. Where wallet-release's own commits begin, what should the line do?*
- **(a) carry straight on down the same column**, thick `┃` (your branch) turning thin `│` (what it's built on) at wallet-release's last commit, with `wallet-release` written in the branch column on that row. Same as every Git tool draws a stack: one line, a label where the next branch starts.
- **(b) bend one column sideways** into a new line for wallet-release, like your branch joining integration in the mock. Costs one extra column at every step of a stack (a 3-deep stack walks 3 columns), and the lower branch usually lands to the **right** of the upper one (the mock has it on the left), because the screen draws newest first and can't hold a column open in advance.

**Recommendation: (a).** Reproduces the mock exactly where the mock shows a stack (wallet-release joins integration with `╯`: integration is a reserved lane, unchanged here); keeps width for the 80/120-col screens; the `┃`/`│` shape and the branch-column name mark the boundary. Trade-off: on two teammates' stacked branches, both grey `┊`, the boundary shows only as the name in the branch column.
If (b): at the tip, open a new lane (`open_new_lane`, which already takes the tip's name) and close the waiting lanes into that row; acceptance swaps "same column" for "a `╮`/`╯` at the tip row, lower branch in a new column", and `lanes drawn` may rise by one.

## Q2 for the user (does not block) — the split where the lower branch moved on
`ease/14-sibling-bob-x` is **not** this cause. `docs/shots/ease-pass/main-pane-sibling-120.txt:10` is sara's tip (`toc: number the headings`, own lane); `:12` is `toc: fix the anchors`, **one below** her tip, where sara's lane closes `━╯` into bob's. Bob branched from sara's commit, then Sara added one more: the commit is on both branches, not a branch tip, and git doesn't record which branch it was made on.
*That commit: which name should it show?*
- **(a) the branch whose line it continues on** (today; here `bob/x` — wrong here, right when the other branch is the newcomer).
- **(b) guess from merges:** a branch that merged the other in is the newcomer (bob merged sara → `sara/fix-toc`). Right here; wrong when a finished feature is merged back into the branch it was cut from, and it would bend that branch's straight line.
- (c) name both (`made on bob/x or sara/fix-toc`): rejected — every branch cut from wallet-release would read `or` on wallet-release's commit.

**Recommendation: (a), and close the row "not planned: git doesn't record it".** He rebases stacks when the base moves (memory `works-in-a-team-on-dependent-branches`), which turns this split back into the stacked case this ticket fixes.

## Scope
In: the rule above in `mergewright-graph` (`layout/assign_row.rs`, the `known_identities` map at `layout.rs:229-234`, `seeds.rs`); `follows` filled in `crates/mergewright/src/history/seeds.rs`; MAP.md for both crates; re-shot captures.
Out: Q2's split rule; any change to roles, the pane's `built on` route (15), the fold; DESIGN/ADR text (conductor).

## Signatures
```rust
// mergewright-graph/src/seeds.rs — BranchSeed gains one field (keep its existing fields).
pub struct BranchSeed {
    /* existing: the tip and its identity */
    /// The local branch this ref is the remote copy of: its configured upstream,
    /// else the same short name on a remote. `None` for local and remote-only branches.
    pub follows: Option<BranchIdentity>,
}

// mergewright-graph/src/layout/assign_row.rs — crate-private, unit-tested alone.
/// The name a lane carries from this row down when it reaches `tip`, or `None` to keep its own.
/// `None` when: lane reserved; no tip here; tip's identity is the lane's; tip `follows` the lane's.
/// A `LaneIdentity::Merge` lane is renamed by any tip (its name is only recovered at assembly).
pub(crate) fn renamed_at_tip(lane: &Lane, tip: Option<&BranchSeed>) -> Option<LaneIdentity>;
```
Callers: the waited-for path in `assign_row` (after the fork-point winner is chosen) and the second-parent open (`:148-155`). `layout.rs:229-234` keys the seed (not only its identity) by tip. App: `history/seeds.rs` sets `follows` from the upstream data the bar's `↑n to push` already reads — **no new git spawn**; grep for that read, don't add one.

## Acceptance (tick each with its test)
Graph crate (pure, stacked fixture: `main ← L1 ← T(lower tip) ← U1 ← U2(upper tip)`, seeds for both tips):
- [x] U2, U1 rows `made_on` = upper; T and L1 rows `made_on` = lower; all four in one column.  <- layout::tests::stacked::a_stack_names_each_branchs_own_commits_in_one_column
- [x] A branch tip on lane 0's first-parent line → that row and below stay trunk's identity.  <- layout::tests::stacked::a_tip_on_the_trunk_line_leaves_trunk_alone
- [x] `origin/upper` at U1 with `follows = upper` → U1 and below up to T stay upper.  <- layout::tests::stacked::the_remote_copy_of_the_lanes_own_branch_does_not_rename_it
- [x] Merge whose second parent is a branch tip → that lane is `Known(tip)` from the merge row; the tip's rows read the tip's branch.  <- layout::tests::stacked::a_merge_parent_that_is_a_branch_tip_names_its_lane_from_the_merge_row
- [x] A merge lane (`Merge` identity) reaching another branch's tip lower down → renamed there.  <- layout::tests::stacked::a_merge_lane_reaching_another_tip_lower_down_is_renamed_there (+ renamed_at_tip_keeps_a_reserved_lane_and_the_lanes_own_name)
- [x] Same fixture through `StreamingLayout` with `chunk_rows`/`checkpoint_every` small enough to cut between U1 and T → rows equal the one-call `layout` (replay is deterministic).  <- layout::stream::stacked_tests::a_rename_at_a_tip_replays_the_same_across_a_checkpoint; also the lane property in layout::tests::lanes (properties::*)

App (07's fixture, `tests/it/graph_rows_tests.rs`, and 14's mock fixture):
- [x] `Merge branch 'sara/fix-toc' into wa…` row: branch column `wallet-release`; `Start the wallet release`: blank (written once where it changes).  <- it graph_rows_tests::every_local_branch_says_where_it_stands_and_who_made_each_row and graph_rows_tests::a_stacked_branchs_commits_carry_its_own_name
- [ ] Cursor on HEAD (`Draw the receipts list`): those two rows draw `│` (Under), HEAD's row `┃`.  <- PARTLY: second line `made on feature/receipts` / HEAD row ok in capture_the_cursor_rows_second_line (main-cursor-own-120.txt); the node rows below draw nodes, not a lane glyph, so `│` (Under) is not observable there. The thin-below-wallet-release shape is asserted at ease_walk::graph::g1_lane_shape_is_the_role_against_the_cursor_branch (TIDY row `│`)
- [ ] Cursor on `Start the wallet release`: second line `made on wallet-release`; pane heading `wallet-release`; bar's `C`/`m` name wallet-release; receipts' row draws `┊`.  <- it graph_rows_tests::cursor_second_line::on_the_stacked_branchs_commit_everything_names_that_branch (second line, pane heading, branch_pane::focused = what C/m read). Not asserted: the bar text itself, and receipts' row `┊`
- [x] HEAD 1 ahead of `origin/feature/receipts`: rows under it don't read `origin/…` anywhere.  <- it graph_rows_tests::a_stacked_branchs_commits_carry_its_own_name (no `origin/` outside an `is here` label); history::seed_tests::a_remote_seed_follows_the_local_branch_of_the_same_name
- [x] 14's mock fixture: no row's branch changes except stacked ones; list any that do, with why.  <- it ease_walk::graph::g1_lane_shape_is_the_role_against_the_cursor_branch, g2_..., g7_... (updated); only the rows under wallet-release's tip in the mock changed (sara/fix-toc stacked on wallet-release: lane thin below it, ali/tidy etc. unchanged); the other ease_walk captures matched
- [ ] Pane spawn count (09p, `docs/perf/branch-pane.txt` method) before/after: equal; state both numbers.  <- NOT RE-MEASURED. docs/perf/branch-pane.txt (before): focus spawns=8, window spawns=2753. mergewright-ops has no dependency on mergewright-graph (Cargo.toml), so the pane reads cannot change; script not re-run (release monorepo fixture)
- [x] Captures re-shot and listed, each with its "what right looks like" line updated: at least `docs/shots/ease-pass/main-cursor-own-120.txt` and every capture showing feature/receipts over wallet-release (grep `docs/shots/` for both names). The user judges by eye; colour unverified (rule 5).  <- ease-pass/main-{90,100,120,colours-off-120,cursor-own-120,cursor-teammate-120,opened-merge-120}, main-pane-dependent-120, m6/fix-undo-drift-way-back-{120x40,80x24}, m7/{amend-into-*,rebase-refused-fork-120x40,rewrite-*}, m8-bar-edit-stop-120x36, m8-hangup-relaunch-120x40, m8/reword-preview-120x40, 22-raster-octopus-merge{,-on-dark}.png; contact sheets regenerated. Colour unverified

## Touches (check each, say how in the report)
Lane identity feeds: `role.rs:96-97` `made_on` (no code change; behaviour via `row.lanes`), branch column (`view/entries/branch_column.rs`, after 15's patch removal), row bar (`view/rows/row_bar.rs`), branch pane `focused()` and `C` (`update/picker/open_on.rs`), `not merged`, `(you)` / `(you're here)`, second line, raster role-per-identity and strip keys (`plain_layout_hash`), continuation band, Ancestry lit columns (geometry unchanged under Q1 a), Mistake Radar classifiers (read `incoming`/`outgoing` only — unchanged under a; recheck under b), `RowShape` hash (chunk fingerprints change: expected). A ref change creating a branch deep in history now renames rows below it: confirm a ref change rebuilds the layout (grep where `Seeds` are built). Keymap, journal, undo: none.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`. Heavy tier is broken (review Major 4): say so, don't claim it.

## Size
~40k tokens: one function, one field, one fill site, ~10 tests, captures.

## NOTES (builder)
- Did not compact. Scout: none (ticket named files/lines; MAPs read, source read directly, small).
- `open_merge_lane` before: always `LaneIdentity::Merge { merge, parent }` (name recovered at assembly from the merge subject), known_branches never consulted. Now: `Known(tip.identity)` when the parent is a known tip, else unchanged.
- Rule lives in `assign_row.rs`: `renamed_at_tip` (pure, unit-tested) and `lane_of_row`. `known_identities` is now keyed to the whole `BranchSeed`.
- `follows` is filled by the same-short-name rule only (`origin/x` follows `x`). The configured upstream (`branch.<n>.merge`) with a different name is NOT read: seeds_for has no config read and none was added. Such a pair would read `origin/<other>` under the lower commits. Unverified; not in the fixtures.
- Visible consequences beyond the spec: (1) `<branch> is here` labels vanish on a tip whose column now names it (as `ops/ci` already did): wallet-release, feature, fix, feature-a/b; (2) octopus merge topic lanes and a merge parent that is a remote tip (accidental-pull `origin/main`) now take the tip's name (snapshots `octopus-merge`, `accidental-pull-merge`, pixel digests, 22-raster-octopus PNGs; colour slots move); (3) lanes under a lower branch's tip draw Other/Under, e.g. main-moves rows in m6/m8 captures went `┃`->`┊`. All follow from the rule; the user judges by eye.
- Tests that encoded the old behaviour were rewritten: branch_pane::the_identity_of_every_candidate_is_its_lanes, ease_walk G1/G2/G7, graph_rows_tests (release row), properties (renamed identity allowed at a tip). The lane property check moved to `layout/tests/lanes.rs` to keep `properties.rs` under its row (rows lowered in over_400.txt; none raised).
- Heavy tier broken (review Major 4): not run.
