# 18 — rulings on the scout's findings; 18 splits into 18a–18d (planner spec-only pass, 2026-10-08)

Status: draft. **Supersedes `18-first-run-clone-and-first-launch.md`** — its Decisions, Scope and Acceptance are
carried into 18a–18d below, amended where these rulings say. Read with: `scouts/18-first-run.md`, 18,
design note `m8-polish-and-ship.md` J11/J15 and "Where he lands" clone rows, DESIGN §1.15, keymap §2.13.

## Rulings

**R1 — graft-ops' remote runner takes what it reads, not a `Context`.** Scout Q1: `apply::remote::run(ctx: &Context, …)`
(`remote.rs:76`) reads only `ctx.remote` (:82) and `ctx.stop` (:121, :136). Narrow it to a crate-private
`RemoteRun<'a> { hooks: Option<&'a RemoteHooks>, stop: &'a dyn Stop }`; `Context::remote_run()` builds one for every
existing caller. Clone builds one directly — no fake repo, no fake journal. Rejected: a `Context` with `Option` repo
and journal (every write path would have to handle "no repo"); a second copy of the runner in `clone` (two places
that spawn askpass, two progress parsers — they drift). Reversible: crate-private, one file.

**R2 — `StopFlag` does not exist; the stop is `&dyn graft_git::run::Stop`** (`run.rs:112`). Hooks are
`Option<&RemoteHooks>`, as `remote_invocation` already takes (`remote.rs:57-62`): `None` = headless, git asks nothing.

**R3 — Clone returns what happened to the folder; Graft never deletes it.** Plan `git clone --progress -- <url> <into>`
through `GitInvocation::in_folder(<into's parent>)` (`--`: a URL starting `-` is never an option). Refused before
anything runs, the stop included: `BadUrl` (no name from the URL, or it starts with `-`), `TargetNotEmpty`. After a
failure or stop, git's own cleanup removes a folder it created (SIGTERM path; graft-git stops with SIGTERM first).
If the folder is still there and git made it (or a pre-existing empty one now has files), the error says
`left: Some(path)` and the words end `· <path> was left behind`. No `remove_dir_all` in Graft: deleting a folder on
a guess is the one irreversible thing here (rule 2: if the cancel test finds git leaves it, **stop and report**, do
not add deletion). Askpass close failure after a good clone is `Cloned.askpass`, never swallowed (rule 3).

**R4 — The empty state hosts a clone on a job pool of its own, started on the first `⏎`.** Scout Q2: `outside_loop`
(`places.rs:296-363`) has no pool and no `AppState`. Options: (a) a bare thread + new wake plumbing; (b) the
existing `JobRunner`, started lazily by the outside loop; (c) open a repo-less `AppState`. **(b)**: `JobRunner::submit`
needs no repo, and it already owns cancel, `Waiting`/`ChannelAskPass` (`jobs/askpass.rs:126`) and
`answer_credential` — (a) would rebuild all of it, (c) drags every repo-shaped field into a screen with no repo. The
hooks builder at `jobs/operate.rs:181-202` is extracted into one function operate and clone both call. The clone job
is **stoppable**: `Ctrl+C` quits (pool shutdown cancels it, git cleans up — keymap §2.2: box only over a
`Stoppable::No` write); `Ran` carries the pool's `Shutdown` when a pool was started, `Ran::without_jobs` otherwise.
**Check first** (scout could not confirm): the outside loop's `ChannelWakes` sender accepts job `Wake`s as
`run_repo`'s does — read and quote the lines in NOTES before writing; if it needs the sender wired, wire it the way
`run_repo` does (not a design change).

**R5 — The credential prompt's state machine moves off `AppState` into a host-agnostic core.**
`update::credential::{credential_needed, credential_key, credential_paste, close_if_orphaned}` keep their signatures
as thin wrappers over `open_prompt`/`prompt_key`/`prompt_paste`/`close_orphaned`, which take
`&mut Option<CredentialPrompt>` and the running job id. `OutsideState` holds its own `credential` slot. One prompt
logic, two hosts; the M4/M5 credential tests stay green **unedited**. The box is graft-ui's existing
`credential_prompt::CredentialView` drawn over the empty state, bar = the existing `Credential` hint context.

**R6 — Clone landings** (extends the note's two clone rows; all on the prompt, UX §3.1):

| Event | Lands |
|---|---|
| `c` | prompt `clone › ▏`, under it `→ ~/code/…` muted; bar `⏎ clone · esc cancel` |
| typing | `→ ~/code/<name>` follows the URL (`name_from_url`), `→ ~/code/…` while none |
| `⏎`, no name / `-…` | prompt stays, reason `can't name a folder from this URL`; bar `⏎ retry · esc cancel` |
| `⏎`, target non-empty | reason `<name> already exists here`; nothing ran |
| `⏎`, runs | field read-only; `→ ~/code/x · <progress brief>` (`connecting…` before git's first line); bar `esc cancels` |
| git asks | credential box over the empty state (M4's box); `esc` in it stops the clone |
| fails | field editable, URL kept, reason (Problem ink) `clone failed: <words>`; bar `⏎ retry · esc cancel` |
| `esc` while running | stops; reason `clone stopped · nothing kept` (or `· <path> was left behind`); bar as fails |
| `esc`, not running | prompt closes, list as before, nothing written |
| done | `Exit::Open(target)` → graph, cursor on HEAD (12d); recents gain it (`drive` remembers opens) |

`c` is gated like `i` (`!missing`): a folder that does not exist offers neither.

**R7 — First-launch line: counted and shown by `drive` only (tty), drawn where nothing else is.** Headless `run`, the
gallery and every `Harness` never read or write `launches` — no existing test changes. Graph: the line is the status
row's **Ready fallback** (a view-level draw, not a `Status` value), so problem and notice win by construction. Empty
state has no status row (its bar row is where a problem goes): the line draws muted on the **row above the rule**
when that row is free, else not this launch. `done` is written once per process, on the first palette open (`?`,
`⌃P`, `:`) in any place; the line goes the same frame. The `⌃P "tour" to practise` clause is drawn only when the
palette has a tour row (a key the screen does not answer is never offered); G-tour adds it otherwise. Narrow: whole
clauses drop from the right; never a fragment (ADR-0039).

**No ADR.** R1, R4, R5 are crate-private seams, each undone in one ticket. R3's no-delete rule and R7's file format
(`<n>` or `done`) are one-function policies. None is expensive to reverse.

## Split (each one builder context)

| Ticket | Crate set | Build with | Blocked by |
|---|---|---|---|
| 18a `clone-op` | graft-ops | Opus | 08 |
| 18d `first-launch-line` | graft, graft-ui | Sonnet | 11, 11b |
| 18b `clone-job-host` | graft | Opus | 18a, 18d (both edit `places.rs`, `empty_state.rs`) |
| 18c `clone-prompt-and-walk` | graft-ui, graft | Sonnet | 18b |

18a ∥ 18d (disjoint crates). No Fable: nothing here meets all three tests — every seam is reversible and judged by
tests that run git.

## For the user (beyond the approved design)
- First-launch line on the empty state sits above the rule, not on a status row (the screen has none). Capture.
- `esc` mid-clone keeps the prompt and URL, like a failure (design named only failure).
- A folder git leaves behind is reported, never deleted by Graft.
- `c` not offered in a folder that does not exist (same as `i`).
