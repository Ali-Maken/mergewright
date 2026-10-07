# G-palette 2a — `⌃P` finds branches and tags; RECENT; the find job

Status: draft (re-spec 2026-10-07, spec-only pass; replaces the 2b spec — see "Why re-specced")
Build with: **Opus** — design-heavy: a per-keystroke job contract (cancel, stale drop, re-filter), cursor anchoring;
not Fable (correctness is judged from tests: stale ids, cancel commands, HEAD unchanged).
Blocked by: 11b (same `update/palette.rs`, `graft-ui/src/palette.rs`, palette captures).
Needs: 11's `PaletteState`, `PaletteTarget`, `Group`, `rows`/`listing`, `run`; `graft_core::matcher::fuzzy_score`;
`update::map::seek_to` (`pub(super)`, update/map.rs:202); `refs::RefBadges::{branches, remotes, tags}`
(refs/mod.rs:87, :107, :123); `AppState::refs: Option<Arc<RefBadges>>`.

## Why re-specced (scout `.scratch/m8-polish-and-ship/scouts/12-find-anything.md`, checked by the planner)
- `PaletteTarget` has `Note` (update/palette.rs:59) and derives `Copy` (:52); the old spec dropped one, broke the other.
  Now: **`Note` kept, `Copy` kept** — no `PathBuf` in the target (files are an index, 12b).
- `/` is a job walking every ref (search.rs:142 `scan`, `Walk::all_refs` :153); history holds a window only. There is
  no in-memory commit list to filter per key. Commits move to 12b, on a job.
- No tracked-paths read exists (diff/read.rs:276 is `--others`). Files move to 12b.
- `⇥` seek handles a tip not laid out (`seek_to`: `History::seek` + `Command::ShowRows`, map.rs:208). Reused as is.

## Read first
- `docs/design/01-view-mockups.md` §1.6 (lines 257–277); `docs/design/02-keymap.md` §2.12 (lines 528–542)
- `docs/design-notes/m8-polish-and-ship.md`: Journeys (J6 row and "Where he lands", lines 84–143), New UI "Palette
  sources" (line 352)
- `crates/graft/MAP.md`: `update::palette` (691), `update::map` `seek_to` (687), `refs` (104), `jobs` (1013, the
  `switch_list` and `map` jobs as the pattern), `trace` (126); `crates/graft-ui/MAP.md`: `palette`;
  `crates/graft-core/MAP.md`: `fuzzy_score` (81)
- The scout file above. Scout again (≤1 page) only if 11b changed `palette.rs` past the line numbers cited here.

## Decisions (planner)
**Cost per keystroke** (PROMPT §6: input to frame < 16 ms; must hold at 50k+ refs). The loop never scans refs, paths or
history. Per key it does: actions + RECENT filter (≤ ~200 rows, as today), re-filter of the last answer's shown rows
(≤ 50 per group), one `CancelJob` + one `Command::PaletteNames`. Matching 50k ref names runs on a worker. Every shown
row always matches the current query (re-filter), so `⏎` never runs a row the query excludes. A group whose answer
is for an older query shows its re-filtered rows plus a muted `searching…` row in place of `+N more` (H1, UX §2).

| Source | Where it comes from | Rows | `⏎` |
|---|---|---|---|
| Actions | `listing` (11), unchanged | as 11 | as 11 |
| BRANCHES | `RefBadges::branches` then `remotes` (resident `Arc`), matched by the names job | `main`, `origin/feat` | `Seek(tip)` via `seek_to`; never switches (`C` does) |
| TAGS | `RefBadges::tags`, same job | `v2.1.0` | `Seek(tip)` |
| RECENT | `AppState::palette_recent`, in memory, this session | the action row as 11 draws it | same `Action`/`App` target, its key's path |

- **Group order** (§2.12): `YOUR CONFIG` (11) · empty query: `RECENT`, `ACTIONS` — no ref groups. Typed: `ACTIONS`,
  `BRANCHES`, `TAGS`, (12b: `FILES`, `COMMITS`), `RECENT`. Fixed; §1.6's "then recency" applies inside RECENT only —
  ref dates are not resident. `?` mode: no find groups, no RECENT (§2.12).
- **Inside a group**: `fuzzy_score` (lower first); branches: local before remote, then name; tags: name.
- **RECENT**: last 8 distinct `Action`/`App` targets run from either palette mode, newest first; a rerun moves to the
  front. Typed query: filtered like actions, may repeat an ACTIONS row (the §1.6 drawing does). Header `RECENT`
  (the §1.6 drawing says `COMMANDS RUN RECENTLY`; conductor: doc follow-up).
- **Caps**: 8 rows per find group; `+N more` (`More(group)`) widens that group to 50, cursor on the first new row; past
  50 the last row is `+N more · type more to narrow` (`Note`). Query change resets the widening.
- **Cursor**: anchored by `(group header, target)`; re-found when an answer lands, else index clamped.
- **refs not read yet** (`AppState::refs` `None`): BRANCHES shows one muted `reading…` row (`Note`).
- **Lands** (UX §3.1): seek row `⏎` → palette closed, graph focused, cursor on the commit, detail follows; not laid out
  → existing jump (`jumping`, `Esc` withdraws); walk ends without it → `seek_to`'s existing fallback, unchanged.

## Scope
In: target arms, names job (refs only; 12b adds paths to the same request), stale drop, cancel on edit and close,
re-filter, `+N more`, cursor anchor, RECENT, `RowKind::Branch/Tag/Recent/More` drawing, perf script, capture.
Out: files, commits (12b); switching from the palette; persistent RECENT; cross-group ranking.

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
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct Find {
    pub names: Asked<Arc<NameHits>>,
    pub widened: Option<FindGroup>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Asked<T> { pub pending: Option<(JobId, String)>, pub answer: Option<(String, T)> } // query each is for
// PaletteState gains `pub find: Find` (Default for `?`).
pub(crate) struct RecentRuns(VecDeque<PaletteTarget>);           // in AppState as `palette_recent`
impl RecentRuns { pub(crate) fn ran(&mut self, t: PaletteTarget); pub(crate) fn iter(&self) -> impl Iterator<Item = PaletteTarget> + '_; }
pub const RECENT_CAP: usize = 8; pub const GROUP_ROWS: usize = 8; pub const WIDE_ROWS: usize = 50;

// graft — jobs::palette (new private mod), dispatched by JobRunner like `switch_list`
pub(crate) struct NamesRequest { pub query: Arc<str>, pub refs: Option<Arc<RefBadges>> } // 12b adds `files`
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct NameHits { pub branches: Ranked<RefHit>, pub tags: Ranked<RefHit> }      // 12b adds `files`
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct Ranked<T> { pub top: Vec<T>, pub total: usize }                          // top ≤ WIDE_ROWS, best first
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RefHit { pub name: Box<[u8]>, pub tip: Oid, pub remote: bool }
pub(crate) fn match_names(req: &NamesRequest, stop: &dyn Fn() -> bool) -> Option<NameHits>; // None = stopped; checks stop every 4096
// update: Command::PaletteNames(JobId, NamesRequest) → AppEvent::Produced(id, Product::PaletteNames(Arc<NameHits>))

// graft-ui — palette
pub enum RowKind { Action, App, Branch, Tag, File, Commit, Recent, More } // + More: muted (`+N more`, `reading…`, `searching…`)
```
`run` (palette.rs:479): `Seek(oid)` → close, `super::map::seek_to`; `More(g)` → widen; `Action`/`App` → `palette_recent.ran`
then today's path.

## Acceptance (each box names its test at tick time)
- [ ] `⌃P` `feat` `⏎` on a branch row → cursor on its tip, HEAD unchanged (`enter_on_branch_row_seeks_tip_and_keeps_head`, reads HEAD with git).
- [ ] Remote branch and tag rows seek their commit (`enter_on_remote_and_tag_rows_seek`).
- [ ] Tip not laid out → `Command::ShowRows`, cursor lands when rows arrive (`branch_tip_not_laid_out_seeks_through_the_walk`).
- [ ] Empty query → `RECENT` (if any) then `ACTIONS`, no ref rows (`empty_query_lists_recent_then_actions_only`).
- [ ] `?` has neither find groups nor RECENT (`help_palette_has_no_find_groups`).
- [ ] RECENT: run stash from the palette, reopen → `RECENT` first, `⏎` opens the stash prompt (`recent_row_runs_its_key_path`); cap 8, newest first, rerun moves to front (`recent_runs_cap_and_order`).
- [ ] Each query edit cancels the previous names job and asks one new one; close cancels it (`query_edit_cancels_the_previous_find_job`, `closing_the_palette_cancels_find_jobs`).
- [ ] Answer for an older id dropped (`stale_names_answer_is_dropped`); every shown row matches the current query while an answer is pending (`shown_rows_always_match_the_current_query`).
- [ ] `+N more` widens to 50, cursor on first new row; query edit resets (`more_row_widens_then_resets`).
- [ ] Cursor stays on its row when an answer lands above it (`cursor_stays_on_its_row_when_an_answer_lands`).
- [ ] `match_names`: segment start first, local before remote, totals right, stops when asked (`names_rank_and_total`, `names_stop_when_cancelled`).
- [ ] Journey: seek a branch = 3 keys (`⌃P` text `⏎`; VS Code 3, `docs/research/checkout-cherry-pick.md:20`) — walk asserts ≤ 3 (`palette_seek_branch_is_three_keys`).
- [ ] Click on a branch row seeks, as `⏎` (`click_on_branch_row_seeks`; 11b's mouse path).
- [ ] Perf: `scripts/perf-palette.sh` (release; 50k-ref fixture, built under the scratch dir if none exists) → `docs/perf/m8-palette.txt`: loop time per key p50/p95/max from the G-trace frame trace, and key → names answer p50/p95. p95 per key < 16 ms. Summary lines only in context; no fast-tier timing assert.
- [ ] Capture `docs/shots/m8-palette-find-120x36.txt`: query `rebase`, groups ACTIONS, BRANCHES, RECENT (§1.6). **Colour unverified.**

## Touches (each checked in the report)
`update/palette.rs` (`PaletteState`, `run`, `rows`, `edit`, `view`), `AppState` (new `palette_recent`), update events
(`Product::PaletteNames` route beside `SwitchList`), `jobs` runner + new `jobs/palette.rs`, `graft-ui/src/palette.rs`
(`RowKind`, row drawing; 11b's `palette_row_at` must count the new rows), empty state (`listing` unchanged — no find
groups there; grep), 11's captures (empty RECENT → unchanged; re-run), `seek_to` callers (map, picker: unchanged),
keymap (no key), journal/undo (none: a seek is not an operation), MAP.md graft + graft-ui.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.
