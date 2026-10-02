# graft-term — map
One line per public item. Update when a public item changes.

## lib.rs
- `clipboard` (mod) — OSC 52 clipboard requests (ADR-0023).
- `detect` (mod) — terminal capability detection and the rendering tier.
- `input` (mod) — the one reader of stdin: keys to events, replies to waiters.
- `iterm2` (mod) — iTerm2 inline images (OSC 1337): bands re-sent per frame, PNG encoded once per (strip, crop, cells).
- `kitty` (mod) — Kitty graphics protocol: strips to bytes, error replies back (ADR-0012 rules).
- `pace` (mod) — the frame's bracket and fence (ADR-0054, m8 G-pace). Re-exported: `SYNC_BEGIN`/`SYNC_END` (DEC 2026 begin/end bytes), `write_fence` (writes `CSI 5 n`, no flush), `fence_answers` (count of `CSI 0 n` in reply bytes).
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
- `detect` (fn) — probe the terminal and return `Capabilities`; never reads stdin itself. Fills `kitty_deflate` from an `o=z` query (`i=32`) in the same write; replies matched by id. Fills `answers_dsr` from a `CSI 5 n` in the first round, before the DA1 fence (m8 G-pace); no new round.
- `remeasure_cell_size` (fn) — after a resize: `CSI 16 t`, then `TIOCGWINSZ`, else the previous size marked `Assumed`.
- `requery_theme` (fn) — on focus regain: `OSC 11`; silence returns the previous hint unchanged.

## input
- `InputMux` (struct) — owns stdin for the process's life; decodes bytes on its own thread; events go to the current `Inputs`, or are kept for the next one while there is none.
- `InputMux::on_stdin` (fn) — first call starts the process's one stdin reader; every call returns that mux and a fresh `Inputs` (the previous one ends). Never a second reader.
- `InputMux::start` (fn) — start reading any fd (tests use a socket pair).
- `InputMux::inputs` (fn) — open a fresh `Inputs` that takes over; events held while nobody listened come first.
- `InputMux::hold` (fn) — end the current `Inputs`; keep every event for the next `inputs()`. `event_loop::run` calls it when its loop is done.
- `InputMux::pause` (fn) — stop the reader thread touching the fd at all, and return only once it has stopped, so another program (`$EDITOR`, or the shell after a `SIGTSTP`) gets every byte the user types. `hold` is not enough: it only detaches the listener, and the thread keeps reading stdin. Implemented with a self-pipe the reader polls beside the input fd (m3-working-tree-undo-and-stash/05). **Errs** if the reader neither parks nor ends within 250 ms — the caller must then keep the terminal, because a reader still on it would share the user's keystrokes with the editor. It used to answer `Ok` on that timeout, which was right for a reader that had exited and wrong for one that was merely slow (m3/05 review M6).
- `InputMux::resume` (fn) — let the reader go back to the fd. Input typed while paused was never read by Graft, so it is still in the kernel's buffer for whoever owned the terminal.
- `InputMux::waiter` (fn) — open a `ReplyWaiter`; every open waiter gets a copy of each reply.
- `ReplyWaiter` (struct) — terminal replies for one asker, as raw bytes.
- `ReplyWaiter::read` (fn) — copy reply bytes into a buffer, waiting until a deadline; `Ok(0)` when none came.
- `Inputs` (type) — `Receiver<io::Result<Event>>`: key/mouse/paste/focus events in arrival order; an `Err` is the last item.
- `InputMux::answer_fences` (fn) — `(FenceListener) -> FenceAnswers`: replies holding a fence answer (`CSI 0 n`) go to the listener (called on the reader thread with the count), not to waiters; a listener returning `false` is removed (m8 G-pace).
- `FenceListener` (type) — `Box<dyn FnMut(usize) -> bool + Send>`.
- `FenceAnswers` (struct, `#[must_use]`) — the listener's lease: dropping it removes that listener, and fence answers are ordinary replies again.
- `SEQUENCE_TIMEOUT` (const) — 50 ms an unfinished escape sequence waits before it is read as keys (lone `Esc`).

## kitty
- `KittyImages` (struct) — uploads held by the terminal; emits transmit-once, `p=1` replace, `q=1` commands.
- `KittyImages::new` (fn) — track uploads, at most `capacity` unplaced images kept.
- `KittyImages::with_transport` (fn) — send commands over a `Transport`; inside tmux each command and upload chunk is wrapped alone.
- `KittyImages::frame` (fn) — append commands so the screen shows exactly these `Place`s.
- `KittyImages::hide` (fn) — remove every placement, keep data (tier dropped away from pixels).
- `KittyImages::free_all` (fn) — delete every upload with its data; call on exit.
- `KittyImages::forget_uploads` (fn) — after a clear or resize: retransmit everything next frame.
- `KittyImages::answer` (fn) — take in a reply; `ENOENT` forgets that upload so it retransmits; any other error on a deflated upload turns deflating off for good and retransmits it raw.
- `KittyImages::deflating` (fn) — builder: transmit with `o=z` (zlib RGBA). Off by default.
- `KittyImages::is_deflating` (fn) — still deflating; false once an error reply turned it off.
- `KittyImages::uploaded` (fn) — images the terminal holds, as far as Graft knows.
- `Place` (struct) — one image this frame: `first_row`, `frame` (`&Arc<RasterFrame>`), `rows` crop, `cells`, `layer`.
- `Place::layer` (field) — `Layer::Strip` or `Layer::Emphasis`; each layer of a slot is its own upload.
- `Layer` (enum) — `Strip` (no `z=`, default) or `Emphasis` (`z=1`, emitted after every strip, id has `EMPHASIS_ID_BIT`).
- `EMPHASIS_ID_BIT` (const) — `1 << 31`; set in every emphasis image id, in no strip id.
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

## clipboard
- `osc52_copy` (fn) — `ESC ] 52 ; c ; <base64(text)> ESC \`; write it through `Transport::write` so tmux wraps it (m1-the-graph/45).
