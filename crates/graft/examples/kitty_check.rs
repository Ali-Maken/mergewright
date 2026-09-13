//! Real-terminal check of the Kitty image commands Graft sends
//! (m1-the-graph/24d, ADR-0012). Run it *inside* the terminal under test —
//! a pty capture proves nothing about what the terminal does with the bytes.
//!
//! ```text
//! kitty_check <report> <capture>
//! ```
//!
//! Drives `graft_term::kitty::KittyImages` — the encoder the app uses —
//! through the pixel tier's lifecycle, every command at `q=1`:
//!
//! 1. first frame: transmit two strips and place them;
//! 2. scroll: the same uploads re-cropped (no transmit);
//! 3. resize: rows erased one by one (the app's resize clear since
//!    m1-the-graph/25), then a frame that only re-places — no transmit;
//! 4. tier drop: `hide` every placement;
//! 5. controls: a missing id must ENOENT; a full `ESC[2J` then placements
//!    only shows whether the clear freed the uploads;
//! 6. exit: `free_all`.
//!
//! After each step a DA1 query (`CSI c`) fences the replies: everything the
//! terminal says about images before the DA1 answer belongs to that step.
//! With `q=1` the terminal answers only failures, so zero replies is a pass —
//! and the DA1 answer proves the terminal was listening. The strips are
//! synthetic lane stripes, not a rasterised history: this checks the
//! protocol, not the picture (the user judges that by eye).
//!
//! Writes a plain-text report to `<report>`, every byte sent to `<capture>`,
//! and a decoded, human-readable summary of those bytes to `<capture>.txt`.

use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use graft_core::raster::RasterFrame;
use graft_term::kitty::{self, CellRect, KittyImages, Place};

/// Rows per synthetic strip, and the pixels of one cell.
const ROWS: usize = 8;
const CELL: (u32, u32) = (8, 16);
/// Lane columns per strip: two cells per lane, three lanes.
const COLUMNS: u32 = 6;
/// How long a step may wait for the DA1 answer before counting as silent.
const FENCE_DEADLINE: Duration = Duration::from_secs(3);

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(report_path), Some(capture_path)) = (args.next(), args.next()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: kitty_check <report> <capture>",
        ));
    };

    crossterm::terminal::enable_raw_mode()?;
    let outcome = check();
    let restored = crossterm::terminal::disable_raw_mode();
    let (report, capture) = outcome?;
    restored?;

    std::fs::write(&report_path, &report)?;
    std::fs::write(&capture_path, &capture)?;
    std::fs::write(format!("{capture_path}.txt"), decode(&capture))?;
    Ok(())
}

/// One step's name, the bytes it sent, and what the terminal answered.
struct Step {
    name: &'static str,
    transmits: usize,
    placements: usize,
    replies: Vec<kitty::GraphicsReply>,
    fenced: bool,
}

fn check() -> io::Result<(String, Vec<u8>)> {
    let input = spawn_stdin();
    let mut stdout = io::stdout().lock();
    let mut capture = Vec::new();
    let mut images = KittyImages::new(16);
    let strips = [Arc::new(stripes(0)), Arc::new(stripes(1))];
    let mut steps = Vec::new();

    // Start on a clean screen, like the app's alternate screen does.
    let mut out = b"\x1b[?1049h\x1b[2J".to_vec();
    out.extend_from_slice(&frame(&mut images, &strips, 0));
    steps.push(run_step(
        "first frame",
        &out,
        &mut stdout,
        &input,
        &mut capture,
    )?);

    let out = frame(&mut images, &strips, 3);
    steps.push(run_step(
        "scroll 3 rows",
        &out,
        &mut stdout,
        &input,
        &mut capture,
    )?);

    // What the app's resize does since m1-the-graph/25: rows erased one by
    // one (`ESC[2K`, never `ESC[2J`), uploads kept, strips only re-placed.
    let mut out = erase_rows(2 * ROWS + 2);
    out.extend_from_slice(&frame(&mut images, &strips, 0));
    steps.push(run_step(
        "erase rows + replace",
        &out,
        &mut stdout,
        &input,
        &mut capture,
    )?);

    let mut out = Vec::new();
    images.hide(&mut out);
    steps.push(run_step("hide", &out, &mut stdout, &input, &mut capture)?);

    let (control, cleared) = controls(&mut images, &strips, &mut stdout, &input, &mut capture)?;

    let mut out = Vec::new();
    images.free_all(&mut out);
    out.extend_from_slice(b"\x1b[?1049l");
    steps.push(run_step(
        "free all",
        &out,
        &mut stdout,
        &input,
        &mut capture,
    )?);

    Ok((report(&steps, &control, &cleared), capture))
}

/// The two controls: a placement of a never-sent id, and a full clear
/// followed by placements only. Replies to the second are answered, so the
/// exit step frees only what the terminal still holds.
fn controls(
    images: &mut KittyImages,
    strips: &[Arc<RasterFrame>; 2],
    stdout: &mut impl Write,
    input: &Receiver<Vec<u8>>,
    capture: &mut Vec<u8>,
) -> io::Result<(Step, Step)> {
    // Control: a placement of an image never sent must come back as one
    // ENOENT. Proves the fence hears errors, so zero above means zero.
    let out = b"\x1b_Ga=p,i=999999,p=1,q=1\x1b\\".to_vec();
    let control = run_step("control: missing id", &out, stdout, input, capture)?;

    // Second control: a full clear, then placements only. Ghostty frees
    // image data on `ESC[2J` (ADR-0012), so this should hear an ENOENT per
    // strip: the reason the app's resize erases rows instead.
    let mut out = b"\x1b[2J".to_vec();
    out.extend_from_slice(&frame(images, strips, 0));
    let cleared = run_step("control: full clear", &out, stdout, input, capture)?;
    for reply in &cleared.replies {
        images.answer(reply);
    }

    Ok((control, cleared))
}

/// `ESC[2K` on each of the first `rows` screen rows, as the app's resize
/// clear does.
fn erase_rows(rows: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for row in 1..=rows {
        out.extend_from_slice(format!("\x1b[{row};1H\x1b[2K").as_bytes());
    }
    out
}

/// The commands showing both strips with `scroll` rows cropped off the top.
fn frame(images: &mut KittyImages, strips: &[Arc<RasterFrame>; 2], scroll: usize) -> Vec<u8> {
    let places: Vec<Place<'_>> = strips
        .iter()
        .enumerate()
        .map(|(index, strip)| {
            let rows = if index == 0 { scroll..ROWS } else { 0..ROWS };
            let row = if index == 0 { 1 } else { 1 + ROWS - scroll };
            Place {
                first_row: index * ROWS,
                frame: strip,
                cells: CellRect {
                    column: 2,
                    row: u16::try_from(row).unwrap_or(u16::MAX),
                    columns: u16::try_from(COLUMNS).unwrap_or(u16::MAX),
                    rows: u16::try_from(rows.len()).unwrap_or(u16::MAX),
                },
                rows,
            }
        })
        .collect();
    let mut out = Vec::new();
    images.frame(&places, &mut out);
    out
}

/// Send `out`, fence it with DA1, and collect the image replies before the
/// DA1 answer.
fn run_step(
    name: &'static str,
    out: &[u8],
    stdout: &mut impl Write,
    input: &Receiver<Vec<u8>>,
    capture: &mut Vec<u8>,
) -> io::Result<Step> {
    let text = String::from_utf8_lossy(out);
    let transmits = text.matches("a=t,").count();
    let placements = text.matches("a=p,").count();
    stdout.write_all(out)?;
    stdout.write_all(b"\x1b[c")?;
    stdout.flush()?;
    capture.extend_from_slice(out);
    capture.extend_from_slice(b"\x1b[c");

    let deadline = Instant::now() + FENCE_DEADLINE;
    let mut heard = Vec::new();
    let fenced = loop {
        if da1_answer(&heard) {
            break true;
        }
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break false;
        };
        match input.recv_timeout(left) {
            Ok(bytes) => heard.extend_from_slice(&bytes),
            Err(mpsc::RecvTimeoutError::Timeout) => break false,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "stdin closed before the DA1 answer",
                ));
            }
        }
    };
    Ok(Step {
        name,
        transmits,
        placements,
        replies: kitty::replies(&heard),
        fenced,
    })
}

/// Whether `bytes` hold a DA1 answer, `CSI ? … c`.
fn da1_answer(bytes: &[u8]) -> bool {
    bytes.windows(3).enumerate().any(|(start, window)| {
        window == b"\x1b[?"
            && bytes[start + 3..]
                .iter()
                .find(|byte| !(byte.is_ascii_digit() || **byte == b';'))
                == Some(&b'c')
    })
}

/// Read stdin on a thread, handing each chunk over as it arrives.
fn spawn_stdin() -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut chunk = [0u8; 4096];
        loop {
            match stdin.read(&mut chunk) {
                Ok(count) if count > 0 => {
                    if sender.send(chunk[..count].to_vec()).is_err() {
                        // The main thread is gone: nobody left to hand to.
                        return;
                    }
                }
                // End of input or a read error: dropping the sender is what
                // reports it, as the waiting step's `Disconnected` error.
                Ok(_) | Err(_) => return,
            }
        }
    });
    receiver
}

/// A strip of three coloured lane stripes, each row a slightly different
/// shade so a wrong crop shows. `seed` offsets the hues between strips.
fn stripes(seed: u8) -> RasterFrame {
    let (cell_width, cell_height) = CELL;
    let width = COLUMNS * cell_width;
    let height = u32::try_from(ROWS).unwrap_or(u32::MAX) * cell_height;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let shade = u8::try_from(y / cell_height * 20).unwrap_or(u8::MAX);
        for x in 0..width {
            let lane = x / (2 * cell_width);
            let on_line = x % (2 * cell_width) < 3;
            let rgba = match (on_line, lane) {
                (false, _) => [0, 0, 0, 0],
                (true, 0) => [255, shade.wrapping_add(seed * 60), 64, 255],
                (true, 1) => [64, 200, shade, 255],
                (true, _) => [shade, 96, 255, 255],
            };
            pixels.extend_from_slice(&rgba);
        }
    }
    RasterFrame {
        width,
        height,
        rows: ROWS,
        columns: COLUMNS,
        pixels,
    }
}

fn report(steps: &[Step], control: &Step, cleared: &Step) -> String {
    let errors: usize = steps
        .iter()
        .map(|step| step.replies.iter().filter(|reply| reply.is_error()).count())
        .sum();
    let silent = steps.iter().filter(|step| !step.fenced).count();
    let mut text = String::new();
    let term = std::env::var("TERM").unwrap_or_else(|_| "(unset)".to_owned());
    let program = std::env::var("TERM_PROGRAM").unwrap_or_else(|_| "(unset)".to_owned());
    let version = std::env::var("TERM_PROGRAM_VERSION").unwrap_or_else(|_| "(unset)".to_owned());
    // `write!` into a `String` cannot fail.
    writeln!(
        text,
        "kitty_check: TERM={term} TERM_PROGRAM={program} {version}"
    )
    .ok();
    for step in steps.iter().chain([control, cleared]) {
        writeln!(
            text,
            "{:<20} transmits {:>2}  placements {:>2}  replies {:>2}  errors {:>2}  {}",
            step.name,
            step.transmits,
            step.placements,
            step.replies.len(),
            step.replies.iter().filter(|reply| reply.is_error()).count(),
            if step.fenced {
                "DA1 answered"
            } else {
                "DA1 SILENT"
            },
        )
        .ok();
        for reply in &step.replies {
            writeln!(text, "    reply i={:?} {}", reply.image, reply.message).ok();
        }
    }
    let heard = control
        .replies
        .iter()
        .filter(|reply| reply.is_missing_image())
        .count();
    let verdict = if errors == 0 && silent == 0 && heard == 1 && control.fenced {
        "PASS"
    } else {
        "FAIL"
    };
    writeln!(
        text,
        "{verdict}: {errors} error replies, {silent} unfenced steps; control heard {heard} ENOENT (want 1)"
    )
    .ok();
    let freed = cleared
        .replies
        .iter()
        .filter(|reply| reply.is_missing_image())
        .count();
    writeln!(
        text,
        "full clear then place: {freed} ENOENT of 2 placements (2 = the clear freed the uploads)"
    )
    .ok();
    text
}

/// The capture made readable: one line per escape sequence, image payloads
/// replaced by their length.
fn decode(bytes: &[u8]) -> String {
    let mut text = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\x1b_G") {
            let end = find(bytes, index, b"\x1b\\").unwrap_or(bytes.len());
            let body = String::from_utf8_lossy(&bytes[index + 3..end]);
            let (control, payload) = body.split_once(';').unwrap_or((&body, ""));
            writeln!(text, "APC G {control}  [payload {} bytes]", payload.len()).ok();
            index = (end + 2).min(bytes.len());
        } else if bytes[index] == 0x1b {
            let end = sequence_end(bytes, index);
            let shown: String = bytes[index + 1..end]
                .iter()
                .map(|&byte| char::from(byte))
                .collect();
            writeln!(text, "ESC {shown}").ok();
            index = end;
        } else {
            let end = find(bytes, index, b"\x1b").unwrap_or(bytes.len());
            writeln!(
                text,
                "text {:?}",
                String::from_utf8_lossy(&bytes[index..end])
            )
            .ok();
            index = end;
        }
    }
    text
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}

/// One past the end of the non-APC escape at `start`: CSI up to its final
/// byte, anything else two bytes.
fn sequence_end(bytes: &[u8], start: usize) -> usize {
    if bytes.get(start + 1) == Some(&b'[') {
        bytes[start + 2..]
            .iter()
            .position(|byte| (0x40..=0x7e).contains(byte))
            .map_or(bytes.len(), |at| start + 3 + at)
    } else {
        (start + 2).min(bytes.len())
    }
}
