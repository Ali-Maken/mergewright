# m1-the-graph/20 — One owner of stdin

Status: ready
Build with: **Opus** (high) — concurrency and terminal protocol parsing
Blocked by: 09
Leaves runnable: yes — keys typed during startup are no longer eaten.

Why: detection reads the fd itself today, so typeahead during the probe is lost, and 21 and 24 both need terminal replies routed while the app runs.

## Read first
- DESIGN.md §4.1 and §4.5
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
- [ ] Under a pty: keys sent during the probe window all arrive afterwards, in order; record the measurement.
- [ ] A reply split across two reads is still routed to its waiter.
- [ ] A lone `Esc` key is delivered as a key after the reply timeout, not swallowed as a reply prefix.
- [ ] M0's termios-restore tests still pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
