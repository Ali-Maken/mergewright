//! graph-spike: throwaway prototype for PROGRESS.md step 3.
//!
//! Draws a hard-coded 30-commit graph as pixel images next to ordinary text
//! rows, to prove that image rows and text rows line up and stay lined up
//! while scrolling with `j`/`k`. Two strategies, chosen by detection:
//!
//! - **Kitty** (Ghostty, Kitty): the whole 30-row strip is rasterized and
//!   transmitted once; every frame just places a crop of it — a miniature
//!   of DESIGN.md §3.8's image reuse.
//! - **iTerm2**: no image ids or crops exist, so the visible band is
//!   re-rasterized, PNG-encoded and re-sent on every frame.
//!
//! `--snapshot [path]` writes the full strip as a PNG and exits (headless
//! geometry check). `GRAPH_SPIKE_PROTOCOL=kitty|iterm2` forces a protocol.
//! Findings: docs/adr/0012.

mod data;
mod raster;
mod term;

use std::io::{self, Write};
use std::time::Instant;

use crossterm::terminal;
use crossterm::tty::IsTty;

use data::{GRAPH_COLS, LANE_COLORS, ROWS};
use raster::Canvas;
use term::{CellSize, Key, Protocol};

/// 1-based terminal column where commit text starts (right of the graph).
const TEXT_COL: u16 = GRAPH_COLS + 2;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--snapshot") {
        let path = args.get(i + 1).map_or("graph-spike.png", String::as_str);
        return snapshot(path);
    }
    if !io::stdout().is_tty() {
        eprintln!("graph-spike: stdout is not a terminal");
        std::process::exit(1);
    }
    run()
}

fn run() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let protocol = term::detect_protocol();
    let cell = term::detect_cell_size();
    let Some(protocol) = protocol else {
        terminal::disable_raw_mode()?;
        eprintln!("graph-spike: no pixel image protocol detected.");
        eprintln!("Run it in Ghostty, Kitty or iTerm2 — or force emission anywhere");
        eprintln!("with GRAPH_SPIKE_PROTOCOL=kitty or GRAPH_SPIKE_PROTOCOL=iterm2.");
        std::process::exit(1);
    };
    let session = Session::start(protocol)?;
    let result = event_loop(protocol, &cell);
    drop(session);
    result
}

/// RAII terminal session: alternate screen and hidden cursor on start;
/// images freed, screen restored and raw mode off on drop, panics included.
struct Session {
    protocol: Protocol,
}

impl Session {
    fn start(protocol: Protocol) -> io::Result<Self> {
        let mut out = io::stdout();
        out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J")?;
        out.flush()?;
        Ok(Session { protocol })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let mut out = io::stdout();
        if self.protocol == Protocol::Kitty {
            let _ = term::kitty_free_all(&mut out);
        }
        let _ = out.write_all(b"\x1b[?2026l\x1b[?25h\x1b[?1049l");
        let _ = out.flush();
        let _ = terminal::disable_raw_mode();
    }
}

/// What the current frame shows: terminal size, visible row count, scroll.
struct Viewport {
    cols: u16,
    rows: u16,
    view: usize,
    offset: usize,
}

/// Frame timings shown in the status line, in milliseconds.
#[derive(Default)]
struct Stats {
    frame: f64,
    raster: f64,
    encode: f64,
    /// Kitty only: the one-off cost of transmitting the full strip.
    upload_once: f64,
}

fn event_loop(protocol: Protocol, cell: &CellSize) -> io::Result<()> {
    let mut out = io::BufWriter::new(io::stdout());
    let total = ROWS.len();

    let mut upload_once = 0.0;
    if protocol == Protocol::Kitty {
        let t = Instant::now();
        let full = rasterize(0, total, cell);
        term::kitty_transmit(&mut out, full.rgba(), full.w, full.h)?;
        out.flush()?;
        upload_once = ms(t);
    }

    let mut offset = 0usize;
    loop {
        let (cols, rows) = terminal::size()?;
        let view = (rows.saturating_sub(1) as usize).clamp(1, total);
        let max_offset = total - view;
        offset = offset.min(max_offset);
        let vp = Viewport {
            cols,
            rows,
            view,
            offset,
        };
        let mut stats = draw_frame(&mut out, protocol, cell, &vp)?;
        stats.upload_once = upload_once;
        draw_status(&mut out, protocol, cell, &vp, &stats)?;
        out.write_all(b"\x1b[?2026l")?; // end synchronized update
        out.flush()?;
        match term::read_key() {
            Key::Quit => return Ok(()),
            Key::Down => offset = (offset + 1).min(max_offset),
            Key::Up => offset = offset.saturating_sub(1),
            Key::Other => {}
        }
    }
}

/// Draw the graph image and the text rows for one frame. Leaves the
/// synchronized update open so the status line joins the same frame.
fn draw_frame(
    out: &mut impl Write,
    protocol: Protocol,
    cell: &CellSize,
    vp: &Viewport,
) -> io::Result<Stats> {
    let frame_start = Instant::now();
    let mut stats = Stats::default();
    out.write_all(b"\x1b[?2026h\x1b[2J")?; // begin synchronized update, clear
    match protocol {
        Protocol::Kitty => {
            term::kitty_clear_placements(out)?;
            out.write_all(b"\x1b[1;1H")?;
            term::kitty_place(
                out,
                vp.offset as u32 * cell.h,
                strip_width(cell),
                vp.view as u32 * cell.h,
                GRAPH_COLS,
                vp.view as u16,
            )?;
        }
        Protocol::Iterm2 => {
            let t = Instant::now();
            let band = rasterize(vp.offset, vp.view, cell);
            stats.raster = ms(t);
            let t = Instant::now();
            let png = band.png()?;
            stats.encode = ms(t);
            out.write_all(b"\x1b[1;1H")?;
            term::iterm2_inline(out, &png, GRAPH_COLS, vp.view as u16)?;
        }
    }
    draw_text(out, vp)?;
    out.flush()?;
    stats.frame = ms(frame_start);
    Ok(stats)
}

/// One text line per commit, at a fixed column right of the graph pane.
/// Every write starts with an absolute cursor move — never trust where an
/// image emission left the cursor.
fn draw_text(out: &mut impl Write, vp: &Viewport) -> io::Result<()> {
    let budget = usize::from(vp.cols.saturating_sub(TEXT_COL)) + 1;
    for i in 0..vp.view {
        let row = &ROWS[vp.offset + i];
        write!(out, "\x1b[{};{}H", i + 1, TEXT_COL)?;
        let mut left = budget;
        write_clipped(out, "\x1b[2m", row.id, &mut left)?; // dim
        write_clipped(out, "\x1b[0m", " ", &mut left)?;
        write_clipped(out, "", row.msg, &mut left)?;
        if !row.refs.is_empty() {
            write_clipped(out, "", "  ", &mut left)?;
            write_clipped(out, "\x1b[36m", row.refs, &mut left)?; // cyan
        }
        out.write_all(b"\x1b[0m")?;
    }
    Ok(())
}

/// Write `text` in `style`, clipped to the remaining `left` columns.
/// Byte clipping is fine: the graph data is all ASCII.
fn write_clipped(
    out: &mut impl Write,
    style: &str,
    text: &str,
    left: &mut usize,
) -> io::Result<()> {
    if *left == 0 {
        return Ok(());
    }
    let take = text.len().min(*left);
    write!(out, "{style}{}", &text[..take])?;
    *left -= take;
    Ok(())
}

/// Inverse-video status line on the terminal's last row: protocol, cell
/// size and its provenance, scroll position, frame timings, keys.
fn draw_status(
    out: &mut impl Write,
    protocol: Protocol,
    cell: &CellSize,
    vp: &Viewport,
    stats: &Stats,
) -> io::Result<()> {
    let provenance = if cell.reported { "reported" } else { "assumed" };
    let mut text = format!(
        " {} | cell {}x{}px ({provenance}) | rows {}-{}/{} | frame {:.1}ms \
         (raster {:.1} encode {:.1} upload-once {:.0}) | j/k scroll, q quit",
        protocol.name(),
        cell.w,
        cell.h,
        vp.offset + 1,
        vp.offset + vp.view,
        ROWS.len(),
        stats.frame,
        stats.raster,
        stats.encode,
        stats.upload_once,
    );
    text.truncate(usize::from(vp.cols));
    let width = usize::from(vp.cols);
    write!(out, "\x1b[{};1H\x1b[7m{text:<width$}\x1b[0m", vp.rows)
}

/// Pixel width of the graph strip.
fn strip_width(cell: &CellSize) -> u32 {
    u32::from(GRAPH_COLS) * cell.w
}

/// Rasterize graph rows `first..first + count` into a fresh strip.
fn rasterize(first: usize, count: usize, cell: &CellSize) -> Canvas {
    let mut canvas = Canvas::new(strip_width(cell), count as u32 * cell.h);
    draw_rows(&mut canvas, first, count, cell);
    canvas
}

/// Draw graph rows into the canvas, one `cell.h`-tall band per row. Bands
/// map 1:1 onto terminal rows — that identity is the alignment claim the
/// whole spike exists to test.
fn draw_rows(canvas: &mut Canvas, first: usize, count: usize, cell: &CellSize) {
    let (cw, ch) = (cell.w as f32, cell.h as f32);
    let thickness = (ch / 10.0).clamp(2.0, 5.0);
    for (i, row) in ROWS[first..first + count].iter().enumerate() {
        let top = i as f32 * ch;
        let mid = top + ch / 2.0;
        let bottom = top + ch;
        let node_x = lane_x(row.lane, cw);
        for &l in row.through {
            canvas.line(
                lane_x(l, cw),
                top,
                lane_x(l, cw),
                bottom,
                thickness,
                LANE_COLORS[l],
            );
        }
        for &l in row.to_node {
            canvas.line(lane_x(l, cw), top, node_x, mid, thickness, LANE_COLORS[l]);
        }
        for &l in row.from_node {
            canvas.line(
                node_x,
                mid,
                lane_x(l, cw),
                bottom,
                thickness,
                LANE_COLORS[l],
            );
        }
        let radius = (ch * 0.3).min(cw * 0.9);
        canvas.disc(node_x, mid, radius, LANE_COLORS[row.lane]);
        if row.from_node.len() > 1 {
            // Merge commits render as rings: punch out the disc's centre.
            canvas.disc(node_x, mid, radius * 0.45, [0, 0, 0, 0]);
        }
    }
}

/// Pixel x of a lane's centre line: each lane owns two character columns.
fn lane_x(lane: usize, cw: f32) -> f32 {
    (lane as f32 * 2.0 + 1.0) * cw
}

/// `--snapshot`: render the whole graph over an opaque background to a PNG
/// on disk, so the raster geometry can be checked without a terminal.
fn snapshot(path: &str) -> io::Result<()> {
    let cell = CellSize {
        w: 20,
        h: 40,
        reported: false,
    };
    let mut canvas = Canvas::new(strip_width(&cell), ROWS.len() as u32 * cell.h);
    canvas.fill([20, 22, 26, 255]);
    draw_rows(&mut canvas, 0, ROWS.len(), &cell);
    std::fs::write(path, canvas.png()?)?;
    println!("wrote {path}");
    Ok(())
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}
