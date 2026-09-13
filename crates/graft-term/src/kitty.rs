//! The Kitty graphics protocol: rasterised strips to bytes, and the
//! terminal's error replies back.
//!
//! Every rule here was learnt the hard way in a real Ghostty (ADR-0012 and
//! its Correction), and each is pinned by a byte-stream test:
//!
//! - a strip is **transmitted once** under an image id, then each frame only
//!   **places a crop** of it;
//! - each strip has **one placement id, `p=1`, replaced** every frame, never
//!   deleted and placed again;
//! - nothing here ever writes `ESC[2J`: Ghostty frees image *data* on a
//!   clear, so every later placement fails. After a clear the caller says so
//!   with [`KittyImages::forget_uploads`];
//! - every command runs at **`q=1`**: silent on success, errors reported.
//!   `q=2` hid the spike's whole failure. The replies come back through the
//!   input mux; [`replies`] parses them and [`KittyImages::answer`] turns an
//!   `ENOENT` into a retransmit instead of trusting that uploads survive;
//! - placement names a **cell rectangle** (`c`, `r`), so the terminal scales
//!   the strip into exactly those cells and text alignment never depends on
//!   pixel-exact cell-size arithmetic.
//!
//! The encoder is pure: bytes go into a caller's buffer, which the caller
//! writes under the same stdout lock as the text frame.

use std::io::Write as _;
use std::ops::Range;
use std::sync::Arc;

use graft_core::raster::RasterFrame;

use crate::tmux::Transport;

/// Image ids start here, clear of the ids detection's probe uses (`i=31`).
const FIRST_IMAGE_ID: u32 = 1000;
/// Base64 bytes per transmit chunk, the protocol's limit.
const CHUNK: usize = 4096;

/// A rectangle of character cells, zero-based from the screen's top left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    /// Leftmost column.
    pub column: u16,
    /// Top row.
    pub row: u16,
    /// Width in cells.
    pub columns: u16,
    /// Height in cells. `0` means "as many as the crop has rows".
    pub rows: u16,
}

/// One strip to show this frame: which rows of it, and in which cells.
#[derive(Clone, Debug)]
pub struct Place<'a> {
    /// The history row the strip starts on; identifies the strip's slot.
    pub first_row: usize,
    /// The rasterised strip. A different `Arc` for the same slot means the
    /// pixels changed and the strip is transmitted again.
    pub frame: &'a Arc<RasterFrame>,
    /// Rows of the strip to show, counted from the strip's own first row.
    pub rows: Range<usize>,
    /// Where on screen those rows go.
    pub cells: CellRect,
}

/// One APC graphics reply: `ESC _ G i=…,p=…;message ESC \`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsReply {
    /// The image id the reply is about, if it named one.
    pub image: Option<u32>,
    /// The placement id, if it named one.
    pub placement: Option<u32>,
    /// `OK`, or an error such as `ENOENT:image not found`. Control
    /// characters are already replaced with U+FFFD.
    pub message: String,
}

impl GraphicsReply {
    /// True unless the terminal said `OK`.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.message != "OK"
    }

    /// True for `ENOENT`: the terminal no longer has the image.
    #[must_use]
    pub fn is_missing_image(&self) -> bool {
        self.message.starts_with("ENOENT")
    }
}

/// An image the terminal holds for us.
#[derive(Debug)]
struct Upload {
    id: u32,
    first_row: usize,
    frame: Arc<RasterFrame>,
    placed: bool,
    last_used: u64,
}

/// What Graft has uploaded to a Kitty-protocol terminal, and the commands
/// that keep the screen matching the strips a frame wants.
#[derive(Debug)]
pub struct KittyImages {
    uploads: Vec<Upload>,
    capacity: usize,
    next_id: u32,
    clock: u64,
    transport: Transport,
}

impl KittyImages {
    /// Track uploads, keeping at most `capacity` images in the terminal.
    /// Placed strips are never evicted, so a frame showing more strips than
    /// that keeps them all.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            uploads: Vec::new(),
            capacity: capacity.max(1),
            next_id: FIRST_IMAGE_ID,
            clock: 0,
            transport: Transport::Direct,
        }
    }

    /// Send every command over `transport`: inside tmux, each `APC` command
    /// (each upload chunk included) gets its own passthrough wrapper, and the
    /// cursor moves around a placement stay outside it for tmux to see.
    #[must_use]
    pub fn with_transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    /// Images currently held by the terminal, as far as Graft knows.
    #[must_use]
    pub fn uploaded(&self) -> usize {
        self.uploads.len()
    }

    /// Append the commands that make the screen show exactly `places`:
    /// transmit what the terminal lacks, replace each strip's placement,
    /// remove placements of strips no longer shown, evict old uploads.
    pub fn frame(&mut self, places: &[Place<'_>], out: &mut Vec<u8>) {
        self.clock += 1;
        let mut shown = Vec::with_capacity(places.len());
        for place in places {
            let id = self.ensure_uploaded(place, out);
            shown.push(id);
            put_placement(self.transport, id, place, out);
        }
        for upload in &mut self.uploads {
            if upload.placed && !shown.contains(&upload.id) {
                delete_placement(self.transport, upload.id, out);
            }
            upload.placed = shown.contains(&upload.id);
        }
        self.evict(out);
    }

    /// Remove every placement, keeping the data (the tier dropped away from
    /// pixels, or the graph is not on screen).
    pub fn hide(&mut self, out: &mut Vec<u8>) {
        for upload in self.uploads.iter_mut().filter(|upload| upload.placed) {
            delete_placement(self.transport, upload.id, out);
            upload.placed = false;
        }
    }

    /// Delete every upload with its data. Call on exit.
    pub fn free_all(&mut self, out: &mut Vec<u8>) {
        for upload in self.uploads.drain(..) {
            free(self.transport, upload.id, out);
        }
    }

    /// The screen was cleared or the terminal resized: assume the terminal
    /// dropped every image (Ghostty does, on `ESC[2J`), so each strip is
    /// transmitted again on the next frame.
    pub fn forget_uploads(&mut self) {
        self.uploads.clear();
    }

    /// Take in one reply. An `ENOENT` about an image Graft uploaded forgets
    /// that upload, so the next frame transmits the strip again.
    pub fn answer(&mut self, reply: &GraphicsReply) {
        if let (true, Some(id)) = (reply.is_missing_image(), reply.image) {
            self.uploads.retain(|upload| upload.id != id);
        }
    }

    /// The id holding `place`'s pixels, transmitting them first if needed.
    fn ensure_uploaded(&mut self, place: &Place<'_>, out: &mut Vec<u8>) -> u32 {
        let clock = self.clock;
        if let Some(index) = self
            .uploads
            .iter()
            .position(|u| u.first_row == place.first_row)
        {
            if Arc::ptr_eq(&self.uploads[index].frame, place.frame) {
                self.uploads[index].last_used = clock;
                return self.uploads[index].id;
            }
            // Same slot, new pixels: a fresh id, so a late reply about the
            // old image can never be mistaken for one about the new.
            free(self.transport, self.uploads.remove(index).id, out);
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).unwrap_or(FIRST_IMAGE_ID);
        transmit(self.transport, id, place.frame, out);
        self.uploads.push(Upload {
            id,
            first_row: place.first_row,
            frame: Arc::clone(place.frame),
            placed: false,
            last_used: clock,
        });
        id
    }

    /// Free least-recently-used unplaced uploads past capacity.
    fn evict(&mut self, out: &mut Vec<u8>) {
        while self.uploads.len() > self.capacity {
            let Some(index) = self
                .uploads
                .iter()
                .enumerate()
                .filter(|(_, upload)| !upload.placed)
                .min_by_key(|(_, upload)| upload.last_used)
                .map(|(index, _)| index)
            else {
                return;
            };
            free(self.transport, self.uploads.remove(index).id, out);
        }
    }
}

/// `a=t`: the strip's straight-alpha RGBA, base64 in 4096-byte chunks.
fn transmit(transport: Transport, id: u32, frame: &RasterFrame, out: &mut Vec<u8>) {
    let mut encoded = Vec::with_capacity(frame.pixels.len().div_ceil(3) * 4);
    base64(&straight_alpha(&frame.pixels), &mut encoded);
    let mut chunks = encoded.chunks(CHUNK).peekable();
    let mut first = true;
    // An empty strip still needs one (empty) command to say so.
    if chunks.peek().is_none() {
        write_command(
            transport,
            out,
            format_args!(
                "a=t,f=32,t=d,i={id},s={},v={},q=1,m=0",
                frame.width, frame.height
            ),
            b"",
        );
        return;
    }
    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        if first {
            write_command(
                transport,
                out,
                format_args!(
                    "a=t,f=32,t=d,i={id},s={},v={},q=1,m={more}",
                    frame.width, frame.height
                ),
                chunk,
            );
            first = false;
        } else {
            write_command(transport, out, format_args!("m={more}"), chunk);
        }
    }
}

/// `a=p`: move to the cell, place the crop into the cell rectangle,
/// replacing placement `p=1`, and leave the cursor where it was.
fn put_placement(transport: Transport, id: u32, place: &Place<'_>, out: &mut Vec<u8>) {
    let frame = place.frame;
    let cell_height = u32::try_from(frame.height as usize / frame.rows.max(1)).unwrap_or(0);
    let start = u32::try_from(place.rows.start).unwrap_or(u32::MAX);
    let count = u32::try_from(place.rows.len()).unwrap_or(u32::MAX);
    let rows = if place.cells.rows == 0 {
        count
    } else {
        u32::from(place.cells.rows)
    };
    let CellRect {
        column,
        row,
        columns,
        ..
    } = place.cells;
    out.extend_from_slice(b"\x1b7");
    // `write!` into a `Vec` cannot fail.
    write!(
        out,
        "\x1b[{};{}H",
        u32::from(row) + 1,
        u32::from(column) + 1
    )
    .ok();
    write_command(
        transport,
        out,
        format_args!(
            "a=p,i={id},p=1,x=0,y={},w={},h={},c={columns},r={rows},C=1,q=1",
            start.saturating_mul(cell_height),
            frame.width,
            count.saturating_mul(cell_height),
        ),
        b"",
    );
    out.extend_from_slice(b"\x1b8");
}

/// `a=d,d=i`: remove a strip's placement, keep its data.
fn delete_placement(transport: Transport, id: u32, out: &mut Vec<u8>) {
    write_command(transport, out, format_args!("a=d,d=i,i={id},p=1,q=1"), b"");
}

/// `a=d,d=I`: delete an image and free its data.
fn free(transport: Transport, id: u32, out: &mut Vec<u8>) {
    write_command(transport, out, format_args!("a=d,d=I,i={id},q=1"), b"");
}

/// `ESC _ G <control>[;payload] ESC \`, over `transport`.
fn write_command(
    transport: Transport,
    out: &mut Vec<u8>,
    control: std::fmt::Arguments<'_>,
    payload: &[u8],
) {
    let mut command = Vec::with_capacity(payload.len() + 64);
    command.extend_from_slice(b"\x1b_G");
    // `write!` into a `Vec` cannot fail.
    command.write_fmt(control).ok();
    if !payload.is_empty() {
        command.push(b';');
        command.extend_from_slice(payload);
    }
    command.extend_from_slice(b"\x1b\\");
    transport.write(&command, out);
}

/// Premultiplied RGBA to the straight alpha `f=32` means.
pub(crate) fn straight_alpha(pixels: &[u8]) -> Vec<u8> {
    let mut straight = Vec::with_capacity(pixels.len());
    for &[red, green, blue, alpha] in pixels.as_chunks::<4>().0 {
        let channel = |value: u8| {
            // Rounded, and clamped: a premultiplied channel above alpha is
            // malformed but must not overflow. Zero alpha has no colour.
            (u16::from(value) * 255 + u16::from(alpha) / 2)
                .checked_div(u16::from(alpha))
                .map_or(0, |straight| {
                    u8::try_from(straight.min(255)).unwrap_or(u8::MAX)
                })
        };
        straight.extend_from_slice(&[channel(red), channel(green), channel(blue), alpha]);
    }
    straight
}

/// Standard base64 with padding.
pub(crate) fn base64(input: &[u8], out: &mut Vec<u8>) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for group in input.chunks(3) {
        let bits = group.iter().enumerate().fold(0u32, |bits, (at, byte)| {
            bits | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            if at <= group.len() {
                out.push(ALPHABET[(bits >> (18 - 6 * at) & 63) as usize]);
            } else {
                out.push(b'=');
            }
        }
    }
}

/// Every Kitty graphics reply in `bytes`; anything else is skipped.
#[must_use]
pub fn replies(bytes: &[u8]) -> Vec<GraphicsReply> {
    let mut found = Vec::new();
    let mut rest = bytes;
    while let Some(start) = rest.windows(3).position(|w| w == b"\x1b_G") {
        let body = &rest[start + 3..];
        let Some(end) = body.windows(2).position(|w| w == b"\x1b\\") else {
            break;
        };
        found.push(parse_reply(&body[..end]));
        rest = &body[end + 2..];
    }
    found
}

/// `i=…,p=…;message`.
fn parse_reply(body: &[u8]) -> GraphicsReply {
    let text = String::from_utf8_lossy(body);
    let (control, message) = text.split_once(';').unwrap_or((&text, ""));
    let key = |name: &str| {
        control
            .split(',')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.parse().ok())
    };
    GraphicsReply {
        image: key("i"),
        placement: key("p"),
        message: message
            .chars()
            .map(|c| if c.is_control() { '\u{fffd}' } else { c })
            .collect(),
    }
}

#[cfg(test)]
mod tests;
