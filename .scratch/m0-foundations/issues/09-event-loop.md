# m0-foundations/09 — Event loop, `AppState`, `update`, `view`

Status: done
Build with: Opus (routing changed 2026-09-12)
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
- [x] `update` unit tests: quit event sets should_quit; unknown events are no-ops; `update` provably does no I/O (takes/returns plain data only).
- [x] `view` snapshot via ratatui `TestBackend` + `insta`, incl. a 0×0 and a 2×2 terminal (no panic).
- [x] Under a pty: send `SIGINT`, then `SIGTERM`, then `SIGHUP` to a running instance; termios after the process exits is byte-identical to before launch (same check ticket 03 used). Not a green-gate claim — record the measured before/after.
- [ ] Manual: `cargo run` in a real terminal — draws, quits clean, no stray characters. **Open: needs the user's eyes at the M0 checkpoint.** A pty capture stands in for it for now (see Comments); it proves the bytes, not the picture.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Comments

### Signals, measured under a pty (2026-09-12)

Harness: `.scratch/m0-foundations/pty-signal-check.py` (committed, so the M0
review can re-run it). It opens a pty, records the slave's full `termios`
struct, forks `target/debug/graft` onto it in its own session, waits for the
first frame, signals it, waits for it to die, and records `termios` again.

```
python3 .scratch/m0-foundations/pty-signal-check.py target/debug/graft
```

Same three lines for **SIGINT**, **SIGTERM** and **SIGHUP**:

| | ECHO | ICANON | ISIG | IEXTEN | OPOST | IXON | ICRNL |
|---|---|---|---|---|---|---|---|
| before launch | on | on | on | on | on | on | on |
| while running | off | off | off | off | off | off | off |
| after the signal | on | on | on | on | on | on | on |

- Whole `termios` struct (all four flag words, both speeds, every `cc` slot)
  byte-identical before and after: **true, for all three**.
- Raw mode really was entered first, so this is not a no-op passing by
  accident: the "while running" row differs from "before".
- Cause of death: signal 2 / 15 / 1 — the process is still *killed by the
  signal*, so `$?` stays 128 + N. `main` re-raises with the default
  disposition after the session is restored.

Before this ticket the same harness showed `ECHO=false ICANON=false
ISIG=false` after the signal (ticket 03's measurement); the KNOWN_GAPS.md M0
row is now removed.

### The run itself, captured from a pty

`.scratch` note: a second throwaway script drove the binary on a 40×10 pty,
sent `q`, and read back everything written. Exit status 0, `termios`
identical, and the byte stream is:

```
ESC[?1049h  ESC[?1000h ESC[?1002h ESC[?1003h ESC[?1015h ESC[?1006h
ESC[?2004h  ESC[?1004h ESC[?25l
ESC[?2026h  "Graft — press q to quit" at 1;1 … "ready" at 10;1  ESC[?2026l
ESC[?25h    ESC[?1004l ESC[?2004l ESC[?1006l ESC[?1015l ESC[?1003l
ESC[?1002l  ESC[?1000l ESC[?1049l
```

Alt screen in, every mode Graft turned on turned back off in reverse, alt
screen out. Exactly one frame for one keypress (no polling), wrapped in DEC
2026 begin/end, and **nothing after `ESC[?1049l`** — no stray characters.

### What is *not* verified

Nobody has looked at this in a real terminal. A pty capture proves the bytes
were emitted, not that they render (the same mistake ADR-0012 corrected).
`cargo run` in Ghostty / iTerm2 / the Zed terminal, judged by eye, is for the
user at the M0 checkpoint — the last acceptance box stays unchecked until
then.

### Out-of-ticket changes

- `Cargo.toml`: `rust-version` 1.85 → **1.88**. ratatui 0.30 needs it; on 1.85
  the resolver picks an older crossterm and links two versions at once.
- Clippy lints are MSRV-gated, so that bump switched on two new lints and
  broke `-D warnings` in files this ticket does not own. Fixed in place:
  `graft-core/src/oid.rs` (`manual_is_multiple_of`, `chunks_exact_to_as_chunks`)
  and `graft-term/src/detect/reply.rs` (`manual_is_multiple_of`).
