# 04b — frames harness feeds keys from an input thread, as the app does

Status: done
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

- [x] Harness test: a key sent after a 600 ms idle is received within 1 ms of its send (release), named. *(`frame_trace::a_key_sent_after_idle_is_received_within_1ms_of_its_send`; asserts < 1 ms in release, prints in debug. Release, in all 6 script runs: send -> recv 23-32 µs (perf file rev 2, `frame_trace:` lines))*
- [x] `frames` gate green in release with the input thread; the perf file's rev-2 SUMMARY lines quoted in NOTES. *(`frames::*` via `scripts/m7-perf-key.sh`: 6 runs, all `exit 0`, every `spread` line `gate(p99<=16ms)=ok`; quoted below)*
- [x] MAP.md / TESTING.md "Performance" row updated. *(by reading: MAP.md frame-trace line names `KeyFeed`/`SentLog`/`Sent`, `open_into`, `pump_until_from`; TESTING.md "Where a slow key's time goes" says latency runs from the send, what `oversleep`/`queued` mean now, and the rev-N append)*

Touches: `crates/graft/tests/it/frames.rs`, `frame_trace.rs`, the perf script, TESTING.md, MAP.md. Blocks 12-idle-layout-advance
(same files) — run before it.

## NOTES (builder, 2026-09-26)
Scout: none new — used `.scratch/m7-power-tools/scouts/04.md` plus 04's NOTES, as briefed. Compacted: no. Interrupted
once by a network error while waiting on the perf run; the run had finished and written rev 2, nothing re-run.

**What changed (harness only, no product code).** `frame_trace::KeyFeed` is an input thread: it sleeps to each key's
due tick (script clock: key i due one pause after key i-1, however late it woke), stamps `Sent { due, at }`, and
sends `Wake::Event(AppEvent::Core(Event::Key(press)))`. That mirrors `spawn_reader` in `crates/graft/src/event_loop/input.rs`
(the `Wake::Event(AppEvent::Core(event))` at :39, sent at :44). The jobs report into the same `Sender<Wake>`
(`impl JobEvents for Sender<Wake>`, `event_loop/mod.rs`, via new `budget_run::open_into`). `Paced` now waits with `recv()` and
looks with `try_recv()`, as `ChannelWakes` does. A key's latency runs from its send. The trace's `oversleep` is now
due → send: the thread's own sleep, outside the latency but still printed. Control D (`the_machine_alone_after_idle_and_warm`) is unchanged.

**Rev-2 SUMMARY lines** (`docs/perf/m7-04-slow-key.txt`, `# ===== rev 2`; µs, idle = median(max) over 3 plain runs):
```
SUMMARY rev2 idle tier=Pixel refs=FiftyThousand pane=Closed: latency before=12309(16020) after=3206(7556); oversleep (after: outside the latency) before=10009(10037) after=7046(10040)
SUMMARY rev2 idle tier=Unicode refs=FiftyThousand pane=Closed: latency before=10287(12383) after=1977(2442); oversleep (after: outside the latency) before=7934(10042) after=9965(10038)
SUMMARY rev2 idle tier=Unicode refs=FiftyThousand pane=DiffScroll: latency before=9568(11909) after=1807(1887); oversleep (after: outside the latency) before=7791(10041) after=8488(10043)
SUMMARY rev2 idle tier=Unicode refs=FiftyThousand pane=Open: latency before=8890(11895) after=1803(1857); oversleep (after: outside the latency) before=7838(10038) after=6710(10034)
SUMMARY rev2 idle tier=Unicode refs=FiftyThousand pane=Map: latency before=9401(11136) after=1094(1115); oversleep (after: outside the latency) before=8538(10044) after=10025(10044)
SUMMARY rev2 idle tier=Pixel refs=Fixture pane=Closed: latency before=12424(17271) after=1518(7695); oversleep (after: outside the latency) before=9644(10034) after=7828(10028)
SUMMARY rev2 idle tier=Unicode refs=Fixture pane=Closed: latency before=10499(12320) after=1945(2551); oversleep (after: outside the latency) before=9127(10061) after=10022(10034)
SUMMARY rev2 spread tier=Pixel refs=FiftyThousand pane=Closed: p99 before=7934-8308 after=4206-6205; max before=9485-12235 after=21193-23294; gate(p99<=16ms)=ok
SUMMARY rev2 spread tier=Unicode refs=FiftyThousand pane=Closed: p99 before=8839-9471 after=2798-2838; max before=9637-10007 after=14325-18898; gate(p99<=16ms)=ok
SUMMARY rev2 spread tier=Unicode refs=FiftyThousand pane=DiffScroll: p99 before=8360-8820 after=1534-1580; max before=9049-9239 after=10759-12172; gate(p99<=16ms)=ok
SUMMARY rev2 spread tier=Unicode refs=FiftyThousand pane=Open: p99 before=8659-8724 after=663-1686; max before=8948-9210 after=10975-20740; gate(p99<=16ms)=ok
SUMMARY rev2 spread tier=Unicode refs=FiftyThousand pane=Map: p99 before=8360-8580 after=431-1079; max before=8836-8940 after=2576-9725; gate(p99<=16ms)=ok
SUMMARY rev2 spread tier=Pixel refs=Fixture pane=Closed: p99 before=8152-8416 after=2237-5869; max before=8935-10630 after=2801-10131; gate(p99<=16ms)=ok
SUMMARY rev2 spread tier=Unicode refs=Fixture pane=Closed: p99 before=8427-9161 after=869-2491; max before=8858-9820 after=964-3330; gate(p99<=16ms)=ok
```

**Finding: the max rose in 4 scenarios (to 12-23 ms) — cause identified from code; the events it waits behind are not.**
Every scroll key over 16 ms in rev 2 is **key 1** (the second key, ~17 ms after the start), each one `queued` 12-23 ms
behind `other_event` frames of 0.2-1.2 ms. Its own frame is 0.2-0.3 ms. The old `Paced::next_wake` called `recv_timeout`
only when `wait` was nonzero, so an overdue key was given to the loop **ahead of** job events already queued (pre-04b
frames.rs, the `if !wait.is_zero()` branch in `next_wake`). The new harness is FIFO, like the app's `ChannelWakes::next_wake`
(`event_loop/mod.rs`, `self.receiver.recv()`), so key 1 now waits behind the job events still arriving just after
`ready_to_scroll`. **Not verified:** which job events those are, and whether the real app at startup gets the same burst.
That question needs a trace of the non-key wakes, and it belongs to 12-idle-layout-advance or a follow-up. The gate is the
p99 and holds. I changed nothing to hide the burst.

- Script fix found on the way: the first rev-2 append read the perf file after its own marker, so the `before=` columns
  came out `-`. The script now reads a copy (`$WORK/old.txt`). The rev-2 header lines above were recomputed from the
  rev-1 section and the rev-2 body with the same parser. No run was repeated.

**Touches:**
- `budget_run.rs` (shared with `many_refs.rs` and `examples/budgets.rs`): `open`/`pump_until` keep their signatures and
  delegate to the new `start`/`pump_until_from`. Check: clippy `--workspace --all-targets` passes with and without
  `graft/heavy-tests` (examples included). nextest 3562 passed.
- `frames.rs`: only the harness. `Wakes` impls unchanged in product (`ChannelWakes`, `ScriptedWakes`). No product diff:
  `git diff --stat` touches `tests/`, `scripts/`, docs only.
- Fast tier: +1 test (~0.6 s sleep).
- Perf script: filter now `frames:: frame_trace::`; it greps `frame_trace:` lines; it appends `# ===== rev N` when the file exists.
- 12-idle-layout-advance: the rev-2 numbers are its new baseline (idle latency median 1.1-3.2 ms, max 7.7 ms; p99 0.4-6.2 ms).
- Keymap, captures, journal: none.
