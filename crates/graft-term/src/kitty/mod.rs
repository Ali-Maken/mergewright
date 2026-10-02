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

use std::ops::Range;

use graft_core::raster::RasterFrame;

use crate::tmux::Transport;

mod encode;
mod reply;

pub use reply::replies;

pub(crate) use encode::{base64, straight_alpha};

/// Image ids start here, clear of the ids detection's probes use (`i=31`,
/// `i=32`).
const FIRST_IMAGE_ID: u32 = 1000;

/// Bit set in every [`Layer::Emphasis`] image id and in no strip id.
///
/// It is the order belt for terminals that ignore `z` but stack equal-`z`
/// images by id (Kitty's rule: lower id below), and it lets a byte-level
/// harness tell the layers apart.
pub const EMPHASIS_ID_BIT: u32 = 1 << 31;

/// Which of the two stacked images a [`Place`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Layer {
    /// The rasterised graph strip. Placed with no `z` (protocol default 0).
    #[default]
    Strip,
    /// Selection emphasis over a strip: placed with `z=1`, always after
    /// every strip in the same frame, and with [`EMPHASIS_ID_BIT`] set.
    Emphasis,
}

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

/// What a picture is, for the upload cache: equal keys in one slot are
/// equal pixels. The strip cache's own key for the strip or layer (layout,
/// theme, cell size, backdrop, selection), so a strip the cache evicted and
/// drew again is still the image the terminal holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UploadKey(pub u64);

/// One strip to show this frame: which rows of it, and in which cells.
#[derive(Clone, Debug)]
pub struct Place<'a> {
    /// The history row the strip starts on; identifies the strip's slot.
    pub first_row: usize,
    /// What the pixels are. A different key for the same slot means the
    /// pixels changed and the strip is transmitted again.
    pub key: UploadKey,
    /// The rasterised strip; read only to transmit it.
    pub frame: &'a RasterFrame,
    /// Rows of the strip to show, counted from the strip's own first row.
    pub rows: Range<usize>,
    /// Where on screen those rows go.
    pub cells: CellRect,
    /// The strip itself, or the emphasis above it. Each layer of a slot is
    /// its own upload.
    pub layer: Layer,
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

/// An image the terminal holds for us. Its pixels are the terminal's; Graft
/// keeps only what they were.
#[derive(Debug)]
struct Upload {
    id: u32,
    first_row: usize,
    layer: Layer,
    key: UploadKey,
    placed: bool,
    last_used: u64,
    /// Sent with `o=z`: an error reply about it may mean the terminal
    /// cannot inflate.
    deflated: bool,
}

/// What Graft has uploaded to a Kitty-protocol terminal, and the commands
/// that keep the screen matching the strips a frame wants.
#[derive(Debug)]
pub struct KittyImages {
    uploads: Vec<Upload>,
    capacity: usize,
    next_id: u32,
    next_emphasis_id: u32,
    clock: u64,
    transport: Transport,
    deflate: bool,
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
            next_emphasis_id: FIRST_IMAGE_ID | EMPHASIS_ID_BIT,
            clock: 0,
            transport: Transport::Direct,
            deflate: false,
        }
    }

    /// Transmit with `o=z` (zlib-deflated RGBA). Off by default; turn it on
    /// only when detection found `kitty_deflate`.
    #[must_use]
    pub fn deflating(mut self, on: bool) -> Self {
        self.deflate = on;
        self
    }

    /// Still deflating: false once an error reply about a deflated upload
    /// turned it off for good (see [`Self::answer`]).
    #[must_use]
    pub fn is_deflating(&self) -> bool {
        self.deflate
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
    ///
    /// Every [`Layer::Strip`] place is emitted before any
    /// [`Layer::Emphasis`] one, whatever order `places` lists them in, and
    /// every shown image is re-placed each frame: so emphasis always lands
    /// after its strip, even on a terminal that stacks by placement order.
    pub fn frame(&mut self, places: &[Place<'_>], out: &mut Vec<u8>) {
        self.clock += 1;
        let mut shown = Vec::with_capacity(places.len());
        let strips = places.iter().filter(|place| place.layer == Layer::Strip);
        let emphasis = places.iter().filter(|place| place.layer == Layer::Emphasis);
        for place in strips.chain(emphasis) {
            let id = self.ensure_uploaded(place, out);
            shown.push(id);
            encode::put_placement(self.transport, id, place, out);
        }
        for upload in &mut self.uploads {
            if upload.placed && !shown.contains(&upload.id) {
                encode::delete_placement(self.transport, upload.id, out);
            }
            upload.placed = shown.contains(&upload.id);
        }
        self.evict(out);
    }

    /// Remove every placement, keeping the data (the tier dropped away from
    /// pixels, or the graph is not on screen).
    pub fn hide(&mut self, out: &mut Vec<u8>) {
        for upload in self.uploads.iter_mut().filter(|upload| upload.placed) {
            encode::delete_placement(self.transport, upload.id, out);
            upload.placed = false;
        }
    }

    /// Delete every upload with its data. Call on exit.
    pub fn free_all(&mut self, out: &mut Vec<u8>) {
        for upload in self.uploads.drain(..) {
            encode::free(self.transport, upload.id, out);
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
    ///
    /// Any other error about an upload that went out deflated turns
    /// deflating off for the rest of this value's life and forgets that
    /// upload, so it goes again raw — today's known-good encoding. One-way:
    /// no retry, and the cost is one wasted transmit per image in flight.
    pub fn answer(&mut self, reply: &GraphicsReply) {
        let Some(id) = reply.image else {
            return;
        };
        if reply.is_missing_image() {
            self.uploads.retain(|upload| upload.id != id);
        } else if reply.is_error()
            && self
                .uploads
                .iter()
                .any(|upload| upload.id == id && upload.deflated)
        {
            self.deflate = false;
            self.uploads.retain(|upload| upload.id != id);
        }
    }

    /// The id holding `place`'s pixels, transmitting them first if needed.
    fn ensure_uploaded(&mut self, place: &Place<'_>, out: &mut Vec<u8>) -> u32 {
        let clock = self.clock;
        if let Some(index) = self
            .uploads
            .iter()
            .position(|u| u.first_row == place.first_row && u.layer == place.layer)
        {
            if self.uploads[index].key == place.key {
                self.uploads[index].last_used = clock;
                return self.uploads[index].id;
            }
            // Same slot, new pixels: a fresh id, so a late reply about the
            // old image can never be mistaken for one about the new.
            encode::free(self.transport, self.uploads.remove(index).id, out);
        }
        let id = self.allocate(place.layer);
        encode::transmit(self.transport, id, place.frame, self.deflate, out);
        self.uploads.push(Upload {
            id,
            first_row: place.first_row,
            layer: place.layer,
            key: place.key,
            placed: false,
            last_used: clock,
            deflated: self.deflate,
        });
        id
    }

    /// A fresh id for `layer`: strips count up below [`EMPHASIS_ID_BIT`],
    /// emphasis counts up above it, each wrapping back to its own start.
    fn allocate(&mut self, layer: Layer) -> u32 {
        let (next, start) = match layer {
            Layer::Strip => (&mut self.next_id, FIRST_IMAGE_ID),
            Layer::Emphasis => (&mut self.next_emphasis_id, FIRST_IMAGE_ID | EMPHASIS_ID_BIT),
        };
        let id = *next;
        let after = id.wrapping_add(1);
        *next = if (after & EMPHASIS_ID_BIT) == (start & EMPHASIS_ID_BIT) && after != 0 {
            after
        } else {
            start
        };
        debug_assert_eq!(
            id & EMPHASIS_ID_BIT != 0,
            layer == Layer::Emphasis,
            "an id's layer bit must match its layer"
        );
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
            encode::free(self.transport, self.uploads.remove(index).id, out);
        }
    }
}

#[cfg(test)]
mod tests;
