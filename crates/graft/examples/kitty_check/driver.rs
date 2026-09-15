//! Builds the command bytes for each step of the pixel-tier lifecycle and
//! drives them through `terminal::run_step`. See `main` for the check's
//! shape and `terminal` for how a step is sent, fenced and decoded.

use std::fmt::Write as _;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use graft_core::raster::RasterFrame;
use graft_term::kitty::{CellRect, KittyImages, Place};

use crate::terminal::{Step, run_step, spawn_stdin};

/// Rows per synthetic strip, and the pixels of one cell.
const ROWS: usize = 8;
const CELL: (u32, u32) = (8, 16);
/// Lane columns per strip: two cells per lane, three lanes.
const COLUMNS: u32 = 6;

pub(crate) fn check() -> io::Result<(String, Vec<u8>)> {
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

    steps.extend(f3_round_trip(
        &mut images,
        &strips,
        &mut stdout,
        &input,
        &mut capture,
    )?);

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

/// F3 off the pixel tier and back: re-place, free every upload (as the app
/// does on leaving the tier), then a frame that transmits again and places
/// (m1-the-graph/33).
fn f3_round_trip(
    images: &mut KittyImages,
    strips: &[Arc<RasterFrame>; 2],
    stdout: &mut impl Write,
    input: &Receiver<Vec<u8>>,
    capture: &mut Vec<u8>,
) -> io::Result<Vec<Step>> {
    let replaced = run_step(
        "re-place before F3",
        &frame(images, strips, 0),
        stdout,
        input,
        capture,
    )?;
    let mut out = Vec::new();
    images.free_all(&mut out);
    let away = run_step("F3 away: free all", &out, stdout, input, capture)?;
    let back = run_step(
        "F3 back: transmit + place",
        &frame(images, strips, 0),
        stdout,
        input,
        capture,
    )?;
    Ok(vec![replaced, away, back])
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
