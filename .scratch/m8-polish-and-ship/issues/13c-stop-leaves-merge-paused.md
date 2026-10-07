# Stopping a stuck merge leaves it paused: end git's hook, not git

Status: partial — Parts A and B done (seam commits); Part C (graft) not started. Builder compacted.
Build with: **Opus, xhigh**. It changes what the journal records for a stopped write (the undo area) and how every git
child is signalled. A wrong order loses the paused state or strands a lock, and neither shows in a diff. Not Fable:
the shape is decided below.
Needs: none. Continues on the 13b branch (`worktree-agent-a6f85af5902d58b0c`). Supersedes 13b's NOT MET box.
Crate set: **graft-git, graft-ops, graft**, built in that order. Each part ends with a commit at the seam, so the
conductor may split this ticket there.
Brownfield (ADR-0035): **scout first**, on `graft-git` `run/exec.rs` (spawn, stop, reap: which call sends SIGTERM
today) and `graft-ops` `apply.rs:459` `outcome_status` and its callers. The conductor confirms `Build with` after the scout.

## The ruling, and what this ticket makes true
The user, 2026-10-04 (field notes :91): stopping a hung git mid-merge **stops it and leaves the merge paused**, with
continue or abort offered, and **nothing lost**.

Found in 13b (NOTES "Review fix", lines there):
- `ForceStop` SIGTERMs git while its `pre-merge-commit` hook runs. git dies before it writes `MERGE_HEAD`. The
  merge's result is left staged, there is no merge in progress, and the entry is `Aborted` (`graft-ops/src/apply.rs:459`).
- The hook outlives the stop, because only git's pid is signalled (`graft-git/src/run.rs:413`, no process group).

**The cause:** Graft kills git when the thing that is stuck is git's hook. Under its own rules, git **pauses** a merge
whose hook fails. It writes `MERGE_HEAD`, `MERGE_MSG` and `MERGE_MODE`, then exits with "Not committing merge".
**Premise, not verified:** this is read from git's `builtin/merge.c`
(`abort_commit` → `write_merge_state`), from memory, not observed. Step 0 checks it before anything is built.
So the stop ends the hook and its children first, and git pauses the merge itself. Graft writes none of git's
merge files.

## Decisions (planner; hard to reverse)
1. **How a stop gets a paused merge.**
   - (a) End the hook first and let git pause the merge.
   - (b) Run a merge as two steps, `merge --no-commit` then `commit`.
   - (c) Graft writes `MERGE_HEAD`/`MERGE_MSG` after git dies.

   **(a)**. It changes nothing when nothing goes wrong. (b) changes every clean merge's hooks to `pre-commit` and
   `post-commit`, dropping `post-merge`, which teams use to reinstall dependencies. (c) writes bytes git never
   wrote. `merge_state` writes only git's own bytes back (graft-ops MAP).
2. **Every git child gets its own process group** (`CommandExt::process_group(0)`), and every stop signals the group.
   This answers 13b item 10's SIGTTIN worry:
   - git's stdin is null or a pipe, never the tty (graft-git MAP `stdin`). Only a hook that opens `/dev/tty` itself can
     be stopped by SIGTTIN. Today such a hook reads keys out from under Graft's raw-mode input.
   - With the group, that hook waits instead, the 10 s offer appears, and the stop's `SIGCONT` lets it take the SIGTERM.
   - The kernel's hang-up SIGHUP no longer reaches git directly. The 13b signal path (wait ≤ 60 s, then stop)
     governs instead.
3. **A stop's outcome is decided by what git left on disk**, not by how git ended. Three outcomes, below. Decided in
   graft-ops, read with `&()` (never stopped). The stop flag is already set, and the put-back's raw step already reads
   this way.

## What is on disk after a stop (M3: "you cannot lose work")
His uncommitted edits are never on a path the merge writes: git refuses a merge that would overwrite them. Graft's
backup (`worktree_before`, taken before git ran) holds them in every row.

| Where git was | HEAD | merge files | index / tree | journal | he lands (relaunch) |
|---|---|---|---|---|---|
| `pre-merge-commit`, `prepare-commit-msg`, `commit-msg` hook, or gpg signing (git's helper ended, git pauses) | unchanged | `MERGE_HEAD` = source tip, `MERGE_MSG`, `MERGE_MODE` (git's) | merge result staged; his edits as they were | `Pending` | banner, merge in progress, `c continue · a abort`. `a` gives the tree before the merge. |
| `post-merge` hook (merge already committed) | the merge | none | = the merge, plus his edits | `Committed` | graph on the merge; `u` undoes it |
| git itself, with no helper running, or the group could not be listed (**residual**) | unchanged | none | unchanged, or part of the result on clean paths | `Aborted`, in force if anything changed | graph; `u` puts the files back as they were. The exit line says so. |

The residual cannot be paused, because git has not written the merge yet. Pausing it would need (c). It loses
nothing. Expected to be rare: by the time the offer appears at 10 s, git's merge machinery has long finished, and what
is still running is a hook.

## Read first
- This ticket; 13b ticket sections "Review fixes" (items 4–6, 8–10) and "NOTES — Review fix"
- `docs/research/quit-while-git-runs.md` (git itself; the Summary's two caveats)
- `crates/graft-git/MAP.md` `## run` (`run`, `STOP_WITHIN`, `Stop`, `stdin`, `askpass`), `## in_progress`
- `crates/graft-ops/MAP.md` `## apply` (`apply`, `Applied`, `ApplyError` incl. `Conflicted`, `resume`, `live_entry`,
  `recover_interrupted`, `Recovered`, `apply::engine`), `## journal` (`in_force`, `hold_commands`), `## merge_state`
- `crates/graft/MAP.md` `jobs` (`JobRunner::shutdown`, `force_stop`, `Shutdown`, `SHUTDOWN_GRACE`), `Quit`,
  `update::quit`, `event_loop` `exit_wait`
- `docs/design/02-keymap.md` §2.2, the "A stuck write" bullet
- `docs/design-notes/m8-polish-and-ship.md`, "Where he lands", the stuck-write row (`:140`)

## Step 0: a gate. Observe first, then build or stop
In a temp repo on git 2.54, run a merge (`--no-ff`, his unrelated file left dirty) with each hook doing
`echo $$ > pidfile; sleep 600`. Kill **only the hook's pid**. Do this for `pre-merge-commit`, `prepare-commit-msg`,
`commit-msg` and `post-merge`. For each, record in NOTES:
- git's exit code
- whether `MERGE_HEAD`, `MERGE_MSG` and `MERGE_MODE` are there
- the index, HEAD, and his file
- that `git merge --continue` makes the merge commit (`HEAD^2` = source tip), and which hooks it runs (githooks(5):
  `pre-commit`, not `pre-merge-commit`)
- that `git merge --abort` gives the tree back with his file byte-equal

**If `pre-merge-commit` does not leave `MERGE_HEAD`:** stop. Report with the output, and build nothing. The question
for the user is below. Turn the observation into the permanent git-contract test in Part B's first box.

## Part A: graft-git: the group, and the hook-first stop
1. Every child `run` spawns gets its own process group (`process_group(0)`).
2. **Default stop**, for every invocation:
   - `killpg(SIGTERM)` then `killpg(SIGCONT)`.
   - `killpg(SIGKILL)` after `TERM_GRACE`.
   - Group signals go only **before** git is reaped. POSIX keeps the pgid unreused while the group lives.
   - A run that ends **without** a stop signals nothing. A hook's own background job (`npm install &`) is left alone.
3. **Hook-first stop**, for an invocation marked `stop_hooks_first()`:
   - "Helpers" means every group member except git (hooks, their children, filters, gpg).
   - SIGTERM + SIGCONT to every helper. Helpers that appear later get the same, with the list read at most every 250 ms.
   - SIGKILL to any helper still alive `HELPER_TERM_GRACE` (1 s) after its SIGTERM.
   - git itself is not signalled for `HOOKS_GRACE` (2 s). If git has not exited by then, the default stop runs on the
     whole group.
   - A group with no helpers goes straight to the default stop.
4. `members(pgid)` lists the group. On Linux it reads `/proc/*/stat` (field 5). On macOS it runs
   `/bin/ps -A -o pid= -o pgid=`. Any other target, or a failed read, gives `Err`. The stop then falls back to the
   default stop.
   **Safe to ignore, comment it:** the fallback is the stop used before 13c, and graft-ops reads the outcome from
   disk, so the exit line still tells the truth.
5. Signals go through `nix` (add its `signal` feature if absent). No new crate, no `unsafe`. NOTES name what
   `run/exec.rs` used before.
6. `STOP_WITHIN` = `POLL` + `HOOKS_GRACE` + `TERM_GRACE` = 4.05 s, the worst case. A default stop still ends within
   2.05 s.

## Part B: graft-ops: mark the writes; decide the outcome from disk
7. Every `GitInvocation` built from an operation's plan line with `remote: false`, in `apply`, `apply::engine` and
   `resume`, is marked `stop_hooks_first()`. Not marked:
   - remote lines (fetch, push: default group stop)
   - every reader
   - `restore` steps (no hooks; their stop already rolls back)
8. When git's run in `apply`/`resume` returns `Stopped` (after `begin`), read with `&()`, in order:
   - **Paused**: git's state says the operation's `opens_in_progress` kind is live, and HEAD = `head_before`. No
     outcome line. `hold_commands`. Return `ApplyError::Paused { entry, kind, paths }`, where `paths` is what is
     unmerged now (normally none).
   - **Landed** (`Merge` only): no `MERGE_HEAD`, and HEAD ≠ `head_before`. Outcome `Committed` with the after-state.
     `Ok(Applied)` with `stopped_at: Some(Landed)`.
   - **Otherwise** (the residual): outcome `Aborted`, reason exactly `stopped before git could pause it`, after-state
     recorded. Keep today's error variant (NOTES name it). `stopped_at()` maps it to `BeforePause { changed }`, where
     `changed` is `in_force(entry)`.
   - A stop **before** git ran is unchanged.
9. `ApplyError::Paused` closes like `Conflicted`. `resume` finds it through `live_entry`, and `recover_interrupted`
   lists it in `live`.

## Part C: graft: the exit line says where git was left
10. `exit_line` (pure) replaces the single 13b line, exact words:
    - `Paused(k)`: `<bin>: stopped <op> after <n> s · the <k> is paused · <bin> here offers continue or abort`.
      `<k>` is `merge`, `rebase`, `cherry-pick` or `revert`.
    - `Landed`: `<bin>: <op> finished · a git hook after it was stopped after <n> s`
    - `BeforePause { changed: true }`: `<bin>: stopped <op> after <n> s, before git could pause it · u in <bin> puts
      your files back`
    - `BeforePause { changed: false }`: `<bin>: stopped <op> after <n> s · nothing changed`
    - `None` (no answer within the wait): `<bin>: stopped <op> after <n> s · <bin> here shows where git left it`
11. The forced job's answer reaches `wind_down` (`event_loop/exit_wait.rs`). The pool waits for a **forced** job up
    to `FORCED_WAIT` = `SHUTDOWN_GRACE` + 3 s. The 3 s covers the after-state reads.
    - This bound is **not measured**. The walk records the real time in NOTES.
    - Past it: the `None` line, and the job is abandoned. **Why that is safe, comment it:** the entry is `Pending` or
      closed. Recovery at relaunch closes a `Pending` one.
    - The signal path (second signal, or the 60 s bound) uses the same line.
12. The landing for `ApplyError::Paused` is the landing for `Conflicted`. One arm, no new screen.

## Signatures
```rust
// graft-git, run
impl GitInvocation {
    /// On a stop: end git's helpers (hooks, filters, gpg) first, give git HOOKS_GRACE to end the way it
    /// ends when a hook fails, then stop the whole group. Receiver as the other builder methods.
    pub fn stop_hooks_first(self) -> Self;
}
pub const HOOKS_GRACE: Duration = Duration::from_secs(2);
pub(super) const HELPER_TERM_GRACE: Duration = Duration::from_secs(1);
pub const STOP_WITHIN: Duration; // POLL + HOOKS_GRACE + TERM_GRACE
// graft-git, run/group.rs (crate-private)
pub(crate) fn members(pgid: nix::unistd::Pid) -> io::Result<Vec<nix::unistd::Pid>>;

// graft-ops, apply (re-exported)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoppedAt { Paused(InProgress), Landed, BeforePause { changed: bool } }
pub enum ApplyError { /* … */ Paused { entry: Box<JournalEntry>, kind: InProgress, paths: Vec<PathBuf> } }
impl ApplyError { pub fn stopped_at(&self) -> Option<StoppedAt>; }
pub struct Applied { /* … */ pub stopped_at: Option<StoppedAt> } // Some(Landed) only

// graft
pub(crate) fn exit_line(bin: &str, what: &str, after: Duration, at: Option<StoppedAt>) -> String;
pub struct Forced { pub job: JobId, pub what: String, pub at: Option<StoppedAt> } // Shutdown.forced: Vec<Forced>
pub const FORCED_WAIT: Duration; // SHUTDOWN_GRACE + 3 s
```

## Touches (check each; say how in the report)
- Every `run` spawn, through the group. Grep graft-git and graft-ops for `GIT_EDITOR`, `core.editor`, `/dev/tty`,
  `SSH_ASKPASS`, `pager`, and record that none needs the terminal's foreground group. The commit editor handover
  is Graft's own spawn, not `run`: confirm.
- `SHUTDOWN_GRACE` (2.55 → 4.55 s via `STOP_WITHIN`). `the_shutdown_grace_covers_a_git_childs_stop`,
  `no_git_child_outlives_shutdown`.
- `ApplyError`: every exhaustive match (compiler-checked); `explain` for `Paused`. `Applied` literals.
- The rebase walk `stop_now_mid_rebase_with_edits_set_aside` must stay green under the hook-first stop.
- `resume`, `live_entry`, `recover_interrupted` with a `Paused` entry; `in_force` (Pending is not in force).
- Pull: does `opens_in_progress(Pull)` exist? Record it. If not, a pull's merge falls to Landed or the residual, and
  NOTES say so.
- Keymap §2.2, the design note, and MAP rows (doc edits below). The 13b exit-line tests that assert the old words.
- Journal/undo: no new entry kind. `Paused` reuses the conflict-stop shape (ADR on hard choice 2 unchanged).

## Acceptance (each tick names its test, inline)
- [x] Step 0's observation is in NOTES, with all four hooks.
      Git contract: `graft-ops tests/stop_hooks.rs::git_pauses_a_merge_whose_pre_merge_commit_hook_is_killed`
      — ticked by `it::stop_hooks::git_pauses_a_merge_whose_pre_merge_commit_hook_is_killed` (exit 1, MERGE_HEAD = source tip, MERGE_MSG)
      (pure git: hook pid killed → `MERGE_HEAD` = source tip, `MERGE_MSG` present).
- [x] Own group: `run::tests::git_runs_in_its_own_process_group` (child's pgid = its pid ≠ ours).
- [x] A stop ends the hook's children too: `run::tests::a_stop_leaves_no_process_of_the_group` (hook `sleep 600`;
      `kill -0` on the pidfile pid fails; returns within `STOP_WITHIN`).
- [x] Hook first: `run::tests::hooks_first_lets_git_pause_a_merge` (`MERGE_HEAD` present, no `*.lock`, git never
      signalled: its exit code is git's own non-zero, read with `run_raw` or a test hook on the supervise path; say which).
      — which: the supervise path, `exec::stop`'s `StopEnd { status, git_signalled }` (exit 1, `git_signalled == false`).
- [x] `run::tests::hooks_first_kills_a_hook_that_ignores_term` (`trap '' TERM`, merge still paused, within `STOP_WITHIN`).
- [x] `run::tests::a_stopped_hook_is_continued_and_ended` (hook `kill -STOP $$`).
- [x] `run::tests::hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed` (injected lister `Err`).
- [x] `run::group::tests::members_lists_a_child_in_the_group` (runs on the host OS).
- [x] A run that ends unstopped signals nothing: `run::tests::an_unstopped_run_leaves_a_hooks_background_job_alone`.
- [x] Marking: `apply::tests::stop_hooks::local_write_lines_stop_hooks_first_and_remote_lines_do_not`. **Deviation:** a
      rebase's lines (engine, rebase resume) are *not* marked — NOTES Q1; the test asserts that.
- [x] Paused (at `it::stop_hooks::…`, NOTES D1): `apply::tests::stop_hooks::a_merge_stopped_in_its_pre_merge_commit_hook_is_paused`,
      `…_in_its_prepare_commit_msg_hook_is_paused` (entry `Pending`, `Paused { kind: Merge }`, `stopped_at`
      `Paused(Merge)`).
- [x] Landed (`it::stop_hooks::…`): `apply::tests::stop_hooks::a_merge_stopped_in_its_post_merge_hook_has_landed` (`Committed`,
      `stopped_at: Some(Landed)`).
- [x] Residual (`it::stop_hooks::…`): `apply::tests::stop_hooks::a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back`. Driven
      with the hook-first mark off (a `cfg(test)` seam, named in NOTES). **Driven instead** by a hook that leaves
      git's process group (NOTES D2) — no seam. It reproduces 13b's observed state. Checks:
      reason exact, `BeforePause { changed: true }`, then `undo` → HEAD, index and tree equal before, his dirty file
      byte-equal.
- [x] Nothing lost through pause and abort (`it::stop_hooks::…`):
      `apply::tests::stop_hooks::a_paused_merge_aborted_by_resume_gives_his_edits_back` (dirty unrelated file
      byte-equal; entry `Aborted`).
- [x] Continue (`it::stop_hooks::…`): `apply::tests::stop_hooks::a_paused_merge_continues_to_the_merge_git_would_have_made`. Checks: `Committed`,
      `HEAD^2` = source tip, and the message equal to a one-step `git merge` of the same branches in a twin fixture.
- [x] Relaunch: `recover::tests::a_paused_merge_is_live_at_relaunch` — at `it::stop_hooks::a_paused_merge_is_live_at_relaunch` (D1).
- [x] Cherry-pick, same rule (`it::stop_hooks::…`): `apply::tests::stop_hooks::a_cherry_pick_stopped_in_its_hook_is_paused_if_git_pauses_it`.
      If git does not leave `CHERRY_PICK_HEAD`, the test asserts the residual instead, and NOTES say which. — git
      leaves it (`prepare-commit-msg`): the test asserts `Paused { kind: CherryPick }`.
- [ ] Exit line: `update::quit::tests::exit_line_says_where_git_left_it` (five cases, exact words).
- [ ] **The walk (13b's NOT MET box, now met):** `it::quit_waits::stuck::stop_now_mid_merge_leaves_it_paused`.
      Setup: real pool, real repo, `pre-merge-commit` `sleep 600`, his dirty unrelated file, offer injected short.
      Keys: `q y`, wait, `q y`. Then all of:
      - git and the hook's `sleep` are gone (`kill -0`)
      - the app quits within `FORCED_WAIT`, and the time taken is in NOTES
      - `MERGE_HEAD` = source tip, no `*.lock`, entry `Pending`
      - the exit line is exactly the `Paused(Merge)` words
- [ ] Relaunch: `it::quit_waits::stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort`. Checks: the banner
      offers `c continue · a abort` (its words quoted from the existing live-merge banner), `a` gives HEAD and the
      tree as before, and his file is byte-equal.
- [ ] Real binary: `it::quit_waits::signal::stop_now_after_the_real_offer_restores_then_says_where_git_stopped`
      (renamed from 13b's; restore then the `Paused` line on stderr). Signal path:
      `a_signal_restores_then_waits_and_a_second_stops_git` asserts the new line and that the hook is gone.
- [ ] Forced wait: `jobs::tests::a_forced_job_is_waited_for_its_answer_up_to_its_bound`, and past it the `None` line.
- [ ] Hang-up, observation only (13b item 10, re-run): a pty walk, master closed. Record in NOTES whether git or the
      hook gets SIGHUP now, with lines. The 13b exit 101 on a dead stderr is **not** fixed here; it is for the conductor.
- [ ] Capture `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt`: the relaunch landing after a stopped merge, with
      the exit line as its caption. Colour **unverified**. Looks-right is the user's.
      — `it::quit_waits::capture_relaunch_after_a_stopped_merge`.
- [ ] Journey fence `quit-stuck-write 4` still walked: `stuck::stop_now_mid_merge_leaves_it_paused` (`within_the_note`).
- [ ] MAP rows (by hand): graft-git `run` (group, both stops, `STOP_WITHIN`, `stop_hooks_first`, `members`);
      graft-ops `apply` (`StoppedAt`, `Paused`, the stop-outcome rule, `Applied::stopped_at`); graft `exit_line`,
      `Forced`, `FORCED_WAIT`.

## Doc edits (verbatim)
- `docs/design/02-keymap.md`, the "A stuck write" bullet: replace
  `` The shell then reads `<bin>: stopped <op> half-way after <n> s · <bin> here shows where it stopped`. `` with
  `` The shell then reads one line saying where git was left: paused (the next launch offers continue or abort), landed, or stopped before git could pause it (`u` puts the files back). Graft stops git's hook first, so git pauses the operation itself (**Proposed**, M8/13c). ``
- `docs/design-notes/m8-polish-and-ship.md:140`: replace everything after the first `next launch:` with
  `` banner `c continue · a abort` (merge, rebase, cherry-pick, revert), set-aside edits offered. Graft ends the stuck hook, not git, so git pauses the merge itself (13c); git stuck outside a hook: no pause, `u` puts the files back, and the exit line says which. ``

## Question for the user: only if Step 0 fails (in his words; the conductor asks it)
"Stopping a stuck merge can't leave it paused: git only writes the merge down after the hook you're stopping. When you
stop it anyway, should the app **put your files back exactly as they were before the merge** (nothing merged, nothing
lost, merge again later), or **leave the half-done files** for you to undo with `u`?" Recommended: put them back.

## Question (out of scope, rule 7: no line)
A merge whose `pre-merge-commit` hook **rejects** normally, with no stop: git leaves the merge paused. What does Graft
land on, and what does it journal? Observe it, and record it in NOTES with lines. Do not change it. The conductor decides.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context, ~75k tokens, three crates. If it runs long, the conductor splits it at the Part A / Part B seam commit.

## NOTES (builder, 2026-10-07)

**Compacted once**, during Part B. Parts A and B are built and committed at their seams; **Part C (graft) is not
started** — the conductor splits it. Scouts used: `.scratch/m8-polish-and-ship/scouts/13b-review-fix.md` (given), plus
`.scratch/m8-polish-and-ship/scouts/13c-app.md` (app side, for Part C; its findings are listed under Remaining).

### Step 0 (observed, before any build)
Sandbox repos under the session scratchpad, `HOME`/`GIT_CONFIG_GLOBAL` pointed at an empty file, system git (Apple
git, CommandLineTools). `git merge --no-ff feature`; each hook `sleep 600` (shell + child); the **hook's pid only**
sent `SIGTERM`; git waited for:
- `pre-merge-commit`: exit 1, `error: .git/hooks/pre-merge-commit died of signal 15`, `Not committing merge; use
  'git commit' to complete the merge.` `MERGE_HEAD` = source tip, `MERGE_MSG` (`Merge branch 'feature'`),
  `MERGE_MODE` (`no-ff`); `HEAD` unchanged; his dirty `notes.txt` byte-equal. `git merge --continue` → exit 0,
  `HEAD^2` = source tip, message `Merge branch 'feature'`; `--abort` → `HEAD` and tree as before, his file byte-equal.
- `prepare-commit-msg`, `commit-msg`: the same, word for word.
- `post-merge`: exit 0, merge commit made, no `MERGE_*`; `HEAD` moved.
- In every case the hook's own child (`sleep`) **survived in git's group** — what Part A's member re-listing ends.
- Cherry-pick (`git cherry-pick feature`): runs `prepare-commit-msg` and `post-commit` only (`pre-commit`,
  `commit-msg` never ran). `prepare-commit-msg` killed → exit 128, `CHERRY_PICK_HEAD` present, `HEAD` unmoved;
  `post-commit` killed → exit 0, committed.
- **Rebase** (`GIT_SEQUENCE_EDITOR=true git rebase -i main`, `post-checkout` hook killed): exit 0, `Successfully
  rebased and updated refs/heads/feature`, no `rebase-merge/`. A rebase's hook failing does **not** stop it (Q1).

### What `run/exec.rs` used before (item 5)
`kill(1)` via `Command::new("kill")` for `SIGTERM`, then `Child::kill` (`SIGKILL`) — git's pid only, so a hook and
its children outlived the stop. Now `nix` `killpg`/`kill` (`signal` feature added to graft-git's existing `nix`).

### Deviations (for the reviewer)
- **D1 — test paths.** The Paused / Landed / residual / abort / continue / relaunch / cherry-pick tests are in
  `crates/graft-ops/tests/it/stop_hooks.rs` (`it::stop_hooks::<same name>`), not `apply::tests::stop_hooks` /
  `recover::tests`: they need `TempRepo`, `state_of`, `run_undo`, which live in `tests/it/support.rs`. The marking
  test is where the ticket names it (`src/apply/tests/stop_hooks.rs`). Extra: `it::stop_hooks::
  a_resume_stopped_in_its_hook_stays_paused` (the `resume` half of item 8) and `apply::error::tests::
  a_paused_write_says_so_and_only_a_stop_has_a_stopped_at`.
- **D2 — residual without a seam.** `cfg(test)` does not reach integration tests. The residual is driven by a real
  case instead: a hook that daemonises (`perl -e 'setpgrp(0,0); …'`). The stop finds no helper in git's group, stops
  the group — git alone — and git dies before it pauses: no `MERGE_HEAD`, the merge's files in the tree (13b's
  state). The test kills the hook's pid itself (`Hooked`'s drop: only pids the hook wrote, never a pattern).
- **D3 — `HEAD` unmoved** is part of Paused for merge, cherry-pick and revert, not rebase (a rebase moves `HEAD` as
  it goes; its live state alone says paused, as `stopped_on_conflict` read it). `HEAD` before = the entry's
  `refs_before[head_before]` (or the detached oid); unrecorded counts as unmoved (`stopped.rs:113`).
- **D4 — the stop is read for local lines only** (`PlanFailure::stopped_locally`, `apply/run.rs`). A remote
  operation's stop keeps `remote::short`'s words — a pull's local merge included, since `run_plan` gives every line of
  a remote op a `RemoteFailed`. **Pull:** `opens_in_progress(Pull)` = `None` (`operation.rs:1254`), so a pull could
  never be Paused or Landed under this rule anyway.
- **D5 — where the residual's words do not reach.** A write that set edits aside closes through `set_aside::close`,
  whose reason stays `git was stopped` (`set_aside.rs:175–186` call `reason_for`); a folded step keeps it too.
  `stopped_at()` still maps them to `BeforePause` (the variant is `Git { error: Stopped }`).
- **Kept variant (item 8):** the residual returns today's `ApplyError::Git { error: RunError::Stopped, .. }`.

### Questions for the conductor (rule 7: lines given)
- **Q1 — a rebase is not stopped hook-first (deviates from item 7, keeps its fence).** Item 7 marks `apply::engine`;
  the Touches fence says `stop_now_mid_rebase_with_edits_set_aside` must stay green. Both cannot hold: with the
  engine marked, the walk's `post-checkout` hook is ended, git **finishes the rebase** (Step 0, exit 0), and the walk
  failed at `crates/graft/tests/it/quit_waits/stuck.rs:302` (`On branch feature … modified: base.txt`); with Landed
  Merge-only the entry was also journaled `Aborted "git was stopped"` for a rebase that landed. Built: `engine::
  rewrite_git` and a rebase's `resume_invocation` are **not** marked (doc at `apply/engine.rs` `rewrite_git`), so the
  group stop leaves the rebase paused — walk green, entry `Pending`, `c`/`a` offered. Flip = add `.stop_hooks_first()`
  in `rewrite_git` and the rebase arm of `resume_invocation`, then decide the walk's new expectation.
- **Q2 — Landed is Merge-only (as specified), but cherry-pick and revert land too.** Step 0: a cherry-pick whose
  `post-commit` hook is killed exits 0 with the commit made. `where_stopped` (`apply/stopped.rs:74`) then reads
  `HEAD` moved, no `CHERRY_PICK_HEAD`, kind ≠ Merge → residual: `Aborted "stopped before git could pause it"`,
  `BeforePause { changed: true }` for a pick that landed. Nothing lost (`u` undoes it), the words are wrong. Same for
  a revert and a `--continue` whose `post-commit` hangs. Generalising Landed to "the op's in-progress kind not live,
  `HEAD` moved" fixes all three; not done, spec says Merge only. (Before 13c the same case read `git was stopped`.)
- **Q3 — the residual's words on a write git cannot pause.** A stopped discard now reads `stopped before git could
  pause it` (`tests/it/ops_execution_hard.rs` assertion updated to the spec's words). True, but "pause" means nothing
  for a discard. Keep, or keep `git was stopped` where `opens_in_progress` is `None`?
- **Q4 — `FinishMerge` (a merge begun in the shell, `c` from the banner).** `opens_in_progress(FinishMerge)` = `None`,
  so its stop is always the residual even when git leaves `MERGE_HEAD` live (`stopped_on_conflict` has the same
  blind spot, `apply.rs:761`). Not changed.
- **Out-of-scope question (asked by the ticket): a `pre-merge-commit` hook that rejects (exit 1), no stop.** Observed
  with a temporary test (removed): `ApplyError::Conflicted { kind: Merge, paths: [] }`, entry `Pending`, `MERGE_HEAD`
  present, `stopped_at() = None`, `explain()` = `merge feature into main: 0 unmerged · resolve, then c continues, a
  aborts`. Path: `closed` → `stopped_on_conflict` (`apply.rs:418`, `:761`) — live `Merge` after a non-zero exit is a
  stop; the words from `error.rs:104`; graft lands it as `Operated::Conflicted` (`jobs/operate.rs:388`). The
  `0 unmerged · resolve` wording for a hook rejection is the thing to decide. Not changed.

### Verified, and how
- Gates at the Part B commit, workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo nextest run` (5607 passed, 10 skipped), `cargo test --doc` — all pass.
- Mutations (each reverted by hand, not `git checkout`): Part A — without the group and hooks-first, 6 of the
  `run::tests::stop` tests fail. Part B — `plan_git` unmarked: 5 of the 9 `it::stop_hooks` tests fail (the
  `prepare-commit-msg` and cherry-pick ones still pass: git writes `MERGE_HEAD`/`CHERRY_PICK_HEAD` **before** that
  hook, so even the group stop leaves them paused); the outcome rule off: 8 of 9 fail (all but the pure-git one).
- After the run, `ps` showed no `sleep 600` / `setpgrp` process left.
- Not verified: Linux `/proc` member listing (macOS host only; `members_lists_a_child_in_the_group` is
  host-OS); anything in the app (Part C).

### Touches (Parts A–B)
- Every `run` spawn now leads its own process group (`process_group(0)`): checked by the full nextest run (5607
  green); the editor handover is Graft's own spawn, not `run` — not yet re-grepped for Part C (`/dev/tty`,
  `SSH_ASKPASS`, pager grep is still to do).
- `SHUTDOWN_GRACE` follows `STOP_WITHIN` (4.05 s): `the_shutdown_grace_covers_a_git_childs_stop`,
  `no_git_child_outlives_shutdown` green in the run.
- `ApplyError::Paused`: the one exhaustive match (`graft/src/jobs/operate.rs` `refresh_after_failure`) and
  `answer` (Paused lands as `Conflicted`, item 12 — compiler-checked, **no test of its own yet**, Part C).
- `Applied` literals: 6 in graft-ops, 12 in graft (`stopped_at: None`).
- Knock-on tests updated: `it::set_aside::a_stop_mid_rebase_is_held_and_abort_puts_the_edits_back` (`Conflicted` →
  `Paused`), `it::ops_execution_hard::a_plan_stopped_between_invocations_is_aborted_recoverable_and_narrow` (Q3).
- `resume`: a stop now holds (`Paused`) or closes the entry instead of returning `ApplyError::Stopped` with the entry
  untouched; `recover_interrupted` lists a paused entry `live` (`a_paused_merge_is_live_at_relaunch`).
- Journal/undo: no new entry kind; `Paused` uses `hold_commands` as a conflict stop does.
- Refactors for clippy's line limit, pure moves: `resume_args`, `closed_set_aside`.

### Remaining — Part C (graft), not started
1. `exit_line(bin, what, after, at: Option<StoppedAt>)` with item 10's five lines; test
   `update::quit::tests::exit_line_says_where_git_left_it`; update the 13b exit-line tests.
2. `Forced { job, what, at }`, `FORCED_WAIT = SHUTDOWN_GRACE + 3 s`; the forced job's answer must reach `wind_down`
   (`event_loop/exit_wait.rs`). **Scout finding:** today the pool replaces a cancelled job's `Operated` with
   `JobCancelled` (`graft/src/jobs/pool.rs:326`), including after the stop box's `y` — that is what must change.
   Test `jobs::tests::a_forced_job_is_waited_for_its_answer_up_to_its_bound` + the `None` line past it.
3. A test for item 12 (`answer(Err(Paused))` → `Operated::Conflicted`); the arm is in.
4. Walks: `quit_waits::stuck::stop_now_mid_merge_leaves_it_paused` (today's `stop_now_mid_merge…` at
   `tests/it/quit_waits/stuck.rs:244–256` only prints the no-`MERGE_HEAD` state, asserts nothing), the relaunch walk,
   the signal tests' rename/new line, the capture `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt`
   (`it::quit_waits::capture_relaunch_after_a_stopped_merge`; the relaunch banner is drawn in graft-ui — not scouted),
   journey fence `quit-stuck-write 4`, the SIGHUP pty observation.
5. Doc edits (verbatim, keymap §2.2 and design note :140); graft MAP rows (`exit_line`, `Forced`, `FORCED_WAIT`);
   the Touches grep for `GIT_EDITOR`/`core.editor`/`/dev/tty`/`SSH_ASKPASS`/pager.
6. Q1's answer changes the rebase walk; Q2's changes the cherry-pick exit line.
