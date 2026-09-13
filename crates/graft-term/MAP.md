# graft-term — map
One line per public item. Update when a public item changes.

## lib.rs
- `detect` (mod) — terminal capability detection and the rendering tier.
- `input` (mod) — the one reader of stdin: keys to events, replies to waiters.
- `sanitize` (mod) — repo-derived text made safe to draw.
- `SanitizedText`, `sanitize`, `sanitize_line`, `sanitize_path`, `sanitize_path_bytes` (re-exports) — see `sanitize`.

## detect
- `LOCAL_PROBE_BUDGET` (const) — probe deadline on a local terminal, 200 ms.
- `SSH_PROBE_BUDGET` (const) — probe deadline over SSH, 500 ms.
- `ASSUMED_CELL_SIZE` (const) — 8×16 px, used when nothing reports a cell size.
- `WindowPixels` (struct) — `TIOCGWINSZ`'s answer: `columns`, `rows`, `width`, `height`.
- `TerminalHost` (trait) — the terminal detection talks to: `write_probe`, `read` (until a deadline), `window_pixels`, `tmux_version`.
- `TtyHost` (struct) — the real terminal: writes to stdout, reads replies through a `ReplyWaiter`.
- `TtyHost::new` (fn) — a host reading replies from the input mux's waiter.
- `TtyHost::unread` (fn) — a host that is never read (stdout not a tty, so no mux); reads report nothing.
- `Environment` (struct) — environment hints; `Environment::from_process`.
- `Environment::is_probeable` (fn) — whether queries may be written at all (a tty, not `TERM=dumb`).
- `Environment::probe_budget` (fn) — the probe deadline for this terminal (local or SSH).
- `detect` (fn) — probe the terminal and return `Capabilities`; never reads stdin itself.
- `remeasure_cell_size` (fn) — after a resize: `CSI 16 t`, then `TIOCGWINSZ`, else the previous size marked `Assumed`.
- `requery_theme` (fn) — on focus regain: `OSC 11`; silence returns the previous hint unchanged.

## input
- `InputMux` (struct) — owns stdin for the process's life; decodes bytes on its own thread.
- `InputMux::on_stdin` (fn) — start reading a duplicate of stdin; returns the mux and its `Inputs`.
- `InputMux::start` (fn) — start reading any fd (tests use a socket pair).
- `InputMux::waiter` (fn) — open a `ReplyWaiter`; every open waiter gets a copy of each reply.
- `ReplyWaiter` (struct) — terminal replies for one asker, as raw bytes.
- `ReplyWaiter::read` (fn) — copy reply bytes into a buffer, waiting until a deadline; `Ok(0)` when none came.
- `Inputs` (type) — `Receiver<io::Result<Event>>`: key/mouse/paste/focus events in arrival order; an `Err` is the last item.
- `SEQUENCE_TIMEOUT` (const) — 50 ms an unfinished escape sequence waits before it is read as keys (lone `Esc`).

## sanitize
- `SanitizedText` (struct) — text proven free of escape and control sequences.
- `sanitize` / `sanitize_line` / `sanitize_path` / `sanitize_path_bytes` (fn) — make untrusted text safe to draw.
