# G-palette 2b — `⌃P` finds commits and files

Status: draft (spec-only pass 2026-10-07; split from 12; re-aligned the same day to `docs/research/palette-find.md`:
step counts from the note's fence, `ReadOnce` shared with 12)
Build with: **Opus** — git read + two jobs on 12's contract; not Fable (judged from git-truth tests and the stale/cancel
tests 12 established).
Blocked by: 12 (same palette files and job). Also after 09e if 09e is not merged: this edits `update/search.rs`
(one visibility change).
Needs: 12's `PaletteTarget::{Seek, More}`, `FindGroup`, `Find`, `Asked<T>`, `ReadOnce<T>`, `NamesRequest`, `NameHits`,
`Ranked`, `match_names`, `Command::PaletteNames`, the placeholder/`nothing matches` rules; `search::Query`
(search.rs:20, `matches` :56), `search::scan`'s walk (:142); `update::search::start_search` (update/search.rs:74,
private today); `place::history::open_history(state, HistoryFrom)` (update/place/history/open.rs:9),
`HistoryFrom::File { at, path, line, side }` (:101), `Rev::WorkingTree`.

## Read first
- This spec; 12's spec (`.scratch/m8-polish-and-ship/issues/12-palette-find-anything.md`) "Decisions" and "Signatures"
- `docs/design/01-view-mockups.md` §1.6 (257–277); `docs/design/02-keymap.md` §2.12 (528–542)
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows "Find: commits, how far", "Find: commit fields",
  "Find: files"; Journeys rows 28–30, "Find rows judged", "Where he lands" rows "Palette …" and "`more · /` …", the
  `journey-counts` fence; Where I'd differ P-find-1; New UI "Palette sources"
- `docs/research/palette-find.md` "Journeys" (lines 47–53) and "Big repos" (55–57)
- `crates/graft/MAP.md`: `search` (800), `update::place` (721) and the `history/open.rs` layout line (1143), `jobs`
  (1013); `crates/graft-git/MAP.md`: `status` (324), `diff` (124)
- Scout (≤1 page) on 12's merged state: how `f_key` (open.rs:140) makes `Esc` land back (`begin_trail`/`settle_trail`)
  and what a graph-origin open needs for the same landing.

## Decisions (planner)
**Cost per keystroke** (PROMPT §6, 16 ms): unchanged from 12 — no path list or history is scanned on the loop.

| Source | Where it comes from | Cost | Rows | `⏎` |
|---|---|---|---|---|
| FILES | `graft_git::status::tracked_paths` (`ls-files -z`, new; nearest today is `--others`, diff/read.rs:276) — read **once per `⌃P` open**, a job beside 12's branches read; held as `Find::files: ReadOnce<TrackedPaths>` (one byte buffer + `u32` ends, not `Vec<PathBuf>`) | one spawn per open; matching rides 12's names job (`NamesRequest::files`) | path, directory muted, base name normal | open the file's history place (`F`) at `HEAD` |
| COMMITS | `search::first_hits` — `/`'s walk and `Query` (same match: subject, author, body, sha prefix), **stops at 9 hits** | one job per query edit, cancelled by the next; a no-hit query walks to the end off the loop | `a1b2c3d  subject` (sha muted) | `Seek(id)` (12) |

- Full history, not loaded rows (note Prior art "Find: commits, how far": lazygit #3279, GitUp). No `~patch` / diff
  search (rejected there).
- Commits ranking: walk order (topological from every ref, newest first) — the same order `/`'s `n` steps.
- 9th hit found → last COMMITS row `more · / searches all of history` (`More(Commits)`): closes the palette, starts `/`
  with this query (`start_search`); `n`/`N` step. Lands as `/`'s `⏎` does.
- Files ranking: `fuzzy_score`, then shorter path, then index order. `+N more` as 12.
- `File(ix)`: index into the open's `TrackedPaths` — stable, the list is not re-read while the palette is up. `Copy` kept.
- Paths: `ls-files` lists an unmerged path once per stage — adjacent duplicates dropped. Bare repo: empty, no spawn.
  Over `TRACKED_CAP` → `StatusError::TooLarge`; the FILES row reads `files: too many to list` (muted).
- Not in `HEAD` (staged new file) → palette closes, status `<path> isn't committed yet · no history` (open.rs:128, existing).
- Rows while waiting (query non-empty): FILES `reading…` until the list lands (then 12's names job is re-asked);
  COMMITS `searching…` (re-filtered old rows by sha + subject kept above it). Failures: `files: <reason>`,
  `commits: search failed: <reason>` — muted, in the group; other groups unaffected (H9). The cursor and
  `nothing matches` rules are 12's (placeholder → first runnable row; `nothing matches` after every source answers).
- **Empty query**: no FILES, no COMMITS (P-find-1). If P-find-1 is declined: FILES lists its first rows (index
  order) and COMMITS the first 9 of the walk for the empty query, both through the same jobs.
- **Lands**: file row → history place on that path; `Esc` → graph, cursor where it was before `⌃P`. Commit row → as
  12's seek.
- **Journeys** (note rows 28–30, fence): `seek-commit-sha` 3 (`⌃P` sha `⏎`; Sublime Merge 4, GitLens 4);
  `seek-commit-words` 3 (`⌃P` words `⏎`; forgit 3, JetBrains 4 — fixture: the words match one commit and no action,
  branch, tag or path; with a group above matching it is +1 `↓`, not walked); `file-history` 3 (`⌃P` path `⏎`;
  lazygit 3).

## Scope
In: `tracked_paths` read, read job (cancel on close), FILES in the names job, commits job, `File`/`More(Commits)` arms,
`RowKind::File/Commit` drawing, walks for the three fence ids, perf for paths, capture. Out: commit dates/authors in
rows; persistent file cache; working-tree-only (untracked) files; diff-content search.

## Signatures
```rust
// graft-git — status (beside `submodules_not_set_up`)
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TrackedPaths { bytes: Vec<u8>, ends: Vec<u32> }
impl TrackedPaths {
    pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool;
    pub fn get(&self, ix: usize) -> Option<&[u8]>; pub fn iter(&self) -> impl Iterator<Item = &[u8]> + '_;
}
pub const TRACKED_CAP: usize = 64 << 20;
pub fn tracked_paths(repo: &RepoHandle, stop: &dyn Stop) -> Result<TrackedPaths, StatusError>; // `ls-files -z`

// graft — search (scan's signature unchanged; both share one private walk loop)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitHit { pub id: Oid, pub subject: Box<[u8]> }
pub fn first_hits(store: &RepoStore, query: &Query, limit: usize, stop: &dyn Fn() -> bool)
    -> Result<Vec<CommitHit>, String>;

// graft — update::palette (extends 12)
PaletteTarget::File(u32)                            // new arm; Copy kept
// Find gains: `pub files: ReadOnce<TrackedPaths>` (12's enum), `pub commits: Asked<Arc<[CommitHit]>>`
// NamesRequest gains `pub files: Option<Arc<TrackedPaths>>`; NameHits gains `pub files: Ranked<u32>`
pub const COMMIT_HITS: usize = 9;

// graft — update::search: `start_search` becomes pub(in crate::update); body unchanged

// Commands / products
Command::ReadTrackedPaths(JobId)        → Product::TrackedPaths(Result<Arc<TrackedPaths>, String>)
Command::PaletteCommits(JobId, Query)   → Product::PaletteCommits(Result<Arc<[CommitHit]>, String>)
```
File `⏎`: `open_history(state, HistoryFrom::File { at: Rev::WorkingTree, path: PathBuf::from(OsStr::from_bytes(p)), line: None, side: Side::New })`
plus whatever the scout finds `Esc` needs to land on the graph.

## Acceptance (each box names its test at tick time)
- [ ] `tracked_paths`: NUL split, non-UTF-8 path kept, unmerged path once, bare repo empty, over cap `TooLarge` (`tracked_paths_*`, graft-git, git fixtures).
- [ ] `⌃P` asks one `ReadTrackedPaths`; `?` asks none; close cancels it (`palette_open_reads_tracked_paths_once`).
- [ ] FILES `reading…` until it lands, then file rows (`files_group_reads_then_lists`); read failure in the group (`tracked_read_failure_reads_in_files_group`).
- [ ] File row → history place on that path (`enter_on_file_row_opens_its_history_place`); `Esc` → graph, cursor unchanged (`esc_from_palette_file_history_lands_on_graph`); staged-new file → `isn't committed yet` (`file_not_in_head_says_not_committed`).
- [ ] Commit row by sha prefix and by subject word → cursor on it (`commit_row_by_sha_prefix_seeks_it`, `commit_row_by_subject_word_seeks_it`).
- [ ] `first_hits` = the first N of `scan`'s hits, same order (`palette_commit_hits_match_slash_search`); stops at 9 (`commits_job_stops_after_nine_hits`).
- [ ] `more · / searches all of history` → palette closed, `/` search running for the query (`commits_more_row_starts_slash_search`).
- [ ] Each query edit cancels the previous commits job; stale answer dropped (`stale_commits_answer_is_dropped`).
- [ ] Empty query has no FILES or COMMITS rows (P-find-1) (`empty_query_has_no_files_or_commits`); declined → `empty_query_lists_first_files_and_commits` instead.
- [ ] **Journeys, from the fence** (same mechanics as 12: pty test, fence id as a string literal, count read with
      `m5_walk::parse_counts`, fails if the key list is longer; start: graph, cursor on HEAD): `seek-commit-sha` 3
      (`walk_seek_commit_sha`); `seek-commit-words` 3, on the stated fixture (`walk_seek_commit_words`);
      `file-history` 3, ends on the history place for that path (`walk_file_history`).
- [ ] Perf: `scripts/perf-palette.sh` gains a 100k-path index fixture (`update-index --index-info` against one blob; no worktree files) → `docs/perf/m8-palette.txt`: per-key p95 < 16 ms; key → names answer p50/p95 with files; `⌃P` → FILES landed p50/p95. Summary only.
- [ ] Capture `docs/shots/m8-palette-find-120x36.txt` re-taken: query `rebase` over a fixture with a matching branch, file and commit — ACTIONS, BRANCHES, FILES, COMMITS, COMMANDS RUN RECENTLY. **Colour unverified.**

## Touches (each checked in the report)
graft-git `status` (new fn; MAP.md), `search.rs` (shared walk loop — `/`'s tests must stay green unchanged),
`update/search.rs` (visibility only; 09e's file), `update/palette.rs`, update events routing, jobs runner +
`jobs/palette.rs`, `update/place/history/open.rs` callers (`f_key` unchanged), `graft-ui/src/palette.rs` (File/Commit
rows), 12's capture (re-taken), keymap (no key), journal/undo (none), the note's fence (counts read, not edited),
MAP.md graft, graft-git, graft-ui.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.
