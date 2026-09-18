# m1-the-graph/54 — Graft offers to build a missing commit-graph file

Status: done
Build with: **Opus** (high) — a repository write through the system git binary, a child process inside a job, an `update` state machine with a failure path that must surface. Approach decided below; not Fable.
Needs: none. Touches `update::Harness` like `dates-on-a-fixed-clock` does (one new field each): sequence after it, or merge the struct edit.
Blocked by: 53, 52, 52b

Why (user, 2026-09-14, ADR-0026 "User decisions on approval" 2): "When a repo has no commit-graph file, Graft offers to build one, as ADR-0008 already promised. The fallback path must not be silent." Pulled forward from M2 (KNOWN_GAPS R2-4).

## Read first
- ADR-0008 (all, short); ADR-0026 "The gate at 1M" (second and third bullets) and "User decisions on approval"
- docs/design/03-graph-layout-algorithm.md §3.2 step 2, "No commit-graph" bullet only (lines ~646–667); docs/design/06-undo-journal-design.md §6.1 "Append-only log" bullet (the `--git-path` rule, lines ~1228–1235) — this ticket deviates from it, see Decisions
- KNOWN_GAPS.md row "Commit-graph offer plumbing unspecified (R2-4)"
- PROMPT.md §3 "Ease-of-use rules → Destructive actions" (the confirmation shows the exact git command: we borrow that rule, this is not destructive); §6.3 "Security" (run nothing from the repo except through git)
- `crates/graft-git/MAP.md` (`repo`: `RepoHandle::common_dir`, `version`: how `system_git` runs the binary), `crates/graft/MAP.md` (`update`: `Command`, `Task`, `Product`, `Status`, `Harness`; `repository`: `facts`; `jobs`), `crates/graft-ui/MAP.md` (`graph_screen`: `GraphView::problem`, `notice`, `search`, footer order)
- Scouts for source: how `Product::Refs` carries an error across the job channel; how `Status::Copied` clears on the next key; how the footer stacks `problem`/`search`/`notice`/hints.

## Decisions (planner-fable)
1. **The offer is a footer line with two keys, never automatic.** `B` builds, `X` declines for this repository (remembered). No third key: the line stays for the run until one is pressed. Alternative the user may prefer: `Esc` hides it for this run — not chosen because `Esc` already has four ordered meanings on the graph screen (peek, jump-withdraw, focus, walk-cancel) and a fifth would be a surprise.
2. **The build is git's own `git commit-graph write --reachable`**, the system binary, as ADR-0008 says. gix has no writer; PROMPT §6.3 makes git the only thing run. It runs as a job; the UI never waits. **It is not cancelled and not killed**: git writes through a `.lock` and renames, so a finished write is atomic and an interrupted one is git's to clean up; `Child::kill` is SIGKILL, which would leave a stale `.lock` behind and break the next write. If the user quits mid-write, the child finishes on its own (the runner abandons the worker after `SHUTDOWN_GRACE`) — say so in the module doc.
3. **"Declined, remembered" lives in the common dir**, `<common_dir>/graft/commit-graph-offer`, not `--git-path` (per-worktree). The commit-graph is a property of the shared object store; one decline covers every worktree. This is the one deviation from docs/design/06-undo-journal-design.md §6.1's rule; record it in the ADR-0008 amendment.
4. **Outside the undo journal, by name.** A commit-graph is a cache, not history; there is nothing to undo. The ADR-0008 amendment says so, so M3's journal design does not rediscover it. No `ops` crate exists yet; the runner lives in `graft-git` (ARCHITECTURE.md: `git` holds "the typed git command runner").
5. **Only offered when it can be built:** the repository facts say `Absent { declined: false }` and the system git check is `Ok`. With no usable git the line reads why and offers nothing — the path is still not silent.
6. **The running session does not switch to the fast path** after a write; the next open does (DESIGN.md's wording). A KNOWN_GAPS row names it, expiry M2 (`Ctrl+R` refresh reopens the walk).

## Scope
In:
- `graft-git::commit_graph` (new module): `status`, `write`, `offer_state`, `remember_declined` (below). `write` spawns `git --git-dir <common_dir> commit-graph write --reachable --no-progress`, stdin null, stdout/stderr captured, waits; non-zero exit → `Exited { status, stderr_first_line }`; then `status` for the count. `status` reads through gix (`Repository::commit_graph()`); a corrupt file is `Err`, never `Absent`.
- `graft::repository::RepositoryFacts.commit_graph: CommitGraphFact` filled by `facts` (status + offer_state). `Unreadable(msg)` when either read fails: shown as `problem`, no offer.
- `graft::update`: `AppState.commit_graph: CommitGraphOffer`; `Harness.offer_commit_graph: bool` (default `true`; the gallery sets `false` — its fixtures are `borrow`ed read-only); keys `B`/`X` while `Offered`; `Command::WriteCommitGraph` / `Command::DeclineCommitGraph`; `Task`/`Product` variants; `Failed` puts one sanitised line in `problem`; `Written` clears on the next key like `Status::Copied`.
- `graft::event_loop`: the two commands run as jobs (`jobs::JobRunner::submit`), `SharedRepo::handle` for the `RepoHandle`.
- `graft-ui::graph_screen::GraphView.commit_graph: Option<CommitGraphLine>` drawn in the footer directly above the hints (below `notice`), texts per width breakpoint (UI below).
- ADR-0008: append `## Landed (m1-the-graph)` — five lines: system git, job, not killed, common-dir decline file, outside the journal. KNOWN_GAPS: close R2-4 with the decision; add the "running session keeps its walk" row; retitle the no-cg pack-page residual's expiry from "M2 (the offer)" to "the offer is built; the residual is what remains when the user declines".
- MAP.md for `graft-git`, `graft`, `graft-ui`; ARCHITECTURE.md if the module table changes (`codebase-design`).

Out: cancelling or killing the write; switching the live walk to the graph path after the write; `gc.writeCommitGraph` config; a status-screen (F2) entry; the 1M end-to-end (memory-curve-acceptance does that).

## Signatures
```rust
// graft-git::commit_graph
pub enum CommitGraphStatus { Present { commits: u32 }, Absent }
pub fn status(repo: &RepoHandle) -> Result<CommitGraphStatus, CommitGraphError>;
pub struct Written { pub commits: u32, pub elapsed: Duration }
pub fn write(repo: &RepoHandle) -> Result<Written, CommitGraphError>;          // blocking; a job's work; never kills the child
pub enum CommitGraphError {
    GitNotFound(std::io::Error),
    Exited { status: i32, stderr_first_line: String },                         // git's own words, first line, raw (the app sanitises)
    Signalled,
    Io(std::io::Error),
    Graph(String),                                                             // gix could not read the file after the write, or on `status`
}
pub enum Offer { Undecided, Declined }
pub fn offer_state(repo: &RepoHandle) -> Result<Offer, std::io::Error>;         // <common_dir>/graft/commit-graph-offer; absent = Undecided
pub fn remember_declined(repo: &RepoHandle) -> Result<(), std::io::Error>;      // writes "declined\n", creating graft/

// graft::repository
pub struct RepositoryFacts { /* existing */ pub commit_graph: CommitGraphFact }
pub enum CommitGraphFact { Present { commits: u32 }, Absent { declined: bool }, Unreadable(String) }

// graft::update
pub enum CommitGraphOffer { NotNeeded, Offered, Writing(JobId), Written { commits: u32 }, Declining(JobId), Declined, Failed, Suppressed }
pub struct AppState { /* existing */ pub commit_graph: CommitGraphOffer }
pub struct Harness { /* existing */ pub offer_commit_graph: bool }           // Default: true
// Command::WriteCommitGraph, Command::DeclineCommitGraph
// Task::WriteCommitGraph, Task::DeclineCommitGraph
// Product::CommitGraph(Result<Written, String>), Product::CommitGraphDeclined(Result<(), String>)   // String = Display of the error

// graft-ui::graph_screen
pub struct GraphView { /* existing */ pub commit_graph: Option<CommitGraphLine> }
pub enum CommitGraphLine { Offered, NoGitToBuild, Writing, Written { commits: u32 } }
```
Callers: `facts` (status, offer_state); the loop's job closures (`write`, `remember_declined`); `view::graph_view` (maps `CommitGraphOffer` → `CommitGraphLine`); `update` on `B`/`X` and on the two `Product`s.

## New UI: the commit-graph line
One footer row, text-info colour, key letters in the hint accent, while `commit_graph` is `Some`. Footer order top → bottom: `problem` (replaces hints) · `search` · `notice` · **this line** · hints. It costs one graph row while shown; a `problem` never hides it except `Failed`, where the line is gone and `problem` carries the failure.

| State | ≥ 120 cols | 100–119 | < 100 |
|---|---|---|---|
| Offered | ` no commit-graph file: history sorts on every open · B build one (git commit-graph write --reachable) · X never for this repo` | ` no commit-graph file · B build one (git commit-graph write --reachable) · X never here` | ` no commit-graph file · B build one · X never here` |
| NoGitToBuild | ` no commit-graph file, and no git ≥ 2.38 on PATH to build one` | same | ` no commit-graph file · no git to build one` |
| Writing | ` building the commit-graph… (git commit-graph write --reachable)` | same | ` building the commit-graph…` |
| Written | ` commit-graph written: 1,000,000 commits · the next open sorts at once` | same | ` commit-graph written · next open sorts at once` |

A test holds every text under its breakpoint. `B`/`X` do nothing in any state but `Offered`. `Failed`: `problem` = `commit-graph write failed (exit 128): <stderr first line, sanitised>`; no retry this run; the next open offers again (nothing recorded). Decline failure: `problem` = `could not remember the choice: <io error>`, state back to `Offered`. Empty state: `None` — no line, no row taken (repos with a graph: the common case).

## Acceptance
- [x] `update` tests: facts `Absent{declined:false}` + git `Ok` → `Offered`; `B` → `Writing` + `Command::WriteCommitGraph`; `Product::CommitGraph(Ok)` → `Written{commits}`; next key → `NotNeeded`; `Product::CommitGraph(Err)` → `Failed`, `problem` holds the exit code and stderr line; `X` → `Declining` + command; `Ok` → `Declined`, line gone; `Err` → `Offered` + `problem`. `Present` → `NotNeeded`; `Absent{declined:true}` → `NotNeeded`; git not ok → `NoGitToBuild` line; `Harness.offer_commit_graph = false` → `Suppressed`, no line. `B`/`X` ignored in every other state.
- [x] Fast-tier integration, real git: `copy_into_temp(Shape::Linear)` → `status` = `Absent` → `write` → `Present{commits: 200}` and `git commit-graph verify` exits 0 in that repo. `offer_state` round-trip; through a linked worktree (`git worktree add`) the same file is seen. Unwritable `objects/info` (chmod; skip as root) → `write` = `Exited{..}` with a non-empty stderr line, no panic.
- [x] Every string from git or the file system passes the sanitiser before drawing.
- [x] No `let _ =`, no ignored `Result` on any path (the decline write included).
- [x] pty capture on a temp copy of `feature-branches`, 80×24 and 120×40: the offer frame, then `B`, then the written frame → `docs/shots/<ticket-number>-commit-graph-offer-{80,120}.txt`. Contents not printed; the user judges by eye.
- [x] ADR-0008 amended; KNOWN_GAPS rows as scoped; MAP.md × 3.
- [x] Report says: cause vs symptom n/a (new feature); what was observed in the pty run (the file appeared, `verify` passed) and what was not (a 1M write — memory-curve-acceptance).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Seam: (a) `graft-git::commit_graph` + facts + tests; (b) `update`/view/loop + capture. Stop at the seam with a handoff note rather than compacting.

## NOTES (builder, Opus)
- **Session compacted once** mid-ticket (after the code and tests, before clippy splits and the pty run). Nothing remains.
- Deviations from the spec, for review:
  - `CommitGraphOffer::Writing(Option<JobId>)` / `Declining(Option<JobId>)`, not `JobId`: the key press leaves `Offered` before `Started` delivers the id. `None` → id filled on `Started`.
  - `status` uses gix `commit_graph_if_enabled()` (not `commit_graph()`), so `Absent` matches what the walk sees (`core.commitGraph` respected).
  - Wide `Offered` wording ends "never here", not "never for this repo": the spec text was 125 cols, over the 120 breakpoint.
- Clippy line/complexity limits met by splitting `update` → `update` + `answer` (+ `commit_graph_event`, `took_fact`), `core_event`'s job-ended guard → `side_job_ended`, B/X handled in `key_event` before `graph_key`, jobs dispatch arms → `JobRunner::write_commit_graph` / `decline_commit_graph`.
- pty (release, temp copies of `feature-branches`, 80×24 and 120×40): offer line shown, `B` → written line, commit-graph file appeared, `git commit-graph verify` exit 0. Captures: `docs/shots/54-commit-graph-offer-{80,120}.txt` — user judges by eye. Script: `.scratch/m1-the-graph/pty-commit-graph-offer.py`.
- Not observed: a 1M-commit write (time, memory) — left to memory-curve-acceptance. Decline (`X`) and failure paths verified by `update` and integration tests only, not in a pty.
