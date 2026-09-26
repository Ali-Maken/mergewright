# 27 graft-git-run-fixes — gap fixes in graft-git found during M7

Status: ready
Blocked by: none
Crates: graft-git only.
Build with: Opus. Concurrency in the git runner.
Filed by: conductor, 2026-09-26, from KNOWN_GAPS rows with lines. Not a design change.

## Scope
1. **KNOWN_GAPS M7/dbg (unverified):** the debugger read two races in the run loop:
   - `crates/graft-git/src/run.rs:478` checks the stop before `try_wait`. A git that already finished is reported as
     stopped.
   - `run.rs:331` spawns git before the first stop check.
   **Reproduce first**, with a test that fails today (e.g. a stop requested just after git exits). If a race doesn't
   reproduce, say so, with the attempt, and don't change that code. Fix at the cause. Check every caller that reads
   "stopped" (grep, list them in Touches).
2. **KNOWN_GAPS M7/19b (broken):** heavy-feature clippy fails on `examples/trace_perf.rs` (m7/18). Make
   `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` clean for graft-git. (Use the
   feature name the repo uses.)
3. **KNOWN_GAPS M7/18c (question):** the 18 builder reported two parse failures:
   - `diff/unified/primitives.rs:54` can't parse a mixed-quoted `diff --git` header;
   - `log -L` across a non-ASCII rename returns `Parse`.
   **Verify each with a real-git test first.** Reproduced → fix at the cause. Not reproduced → say so.

## Acceptance (name the verifying test at tick time)
- [ ] 1a / 1b: the red-first test for each race, or "not reproduced" with the attempt.
- [ ] 2: the heavy clippy line, clean.
- [ ] 3a / 3b: the real-git test for each, or "not reproduced".

## Gates
Workspace-wide, plus the heavy clippy line.
