# G-look polish 2 — folded merge `+N`, ghosts at Pixel, the whitespace tail shows only off-default

Status: draft (planner spec-only pass 2026-10-07, from 17's NOTES items 2, 6 and the blame question)
Build with: _(blank — brownfield, ADR-0035; scout first.)_ Planner expects **Opus**: one new job + state + answer
(item 2), and a tier rule on the draw path that event_loop graphics must obey (item 6). Each is held by a test; no
choice left that the diff can't judge. Not Fable.
Blocked by: 17 (merged). **Serial with 11c** (both re-shoot `docs/shots/m7/blame-*.txt`). Serial with 10f only if
the scout finds the new answer routes through `update/mod.rs` (10f edits one site there).
Needs: 17's `whitespace_tail_width` (`detail_pane/header.rs`) and title-tail tests; `graft_git::history_reads::
count_between` (`compare.rs:66`); F3's tier machinery (`AppState::tier`, `event_loop::graphics`).
**Crate set:** graft (jobs, update fold toggle + answer, `AppState`, `view::rows`/`view::strips`/`view::preview`,
`event_loop::graphics`, blame title words, MAP.md) + graft-ui (`graph_column` row text, `detail_pane/header.rs`
tail rule, MAP.md) + one sentence in `docs/design/01-view-mockups.md` §1.7. No graft-git or graft-graph change.

## Read first
- 17's **Decisions** 2, 6, 8 and **NOTES** (this ticket rules on them)
- `docs/design/01-view-mockups.md` §1.7 (:279–295); `docs/design/03-graph-layout-algorithm.md` §3.6 last bullet
  (:352–356, folds are an inclusion predicate), §3.8 (:371–423, strips; alpha in the key)
- `docs/design/02-keymap.md` §2.5 (:314–325), §2.8 `w` row (:469)
- `crates/graft-graph/MAP.md` `## inclusion` (:62–73); `crates/graft-git/MAP.md` :206–207 (`count_between`);
  `crates/graft/MAP.md` `jobs::remote::read_leaves` (:1049 — the precedent caller), `view::strips` (:70–73),
  `AppState::tier` (:253), `max_lanes` (:923), Ghost Preview `update::preview` (:545–573)

## Decisions (planner)
1. **Item 2 — N's source: one `rev-list --count` per folded merge, async, cached forever by merge id.**
   N = commits the merge brought in = `count_between(exclude: M^1, include: M) − 1` (the range holds M itself).
   One call covers octopus merges (the union of every later parent, no double count). Why this source: it is a fact
   of the merge commit, so it never changes — a `BTreeMap<Oid, FoldCount>` needs **no recount on refresh** and no
   invalidation (the objection in 17's NOTES goes away). The layout can't count (the walk never reads hidden
   commits, `inclusion.rs:213`); a per-row spawn on draw is ruled out, a spawn per *fold* is not — folds are user
   acts. Requested when a merge enters `Inclusion::folded` and has no entry yet (every insert site — scout lists
   them); unfolding keeps the entry. Not cancelled when unfolded (cheap; the answer stays true).
   - Drawn muted after the subject, one space: `+` while counting, `+3` counted, `+?` failed. N = 0 → no mark.
     The subject is cut (`…`) before the mark; the mark is never cut. All ASCII, every tier. On the cursor row
     through `legible_on` (17 Touches).
   - Failure: `+?` on the row and one status line `couldn't count <sha7>'s commits: <git's reason>` (rule 3), shown
     once per merge, not per frame.
   - Shallow repo: rev-list counts what is present; accepted, no special case.
2. **Item 6 — at Pixel, an open Ghost Preview draws the whole graph column at the Unicode tier.** 17's decision 6
   (text for ghost rows only, strips for real rows) is withdrawn: strips are 32-row aligned bands, so a ghost row
   mid-strip needs the strip punched, and a real lane crossing a ghost row would be half pixel, half text — §1.7 asks
   for "one continuous picture, not two separate graphs". One rule instead: `graph_tier()` = Unicode while a preview
   with ghost rows is open and `tier()` is Pixel; else `tier()`. The preview is modal and short; on close the strips
   re-place from the cache (no re-raster while the budget kept them). The header's `gfx` field and F3 keep reading
   `tier()` (the tier in force did not change). F3 during the preview: `graph_tier()` = `min(tier(), Unicode)`.
   - DESIGN §1.7 "dashed stroke in the real renderer" → carried past v1 (pixel-rasterised ghosts). §1.7 gains one
     sentence: "*At the Pixel tier the graph column draws at the Unicode tier while a preview is open.*" (planner
     amendment). Builder's NOTES asks the conductor for the KNOWN_GAPS row `M8/pixel-ghosts-text`.
3. **Whitespace tail — shown only where the screen departs from its own default.** One rule on all three screens:
   the title tail names a whitespace mode only when it differs from that screen's default. Detail and history
   (default: whitespace counted, §2.5) → `· whitespace ignored` while `w` is on, as 17 built. Blame (default:
   ignored, §2.8 :469, `blame.rs:77`) → nothing by default; `· whitespace counted` after `w`. Why: CLAUDE.md "show it
   only when there's something to report"; 17's always-on blame tail cost 21 title cells on every blame for no news;
   counted-whitespace blame is the state that surprises (reformat commits own the lines), so that is the one named.
   The scout confirms each screen's default with file:line (rule 7); a screen whose default differs from this list is
   a stop.

## Scope
In: decisions 1–3; 17's blame captures back to their pre-17 bytes; `blame_place.rs:160` back to `ends_with`.
Out: rasterised ghosts (post-v1); the rewrite editor's right side (report only — does it draw strips at Pixel? file:line).

## Signatures
```rust
// graft::jobs (pub(crate)) — beside remote::read_leaves
pub(crate) fn count_folded(repos: &RepoStore, merge: Oid, cancel: &Cancel) -> Result<usize, String>;
// Command::CountFolded { id: JobId, merge: Oid }  →  Product::FoldedCounted { merge: Oid, count: Result<usize, String> }
// graft::update
pub(crate) enum FoldCount { Counting, Counted(usize), Failed }
// AppState::fold_counts: BTreeMap<Oid, FoldCount>   (field)
impl AppState { pub(crate) fn graph_tier(&self) -> Tier; }   // decision 2; `Tier` = whatever `tier()` returns
// graft-ui::graph_column — the row's text input gains `folded: Option<FoldMark>`
pub enum FoldMark { Counting, Count(usize), Unknown }
// graft-ui::detail_pane::header (or where 17 put it)
pub fn whitespace_tail(ignored: bool, default_ignored: bool) -> Option<&'static str>;
```
`M^1` is resolved inside the job (by revspec if `count_between` takes one, else the commit's first parent — scout).

## Acceptance (each tick names its test, at tick time)
- [ ] 1: fold a 3-commit merge → row ends ` +` then, after the answer, ` +3` muted; octopus with overlapping sides
      counts the union (fixture, exact N stated). (`graph_column::tests::a_folded_merge_ends_with_its_count`,
      `jobs::tests::count_folded_counts_an_octopus_union`)
- [ ] 1: refresh (`Ctrl+R`) after an answer spawns no second count (command list asserted).
      (`update::tests::a_counted_fold_is_not_recounted_on_refresh`)
- [ ] 1: failing count → `+?` and exactly one status line naming the sha7. (`update::tests::a_failed_fold_count_says_so_once`)
- [ ] 1: at 80 columns a long subject is cut before the mark; ASCII tier row has no non-ASCII byte.
      (`graph_column::tests::the_fold_mark_survives_a_cut_subject`)
- [ ] 2: Pixel fixture, preview open → frame bytes hold no image placement for the graph; text layer holds `┊`
      ghost glyphs; close → strips placed again, zero new rasters. (`event_loop::tests::pixel_scroll::
      a_preview_at_pixel_draws_the_unicode_graph_and_places_no_strip`, `…::closing_the_preview_replaces_the_strips`)
- [ ] 2: mouse hit-test and draw agree on the graph column width during the preview (click a ghost-free real row,
      assert the row hit). (`view::tests::preview_hit_test_uses_the_graph_tier`)
- [ ] 3: blame default → no whitespace tail; after `w` → `· whitespace counted`; detail/history unchanged.
      (`blame::tests::blame_title_names_whitespace_only_when_counted`, 17's two detail/history tests stay green)
- [ ] 3: `git diff 428091d9^ -- docs/shots/m7/blame-*.txt docs/shots/m7/diff-pane-b-with-offer-187x44.txt` empty,
      or each differing file and line listed in NOTES with its cause. (capture regeneration, command in NOTES)
- [ ] Captures: `docs/shots/m8/polish-fold-120x36.txt`, `docs/shots/m8/polish-ghost-pixel-120x36.txt` (text layer),
      `docs/shots/m8/polish-blame-whitespace-counted-120x36.txt`; m8 contact sheet re-assembled. **Colour
      unverified** (rule 5) — the muted mark and tails.
- [ ] §1.7 sentence; MAP.md graft (`count_folded`, `fold_counts`, `graph_tier`), graft-ui (`FoldMark`,
      `whitespace_tail`). (doc, checked by reading)

## Touches
- Every `Inclusion::folded` insert site (grep, list in NOTES); the cursor row fill (`legible_on`); search/filter
  matching on subject text (the mark must not be matched — test or grep shown).
- Every caller of `tier()` on the graph draw and hit-test path → `graph_tier()` (grep each, NOTES lists which moved
  and which stayed); F3; `event_loop::graphics` placement/clear; strip cache budget (no new entries for the preview).
- Blame/detail/history title tests and the captures 17 re-shot (list in its NOTES); `blame_place.rs:160`.
- Journal/undo: none (`git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty). Keymap: none.

## Scout questions
- Every site that inserts into `Inclusion::folded` (file:line); are folds ever set without `space` (startup)?
- `count_between`'s parameter types: Oid or revspec? Does a row's draw input carry the commit oid?
- What clears Kitty/iTerm2 placements when F3 leaves Pixel (file:line)? Does the mouse hit-test read `tier()`?
- Each screen's whitespace default: detail, history, blame (file:line).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~55k: item 2 ~25k (job, command, answer, state, draw, four tests); item 6 ~15k; whitespace + re-shoots ~10k;
MAP/NOTES ~5k. If the scout finds the tier switch reaches deep into graphics, item 6 splits off as 17c.
