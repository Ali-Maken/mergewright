# m0-foundations/09 — Event loop, `AppState`, `update`, `view`

Status: todo
Build with: Sonnet
Blocked by: 03

Why: the Event → update → (state, [Command]) → view spine from ARCHITECTURE.md; every later feature is a new arm of `update`.

## Read first
- ARCHITECTURE.md ("The shape") and DESIGN.md §8.3
- DESIGN.md §8.2 (`Event`, `JobId` live in `graft-core`; `Command` lives in `app` — deliberately, see ARCHITECTURE.md)
- DESIGN.md §2.2 for the global quit binding
- ADR-0012 "What didn't" (degenerate window sizes)

## Goal
`graft` (app): a running event loop drawing a placeholder frame with ratatui, quitting cleanly.

## Scope
In:
- `graft-core`: `Event` (key, mouse, resize, paste, focus gained/lost, job-result variant) and `JobId`. Model job results the minimal way DESIGN.md §8.2 describes; ticket 10 fills in the runner.
- `graft` (app): `AppState` (minimal: should_quit, a placeholder status field), `update(&AppState, Event) -> (AppState, Vec<Command>)` — pure, no I/O, no clock; `Command` enum (can be empty or a `Noop` for now); `view(&AppState, frame)` rendering a placeholder line.
- The loop: crossterm events → `core::Event` → `update` → draw. Wrap each draw in synchronized-output begin/end (DEC 2026, emitted unconditionally — DESIGN.md §4.1).
- Quit on the DESIGN.md §2.2 global quit key and Ctrl+C; loop exits, `TerminalSession` (ticket 03) restores.
- **Signals (moved here from ticket 03, 2026-09-12).** Register handlers for `SIGINT`, `SIGTERM` and `SIGHUP` that only set a flag / wake the loop. The loop must then return **normally** so `TerminalSession`'s `Drop` restores the terminal — never restore from inside the handler, and never `exit()` from it. Default disposition kills without unwinding, which is exactly the measured gap: signal-killed runs leave `ECHO/ICANON/ISIG=false`. Closes the KNOWN_GAPS.md M0 entry from ticket 03.
- Resize handled; 0×0 or tiny sizes clamped, never a panic (ADR-0012).
- Binary runs: `cargo run` shows the placeholder, quit key exits clean. Repo facts arrive in ticket 11.
- Dependency: `ratatui`.

Out:
- The job runner threads (ticket 10) — `Command`s can be discarded for now.
- Theme wiring, capability detection, repo (ticket 11).
- Tick timers, animation, file watching.

## Acceptance
- [ ] `update` unit tests: quit event sets should_quit; unknown events are no-ops; `update` provably does no I/O (takes/returns plain data only).
- [ ] `view` snapshot via ratatui `TestBackend` + `insta`, incl. a 0×0 and a 2×2 terminal (no panic).
- [ ] Under a pty: send `SIGINT`, then `SIGTERM`, then `SIGHUP` to a running instance; termios after the process exits is byte-identical to before launch (same check ticket 03 used). Not a green-gate claim — record the measured before/after.
- [ ] Manual: `cargo run` in a real terminal — draws, quits clean, no stray characters (record the check in the ticket's Comments).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
