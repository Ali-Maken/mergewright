# Stop leaves it paused, part C: rulings on 13c's questions, then the app side

Status: draft
Build with: **Opus, xhigh**. Same reasons as 13c: it decides what the journal records for a stopped write and what
the shell says about it. A wrong outcome loses no bytes but tells him the wrong thing, and that doesn't show in a
diff. Not Fable: the shape is decided below.
Needs: 13c Parts A and B (seam commits on `worktree-agent-a6f85af5902d58b0c`). Continue on that branch.
Crate set: **graft-ops, then graft**. Commit at the graft-ops seam.
Scout: reuse `.scratch/m8-polish-and-ship/scouts/13c-app.md`. No graft-ui edits: the relaunch banner already
exists, so the capture only walks it.

## Rulings after partial (planner, 2026-10-07)
These are measured against the user's ruling (field notes :91: a stop leaves the operation paused, offers continue
or abort, and loses nothing) and against M3 ("you cannot lose work"). None of them needs the user.

**Q1: rebase isn't stopped hook-first. Keep what was built.** A rebase keeps its state on disk at every step. The
group stop already leaves it paused, and continue/abort are offered, which is his ruling exactly. Ending the hook
first makes git **finish** the rebase (Step 0, exit 0) after he asked it to stop, which goes against his ruling.
Item 7 is amended to say this. There's one risk: SIGKILL after `TERM_GRACE` can strand a `*.lock`. git cleans up its
own lockfiles on SIGTERM, but not on SIGKILL. The rebase walk now asserts that no lock is left (box below). If that
fails, stop and report. Don't work around it.

**Q2: Landed covers every kind that makes one commit.** A cherry-pick or revert whose `post-commit` hook was ended has
landed, and the shell must say so. New rule (it replaces 13c item 8's Landed):
- **Landed** = the operation's in-progress kind is `Merge`, `CherryPick` or `Revert`, that kind is **not** live,
  and HEAD ≠ `head_before`. The same applies to `resume`/`--continue` of those kinds.
- A rebase stays out of it: `Paused`, or else the residual. When the state is gone after a stop, a rebase can't be
  told apart from one that git ended half-way through. The residual's `u` loses nothing.

**Q3: the residual's words follow whether the operation can be paused at all.**
- An operation that opens an in-progress state (`opens_in_progress` is `Some`) keeps `stopped before git could pause
  it` and `BeforePause`.
- One that can't be paused (discard and the rest, where it's `None`) goes back to the pre-13c reason `git was
  stopped`. It gets a new `StoppedAt::Stopped { changed }` and its own exit line.
- Revert the `ops_execution_hard` assertion to `git was stopped`.
- D5 (set-aside closes keep `git was stopped`) stands.

**Q4: `FinishMerge` and its siblings read the merge they finish.**
- A stop that leaves that kind live with HEAD unchanged gives `stopped_at() = Paused(k)`. This is what is on disk:
  the merge was paused before `c` and still is.
- The journal takes the shape that an existing `FinishMerge` with a failed commit takes today. Observe it first (one
  test) and record the line in NOTES. Don't hold a `Pending` entry for a merge that Graft never began.
- Siblings means every `Operation` whose `opens_in_progress` is `None` but which runs only while a kind is live. Grep
  `operation.rs` and list them in NOTES.
- `stopped_on_conflict`'s blind spot (`apply.rs:761`) is fixed by the same read. That's the cause, not a second patch.

**Q4b: a `pre-merge-commit` hook that rejects (no stop).**
- `Conflicted { paths: [] }`, `Pending` and the banner are right: git paused the merge, and `c`/`a` are git's own offer.
- The words `0 unmerged · resolve, then c continues, a aborts` are wrong, because there is nothing to resolve. When
  `paths` is empty, `explain` reads `<op>: a git hook stopped it · c continues, a aborts`.
- Note in a comment: `c` runs `git commit`, which runs `pre-commit`, not `pre-merge-commit`. This is git's design.

## Scope
**In:** the graft-ops amendments above, then 13c Part C items 10–12 with the exit lines below. Also 13c's Remaining
list 1–5 (`13c` NOTES, "Remaining — Part C").
**Out:**
- The 13b exit 101 on a dead stderr (conductor).
- Linux `/proc` listing beyond the host test.
- Any graft-ui change.

## Exit lines (replaces 13c item 10; exact words)
- `Paused(k)`: `<bin>: stopped <op> after <n> s · the <k> is paused · <bin> here offers continue or abort`
- `Landed`: `<bin>: <op> finished · a git hook after it was stopped after <n> s`
- `BeforePause { changed: true }`: `<bin>: stopped <op> after <n> s, before git could pause it · u in <bin> puts your files back`
- `Stopped { changed: true }`: `<bin>: stopped <op> after <n> s · u in <bin> puts your files back`
- `BeforePause { changed: false }` and `Stopped { changed: false }`: `<bin>: stopped <op> after <n> s · nothing changed`
- `None`: `<bin>: stopped <op> after <n> s · <bin> here shows where git left it`

## Signatures (changes to 13c's; the rest stand)
```rust
// graft-ops, apply
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoppedAt { Paused(InProgress), Landed, BeforePause { changed: bool }, Stopped { changed: bool } }
impl ApplyError { pub fn stopped_at(&self) -> Option<StoppedAt>; } // unchanged; how a variant carries Q3/Q4's
                                                                   // answer is the builder's, named in NOTES
pub struct Applied { /* … */ pub stopped_at: Option<StoppedAt> }  // Some(Landed) only
// graft: as 13c (exit_line, Forced, FORCED_WAIT = SHUTDOWN_GRACE + 3 s)
```

## Read first
- 13c on the branch: "Decisions", "What is on disk", Part C, all of NOTES
- `.scratch/m8-polish-and-ship/scouts/13c-app.md`
- `crates/graft-ops/MAP.md` `## apply` (as amended in 13c), `## in_progress`; `crates/graft/MAP.md` `jobs`, `Quit`,
  `update::quit`, `event_loop` `exit_wait`
- `docs/design/02-keymap.md` §2.2 "A stuck write"; design note `:140`

## Touches (check each; say how)
- `StoppedAt` gains a variant: every match on it (compiler), and `explain` for every stopped case.
- `where_stopped` (`apply/stopped.rs:74`) and `stopped_on_conflict` (`apply.rs:761`), plus every caller of
  `opens_in_progress`. The new read is for the stop and conflict paths only. Don't change `opens_in_progress` itself,
  because recovery and the banner read it.
- `error.rs:104` words: grep the tests and captures that quote `0 unmerged`.
- Pool `pool.rs:326` (`JobCancelled` replaces a forced job's answer). Unforced cancels must keep today's behaviour,
  and a test must show it.
- The 13b exit-line tests and the signal tests (renamed). Journey fence `quit-stuck-write 4`.
- The grep 13c left undone: `GIT_EDITOR`, `core.editor`, `/dev/tty`, `SSH_ASKPASS`, pager. Record that none of them
  needs the terminal's foreground group.

## Acceptance (each tick names its test, inline)
- [ ] Q2: `it::stop_hooks::a_cherry_pick_stopped_in_its_post_commit_hook_has_landed`,
      `…a_revert_stopped_in_its_post_commit_hook_has_landed`, `…a_continue_stopped_in_its_post_commit_hook_has_landed`.
      Each checks `Committed` and `stopped_at: Some(Landed)`.
- [ ] Q3: `it::ops_execution_hard::a_plan_stopped_between_invocations_is_aborted_recoverable_and_narrow` asserts
      `git was stopped` and `Stopped { changed: true }`. Undo restores the tree and his file byte-equal.
- [ ] Q4: `it::stop_hooks::a_finish_merge_stopped_in_its_hook_leaves_the_merge_paused`. Checks `MERGE_HEAD` = source
      tip, HEAD unchanged, `stopped_at() = Paused(Merge)`, and the entry shape as NOTES record it.
- [ ] Q4b: `apply::error::tests::a_hook_rejection_with_nothing_unmerged_says_a_hook_stopped_it` (exact words).
- [ ] Q1: `it::quit_waits::stuck::stop_now_mid_rebase_with_edits_set_aside` also asserts: no `*.lock` under `.git`,
      the rebase state live, entry `Pending`, and the exit line exactly `Paused(Rebase)`.
- [ ] Every remaining box of 13c's Acceptance from "Exit line" down (exit line seven cases; walk; relaunch; real
      binary; forced wait; hang-up observation; capture `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt`, colour
      **unverified**; fence; MAP rows), plus `jobs::tests::answer_paused_lands_as_conflicted` (item 12).
- [ ] An unforced cancel still yields `JobCancelled`: `jobs::tests::an_unforced_cancel_is_still_job_cancelled`.

## Doc edits (verbatim; these replace 13c's)
- `docs/design/02-keymap.md`, "A stuck write": replace
  `` The shell then reads `<bin>: stopped <op> half-way after <n> s · <bin> here shows where it stopped`. `` with
  `` The shell then reads one line saying where git was left: paused (the next launch offers continue or abort), landed, or stopped before git could pause it (`u` puts the files back). Graft ends a merge's, cherry-pick's or revert's stuck hook first, so git pauses it itself; a rebase is stopped where it stands and stays paused at its step (**Proposed**, M8/13c–13d). ``
- `docs/design-notes/m8-polish-and-ship.md:140`: replace everything after the first `next launch:` with
  `` banner `c continue · a abort` (merge, rebase, cherry-pick, revert), set-aside edits offered. Graft ends the stuck hook, not git, so git pauses a merge, cherry-pick or revert itself; a rebase stays paused at its step (13c–13d); git stuck outside a hook: no pause, `u` puts the files back, and the exit line says which. ``

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, about 70k tokens. If it runs long, the conductor splits it at the graft-ops seam.
