# graft-term — map
Terminal capability detection, image-protocol encoding, escape-sequence sanitising.
May depend on: `core` (ARCHITECTURE.md crate table). One line per public item.

## lib.rs
- `clipboard` (mod) — OSC 52 clipboard requests.
- `detect` (mod) — terminal capability detection and the rendering tier.
- `input` (mod) — the one reader of stdin: keys to events, replies to waiters.
- `iterm2` (mod) — iTerm2 inline images (OSC 1337).
- `kitty` (mod) — Kitty graphics protocol: strips to bytes, error replies back.
- `pace` (mod) — the frame's synchronized-output bracket and DSR fence; re-exported at the root.
- `sanitize` (mod) — repo-derived text made safe to draw; re-exported at the root.
- `tmux` (mod) — DCS passthrough for image sequences inside tmux; re-exported at the root.

## clipboard
- `osc52_copy` (fn) — `(&str) -> Vec<u8>`: `ESC ] 52 ; c ; <base64> ESC \`; write it through `Transport::write`.

## detect
- `LOCAL_PROBE_BUDGET` (const) — `Duration`, probe deadline on a local terminal.
- `SSH_PROBE_BUDGET` (const) — `Duration`, probe deadline over SSH.
- `ASSUMED_CELL_SIZE` (const) — `(u32, u32)`, cell pixels used when nothing reports one.
- `WindowPixels` (struct) — `TIOCGWINSZ`'s answer: `columns`, `rows`, `width`, `height`.
- `TerminalHost` (trait) — what detection talks to: `write_probe`, `read` until a deadline, `window_pixels`, `tmux_version`.
- `detect` (fn) — `(&Environment, &mut dyn TerminalHost) -> Capabilities`: probe the terminal; never reads stdin itself.
- `remeasure_cell_size` (fn) — after a resize: `CSI 16 t`, then `TIOCGWINSZ`, else the previous size marked assumed.
- `requery_theme` (fn) — on focus regain: `OSC 11`; silence keeps the previous hint.
- `TtyHost` (struct) — the real terminal: writes stdout, reads replies through a `ReplyWaiter`.
- `TtyHost::new` (fn) — `(ReplyWaiter) -> Self`: a host reading replies from the input mux.
- `TtyHost::unread` (fn) — a host that is never read (stdout not a tty); reads report nothing.
- `Environment` (struct) — environment hints for detection.
- `Environment::from_process` (fn) — read the hints from this process's environment.
- `Environment::probe_budget` (fn) — `-> Duration`: the probe deadline, local or SSH.
- `Environment::is_probeable` (fn) — `-> bool`: whether queries may be written at all (a tty, not `TERM=dumb`).

## input
- `InputMux` (struct) — owns stdin for the process's life; decodes on its own thread, routes events and replies.
- `InputMux::on_stdin` (fn) — `-> io::Result<(Self, Inputs)>`: the process's one stdin reader and a fresh `Inputs`.
- `InputMux::start` (fn) — `(OwnedFd) -> io::Result<(Self, Inputs)>`: read any fd.
- `InputMux::pause` (fn) — `-> io::Result<()>`: stop the reader touching the fd before handing the terminal away; errs if it does not park.
- `InputMux::resume` (fn) — let the reader go back to the fd.
- `InputMux::inputs` (fn) — `-> Inputs`: a fresh receiver that takes over; held events come first.
- `InputMux::hold` (fn) — end the current `Inputs`; keep events for the next one.
- `InputMux::give_back` (fn) — `(impl IntoIterator<Item = Event>)`: put events back at the front for the next reader.
- `InputMux::waiter` (fn) — `-> ReplyWaiter`: every open waiter gets a copy of each reply.
- `InputMux::answer_fences` (fn) — `(FenceListener) -> FenceAnswers`: fence answers go to the listener, not to waiters.
- `ReplyWaiter` (struct) — terminal replies for one asker, as raw bytes.
- `ReplyWaiter::read` (fn) — `(&mut [u8], Instant) -> io::Result<usize>`: reply bytes until a deadline; `Ok(0)` when none came.
- `Inputs` (type) — `Receiver<io::Result<Event>>`: events in arrival order; an `Err` is the last item.
- `FenceListener` (type) — `Box<dyn FnMut(usize) -> bool + Send>`; returning `false` removes it.
- `FenceAnswers` (struct) — the listener's lease; dropping it removes the listener.
- `SEQUENCE_TIMEOUT` (const) — `Duration` an unfinished escape sequence waits before it is read as keys.

## iterm2
- `ItermImages` (struct) — encoded bands kept for re-sending; erases last frame's cells before writing.
- `ItermImages::new` (fn) — `(capacity: usize)`: keep at most that many encoded bands.
- `ItermImages::with_transport` (fn) — builder: send each band over a `Transport`.
- `ItermImages::encoded` (fn) — `-> u64`: PNG encodes so far.
- `ItermImages::frame` (fn) — `(&[Place], &mut Vec<u8>) -> io::Result<()>`: erase covered cells, write each place.
- `ItermImages::hide` (fn) — erase every cell the last frame covered.

## kitty
- `EMPHASIS_ID_BIT` (const) — `u32` bit set in every emphasis image id, in no strip id.
- `Layer` (enum) — `Strip` or `Emphasis` (`z` ≥ 1, over strips). Stacking by `z`: "If two images with the same z-index overlap then the image with the lower id is considered to have the lower z-index" (Kitty graphics-protocol doc).
- `CellRect` (struct) — zero-based cell rectangle: `column`, `row`, `columns`, `rows`.
- `UploadKey` (struct) — `(pub u64)`: what an upload is known by; same key, same picture, no retransmit.
- `Place` (struct) — one image this frame: `first_row`, `key`, `frame` (`None` only if `holds` said yes), `rows`, `cells`, `layer`, `stack` (rank, higher on top; emphasis sent as `z = 1 + stack`; strips 0). In `kitty/place.rs`.
- `GraphicsReply` (struct) — one APC graphics reply: `image`, `placement`, `message`.
- `GraphicsReply::is_error` (fn) — `-> bool`: the reply is not `OK`.
- `GraphicsReply::is_missing_image` (fn) — `-> bool`: the terminal lost the image (`ENOENT`).
- `replies` (fn) — `(&[u8]) -> Vec<GraphicsReply>`: every graphics reply in a byte buffer.
- `KittyImages` (struct) — uploads the terminal holds; emits transmit-once, replace and quiet commands.
- `KittyImages::new` (fn) — `(unplaced_bytes: usize)`: decoded bytes unplaced images may hold; least recently placed freed first.
- `KittyImages::holds` (fn) — `(UploadKey) -> bool`: the terminal holds that image, so a place may come without pixels.
- `KittyImages::unplaced` (fn) — `-> usize`: decoded bytes of the uploads no frame places now.
- `KittyImages::deflating` (fn) — builder `(bool)`: transmit zlib-compressed RGBA.
- `KittyImages::is_deflating` (fn) — `-> bool`: still deflating; false once an error turned it off.
- `KittyImages::with_transport` (fn) — builder: send commands over a `Transport`.
- `KittyImages::uploaded` (fn) — `-> usize`: images the terminal holds, as far as Graft knows.
- `KittyImages::frame` (fn) — `(&[Place], &mut Vec<u8>)`: commands so the screen shows exactly these places; a key placed `n` times gets ids `p=1..=n`.
- `KittyImages::hide` (fn) — remove every placement, keep data.
- `KittyImages::free_all` (fn) — delete every upload with its data; call on exit.
- `KittyImages::forget_uploads` (fn) — after a clear or resize: retransmit everything next frame.
- `KittyImages::answer` (fn) — `(&GraphicsReply)`: a missing image retransmits; an error on a deflated upload turns deflating off.

## pace
- `SYNC_BEGIN` (const) — `&[u8]`: DEC 2026 synchronized-output begin.
- `SYNC_END` (const) — `&[u8]`: DEC 2026 synchronized-output end.
- `write_fence` (fn) — `(&mut impl Write) -> io::Result<()>`: write `CSI 5 n`, no flush.
- `fence_answers` (fn) — `(&[u8]) -> usize`: count of `CSI 0 n` in reply bytes.

## sanitize
- `SanitizedText` (struct) — text proven free of escape and control sequences.
- `SanitizedText::as_str` (fn) — `-> &str`.
- `sanitize` (fn) — `(&str) -> SanitizedText`: multi-line text made safe to draw.
- `sanitize_line` (fn) — `(&str) -> SanitizedText`: one line; newlines made visible.
- `sanitize_path` (fn) — `(&Path) -> SanitizedText`: a path made safe to draw.
- `sanitize_path_bytes` (fn) — `(&[u8]) -> SanitizedText`: raw path bytes made safe to draw.

## tmux
- `Transport` (enum) — `Direct` or `TmuxPassthrough`: how a sequence reaches the outer terminal.
- `Transport::for_tmux` (fn) — `(&TmuxStatus) -> Self`: wrap unless outside tmux.
- `Transport::write` (fn) — `(self, &[u8], &mut Vec<u8>)`: append the sequence, wrapped if needed.
- `tmux_passthrough` (fn) — `(&[u8], &mut Vec<u8>)`: `ESC P tmux;` + sequence with `ESC` doubled + `ESC \`.
