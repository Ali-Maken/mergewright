//! Byte-level encoding: strips and placements to Kitty graphics protocol
//! commands, pure functions over a caller's output buffer.

use std::io::Write as _;

use graft_core::raster::RasterFrame;

use super::{CellRect, Place};
use crate::tmux::Transport;

/// Base64 bytes per transmit chunk, the protocol's limit.
const CHUNK: usize = 4096;

/// `a=t`: the strip's straight-alpha RGBA, base64 in 4096-byte chunks.
pub(super) fn transmit(transport: Transport, id: u32, frame: &RasterFrame, out: &mut Vec<u8>) {
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
pub(super) fn put_placement(transport: Transport, id: u32, place: &Place<'_>, out: &mut Vec<u8>) {
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
pub(super) fn delete_placement(transport: Transport, id: u32, out: &mut Vec<u8>) {
    write_command(transport, out, format_args!("a=d,d=i,i={id},p=1,q=1"), b"");
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
    let mut straight = vec![0; pixels.len() / 4 * 4];
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
    straight
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
