# 58 — J during a finishing rebase writes two outcomes for one journal entry

Status: done (debugger)
Build with: debugger (Opus, high) — cause unconfirmed, undo/journal area
Blocked by: none
From: 57 NOTES (S5 walk), reported not fixed

## What was seen (57's builder; intermittent, frequency not measured)
After an in-app rebase, `J` pressed after git finished but before the rebase's outcome line was written left a
pending line, then `"status":"committed"`, then `"status":{"aborted":{"reason":"interrupted"}}` for the same id.
J then read `the journal could not be read: journal entry <id> has two outcomes`. J and `u` refuse the whole
journal until the file is fixed by hand — breaks the M3 promise "you cannot lose work".

## Lines named by the reporter (rule 7)
- `J` queues `Command::ReadJournal` — `crates/graft/src/jobs/runner/operations.rs:224-234`
- its job runs `graft_ops::apply::recover_interrupted` — `crates/graft/src/jobs/undo.rs:575`
- recovery closes every `Pending` entry when `state::in_progress` reports nothing live — `crates/graft-ops/src/apply/recover.rs:37-55`
- appends via `Journal::mark_interrupted` — `recover.rs:89` → `crates/graft-ops/src/journal/file.rs:191`, which checks only its own in-memory copy (`file.rs:196-202`)
- the rebase job then appends `committed` from its own copy.
Suspected (NOT confirmed): nothing serialises the read job's recovery against a write job still running in the same process.

## Do
Use `diagnosing-bugs`. Confirm or refute the cause with a test that goes red first (deterministic — no sleeps, no retries;
CLAUDE.md rule 2). Fix the cause. Also decide, with the line, whether the reader should survive an already-corrupted
file (two outcomes) rather than refuse the whole journal — if that is a design change, report it, don't build it.

## Touches (check each)
Every caller of `recover_interrupted` and of `Journal::mark_*`; launch-time recovery; other windows on the same repo
(cross-process lock); undo/redo; the J screen; captures that show J after a rebase.

## Acceptance
- [x] A test reproducing two outcomes, seen red before the fix — `graft-ops::it journal_two_writers::a_journal_read_mid_rebase_leaves_the_running_entry_one_outcome`, red 3/3 runs: `the journal still reads after a read raced the rebase: DuplicateOutcome(EntryId("01HF7YAT000000000000000000"))`
- [x] Same test green after; whole suite green — same test, plus `journal::file::tests::a_writer_keeps_recovery_out_until_it_drops`, `a_claim_reads_what_recovery_closed`; nextest workspace 6430/6430, `cargo test --doc` ok, fmt, clippy `-D warnings`
- [x] Report says cause or symptom, and what it costs the user — NOTES below

## NOTES (debugger)

**Cause confirmed.** `recover.rs:37-55` (pre-fix) closed every `Pending` entry git did not report live, with no
way to tell a dead run's entry from one whose writer is still running. `J` (`update/journal/keys.rs:62`) and
refresh (`update/keys/refresh.rs:30`) queue `ReadJournal`, classed a read (`update/place/write.rs:59`), so it runs
on a second worker beside the operation; each job opens its own `Journal` (`jobs/undo.rs:352`,
`jobs/operate.rs:402`), and `append_outcome` checks only its own copy (`journal/file.rs`). Repro: the read run
from the rebase's own `Stop` poll, the first poll after git moved the branch — deterministic, no sleeps. The test
saw `interrupted` then `committed`; the reporter's order (`committed` then `interrupted`) is the same race with
recovery's read before the outcome and its write after.

**Fix (cause).** An advisory `flock` on the journal file itself (`graft_git::file_lock`): a writer holds it shared
from its first `Pending` line (`Journal::append_pending` → `claim`) or, resuming, from before git moves
(`apply/resume.rs`), until its `Journal` drops; recovery (`recover_interrupted`) closes anything only while it holds
it alone (`Journal::alone`), and both re-read the file under the lock. Kernel drops a dead process's lock, so
crashed runs still recover; two `Journal`s in one process contend as two processes do, so other windows are
covered by the same rule (not exercised with two real processes).

**Cost to the user.** A dead run's entry pressed `J` during another operation closes one read later (the read after
that operation). An operation that starts while recovery runs waits for it (kernel wait, the length of recovery's
state reads).

**Already-corrupted journals (question, not built).** `journal.rs:116` (`parse`) refuses the whole file on a second
outcome. Surviving it means choosing which outcome wins — here the first, there the second — so it is a design
change; journals already damaged by this bug stay refused until fixed by hand.

**Seen, not fixed.** `Journal::open` (`file.rs`, torn-tail cut) truncates an unterminated last line; a second job
opening the journal while another is mid-append could in principle cut that line. Not reproduced; question only.
