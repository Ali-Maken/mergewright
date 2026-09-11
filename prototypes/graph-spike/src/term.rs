//! Terminal probing, raw key reads and image-protocol emission.
//!
//! Raw escape sequences on purpose: exercising the Kitty graphics protocol
//! and iTerm2 inline images directly is the whole point of the spike.

use std::env;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Iterm2,
}

impl Protocol {
    pub fn name(self) -> &'static str {
        match self {
            Protocol::Kitty => "kitty",
            Protocol::Iterm2 => "iterm2",
        }
    }
}

pub struct CellSize {
    /// Cell width in pixels.
    pub w: u32,
    /// Cell height in pixels.
    pub h: u32,
    /// False when no query answered and 10x20 was assumed (DESIGN.md §4.1
    /// wants approximate values flagged, never silently treated as exact).
    pub reported: bool,
}

/// Decide the image protocol. Env override first (`GRAPH_SPIKE_PROTOCOL`,
/// the spike's stand-in for `graph.renderer`), then a real Kitty graphics
/// query (answered by Ghostty and Kitty), then the iTerm2 env hint — the
/// one tier DESIGN.md §4.1 allows to trust the environment alone.
pub fn detect_protocol() -> Option<Protocol> {
    match env::var("GRAPH_SPIKE_PROTOCOL").as_deref() {
        Ok("kitty") => return Some(Protocol::Kitty),
        Ok("iterm2") => return Some(Protocol::Iterm2),
        _ => {}
    }
    if kitty_acks_graphics_query() {
        return Some(Protocol::Kitty);
    }
    if env::var("TERM_PROGRAM").as_deref() == Ok("iTerm.app") {
        return Some(Protocol::Iterm2);
    }
    None
}

/// Send a tiny Kitty graphics query followed by DA1. Every terminal answers
/// DA1; only Kitty-protocol terminals answer the APC query first, so an APC
/// reply arriving before the DA1 fence means support.
fn kitty_acks_graphics_query() -> bool {
    let mut out = io::stdout();
    // i=31: arbitrary id. A 1x1 RGB pixel as direct payload ("AAAA" = 3 bytes).
    let ok = out
        .write_all(b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c")
        .and_then(|()| out.flush())
        .is_ok();
    if !ok {
        return false;
    }
    let reply = read_reply(Duration::from_millis(300), da1_complete);
    reply.windows(3).any(|w| w == b"\x1b_G")
}

/// True once the DA1 answer (`ESC [ ? ... c`) has fully arrived.
fn da1_complete(buf: &[u8]) -> bool {
    match buf.windows(3).position(|w| w == b"\x1b[?") {
        Some(start) => buf[start + 3..].contains(&b'c'),
        None => false,
    }
}

/// Cell size in pixels: ask the terminal (`CSI 16 t`), fall back to the
/// kernel's winsize pixel fields, then assume 10x20 and flag it.
pub fn detect_cell_size() -> CellSize {
    let mut out = io::stdout();
    let ok = out
        .write_all(b"\x1b[16t")
        .and_then(|()| out.flush())
        .is_ok();
    if ok {
        let reply = read_reply(Duration::from_millis(300), |b| {
            reply_ended(b, b"\x1b[6;", b't')
        });
        if let Some((h, w)) = parse_cell_reply(&reply) {
            return CellSize {
                w,
                h,
                reported: true,
            };
        }
    }
    if let Ok(ws) = crossterm::terminal::window_size() {
        if ws.width > 0 && ws.height > 0 && ws.columns > 0 && ws.rows > 0 {
            return CellSize {
                w: u32::from(ws.width / ws.columns),
                h: u32::from(ws.height / ws.rows),
                reported: true,
            };
        }
    }
    CellSize {
        w: 10,
        h: 20,
        reported: false,
    }
}

/// True when `buf` contains `prefix` followed (eventually) by `end`.
fn reply_ended(buf: &[u8], prefix: &[u8], end: u8) -> bool {
    match buf.windows(prefix.len()).position(|w| w == prefix) {
        Some(i) => buf[i + prefix.len()..].contains(&end),
        None => false,
    }
}

/// Parse `ESC [ 6 ; height ; width t` (height first!).
fn parse_cell_reply(buf: &[u8]) -> Option<(u32, u32)> {
    let start = buf.windows(4).position(|w| w == b"\x1b[6;")?;
    let rest = &buf[start + 4..];
    let end = rest.iter().position(|&b| b == b't')?;
    let body = std::str::from_utf8(&rest[..end]).ok()?;
    let mut parts = body.split(';');
    let h: u32 = parts.next()?.parse().ok()?;
    let w: u32 = parts.next()?.parse().ok()?;
    if h == 0 || w == 0 {
        return None;
    }
    Some((h, w))
}

/// Read raw bytes from the tty until `done` says so or the deadline passes.
/// Direct `poll` + `read` on stdin: the probes run before anything else
/// reads the descriptor, so nothing buffers ahead of us.
fn read_reply(timeout: Duration, done: impl Fn(&[u8]) -> bool) -> Vec<u8> {
    let deadline = Instant::now() + timeout;
    let mut buf = Vec::new();
    while !done(&buf) {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let mut fd = libc::pollfd {
            fd: 0,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fd, 1, left.as_millis() as i32) };
        if ready <= 0 {
            break;
        }
        let mut chunk = [0u8; 256];
        let n = unsafe { libc::read(0, chunk.as_mut_ptr().cast(), chunk.len()) };
        if n <= 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n as usize]);
    }
    buf
}

pub enum Key {
    Up,
    Down,
    Quit,
    Other,
}

/// Block for one key. `j`/`k` and the arrow keys scroll; `q`, a lone `Esc`
/// or `Ctrl-C` quit.
pub fn read_key() -> Key {
    let mut b = [0u8; 1];
    let n = unsafe { libc::read(0, b.as_mut_ptr().cast(), 1) };
    if n <= 0 {
        return Key::Quit;
    }
    match b[0] {
        b'q' | 3 => Key::Quit,
        b'j' => Key::Down,
        b'k' => Key::Up,
        0x1b => {
            let seq = read_reply(Duration::from_millis(20), |s| s.len() >= 2);
            match (seq.first(), seq.get(1)) {
                (Some(b'['), Some(b'A')) => Key::Up,
                (Some(b'['), Some(b'B')) => Key::Down,
                (None, _) => Key::Quit,
                _ => Key::Other,
            }
        }
        _ => Key::Other,
    }
}

/// Transmit the full graph strip once as Kitty image 1 (raw RGBA, base64,
/// chunked at 4096 bytes, no display). Frames then place crops of it.
pub fn kitty_transmit(out: &mut impl Write, rgba: &[u8], w: u32, h: u32) -> io::Result<()> {
    let b64 = B64.encode(rgba);
    let mut chunks = b64.as_bytes().chunks(4096).enumerate().peekable();
    while let Some((i, chunk)) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        if i == 0 {
            write!(out, "\x1b_Ga=t,f=32,i=1,s={w},v={h},q=2,m={more};")?;
        } else {
            write!(out, "\x1b_Gm={more};")?;
        }
        out.write_all(chunk)?;
        out.write_all(b"\x1b\\")?;
    }
    Ok(())
}

/// Place a crop of image 1 at the cursor, scaled into `cols` x `rows` cells.
/// The crop's pixel size matches that cell rect exactly, so no scaling
/// actually happens — that exactness is what the spike is checking.
pub fn kitty_place(
    out: &mut impl Write,
    src_y: u32,
    src_w: u32,
    src_h: u32,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    write!(
        out,
        "\x1b_Ga=p,i=1,x=0,y={src_y},w={src_w},h={src_h},c={cols},r={rows},q=2\x1b\\"
    )
}

/// Remove image 1's on-screen placements, keeping the transmitted data.
pub fn kitty_clear_placements(out: &mut impl Write) -> io::Result<()> {
    write!(out, "\x1b_Ga=d,d=i,i=1,q=2\x1b\\")
}

/// Delete every image and free its data (used on exit).
pub fn kitty_free_all(out: &mut impl Write) -> io::Result<()> {
    write!(out, "\x1b_Ga=d,d=A,q=2\x1b\\")
}

/// iTerm2 inline image at the cursor: a PNG scaled into `cols` x `rows`
/// cells. No image ids and no crops exist here — the visible band re-sends
/// every frame.
pub fn iterm2_inline(out: &mut impl Write, png: &[u8], cols: u16, rows: u16) -> io::Result<()> {
    write!(
        out,
        "\x1b]1337;File=inline=1;size={};width={cols};height={rows};preserveAspectRatio=0:{}\x07",
        png.len(),
        B64.encode(png)
    )
}
