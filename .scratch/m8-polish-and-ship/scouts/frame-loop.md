# Scout: frame loop (trace-product-frame-trace, pace-fence-and-sync)

All paths under `crates/`. Code at b5fa1032.

## pump structure
- `pump` running.rs:38 → `pump_with` :92 → `frames` :118. `release` (free uploads) after loop :113.
- Turn: `next_wake` :143; `graphics.drain()` (image replies) :150, `mark(Drained)`; **update** :174-176 (`mark(Updated)`); **carry_out** :178-186 (`mark(CarriedOut)`); batch check :200-204 (`joining_waiting`, `BATCH_BUDGET` 8 ms :110); **draw** :205, `mark(Drawn)` :207. `Wake::Terminal` draws at :164 without the phases.
- Phase marks exist already: `Wakes::mark` / `enum Phase` event_loop/mod.rs:~115-140 (perf seam, empty default).
- **draw** running.rs:426-458. Write is NOT one call per frame: `write_all(BEGIN 2026)` :437, ratatui `terminal.draw` (many writes via backend) :441, `write_all(images)` :447, `write_all(END 2026)+flush` :453-455. Backend = `RowClear(CrosstermBackend::new(io::stdout()))` startup.rs:315; no BufWriter found (stdout is std's LineWriter). Output also guarded by `redetect::hold_output()` :435.
- Other writes outside draw: OSC 52 clipboard :386-395, release :398-414, RowClear row_clear.rs:85.

## Timestamps
- `Wake` (mod.rs:68-87) carries **no Instant**; `Event` in graft-core has none (grep `Instant` in event.rs: nothing). Only `undrawn_since` Instant at running.rs:173 (taken at apply time, not input time). Reader thread has `last_byte` Instant mux.rs:~245 for escape timeout only.

## Blocking write
- stdout never set non-blocking: only `O_NONBLOCK` in tree is the self-pipe gate.rs:242. So `write_all` in draw blocks on a full pty. *Could not confirm* by experiment (no run).

## DEC 2026
- Every `draw` frame: text + Kitty/iTerm2/Sixel image bytes (`graphics.frame` :445) are inside BEGIN :437 / END :453. Tests: event_loop/tests.rs:647-654.
- NOT inside 2026: clipboard :386, `release` :398 (image deletes at exit), `terminal.clear()` after handover :307, RowClear writes (could not confirm whether inside a draw — row_clear.rs:85 is the backend's own write path, so likely inside when called from draw).

## Replies
- Reader: `route` mux.rs:302-317: `Decoded::Reply` → every `ReplyWaiter` sender; `Decoded::Event` → Outlet → Inputs. `ReplyWaiter::read` mux.rs:213 (recv_timeout). Image replies reach loop via `graphics.drain()` running.rs:150; re-query answers via `Wake::Terminal`/`TerminalFailed`.
- CSI classification csi.rs:28: reply only if params start `?`, final `t`, or intermediates. **`CSI 0 n` is not a reply**: falls to `csi_key` csi.rs:90-99 → `cursor_key(b'n')` None → `Step::Took(None)` — silently dropped, reaches neither key decoder output nor any waiter. (A fence on `CSI 5 n` → `CSI 0 n` would never be seen by a ReplyWaiter.)

## Wheel
- `WHEEL_LINES = 3` update/mouse.rs:18; ScrollUp/Down → `wheel` :81-82 → graph `scroll(lines)` :190-198 (or detail files/body :160-171).
- Applied **per event**, one `update` each; the turn batches them into one draw (`joins_frame` mod.rs:235-247 includes Scroll*). No summing.

## Spec contradictions
- Any assumption that a frame is one `write` call: false (running.rs:437/441/447/453).
- Any assumption that `Wake` carries an input timestamp: false (mod.rs:68-87).
- Any assumption that `CSI 0 n` already routes as a reply: false (csi.rs:28, :97).
