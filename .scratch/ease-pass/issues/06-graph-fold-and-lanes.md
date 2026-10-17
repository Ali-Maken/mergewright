# graph-fold-and-lanes — merged branches folded by default, `space` opens one, the folded row says what's inside

Status: ready (rewritten 2026-10-17 after this group's scout, `.scratch/ease-pass/scouts/06-graph-fold-and-lanes.md`; split: lanes-on-screen and header words → **06b**, seeks to a folded commit → **06c**)
Blocked by: none
Build with: Opus (`builder-opus-xhigh`) — layout engine (inclusion, reinclude, promotion); correctness is checkable by property tests over generated and date-skewed histories, so not Fable.
Needs: none. Existing: `Inclusion` (`graft-graph/src/inclusion.rs:55-61`), `Included`/`reinclude` (:37; MAP `inclusion`), `StreamingLayout::truncate`, `EmittedOrder`, `CommitHandle`, `recover_branch_name` (`merge_name.rs:15-18`), `jobs::fold_count::count_folded` (`jobs/fold_count.rs:13-23`), `AppState::fold_counts`.

## Read first
- `docs/design-notes/ease-pass.md`: Prior art row "Too many lanes"; Hard-to-reverse choice 2 (as amended 2026-10-17); Mock behaviours G4, M18; Questions 10, 13.
- `docs/design/03-graph-layout-algorithm.md` §3.6 last bullet. `docs/design/02-keymap.md` §2.3 rows `Space`, `1`, and the "Proposed (ADR-0063) — fold default" paragraph. ADR-0008 (provisional order, promotion; its 2026-09-17 amendment).
- `crates/graft-graph/MAP.md`: `inclusion`, `merge_name`, `layout::stream` (`reinclude`, `truncate`, `EmittedOrder`). `crates/graft/MAP.md`: `history::views`, `history::stream`, `jobs::fold_count`, `update::fold_count`, `update::keys`, `view::rows`, `event_loop::rest`. `crates/graft-ui/MAP.md`: `graph_screen` (`FoldMark`, `Filters`).
- The scout's file.

## Decided here (scout contradictions resolved)
- **The provisional order is not children-first** (`graft-git/src/history/walk.rs:34-37`, `simple.rs:12-21`): date order, a backward clock can emit a parent before its child; strict topo only with a commit-graph or after promotion (`stream.rs:63-79`, ADR-0008). The fold is made correct under it this way:
  - **Shown wins, and only grows.** A commit's verdict is decided from the children already emitted. Every reason to show (a keep tip; a first parent of a shown commit; a second parent of an opened merge) is final. "Hidden" is the only verdict a late child can contradict.
  - **A late contradiction repairs itself** through the path opening a merge already uses: when a shown commit arrives whose first parent was already admitted hidden, `FoldWalk` reports that parent (`take_late`); the history adds it to `Inclusion`'s force-shown set and calls `reinclude`, which truncates from the first changed row and re-walks. Rare (a child dated before its parent), bounded (the force-shown set only grows).
  - **What's left before promotion is one benign error:** a merged branch's tip emitted before the merge that hides it stays shown (extra rows, never missing ones) until promotion, which re-lays out in topo order with a fresh `FoldWalk` and is exact. With a commit-graph (the common case, ADR-0008) the order is topo from the start and the fold is exact at first paint.
  - Why not "draw unfolded until promotion": every launch without a commit-graph would paint every lane, then jump. Why not "accept until promotion": a commit on his own branch would be missing from the picture — a wrong picture, which he rejected in ADR-0008's amendment (2026-09-17).
- **When a merge row folds:** only when something is hidden behind it — a second parent admitted hidden behind it. A sync merge (main into a feature) whose second parent is on main's line folds nothing and draws no mark. `FoldWalk::folds` is `None` until the merge's second parents are admitted; no mark is drawn until then (status rule: show only what's known).
- **Fold counts** (knock-on: today only `Space` asks, `update/fold_count.rs:1-9,29-39`): asked at rest (`event_loop::rest`) for every folding merge row in the drawn window with no cached count, one command per rest; same job (`rev-list --count M ^M^1` minus 1, octopus union once — what the merge brought in), same session cache. Until it lands the tail has no number; a failed count shows `?`.
- **Names:** `OpenedMerges` (not `Opened` — graft-ops already exports `conflict::Opened`) and `InsideMerge` (not `OpenedFrom` — the app's resolver already has `OpenedFrom::{Banner, Pane}`).
- **The opt-in fold retires:** `Inclusion.folded`, its `first_parent_only` use (`inclusion.rs:200-208`) and `Filters.folded` (header `N merges folded`, `view/rows.rs:141`) lose their only producer (`view_change.rs:28-38`). Remove them, or NOTES names what still reads them. 06b adds the new header words.
- **`space` on a non-merge row** that isn't inside an opened merge: nothing, as today (`view_change.rs:33-35`).

## Scope
In (graft-graph `inclusion`, `merge_name`; graft `history::*` (promotion builds a fresh `FoldWalk`; the late-repair loop), `update::keys` (`space`, `view_change.rs`), `update::fold_count`, `event_loop::rest` (the count ask), `view::entries` (fold mark), `view::rows` (`InsideMerge`); graft-ui `graph_screen` `FoldMark`):
- **Fold by default:** shown = first-parent closure of the keep tips (trunk, `HEAD`, integration branches from the seeds) and of every other ref tip (local and remote branches, tags; stashes keep their own inclusion) that no shown merge reaches through a second parent; plus the first-parent closure of each **opened** merge's second parents; plus force-shown. Everything else hidden. `1` (first-parent view) unchanged.
- **`FoldWalk` state rolls back with the stream's checkpoints** exactly as `Included`'s own walk state does today (`inclusion.rs:135-166`); if `Included` rebuilds from scratch on `reinclude`, so does `FoldWalk`.
- **Folded row tail** (`FoldMark::words`): `▸ 4 commits from PR 16021` · `▸ 1 commit from sara/fix-toc` · pending `▸ commits from …` · failed `▸ ? commits from …` · no name recovered: drop ` from …`. Opened: `▾` for `▸`, same words. ASCII tier `>` / `v`. Name = `recover_branch_name` on the merge subject, full; **Q13 a:** `Merged PR <n>: …` → `PR <n>`.
- **`space`** on a folding merge → `OpenedMerges::open`, `reinclude`; its side appears on **its own lane joining the merge (Q10 a)** — the layout's own placement of a second parent. On an opened merge → close. On a row inside an opened merge → close its innermost opened merge, cursor on that merge. On a folding merge that is itself inside an opened one → opens it (the merge rule wins). Opened merges live for the session, by id.
- **`InsideMerge`** per row (the innermost opened merge whose side shows the row, and that merge's recovered name) — G draws from it (dim rows, branch column, focus).
Out: lanes-on-screen and header words (06b); seeks to hidden commits (06c); row colours and the opened rows' look (07, G); pixel tier (13, X); the bar per row (G).

## Signatures
graft-graph `inclusion`:
```rust
/// Merges whose merged side is shown. Empty = every merged branch folded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenedMerges { /* merge ids, ordered */ }
impl OpenedMerges {
    pub fn open(&mut self, merge: Oid) -> bool;     // false if already open
    pub fn close(&mut self, merge: Oid) -> bool;    // false if not open
    pub fn contains(&self, merge: Oid) -> bool;
}
impl Inclusion {
    /// Merged branches folded by default; `keep` = trunk, HEAD and integration tips.
    pub fn folded_by_default(keep: Vec<Oid>, opened: OpenedMerges) -> Self;
    pub fn opened(&self) -> Option<&OpenedMerges>;
    pub fn opened_mut(&mut self) -> Option<&mut OpenedMerges>;
    /// Commits the provisional order hid too early; shown from now on. True if any was new.
    pub fn force_shown(&mut self, ids: impl IntoIterator<Item = Oid>) -> bool;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admit { Shown { inside: Option<CommitHandle> }, Hidden { behind: CommitHandle } }
/// The streaming predicate, fed in emitted order. Exact when the order is topological.
pub struct FoldWalk { /* builder's: dense per-handle state, ADR-0026 */ }
impl FoldWalk {
    pub fn new(keep: &[CommitHandle], ref_tips: &[CommitHandle], opened: &[CommitHandle], forced: &[CommitHandle]) -> Self;
    /// `inside`: the innermost opened merge whose side shows it. `behind`: the merge it hides behind.
    pub fn admit(&mut self, commit: CommitHandle, parents: &[CommitHandle]) -> Admit;
    /// Hidden commits a later-emitted shown child reaches by its first parent (date order only). Drains.
    pub fn take_late(&mut self) -> Vec<CommitHandle>;
    /// `Some(true)` once a second parent was admitted hidden behind `merge`; `Some(false)` once all were not; `None` before.
    pub fn folds(&self, merge: CommitHandle) -> Option<bool>;
    /// The merge a walked hidden commit sits behind; `None` if shown or not walked. (06c chains it.)
    pub fn behind(&self, commit: CommitHandle) -> Option<CommitHandle>;
}
```
graft-graph `merge_name`: `recover_branch_name(&[u8]) -> Option<Vec<u8>>` unchanged; adds `Merged PR <n>: ` → `PR <n>`.
graft-ui `graph_screen`:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldCount { Pending, Known(u32), Failed }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldMark { pub open: bool, pub count: FoldCount, pub from: Option<SanitizedText> }
impl FoldMark { pub fn words(&self, ascii: bool) -> String; }
```
graft app (`view::rows`):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsideMerge { pub merge: Oid, pub label: Option<Vec<u8>> }
```
carried per row into `GraphEntry` (G names the field; this group fills a value testable in `view::rows`). Counts from the job (`usize`) convert to `u32` saturating.

## Touches
`Included`/`reinclude`/`truncate` (opening and late repair invalidate from a row); promotion (`history::views`: fresh `FoldWalk`, selection re-anchors by id — an anchor now hidden lands on the merge it hides behind); history worker requests; `jobs::fold_count` cache and its new caller at rest; `FoldMark` readers (`graph_screen/mod.rs:404,411-430`, `view/entries.rs:367-376`), pixel strips if they read it; `Filters.folded` and header captures; `+` and the rail (unchanged here); first-parent `1`; `space` in §2.6a's edit stop ("on any other row §2.3's fold") — same handler, test; `02-keymap.md` §2.3 `Space` (V's Proposed edit, not here); ADR-0026 heap slope (per-handle `FoldWalk` state — say bytes/commit in NOTES; the review's memory run gates it).

## Acceptance
- [ ] Property test (graft-graph), generated histories in topo order: `FoldWalk` shows exactly the first-parent closure of keep tips, unmerged ref tips and opened merges' second parents; every hidden commit's `behind` chain ends at a shown, unopened merge.
- [ ] Property test, the same histories emitted in a date-skewed order (children dated before parents): after late repair, shown ⊇ exact; no commit on a keep tip's first-parent chain is hidden; re-walked in topo order (promotion), shown = exact.
- [ ] Opening merge M shows exactly M's second-parent side down to the first shown commit; closing restores the previous row set (row ids equal).
- [ ] Sync merge (main into a feature): `folds == Some(false)`, no mark drawn.
- [ ] Fixture `main ← wallet-release ← sara/fix-toc` with PR merges (branch deleted) and one named merge: first paint has no merged-branch commits; tails read `▸ 4 commits from PR 16021` (Q13) and `▸ 1 commit from sara/fix-toc`; before the count lands `▸ commits from PR 16021`; a failing count `▸ ? commits from PR 16021`.
- [ ] Counts are asked at rest for every folding merge in the window (job count test), once per merge per session.
- [ ] `space` on the folded row → `▾`, 4 rows on a lane that is not the merge's own column and joins the merge row (Q10, row-shape test); `space` on one of them → closed, cursor on the merge; `space` on a plain row → nothing.
- [ ] First paint and held-`j` fast-tier tests pass: `event_loop/tests/settle.rs:312`, `event_loop/tests/pace.rs:220`, `event_loop/tests/pixel_scroll.rs:192`. Release numbers are the milestone review's.
- [ ] Capture `docs/shots/ease-pass/main-folded-120.txt`, `main-opened-120.txt` (next to `prototypes/ease-mocks/mergewright-main-screen.html` in the sheet). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k. Split point: predicate + late repair + promotion here; `space`, words and counts as a follow-on.
