# m0-foundations/03 — Terminal session (setup/teardown) and panic hook

Status: todo
Build with: Sonnet
Blocked by: 01

Why: this IS the checkpoint test's second half — "quitting leaves the terminal clean" — including quitting via panic.

## Read first
- PROMPT.md Technology section (the crossterm feature list) and §6.3 Robustness
- ARCHITECTURE.md ("app owns the wiring"; `term` owns detection/encoding/sanitising, not the session)

## Goal
An RAII terminal session in the `graft` binary crate plus a panic hook that restores the terminal and writes a crash report.

## Scope
In:
- `graft` (app) module, e.g. `terminal.rs`: a `TerminalSession` guard. Setup: raw mode, alternate screen, mouse capture incl. motion events, bracketed paste, focus events, hide cursor. Kitty keyboard protocol push behind a caller-supplied flag (ticket 11 passes it from detected `Capabilities`; default off).
- Teardown in exact reverse order, on `Drop`, idempotent (safe to run twice; safe if setup half-failed).
- Panic hook: restore the terminal FIRST, then write a crash report (panic message, location, backtrace) to `${XDG_STATE_HOME:-~/.local/state}/graft/crash-<timestamp>.log`, then print the report path as one plain line to stderr. Never panic inside the hook.
- Crossterm commands queued to a generic `io::Write` so tests can capture the byte stream.
- Dependencies: `crossterm` (and only what this needs).

Out:
- The event loop and drawing (ticket 09). Synchronized-output brackets per frame (ticket 09).
- Capability detection (ticket 04).
- PTY-based integration tests — fast tier tests the escape byte stream against a buffer, not a real terminal.

## Acceptance
- [ ] Unit test: teardown writes the restore sequences in reverse order of setup (assert on captured bytes).
- [ ] Unit test: double drop / failed-setup drop does not double-emit or panic.
- [ ] Unit test: crash-report writer produces a file containing message, location and backtrace for a synthetic panic payload.
- [ ] No `unwrap`/`expect` outside tests (clippy enforces).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
