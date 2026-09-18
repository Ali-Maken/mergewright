# m1-the-graph/20 — One owner of stdin

Status: done
Build with: **Opus** (high) — concurrency and terminal protocol parsing
Blocked by: 09
Leaves runnable: yes — keys typed during startup are no longer eaten.

Why: detection reads the fd itself today, so typeahead during the probe is lost, and 21 and 24 both need terminal replies routed while the app runs.

## Read first
- docs/design/04-terminal-capability-detection.md §4.1 and §4.5
- KNOWN_GAPS.md M0 row "The probe eats typeahead"
- Code: `crates/graft-term/src/detect/` (how it reads replies now), the event loop's input source in `crates/graft/src/event_loop/`, M0's pty test harness

## Scope
In:
- A single reader owns stdin for the process's life, classifies bytes, routes terminal replies to whoever is waiting, and turns everything else into key events.
- Detection stops reading the fd; it asks the mux.
- Typeahead pressed during detection arrives afterwards, in order.
- A reply-waiter API that 21 (re-queries) and 24 (graphics replies) can use.

Out:
- SIGWINCH and re-detection (21). Changing the probe sequence or tier table.

## Acceptance
- [x] Under a pty: keys sent during the probe window all arrive afterwards, in order; record the measurement. (Arrival on screen; order by unit test — see Measurement.)
- [x] A reply split across two reads is still routed to its waiter. (`input::mux::tests`, `input::decode::tests` + chunking proptest.)
- [x] A lone `Esc` key is delivered as a key after the reply timeout, not swallowed as a reply prefix. (`SEQUENCE_TIMEOUT` 50 ms; mux test asserts it arrives, and not before.)
- [x] M0's termios-restore tests still pass. (`cargo test`; `.scratch/m0-foundations/pty-signal-check.py`: all three restored.)

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Measurement (2026-09-13, macOS, debug build, pty 80x24 via pyte)

`python3 .scratch/m1-the-graph/pty-typeahead-check.py target/debug/graft . <mode>`. pyte answers no queries, so the probe runs its full 200 ms budget.

| Mode | Keys sent (after DA1 query seen at ~3 ms) | First frame | Result |
|---|---|---|---|
| baseline | none | 210 ms | graph screen |
| probe-f2x2 | F2 at 3, 28 ms | 209 ms | graph screen (2 toggles arrived) |
| probe-f2 | F2 at 3, 24, 52 ms | 215 ms | status screen (odd count arrived); capture `docs/shots/20-typeahead-f2-during-probe.txt` |
| resize | SIGWINCH after widening to 110 cols | — | frame re-laid out 110 wide |

termios identical before launch / after quit in every run. F3 check (`pty-tier-drop-check.py`) still passes.

Not verified on screen: key **order**. The script's selection detector cannot see the cursor row (`after-jjjk` reads like baseline), so `jjjk` during the probe proves nothing. Order is proven by `input::mux::tests::keys_typed_around_a_reply_arrive_in_order_and_the_reply_goes_to_its_waiter` over a socket pair. Logged in KNOWN_GAPS.md.

## Notes

- Replaced `crossterm::event::read` entirely: crossterm has no public byte parser, so `graft_term::input::decode` decodes keys (xterm CSI/SS3, kitty `CSI u`), SGR/X10 mouse, bracketed paste and focus. `event_loop::input::translate` removed.
- crossterm's reader also delivered `Resize`; that now comes from `signals::watch_resizes` (SIGWINCH → `crossterm::terminal::size`). Verified by the `resize` pty mode.
- `poll` moved from `detect/tty.rs` to `input/mux.rs`; ADR-0013 updated (now also takes `-1`, wait forever).
- Did not compact.
