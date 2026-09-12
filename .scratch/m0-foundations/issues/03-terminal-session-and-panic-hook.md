# m0-foundations/03 — Terminal session (setup/teardown) and panic hook

Status: done
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
- [x] Unit test: teardown writes the restore sequences in reverse order of setup (assert on captured bytes).
- [x] Unit test: double drop / failed-setup drop does not double-emit or panic.
- [x] Unit test: crash-report writer produces a file containing message, location and backtrace for a synthetic panic payload.
- [x] No `unwrap`/`expect` outside tests (clippy enforces).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Comments

**Built.**
- `crates/graft/src/terminal.rs` — `TerminalSession` (RAII), `TerminalRestorer` (a clone of the same shared state, for the panic hook), `SessionOptions { kitty_keyboard }`, and a `RawMode` seam (`ProcessRawMode` in production, a fake in tests) because raw mode is a termios call, not bytes on the sink. Setup order: raw → alternate screen → mouse (incl. motion) → bracketed paste → focus → hide cursor → kitty push. Teardown pops each applied step and undoes it, so a half-failed setup undoes exactly what it applied, a second restore emits nothing, and a failing undo does not stop the rest. The lock is `try_lock`, never blocking: a panic *inside* a restore must not deadlock the hook.
- `crates/graft/src/crash.rs` — hook order is restore → write report → one stderr line. `Backtrace::force_capture()` (ignores `RUST_BACKTRACE`). Report at `${XDG_STATE_HOME:-~/.local/state}/graft/crash-<ts>.log`; if it cannot be written, the whole report goes to stderr rather than being lost. Timestamps are computed once per panic and formatted without a date crate.
- `crates/graft/src/lib.rs` + a thin `main.rs` — `app` became a lib with a thin binary so tests and the example drive the same code. Target split inside one package, not a new crate; `ARCHITECTURE.md` updated, dependency-edge test unchanged.
- `crates/graft/examples/terminal_probe.rs` — the manual/pty check: modes `quit`, `panic`, `hang`.

**Verified for real** (pty harness: child's stdin/stdout/stderr = pty slave, `tcgetattr` on the master before / during / after, plus the raw byte stream):
- `quit` (exit 0): termios byte-identical to before. Bytes on the wire = full setup then the exact reverse teardown.
- `panic` (exit 101): termios byte-identical. Restore sequence appears **once** (hook and `Drop` share one state). Then `graft crashed. Report: …/graft/crash-20260912-060948.log`; that file held `graft 0.1.0 panicked`, the time, `location: crates/graft/examples/terminal_probe.rs:53:20`, the message and real backtrace frames despite `RUST_BACKTRACE=0`.
- `hang` + `SIGINT` (exit -2) and `hang` + `SIGTERM` (exit -15): terminal left at `ECHO=false ICANON=false ISIG=false OPOST=false` **after** the process died. Logged in `KNOWN_GAPS.md` (M0) — default signal disposition kills without unwinding, so neither hook nor `Drop` runs. Not Ctrl+C at the keyboard (raw mode delivers that as a key event).
- Could **not** verify: the mid-run "during" snapshot for `quit`/`panic` raced with process exit and read cooked mode. Raw-mode-on is proven instead by the two `hang` runs, which take the same setup path.

**Code review (self, before commit)** found no critical/high/medium defects. Three low ones, all fixed:
- Two panics in the same second overwrote each other's report (`fs::write`). Now `File::create_new` with a `-1`, `-2` … suffix, so a second report never clobbers the first; if no name is free the error propagates and the whole report goes to stderr.
- `crossterm` pulled its `derive-more` default (three crates of `Display` derives this code never uses). Now `default-features = false` with `events`, `bracketed-paste`, `windows`.
- A step whose write fails part-way through is never undone. Left as is, with a comment saying why: a sink that fails mid-sequence is a broken fd, so the undo write would fail too.

Re-ran the pty harness after those changes — same result: quit and panic leave termios identical, teardown exact and once.

Tests: 16 lib tests (8 terminal, 8 crash incl. `Timestamp`), all passing; the dependency-edge integration test still passes.
