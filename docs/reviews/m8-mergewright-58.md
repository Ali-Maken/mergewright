# Review — m8-polish-and-ship 58: journal two outcomes (writer's `flock`)

Fixed hash: `da1c79cb` (merge). Diff: `git diff da1c79cb^1 da1c79cb`, 9 files, +371/−13, all `graft-ops`.
Read-only review. **No cargo run** (conductor: gates running on main) — every finding below is
from reading the code. Skill: `code-review`.

## Verdict

The cause is right and the fix closes it: a `Pending` entry whose writer is alive can no longer be
closed by recovery, in one process or two. No deadlock found. But the read that the fix now makes
*safe* still makes the screen say the wrong thing: a busy read reports "nothing is live", and the
app trusts it (Major 1). The question about `open` cutting a torn tail was reported and then left
out of `KNOWN_GAPS.md` (Major 2).

**Critical 0 · Major 2 · Minor 8**

## Ticks (3 of 3 checked, 3 held)

| Tick | Checked | Held? |
|---|---|---|
| Red test `journal_two_writers::a_journal_read_mid_rebase_leaves_the_running_entry_one_outcome` | The test exists (`tests/it/journal_two_writers.rs:84`). Before the fix, recovery inside the stop poll closes the rebase's entry, then the rebase appends `committed` from its stale copy, and `open_as` panics with `DuplicateOutcome`. That matches the quoted red message. **Not re-run**: no cargo. | yes (by reading the code) |
| Green, plus the two unit tests | `journal/file/tests.rs:27` and `:63` exist and assert what the tick says. The 6430/6430 count was not re-run. | yes (by reading the code) |
| Cause or symptom, and the cost | NOTES say "Fix (cause)" and give a "Cost to the user" paragraph. That paragraph is incomplete: see Minor 5. | yes |

**The red test is deterministic.** It uses no sleeps. `Command::run` checks `should_stop` before it spawns each git (`graft-git/src/run.rs:422`), so the after-state reads that follow the rebase poll at least once inside the window. If that window is never reached, `recovered.expect(...)` fails loudly (`journal_two_writers.rs:103`); the test cannot pass by accident.

## Checks asked for

- **Lock ordering and deadlock: no cycle found.**
  - Only one call ever waits: `claim()` taking the shared lock (`journal/file.rs:140`). Taking the lock alone is try-only (`:155`), and so is the rename move's `lock_alone` (`rename/locks.rs:80`).
  - While recovery holds the journal alone, it runs git subprocesses only (state reads, `seal_recovered`, stash list). None of them waits on a journal lock.
  - The rename lock lives in a separate file (`<dir>/lock`), not the journal.
  - No self-deadlock:
    - `alone()` drops this `Journal`'s own shared lock first (`file.rs:154`).
    - Recovery running inside the writer's own thread gets `None`, not a wait. The red test exercises exactly this case.
- **Every writer path holds the lock.** Every outcome goes through `record::finish`, called from `restore.rs:317`, `set_aside.rs:197/:315`, `close.rs:130/:299`, `restart.rs:399` and `resume/close.rs:103`.
  - The `Pending` each of those finishes came from `begin` → `append_pending` → `claim` (`apply.rs:326`, `record.rs:136`), or from `resume`, which claims at `resume.rs:131`.
  - `rename/record.rs:74` goes through `append_pending`, so it claims too.
  - `mark_interrupted` is called only by recovery, under `Alone` (`recover.rs:107`).
  - `hold_commands` writes nothing to disk.
- **A process that dies holding the lock:** the kernel releases the `flock` when the descriptor closes. `std` opens with `O_CLOEXEC`, so git children never inherit the lock.
- **Platforms:** Windows is a non-goal (PROMPT.md:19). `nix` `flock` arrived with m8/26, not this ticket. Filesystems without `flock` and NFS: see Minor 6.

## Major

### 1. A busy read reports "nothing live", and the app shows a running or stopped operation as interrupted

`crates/graft-ops/src/apply/recover.rs:47-48`. When any writer holds the lock, `recover_interrupted` returns `Recovered::default()`, which has `live: []`. `read_journal` passes that through (`crates/graft/src/jobs/undo.rs:574-598`).

The app then overwrites what it knew: `state.live_entries.clone_from(&read.live)` (`crates/graft/src/update/journal/answers.rs:32`). The comment just above that line (`:28-31`) forbids exactly this: "silently claiming 'nothing is live', which would put the interrupted notice back on a merge that is standing right there".

Two things go wrong on screen:

- **A stopped merge or rebase loses its live mark.**
  - The footer notice treats any `Pending` entry not in `live_entries` as interrupted (`crates/graft/src/update/undo/notice.rs:56-63`).
  - The journal row is marked `Interrupted` (`crates/graft/src/view/journal.rs:191-198`).
  - The banner wording (`update/banner/stop/wording.rs:206`) and the commit-editor refusal (`update/commit_editor/refusal.rs:32`) read the same field.
  - Before this fix, every read worked out `live`, so this part is a **regression**.
- **The running operation's own entry** comes back `Pending` and not live, so it is drawn as "interrupted" with a footer notice. Every journaled operation holds the lock for its whole run (`apply.rs:326`), push and fetch included, so the window is the full network time.

**Triggers** (all lines cited):

- `J` or refresh during any journaled operation (the ticket's own NOTES: `update/journal/keys.rs:62`, `update/keys/refresh.rs:30`).
- A read during a resume (`resume.rs:131` claims before git moves).
- Two concurrent journal reads: `J` plus refresh, or `J` twice. The loser of `try_exclusive` answers `live: []`, and since it is the newer id it is the answer kept (`update/journal.rs:171-180`).

**Why it matters:**

- CLAUDE.md: "A status field must never read as an error when nothing is wrong."
- It sits on the M3 promise: a live conflict shown as interrupted invites `u` on it.

The new test checks only `recovered.closed` (`journal_two_writers.rs:118-122`). Nothing checks what the read *reports*. The ticket's Touches list names "the J screen", but the NOTES do not say whether it was checked.

**Fix:**

- Let a busy read say so. Add a `busy: bool` to `Recovered`, or better, list the entries held by a running writer. Still work out `live` from git's state, which is a read and needs no lock.
- `answers.rs` keeps the previous `live_entries` on a busy read.
- The view draws a writer-held `Pending` entry as running, not interrupted.
- Add a test: while a writer holds the lock, a read keeps a standing conflict live and does not mark the running entry interrupted.

### 2. `Journal::open` still cuts a "torn tail" with no lock, and the question was not routed

`crates/graft-ops/src/journal/file.rs:97-104` truncates the file at the last newline whenever the bytes it read end mid-line. It does this with no lock, while another job (or window) may be appending.

The debugger saw this and reported it as a question ("Seen, not fixed", ticket NOTES). It is **not in `KNOWN_GAPS.md`**: `bd2c580e` adds only `M8/58-two-windows-flock` and `M8/58-corrupt-journal-refused`. That breaks CLAUDE.md rule 4.

The consequence is the same class as this ticket. If the cut line is a `Pending` line, the later outcome has no `Pending`, and `parse` refuses the whole journal (`journal.rs:112-113`, `OutcomeWithoutPending`).

What is **unverified**: whether a read on macOS APFS or Linux ext4 can see a partial `O_APPEND` write. It is filed as a question about behaviour, but the missing gap row is a fact.

**Fix:**

- Add a `KNOWN_GAPS` row now: question, `file.rs:97-104`.
- The cheap cause fix now exists: cut the tail only while holding the journal `alone()`, and otherwise ignore it the way `reread` already does (`file.rs:164-168`).

## Minor

1. **`alone()` silently gives up this `Journal`'s own claim.** `file.rs:153-154`. The rule ("asking alone says this `Journal` is not part-way through an entry") lives only in a doc comment. If a future caller recovers on a `Journal` with an entry in flight, its outcome is written unprotected, and nothing warns. Fix: return an error, or `debug_assert!(self.writing.is_none())`, rather than dropping the lock.

2. **The resume path has no red test.** The fix's second writer site, `resume.rs:131`, is not exercised: `J` mid-resume after git finishes the continue. Only the `apply` path is tested (`journal_two_writers.rs`). Fix: the same stop-poll reader, driven through `resume(Continue)` on a stopped rebase.

3. **`claim()` waits and cannot be cancelled.** `FileLock::shared` blocks in the kernel (`graft-git/src/file_lock.rs:45-50`; `file.rs:140`). While it waits, the job's `Cancel`/`Stop` is not checked. Recovery holds the journal alone across `read_state_now` for every dead entry (`recover.rs:89`), which can take seconds on a large tree. Fix: note it in the cost, or try a non-blocking shared lock and poll `Stop`.

4. **No ADR or design entry for an on-disk protocol shared across processes.**
   - Who may close a `Pending` entry, and how that is decided, is described only in `file.rs:12-21`.
   - `docs/design/06-undo-journal-design.md` §6.2 (`:79-83`) still says only "on next launch".
   - Builds from before 58, Graft-generation ones included, don't take the lock, so windows running mixed builds still race.
   - CLAUDE.md: hard-to-reverse decisions go in an ADR. Fix: a short ADR or a §6.2 paragraph.

5. **The cost note leaves out how deferred recovery changes the after-state.**
   - `recover.rs:21-24` says: "the later it runs, the more of the user's own work it attributes to that operation."
   - With `recover.rs:47-48`, suppose an operation starts before the launch read takes the lock. The dead entry then closes after that operation, and its after-state includes it. `u` on that entry would later meet drift.
   - The doc line "Run this at launch, before the user can touch anything" (`:21`) is no longer what happens.
   - This is a **question** on impact. Fix: state it in the cost, or have the launch read take the lock before the first write can be queued.

6. **Filesystems without `flock` (question, unverified).**
   - `claim()` turns any `flock` error into an operation failure (`file.rs:140`, `file_lock.rs:47-49`). On a mount that does not support `flock` (some SMB/FUSE), every journaled operation would now refuse to start. Before this fix, none took the lock.
   - Linux NFS emulates `flock` with per-process POSIX locks. Two `Journal`s in one process would then not contend, and the open and close in `append` (`file.rs:319-324`) and in `reread` (`:170`) would release the lock.
   - Fix: decide and document. Either degrade to the old behaviour with a status note, or refuse with words that name the filesystem.

7. **The NOTES have no `Touches:` line** (CLAUDE.md, Running work). The ticket's own list includes the J screen and captures showing J after a rebase; neither is reported checked. Major 1 sits exactly there.

8. **The same error construction is written twice.** `resume.rs:131-135` builds `ApplyError::Journal { error, after: None, kept: None }` inline. That is exactly what the new `recover.rs:121` `journal_failed` names. Fix: share one helper.

## Not re-filed (known, gapped)

- `M8/58-two-windows-flock`: no real run with two windows.
- `M8/58-corrupt-journal-refused`: a journal already holding two outcomes is refused whole.

## Gates

Not run, per the conductor (gates running on main; reviewer read-only). The fast-suite time, heavy tier, release perf run, memory and `target/` size are **not measured in this review**.
