# G-look polish 2 — folded merge `+N`, the whitespace tail shows only off-default

Status: done (8/9; contact sheet box open — none exists. Builder 2026-10-07.) Was: ready (planner spec-only pass 2026-10-07, from 17's NOTES items 2, 6 and the blame question; decision 2
**approved** by the user 2026-10-07, field notes :92 item 1, ADR-0060). **Re-specced against the scout 2026-10-07**
(`.scratch/m8-polish-and-ship/scouts/17b-fold-count-and-pixel-ghosts.md`): item 6 (Pixel ghosts, ADR-0060) **split
off as 17c** — the graph draws by `view::tier_in_force` (view/frame.rs:84, ~15 callers), not `AppState::tier()`, so
it is its own draw-path change. The file name keeps "pixel-ghosts" for the scout link; the work is not here.
Build with: **Opus** — one new job + command + answer + state + row draw, each held by a named test; no choice left
that the diff can't judge. Not Fable.
Blocked by: 17 (merged). **09g** (building; holds `update/mod.rs` — the new answer routes there, beside
`Product::Leaves` at update/mod.rs:426 and :1017). **12** (building; adds `Product`/`Command` routes, an `AppState`
field, the jobs runner entry and MAP.md lines — the same files this adds to). **Serial with 11c** on MAP.md graft +
graft-ui only (11c shoots `docs/shots/m8/doorway-blame-80x24.txt`, not `m7/blame-*` — scout). 10f is done; no
longer a constraint.
Needs: 17's `whitespace_tail_width` (`detail_pane/header.rs`) and title-tail tests; `graft_git::history_reads::
count_between` (`compare.rs:66`, takes `Oid`s — scout).
**Crate set:** graft (jobs, update fold toggle + answer, `AppState::fold_counts`, `view::rows`, blame title words,
MAP.md) + graft-ui (`graph_column` row text, `detail_pane/header.rs` + `blame.rs` tail rule, MAP.md). No graft-git
or graft-graph change. **Not** `view/frame.rs`, `event_loop/graphics.rs`, `AppState::tier` (17c).

## Read first
- `.scratch/m8-polish-and-ship/scouts/17b-fold-count-and-pixel-ghosts.md` (whole, one page)
- 17's **Decisions** 2, 8 and **NOTES** (this ticket rules on them)
- `docs/design/03-graph-layout-algorithm.md` §3.6 last bullet (:352–356, folds are an inclusion predicate)
- `docs/design/02-keymap.md` §2.5 (:314–325), §2.8 `w` row (:469)
- `crates/graft-graph/MAP.md` `## inclusion` (:62–73); `crates/graft-git/MAP.md` :206–207 (`count_between`);
  `crates/graft/MAP.md` `jobs::remote::read_leaves` (:1049 — the precedent caller), `max_lanes` (:923)

## Decisions (planner)
1. **Item 2 — N's source: one `rev-list --count` per folded merge, async, cached forever by merge id.**
   N = commits the merge brought in = `count_between(exclude: M^1, include: M) − 1` (the range holds M itself).
   One call covers octopus merges (the union of every later parent, no double count). Why this source: it is a fact
   of the merge commit, so it never changes — a `BTreeMap<Oid, FoldCount>` needs **no recount on refresh** and no
   invalidation (the objection in 17's NOTES goes away). The layout can't count (the walk never reads hidden
   commits, `inclusion.rs:213`); a per-row spawn on draw is ruled out, a spawn per *fold* is not — folds are user
   acts. Requested when a merge enters `Inclusion::folded` and has no entry yet. **One insert site** (scout):
   `update/keys/view_change.rs:36`; none at startup. A new insert site added later must request too (NOTES says so
   in MAP.md beside `fold_counts`). Unfolding keeps the entry. Not cancelled when unfolded (cheap; the answer stays
   true).
   - Route: `Command::CountFolded` beside `Command::ReadLeaves` (update/events/command.rs:244); `Product::
     FoldedCounted` handled beside `Product::Leaves` (update/mod.rs:426, :1017).
   - Drawn muted after the subject, one space: `+` while counting, `+3` counted, `+?` failed. N = 0 → no mark.
     The subject is cut (`…`) before the mark; the mark is never cut. All ASCII, every tier. On the cursor row
     through `legible_on` (17 Touches).
   - Draw input: today view/rows.rs:134 passes only `folded.len()` to graft-ui (graph_screen/mod.rs:287); whether the
     row text path sees the commit oid is **unconfirmed** (scout). So graft computes the mark per row in
     `view::rows` (it holds `fold_counts` and the oids) and passes `Option<FoldMark>` on the row's text input
     (`GraphLine`, graph_column/mod.rs:193). graft-ui never looks up a map.
   - Failure: `+?` on the row and one status line `couldn't count <sha7>'s commits: <git's reason>` (rule 3), shown
     once per merge, not per frame.
   - Shallow repo: rev-list counts what is present; accepted, no special case.
2. **Item 6 — moved to 17c** (ADR-0060; the graph tier source is `view::tier_in_force`, not `AppState::tier()`).
3. **Whitespace tail — shown only where the screen departs from its own default.** One rule on all three screens:
   the title tail names a whitespace mode only when it differs from that screen's default. Detail and history
   (default: whitespace counted, §2.5) → `· whitespace ignored` while `w` is on, as 17 built. Blame (default:
   ignored, §2.8 :469, `blame.rs:77`) → nothing by default; `· whitespace counted` after `w`. Why: CLAUDE.md "show it
   only when there's something to report"; 17's always-on blame tail cost 21 title cells on every blame for no news;
   counted-whitespace blame is the state that surprises (reformat commits own the lines), so that is the one named.
   **Defaults confirmed by the scout:** blame ignored `graft-git/src/blame.rs:77`; detail counted
   `config/types.rs:110`; history counted `update/place/history/open.rs:41` — all match; no stop. Tails drawn at
   graft-ui `blame.rs:550–554` (`put_whitespace_tail`, called from `blame.rs:231`, `history.rs:299`) and
   `detail_pane/header.rs:261, :281, :329, :343` — every one goes through `whitespace_tail`.

## Scope
In: decisions 1 and 3; 17's blame captures back to their pre-17 bytes; the blame title asserts 17 changed in
`crates/graft/tests/it/blame_place.rs` back to their pre-17 form — `git diff 428091d9^ -- crates/graft/tests/it/
blame_place.rs` names them (scout: `ends_with` asserts sit at :90 and :256; **:160 is a `contains("· find …")`
assert, not whitespace — leave it**).
Out: item 6 / Pixel ghosts (17c); rasterised ghosts (post-v1).

## Signatures
```rust
// graft::jobs (pub(crate)) — beside remote::read_leaves
pub(crate) fn count_folded(repos: &RepoStore, merge: Oid, cancel: &Cancel) -> Result<usize, String>;
// Command::CountFolded { id: JobId, merge: Oid }  →  Product::FoldedCounted { merge: Oid, count: Result<usize, String> }
// graft::update
pub(crate) enum FoldCount { Counting, Counted(usize), Failed }
// AppState::fold_counts: BTreeMap<Oid, FoldCount>   (field)
// graft-ui::graph_column — the row's text input (`GraphLine`, mod.rs:193) gains `folded: Option<FoldMark>`
pub enum FoldMark { Counting, Count(usize), Unknown }
// graft-ui::detail_pane::header (or where 17 put it)
pub fn whitespace_tail(ignored: bool, default_ignored: bool) -> Option<&'static str>;
```
`count_between` takes `Oid`s (scout), so `M^1` is the merge commit's first parent, read inside the job.

## Acceptance (each tick names its test, at tick time)
- [x] 1: fold a 3-commit merge → row ends ` +` then, after the answer, ` +3` muted; octopus with overlapping sides
      counts the union (fixture, exact N stated). (`graft-ui graph_screen::tests::a_folded_merge_ends_with_its_count`
      — ` +`, ` +3`, ` +?`, `+0` no mark, `TextMuted` fg off-cursor and `on_selection(TextMuted)` on the cursor row;
      `graft jobs::fold_count::tests::count_folded_counts_an_octopus_union` — base ⊕ topic-1, t4 (on topic-1),
      topic-2: per-side 1+2+1 = 4, union **3**; `…count_folded_counts_an_octopus` — corpus octopus **3**;
      end to end: `it look_polish::capture_folded_merge_count_for_docs_shots` pumps `Space` → real job → ` +3`.)
      Test paths differ from the spec: the mark lives on `GraphEntry`, not `GraphLine` (see NOTES).
- [x] 1: refresh (`Ctrl+R`) after an answer spawns no second count (command list asserted).
      (`graft update::fold_count::tests::a_counted_fold_is_not_recounted_on_refresh` — also unfold+fold: no ask)
- [x] 1: failing count → `+?` and exactly one status line naming the sha7.
      (`graft update::fold_count::tests::a_failed_fold_count_says_so_once` — status exact, `Esc`, unfold+fold: no
      second ask/line; `+?` drawn: `a_folded_merge_ends_with_its_count`'s `Unknown` case. The `Failed → Unknown`
      mapping in `view::entries::fold_mark` has no test of its own — same match as the `Counted` arm the it test drives.)
- [x] 1: at 80 columns a long subject is cut before the mark; ASCII tier row has no non-ASCII byte.
      (`graft-ui graph_screen::tests::the_fold_mark_survives_a_cut_subject` — `… +12 `, with a tag `… +12  ✎ changed`;
      ASCII tier: uncut rows `+`/`+3`/`+?` are `is_ascii()`. A *cut* ASCII-tier subject still draws `…` — `fit` has
      no ASCII ellipsis, pre-existing, not the mark.)
- [x] 3: blame default → no whitespace tail; after `w` → `· whitespace counted`; detail/history unchanged.
      (`graft-ui blame::tests::blame_title_names_whitespace_only_when_counted`; 17's
      `detail_pane::tests::detail_title_says_whitespace_ignored_while_w_is_on` and
      `history::tests::history_title_says_whitespace_ignored_while_w_is_on` green, unchanged)
- [x] 3: `git diff 428091d9^ -- docs/shots/m7/blame-*.txt docs/shots/m7/diff-pane-b-with-offer-187x44.txt` empty.
      (regenerated by `GRAFT_UPDATE_SHOTS=1 cargo nextest run -p graft -E 'test(blame) | test(diff_pane_b) |
      test(esc_back)'`, then the same without the variable: 70/70; the diff is empty, `esc-back-to-blame-…` too)
- [x] 3: `git diff 428091d9^ -- crates/graft/tests/it/blame_place.rs` shows no whitespace-tail assert left changed
      (`blame_place.rs:160`'s `· find …` assert untouched). (diff in NOTES)
- [ ] Captures: `docs/shots/m8/polish-fold-120x36.txt` and `docs/shots/m8/polish-blame-whitespace-counted-120x36.txt`
      written (`it look_polish::capture_folded_merge_count_for_docs_shots`,
      `…capture_blame_whitespace_counted_for_docs_shots`). **Not ticked: no m8 contact sheet exists to re-assemble**
      (no `m8-contact-sheet*.txt`, no captions TSV; `scripts/contact-sheet.sh` reads `docs/shots/m8-*.txt`, not
      `docs/shots/m8/`). **Colour unverified** (rule 5) — the muted mark and tails.
- [x] MAP.md graft (`count_folded`, `fold_counts`, the one insert site), graft-ui (`FoldMark`, `whitespace_tail`).
      (doc, checked by reading)

## Touches
- Every `Inclusion::folded` insert site (scout: one, `view_change.rs:36`; re-grep, list in NOTES); the cursor row
  fill (`legible_on`); search/filter matching on subject text (the mark must not be matched — test or grep shown);
  `graph_screen/mod.rs:287`'s `folded.len()` use (unchanged or listed).
- `Command`/`Product` enums and their exhaustive matches (12 adds variants to the same enums — rebase on 12, grep
  every match).
- Blame/detail/history title tests and the captures 17 re-shot (list in its NOTES); `blame_place.rs` per Scope.
- Journal/undo: none (`git diff main -- crates/graft/src/update/journal* crates/graft-ops` empty). Keymap: none.

## Scout questions
Answered in `.scratch/m8-polish-and-ship/scouts/17b-fold-count-and-pixel-ghosts.md`. Still open for the builder
(first step, file:line in NOTES): does the row text path see the commit oid — decides only where `FoldMark` is
attached, not the design (Decision 1 "Draw input").

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~40k: item 2 ~25k (job, command, answer, state, draw, four tests); whitespace + re-shoots ~10k; MAP/NOTES ~5k.

## NOTES (builder, 2026-10-07)
Done, 8/9 boxes (contact sheet: none exists). Scout used (re-checked lines after 09g/09h/12/12b; rebased on 12c + 13h).
No compaction.

- **Scout question answered:** `GraphLine` (graft-ui `graph_column/mod.rs:193`) carries only the lane geometry; the
  subject text is drawn from `GraphEntry` (`graph_screen/mod.rs`, `render.rs` `draw_subject`). So `FoldMark` is
  `graft_ui::graph_screen::FoldMark` on `GraphEntry::fold`, beside `RowTag` — not `graph_column`. graft computes it in
  `view::entries::fold_mark` (reads `history.view.inclusion.folded` and `AppState::fold_counts`). Test paths follow.
- **Draw order:** subject, ` +N` (1-cell gap, muted), then a `RowTag` (2-cell gap). The mark is left off only when it
  plus one subject cell does not fit. `subject_truncated` subtracts it, so the footer reveal still works.
- **Insert sites** (`grep -rn "folded\.\(insert\|extend\|append\)" crates/graft/src`): one, `update/keys/view_change.rs:36`.
  `request` is called from `keys::apply_view_change` after any view change, with the new view's whole folded set, so a
  later insert inside `ViewChange::apply` is covered automatically; one outside it must call `fold_count::request`.
- **Failure status:** `Status::Failed(...)` — a read failure; cleared by the next `JobFinished` like every other read failure.
- **Whitespace:** `graft_ui::blame::whitespace_tail(ignored, default_ignored)` (in `blame.rs` where 17 put the constants,
  not `detail_pane::header`). Blame/history/detail all route through it. graft-ui's blame unit fixture now builds with
  `whitespace_ignored: true` (blame's default) — its snapshots are unchanged.
- **`blame_place.rs` diff vs `428091d9^`:** one hunk — `:160` `ends_with("· find \"refresh\" 8 of 8")` →
  `contains(...)`. Left as the ticket says. **Question for the conductor:** 17 changed it *because of* the blame
  whitespace tail; with the tail gone `ends_with` would hold again (the capture's title ends `8 of 8`). Restoring it is
  a one-word change if wanted.
- **Gates (after rebase on c250612e):** fmt, clippy clean; doctests ok; nextest 5861/5862 — the one failure,
  `graft-fixtures remote::tests::http_remote_frees_its_port_on_drop`, passed alone on rerun; graft-fixtures is untouched
  here, so it looks like a load-only flake (unverified cause — for the debugger's list). Pre-rebase run: 5856/5856.
- **Contact sheet:** not built — inventing captions for ~60 m8 files is the acceptance group's job.

**Touches:** `Command`/`Product` exhaustive matches — compiler-checked (`place/write.rs`, `update/mod.rs` arm + "not a
fact" list, `event_loop/label.rs`, `jobs/runner.rs`); `AppState` field → `update/tests.rs every_field_spelled_out`
updated; cursor row → `ink` (`on_selection`), asserted; search — `search.rs:231` matches `commit.subject()`, never the
drawn `GraphEntry`, so the mark can't match; `view/rows.rs:134 folded.len()` unchanged; graph captures: no existing
capture folds a merge (full nextest green, no shot drifted); journal/undo/keymap/graft-git/graft-graph/`view/frame.rs`/
`event_loop/graphics.rs`: not touched (`git diff main --stat` on them empty).
