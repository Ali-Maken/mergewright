# drawn-rows-are-his — stash internals leave the graph, a commit only a stash reaches is marked as such, and stash commits lose `evil merge: unchecked`

Status: draft
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
  - node `◇` (new `NodeKind::StashOnly`), row paint `RowPaint::Dimmed`;
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
- [ ] `which_stash_entries_are_drawn` (first; records the answer).
- [ ] `stash_index_and_untracked_commits_are_not_drawn` (red first, app level, `stash -u` fixture).
- [ ] `a_commit_only_a_stash_reaches_is_drawn_as_only_in_stash` (red first) — `◇`, dimmed, `only in stash@{0}`.
- [ ] `a_rebased_base_on_a_branch_again_loses_the_mark` — a branch created on it → the next refs read unmarks it.
- [ ] `h_hides_the_only_in_stash_rows_too` (regression; green from the start is fine — say so at tick time).
- [ ] `a_failed_stash_only_read_says_so`.
- [ ] `a_stash_commit_has_no_evil_merge_line` (red first). Updated, named in NOTES: `radar.rs:379,437,464,478`;
      `graft-ui` `detail_pane/tests.rs:1072-1126` unchanged (header drawing is not what moved).
- [ ] graft-ui: `stash_only_node_draws_a_diamond`, `only_in_stash_badge_reads_its_entry`.
- [ ] **Captures:** `docs/shots/m7/stash-in-history-120x40.txt` and `-80x24.txt`. **Right:** no `index on` and no
      `untracked files on` rows; exactly one `◇` row with `only in stash@{0}`; the stash's own row still there. Colour
      unverified. Every changed older capture that a fast-tier test writes is re-taken and listed in NOTES (grep
      `docs/shots` for `index on`); others listed as stale, not hand-edited.
- [ ] **Release perf:** the perf script `12d` added if landed, else `scripts/m4-review-perf.sh`, into
      `docs/perf/drawn-rows-are-his.txt`; summary lines only. Right: frames p99 unchanged within the run-to-run spread
      recorded there; one extra line timing a stash push → rows back (the `reinclude`).
- [ ] MAP.md for graft and graft-ui. TESTING.md row.

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
