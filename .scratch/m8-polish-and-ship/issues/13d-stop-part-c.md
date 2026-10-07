# Stop leaves it paused, part C: rulings on 13c's questions, then the app side

Status: done
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
- [x] Q2: `it::stop_hooks::a_cherry_pick_stopped_in_its_post_commit_hook_has_landed`,
      `…a_revert_stopped_in_its_post_commit_hook_has_landed`, `…a_continue_stopped_in_its_post_commit_hook_has_landed`.
      Each checks `Committed` and `stopped_at: Some(Landed)`.
- [x] Q3: `it::ops_execution_hard::a_plan_stopped_between_invocations_is_aborted_recoverable_and_narrow` asserts
      `git was stopped` and `Stopped { changed: true }`. Undo restores the tree and his file byte-equal (:1051, :1062,
      :1078).
- [x] Q4: `it::stop_hooks::a_finish_merge_stopped_in_its_hook_leaves_the_merge_paused`. Checks `MERGE_HEAD` = source
      tip, HEAD unchanged, `stopped_at() = Paused(Merge)`, and the entry shape as NOTES record it.
- [x] Q4b: `apply::error::tests::a_hook_rejection_with_nothing_unmerged_says_a_hook_stopped_it` (exact words). Merge
      only: `apply::error::tests::nothing_unmerged_outside_a_merge_does_not_blame_a_hook` (NOTES).
- [x] Q1: `it::quit_waits::stuck::stop_now_mid_rebase_with_edits_set_aside` also asserts: no `*.lock` under `.git`,
      the rebase state live, entry `Pending`, and the exit line exactly `Paused(Rebase)`.
- [x] Every remaining box of 13c's Acceptance from "Exit line" down (exit line seven cases; walk; relaunch; real
      binary; forced wait; hang-up observation; capture `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt`, colour
      **unverified**; fence; MAP rows), plus `jobs::tests::answer_paused_lands_as_conflicted` (item 12). Each ticked in
      13c with its test. Item 12's test is `jobs::operate::tests::answer_paused_lands_as_conflicted` (where `answer` lives).
- [x] An unforced cancel still yields `JobCancelled`: `jobs::tests::an_unforced_cancel_is_still_job_cancelled`.

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

## NOTES (builder, 2026-10-07)
**Context compacted twice.** Nothing remains; all boxes ticked with tests. Scout reused (`scouts/13c-app.md`).

Measured (`stuck` walks, eprintln, debug build): `y` to the pool down 53.7 ms (merge), 13.2 ms (rebase); bound
`FORCED_WAIT` = `SHUTDOWN_GRACE` + 3 s (the 3 s stays the planner's allowance, not measured as a bound).

**How the answer travels (signature freedom the ticket gave).**
- graft-ops (seam d9efe3e0): Q3 rides `ApplyError::Git { paused: Option<InProgress> }`; `stopped_at()` gives
  `Paused(k)` if set, else `BeforePause`/`Stopped` by `opens_in_progress`. Q4's read is `apply/stopped.rs`
  `where_stopped`, for the stop path only; `opens_in_progress` unchanged.
- graft: a forced write's `StoppedAt` rides the `Cancel` token (`left_at`, set in `jobs::operate::in_context`), so
  `run_job`'s event mapping is untouched: a cancelled job is still `JobCancelled` (pool.rs `finish`). `Live` keeps
  `forced` + `started`; a forced write that never started is answered at once with `None`.
- `exit_line` is `pub` and takes the undo key spelled from the keymap (`app.undo`, startup.rs `UNDO`), not a literal `u`
  (10b's ratchet).

**Q4.** Observed first: `crates/graft-ops/tests/it/finish_merge.rs:503` `a_hook_refusal_leaves_the_merge_in_progress`:
a `FinishMerge` whose commit fails is `ApplyError::Git`, entry `Aborted { <hook stderr> }`, not in force. The stopped
one takes that shape with reason `git was stopped` (stop_hooks.rs:655-661). Siblings (`opens_in_progress` `None`, run only
while a kind is live): `FinishMerge`, `AbortMerge`. Not `ResolveWholeFile`/`RestartConflict`/`Commit`: a stash clash
leaves unmerged paths with no kind live.

**Question for the conductor (not done):** `stopped_on_conflict` (apply.rs:761) is **not** switched to the new read.
`live_entry` there reads `opens_in_progress`, and Q4 forbids holding a `Pending` entry for a merge Graft never began.
The ruling said "fixed by the same read"; I found it cannot be without breaking that rule. Decide.

**Q4b narrowed to `Merge`.** Empty `paths` is not only a hook: a rewrite's `edit` row (apply.rs:418 `edit_stop`) and
an empty pick also stop with nothing unmerged. They keep today's words (test above).

**Deviations from the ticket's words.**
- The relaunch banner reads `c finish  a abort`, not `c continue · a abort` (graft-ui, existing). The test quotes the
  real banner. The design note `:140` edit says `c continue · a abort` verbatim as ruled: the doc and the banner
  disagree. Conductor's call.
- Design note `:140`: replaced only text after `next launch:`, as ruled. The stale prefix before it
  (`<bin>: stopped <op> half-way after <n> s · <bin> here shows where it stopped`) is still there.
- The capture at 80x24 shows the Detail tab: banner `MERGING … c finish  a abort`, with no room for "from a previous
  run". Colour **unverified**; looks-right is the user's.

**Known limits.**
- A forced undo jump (`update/undo.rs:508`, `jobs::undo`) does not go through `in_context`: its exit line is always
  the `None` words.
- A third signal does not cut the forced wait (bounded, `FORCED_WAIT`).

**Hang-up observation (13b item 10 re-run, scratch script, no test).** Real binary as a pty session leader, merge
with a hung `pre-merge-commit`, master closed: graft exited **101** after 0.1 s (13b's dead-stderr exit, out of scope).
git (own pgid since 13c, graft-git run.rs:478 `process_group(0)`), the hook and its sleep all **stayed alive**;
the hook's HUP/TERM/INT traps fired none. No `MERGE_HEAD` yet, no locks. Leftovers killed by pid. So on hang-up git
is orphaned and keeps running: SIGHUP no longer reaches it, and the 101 skips the stop. Needs the conductor.

**Touches.**
- `StoppedAt` variant: every match by compiler; `explain` by `apply::error::tests`.
- `0 unmerged` quotes: `grep -rn "0 unmerged" crates docs/shots` finds only
  `nothing_unmerged_outside_a_merge_does_not_blame_a_hook` (error/tests.rs:154); no capture quotes it.
- pool.rs `JobCancelled`: `an_unforced_cancel_is_still_job_cancelled`.
- 13b exit-line and signal tests: rewritten to the new lines (`quit_waits::signal`); fence `quit-stuck-write 4` walked.
- Foreground-group grep: editor/mergetool/`--continue` handovers (`event_loop/outside.rs:178,215,237,454,461,489,504`)
  use plain `Command`, stay in graft's group. `git var GIT_EDITOR` (graft-git editor.rs:45) runs without a tty read.
  Plan steps set `GIT_EDITOR=true` (graft-ops apply/run.rs:191); rewrite helpers are scripts (engine.rs:145-146).
  `GIT_ASKPASS`/`SSH_ASKPASS` → graft's helper, `SSH_ASKPASS_REQUIRE=force` (graft-git run.rs:341-353);
  `GIT_TERMINAL_PROMPT=0` (run.rs:10); `--no-pager` (run.rs:458). No source opens `/dev/tty`. None needs the
  foreground group; only a user's hook that opens `/dev/tty` would (13c decision 2).
- **Outside the crate set:** graft-ui test data only: ratchet rows in 3715a1fb, plus one prose row in
  `crates/graft-ui/tests/no_literal_keys/prose.rs` (the `Landed` line's `· a git hook`). No graft-ui source changed.
- Examples (`gallery`, `m3_walk`, `m4_walk`, `m5_walk`) match the new `Exit::StoppedHalfWay`.

Gates (workspace): fmt ok; clippy clean; nextest 5660/5660 (10 skipped); doctests ok.
