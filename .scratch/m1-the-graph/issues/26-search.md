# m1-the-graph/26 — Search over history

Status: done
Build with: **Opus** (high) — concurrency: a cancellable search job over history that may still be streaming
Blocked by: 09, 16
Leaves runnable: yes — `/` in the graph screen finds and jumps to commits.

Why: PROMPT.md §8 lists search in M1, and on a big repo it is the only way to reach a commit that is not on screen. It is the first feature that has to admit "I have not read all the history yet" without lying. (Retired ticket 15, text in 35bec3e; dropped by the re-scope.)

## Read first
- PROMPT.md §8, the M1 row only; §6.3 "Security" (two bullets)
- DESIGN.md §2.2 (the `/` row only), §3.6 (the checkpoint and deep-jump paragraphs only)
- CONTEXT.md — "job"
- ARCHITECTURE.md "The layout and the walk: `graph::CommitSource`"
- Code: `MAP.md` of `graft-graph`, `graft-git`, `graft`; then 16's selection API

## Scope
In:
- `/` opens a query; search runs as a job on `JobRunner`, incremental: hits appear as found, the first hit is jumped to as soon as it exists.
- Fields: subject, author name/email, sha prefix, full message body. Case-insensitive literal substring. No regex, no query language.
- `n` / `N` walk hits forward/back with wrap. A far hit is reached through `StreamingLayout`'s checkpoints, never a re-walk from HEAD.
- Honest counts: while the walk runs the count reads partial (`12 matches so far`); it settles visibly when done.
- Zero matches, cancelled and failed are three distinct visible states.
- Match against raw repo text; everything drawn goes through `SanitizedText`.
- `Esc` cancels the job and restores the prior selection.

Out:
- Searching diffs or file contents. Filtering the graph to hits (highlight and jump only).
- `/` in other panes.

## Signatures
Expose (pure matcher; place per ARCHITECTURE.md, update MAP.md):
- `pub struct Query` + `pub fn Query::parse(raw: &str) -> Option<Query>` — `None` for an empty query.
- `pub fn Query::matches(&self, id: Oid, subject: &[u8], author: &[u8], body: &[u8]) -> bool`
- `pub enum SearchState { Running { hits: usize }, Done { hits: usize }, Cancelled { hits: usize }, Failed(String) }` — rendered by state, never an error-looking label when nothing is wrong.

Call (existing):
- `JobRunner::submit(&self, impl FnOnce(&Cancel) -> Outcome + Send) -> JobId`, `JobRunner::cancel(JobId)`, `Cancel::is_cancelled()`
- `graft_git::history::Walk::all_refs(&RepoHandle, Order)`, `read_commit(&RepoHandle, Oid) -> Result<Commit, WalkError>`
- `StreamingLayout::row_of(Oid) -> Option<usize>`, `advance(budget, stop)`, `reanchor(&Anchor, height)`, `set_viewport(Range<usize>)`

## Acceptance
- [x] Fixture tests: a hit in each field, no hit, a hit beyond the first chunk, `n`/`N` wrap, cancel mid-search.
- [x] A search started while history streams shows a partial count that later settles — asserted.
- [x] A result from a superseded job id is discarded — asserted.
- [x] A subject containing an escape sequence matches and renders inert.
- [x] Heavy tier: time from `Enter` to a far hit on screen on the 100k fixture, measured and reported with the machine.
- [x] Input stays responsive during a search on the 100k fixture — say how you verified it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only; the heavy measurement runs once, alone)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Result (builder, Opus)
- Matcher and scan: `crates/graft/src/search.rs`, a job on `JobRunner` (`jobs/mod.rs`) that hands over each chunk's hits.
- Far hit: `Wanted::Find` lays the history walk out to the commit and selects it; `History::seek`/`restore`.
- Tests:
  - scan on fixtures: each field incl. body and sha prefix, no hit, the last commit at chunk 2, cancel mid-scan
  - update flow: partial count settles, superseded job dropped, n/N wrap, Esc restores
  - Find round trip in history/tests.rs
  - view: ESC in the query is inert
- Heavy, run once, alone, on an Apple M5 with 24 GB and the release build under a pty:
  - Enter to the oldest of 100k commits' row plus "(1/1)" on screen: **641 ms** (646 ms on the first run)
  - Responsiveness: F2 status toggles sent back to back after Enter on a no-match 100k search, 401 toggles over ~1.5 s, each timed from key to changed screen: median 3.5 ms, max 9.2 ms, none missed
  - Script: `.scratch/m1-the-graph/pty-search-check.py`
- Captures (text, pyte): `docs/shots/26-search-far.txt` (far hit, partial, cancelled, settled, n, N wrap, zero) and `docs/shots/26-search-typing.txt`. The by-eye look is the user's to judge, not ticked.
- Not a unit test: "a subject with an escape sequence matches" is asserted on the matcher (`search.rs`) and the inert rendering on the view; there is no fixture commit with a raw ESC subject.
- The builder compacted once mid-ticket.
- Gaps: KNOWN_GAPS.md, m1-the-graph/26.
