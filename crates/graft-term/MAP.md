# graft-term — map
One line per public item. Update when a public item changes.

## lib.rs
- `detect` (mod) — terminal capability detection and the rendering tier.
- `input` (mod) — the one reader of stdin: keys to events, replies to waiters.
- `iterm2` (mod) — iTerm2 inline images (OSC 1337): bands re-sent per frame, PNG encoded once per (strip, crop, cells).
- `kitty` (mod) — Kitty graphics protocol: strips to bytes, error replies back (ADR-0012 rules).
- `sanitize` (mod) — repo-derived text made safe to draw.
- `tmux` (mod) — getting image sequences through tmux: the DCS passthrough wrapper.
- `Transport`, `tmux_passthrough` (re-exports) — see `tmux`.
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

## kitty
- `KittyImages` (struct) — uploads held by the terminal; emits transmit-once, `p=1` replace, `q=1` commands.
- `KittyImages::new` (fn) — track uploads, at most `capacity` unplaced images kept.
- `KittyImages::with_transport` (fn) — send commands over a `Transport`; inside tmux each command and upload chunk is wrapped alone.
- `KittyImages::frame` (fn) — append commands so the screen shows exactly these `Place`s.
- `KittyImages::hide` (fn) — remove every placement, keep data (tier dropped away from pixels).
- `KittyImages::free_all` (fn) — delete every upload with its data; call on exit.
- `KittyImages::forget_uploads` (fn) — after a clear or resize: retransmit everything next frame.
- `KittyImages::answer` (fn) — take in a reply; `ENOENT` forgets that upload so it retransmits.
- `KittyImages::uploaded` (fn) — images the terminal holds, as far as Graft knows.
- `Place` (struct) — one strip this frame: `first_row`, `frame` (`&Arc<RasterFrame>`), `rows` crop, `cells`.
- `CellRect` (struct) — zero-based cell rectangle: `column`, `row`, `columns`, `rows` (0 = crop's rows).
- `GraphicsReply` (struct) — `image`, `placement`, `message` (controls replaced); `is_error`, `is_missing_image`.
- `replies` (fn) — every APC graphics reply in a byte buffer; other replies skipped.

## iterm2
- `ItermImages` (struct) — encoded bands kept for re-sending; erases last frame's cells (`ECH`) before writing.
- `ItermImages::new` (fn) — keep at most `capacity` encoded bands (current frame's always kept).
- `ItermImages::with_transport` (fn) — send each band's OSC 1337 over a `Transport`; cursor moves and erases stay unwrapped.
- `ItermImages::frame` (fn) — erase covered cells, write each `Place` at its cell (`doNotMoveCursor=1`); `io::Error` on a bad crop.
- `ItermImages::hide` (fn) — erase every cell the last frame covered; bands stay encoded.
- `ItermImages::encoded` (fn) — PNG encodes so far (an unchanged re-send does not count).

## tmux
- `tmux_passthrough` (fn) — append one sequence as `ESC P tmux;` + sequence with every `ESC` doubled + `ESC \`.
- `Transport` (enum) — `Direct` or `TmuxPassthrough`; `Transport::for_tmux` (anything but `TmuxStatus::Outside` wraps), `Transport::write`.

## sanitize
- `SanitizedText` (struct) — text proven free of escape and control sequences.
- `sanitize` / `sanitize_line` / `sanitize_path` / `sanitize_path_bytes` (fn) — make untrusted text safe to draw.
