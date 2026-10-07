# Scout 12b: FILES and COMMITS in ⌃P

Paths are relative to `crates/`.

## 1. File history: opening it and how Esc lands
- `pub fn open_history(state: &mut AppState, from: HistoryFrom) -> Vec<Command>`: `graft/src/update/place/history/open.rs:9`. It pushes `Place::History` onto `state.trail.places` and sets `trail.shown = true` (:46-50).
- `HistoryFrom::File { at: Rev, path: PathBuf, line: Option<u32>, side: Side }`: `update/place/history.rs:59-69`.
- `f_key` (open.rs:140-177) does this in order: `begin_trail(next, commands, &cursor)` → `open_history` → `settle_trail(next, base)` → `commands.extend`.
- `begin_trail` (`update/place.rs:265`) takes a **`&DiffCursor`**. It cancels the old trail and builds `Origin { spec, path, at, section, focus, base, spot: Spot::of(next) }` from that cursor.
- `settle_trail` is at place.rs:287.
- Esc path: `history_keys.rs:229 escape` → `trail.rs:14 trail_back` → `leave` (:34). If `Spot::of(state) == origin.spot` it returns (:51). Otherwise it calls `show_line(origin.spec, &origin.path, …)`.
- `Spot` covers the graph cursor, `detail.shown`, spec, file cursor and focus (place.rs:186-196).
- **Contradiction:** to land the same way from the palette (graph origin), the caller needs an `Origin`, and `begin_trail` only builds one from a `DiffCursor`. With the graph focused there may be no diff cursor at all. `Origin`'s `spot` field is private (place.rs:172). With `origin: None`, `leave` returns early (trail.rs:42-44), so nothing is restored. The graph cursor would only stay where it was if nothing moved it.
- Where the words "isn't committed yet · no history" come from: `not_committed_words`, open.rs:128-133. It is used at :149.

## 2. update/palette.rs
- Opening:
  - `open` :312. Under `Opens::Everything` it sets `find.branches = ReadOnce::Reading(id)` and pushes `Command::ReadPaletteBranches(id)` (:315-319).
  - `open_key` :299. `next_id` :323.
- State:
  - `Asked<T>{pending: Option<(JobId,String)>, answer: Option<(String,T)>}` :123.
  - `Find{branches, names, widened}` :140. The comment at :70 already says "12b adds `Files` and `Commits`".
  - `FindGroup::header` :81. `ReadOnce` :89.
- Keys:
  - `palette_key` :809. Close → `cancel_finds` (:827).
  - `Run(More)` → `widen` (:832, fn :907).
  - `Run(other)` → `cancel_finds` + `run` (:836-838).
- Query edits:
  - `query_changed` :847. It resets `widened` and asks only if the trimmed query differs from the pending query, or from the answered one when nothing is pending.
  - `ask_names` :867. It cancels the pending job, returns on an empty query or when neither branches nor `state.refs` are read, then pushes `PaletteNames(id, NamesRequest{query, branches, tags})` and sets `pending`.
  - `paste` :1052 also calls `query_changed`.
- `cancel_finds` :896 cancels a `Reading` branches job and a pending names job.
- Answers come in through `update/mod.rs:291-295`:
  - `Product::PaletteBranches` → `branches_read` (:921). Stale ids are dropped. On success it calls `ask_names` again.
  - `Product::PaletteNames` → `names_answered` (:947). It moves `pending` into `answer`.
  - `refs_landed` :965 and `landed` :993.
- The same two jobs are also listed at `mod.rs:618-619`, `mod.rs:1028-1029` and `place/write.rs:64-65`.
- Rows:
  - `rows` :355 → `listing` :363.
  - `find_groups(find, query, tags_read)` :389 loops over a hardcoded `[Branches, Tags]` (:391-394). It shows `reading…` at :416 and a failure row at :399-408.
  - `group_rows` :427, with stale rows + `searching…` at :455.
  - `current_rows` :463 adds `+N more` (:475-481), or `· type more to narrow` once widened.
  - `hit_row` :487 (→ `Seek(hit.tip)`), `placeholder` :500 (`RowKind::More`/`Note`), `is_placeholder` :513, `first_runnable` :740.
  - No "nothing matches" string in palette.rs (grepped). *Could not confirm* where it is produced.
- `run` :1021:
  - `Seek(tip)` focuses the graph, then `map::seek_to` (:1032-1035).
  - `More | Note` do nothing (:1038).
  - `recent.ran` skips Seek (:159).

## 3. Jobs
- `jobs/runner.rs:1392-1393` routes to `read_palette_branches` :1208 and `palette_names` :1224.
- Both use `queue_as(id, Some(Task::…))` with a `Cancel` token. Names runs `match_names(&request, &|| cancel.is_cancelled())`, and `None` gives `Outcome::Cancelled`.
- Cancel: `Command::CancelJob` (:1403) → `cancel` :454, which runs `live.cancel(id)` and drops the row-request sender.
- `jobs/palette.rs`: `read_branches` :29, `match_names` :86, `tier` :150 (= `fuzzy_score/1024`), `ranked` :155 (top `TOP`, then total). `fuzzy_score` is in `graft-core/src/matcher.rs:19`.

## 4. Search
- `graft/src/search.rs`:
  - `pub struct Query` :20, `parse` :32, `matches(id, subject, author, body)` :56.
  - `scan(store, query, chunk, stop: &dyn Fn()->bool, found: &mut dyn FnMut(Arc<[Oid]>)->bool) -> Result<Scanned,String>` :142.
  - The loop at :155-184 walks `Walk::all_refs` in topological order. It checks `stop` before each chunk and hands the hit oids to `found`.
  - It returns `Scanned::{Finished, Stopped}` (:130). It yields oids only, not commits.
- `update/search.rs:88 fn start_search(next, commands, query: Query)` is private. It needs `next.history` (for the prior cursor) and `next.search`, and pushes `Command::Search(query)`.

## 5. graft-git
- `diff/read.rs:276 append_untracked` uses `GitInvocation::new(repo).args([ls-files,-z,--others,--exclude-standard]).output_cap(OUTPUT_CAP).run(stop)`. If `truncated` it returns `DiffError::TooLarge`.
- Runner, in `run.rs`:
  - `GitInvocation` :201, `new` :277, `output_cap` :401.
  - `run(&self, stop:&dyn Stop)->Result<GitOutput,RunError>` :412, `run_raw` :436.
  - `pub trait Stop: Sync` :112.
- `status.rs`:
  - `StatusError` :61 has variants Start, Begin, Item, Run(RunError), Malformed(String) and `TooLarge{bytes}` (:85).
  - `OUTPUT_CAP` is 64 MiB (:94). `read` :326.

## 6. graft-ui/src/palette.rs
- `RowKind` :56 already has `File` and `Commit` (:65-68), plus Action, App, Branch, Tag, Recent and More.
- `draw_row` :351. The label style (:386-390) is muted only for `More`. Bold text when chosen, plain text otherwise. Branch, Tag, File and Commit are not styled differently from each other.
- `draw_list_line` :315 draws muted headers.

## 7. Tests
- Unit tests in `update/tests/keymap_edge/palette_find.rs`. Helpers:
  - `start` :38, `branches` :64, `press` :76, `ctrl_p` :80, `typed` :89.
  - `branch_read` :100, `names_asked` :107, `cancelled` :117, `produced` :127, `read_lands` :132, `answer` :141, `found` :148.
  - `shown` :158, `headers` :176, `under_cursor` :184, `cursor_on` :194, `selected` :203, `head` :211.
- The capture test is `capture_find_and_empty_palette` (~:762), a unit test that writes `docs/shots/m8-palette-find-120x36.txt`.
- Pty walks in `tests/it/palette_find.rs`:
  - `NOTE` const :17. `noted_keys` :23 uses `m5_walk::parse_counts` (`tests/it/m5_walk/journeys.rs:51`).
  - `assert_within_the_note(id, keys)` :37, `repo()` :56 (FeatureBranches fixture), `spawn` :79, `seeked_to` :96, `quit` :105.
  - Walks: `walk_seek_branch` :114, `walk_seek_tag` :146, `walk_rerun_recent` :174.
  - There is no capture in that pty file.
