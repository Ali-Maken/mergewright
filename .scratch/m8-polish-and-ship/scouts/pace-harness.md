# Scout: pace harness (ticket 02 fence) — crates/graft/tests/it

Paths below relative to `crates/graft/tests/it/` unless stated.

## 1. drain_sink.rs
- `DrainSink(Arc<Shared>)` :38; `Shared{state: Mutex<State>, room: Condvar, capacity}` :40; `State{buffered, written, progress: Vec<(u64,Instant)>, last_activity, scan: Scan, transmits}` :47. Keeps counts only, never bytes (:5-7).
- Drain thread `drain(&Weak<Shared>, rate)` :133: every 1 ms (`DRAIN_TICK` :18) subtracts credit from `buffered`, pushes `progress`, `notify_all` on `room`. Exits when every handle dropped (:140). It sees no bytes, only counts.
- Writer `PipeWriter` :167: `write` blocks on `room` while full (:176), takes `min(len, room)`, and **scans at write time** — `scan.feed(written, &buf[..taken], transmits)` :189 — not at drain time.
- `Scan` :216 is a byte state machine (`Parse` :203) for `ESC _ G … ESC \`; state persists across writes (split-safe; test `counts_kitty_transmits_across_writes` :357 cuts inside a header). It recognises only APC-G; `ESC [` falls to `Outside` (:241). No CSI/DSR parse exists.
- Public: `new(capacity, bytes_per_second)` :74, `writer()` :99, `written()` :104, `drained_at(offset)` :110, `quiet_for(d)` :120, `kitty_transmits()` :126.
- **No wake channel access**: no `Sender` in `Shared`/`State`; constructor takes none. A fence detected in `Scan::feed` would know its byte offset; "answer as it drains" would need the drain thread to compare `progress` against that offset — no such link today.

## 2. held_key.rs / held_key_bound.rs
- Gating: `main.rs:39-44` — `held_key.rs` only under `feature="heavy-tests"`; fast tier mounts `held_key_bound.rs` alone (pure `after_release` + test `b_bound_stops_at_js_frame` :39). All 7 held tests (:69-102) heavy, no `#[ignore]`; counts asserted every build, times release-only (:277).
- `run()` :288: `mpsc::channel::<Posted>()` :295; `ready_to_scroll_at(..., Post(post.clone()), &events, SIZE)` (frames/run.rs:189) then local `settle` :361 (rejects non-`Wake::Event` :378); `Session::settle(&Loaded::default(), encoding.capabilities(tier))` :306; `DrainSink::new(PIPE=16 KiB, rate)` :307; `Sink::draining(&drain)` :308; `in_memory(&sink,&state)` :309 (CrosstermBackend<Sink>, frames/run.rs:274); `KeyFeed::start(script, post)` :316 — `post` (the `Sender<Posted>`) is **moved into KeyFeed**; `Paced::new(...)` :318; **`graft::event_loop::pump`** :319. After: waits `drain.quiet_for(1 s)` :331.
- Keys: 188 × `j` at 16 ms, + `J` 50 ms later in B (:130). `KeyFeed` thread sends `Posted{at, wake}` on script clock (frame_trace.rs:212-242).
- Capabilities: helper `frames::run::capabilities(tier)` (run.rs:351, literal) then fields mutated (:107-115), "never a literal" (:106).
- Measures: `frames_after_release` (frames drawn after last `j` stamp, via `after_release`), `tail_ms` = last `j` sent → `drain.drained_at(tail byte)` (:450-458), `j_ms`, bytes/frame, transmits. Frame bytes = `Paced.written[k]..[k+1]` (`sink.sent()` stamped at first `next_wake`/`waiting` look, paced.rs:185).

## 3. Wakes impl: `frames/paced.rs` `Paced`
- `impl Wakes for Paced` :259: `next_wake` blocks on `events.recv()` (`&Receiver<Posted>`) while script keys remain; after script, waits only for outstanding `ShowRows` windows (:281-299). `waiting`/`joining_waiting` use `try_recv` (:312, :360).
- **Contradiction for injection**: `Paced::take` :235-242 accepts only `Wake::Event`; any other wake (a `Wake::Fence`) sets `failure = "a wake the harness never sends"` and returns `None` (ends the loop). Same rejection in held_key `settle` :378.
- Injection route that exists: any clone of `Sender<Posted>` (`Post.0`, frame_trace.rs:175; `Posted{at,wake}` :165) reaches the same channel. Sink thread has none today. `Paced` also uses "script over → return None if no rows out" (:290) — a pending fence is not considered.
- `budget_run.rs` (heavy-gated, main.rs:13): no `Wakes` impl; `pump_until_from` :624 is a plain `update` loop over `AppEvent`s (no draw, no Wake). `open_into` :553.
- `frames/run.rs::scroll` :63 uses `Sink::timed` :78 + `Paced` :88 + `pump` :97 (no DrainSink).

## 4. frames/after_idle.rs `g_after_idle` :52
- `frames` module heavy-gated (main.rs:31). Sink `Sink::timed(false)` :134 (in-memory, no DrainSink → no one to answer a fence). `Paced` :144, `pump` :145, one key `G` after 2 s gap. Asserts `gap_wakes.is_empty()` (:88; `unasked_wakes(0..key)` :242) — any fence wakes in the gap would land here. Latency asserted release-only (:95). Env `GRAFT_FRAME_NO_IDLE_ADVANCE` (:47).

## 5. `Capabilities { ... }` struct literals (no `..` update found)
- crates/graft/src/session.rs:245
- crates/graft/tests/it/frames/run.rs:353
- crates/graft/tests/it/captures.rs:24
- crates/graft/tests/it/frames/journal_find.rs:452
- crates/graft-term/src/detect/mod.rs:190, :229
- crates/graft-term/src/detect/tests.rs:159
- crates/graft-ui/src/test_support.rs:24
- crates/graft-ui/tests/support/mod.rs:26
- crates/graft-ui/examples/graph_column.rs:70
- crates/graft-ui/benches/row_render.rs:80
(graft-ui/src/graph_screen/tests.rs:228 calls `terminal(...)`, not a literal.)

## 6. Scripts
- `scripts/fix-scroll-held-key.sh`: `cargo test --release --features heavy-tests --test it --no-run` (:40), then `-- held_key:: --test-threads=1 --nocapture` (:46). Writes `docs/perf/fix-scroll-held-key.txt` or `$1` (:13): HEAD, machine, load, python-measured pty capacity, SUMMARY/VERDICT lines, results. Sources `scripts/lib/machine-state.sh` (:11). No env vars beyond that.
- `scripts/47-scroll-acceptance.sh`: honours `CARGO_TARGET_DIR` (:35-37); raw output in `$TARGET/47-acceptance`. `TESTS=(cargo test --release --features heavy-tests --test it)` :58; held_key in two runs (judged :83, raw Kitty pair :85, known-gap list :70); `frames:: heavy::budgets_hold` :183 (needs g_after_idle ran, :202); pty walk `python3 scripts/scroll-lag-pty.py "$BIN" "$REPO" 187 50 5 0.016 3` :218 with env `PROFILE`, `RATE`, `DEFLATE`, `THEN_J` (scroll-lag-pty.py:76-79); `ghostty_shot::` :269. Writes `docs/perf/47-scroll-acceptance.txt` (:38).
- **Flag**: scroll-lag-pty.py `REPLIES` (:60-68) has no `\x1b[5n` → `\x1b[0n` reply; matches `query in data` per 4 KiB/64 KiB read (:128), not split-safe; answers nothing when `PROFILE` unset.
