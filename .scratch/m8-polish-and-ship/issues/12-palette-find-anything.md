# G-palette 2a — `⌃P` finds branches and tags; COMMANDS RUN RECENTLY; the find job

Status: partial (17/18 ticked; the perf box is open, see NOTES) — was: ready (re-spec 2026-10-07, spec-only pass; replaces the 2b spec — see "Why re-specced"; re-aligned the same
day to `docs/research/palette-find.md` — see "Aligned to prior art"; P-find-1 and P-find-2 **approved** 2026-10-07,
field notes :92, ADR-0061)
Build with: **Opus** — design-heavy: a per-keystroke job contract (cancel, stale drop, re-filter), cursor anchoring;
not Fable (correctness is judged from tests: stale ids, cancel commands, HEAD unchanged).
Blocked by: 11b (same `update/palette.rs`, `graft-ui/src/palette.rs`, palette captures). P-find-1 and P-find-2
**approved** 2026-10-07 (ADR-0061): build the approved branch; the "declined" branch below is not built.
Needs: 11's `PaletteState`, `PaletteTarget`, `Group`, `rows`/`listing`, `run`, `NOTHING_MATCHES`;
`graft_core::matcher::fuzzy_score`; `update::map::seek_to` (`pub(super)`, update/map.rs:202);
`graft_git::switch_refs::switch_refs` (as `graft_ops::switch_list` calls it; `SwitchRefs`, `RefListing { name, tip,
when, .. }`); `refs::RefBadges::tags` (refs/mod.rs:123); `AppState::refs: Option<Arc<RefBadges>>`.

## Why re-specced (scout `.scratch/m8-polish-and-ship/scouts/12-find-anything.md`, checked by the planner)
- `PaletteTarget` has `Note` (update/palette.rs:59) and derives `Copy` (:52); the old spec dropped one, broke the other.
  Now: **`Note` kept, `Copy` kept** — no `PathBuf` in the target (files are an index, 12b).
- `/` is a job walking every ref (search.rs:142 `scan`, `Walk::all_refs` :153); history holds a window only. There is
  no in-memory commit list to filter per key. Commits move to 12b, on a job.
- No tracked-paths read exists (diff/read.rs:276 is `--others`). Files move to 12b.
- `⇥` seek handles a tip not laid out (`seek_to`: `History::seek` + `Command::ShowRows`, map.rs:208). Reused as is.

## Aligned to prior art (2026-10-07; design note Prior art "Find: …" rows)
- **Branch order**: Magit #2872 (alphabetical is wrong; newest first). `RefBadges` has no dates, so BRANCHES now come
  from `graft_git::switch_refs` (every local and remote branch, committer date), read **once per `⌃P` open** on a
  job — the same rule as the `C` picker (`filter_switch_rows`: segment start first, ties newest tip), so one query
  orders alike in both (H4). Tags stay on resident `RefBadges::tags`.
- **Header**: `COMMANDS RUN RECENTLY`, as DESIGN §1.6 draws it (P-find-2): §1.15 already calls the repo list
  `RECENT`, and the palette opens over it. Rows are action rows, no stored argument (P-find-2).
- **Journeys**: J26 `seek-branch` 3, J27 `seek-tag` 3, J31 `rerun-recent` 2 — the note's `journey-counts` fence.

## Read first
- `docs/design/01-view-mockups.md` §1.6 (lines 257–284, the ADR-0061 paragraph included); `docs/design/02-keymap.md`
  §2.12 (lines 528–544)
- `docs/adr/0061-the-empty-palette-opens-on-recent-commands-and-stores-no-arguments.md` (Decision)
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows "Find: …"; Journeys rows 26–32, "Find rows judged",
  "Where he lands" rows "Palette …", the `journey-counts` fence; Where I'd differ P-find-1, P-find-2; New UI
  "Palette sources"
- `docs/research/palette-find.md` "Take / reject candidates" (lines 59–67)
- `crates/graft/MAP.md`: `update::palette` (691), `update::map` `seek_to` (687), `refs` (104), `jobs` (1013, the
  `switch_list` job :1042 as the pattern), `trace` (126); `crates/graft-git/MAP.md`: `switch_refs` (115);
  `crates/graft-ops/MAP.md`: `switch_list` (384, the ranking rule); `crates/graft-ui/MAP.md`: `palette`;
  `crates/graft-core/MAP.md`: `fuzzy_score` (81)
- The scout file above. Scout again (≤1 page) only if 11b changed `palette.rs` past the line numbers cited here.

## Decisions (planner)
**Cost per keystroke** (PROMPT §6: input to frame < 16 ms; must hold at 50k+ refs). The loop never scans refs, paths or
history. Per key it does: actions + COMMANDS RUN RECENTLY filter (≤ ~200 rows, as today), re-filter of the last
answer's shown rows (≤ 50 per group), one `CancelJob` + one `Command::PaletteNames`. Matching 50k ref names runs on a
worker. Every shown row always matches the current query (re-filter), so `⏎` never runs a row the query excludes. A
group whose answer is for an older query shows its re-filtered rows plus a muted `searching…` row in place of
`+N more` (H1, UX §2).

| Source | Where it comes from | Rows | `⏎` |
|---|---|---|---|
| Actions | `listing` (11), unchanged | as 11 | as 11 |
| BRANCHES | `switch_refs`, one read per `⌃P`/`:` open (`Command::ReadPaletteBranches`), held as `Find::branches`; matched by the names job | `main`, `origin/feat` (short name) | `Seek(tip)` via `seek_to`; never switches (`C` does) |
| TAGS | `RefBadges::tags` (resident `Arc`), same names job | `v2.1.0` | `Seek(tip)` |
| COMMANDS RUN RECENTLY | `AppState::palette_recent`, in memory, this session | the action row as 11 draws it (key, label, config id) | same `Action`/`App` target, its key's path |

- **Group order** (§2.12): `YOUR CONFIG` (11) first when present. Typed: `ACTIONS`, `BRANCHES`, `TAGS`, (12b: `FILES`,
  `COMMITS`), `COMMANDS RUN RECENTLY`. Fixed across groups; §1.6's "then recency" applies inside groups (branches by
  tip date, recent runs newest first). `?` mode: no find groups, no recent group (§2.12).
- **Empty query (P-find-1, approved)**: `COMMANDS RUN RECENTLY` (if any), then `ACTIONS`; no find groups; cursor on the
  first runnable row (11), so `⌃P ⏎` reruns the newest. The empty field draws, muted, `type to find actions, branches,
  tags, files, commits` (graft-ui; gone on the first character). **Not built (P-find-1 approved; kept for the
  record)** — had it been declined: §2.12's order —
  `ACTIONS`, then the find groups for the empty query (the same names job, `fuzzy_score("")` = `Some(0)`, so newest
  branches first), then `COMMANDS RUN RECENTLY`; no placeholder; set the fence's `rerun-recent` to 3.
- **Inside a group**: `fuzzy_score` (lower first). Branches: then local before remote, then newer `when`, then name
  (the `C` picker's tie rule, plus local first). Tags: then name.
- **COMMANDS RUN RECENTLY**: last 8 distinct `Action`/`App` targets run from either palette mode, newest first; a rerun
  moves to the front. Typed query: filtered like actions, may repeat an ACTIONS row (the §1.6 drawing does). Header
  exactly `COMMANDS RUN RECENTLY` (DESIGN §1.6). No argument stored: `⏎` runs as the key would (§2.12, P-find-2).
- **Caps**: 8 rows per find group; `+N more` (`More(group)`) widens that group to 50, cursor on the first new row; past
  50 the last row is `+N more · type more to narrow` (`Note`). Query change resets the widening.
- **Cursor**: anchored by `(group header, target)` on a runnable row; re-found when an answer lands, else index
  clamped. A cursor on a placeholder (`reading…`, `searching…`) moves to the first runnable row when one lands.
  `⏎` on a placeholder does nothing.
- **Not read yet**: BRANCHES shows one muted `reading…` row until `switch_refs` lands (then the names job is re-asked
  for the current query); TAGS the same while `AppState::refs` is `None`. Read failure: BRANCHES reads
  `branches: <reason>` (muted, `Note`); other groups unaffected (H9).
- **Nothing matches**: `NOTHING_MATCHES` (11) only once every asked source has answered for this query and all are
  empty; before that, the placeholders.
- **Lands** (UX §3.1): seek row `⏎` → palette closed, graph focused, cursor on the commit, detail follows; not laid out
  → existing jump (`jumping`, `Esc` withdraws); walk ends without it → `seek_to`'s existing fallback, unchanged.

## Scope
In: target arms, branches read job (cancel on close), names job (branches + tags; 12b adds paths to the same
request), stale drop, cancel on edit and close, re-filter, `+N more`, cursor anchor, COMMANDS RUN RECENTLY, empty-box
placeholder, `RowKind::Branch/Tag/Recent/More` drawing, walks for `seek-branch`/`seek-tag`/`rerun-recent`, perf
script, capture.
Out: files, commits (12b); switching from the palette; persistent recent runs; stored arguments; kind prefixes
(rejected, note Prior art); cross-group ranking; MRU (reflog) branch order.

## Signatures
```rust
// graft — update::palette
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaletteTarget {
    Action(ActionId), App(AppAction), Note,  // unchanged (11)
    Seek(Oid),                               // branch, tag (12), commit (12b) rows
    More(FindGroup),                         // `+N more`
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindGroup { Branches, Tags, Files, Commits } // Files/Commits used by 12b
/// One read per `⌃P` open, cancelled on close. `PartialEq`: `Read` compares by `Arc::ptr_eq` (manual impl — no
/// derives added to graft-git types).
#[derive(Clone, Debug, Default)]
pub(crate) enum ReadOnce<T> { #[default] NotAsked, Reading(JobId), Read(Arc<T>), Failed(String) }
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Find {
    pub branches: ReadOnce<graft_git::switch_refs::SwitchRefs>,
    pub names: Asked<Arc<NameHits>>,
    pub widened: Option<FindGroup>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Asked<T> { pub pending: Option<(JobId, String)>, pub answer: Option<(String, T)> } // query each is for
// PaletteState gains `pub find: Find` (Default for `?`).
pub(crate) struct RecentRuns(VecDeque<PaletteTarget>);           // in AppState as `palette_recent`
impl RecentRuns { pub(crate) fn ran(&mut self, t: PaletteTarget); pub(crate) fn iter(&self) -> impl Iterator<Item = PaletteTarget> + '_; }
pub const RECENT_CAP: usize = 8; pub const GROUP_ROWS: usize = 8; pub const WIDE_ROWS: usize = 50;
pub const RECENT_HEADER: &str = "COMMANDS RUN RECENTLY";
pub const FIND_PLACEHOLDER: &str = "type to find actions, branches, tags, files, commits"; // P-find-1 only

// graft — jobs::palette (new private mod), dispatched by JobRunner like `switch_list`
pub(crate) fn read_branches(repos: &RepoStore, cancel: &Cancel) -> Result<Arc<SwitchRefs>, String>; // switch_refs off the loop
pub(crate) struct NamesRequest {
    pub query: Arc<str>,
    pub branches: Option<Arc<SwitchRefs>>,  // None until read
    pub tags: Option<Arc<RefBadges>>,       // None until the refs job answers
}                                           // 12b adds `files`
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct NameHits { pub branches: Ranked<RefHit>, pub tags: Ranked<RefHit> }      // 12b adds `files`
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct Ranked<T> { pub top: Vec<T>, pub total: usize }                          // top ≤ WIDE_ROWS, best first
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RefHit { pub name: Box<[u8]>, pub tip: Oid, pub remote: bool }           // short name
pub(crate) fn match_names(req: &NamesRequest, stop: &dyn Fn() -> bool) -> Option<NameHits>; // None = stopped; checks stop every 4096
// update: Command::ReadPaletteBranches(JobId) → AppEvent::Produced(id, Product::PaletteBranches(Result<Arc<SwitchRefs>, String>))
//         Command::PaletteNames(JobId, NamesRequest) → AppEvent::Produced(id, Product::PaletteNames(Arc<NameHits>))

// graft-ui — palette
pub enum RowKind { Action, App, Branch, Tag, File, Commit, Recent, More } // + More: muted (`+N more`, `reading…`, `searching…`)
// PaletteView: the query field draws `FIND_PLACEHOLDER` muted when empty in `Everything` mode (P-find-1).
```
`run` (palette.rs:479): `Seek(oid)` → close, `super::map::seek_to`; `More(g)` → widen; `Action`/`App` → `palette_recent.ran`
then today's path. Open in `Everything` mode → `Command::ReadPaletteBranches`; `?` asks none; close cancels both jobs.

## Acceptance (each box names its test at tick time)
- [x] `⌃P` `feat` `⏎` on a branch row → cursor on its tip, HEAD unchanged (`enter_on_branch_row_seeks_tip_and_keeps_head`, reads HEAD with git).
- [x] Remote branch and tag rows seek their commit (`enter_on_remote_and_tag_rows_seek`).
- [x] Tip not laid out → `Command::ShowRows`, cursor lands when rows arrive (`branch_tip_not_laid_out_seeks_through_the_walk`).
- [x] `⌃P` asks one `ReadPaletteBranches`; `?` asks none; close cancels it (`palette_open_reads_branches_once`); BRANCHES `reading…` until it lands, then rows for the current query (`branches_group_reads_then_lists`); read failure reads in the group (`branch_read_failure_reads_in_branches_group`).
- [x] Branch order: score, local before remote, newer tip, name — same order as `filter_switch_rows` for local rows on one fixture (`branches_rank_local_then_newest_tip`, `palette_and_picker_order_branches_alike`).
- [x] Empty query (P-find-1, approved) → `COMMANDS RUN RECENTLY` (if any) then `ACTIONS`, no find rows, placeholder drawn (`empty_query_lists_recent_then_actions_only`, `empty_everything_field_names_what_it_finds`). No declined-branch test (ADR-0061).
- [x] `?` has neither find groups nor the recent group (`help_palette_has_no_find_groups`).
- [x] Recent group: header reads exactly `COMMANDS RUN RECENTLY` (`recent_group_header_is_commands_run_recently`); run stash from the palette, reopen → that group first, `⏎` opens the stash prompt (`recent_row_runs_its_key_path`); cap 8, newest first, rerun moves to front (`recent_runs_cap_and_order`).
- [x] Each query edit cancels the previous names job and asks one new one; close cancels it (`query_edit_cancels_the_previous_find_job`, `closing_the_palette_cancels_find_jobs`).
- [x] Answer for an older id dropped (`stale_names_answer_is_dropped`); every shown row matches the current query while an answer is pending (`shown_rows_always_match_the_current_query`).
- [x] `+N more` widens to 50, cursor on first new row; query edit resets (`more_row_widens_then_resets`).
- [x] Cursor stays on its row when an answer lands above it (`cursor_stays_on_its_row_when_an_answer_lands`); a cursor on `reading…` moves to the first runnable row that lands; `⏎` on a placeholder does nothing (`cursor_on_placeholder_moves_to_first_landed_row`).
- [x] `nothing matches · esc close` only after every asked source answered empty (`nothing_matches_waits_for_every_source`).
- [x] `match_names`: segment start first, local before remote, newer tip, totals right, stops when asked (`names_rank_and_total`, `names_stop_when_cancelled`).
- [x] **Journeys, from the fence.** Each walk is a pty test that contains its fence id as a string literal, reads the
      count with `m5_walk::parse_counts` from the note's `journey-counts` fence, replays the keys and fails if the key
      list is longer (so `accept-m8`'s `WALKS` can map it): `seek-branch` 3 (`⌃P` name `⏎`, cursor on the tip, HEAD
      unchanged: `walk_seek_branch`); `seek-tag` 3 (`walk_seek_tag`); `rerun-recent` 2 (`⌃P` `⏎` after one palette
      run: `walk_rerun_recent`). Start state: graph, cursor on HEAD.
- [x] Click on a branch row seeks, as `⏎` (`click_on_branch_row_seeks`; 11b's mouse path).
- [ ] Perf: `scripts/perf-palette.sh` (release; 50k-ref fixture, built under the scratch dir if none exists) → `docs/perf/m8-palette.txt`: loop time per key p50/p95/max from the G-trace frame trace; `⌃P` → BRANCHES landed p50/p95; key → names answer p50/p95. p95 per key < 16 ms. Summary lines only in context; no fast-tier timing assert.
- [x] Capture `docs/shots/m8-palette-find-120x36.txt`: query `rebase`, groups ACTIONS, BRANCHES, COMMANDS RUN RECENTLY (§1.6); and `docs/shots/m8-palette-empty-120x36.txt`: empty box after one run (recent group, ACTIONS, placeholder). **Colour unverified.**

## Touches (each checked in the report)
`update/palette.rs` (`PaletteState`, `run`, `rows`, `edit`, `view`), `AppState` (new `palette_recent`), update events
(`Product::PaletteBranches`, `Product::PaletteNames` routes beside `SwitchList`), `jobs` runner + new
`jobs/palette.rs`, `graft-ui/src/palette.rs` (`RowKind`, row drawing, placeholder; 11b's `palette_row_at` must count
the new rows), the `C` picker (`switch_list` job and `filter_switch_rows` unchanged — grep; ordering test only reads
it), empty state (`listing` unchanged — no find groups there; grep), 11's captures (empty box now draws the
placeholder → re-run and re-shoot), `seek_to` callers (map, picker: unchanged), keymap (no key), journal/undo (none: a
seek is not an operation), the note's fence (counts read, not edited), MAP.md graft + graft-ui.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~65k tokens.

## NOTES (builder, 2026-10-07)
**Compacted once mid-ticket.** Nothing remains from before the compaction except the perf box below; every other box was
built and its test run after it.

**Scout:** used `.scratch/m8-polish-and-ship/scouts/12-find-anything.md`; written before 11/11b/09e–09g/10f–10j merged,
so each line it cites was re-read in current `update/palette.rs`, `update/map.rs`, `graft-ui/src/palette.rs`. No new
scout: the files were read directly.

**Red first, per box.** Unit/edge tests (`update/tests/keymap_edge/palette_find.rs`, `graft-ui/src/palette/tests.rs`)
were seen red as compile failures (the APIs did not exist) before the code. `match_names` tests
(`jobs/palette/tests.rs`) were seen red against a stub returning `NameHits::default()`. The pty walks
(`tests/it/palette_find.rs`) were written after the code (behaviour already green underneath); their first failures
were test faults (below), not app faults. 11's tests in `keymap_edge/palette.rs` went red on this ticket's extra find
commands and job ids; fixed by comparing with the find commands removed (`without_finds`) and against the same state
with the palette closed (`under_the_palette`), not by loosening what the row runs.

**Perf box — open.** `scripts/perf-palette.sh` (+ `scripts/perf-palette-pty.py`) runs and writes
`docs/perf/m8-palette.txt`. 40 000 branches + 10 000 tags on 3 commits, release, pty 120x36, on battery:
`loop_us p50=36249 p95=60739` → **`check loop_p95_under_16ms FAIL`**; `update_us p95=5368` (ok); `⌃P 4` → BRANCHES
drawn p50 2083 ms / p95 2414 ms; key → names drawn p50 52 ms / p95 2416 ms (the first key of a query waits on the
branch read). Cause of the loop FAIL, measured: `draw_us` is 33 ms p50 on the **same fixture with no palette** (held
`j`, `scroll-lag-pty.py`, 18 frames: draw p50 33383 / p95 64777 µs) — every frame draws ~16 000 ref badges on one row.
Pre-existing graph draw cost on a degenerate fixture, not the palette's (its `update_us` is 5 ms p95). Not fixed: out
of scope (graph badge drawing). Second fact, not judged: `switch_refs` on 50k refs takes ~2 s, so BRANCHES reads
`reading…` for ~2 s on such a repo; the `C` picker shares the read. Needs a ruling: a realistic 50k-ref fixture
(refs spread over many commits) or a badge cap in the graph.

**Deviations from the Signatures section (each a smaller shape, same behaviour):**
- `NameHits { branches, tags }` are `Option<Ranked<RefHit>>`: `None` = that source was not in the request (still
  `reading…`/`searching…`), so "answered empty" and "not asked" are told apart (`nothing_matches_waits_for_every_source`).
- `FindGroup` has `Branches`, `Tags` only: `Files`/`Commits` were never constructed (clippy dead code); 12b adds them.
- `jobs::palette` types are `pub` (they ride in `Product`/`Command`, which are `pub`).
- A seek from a non-graph screen sets `detail.focus = Graph` and calls `map::seek_to`, as the picker's `⇥` does; it
  does not close other overlays.
- `enter_on_branch_row_seeks_tip_and_keeps_head` reads HEAD from `state.refs` (a unit test has no repository); the
  git read of HEAD is in `walk_seek_branch` (`git rev-parse HEAD`, `git branch --show-current`).

**Question (rule 7: no line, so a question, not a defect).** Launched by the binary on `FeatureBranches` (HEAD `main`,
newest row `feature/three`), the graph's cursor stayed on the newest row, not HEAD, for 20 s
(`tests/it/palette_find.rs` first run). `places::drive` lands on HEAD only "by an open" (MAP.md). The walks start on
HEAD by switching the fixture to `feature/three` first (comment in `repo()`). Is "cursor on the newest row at launch"
intended?

**Test faults found and fixed in the walks:** the wait for the find row matched the graph's own cursor row through the
box's query line (`▸ … › vqz9`), so `⏎` sometimes landed on `searching…` (which correctly runs nothing; flaky 2/5).
Now matches `│▸` inside the box: 5/5 runs green.

**Captures** (colour unverified, rule 5): `docs/shots/m8-palette-find-120x36.txt`, `docs/shots/m8-palette-empty-120x36.txt`
(`capture_find_and_empty_palette`), and 11's `docs/shots/m8-palette-everything-120x36.txt` re-shot (the empty field now
draws the placeholder). The placeholder starts one cell after the field cursor: `›  type to find…` (two spaces).
