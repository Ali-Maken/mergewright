//! graph-spike: throwaway prototype for PROGRESS.md step 3.
//!
//! Draws a hard-coded 30-commit graph as pixel images next to ordinary text
//! rows, to prove that image rows and text rows line up and stay lined up
//! while scrolling with `j`/`k`. Two strategies, chosen by detection:
//!
//! - **Kitty** (Ghostty, Kitty): the whole 30-row strip is rasterized and
//!   transmitted after each full clear; every frame then just replaces one
//!   placement (`p=1`) with a new crop of it — a miniature of DESIGN.md
//!   §3.8's image reuse. The frame loop itself never emits `ESC[2J`:
//!   Ghostty frees kitty image *data* on ED, so a clear after the upload
//!   silently kills every later placement (the step-3 "blank in Ghostty"
//!   bug). Full clears happen only on terminal-size changes, immediately
//!   followed by a retransmit.
//! - **iTerm2**: no image ids or crops exist, so the visible band is
//!   re-rasterized, PNG-encoded and re-sent on every frame, after a full
//!   clear (kept as originally written). Confirmed to draw in a real
//!   iTerm2 by the user, 2026-09-12; see docs/adr/0012.
//!
//! `--snapshot [path]` writes the full strip as a PNG and exits (headless
//! geometry check). `GRAPH_SPIKE_PROTOCOL=kitty|iterm2` forces a protocol.
//! `GRAPH_SPIKE_DEBUG=1` dumps the sent escape stream and the probe replies
//! to `graph-spike-debug.log` in the working directory.
//! Findings and their correction: docs/adr/0012.

mod data;
mod debug;
mod raster;
mod term;

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::terminal;
use crossterm::tty::IsTty;

use data::{GRAPH_COLS, LANE_COLORS, ROWS};
use raster::Canvas;
use term::{CellSize, Key, Protocol};

/// 1-based terminal column where commit text starts. Columns 1..=GRAPH_COLS
/// are reserved for the graph image, plus one blank separator column; text
/// never writes left of this, and its end-of-line erase never reaches back
/// into the reserved columns.
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
    let result = match bench_frames() {
        Some(n) => bench(protocol, &cell, n),
        None => event_loop(protocol, &cell),
    };
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
        let mut out = debug::Tee::new(io::stdout());
        out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J")?;
        out.flush()?;
        Ok(Session { protocol })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Ignored results, deliberately: Drop cannot propagate errors, and
        // a failed write here means the terminal itself is gone — there is
        // nobody left to tell and nothing better to do than keep restoring.
        let mut out = debug::Tee::new(io::stdout());
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
///
/// On the Kitty path, `raster` and `upload` are nonzero only on frames that
/// retransmit the strip (the first frame, and after a resize); ordinary
/// scroll frames are placement-only and honestly show 0.0. `encode` is
/// iTerm2-only (the Kitty path sends raw RGBA, nothing to encode).
///
/// Every number here belongs to the frame being drawn, except `frame`,
/// which is the *previous* frame's total: a frame's own cost isn't known
/// until its last byte has been flushed, which is after the status line is
/// written. An earlier version carried the last upload forward into every
/// frame's status instead, which made a place-only frame read as if it had
/// just re-uploaded the strip. Numbers that outlive their frame lie.
#[derive(Default)]
struct Stats {
    frame: f64,
    raster: f64,
    encode: f64,
    /// Cost of this frame's full-strip transmit (Kitty only); 0.0 on the
    /// scroll frames, which only move a crop.
    upload: f64,
    /// Whether this frame sent image pixels at all. True on a Kitty
    /// transmit, and on *every* iTerm2 frame — that tier has no image ids,
    /// so it cannot reuse anything. The status line's `tx` counter adds
    /// these up, which is what makes "uploaded once" checkable by eye
    /// instead of a claim in a comment.
    sent_image: bool,
}

/// Frame count for `GRAPH_SPIKE_BENCH`, when it names a positive number.
fn bench_frames() -> Option<usize> {
    std::env::var("GRAPH_SPIKE_BENCH")
        .ok()?
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0)
}

/// Wanted time per bench frame, from `GRAPH_SPIKE_BENCH_HZ` (default 30,
/// roughly a held-down key's repeat rate; `0` means flat out).
fn bench_gap() -> Duration {
    let hz = std::env::var("GRAPH_SPIKE_BENCH_HZ")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(30);
    if hz == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(1.0 / f64::from(hz))
    }
}

/// The frame loop's state, shared by the interactive loop and the bench
/// driver so both measure exactly the same work.
struct Frames {
    offset: usize,
    /// The terminal size the last frame was drawn for. A change forces a
    /// full clear — and, on Kitty, a retransmit, because the clear frees
    /// the image data (see `draw_frame`).
    drawn_for: Option<(u16, u16)>,
    /// Full-strip transmits so far, shown in the status line as `tx N`.
    /// In the steady state it must stay at 1: a climbing counter is an
    /// "upload once" path that is quietly re-uploading, which is the
    /// failure mode this spike keeps re-growing.
    transmits: u32,
    /// Total wall time of the previous frame, ms.
    prev_frame: f64,
    /// Largest scroll offset the current window allows.
    max_offset: usize,
    /// Frames drawn so far.
    count: usize,
    /// Every frame that had to clear and retransmit, with the terminal size
    /// that triggered it. The bench prints these: "upload once" is only a
    /// claim until the list of reasons is on the page next to the count.
    refreshes: Vec<(usize, u16, u16)>,
}

impl Frames {
    fn new() -> Self {
        Frames {
            offset: 0,
            drawn_for: None,
            transmits: 0,
            prev_frame: 0.0,
            max_offset: 0,
            count: 0,
            refreshes: Vec::new(),
        }
    }

    /// Draw one whole frame — graph, text rows, status line — and flush it.
    /// Returns the frame's total wall time in ms, which includes the writes
    /// blocking on the terminal: on a real terminal that back-pressure is
    /// most of the cost.
    fn render(
        &mut self,
        out: &mut impl Write,
        protocol: Protocol,
        cell: &CellSize,
        gfx_err: Option<&str>,
    ) -> io::Result<f64> {
        let start = Instant::now();
        let (cols, rows) = terminal::size()?;
        let refresh = self.drawn_for != Some((cols, rows));
        self.drawn_for = Some((cols, rows));
        self.count += 1;
        if refresh {
            self.refreshes.push((self.count, cols, rows));
        }
        let total = ROWS.len();
        let view = (rows.saturating_sub(1) as usize).clamp(1, total);
        self.max_offset = total - view;
        self.offset = self.offset.min(self.max_offset);
        let vp = Viewport {
            cols,
            rows,
            view,
            offset: self.offset,
        };
        debug::note(
            "frame",
            format!(
                "offset={} view={view} refresh={refresh} tx={}",
                self.offset, self.transmits
            )
            .as_bytes(),
        );
        let mut stats = draw_frame(out, protocol, cell, &vp, refresh)?;
        if stats.sent_image {
            self.transmits += 1;
        }
        stats.frame = self.prev_frame;
        draw_status(out, protocol, cell, &vp, &stats, self.transmits, gfx_err)?;
        out.write_all(b"\x1b[?2026l")?; // end synchronized update
        out.flush()?;
        self.prev_frame = ms(start);
        Ok(self.prev_frame)
    }

    fn down(&mut self) {
        self.offset = (self.offset + 1).min(self.max_offset);
    }

    fn up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }
}

fn event_loop(protocol: Protocol, cell: &CellSize) -> io::Result<()> {
    let mut out = debug::Tee::new(io::BufWriter::new(io::stdout()));
    let mut frames = Frames::new();
    // Last graphics error the terminal reported (kitty commands run at
    // q=1: silent on success, APC reply on failure). Shown in the status
    // line — the q=2 of the original spike swallowed these, which is how
    // "blank graph in Ghostty" shipped as "done".
    let mut gfx_err: Option<String> = None;
    loop {
        frames.render(&mut out, protocol, cell, gfx_err.as_deref())?;
        match term::read_key() {
            Key::Quit => return Ok(()),
            Key::Down => frames.down(),
            Key::Up => frames.up(),
            Key::Graphics(reply) => {
                debug::note("graphics error reply", &reply);
                gfx_err = Some(term::reply_message(&reply));
            }
            Key::Other => {}
        }
    }
}

/// `GRAPH_SPIKE_BENCH=<frames>`: scroll without a keyboard and report what
/// the frames cost.
///
/// Holding `j` down *is* this — one scroll step per frame, as fast as the
/// terminal will take them — so it measures the case the frame budget is
/// about. It has to run inside the real terminal: a pty drains instantly
/// and hides the write cost that dominates a real frame, and a pty answers
/// no graphics command, so it can't tell a drawn frame from a rejected one.
/// The report is written to `$GRAPH_SPIKE_BENCH_OUT` (default
/// `graph-spike-bench.txt`) because the window is usually gone by the time
/// anyone could read stdout.
fn bench(protocol: Protocol, cell: &CellSize, count: usize) -> io::Result<()> {
    let mut out = debug::Tee::new(io::BufWriter::new(io::stdout()));
    let mut frames = Frames::new();
    let mut times = Vec::with_capacity(count);
    let mut errors: Vec<String> = Vec::new();
    let mut descending = true;
    // A key repeat delivers ~30 keys a second, so an unpaced burst of 300
    // frames is over in a tenth of a second and can only catch what goes
    // wrong instantly. Pacing spreads the run over real seconds, which is
    // where a drifting terminal size or a periodic retransmit would show.
    let gap = bench_gap();
    for _ in 0..count {
        let frame_start = Instant::now();
        let last_err = errors.last().cloned();
        times.push(frames.render(&mut out, protocol, cell, last_err.as_deref())?);
        // Errors arrive as APC replies on stdin (q=1). Draining them every
        // frame is the point: a frame that the terminal rejected costs
        // almost nothing, so a fast average with hidden ENOENTs is exactly
        // the bug ADR-0012 recorded.
        collect_graphics_errors(&mut errors);
        if descending && frames.offset == frames.max_offset {
            descending = false;
        } else if !descending && frames.offset == 0 {
            descending = true;
        }
        if descending {
            frames.down();
        } else {
            frames.up();
        }
        if let Some(left) = gap.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(left);
        }
    }
    // Replies are asynchronous; give the last frames' errors time to land.
    std::thread::sleep(Duration::from_millis(200));
    collect_graphics_errors(&mut errors);
    let (cols, rows) = terminal::size()?;
    let report = bench_report(protocol, cell, (cols, rows), &frames, &times, &errors);
    let path = std::env::var("GRAPH_SPIKE_BENCH_OUT")
        .unwrap_or_else(|_| "graph-spike-bench.txt".to_string());
    std::fs::write(path, report)
}

fn collect_graphics_errors(errors: &mut Vec<String>) {
    for reply in term::pending_graphics_replies() {
        debug::note("graphics error reply", &reply);
        errors.push(term::reply_message(&reply));
    }
}

fn bench_report(
    protocol: Protocol,
    cell: &CellSize,
    size: (u16, u16),
    frames: &Frames,
    times: &[f64],
    errors: &[String],
) -> String {
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    let pick = |q: f64| sorted[((sorted.len() - 1) as f64 * q) as usize];
    let over = times.iter().filter(|&&t| t > 16.0).count();
    let provenance = if cell.reported { "reported" } else { "assumed" };
    format!(
        "protocol      {}\n\
terminal      {}x{} cells, cell {}x{}px ({provenance})\n\
dataset       {} rows, view {} rows, scroll range {}\n\
frames        {}\n\
transmits     {}\n\
graphics err  {} {:?}\n\
frame ms      first {:.1} | min {:.1} med {:.1} p95 {:.1} max {:.1}\n\
over 16ms     {} of {}\n\
retransmits   {:?}  (frame, cols, rows)\n\
steady ms     min {:.1} med {:.1} max {:.1}  (frames 2..)\n",
        protocol.name(),
        size.0,
        size.1,
        cell.w,
        cell.h,
        ROWS.len(),
        ROWS.len() - frames.max_offset,
        frames.max_offset,
        times.len(),
        frames.transmits,
        errors.len(),
        errors,
        times.first().copied().unwrap_or(0.0),
        pick(0.0),
        pick(0.5),
        pick(0.95),
        pick(1.0),
        over,
        times.len(),
        frames.refreshes,
        steady(times, f64::min),
        median(&times[1.min(times.len())..]),
        steady(times, f64::max),
    )
}

/// Fold over every frame but the first, which pays for the initial upload.
fn steady(times: &[f64], f: impl Fn(f64, f64) -> f64) -> f64 {
    times[1.min(times.len())..]
        .iter()
        .copied()
        .fold(f64::NAN, f)
}

fn median(times: &[f64]) -> f64 {
    if times.is_empty() {
        return f64::NAN;
    }
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}

/// Draw the graph image and the text rows for one frame. Leaves the
/// synchronized update open so the status line joins the same frame.
///
/// `ESC[2J` runs only on `refresh` — the first frame and a terminal-size
/// change — never in the steady state. On Kitty that is not a nicety:
/// Ghostty frees stored image *data* on ED, so a clear between transmit
/// and placement makes every later placement answer `ENOENT` and the graph
/// vanishes (verified against Ghostty 1.3.1 at `q=0`; ADR-0012
/// correction). That is why a clear and a retransmit are the same event.
///
/// A scroll frame therefore costs one placement: Kitty *replaces* the
/// single placement `p=1` with a new crop of the strip already in the
/// terminal, and the text rows erase only their own tails. Measured in
/// Ghostty 1.3.1: 0.2 ms median against ~32 ms for a frame that
/// retransmits.
///
/// iTerm2 has no image ids and no crops, so its band is re-rasterized and
/// re-encoded every frame; it skips the clear too, drawing the new image
/// over the same cell rectangle. Confirmed to draw in a real iTerm2 by the
/// user, 2026-09-12 (ADR-0012); its frame cost with write back-pressure
/// included is still unmeasured there.
fn draw_frame(
    out: &mut impl Write,
    protocol: Protocol,
    cell: &CellSize,
    vp: &Viewport,
    refresh: bool,
) -> io::Result<Stats> {
    let mut stats = Stats::default();
    out.write_all(b"\x1b[?2026h")?; // begin synchronized update
    if refresh {
        out.write_all(b"\x1b[2J")?;
    }
    match protocol {
        Protocol::Kitty => {
            if refresh {
                let t = Instant::now();
                let full = rasterize(0, ROWS.len(), cell);
                stats.raster = ms(t);
                let t = Instant::now();
                term::kitty_transmit(out, full.rgba(), full.w, full.h)?;
                stats.upload = ms(t);
                stats.sent_image = true;
            }
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
            stats.sent_image = true;
        }
    }
    draw_text(out, vp)?;
    out.flush()?;
    Ok(stats)
}

/// One text line per commit, at a fixed column right of the reserved graph
/// columns. Every write starts with an absolute cursor move — never trust
/// where an image emission left the cursor. Each row ends with an
/// erase-to-end-of-line so the previous frame's longer text can't linger:
/// that erase starts right of the text, so it can never touch the graph
/// columns (erasing cells under a kitty image deletes it in Ghostty).
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
        out.write_all(b"\x1b[0m\x1b[K")?;
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

/// Inverse-video status line on the terminal's last row: graphics health,
/// protocol, cell size and its provenance, scroll position, frame timings,
/// keys.
fn draw_status(
    out: &mut impl Write,
    protocol: Protocol,
    cell: &CellSize,
    vp: &Viewport,
    stats: &Stats,
    transmits: u32,
    gfx_err: Option<&str>,
) -> io::Result<()> {
    let provenance = if cell.reported { "reported" } else { "assumed" };
    // Named by its state, not by its worst state: `gfx ok` while the
    // terminal has reported nothing, `gfx: <error>` once it has. It stays
    // visible in both, and stays first so a narrow window truncates the
    // tail instead of the one field that says whether the pixels landed.
    // Always-visible matters here specifically: the bug this spike shipped
    // was silence mistaken for success (q=2 swallowed every ENOENT), and a
    // field that only appears on failure is indistinguishable from a
    // reporting path that has died. `ok` means "asked at q=1, nothing came
    // back" — not "a human has seen the graph".
    let gfx = gfx_err.map_or_else(|| "gfx ok".to_string(), |e| format!("gfx: {e}"));
    let mut text = format!(
        " {gfx} | {} | cell {}x{}px ({provenance}) | rows {}-{}/{} | frame {:.1}ms \
         (raster {:.1} encode {:.1} upload {:.1}) | tx {transmits} | j/k scroll, q quit",
        protocol.name(),
        cell.w,
        cell.h,
        vp.offset + 1,
        vp.offset + vp.view,
        ROWS.len(),
        stats.frame,
        stats.raster,
        stats.encode,
        stats.upload,
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
