# 04b — frames harness feeds keys from an input thread, as the app does

Status: ready
Blocked by: 04 (merged)
Build with: Opus
Crates: graft (the `frames` perf tests and their harness only; no product change).

Source: ticket 04's measurement (`docs/perf/m7-04-slow-key.txt`, `## cause`). The first key after idle reads late
because the **harness's own** `recv_timeout` wakes up to 10.0 ms late after the 600 ms pause: OS timer slack, present with
no Graft code at all (control D). The real app wakes on a thread send (7–36 µs). Conductor's ruling (2026-09-26): the
harness must deliver keys the way the app receives them, so the metric measures Graft, not the OS timer. This fixes the
measurement's cause; the latency definition is not loosened.

- Keys reach the loop from an input thread that sleeps the script's pauses and sends, the way `InputMux` forwards
  stdin (name the file:line you mirror).
- Re-run `scripts/m7-perf-key.sh`; the perf file gains a rev-2 section: first-key-after-idle before/after, p99, max.
- Control D (no Graft code) stays in the script so the slack stays visible.

## Acceptance (each tick names its test, at tick time)

- [ ] Harness test: a key sent after a 600 ms idle is received within 1 ms of its send (release), named.
- [ ] `frames` gate green in release with the input thread; the perf file's rev-2 SUMMARY lines quoted in NOTES.
- [ ] MAP.md / TESTING.md "Performance" row updated.

Touches: `crates/graft/tests/it/frames.rs`, `frame_trace.rs`, the perf script, TESTING.md, MAP.md. Blocks 12-idle-layout-advance
(same files) — run before it.
