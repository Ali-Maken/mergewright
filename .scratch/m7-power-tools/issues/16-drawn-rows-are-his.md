# drawn-rows-are-his — stash internals leave the graph, a commit only a stash reaches is marked as such, and stash commits lose `evil merge: unchecked`

Status: done except the release perf box (the conductor's)
Blocked by: 16a (`Inclusion::with_stashes`), 14a (`only_in_stash`)
Crates: graft (the history view's inclusion, a new read job, the row view, `view::radar`), graft-ui
(`graph_column::NodeKind`, `graph_screen::badges`). **Not** graft-ops, not graft-graph (16a holds it).
Source: note "Pass 2a" R7; gap rows :858, :839. Respecified from scout `.scratch/m7-power-tools/scouts/16.md`. The
force box (:859) moved to **16b**. *Citation fix:* field notes :38 is about Ghost Preview rows ("these are in my
history twice"), not stash rows; :858's evidence is the gap row itself.
Build with: Opus. An inclusion change on every stash push/pop (the `reinclude` path, KNOWN_GAPS "inclusion change
re-reads history") plus a mark that arrives after rows are drawn and must reach the pixel tier's strip cache.
Scout: done (16). No second scout.
Needs: `graft_graph::Inclusion::with_stashes` (16a); `graft_git::history_reads::{only_in_stash, StashOnly}` (14a).

## Today (scout 16, with lines)
- Stashes enter the walk as ordinary tips: `Walk::all_refs` → `commit_tips` (`graft-git/src/repo/refs.rs:122`), so
  `refs/stash` (newest entry) is a tip. Whether `stash@{1..}` (reflog only, `graft/src/refs/mod.rs:59,243`) are tips:
  **not confirmed.** First step below settles it.
- `H`: `update/keys/view_change.rs:45-58` puts every `refs.stashes()` commit in `hidden_stashes`, or clears it.
- `EvilUnchecked`: `graft/src/view/radar.rs:172-199` adds it for any `parents.len() >= 2` with no verdict; no stash
  check. `RefBadges::is_stash(id)` (`refs/mod.rs:72`) exists and the probe already uses it (`update/selection.rs:140`).

## Decided
- **First step (a test, before any change):** `which_stash_entries_are_drawn` — two stashes; record in NOTES whether
  `stash@{1}` is a row. Everything below holds either way: marks only apply to rows that are drawn.
- **Stash parents:** graft sets `Inclusion::with_stashes(<every stash entry's commit>)` on the history view and keeps
  it current when the stash list changes (the same place `hidden_stashes` is set). Index/untracked commits then leave
  the graph (16a's rule).
- **Only in a stash:** after each refs read with ≥ 1 stash, a job runs `only_in_stash(repo, stashes, cap, stop)`
  (newest first, `cap` = 500). While its answer is current, a drawn row whose commit is in it:
  - node `□` (new `NodeKind::StashOnly`), row paint `RowPaint::Dimmed`; **glyph changed from `◇` to `□`, accepted by the conductor 2026-09-27**: `◇` is `NodeKind::Root` (`graft-ui/src/graph_column/mod.rs:123`), and `□` is the hollow form of the stash's `■`;
  - badge `only in stash@{n}` (new `BadgeKind::OnlyInStash(n)`), last in its badge list.
  - Pixel tier: the row's node draws with `Emphasis::Dimmed`; the `◇` shape is the text tiers' only (no new ring).
  - ASCII tier: `*` as every node; the badge words carry it.
  - `capped` → the job's answer is used as far as it goes; NOTES says so. Failure → a problem line
    `couldn't read which commits only a stash holds: <error>`, rows unmarked (rule 3).
  - Stale answer (refs changed since) is dropped by job id, as the other refs jobs.
  - **Why `◇`, not the note's `◌`:** `◌` is the uncommitted-changes row's node in the approved mockups
    (`docs/design/01-view-mockups.md:27,59,81`); two different things must look different. `○`/`◎` are Ghost
    Preview's (graft-ui MAP `RowPaint::Ghost`). `◇` is unused in DESIGN, `docs/design/`, the MAPs (grep).
- **`H`:** unchanged. A hidden stash already hides the commits only it reaches (`decide`, scout 16 §2); the marks
  go with the rows. A regression test, not new work.
- **`EvilUnchecked`:** `radar_lines_for` skips a commit `refs.is_stash(id)` answers for. The detail header draws no
  evil-merge line on a stash. `is_stash` is capped at `stash::MAX_ENTRIES`; an entry past the cap still gets the line
  — NOTES names it as a gap row.

## Read first
- Note "Pass 2a" R7. Scout 16 (all). 16a's "Decided". 14a's `only_in_stash`.
- `crates/graft/MAP.md`: `history` (`View`, `Views`, `RowRequest`), `refs` (`RefBadges::is_stash`, `stashes`),
  `view::radar`, the refs jobs. `crates/graft-ui/MAP.md`: `graph_column` (`NodeKind`, `RowPaint`),
  `graph_screen::badges`. `crates/graft-graph/MAP.md` `inclusion`, `raster::cache` (`layout_hash`).
- `docs/design/02-keymap.md` §2.3 `H`.

## Signatures
```rust
// graft-ui::graph_column
pub enum NodeKind { /* existing */ StashOnly } // glyph '◇'; ASCII '*'
// graft-ui::graph_screen::badges
pub enum BadgeKind { /* existing */ OnlyInStash(usize) } // text "only in stash@{n}"
// graft: calls
graft_graph::Inclusion::with_stashes(self, stashes: BTreeSet<Oid>) -> Inclusion;
graft_git::history_reads::only_in_stash(repo, stashes: &[Oid], cap: usize, stop: &dyn Stop) -> Result<StashOnly, RunError>;
```
If 14's `BadgeKind::CompareBase` has landed, add beside it; the two tickets share `badges.rs` and are serial (both
hold graft).

## Acceptance (each box names its test at tick time)
- [x] `which_stash_entries_are_drawn` (first; records the answer) — `crates/graft/tests/it/stash_in_history.rs::which_stash_entries_are_drawn`: **`stash@{1}` is not a row**, only `stash@{0}` (the walk's tips are refs; older entries are reflog only).
- [x] `stash_index_and_untracked_commits_are_not_drawn` (red first, app level, `stash -u` fixture) — `stash_in_history.rs::stash_index_and_untracked_commits_are_not_drawn`. Red: `waiting for the view's stash entries and its rows: timed out waiting on channel` (the view never learnt the entries).
- [x] `a_commit_only_a_stash_reaches_is_drawn_as_only_in_stash` (red first) — **`□`, not `◇`** (see NOTES), dimmed, `only in stash@{0}` — `stash_in_history.rs::a_commit_only_a_stash_reaches_is_drawn_as_only_in_stash`. Red: `left: Commit, right: StashOnly` (the read answered; nothing drew it).
- [x] `a_rebased_base_on_a_branch_again_loses_the_mark` — a branch created on it → the next refs read unmarks it — `stash_in_history.rs::a_rebased_base_on_a_branch_again_loses_the_mark` (green on first run: written after the read existed).
- [x] `h_hides_the_only_in_stash_rows_too` (regression) — `stash_in_history.rs::h_hides_the_only_in_stash_rows_too`, green from the start.
- [x] `a_failed_stash_only_read_says_so` — `crates/graft/src/update/tests.rs::a_failed_stash_only_read_says_so` (also: an older id's answer is dropped). Green on first run (written after the handler).
- [x] `a_stash_commit_has_no_evil_merge_line` (red first) — `crates/graft/src/view/radar.rs::tests::a_stash_commit_has_no_evil_merge_line`, both entries (`stash@{0}`, `stash@{1}`). Red: panicked with `[EvilUnchecked]`. Updated, named in NOTES: `radar.rs:379,437,464,478`;
      `graft-ui` `detail_pane/tests.rs:1072-1126` unchanged (header drawing is not what moved).
- [x] graft-ui: `graph_column::tests::stash_only_node_draws_a_hollow_square` (renamed from `…_a_diamond`; `│ □ ` Unicode, `| * ` ASCII), `graph_screen::tests::only_in_stash_badge_reads_its_entry` (`[only in stash@{2}]`, Nerd `\u{f01c} only in stash@{2}`). Red: compile (no variant). `special_commits_differ_in_shape` now lists `StashOnly`.
- [x] **Captures:** (`stash_in_history.rs::capture_stash_in_history_{120x40,80x24}`, which assert the Right below before writing) `docs/shots/m7/stash-in-history-120x40.txt` and `-80x24.txt`. **Right:** no `index on` and no
      `untracked files on` rows; exactly one `◇` row with `only in stash@{0}`; the stash's own row still there. Colour
      unverified. Every changed older capture that a fast-tier test writes is re-taken and listed in NOTES (grep
      `docs/shots` for `index on`); others listed as stale, not hand-edited.
- [ ] **Release perf — conductor runs it** (not run by the builder: measurement never runs in a helper). Command: `REV_NOTE='16: stash inclusion + stash-only read' scripts/m7-perf-key.sh docs/perf/drawn-rows-are-his.txt` (12d's script). **The stash push → rows back line has no harness yet**: `m7-perf-key.sh` drives the frames fixture, which has no stash; that line needs a `frames::` scenario (a stash push, then time until `caught_up`) — not written here. Original box: the perf script `12d` added if landed, else `scripts/m4-review-perf.sh`, into
      `docs/perf/drawn-rows-are-his.txt`; summary lines only. Right: frames p99 unchanged within the run-to-run spread
      recorded there; one extra line timing a stash push → rows back (the `reinclude`).
- [x] MAP.md for graft and graft-ui. TESTING.md row (`## M7: stashes in history`). Checked by reading the diff; no test.

## Touches (check each, report how)
- **Strip cache (pixel tier):** a mark that lands after a strip was rasterised must redraw it. Check: whether
  `layout_hash` covers emphasis (graft-graph MAP `raster::cache`); if not, the flush/invalidate the builder uses,
  named, with a test.
- **The stash overlay (`S`), stash badges, `H`:** unchanged except the rows above. Check: stash tests and captures.
- **Ghost Preview slices** (`update::preview` lays out with `graft_graph::layout`): list 16a's grep hits and whether a
  preview over a stash needs `with_stashes`; a preview must not redraw index commits the graph no longer shows.
- **Mistake Radar:** `EvilUnchecked` one fewer per stash; the radar screen's counts. Check: radar tests.
- **Refs jobs:** one more job after a refs read, only with stashes; ids and staleness as the others. Check: a refs
  test with no stash queues nothing new.
- **Keymap, journal/undo, graft-ops:** none.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).

## NOTES (builder, 2026-09-27)

Scout: 16's own (no second, as the ticket says). No compaction.

**`◇` → `□` — accepted by the conductor 2026-09-27.** `◇` is `NodeKind::Root`'s glyph
(`crates/graft-ui/src/graph_column/mod.rs:123`, drawn in dozens of captures, e.g. `docs/shots/38-list.txt:35`);
the ticket's grep covered DESIGN, `docs/design/` and the MAPs, not the code. With `◇` the existing
`graph_column::tests::special_commits_differ_in_shape` fails (`left: 6, right: 7` — two kinds, one shape). Used
`□` (U+25A1, unused anywhere in `crates/`, DESIGN, `docs/design/`): the stash's `■` emptied. One line to change
(`graph_column/mod.rs`, `NodeKind::glyph`) plus the test name and captures if another glyph is wanted.

**Pixel tier — built** (strips.rs granted by the conductor 2026-09-27). `view::strips::Painter::row` sets
`Emphasis::Dimmed` for a row in `AppState::stash_only`; test `crates/graft/src/view/strips/tests.rs::a_stash_only_commit_is_dimmed_in_its_strip`,
red first: `left: Normal, right: Dimmed` (strips/tests.rs:281). Strip cache: `layout_hash` hashes the row's
`emphasis` when no thread is in play (`graft-graph/src/raster/cache.rs:295-298`), so a mark landing after a strip was
rasterised changes the hash and the strip re-rasterises. No flush needed. The test checks the painted row, not pixels.

**Question: row dimming never shows under a selection. This is pre-existing, not something 16 changed.**
`graft-graph/src/raster.rs:130-136` `RasterRow::alpha_at`: with a thread, a column is `1.0` on the thread or
`OFF_THREAD_ALPHA`, and `self.emphasis` is read only when `thread` is `None`. The same is true of the rail
(`raster/pen.rs:254-257`) and of Focus mode's `Dimmed`, which `Painter::row` sets only *inside* a thread
(`strips.rs:355-358`). So at the pixel tier, with a selection, a stash-only row reads as dim only when its lane is off
the cursor's thread (which is usual, since a stash's base is rarely on it). graft-graph is outside 16's set. Is that
enough, or should `alpha_at` take the lower of the two alphas? That is a graft-graph design question.

**Question: are older stash entries meant to be rows?** `which_stash_entries_are_drawn` shows `stash@{1}` is not a
row. 16's spec does not require it: "`which_stash_entries_are_drawn` — two stashes; record in NOTES whether
`stash@{1}` is a row. Everything below holds either way: marks only apply to rows that are drawn." (ticket, Decided).
So no box here. Whether older entries should be drawn is a question for the design.

**What the build does.**
- Inclusion: `update::keys::view_change::with_stash_entries` (next to `H`) gives the view
  `Inclusion::with_stashes(<every refs.stashes() commit>)`; called on every `Product::Refs(Ok)`
  (`update::stash_entries_read` → `History::change_view` + `ShowRows`) and in `history_started` (a walk that starts
  after the refs asks with the entries already set). A stash push/pop changes the set → one `reinclude`.
- Job: `Command::ReadStashOnly(id, stashes)` after each refs read with ≥ 1 stash → `jobs::stash_only::read_stash_only`
  (`only_in_stash`, cap 500) → `Product::StashOnly`. Stale ids dropped (`update::stash_only_answered`). The last
  answer is **kept while the next read runs** (no flicker on every refresh); cost: for the ~200 ms of a re-read a
  commit a branch was just put on can still read `only in stash`. Refs with no stash → marks dropped, nothing asked.
  `capped`: used as far as it goes (the first 500), nothing said on screen. `skipped` (unreadable `^1`): ignored.
- Rows: `view::rows::mark_stash_only` — `□`, `Dimmed` unless the row is highlighted or a drag's (cursor stays
  visible), `only in stash@{n}` last; `geometry::badge_at` indexes refs from the front and `.get()`s, so the extra
  badge is never taken for a ref.
- Radar: `radar_lines_for` skips the evil-merge line on `refs.is_stash(id)`. **Gap row:** an entry past
  `graft_git::stash::MAX_ENTRIES` still gets `evil merge: unchecked` (as the ticket says).
- Radar tests `radar.rs:379,437,464,478` (ticket's list): **unchanged** — none uses a stash (read them); all pass.
  `graft-ui` `detail_pane/tests.rs:1072-1126`: unchanged.

**Files outside the `Files:` line (append-only arms the compiler forces with a new job; no running ticket holds
them — checked 13b's and 01b's `Files:`):** `crates/graft/src/event_loop/label.rs` (2 arms), `crates/graft/src/update/place/write.rs`
(1 arm); `update/keys.rs` one `mod` line (`view_change` → `pub(in crate::update)`); `tests/it/main.rs` one `mod` line.

**Captures re-taken** (fast-tier tests that failed on the change; each diff read — only the stash's index row gone,
header count −1, lanes narrowed; in `fix-radar-bookkeeping` the pull-rebased `ours` now reads
`□ [only in stash@{0}] ours`, the case this ticket is for): `docs/shots/m6/fix-preview-verb-name-prompt-120x40.txt`
(the Ghost Preview slice: built from resident rows, already trimmed — no `with_stashes` needed in `update::preview`),
`m6/fix-radar-bookkeeping-120x40.txt`, `m6/stash-clash-stop-line-80x24.txt`, `m6/stash-clash-take-esc-pane-120x40.txt`,
`m7/30c-relaunch-edits-back-{120x40,80x24}.txt`. Six stable re-runs of those tests + ours.
**Stale, not re-taken** (no fast-tier test writes them; still show `index on`): `docs/shots/08-unicode-graph-column.txt`,
`11-ref-badges.txt`, `15-first-parent-back.txt`, `15-merges-folded.txt`, `15-merges-full.txt`, `15-merges-unfolded.txt`,
`15-stash-hidden.txt`, `35-non-utf8-path-{ascii,unicode}.txt`, `35-stash-{ascii,unicode}.txt`,
`m6/m6-contact-sheet-{1,3,4}.txt`.

Seen, not touched: in the Ghost Preview the stash row draws `●` (was `◉`), not `■` — `view::preview` never applied
the stash shape; pre-existing.
