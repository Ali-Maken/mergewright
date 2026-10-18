//! Byte-level encoding: strips and placements to Kitty graphics protocol
//! commands, pure functions over a caller's output buffer.

use std::fmt;
use std::io::Write as _;

use mergewright_core::raster::RasterFrame;
use miniz_oxide::deflate::core::{
    CompressorOxide, TDEFLFlush, TDEFLStatus, compress, create_comp_flags_from_zip_params,
};

use super::{CellRect, Layer, Place};
use crate::tmux::Transport;

/// Base64 bytes per transmit chunk, the protocol's limit.
const CHUNK: usize = 4096;

/// zlib level for `o=z`: the fastest, because encode time is frame budget
/// and a graph strip is mostly runs of transparent pixels, which even the
/// fastest level collapses.
const DEFLATE_LEVEL: u8 = 1;

/// The buffers one transmit works in, kept for the next (m8 04): each is
/// strip-sized, and making them afresh for every upload was a large
/// allocation churn malloc kept resident.
#[derive(Default)]
pub(super) struct Scratch {
    pub(super) straight: Vec<u8>,
    pub(super) deflated: Vec<u8>,
    pub(super) encoded: Vec<u8>,
    /// Made on the first deflated upload, reset for each after.
    compressor: Option<Box<CompressorOxide>>,
}

impl fmt::Debug for Scratch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Scratch")
            .field("straight", &self.straight.capacity())
            .field("deflated", &self.deflated.capacity())
            .field("encoded", &self.encoded.capacity())
            .finish_non_exhaustive()
    }
}

/// `a=t`: the strip's straight-alpha RGBA, base64 in 4096-byte chunks;
/// zlib-deflated first, with `o=z`, when `deflate` is on. Whether it went
/// deflated: only if the compressor fails does a `deflate` upload go raw.
pub(super) fn transmit(
    transport: Transport,
    id: u32,
    frame: &RasterFrame,
    deflate: bool,
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> bool {
    straight_alpha_into(&frame.pixels, &mut scratch.straight);
    let deflated = deflate && {
        let compressor = scratch.compressor.get_or_insert_with(|| {
            Box::new(CompressorOxide::new(create_comp_flags_from_zip_params(
                DEFLATE_LEVEL.into(),
                1,
                0,
            )))
        });
        deflate_into(compressor, &scratch.straight, &mut scratch.deflated)
    };
    let (payload, compression) = if deflated {
        (&scratch.deflated, "o=z,")
    } else {
        (&scratch.straight, "")
    };
    let encoded = &mut scratch.encoded;
    encoded.clear();
    base64(payload, encoded);
    let mut chunks = encoded.chunks(CHUNK).peekable();
    let mut first = true;
    // An empty strip still needs one (empty) command to say so.
    if chunks.peek().is_none() {
        write_command(
            transport,
            out,
            format_args!(
                "a=t,f=32,t=d,{compression}i={id},s={},v={},q=1,m=0",
                frame.width, frame.height
            ),
            b"",
        );
        return deflated;
    }
    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        if first {
            write_command(
                transport,
                out,
                format_args!(
                    "a=t,f=32,t=d,{compression}i={id},s={},v={},q=1,m={more}",
                    frame.width, frame.height
                ),
                chunk,
            );
            first = false;
        } else {
            write_command(transport, out, format_args!("m={more}"), chunk);
        }
    }
    deflated
}

/// zlib-deflate `input` into `output` with `compressor`, both reused from
/// the last upload. `false` if the compressor reports a failure, which
/// `miniz_oxide` documents as a bug in itself: the caller sends raw instead,
/// a correct picture at a higher byte cost.
fn deflate_into(compressor: &mut CompressorOxide, input: &[u8], output: &mut Vec<u8>) -> bool {
    compressor.reset();
    output.clear();
    output.resize((input.len() / 2).max(64), 0);
    let (mut input, mut at) = (input, 0);
    loop {
        let (status, read, wrote) =
            compress(compressor, input, &mut output[at..], TDEFLFlush::Finish);
        at += wrote;
        match status {
            TDEFLStatus::Done => {
                output.truncate(at);
                return true;
            }
            TDEFLStatus::Okay if read <= input.len() => {
                input = &input[read..];
                if output.len() - at < 30 {
                    output.resize(output.len() * 2, 0);
                }
            }
            _ => return false,
        }
    }
}

/// What a placement reads of an uploaded image: its width and the height of
/// one of its rows, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Shape {
    width: u32,
    cell_height: u32,
}

impl Shape {
    pub(super) fn of(frame: &RasterFrame) -> Shape {
        Shape {
            width: frame.width,
            cell_height: u32::try_from(frame.height as usize / frame.rows.max(1)).unwrap_or(0),
        }
    }
}

/// `a=p`: move to the cell, place the crop into the cell rectangle,
/// replacing placement `p` (the place's count among this frame's places of
/// the image, from 1: a strip has one, an emphasis piece one per spot its
/// key turns up), and leave the cursor where it was.
pub(super) fn put_placement(
    transport: Transport,
    id: u32,
    placement: u32,
    shape: Shape,
    place: &Place<'_>,
    out: &mut Vec<u8>,
) {
    let Shape { width, cell_height } = shape;
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
            "a=p,i={id},p={placement},x=0,y={},w={},h={},c={columns},r={rows},C=1,{z}q=1",
            start.saturating_mul(cell_height),
            width,
            count.saturating_mul(cell_height),
            // Strips send no `z` at all (protocol default 0), so their bytes
            // are what they were before layers existed. Emphasis stacks by
            // `z` alone: rank 0 at `z=1`, each rank one higher.
            z = match place.layer {
                Layer::Strip => String::new(),
                Layer::Emphasis => format!("z={},", 1 + u32::from(place.stack)),
            },
        ),
        b"",
    );
    out.extend_from_slice(b"\x1b8");
}

/// `a=d,d=i`: remove one placement of an image, keep its data.
pub(super) fn delete_placement(transport: Transport, id: u32, placement: u32, out: &mut Vec<u8>) {
    write_command(
        transport,
        out,
        format_args!("a=d,d=i,i={id},p={placement},q=1"),
        b"",
    );
}

/// `a=d,d=I`: delete an image and free its data.
pub(super) fn free(transport: Transport, id: u32, out: &mut Vec<u8>) {
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
///
/// Encode time is frame budget (m1-the-graph/30f: three strips in one
/// `PageDown` frame). A graph strip is nearly all empty background (alpha 0)
/// and opaque ink (alpha 255), which need no division; only antialiased edges
/// pay for one per channel.
pub(crate) fn straight_alpha(pixels: &[u8]) -> Vec<u8> {
    let mut straight = Vec::new();
    straight_alpha_into(pixels, &mut straight);
    straight
}

/// [`straight_alpha`] into `straight`, reusing its buffer.
fn straight_alpha_into(pixels: &[u8], straight: &mut Vec<u8>) {
    straight.clear();
    straight.resize(pixels.len() / 4 * 4, 0);
    for (to, &[red, green, blue, alpha]) in straight
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(pixels.as_chunks::<4>().0)
    {
        *to = match alpha {
            // Zero alpha has no colour; the buffer is zeroed already.
            0 => continue,
            255 => [red, green, blue, alpha],
            _ => {
                let channel = |value: u8| {
                    // Rounded, and clamped: a premultiplied channel above
                    // alpha is malformed but must not overflow.
                    let straight =
                        (u16::from(value) * 255 + u16::from(alpha) / 2) / u16::from(alpha);
                    u8::try_from(straight.min(255)).unwrap_or(u8::MAX)
                };
                [channel(red), channel(green), channel(blue), alpha]
            }
        };
    }
}

/// Standard base64 with padding, appended to `out`.
pub(crate) fn base64(input: &[u8], out: &mut Vec<u8>) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let symbol = |bits: u32, shift: u32| ALPHABET[(bits >> shift & 63) as usize];
    let start = out.len();
    out.resize(start + input.len().div_ceil(3) * 4, b'=');
    let (groups, rest) = input.as_chunks::<3>();
    let (whole, tail) = out[start..].as_chunks_mut::<4>();
    for (to, &[a, b, c]) in whole.iter_mut().zip(groups) {
        let bits = u32::from(a) << 16 | u32::from(b) << 8 | u32::from(c);
        *to = [
            symbol(bits, 18),
            symbol(bits, 12),
            symbol(bits, 6),
            symbol(bits, 0),
        ];
    }
    debug_assert!(tail.is_empty(), "the output is a whole number of groups");
    if let Some(last) = whole.last_mut().filter(|_| !rest.is_empty()) {
        let bits = rest.iter().enumerate().fold(0u32, |bits, (at, byte)| {
            bits | u32::from(*byte) << (16 - 8 * at)
        });
        // One byte left: two symbols and `==`; two bytes: three and `=`.
        for (at, to) in last.iter_mut().enumerate().take(rest.len() + 1) {
            *to = symbol(bits, 18 - 6 * u32::try_from(at).unwrap_or(0));
        }
    }
}
