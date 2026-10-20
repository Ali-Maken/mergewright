# fix round: a stacked branch's commits carry its own name

Status: ready (Q1 answered: (a) straight on, ADR-0064; Q2: bob/x not planned, user 2026-10-20)
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
- [ ] U2, U1 rows `made_on` = upper; T and L1 rows `made_on` = lower; all four in one column.
- [ ] A branch tip on lane 0's first-parent line → that row and below stay trunk's identity.
- [ ] `origin/upper` at U1 with `follows = upper` → U1 and below up to T stay upper.
- [ ] Merge whose second parent is a branch tip → that lane is `Known(tip)` from the merge row; the tip's rows read the tip's branch.
- [ ] A merge lane (`Merge` identity) reaching another branch's tip lower down → renamed there.
- [ ] Same fixture through `StreamingLayout` with `chunk_rows`/`checkpoint_every` small enough to cut between U1 and T → rows equal the one-call `layout` (replay is deterministic).

App (07's fixture, `tests/it/graph_rows_tests.rs`, and 14's mock fixture):
- [ ] `Merge branch 'sara/fix-toc' into wa…` row: branch column `wallet-release`; `Start the wallet release`: blank (written once where it changes).
- [ ] Cursor on HEAD (`Draw the receipts list`): those two rows draw `│` (Under), HEAD's row `┃`.
- [ ] Cursor on `Start the wallet release`: second line `made on wallet-release`; pane heading `wallet-release`; bar's `C`/`m` name wallet-release; receipts' row draws `┊`.
- [ ] HEAD 1 ahead of `origin/feature/receipts`: rows under it don't read `origin/…` anywhere.
- [ ] 14's mock fixture: no row's branch changes except stacked ones; list any that do, with why.
- [ ] Pane spawn count (09p, `docs/perf/branch-pane.txt` method) before/after: equal; state both numbers.
- [ ] Captures re-shot and listed, each with its "what right looks like" line updated: at least `docs/shots/ease-pass/main-cursor-own-120.txt` and every capture showing feature/receipts over wallet-release (grep `docs/shots/` for both names). The user judges by eye; colour unverified (rule 5).

## Touches (check each, say how in the report)
Lane identity feeds: `role.rs:96-97` `made_on` (no code change; behaviour via `row.lanes`), branch column (`view/entries/branch_column.rs`, after 15's patch removal), row bar (`view/rows/row_bar.rs`), branch pane `focused()` and `C` (`update/picker/open_on.rs`), `not merged`, `(you)` / `(you're here)`, second line, raster role-per-identity and strip keys (`plain_layout_hash`), continuation band, Ancestry lit columns (geometry unchanged under Q1 a), Mistake Radar classifiers (read `incoming`/`outgoing` only — unchanged under a; recheck under b), `RowShape` hash (chunk fingerprints change: expected). A ref change creating a branch deep in history now renames rows below it: confirm a ref change rebuilds the layout (grep where `Seeds` are built). Keymap, journal, undo: none.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`. Heavy tier is broken (review Major 4): say so, don't claim it.

## Size
~40k tokens: one function, one field, one fill site, ~10 tests, captures.
