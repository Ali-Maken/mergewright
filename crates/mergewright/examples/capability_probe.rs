//! Manual check for capability detection (PROMPT.md §7.1, DESIGN.md §4).
//!
//! The unit tests drive detection against scripted replies, which proves the
//! *decisions* and nothing about real terminals: whether Ghostty really acks
//! the Kitty query, whether Terminal.app really answers `CSI 16 t`, whether a
//! tmux with `allow-passthrough` really forwards anything. Only a binary in a
//! real terminal answers those, so this example runs the real probe and prints
//! what came back:
//!
//! ```text
//! cargo run -p mergewright --example capability_probe
//! ```
//!
//! Run it in each terminal on the checklist, and inside tmux and over SSH.
//! Nothing is drawn and the alternate screen is not used, so the report stays
//! on screen afterwards.

use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Instant;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use mergewright_core::capabilities::{Capabilities, CellSizeSource, RenderingTier, ThemeSource};
use mergewright_term::detect::{Environment, TtyHost, detect};
use mergewright_term::input::InputMux;

fn main() -> ExitCode {
    let environment = Environment::from_process();

    // Raw mode, and nothing else: in cooked mode the terminal's replies sit in
    // the line buffer until Return is pressed, so every query "times out".
    // Skipped when there is no terminal — detection skips the probe too, and
    // running with the output piped is one of the cases worth seeing.
    let raw_mode = environment.stdout_is_tty;
    if raw_mode && let Err(error) = enable_raw_mode() {
        eprintln!("capability_probe: could not enter raw mode: {error}");
        return ExitCode::FAILURE;
    }
    let mut host = if raw_mode {
        match InputMux::on_stdin() {
            Ok((mux, _inputs)) => TtyHost::new(mux.waiter()),
            Err(error) => {
                eprintln!("capability_probe: could not read the terminal: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        TtyHost::unread()
    };
    let started = Instant::now();
    let capabilities = detect(&environment, &mut host);
    let elapsed = started.elapsed();
    if raw_mode && let Err(error) = disable_raw_mode() {
        eprintln!("capability_probe: could not leave raw mode: {error}");
        return ExitCode::FAILURE;
    }

    let mut out = io::stdout();
    match report(&mut out, &environment, &capabilities, elapsed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("capability_probe: could not write the report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn report(
    out: &mut impl Write,
    environment: &Environment,
    capabilities: &Capabilities,
    elapsed: std::time::Duration,
) -> io::Result<()> {
    writeln!(out, "env      {environment:?}")?;
    writeln!(out)?;
    writeln!(out, "tier     {:?}", capabilities.tier)?;
    // The protocol is reported whatever the tier is (ADR-0016): a reachable
    // one that Unicode outranks is exactly what this report exists to show.
    writeln!(
        out,
        "protocol {}",
        match (capabilities.pixel_protocol, capabilities.tier) {
            (Some(protocol), RenderingTier::Pixel) => format!("{protocol:?}, drawing"),
            (Some(protocol), _) => format!("{protocol:?}, reachable but not chosen (ADR-0016)"),
            (None, _) => "none reachable".to_owned(),
        }
    )?;
    writeln!(out, "probe    {:?} in {elapsed:?}", capabilities.probe)?;
    writeln!(out, "budget   {:?}", capabilities.probe_budget)?;
    writeln!(out, "kitty    {:?}", capabilities.kitty_graphics)?;
    writeln!(out, "sixel    {:?}", capabilities.sixel)?;
    writeln!(out, "iterm2   {}", capabilities.iterm2_images)?;
    writeln!(out, "keyboard {:?}", capabilities.kitty_keyboard)?;
    writeln!(out, "sync     {:?}", capabilities.synchronized_output)?;
    let cell = capabilities.cell_size;
    writeln!(
        out,
        "cell     {}x{} ({})",
        cell.width,
        cell.height,
        match cell.source {
            CellSizeSource::Reported => "reported",
            CellSizeSource::WindowSize => "from the window size",
            CellSizeSource::Assumed => "assumed, not measured",
        }
    )?;
    writeln!(
        out,
        "theme    {:?} ({})",
        capabilities.theme.theme,
        match capabilities.theme.source {
            ThemeSource::Background(background) => format!(
                "background #{:02x}{:02x}{:02x}",
                background.red, background.green, background.blue
            ),
            ThemeSource::Assumed => "assumed, not reported".to_owned(),
        }
    )?;
    writeln!(out, "colour   {:?}", capabilities.color_depth)?;
    writeln!(out, "no_color {}", capabilities.no_color)?;
    writeln!(out, "tmux     {:?}", capabilities.tmux)?;
    writeln!(out, "ssh      {}", capabilities.over_ssh)?;
    out.flush()
}
