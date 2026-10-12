# 58 — J during a finishing rebase writes two outcomes for one journal entry

Status: ready
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
- [ ] A test reproducing two outcomes, seen red before the fix (name it, quote the red line)
- [ ] Same test green after; whole suite green (gates workspace-wide)
- [ ] Report says cause or symptom, and what it costs the user
