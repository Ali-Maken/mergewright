# graph-fold-and-lanes — merged branches folded by default, `space` opens one, the folded row says what's inside

Status: **partial** (2026-10-17) — engine done (graft-graph: predicate, late repair, rejoins, promotion-ready); the app wiring is not started and the app does not fold yet. Stopped at the Size section's split point after a compaction; see NOTES and Handoff. (Rewritten 2026-10-17 after this group's scout, `.scratch/ease-pass/scouts/06-graph-fold-and-lanes.md`; split: lanes-on-screen and header words → **06b**, seeks to a folded commit → **06c**)
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
- [x] Property test (graft-graph), generated histories in topo order: `FoldWalk` shows exactly the first-parent closure of keep tips, unmerged ref tips and opened merges' second parents; every hidden commit's `behind` chain ends at a shown, unopened merge. — `inclusion::fold_walk::tests::properties::in_topological_order_the_fold_is_exact` (also: no late commit; `trim` = parents ∩ shown once rejoins are drawn; `folds`, `inside`). Streaming = slice form under fold mode: `inclusion::tests::streaming_a_view_matches_laying_out_its_slice`, `layout::tests::properties::every_invariant_holds_under_every_inclusion` (`viewed()` now generates fold-by-default with random opened merges).
- [x] Property test, the same histories emitted in a date-skewed order (children dated before parents): after late repair, shown ⊇ exact; no commit on a keep tip's first-parent chain is hidden; re-walked in topo order (promotion), shown = exact. — `inclusion::fold_walk::tests::properties::in_date_skewed_order_late_repair_never_hides_a_shown_commit` (the rule), and streamed through `Included` + `advance_folded`: `inclusion::tests::fold_by_default::a_date_skewed_walk_streams_every_commit_the_exact_fold_shows`. Both checked non-vacuous by mutation (dropping `late.push` / ignoring late in `Included::repair` fails them).
- [x] Opening merge M shows exactly M's second-parent side down to the first shown commit; closing restores the previous row set (row ids equal). **Engine only** — the app's `space` path is box 7. — `inclusion::tests::fold_by_default::opening_a_merge_invalidates_only_the_rows_from_the_merge_down` (fixture: exactly 101, 102 appear under merge 19; close = original rows), `inclusion::tests::fold_by_default::open_then_close_returns_the_original_rows` (proptest: open = slice form, ids above M unchanged, close = original rows), exactness of the side by `in_topological_order_the_fold_is_exact` (random opened mask).
- [ ] Sync merge (main into a feature): `folds == Some(false)`, no mark drawn. **Half:** `folds == Some(false)` — `inclusion::fold_walk::tests::a_sync_merge_folds_nothing`; "no mark drawn" is app-side, not built.
- [ ] Fixture `main ← wallet-release ← sara/fix-toc` with PR merges (branch deleted) and one named merge: first paint has no merged-branch commits; tails read `▸ 4 commits from PR 16021` (Q13) and `▸ 1 commit from sara/fix-toc`; before the count lands `▸ commits from PR 16021`; a failing count `▸ ? commits from PR 16021`.
- [ ] Counts are asked at rest for every folding merge in the window (job count test), once per merge per session.
- [ ] `space` on the folded row → `▾`, 4 rows on a lane that is not the merge's own column and joins the merge row (Q10, row-shape test); `space` on one of them → closed, cursor on the merge; `space` on a plain row → nothing.
- [ ] First paint and held-`j` fast-tier tests pass: `event_loop/tests/settle.rs:312`, `event_loop/tests/pace.rs:220`, `event_loop/tests/pixel_scroll.rs:192`. Release numbers are the milestone review's.
- [ ] Capture `docs/shots/ease-pass/main-folded-120.txt`, `main-opened-120.txt` (next to `prototypes/ease-mocks/mergewright-main-screen.html` in the sheet). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k. Split point: predicate + late repair + promotion here; `space`, words and counts as a follow-on.

## NOTES (builder, 2026-10-17)
- **Compacted once mid-ticket.** Stopped at the split point above, engine side: graft-graph is done and tested; **nothing in the app folds yet** (`View::default()` unchanged). Remaining work is under Handoff; the conductor splits it.
- **Scout used:** `.scratch/ease-pass/scouts/06-graph-fold-and-lanes.md` (given), plus `.scratch/ease-pass/scouts/06-app-wiring.md`, which I had a scout write for the app half (committed for the follow-on).
- **Deviations from Signatures:**
  - **No `Inclusion::force_shown`.** The forced set lives in `Included` (`forced`), not in `Inclusion`. Two reasons. The worker reincludes whenever `layout.source().inclusion() != &view.inclusion` (`crates/graft/src/history/views/mod.rs:164`), so a forced set held in the layout's inclusion would be undone on the next request. And forcing is a fact of this walk's order, not of the view: promotion builds a new `Included` and must start without it. Repair goes through `Included::repair` / `repair` / `advance_folded`, not `reinclude`.
  - **`FoldWalk::new(keep, opened, forced)` has no `ref_tips`.** In topo order a ref tip that no shown merge reaches through a second parent is exactly a commit nothing hidden points at, i.e. one admitted unseen, which shows as a tip. So the list is redundant.
  - **Added beyond the spec.** Keep and opened ids have no handle until the walk names them, so `Mark` / `FoldWalk::mark` were added. A merge can be emitted before main's line says its second parent is shown (sync merges, common in topo order), so `trim`, `take_rejoined` / `rejoin` / `rejoins` were added: the dropped edge is drawn again from the merge's row by rewinding what is served, with no re-decision. Also added: `inside`, `approximate_bytes`, `Included::fold`, `Inclusion::keep` / `keep_mut`, `OpenedMerges::iter`, and `Folding` (boxed). `folded_by_default(keep, …)`: `keep` holds the tips kept **besides** the seeds' default and integration branches, which are always kept, so the app passes `HEAD`.
  - **Legacy opt-in `Inclusion.folded` is kept** (the spec allows "or NOTES names what still reads them"). This leaves the app working unchanged until the follow-on flips the default. Readers: `crates/graft/src/update/keys/view_change.rs:32,36`, `update/keys.rs:449`, `view/entries.rs:375`, `view/rows.rs:141`; tests `update/tests/graph_toggles.rs:12`, `update/keys/edit_stop_tests.rs:115,135`. The follow-on retires it, along with `Filters.folded`.
- **Memory (ADR-0026):** `FoldWalk` costs about 5.75 B/commit dense: a `u8` and a `u32` per handle, plus six 1-bit `HandleSet`s. On top of that come sparse maps (nesting of opened merges, waiting second parents, dropped edges outside the 2-parent shape) and the `rejoins` list. This is paid only while folding. **Unverified:** not measured; the review's memory run gates it.
- **Cost of repairs:** each `repair` truncates to the earliest late commit or rejoined merge and re-lays out from there. A rejoin re-lays out from the sync merge's row down to the frontier. **Unverified** on a real repo. The follow-on's budget tests (`settle.rs:312`, `pace.rs:220`, `pixel_scroll.rs:192`) are where it shows first.
- **Knock-on: `AppState` sits at a clippy cliff.** At main it was 8176 B (measured with a throwaway `size_of` test). A 2-element array of `(&str, AppState)` is then exactly the 16384 B `large_stack_arrays` bound. `Inclusion` grew 8 B (`fold: Option<Box<Folding>>`), which tripped 3 test sites; I converted them to `vec!`: `update/banner/focus_keys.rs:321`, `update/remote/tests.rs:62`, `view/tests.rs:4775`. Any builder adding a field to `AppState` would have hit this; it no longer can at these sites. The root cause (an 8 KB inline `AppState`) is out of scope.
- **Knock-on: `merge_name`.** Azure `Merged PR <n>: …` now recovers `PR <n>`. Those merges' lanes take `BranchIdentity::of_name("PR <n>")` instead of `of_short_sha(parent)` (`crates/graft-graph/src/row.rs:140-142`), so their lane colour changes. No fixture or capture has that subject (grep `Merged PR`: only `merge_name` and its tests).
- **File sizes (PROMPT 6.2):** production files were split to stay under 400 lines: `fold_walk.rs` + `fold_walk/pointing.rs`, `included.rs` + `included/redecide.rs`. Test rows were raised or added in `crates/graft/tests/it/over_400.txt`.
- **Tests written after the code** (all in `inclusion::tests::fold_by_default`):
  - `a_folding_merge_streams_its_first_parent_only`, `a_merged_branch_folds_into_its_merge_down_to_the_fork_point`, `opening_a_merge_invalidates_only_the_rows_from_the_merge_down` and `a_date_skewed_walk_streams_every_commit_the_exact_fold_shows` were green on first run. The last was then checked by mutation.
  - `open_then_close_returns_the_original_rows` first failed on an assertion of mine, not on the code. A row above an opened merge can redraw: an unopened merge whose second parent the opening showed gets its edge back. So it now compares ids above the merge, not whole rows.

## Handoff (the app half — not started)
In order. Line refs are from `.scratch/ease-pass/scouts/06-app-wiring.md`; re-check them, since 03b ran alongside.
1. **Turn the fold on and lay out with repairs.**
   - `View`'s default inclusion becomes `Inclusion::folded_by_default(vec![HEAD], OpenedMerges::default())` (`history/state.rs:62-71, 271`). HEAD goes into `keep_mut()` when refs arrive, next to `with_stash_entries` (`update/tree_answers.rs:67,193`). A keep change redecides from HEAD's position.
   - Replace `advance` with `advance_folded` at `history/serve.rs:190`, `history/views/mod.rs:126`, `history/views/promotion.rs:146,226`.
   - **Gotcha:** plain `advance` on a folding layout leaves late and rejoined work pending, so rows are wrong (missing commits, missing sync-merge edges).
2. **Per-row fold facts to the UI.** `Window` (`serve::answer`, `serve.rs:166+`) carries no per-row payload. Add one, built from `layout.source().fold()`: `folds(handle)`, `inside(handle)` and the merge's name.
3. **`space`** (`update/keys/view_change.rs`): open a folding merge or close an opened one. On a row `inside` an opened merge, close the innermost one and move the cursor to the merge. On a plain row, do nothing. Retire `Inclusion.folded` and its readers listed above, plus `Filters.folded` (`graft-ui/src/graph_screen/mod.rs:298,316`; tests `graph_screen/tests.rs:948,957`; `folded: 0` in journal tests per the scout). The edit stop's `space` takes the same handler (`keys.rs:379-385`).
4. **Promotion re-anchor:** an anchor that is now hidden lands on its `behind` merge (`history/scroll/apply.rs:113-170`). Promotion already starts a fresh `FoldWalk` with nothing forced, because `open_layout` builds a new `Included`.
5. **Counts at rest:** for every folding merge in the window with no cached count, ask once per merge per session, one command per rest (`Turn::rested`, `event_loop/running/turn.rs:364` → `AppEvent::Rested`, `update/mod.rs:229-247`; `update/fold_count.rs:30`).
6. **Words and marks:** `FoldCount` / `FoldMark` / `words` in graft-ui `graph_screen` per Signatures (`FoldMark` today at `graph_screen/mod.rs:404-429`), `view::entries` fold mark, `view::rows` `InsideMerge`.
7. **Then the checks:** boxes 4 (no mark) to 9, the three budget tests, the two captures, and the MAP.md updates for graft and graft-ui.
