# fold-seek-reveal — going to a commit inside a folded merge opens that merge and lands on it

Status: ready
Blocked by: 06-graph-fold-and-lanes
Build with: Opus (`builder-opus-xhigh`) — the history worker's seek and serve; checkable by state tests per route.
Needs: from 06: `FoldWalk::behind`, `OpenedMerges::open`, `Inclusion::opened_mut`, the `space` open path (`reinclude`). Existing: `History::seek` (`graft/src/history/scroll/query.rs:71-111`), `Wanted::Find`, serve (`history/serve.rs:180-181,213-219`), `scroll/apply.rs:156-161`.

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviour G4; Hard-to-reverse choice 2 (as amended 2026-10-17, and its 2026-10-18 fold rule); "Mock gaps" paragraph, its 2026-10-18 search sentence.
- `crates/graft-graph/MAP.md` `inclusion` (`Included::walked`/`shown`). `crates/graft/MAP.md`: `history::scroll`, `history::state`, `update::search`, `update::map`, `update::palette`, `update::detail`, `update::banner`, `update::operate`.
- 06's scout file, "Seeks to a possibly hidden commit".

## Why (scout: today every route says "not in this view" or nothing)
With folding the default, a merge's second parent, a search hit on a merged PR, a `J` put-back on a merged branch are hidden in the everyday case — not, as before, only after he chose to fold. Each route today lands nowhere (search: footer `(not in this view)`, `graft/src/search.rs:332-336`; palette, Map, `J`, detail links: silent) **and** first lays out the whole history to look (`serve.rs:180-181`, `usize::MAX`). Planner decision (`docs/research/ux-principles.md`: visibility of system state, fewest steps): a commit hidden **by the default fold** is revealed — its merge opens, the cursor lands on it, 0 extra keys; the alternative, a miss he must answer by finding the merge and pressing `space`, costs a search he can't do (the merge isn't named for the commit). A commit hidden by a view **he** chose (first-parent `1`, hidden stashes) keeps today's answer.

**Search count (added 2026-10-18).** Observed: `/topic` says `4 matches` while 3 of them are inside folded merges and one row is lit. The count is right — every one of the 4 is a hit and, with this ticket, `n` reaches each — but the screen gives no reason three are not visible. So the count stays; the footer says how many are folded, and `n`/`N` are pinned to visit every counted hit.

## Decided here
- **Find stops when the id is walked, not when it has a row.** Serve lays out until `Included::walked` holds the id (or the walk ends), then answers: a row → as today; hidden by the fold → `Folded { open }`, `open` = the `behind` chain from the id outward until a shown merge, outermost first; hidden by a chosen view → today's miss.
- **Reveal = `space` on each merge in `open`, as one view change** (one `reinclude`), then the same seek again; it lands. The revealed rows are ordinary opened rows (`space` closes, cursor on the merge).
- **Routes:** search `n`/`N` (`update/search.rs:178-182,191-209`), palette commit find (`update/palette/run.rs:53-56` → `map::seek_to`, `update/map.rs:202-208`), Map `⏎`, `J` put-back landing (`update/operate.rs:199-200` → `update/banner/stop.rs:348-375`), detail parent links (`update/detail/landing.rs:119,142-151`): all reveal. Their misses for a chosen view stay as today (search's footer words; the others' silence — unchanged, not in scope).
- **Search count and order (2026-10-18):** the count is every walked hit, folded ones included (as today). While ≥1 counted hit is hidden by the default fold, the footer appends ` · N in folded merges` (status rule: absent at 0; recounted after each reveal, so it falls as `n` opens merges). Hits hidden by a chosen view (`1`) keep today's count and words — no suffix. `n`/`N` order is walk (emitted) order, which is the row order once every merge is opened; `n` from the last hit wraps as today.
- **Unchanged, by test:** overview strip marks (resident rows only, `view/strip.rs:17-24`); tour's HEAD seek (`tour.rs:178`; HEAD is a keep tip); **Ghost Preview** slices (`view/preview.rs:3-29` draw the preview's own slice and never consult `Inclusion` — scout grep), so a preview shows every commit the operation touches, folded or not (M5's promise).

## Scope
In (graft `history::scroll`, `history::serve`, `history::state` (the miss answer), `update::search` (+ the footer suffix), `update::map`, `update::palette` (seek result only), `update::detail::landing`, `update::banner::stop` (`land_on_head` result only); graft-graph `inclusion` (one query on `Included`); graft-ui search footer words (suffix only)).
Out: what folds (06); header and lanes (06b); words for a chosen-view miss on the silent routes (unchanged); marking a folded merge row that hides a hit (not in the mock; not decided).

## Signatures
graft-graph `inclusion`:
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Whereabouts { Shown, Folded { open: Vec<Oid> /* outermost first */ }, Filtered, NotWalked }
impl<S: CommitSource> Included<S> {
    /// Where a walked commit stands in this view; `Folded` lists the merges to open to show it.
    pub fn whereabouts(&self, id: Oid) -> Whereabouts;
}
```
graft app (`history`): the seek's miss answer gains `Folded { open: Vec<Oid> }` beside today's not-in-this-view (names follow `history::state`'s; payload as given). Search: the hit summary the footer reads gains `folded: usize` (hits whose `whereabouts` is `Folded`), named in NOTES with its file and line.

## Touches
Every `History::seek` caller (scout's list, above); `scroll/apply.rs:156-161` cursor-stays path (now only for a chosen-view miss); search state across a reveal (the hit list stays valid: same ids); `n` repeated through one PR's commits opens that merge once; the search footer (captures `m8-bar-after-search-120x36`, `m8/search-hint-120x36` — re-shot only if their fixture has a folded hit, D3 rule in 06; `graft-ui` `no_literal_keys` allow-list if the suffix trips it); the overview strip; Ghost Preview; `J`'s landing words; the full-history layout on a miss (now only for an id never walked — one number in NOTES: rows laid out for a seek to a walked hidden commit, before/after).

## Acceptance
- [ ] Detail pane on a folded merge, follow parent 2 → that merge opens, cursor on parent 2 (state test).
- [ ] Search for a subject on a merged PR's commit → merge opened, cursor on the hit; `n` to the next hit in the same PR opens nothing new.
- [ ] Search with 4 hits, 3 inside folded merges (two merges): footer reads `4 matches · 3 in folded merges`; `n` from the first visits all 4 in walk order, each folded one revealed and landed; after the walk the footer reads `4 matches` (no suffix). State test plus footer-words test.
- [ ] Search with no folded hit: footer has no suffix (today's words, byte-equal).
- [ ] Palette commit find and Map `⏎` on a hidden commit → revealed and landed (one test each).
- [ ] `J` put-back of a commit on a merged branch → lands on it, merge opened.
- [ ] Nested: a commit behind a merge that is itself behind a folded merge → both open, cursor on it.
- [ ] First-parent view `1`, search hit off the first-parent line → today's `(not in this view)`, nothing opened, no `in folded merges` suffix.
- [ ] A seek to a walked hidden commit lays out no further than the walk already is (row count before/after the answer).
- [ ] Ghost Preview slice identical with every merge folded and with every merge opened (row-set test); overview strip marks unchanged.
- [ ] Capture `docs/shots/ease-pass/main-search-folded-120.txt`: the footer with the suffix before `n`, cursor on the first hit (06e's `wallet_release` fixture, 120×36). Look unjudged (rule 6), colour unverified (rule 5).
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~55k.
