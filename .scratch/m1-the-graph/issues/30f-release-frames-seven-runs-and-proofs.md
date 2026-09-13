# m1-the-graph/30f — Release frame check 7/7, idle-wake report, proofs

Status: ready
Build with: **Opus** (high) — frame timing under the job/redraw loop (split from 30c part b seam 2)
Blocked by: 30e
Leaves runnable: yes — release frame check passes 7 runs of 7; idle-wake reported separately.

Why: the remainder of 30c part b after seam 2 (commit 5c4594e). Scope is the unchecked boxes of 30c's "Decision (key 37 / release assert flake) — user, 2026-09-13" section, minus the key-path fix (done in seam 2) and the eviction bug (30e).

State: `crates/graft/tests/it/frames.rs` reworked in 5c4594e (own-frame latency, replay check, idle-wake printed): **compiles, never run**.

## Read first
- Ticket 30c: "Decision (key 37 …)" section and part b handoff notes; ADR-0021
- TESTING.md "The heavy tier" (release perf run command)
- `crates/graft/MAP.md`; scouts for source

## Scope
In:
- Run and fix the reworked frames.rs (debug records, release fails — ADR-0021).
- Check that no key waits on a background answer (assert on the key path, already tested in seam 2 — confirm it holds in the frames run).
- Idle-wake (first key after ≥0.5 s idle): measured and printed separately in the release run, not failing.
- Release frame check: **7 runs, all 7 pass**; record every run's numbers in the ticket. A failure is a finding: find its cause, don't retry until green.
- The proofs from 30c's original scope, and one full release perf run.
- TESTING.md/KNOWN_GAPS updates the result implies. Tick 30c's boxes; mark 30c done if all are done.
- Report: cause or symptom per fix, and what it costs the user.

Out: eviction (30e). The user's by-eye check (no lag after a pause) happens at the gallery re-walk.

## Acceptance
- [ ] frames.rs green in debug (records) and release.
- [ ] 7/7 release runs recorded.
- [ ] Idle-wake numbers printed separately.
- [ ] Proofs done; release perf run numbers in the ticket.
- [ ] Gates pass.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
