# fold-seek-reveal — going to a commit inside a folded merge opens that merge and lands on it

Status: done but one box — 17/18. Reveal half (11) and "Search folded count" half (6 of 7) built; `m8/search-hint-120x36` re-shoot is a D3 **stop** (not suffix-only), left open for the conductor (NOTES, second group)
Blocked by: 06-graph-fold-and-lanes
Build with: Opus (`builder-opus-xhigh`) — the history worker's Find and serve, the app's reveal and six routes across two crates; every outcome is checkable by a state test per route (scout 06c confirms; facts don't change it).
Needs: from 06: `FoldWalk::behind(CommitHandle) -> Option<CommitHandle>` (`graft-graph/src/inclusion/fold_walk.rs:254`), `Included::walked`/`shown`/`fold` (`inclusion/included.rs:119,125,133`), `HandleLookup::handle_of` / `id_of` (MAP `layout`; called inside `included.rs:332`), `OpenedMerges::open` (`inclusion.rs:91`), `Inclusion::opened_mut` (`:134`), the `space` open path (`update/keys/view_change.rs:41-57`, `update/keys.rs:447-454`). Existing: `History::seek` (`graft/src/history/scroll/query.rs:79-96`, `find` `:102-115`), `Wanted::Find` (`history/state.rs:51-63`), serve (`history/serve.rs:187-188`, answer range `:221-227`), answer applied (`history/scroll/apply.rs:155-171`, cursor-stays `:166`, `jump`/`seeking` cleared `:176-177`), promotion re-seek (`apply.rs:93`). Paths under `crates/`.

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviour G4 (lines 117-120); Hard-to-reverse choice 2 (lines 226-242); "Mock gaps decided in pass 2" (lines 153-169), its 2026-10-18 search sentence.
- `crates/graft-graph/MAP.md` `inclusion` (lines 37-62), `layout` `HandleLookup` (line 71). `crates/graft/MAP.md`: `history` (77-107, note `history::fold` line 93 already finds the merge a folded commit hides behind — reuse it, don't duplicate), `search` (198-203), `update::detail`, `update::banner`, `update::journal`, `update::recover`, `update::keys`, `update::map`, `update::palette`, `update::picker`, `update::search`.
- Scout `.scratch/ease-pass/scouts/06c-fold-seek-reveal.md` (whole, 1 page) — line refs below are its, at ec7d4c3f.
- 06's scout file, "Seeks to a possibly hidden commit".
- `docs/reviews/ease-pass-06.md` Major M3.

## Why (scout: today every route says "not in this view" or nothing)
With folding the default, a merge's second parent, a search hit on a merged PR, a `J` put-back on a merged branch are hidden in the everyday case. Each route today lands nowhere (search: footer `(not in this view)`, `graft/src/search.rs:332-336`; palette, Map, picker `⇥`, `J`, detail links: silent or "can't show … not in this view", `update/detail/landing.rs:237-239`) **and** first lays out the whole history to look (`serve.rs:187-188`, `usize::MAX`). Planner decision (`docs/research/ux-principles.md`: visibility of system state, fewest steps): a commit hidden **by the default fold** is revealed — its merge opens, the cursor lands on it, 0 extra keys. A commit hidden by a view **he** chose (first-parent `1`, hidden stashes) keeps today's answer.

**Search count (2026-10-18).** `/topic` says `4 matches` while 3 are inside folded merges and one row is lit. The count stays; the footer says how many are folded; `n`/`N` visit every counted hit.

## Decided here (reconciled with scout 06c, 2026-10-18)
- **No miss type exists today** — a miss is inferred as selected ≠ target (`update/search.rs:181,206`; detail `landing.rs:142-160`). **Decision: the worker's Find answer gains an explicit `Found`; callers keep their inference.** `Found::Folded` never reaches a caller: the app reveals and re-seeks first, so a route's landing hook only ever sees the two outcomes it handles today (landed, or not in this view). Why: smallest change to six routes; the new case is handled once.
- **Find stops when the id is walked, not when it has a row.** Serve lays out until `Included::walked` holds the id or the walk ends (not `usize::MAX` for a walked id), then answers `Found` from `Included::whereabouts`. On `Folded { behind }` the answer's rows are `behind`'s rows (a shown merge has one), else `fallback`.
- **`Oid` → handle:** `whereabouts` lives in graft-graph `inclusion::included`, beside `handle_of` (`included.rs:332`, same crate — its privacy is no barrier; `HandleLookup` is public per MAP line 71). It converts with `handle_of`, tests `walked()`/`shown()`, calls `FoldWalk::behind`, converts back with `id_of`. **Stop condition:** if `handle_of` on an id the walk never named panics (rather than returning a handle absent from `walked`), stop and report `blocked: handle_of on an unwalked id` — no second index.
- **One merge per reveal, repeated.** `behind` names the *shown* merge (MAP line 50), so a nested commit answers `Folded { behind: outer }`; the app opens `outer` through `space`'s own open half (one `ViewChange`, one `reinclude` — scout item 9: no multi-merge path exists, none added), re-seeks, gets `Folded { behind: inner }`, opens it, re-seeks, lands. Each round opens a merge not yet open, so it ends. A `Folded` whose merge is already open is a worker bug: the app ends the seek as not-in-this-view and logs it through the existing error path (not swallowed; covered by a test).
- **Promotion re-seek (`apply.rs:93`) does not reveal.** The cursor commit hidden by the exact fold after promotion → cursor on `behind` (the merge holding it), nothing opened — same landing as `space` closing a merge (G4). Mock silent; consistency rule.
- **Routes that reveal** (all via `History::seek`): search `n`/`N` (`update/search.rs:178`, `missed` `:180-182`, `seek_landed` `:189-209`); `map::seek_to` (`update/map.rs:202-212`) and its three callers — Map `⏎` (`:196`), picker `⇥` (`update/picker.rs:266`), palette commit find (`update/palette/run.rs:62`); detail parent links (`update/detail/landing.rs:119,142-160`); banner `Landing::Commit` (`update/banner/stop.rs:359`, seek `:378`). **`J` put-back:** MAP `update::recover` (line 292) says the journal's put-back is `Recover` (`operate.rs:199-200` `MoveRef`/`Recreate`, scout); confirm by the acceptance test driving `J` → put back → landing. If that put-back never reaches `stop.rs:359/378`, don't add a landing: say so in NOTES (out of scope). `DropPutBack` has no landing (`operate.rs:232`) — unchanged.
- **Unchanged:** tour's HEAD seek (`tour.rs:178`; HEAD is a keep tip); `space`'s own landing seek (`keys.rs:453`, a shown merge); overview strip marks (resident rows only, `view/strip.rs:17-24`); **Ghost Preview** slices (`view/preview.rs:1-9`: drawn from rows already sent, never `Inclusion`).
- **Search folded count.** The search job walks on its own; only the history worker knows the fold. The app asks the worker, beside the rows: `FoldedAsk { search, ids }` rides on every `RowRequest` until an answer with the same `search` arrives (survives a superseded request); if no request is due, the app sends one for the current viewport. Asked when the hits finish, after each reveal while the search is live, and once more when the walk completes if the last answer had `unwalked > 0`. The worker answers `HistoryUpdate::FoldedHits` without laying out further. Footer appends ` · N in folded merges` while `folded > 0` (status rule); hits hidden by a chosen view (`Filtered`/never walked) aren't counted. `n`/`N` order is the hit list's (walk) order; `n` from the last wraps as today.

## Scope
In: graft-graph `inclusion::included` (`Whereabouts`, `whereabouts`); graft `history::state` (`Found`, `OnFold`, `Reveal`, `FoldedAsk`, `HistoryUpdate::FoldedHits`), `history::serve` (Find stop rule, `Found`, `FoldedHits`), `history::scroll` (`query.rs` seek/find, `apply.rs` `Found` handling, promotion), `update::keys::view_change` (factor `open_merge` out of the `space` open half), the `update` arm that applies a Find answer (reveal + re-seek; builder names its file:line in NOTES), `update::search` + `search.rs` (ask, `folded`, footer suffix), route tests for map/picker/palette/detail/banner/journal.
Out: what folds (06); header and lanes (06b); words for a chosen-view miss on silent routes; marking a folded merge row that hides a hit (not in mock); `DropPutBack`; graft-ui (unless `no_literal_keys` trips on the suffix — then only its allow-list).

## Signatures
graft-graph `inclusion`:
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Whereabouts {
    Shown,
    /// Hidden by the default fold behind this shown merge.
    Folded { behind: Oid },
    /// Walked, hidden by a view he chose (hidden stashes, stash entries).
    Filtered,
    NotWalked,
}
impl<S: CommitSource> Included<S> {
    /// Where a commit stands in this view; never advances the walk.
    pub fn whereabouts(&self, id: Oid) -> Whereabouts;
}
```
graft `history::state`:
```rust
/// How the worker answered a `Wanted::Find`; rides on the update carrying the Find's rows (built at serve.rs:221-227).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found { Row, Folded { behind: Oid }, NotInView }

/// What a pending find does with `Found::Folded`. `seek` → `Reveal`; the promotion re-seek → `LandOnMerge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OnFold { Reveal, LandOnMerge }

/// A reveal the app owes: open `merge`, then seek `target` again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reveal { pub merge: Oid, pub target: Oid }

/// Asked beside rows: how many of a search's hits the default fold hides now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldedAsk { pub search: u64, pub ids: std::sync::Arc<[Oid]> }
// RowRequest gains: pub folded: Option<FoldedAsk>
// HistoryUpdate gains: FoldedHits { search: u64, folded: usize, unwalked: usize }
```
`search: u64` is the key `Search` already tells its job's answers apart by (name it in NOTES; if none, add a `u64` generation on `Search`).

graft `history::scroll`: `History::seek` keeps today's signature (sets `OnFold::Reveal`); new
```rust
impl History {
    /// The reveal the last Find answer left owed, if any. Taken once.
    pub fn take_reveal(&mut self) -> Option<Reveal>;
}
```
graft `update::keys::view_change`:
```rust
/// Open one folded merge as one view change (space's open half; the reveal). False if already open.
pub(crate) fn open_merge(state: &mut AppState, merge: Oid) -> bool;
```
Search: the hit summary the footer reads gains `folded: usize` (name file:line in NOTES).

## Touches (each checked how, in the report)
- Every `History::seek` caller: search `:178`, `map::seek_to` (Map `:196`, picker `:266`, palette `run.rs:62`), detail `landing.rs:119,142-160`, banner `stop.rs:378`, tour `tour.rs:178`, `space` `keys.rs:453`, promotion `apply.rs:93` — grep `\.seek(` and list each with its outcome.
- `apply.rs:155-171` cursor-stays path: now only `Found::NotInView`; `jump`/`seeking` clearing (`:176-177`) for each `Found`.
- Landing hooks run only after a seek ends `Row`/`NotInView`: search `seek_landed`, detail landing, banner landing — name where each is triggered.
- `space` (`ViewChange::Fold`, `view_change.rs:41-57`) after `open_merge` factoring: existing `space` tests unchanged.
- Search across a reveal: hit list stays valid (same ids); `missed` (`search.rs:273`) only for `NotInView`; footer `line` `search.rs:327-340`.
- Existing tests: `update/detail/landing/tests.rs:287-294` (miss words), `update/place/tests/esc_back.rs:275,340` — pass unchanged.
- Captures: `m8-bar-after-search-120x36`, `m8/search-hint-120x36` (06 D3 rule; the latter has a folded hit per review M3).
- Overview strip, Ghost Preview: by test, below. Journal/undo: a reveal is a view change, not an operation — not journaled, `u` unaffected (assert in the reveal test). Keymap: no key added.
- Docs: graft-graph MAP `inclusion` (`Whereabouts`, `whereabouts`); graft MAP `history::state` (`Found`, `Reveal`, `FoldedAsk`), `update::keys` (`open_merge`).
- Layout cost: rows laid out for a Find to a walked hidden commit, before/after — one number in NOTES.

## Acceptance (each tick names its test, at tick time)
- [x] `whereabouts` answers `Shown`, `Folded { behind }` (the shown merge), `Filtered` (hidden stash), `NotWalked`, without advancing (graft-graph `inclusion` tests: `whereabouts_*`). **Ticked:** graft-graph `inclusion::tests::fold_by_default::{whereabouts_answers_shown_and_folded_behind_its_merge, whereabouts_names_the_shown_merge_through_a_hidden_one, whereabouts_answers_filtered_for_a_view_he_chose, whereabouts_answers_not_walked_without_advancing}`.
- [x] Detail on a folded merge, follow parent 2 → merge opened, cursor on parent 2 (`update/detail/landing/tests.rs`). **Ticked:** `update::detail::landing::tests::reveal::show_line_on_a_folded_merges_second_parent_opens_the_merge_and_lands`.
- [x] Search hit on a merged PR's commit → merge opened, cursor on hit; `n` to the next hit in the same PR opens nothing new (`OpenedMerges` unchanged) (`update/search` state test). **Ticked:** `update::search::tests::reveal::n_onto_a_merged_prs_commit_opens_it_and_n_within_the_pr_opens_nothing_new`.
- [x] 4 hits, 3 inside folded merges (two merges): footer `4 matches · 3 in folded merges`; `n` from the first visits all 4 in walk order, each revealed and landed; after, footer `4 matches` (state test + footer-words test on `search.rs` `line`). **Ticked:** state `update::search::tests::folded::n_from_the_first_visits_every_hit_and_the_line_counts_the_folded_down` (hits 0x30 shown, 0x11/0x12 behind 0x20, 0x05 behind 0x06; line asserted whole at each of the 4 steps: `· 3 …` → `· 1 …` → `· 1 …` → none; each reveal's find carries a new ask); words `search::tests::the_line_says_how_many_hits_are_in_folded_merges`. The walk's counts in the state test are fed by hand; the worker's counting is `history::idle_tests::folded_tests::a_find_answers_how_many_asked_hits_fold_even_when_superseded`.
- [x] Same search, `N` from the first: wraps to the last, merge opened, cursor on it; `N` again visits the rest backwards; no `(not in this view)` at any step (state test). **Ticked:** `update::search::tests::reveal::capital_n_from_the_first_wraps_and_reveals_each_hit_backwards` (4 hits, 3 behind two merges; `missed` false at every step).
- [x] No folded hit: footer byte-equal to today's (footer-words test). **Ticked:** `search::tests::with_no_folded_hit_the_line_is_unchanged` — literal `/login  4 matches (1/4)  n/N next/prev  esc close`, the shape the committed `m8/search-hint-120x36` shows.
- [x] Map `⏎`, picker `⇥` on a merged branch, palette commit find — each on a hidden commit → revealed and landed (one test each, in `update/map`, `update/picker`, `update/palette`). **Ticked:** `update::map::tests::enter_on_a_merged_branch_opens_its_merge_and_lands_on_its_tip`, `update::picker::tests::seek::tab_on_a_merged_branchs_tip_opens_its_merge_and_lands_on_it`, `update::palette::tests::seek::a_commit_row_on_a_folded_commit_opens_its_merge_and_lands_on_it`.
- [x] `J` → put back a commit on a merged branch → lands on it, merge opened (`update/journal` or `update/recover` test); or NOTES says the put-back has no landing, with file:line. **Ticked**, as a chain of two tests: `J` → `Recover(Recreate)` is the existing `update::journal::tests::enter_on_a_gone_branch_opens_make_it_again`; the put-back's `Done` → `Landing::Commit` → `stop.rs:378` seek → reveal is the new `update::operate::tests::landing::a_put_back_onto_a_merged_branch_opens_its_merge_and_lands_on_it`. No single test drives the `J` key through to the landing.
- [x] Nested: commit behind a merge behind a folded merge → both open, cursor on it (state test, two reveal rounds asserted). **Ticked:** `update::history::tests::reveal::a_commit_behind_two_merges_opens_both_in_two_rounds_and_lands`.
- [x] `Folded` naming an already-open merge → seek ends as not-in-this-view, error logged (state test). **Ticked:** `update::history::tests::reveal::folded_behind_a_merge_already_open_ends_the_seek_and_says_so` (`Status::Failed`, seek over, cursor on the merge, nothing re-asked).
- [x] Promotion: cursor commit hidden by the exact fold → cursor on its merge, `OpenedMerges` unchanged (`history::scroll` test). **Ticked:** `history::reveal_tests::a_promoted_anchor_the_new_order_folds_lands_on_its_merge_and_opens_nothing` (in `history/reveal_tests.rs`, not `history::scroll`).
- [x] First-parent `1`, hit off the line → `(not in this view)`, nothing opened, no suffix (state test). **Ticked:** state `update::search::tests::folded::off_the_first_parent_line_the_line_says_not_in_this_view_and_nothing_folded` (line asserted whole: `/login  2 matches (2/2)  (not in this view)  n/N next/prev  esc close`; view unchanged; nothing asked); worker `history::idle_tests::folded_tests::hits_off_the_first_parent_line_are_not_counted_folded` (every ManyLanes commit asked: 0 folded in first-parent, >0 folded in the folded view; mutation-checked — counting `Filtered`/`NotWalked` as folded fails it, 28 ≠ 0). Earlier: `update::history::tests::reveal::off_the_first_parent_line_opens_nothing`.
- [x] Find to a walked hidden commit lays out no further than the walk already is (`history::serve` test, row count before/after). **Ticked:** `history::idle_tests::folded_tests::a_find_to_a_walked_folded_commit_lays_out_no_further_and_answers_its_merge` — ManyLanes fixture: 3 rows laid out now, 40 under the old rule (to the end).
- [x] `FoldedHits` survives a superseding `RowRequest`; re-asked on walk completion when `unwalked > 0` (`history::serve` / `update::search` tests). **Ticked:** worker `history::idle_tests::folded_tests::a_find_answers_how_many_asked_hits_fold_even_when_superseded`, `history::reveal_tests::a_folded_ask_rides_on_every_request_until_its_answer`; app `update::search::tests::folded::hits_the_walk_had_not_reached_are_counted_once_it_completes` (answer `unwalked: 2` while streaming asks nothing; `LaidOut` → one new ask; its answer is the line's) and `…::when_the_hits_finish_the_walk_is_asked_and_its_count_is_said` (an answer to another ask number is dropped).
- [x] Ghost Preview slice identical with every merge folded and opened (row-set test); overview strip marks unchanged (strip test). **Ticked:** `update::preview::resident::tests::a_preview_slice_is_the_same_with_every_merge_folded_or_opened` (same window rows, `View::folded()` vs every merge opened through `open_merge` — asserted ≥ 2 merges opened; the plan job's slice ids and the drawn picture are equal); `view::strip::tests::hits_in_folded_merges_leave_the_strip_marks_unchanged` (3 hits, 2 not resident; marks equal with `folded` 0 and 2; one `SearchHit`, row 6).
- [x] Reveal is not journaled; `u` after a reveal does what it did before it (state test). **Ticked:** `update::history::tests::reveal::a_reveal_is_not_journaled_and_u_is_unchanged`.
- [x] Capture `docs/shots/ease-pass/main-search-folded-120.txt`: footer with suffix before `n`, cursor on the first hit (06e's `wallet_release` fixture, 120×36). Look unjudged (rule 6), colour unverified (rule 5). **Ticked:** written and checked by `tests/it/search_folded.rs::capture_search_folded_count_at_120` (`/totals`: asserts 5 hits, 4 folded, cursor on `Merged PR 16021…`, header `/totals  5 matches · 4 in folded merges (1/5)`, the merge still `▸ 4 commits from PR 16021`). Look unjudged, colour unverified. The header's `2 merged branches folded` gives way at 120 columns (graft-ui `header_tail.rs` drops lane words first); said in the capture's caption.
- [ ] `m8/search-hint-120x36` re-shot: footer carries ` · 3 in folded merges`; NOTES gives its D3 classification (suffix-only, or a stop). **Open — D3: a stop, not re-shot.** The settled frame is not suffix-only: the header also loses `1 merged branch folded` (see NOTES, second group, for the drawn lines). Same for `m8-bar-after-search-120x36`. The committed captures still pass: they are taken the moment the search finishes, before the walk's count lands.

## User-visible vs the mock (for the conductor)
- Nested reveal: one round trip per level — a frame may show the outer merge opened before the cursor arrives. Final state as decided.
- Promotion: cursor moves to the merge holding his commit, not revealed (mock silent; matches `space` close).
- Large repo: hits the layout hasn't walked yet aren't in the folded count until the walk completes (re-asked then).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k. If it runs over, the split line is "Search folded count": `FoldedAsk`/`FoldedHits`, the suffix and both captures become a second group.

## NOTES (builder, 2026-10-18)
- **Scout used:** `.scratch/ease-pass/scouts/06c-fold-seek-reveal.md`.
- **Compacted: yes**, once, during the route tests. Stopped at the split line afterwards; the remainder is the Handoff below.
- **The apply arm:** `crates/graft/src/update/history.rs` `walk_progress` takes the owed reveal after `history.apply` and before `search::seek_landed`, and hands it to `reveal()` in the same file. History side: `history/scroll/reveal.rs::land_find`, called from `history/scroll/apply.rs` (plain find landing and the promoted path).
- **Where this differs from the ticket** (flagged here, not a silent change):
  - a. **No `OnFold`.** The "promotion re-seek" (`apply.rs:95`) asks again for a route's seek that is still in flight, under the new order. Landing it on the merge without opening it would make a search hit there read "(not in this view)". So it reveals like any other find. The promotion's own cursor anchor still lands on its merge with nothing opened, through the existing `anchor_behind`. That is the ticket's promotion box.
  - b. **`open_merge(&View, Oid) -> Option<View>` is pure**, rather than taking state and returning a bool. The reveal must send the find, not the request `change_view` makes.
  - c. **`FoldedAsk.search` is a per-ask counter on `History` (`folded_asks`)**, not the search's `JobId`. `Search.job` is cleared when the job ends, and each re-ask after a reveal needs its own number.
  - d. **`found` rides in `Window.sought: Option<Box<Sought>>`**, together with `anchor_behind`.
- **`.seek(` callers** (`grep -rn '\.seek(' crates/graft/src`, non-test):
  - Reveal on a folded answer, each with a test:
    - `update/search.rs:178` — `seek_landed` returns early while `history.seeking` is set again.
    - `update/map.rs:208` `seek_to` — Map `⏎`, picker `⇥`, palette `run.rs:62`.
    - `update/detail/landing.rs:151`.
    - `update/banner/stop.rs:378` — tested through `operate`. No banner file was edited.
  - Unchanged:
    - `update/tour.rs:178` — `HEAD` is a keep tip, never folded. Not tested here.
    - `update/keys.rs:453` — `space` lands on a shown merge. The existing `space` tests pass.
  - `history/scroll/apply.rs:95` — the promotion re-ask (deviation a).
  - `update/history.rs:67` — the reveal's own re-seek.
- **Landing hooks after a reveal:**
  - search `seek_landed` waits through the re-seek.
  - detail `sought()` polls `history.seeking`, so it also waits.
  - the banner landing is a one-shot seek with no hook after it.
- **Layout cost:** a Find to a walked hidden commit lays out **3 rows now vs 40 under the old rule** (to the end of the graph). Measured on the ManyLanes fixture in the test above, by a temporary `eprintln` that has since been removed. A fixture number, not a measurement on a real repo.
- **Ratchet rows** (all moves are move-only):
  - Lowered: `history/tests.rs` 1475→1446 (`RowRequest::new`); `picker/tests.rs` 628→616 (the `Tab` seek test moved to `picker/tests/seek.rs`).
  - Removed: `update/map.rs`, whose inline tests moved to `update/map/tests.rs`; `update/palette/tests.rs`, whose placeholder test moved to `palette/tests/placeholder.rs`.
  - `jobs/tests.rs` is back at 1285 after fmt.
- **Touches:**
  - Every `HistoryUpdate` match: the new arm was added in `event_loop/label.rs`, `history/tests.rs` and `jobs/tests.rs`; the compiler confirms the matches are exhaustive.
  - Every `Window` literal: the field rename covers 26 sites; the compiler confirms them.
  - `space`: the existing view-change tests pass.
  - Ghost Preview: `view/preview.rs` is untouched.
  - Overview strip: `view/strip.rs` changed only in a test fixture.
  - Keymap: no key added.
  - Journal and undo: covered by a test.
  - MAP: graft-graph `inclusion`; graft `history`, `history::sought` and `update::keys`.
  - Captures: none re-shot in this half.
- **Broken vs unverified:**
  - Nothing known broken.
  - Unverified: the intermediate frame of a nested reveal (the outer merge open before the cursor arrives) has not been seen by eye. No capture was taken in this half.

## Handoff — "Search folded count" (second group)
**Done, worker side:**
- `HistoryUpdate::FoldedHits { search, folded, unwalked }` is sent after each `Window` whenever `RowRequest.folded` is set. It is built by `history/serve.rs::folded_hits` and never lays out further.
- `History::ask_folded(ids) -> u64`, `folded_ask_out()` and `ask_again(height)` live in `history/scroll/reveal.rs`.
- `apply` clears `folded_ask` when the answer with the same number arrives.
- `query.rs` puts the ask on every request.

**Not done, app side:** nothing reads the count yet. Exact next steps:
1. **`update/history.rs::walk_progress`:** before `history.apply`, pass a `FoldedHits` whose number matches the search's ask to a new `update::search::folded_hits`.
2. **`Search` (`crates/graft/src/search.rs`):** gains `folded`, its ask number and `unwalked`. The file has a row of 634 in the ratchet list, so split something out before adding.
3. **Asking:** call `history.ask_folded(<all hit ids>)` at three points:
   - when the hits finish (`search_job_ended`, Done);
   - in `reveal()` before the re-seek, while a search is live, so the find carries the ask;
   - once when the walk completes, if `unwalked > 0`.
   If no request is due, push `history.ask_again(height)` as `ShowRows`.
4. **Footer:** `line()` (`search.rs:327-350`) appends ` · N in folded merges` while `folded > 0`, and stays byte-equal otherwise.
5. **Remaining boxes:** 4-hits footer, no-folded-hit footer, first-parent footer, app re-ask on completion, Ghost Preview/strip, and both captures (06e's `wallet_release` fixture, 120×36).

**Gotchas:**
- The package is `mergewright`. Use `-p mergewright` while iterating; run the gates workspace-wide.
- `over_400.txt` rows must match the counts after `cargo fmt`.
- The Bash isolation guard refuses `export`, `$VAR` in paths and long `&&` chains. Use absolute paths.
- Shared fixtures: `crate::update::tests::reveal::{answer, find_for, folded_answer, revealed, on, oid_of}`.

## NOTES — second group (builder, 2026-10-18)
- **Scout:** none new; the first half's `.scratch/ease-pass/scouts/06c-fold-seek-reveal.md` and its Handoff above were enough (the handoff named every file).
- **Compacted: no.**
- **Built, as the Handoff's steps:**
  - `Search` gains `folded`, `unwalked`, `folded_ask` (`crates/graft/src/search.rs:278-284`). The hit summary the footer reads is `Search.folded`; the suffix is built in `line()` (`search.rs:353-357`), placed right after the count: `/login  4 matches · 3 in folded merges (1/4)  …`.
  - Asking: `update/search.rs:260` `ask_folded` (a finished search with hits only), `:279` `ask_folded_now` (also sends `ask_again` as `ShowRows`; not while `head_pending`), `:295` `walk_completed`, `:312` `folded_hits` (takes an answer only if its number is the search's newest ask).
  - Three ask points: hits finish — `search_job_ended` (`update/search.rs:253`, `Done` only; `dispatch.rs` passes `commands` through); after a reveal — `update/history.rs:88`, asked **before** the view change so whichever request goes carries it; walk completes — `update/history.rs:42-55`, `walked_whole` (`:62`: phase `Complete` or `laid_out` heard), only on the transition, only if `unwalked > 0` and no ask is out.
  - `walk_progress` routes `FoldedHits` to `folded_hits` after `history.apply` (apply clears the history's ask; the search keeps its own number).
- **Where this differs from the ticket:** none in behaviour. Only `Done` asks: a cancelled or failed search shows no suffix (the ticket names `Done`; a cancelled search's hits are partial).
- **D3 for `m8/search-hint-120x36` and `m8-bar-after-search-120x36`: a stop, not re-shot.** Pumped until the count lands, the settled frame's header is:
  - committed: ` ◆ api  ~/code/api · main          /topic  4 matches (1/4)  n/N next/prev  esc close  2 commits  1 merged branch folded`
  - drawn: ` ◆ api  ~/code/api · main             /topic  4 matches · 3 in folded merges (1/4)  n/N next/prev  esc close  2 commits`
  - The suffix is there, and `1 merged branch folded` is gone: at 120 columns the header gives way by its own rule, lane words first (graft-ui `graph_screen/header_tail.rs` `Drop`, `fitted_tail`). graft-ui is out of scope. Conductor's call: accept the re-shot (one line in `tests/it/search_hint.rs` `searched()` — also wait for `!history.folded_ask_out()` — then `MWR_UPDATE_SHOTS=1`), or change the header's drop order first.
  - Until then the committed captures still pass, deterministically: the harness's pump stops on the event that finishes the search, before the ask is answered, and the corpus walk is complete by then, so `settled_frame` pumps nothing more. The capture shows the moment before the count lands. The running frame would also need `search.folded = 0` when re-shot: it is the settled state set back to `Running`, and a running search is never counted.
- **Contact sheet:** the new capture has no caption row in `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv` and the sheet is not regenerated — left to the acceptance group (hand-authored file).
- **Tests run red first:** the line words, the ask on finish, the reveal's ask, the completion re-ask. Passed on first run (guards, no new code needed): the first-parent state test, the worker first-parent count (mutation-checked, above), the Ghost Preview row-set test, the strip test, the new capture.
- **Ratchet:** `search.rs` 634 → 398 by moving its inline tests to `search/tests.rs` (move-only; the row is removed).
- **Touches:**
  - `search_job_ended` signature: one caller, `update/dispatch.rs` `side_job_ended` (grep).
  - `Search` literals: only `Search::new` builds one (grep `Search {`); compiler confirms.
  - `Search::line` callers: `view/rows.rs:367` (header tail) and tests. At 120 columns the suffix can push the header's lane words out (above).
  - `reveal()`: the existing `update::history::tests::reveal` tests pass. They assert exactly one row request, and the ask adds none.
  - `walk_progress` restructure (`if awaiting == id && let …` → early return): the same guard; the `update::history` tests pass.
  - The extra request when the hits finish or the walk completes sets `outstanding`. While the walk still streams, `History::is_reading` is true for its round trip, so the header can say reading and the `Esc` ladder treats the walk as reading for that moment. Not tested here; a complete walk (every capture) is unaffected.
  - Ghost Preview, overview strip: tests above. Journal and undo: an ask is not an operation; nothing journaled. Keymap: no key added. graft-ui: untouched; `no_literal_keys` passes.
  - MAP: graft `search` (`Search` fields) and `update::search` rows.
  - Captures: new `ease-pass/main-search-folded-120.txt`; `m8/search-hint`, `m8-bar-after-search` unchanged (stop above).
- **Gap, not built (for the conductor):** a view change other than a reveal (`space` opening or closing a merge, `1`, `H`) does not re-ask. The suffix can go stale until the next reveal, e.g. after `space` opens a merge holding hits. The ticket names three ask points only. One more call to `ask_folded` beside `change_view` in `update/keys.rs` would cover it.
- **Review (scout, read-only, before commit):** it checked all 13 test names in the boxes ticked here, and all 13 exist. It found no defect it could tie to a line. Minor: a search closed with `Esc` leaves its ask riding on requests until answered (one wasted count, nothing drawn).
- **Gates:** fmt and clippy clean. Workspace nextest: 6898/6899 passed. The one failure was `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`: my `Search` MAP row was over 240 characters. I shortened it, and `map_coverage` re-ran 15/15. The full suite was not re-run after that MAP-only edit. Doc tests pass.
- **Broken vs unverified:**
  - Broken: none known.
  - Unverified: the look of `main-search-folded-120.txt` (rule 6) and its colour (rule 5). The "reading" moment in a streaming walk has not been seen. The suffix on a real large repository, where the count lands after the walk completes, has not been tried.
