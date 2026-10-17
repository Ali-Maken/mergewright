# graph-fold-and-lanes — merged branches folded by default, `space` opens one, the folded row says what's inside

Status: **partial, closed for building** (2026-10-17). Engine done (06); app wiring done but unreconciled (continuation 1, branch `worktree-agent-acb2cc0f5685a9e78` @ 7a4c11f0, unmerged). The remainder is split into **06d** (parents shape, HEAD keep, untested paths), **06e** (integration tests, boxes 5, 7, 8) and **06f** (captures, box 9, MAPs). See "Decisions for continuation 2" at the end. Earlier splits: lanes-on-screen and header words → **06b**; seeks to a folded commit → **06c**. (Rewritten 2026-10-17 after this group's scout, `.scratch/ease-pass/scouts/06-graph-fold-and-lanes.md`.) This file is the branch's ticket plus the decisions. If the branch's copy conflicts on merge, take this one.
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
- [x] Sync merge (main into a feature): `folds == Some(false)`, no mark drawn. — engine: `inclusion::fold_walk::tests::a_sync_merge_folds_nothing`; app: `view::rows::fold::tests::no_tail_on_a_sync_merge_a_plain_commit_or_a_row_the_fold_has_no_fact_for` and `the_graph_draws_each_folding_merge_with_its_tail` (the sync merge's drawn row has no tail). The app half reads hand-built fold facts, not a walked repo.
- [x] Fixture `main ← wallet-release ← sara/fix-toc` with PR merges (branch deleted) and one named merge: first paint has no merged-branch commits; tails read `▸ 4 commits from PR 16021` (Q13) and `▸ 1 commit from sara/fix-toc`; before the count lands `▸ commits from PR 16021`; a failing count `▸ ? commits from PR 16021`. **Half:** the words and forms — `view::rows::fold::tests::{a_folding_merge_tails_with_its_count_and_the_name_its_subject_gives, the_count_is_pending_until_it_lands_and_failed_when_it_failed, the_graph_draws_each_folding_merge_with_its_tail}`, graft-ui `graph_screen::tests::fold_tail::*`. Not built: the fixture as a real repo through the worker ("first paint has no merged-branch commits" is proven engine-side only). **→ 06e.** **06e:** `fold_by_default::first_paint_folds_merged_branches_and_the_tails_name_them` — the real repo through the job pool: first window and frame hold no merged-branch commit, a fold fact per row, both merges `folds == Some(true)`, tails `▸ 4 commits from PR 16021` and `▸ 1 commit from sara/fix-toc`.
- [x] Counts are asked at rest for every folding merge in the window (job count test), once per merge per session. — `update::fold_count::tests::every_folding_merge_on_screen_is_counted_one_job_each` (jobs = [first, second]), `a_folding_merge_on_screen_has_its_count_asked_at_rest_once`, `no_count_is_asked_mid_burst`, `opening_and_closing_asks_no_second_count`, `a_counted_fold_is_not_recounted_on_refresh_or_open_and_close`, `a_failed_fold_count_says_so_once`. "In the window" built as "drawn on screen" (`top..top+graph_rows`).
- [x] `space` on the folded row → `▾`, 4 rows on a lane that is not the merge's own column and joins the merge row (Q10, row-shape test); `space` on one of them → closed, cursor on the merge; `space` on a plain row → nothing. **Half:** `update::tests::graph_toggles::{space_opens_a_folding_merge_and_closes_it_again, space_inside_an_opened_merge_closes_it_and_lands_on_the_merge, space_off_a_merge_changes_nothing}`, edit stop `update::keys::edit_stop_tests::space_on_a_merge_row_at_an_edit_stop_still_opens_it`. Not built: the opened side's lane shape (Q10) app-side. **→ 06e.** **06e:** `fold_by_default::space_opens_the_merged_side_on_its_own_lane_joining_the_merge` — after `space` the 4 rows share one lane ≠ the merge's, the merge's `outgoing` holds `(that lane, side tip)`, each row `inside` the merge, tail `▾ 4 commits from PR 16021`.
- [x] First paint and held-`j` fast-tier tests pass: `event_loop/tests/settle.rs:312`, `event_loop/tests/pace.rs:220`, `event_loop/tests/pixel_scroll.rs:192`. Release numbers are the milestone review's. **Not ticked:** all three pass (`a_held_j_follows_the_first_row_and_the_row_it_rests_on_and_nothing_between`, `three_hundred_held_keys_never_put_two_frames_in_the_pipe`, `held_j_on_wide_strips_stays_within_the_byte_budget`), but none exercises the fold: settle's window carries `fold: []`, pace and pixel_scroll walk no repo. A passing tick would be vacuous. **→ 06e.** **06e:** held `j` over a walked repo whose window folds: `event_loop::tests::settle::fold::a_held_j_over_a_folded_walk_follows_and_counts_only_at_rest` (asserts 60 of 61 fold facts `Some(true)` first; follow only on the first and rest rows, no `CountFolded` mid-burst, one at rest); first paint: `fold_by_default::first_paint_folds_merged_branches_and_the_tails_name_them`. Release numbers: milestone review.
- [x] Capture `docs/shots/ease-pass/main-folded-120.txt`, `main-opened-120.txt` (next to `prototypes/ease-mocks/mergewright-main-screen.html` in the sheet). Each tick names its test. **→ 06f.** **06f:** both written by `fold_by_default::capture_main_folded_and_opened_at_120` (06e's `wallet_release` fixture, 120×36, every count landed); in `docs/shots/ease-pass/ease-pass-contact-sheet.txt`, captions naming the mock. Look unjudged (rule 6), colour unverified (rule 5).

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

## NOTES (continuation 1, app half, 2026-10-17) — copied from branch `worktree-agent-acb2cc0f5685a9e78` @ 7a4c11f0
- **Compacted once mid-continuation.** Stopped at a seam: the app folds by default and all the wiring is built, with unit tests green. **The integration tier is not reconciled** — see Handoff 2. Report: `partial`.
- **Scout used:** `.scratch/ease-pass/scouts/06-app-wiring.md` and `06-graph-fold-and-lanes.md` (given). No new scout.
- **Built (Handoff 1 items 1–6 below, all done):**
  - `View::folded()` is what a walk starts in (`History::started`). `View::default()` still draws everything; layout-level tests use it.
  - HEAD goes into `keep` when the refs arrive and at walk start (`update/tree_answers.rs` `refs_view` → `keys::view_change::with_head_kept`).
  - The worker lays out through `history::fold::lay_out`, which advances and repairs and tracks the earliest invalidated row. It is used by `serve::answer`, `Views::idle_step`, `promotion_step` and `reach`.
  - `Window` gains `fold: Arc<[RowFold]>` (`RowFold { folds, inside: Option<Inside { merge, label }> }`) and `anchor_behind: Option<Box<(Oid, Oid)>>`. The box exists because of AppState size and `large_stack_arrays`.
  - `Views::answer` fills the fold facts (`views/sent.rs` `with_folds`).
  - A window already sent that a later repair invalidates, or whose fold facts change, is sent again under the same request id (`views/sent.rs` `refresh_sent` / `refreshed`).
  - Promotion: an anchor the new order folds away lands on its `behind` merge (`fold::landing`, `scroll/apply.rs` `promoted`, `views/promotion.rs` `has_row`).
  - `space` (`keys/view_change.rs`):
    - on a merge with `folds == Some(true)`, opens it, or closes it if already open;
    - on a row `inside` an opened merge, closes that merge and seeks the cursor to it (`keys.rs` `apply_view_change`, `land_on`);
    - otherwise does nothing.
  - `Inclusion.folded` is retired, and with it `Filters.folded` and the header's "N merges folded".
  - Counts at rest: `update::fold_count::at_rest` from `settle` when not `defer_follow`. One `CountFolded` per rest, for the first uncounted folding merge on screen. Each answer's rest asks for the next.
  - graft-ui: `FoldCount { Pending, Known(u32), Failed }`, `FoldMark { open, count, from }`, and `FoldMark::words(ascii)` (`graph_screen/fold_mark.rs`).
  - `view::rows::fold_tail` draws the tail. `view::rows::inside_merge` is not drawn yet (07 draws opened rows); it is `expect(dead_code)` outside tests.
- **Test-first was not kept** for most of the worker wiring: it was written, then tested. **Untested code paths** (gaps):
  - `views/sent.rs` re-send of a stale window;
  - `fold::landing` / `anchor_behind` on promotion;
  - `with_head_kept`.
- **Fixture knock-ons:**
  - `update::tests::merge_on_screen` now carries fold facts (its merge folds) and a known count `Counted(1)`. Without the count, every event would carry a `CountFolded`.
  - `update::tests::request` asks in `View::folded()`.
  - Three `m8-*` captures driven by that fixture now draw `▸ 1 commit` on the merge row. They are **not regenerated**: Handoff 2.
- **Spec question (not fixed — needs a decision): a folded merge's row reports one parent.** `Row::assemble` sets `commit.parents` to the drawn parents (`crates/graft-graph/src/row.rs:201`), and the fold trims a folding merge's second parent (`inclusion/fold_walk.rs:190-195`). Every app reader of `row.commit.parents` therefore treats a folded merge as a plain commit. That is the root cause behind the failures below:
  - `update/selection.rs:208` — the evil probe and `selected_merge`.
  - `update/context_menu.rs:59` — the Fix menu: `m5_walk::every_scenario::fix_*` fail with "no menu row starts with Fix: …".
  - `update/preview/take.rs:131` — take refusal: `m7_walk::picker::take_refused_on_a_merge`.
  - `view/entries.rs:340` — radar markers.
  - `update/detail/control.rs:131` — parents shown.
  - The revert path: `m5_walk::every_scenario::revert_merge_uses_mainline` fails with git's "commit … is a merge but no -m option was given". The revert ran **without `-m`** on a merge.
  - First-parent view (`1`) has always trimmed the same way, so this was latent before.
  - Options:
    - (a) `Row` keeps the commit's real parents; the drawn ones are already in `outgoing`. This is the cause fix. It touches every reader above, plus stash rows: their index/untracked parents would reappear, so `selection.rs:208` already excludes stashes.
    - (b) The worker restores real parents on folding-merge rows from `Included::read`, which is untrimmed. This is a symptom fix.
  - I recommend (a), as a planner/conductor call.
- **Second finding (behavioural, with line): the HEAD keep changes the view on every launch.** `with_head_kept` (`update/keys/view_change.rs:106`) returns a new view whenever `keep != {HEAD}`, and keep starts empty. So the refs read always re-asks rows, and the phase goes back to `Streaming`. `m7_walk::pull::pull_ff_over_edits` fails on exactly that ("left: Some(Streaming), right: Some(Complete)").
  - Fix options: the worker takes HEAD from the seeds, or the walk starts with refs known. Either way, keep must not cost a re-layout on every launch.
  - Also unmeasured: what that re-layout costs first paint (box 8).

- **Test state at the commit:** workspace nextest after merging main (05) ran 6615 tests: 6567 passed, 48 failed. One of those, `graft-ui::no_literal_keys` (`1 commit` / `? commits` read as keys), was fixed after the run with two `ALLOWED` rows, so **47 fail**. All 47 are the knock-ons listed in Handoff 2. fmt, clippy `-D warnings` and `test --doc` pass. Every test named in a tick passed in that run.
- **Touches** (how each was checked):
  - graft-graph `Inclusion.folded` retired — grep `inclusion.folded|folded: \[` is empty;
  - graft-ui `Filters` / `FoldMark` / `FoldCount` (graph_screen tests);
  - `Window` literals in every test site (compile);
  - `merge_on_screen` / `request` fixtures (lib tests);
  - the m8 captures driven by that fixture (failing: Handoff 2.5);
  - `space` at the edit stop (`edit_stop_tests`);
  - `1` / `H` base view (`graph_toggles`);
  - `over_400.txt` rows — 17 changed, and `inclusion/fold_walk/tests.rs` (415) was missing from main's list, now added;
  - capability M1-09 citation (`graft-capabilities` checker passes);
  - graft and graft-ui MAP.md (`map_coverage` passes);
  - `no_literal_keys` allow-list.
  - **Not touched:** `docs/design/02-keymap.md` §2.3 `Space` wording; journal/undo (a fold is not an operation).

## Handoff 2 (what remains after continuation 1) — copied from the branch
1. **Decide the parents question above**, then fix it. It unblocks the m5 Fix scenarios, the revert, the picker refusal, radar markers on folded merges, and probably the 120 s timeouts (`radar_not_checked`, `hint_bar::capture_the_doorway_on_a_marked_row`, which wait on a merge's marker).
2. **Fix the launch re-layout** (finding 2) and re-run `m7_walk::pull::pull_ff_over_edits`.
3. **Integration tests that need a merged branch's rows on screen** must open the merge (`space`) or pick a row the fold keeps. Each fails because the rows they looked for are folded:
   - `m8_walk::journeys::{find_key_by_name, run_unrecalled, what_does_key, light_theme}` — "topic-1's row is drawn";
   - `m8_walk::teaches::{merge_is_found_on_a_branch_row, create_a_branch_is_found, push_is_found}`;
   - `look_chrome::{every_drawn_row_is_the_row_a_click_selects_at_every_designed_size, capture_the_cursor_row_in_dark_and_light}`;
   - `m8_walk::look::capture_the_cursor_row_at_80x24_and_187x50`;
   - `lane_cap::{the_pane_narrows_the_graph_so_the_graph_folds_more, capture_lane_cap_for_docs_shots}` — no lanes left to cap: the fixture's topics fold. Open them, or build the fixture with unmerged branches;
   - `context_menu::the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it`;
   - **Not diagnosed:**
     - `m7_walk::pr_contents::{pr_contents, capture_compare_since_split}` — 120 s timeouts;
     - `tour::{an_unwritable_temp_folder_stays_in_his_repo_and_says_why, the_palette_starts_the_tour_and_esc_returns_to_his_commit}`, `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home`, `tour_tasks::task_7_ticks_by_resolving_clash_and_finishing` — about 20 s each;
     - `preview::the_branch_badge_still_lands_on_the_newest_predicted_row` — "no label row".
4. **The old `+N` journey and capture** — `m8_walk::journeys::fold_merge_count` and `look_polish::capture_folded_merge_count_for_docs_shots` (times out at 120 s waiting for `+N`) — are superseded by the tail. Rewrite them to the new words, or retire them for box 9's captures.
5. **Captures** whose only change is the fold (rows hidden, tails, header commit count): regenerate them with `MWR_UPDATE_SHOTS=1` **after 1–3**, and list each for the user's eye (rule 6):
   - `m8-look-chrome-80x24`, `m8-look-help-graph-187x50`, `m8-help-80x24`, `m8-keymap-config-problem-120x36`, `m8-keymap-remapped-120x36`;
   - `m8-quit-stop-now-80x24`, `m8-quit-waits-80x24`, `m8-bar-after-search-120x36`, `m8/search-hint-120x36`;
   - `m7/rebase-line-preview-187x50`, `m7/rewrite-refused-fork-120x40`, `m7/refused-box-single-close-120x40`, `m7/rebase-stop-forecast-80x24`;
   - `m6/fix-preview-verb-120x40`, `m6/fix-finish-shell-merge-120x40`;
   - `m5-ui-name-prompt-80x24`, `m5-ui-preview-rebase-80x24`, `m5-ui-preview-scrolled-80x24`, `m5-lane-cap-160x24`.
   - Also stale by meaning: `docs/shots/15-merges-folded.txt` / `15-merges-unfolded.txt`, cited by capability M1-09. Its test citation was updated here, but those shots were not.
6. **Boxes still open:**
   - 5: a real fixture repo through the worker. "First paint has no merged-branch commits" is proven only engine-side.
   - 7: the opened side's lane shape (Q10) is not tested app-side.
   - 8: the budget tests — results in NOTES below.
   - 9: the two captures.
7. **Untested paths** listed in NOTES: re-send, landing, head keep. Write their tests.

## Handoff (the app half — continuation 1 did items 1–6)
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

## Decisions for continuation 2 (planner, 2026-10-17)
This is the last continuation. The remainder is more than one context, so it is split three ways, run in order on the branch above (after it merges): **06d → 06e → 06f**. Each is its own spec file in this folder. The line refs above are the builders'. The planner did not read source, so the first step of each ticket re-checks them.

### D1. Parents: `Row.commit` is git's commit, always. Drawing reads `Row.drawn_parents`. (Option (a), the cause fix.)
- **Shape** (graft-graph `row`): `commit.parents` = every real parent, under every inclusion (fold, opened merges, first-parent `1`, hidden stashes). The new field `drawn_parents` holds what `commit.parents` holds today. `Row::is_merge()` is the one answer to "is this a merge" outside drawing. Signatures: 06d.
- **Why this direction** (truth in the old field name, the trimmed set in a new one), and not a `real_parents` field next to a trimmed `commit.parents`: say a later reader reaches for `commit.parents`. Here it gets the truth. If that reader is drawing code, the worst case is an extra drawn edge, which a capture and the layout invariants catch. The other direction fails silently, as a wrong git write: today's `revert` without `-m`.
- **No git write may see a folded view.** This is enforced by a test, not by care. A graft-graph property test says every row's `commit.parents` equals the source commit's parents, under every inclusion `viewed()` generates. App tests say a folded merge, and a merge in `1`, is a merge to every semantic reader.
- **Drawing stays byte-identical.** `Ring::of`, raster, `Ancestry`, `Fold::plan`/`view`, `made_on`, the layout invariants and the `RowShape` fingerprint read `drawn_parents`. A folded merge keeps today's node; how it looks is 07's call.
- **Stash rows:** `is_merge()` is false for them. The detail pane for a stash keeps today's parents output.
- **Rejected (b)**, restoring parents in the worker: it treats the symptom and leaves `1` broken.

### D2. HEAD's keep is known when the walk starts. A refs read re-lays out only when HEAD really moved.
- **Cause:** the walk starts with `keep = []`. The first refs read writes `{HEAD}`, so the view always differs. That means a reinclude and `Streaming` on every launch (`view_change.rs:106`).
- **Rule:** every walk start (launch, and any re-walk after an operation) carries HEAD's commit in `keep`. It comes from the read that starts the walk: the repository opening's facts. `with_head_kept` returns `None` when HEAD's commit equals the one already kept. A real HEAD move is one reinclude from the first changed row, which is correct.
- **Rejected:**
  - HEAD in graft-graph `Seeds`: seeds start lanes (`LayoutCursor::new`), so HEAD there risks a drawing change and pushes an app fact into the engine.
  - Suppressing the re-ask or the phase change on keep changes: a symptom fix.

### D3. Captures and failing tests: a diff rule, not a list to re-shoot blindly.
- **Legitimate fold change** (re-shoot with `MWR_UPDATE_SHOTS=1`, the user judges by eye): the diff against the committed capture is **only**:
  - merged-branch rows gone, and the rows below moving up;
  - `▸ N commit(s) from …` tails on merge rows;
  - the header's commit count;
  - lanes no longer drawn because their commits folded.
- **Regression** (fix the cause, never re-shoot) — anything else, for example:
  - the cursor on a different commit;
  - a badge, radar mark, menu row, ghost row or banner line that differs;
  - the thing the capture's `TESTING.md` line is about no longer on screen.
- **When the fold hid a scenario's subject**, the test changes; "what right looks like" does not. In order of preference:
  1. target a row the fold keeps that serves the same purpose;
  2. add an **unmerged** branch to the fixture (a new ref only; existing refs untouched);
  3. press `space` to open the merge, only when the scenario is about a merged branch's rows.
- **m8 journey step counts never change.** A journey that would need `space` to reach its row takes route 2.
- **Ghost / predicted rows never fold** (M5 promise). If one did, that is a regression: stop and report it with the line.
- **The `tour` is product, not fixture.** If a tour task asks him to act on a row the fold now hides, fix the tour (its repository or its step), and capture it.
- **Superseded:**
  - `fold_merge_count` → rewritten to the tail words and `space`;
  - `capture_folded_merge_count_for_docs_shots` → retired for box 9;
  - `15-merges-folded/unfolded.txt` → deleted only if `grep` finds no other citer; M1-09 is repointed to box 9's captures.
- **Lane cap:** the scenario's purpose is many live lanes, so take route 2 (unmerged topic branches), not N presses of `space`. 06b reworks lane drawing after this, so `m5-lane-cap-160x24` will be re-shot again there.
- **Every re-shot capture is listed in NOTES**, each with its classification and a one-line diff summary, for the user's eye (rules 5, 6).
