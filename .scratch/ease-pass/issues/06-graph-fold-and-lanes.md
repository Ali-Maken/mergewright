# graph-fold-and-lanes — merged branches folded by default, lanes only for what's on screen

Status: ready
Blocked by: none
Build with: (blank — brownfield; conductor sets after the scout). Planner suggests `builder-opus-xhigh`: layout-engine area; correctness is checkable by property tests, so not Fable.
Needs: none (draws nothing new itself; G draws roles and the opened rows' look).

## Read first
- `docs/design-notes/ease-pass.md`: Prior art rows "Too many lanes", "Lane count"; Hard-to-reverse choice 2; Mock behaviours G4, G10, G11, M18; Questions 10, 11, 13 (answered a) and 14 (open).
- `docs/design/03-graph-layout-algorithm.md` §3.4, §3.5, §3.6 last bullet. `docs/design/02-keymap.md` §2.3 rows `Space`, `+`, `1`.
- `crates/graft-graph/MAP.md`: `inclusion`, `fold`, `merge_name`, `layout::stream` (`reinclude`, `truncate`). `crates/graft/MAP.md`: `history`, `history::views`, `history::seeds`, `jobs::fold_count`, `update::fold_count`, `update::keys`, `view::rows`. `crates/graft-ui/MAP.md`: `graph_screen` (`Filters`, `FoldMark`, `HeaderView`).
- The scout's file.

## Scout questions (≤1 page)
- How today's opt-in merge fold is expressed (`Inclusion` folded merges? `FoldMark`?), what `space` on a merge does, and what `jobs::fold_count` counts.
- **Whether the provisional (pre-promotion) emitted order guarantees children before parents.** The predicate below needs it. If not: say what order it is; the builder stops and reports — the conductor brings it back as a question, not a workaround.
- Every place that seeks to a commit that could now be hidden: search `n`, palette commit find, `J` put-back landing, Map `⏎`, detail pane parent links, Ghost Preview slice, overview strip marks. What each does today with an opt-in folded target.
- Where `+N lanes` is drawn and what `Fold::plan` does with a column open but holding no node in the window.

## Scope
In (graft-graph `inclusion`, `fold`, `merge_name`; graft `history::*`, `update::keys` (`space`), `update::fold_count`, `view::rows`; graft-ui `graph_screen` header words and `FoldMark::words`):
- **Fold by default** as a streaming inclusion predicate (Hard choice 2): shown = first-parent closure of the keep tips (trunk, `HEAD`, integration branches) and of every ref tip that no shown merge reaches through a second parent, plus the second-parent side of each **opened** merge. Two bitsets filled as the walk emits (children before parents). `1` first-parent view unchanged.
- Folded merge row tail: `▸ N commit(s) from <branch>` (N from `fold_count`; `1 commit` singular). `<branch>` from the merge subject via `merge_name`; **Q13 a:** `Merged PR <n>: …` → `PR <n>` (`▸ 4 commits from PR 16021`); nothing recovered → `▸ N commits` alone.
- `space` on a folded merge → opens it (`▾` replaces `▸`), its commits appear on **their own lane curving into the merge (Q10 a)**; `space` on the opened merge or any opened row → closes it, cursor on the merge. Each opened row carries `OpenedFrom` (the merge and the label) for G: rows dim, branch column = the label's last path segment; with the cursor on an opened row, focus for roles = the merge row's branch (G4).
- **Lanes only for branches with a commit in view (G11):** `Fold::plan` drops every column with no node in the window before ranking (edges into a dropped column draw as today's stubs). The rail and `+` stay for overflow of node-holding columns.
- **Header (G10, Q11 a):** today's top-line frame; at its right end `lanes drawn: N of M` (N drawn, M open in the window; shown only while N < M) and `K merged branches folded` (folded merges in view; shown only while K > 0), replacing `+N lanes`. **Q14 (open):** whether `on <branch>` is added too — build without it; the line is one edit if he answers (b).
Out: role colours and shapes, opened rows' dimming and branch column (G), pixel tier (X), the branch pane (B).

## Signatures
graft-graph `inclusion`:
```rust
/// Merges whose merged side is shown. Empty = every merged branch folded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Opened { /* merge ids, ordered */ }
impl Opened {
    pub fn open(&mut self, merge: Oid) -> bool;    // false if already open
    pub fn close(&mut self, merge: Oid) -> bool;   // false if not open
    pub fn contains(&self, merge: Oid) -> bool;
}
impl Inclusion {
    /// Merged branches folded by default; `keep` = trunk, HEAD and integration tips.
    pub fn folded_by_default(keep: Vec<Oid>, opened: Opened) -> Self;
    pub fn opened(&self) -> Option<&Opened>;        // None for an opt-in (today's) inclusion
}
/// The streaming predicate. Fed in emitted order (children before parents).
pub struct FoldWalk { /* shown-by-first-parent bitset, behind-a-folded-merge bitset */ }
impl FoldWalk {
    pub fn new(keep: &[CommitHandle], ref_tips: &[CommitHandle], opened: &[CommitHandle]) -> Self;
    /// Whether `commit` is shown; records what its parents inherit.
    pub fn admit(&mut self, commit: CommitHandle, parents: &[CommitHandle]) -> bool;
}
```
graft-graph `fold`: `Fold::plan` keeps its signature; new `pub fn open_columns(&self) -> usize` (M of `lanes drawn: N of M`).
graft-graph `merge_name`: `recover_branch_name(&[u8]) -> Option<Vec<u8>>` unchanged; adds the `Merged PR <n>: ` pattern → `PR <n>`.
graft app (`view::rows`):
```rust
pub struct OpenedFrom { pub merge: Oid, pub label: Option<Vec<u8>> }
```
carried per row into `GraphEntry` (field name G's; G adds the field, this group fills a value it can test in `view::rows`).

## Touches
`Included`/`reinclude` and `truncate` (opening a merge invalidates from its row); history worker requests; `jobs::fold_count` cache; search, palette, `J` landing, Map `⏎`, detail links, Ghost Preview, overview strip (scout's list — each keeps today's behaviour for a hidden target or the builder reports it; no new behaviour invented); `+` and the rail; first-parent `1`; header captures; `02-keymap.md` §2.3 `Space` row wording (via V's Proposed edit, not here); `docs/design/03-graph-layout-algorithm.md` §3.5/§3.6 (Proposed edit text in NOTES, not applied).

## Acceptance
- [ ] Property test (graft-graph): over generated histories, `FoldWalk` shows exactly the first-parent closure of keep tips and unmerged ref tips; every hidden commit is behind some shown, unopened merge's second parent.
- [ ] Opening merge M shows exactly M's second-parent side down to the first shown commit; closing restores the previous row set (row ids equal).
- [ ] Fixture `main ← wallet-release ← sara/fix-toc` with merged PRs: first paint has no merged-branch commits; folded row reads `▸ 4 commits from PR 16021` (Q13) and `▸ 1 commit from sara/fix-toc` for a named merge.
- [ ] `space` on the folded row → `▾`, 4 rows appear on a lane that is not the merge's own column and joins the merge row (Q10, row-shape test); `space` on one of them → closed, cursor on the merge.
- [ ] Header: `lanes drawn: 5 of 8` and `3 merged branches folded` on the fixture at 120 cols; neither shown when nothing is dropped/folded; no `+N lanes`.
- [ ] No column without a node in the window is drawn (fold test over the fixture window).
- [ ] First paint and held-`j` budgets: the fast-tier frame tests that pin them still pass; the scout names them. Release numbers are the milestone review's.
- [ ] Captures `docs/shots/ease-pass/main-folded-120.txt`, `main-opened-120.txt` (next to `prototypes/ease-mocks/mergewright-main-screen.html` in the sheet). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~70k). Split point: predicate + `space` + `merge_name` here; header words and the no-node lane drop as a follow-on.
