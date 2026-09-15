//! Detection against scripted terminals.
//!
//! The fake never sleeps: silence is `Ok(0)` straight away, and the deadline
//! it was handed is recorded instead. So the budget rules of DESIGN.md §4.4
//! are asserted exactly — `deadline - start == 500 ms` over SSH — rather than
//! approximated by a test that waits half a second to watch a clock.

use std::collections::VecDeque;
use std::io;

use graft_core::capabilities::{ColorDepth, PixelProtocol, Rgb};

use super::environment::version;
use super::*;

/// A terminal that answers everything: Ghostty's replies, byte for byte, as
/// captured in the graph spike (ADR-0012).
const ANSWERS_EVERYTHING: &[u8] = b"\x1b_Gi=31;OK\x1b\\\x1b[?0u\x1b[6;37;17t\x1b[?2026;2$y\x1b]11;rgb:1d1d/1f1f/2121\x1b\\\x1b[?62;22c";
/// A terminal with no graphics and nothing else to say: just the fence.
const ONLY_THE_FENCE: &[u8] = b"\x1b[?62;22c";
/// `DA1` with parameter 4: Sixel, and a reported cell size.
const SIXEL_ONLY: &[u8] = b"\x1b[6;30;14t\x1b[?62;4;22c";
/// The graphics round coming back through tmux passthrough.
const GRAPHICS_THROUGH_TMUX: &[u8] = b"\x1b_Gi=31;OK\x1b\\\x1b[?62;22c";
/// A round the terminal never answers at all: no replies, no fence.
const SAYS_NOTHING: &[u8] = b"";

/// A terminal that replies from a script: one entry per probe round, in the
/// order the rounds go out. An unscripted round is a silent one.
struct FakeHost {
    rounds: VecDeque<Vec<u8>>,
    pending: Vec<u8>,
    written: Vec<Vec<u8>>,
    deadlines: Vec<Instant>,
    window: Option<WindowPixels>,
    tmux_version: Option<String>,
    /// Which write fails, counting from zero. One round per write, so this
    /// picks the round: `Some(1)` is a terminal that took the first batch and
    /// broke before the tmux re-probe.
    write_failure_at: Option<usize>,
}

impl FakeHost {
    fn answering(rounds: &[&[u8]]) -> Self {
        Self {
            rounds: rounds.iter().map(|round| round.to_vec()).collect(),
            pending: Vec::new(),
            written: Vec::new(),
            deadlines: Vec::new(),
            window: None,
            tmux_version: None,
            write_failure_at: None,
        }
    }

    fn silent() -> Self {
        Self::answering(&[])
    }

    fn with_window(mut self, columns: u16, rows: u16, width: u16, height: u16) -> Self {
        self.window = Some(WindowPixels {
            columns,
            rows,
            width,
            height,
        });
        self
    }

    fn with_tmux_version(mut self, reported: &str) -> Self {
        self.tmux_version = Some(reported.to_owned());
        self
    }

    fn failing_to_write(mut self) -> Self {
        self.write_failure_at = Some(0);
        self
    }

    fn failing_to_write_on_round(mut self, round: usize) -> Self {
        self.write_failure_at = Some(round);
        self
    }

    /// The furthest deadline any read was given, as an offset from `start`.
    fn longest_wait(&self, start: Instant) -> Duration {
        self.deadlines
            .iter()
            .map(|deadline| deadline.saturating_duration_since(start))
            .max()
            .unwrap_or_default()
    }
}

impl TerminalHost for FakeHost {
    fn write_probe(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.written.push(bytes.to_vec());
        if self.write_failure_at == Some(self.written.len() - 1) {
            return Err(io::Error::other("scripted write failure"));
        }
        // A round with no script is a terminal that says nothing back.
        self.pending = self.rounds.pop_front().unwrap_or_default();
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8], deadline: Instant) -> io::Result<usize> {
        self.deadlines.push(deadline);
        if self.pending.is_empty() {
            return Ok(0);
        }
        let taken = buffer.len().min(self.pending.len());
        buffer[..taken].copy_from_slice(&self.pending[..taken]);
        self.pending.drain(..taken);
        Ok(taken)
    }

    fn window_pixels(&mut self) -> Option<WindowPixels> {
        self.window
    }

    fn tmux_version(&mut self) -> Option<String> {
        self.tmux_version.clone()
    }
}

/// Whether a recorded deadline is the given budget and not some other one.
/// The slack covers only the work between taking `start` and computing the
/// deadline; nothing in these tests waits, so it never approaches it.
fn within_budget(waited: Duration, budget: Duration) -> bool {
    let generous = waited >= budget && waited < budget + Duration::from_millis(50);
    assert!(generous, "waited {waited:?}, budget {budget:?}");
    generous
}

/// A plain 256-colour terminal, on a tty, outside tmux and SSH.
fn terminal() -> Environment {
    Environment {
        term: Some("xterm-256color".to_owned()),
        stdout_is_tty: true,
        ..Environment::default()
    }
}

fn inside_tmux() -> Environment {
    Environment {
        tmux: Some("/tmp/tmux-501/default,1234,0".to_owned()),
        ..terminal()
    }
}

#[test]
fn a_terminal_that_answers_everything_gets_the_pixel_tier() {
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]);

    // One assertion for the whole contract: every field detection produces,
    // from one round of real captured bytes.
    assert_eq!(
        detect(&terminal(), &mut host),
        Capabilities {
            tier: RenderingTier::Pixel,
            pixel_protocol: Some(PixelProtocol::Kitty),
            kitty_graphics: Supported::Yes,
            sixel: Supported::No,
            iterm2_images: false,
            kitty_keyboard: Supported::Yes,
            synchronized_output: Supported::Yes,
            cell_size: CellSize {
                width: 17,
                height: 37,
                source: CellSizeSource::Reported,
            },
            theme: ThemeHint {
                theme: Theme::Dark,
                source: ThemeSource::Background(Rgb {
                    red: 0x1d,
                    green: 0x1f,
                    blue: 0x21,
                }),
            },
            color_depth: ColorDepth::Palette256,
            no_color: false,
            is_tty: true,
            over_ssh: false,
            tmux: TmuxStatus::Outside,
            probe: ProbeOutcome::Answered,
            probe_budget: LOCAL_PROBE_BUDGET,
        }
    );
}

#[test]
fn a_silent_terminal_falls_back_per_query_and_says_so() {
    let mut host = FakeHost::silent();

    let capabilities = detect(&terminal(), &mut host);

    assert_eq!(capabilities.probe, ProbeOutcome::TimedOut);
    assert_eq!(capabilities.tier, RenderingTier::Ascii);
    assert_eq!(capabilities.kitty_graphics, Supported::Silent);
    assert_eq!(capabilities.sixel, Supported::Silent);
    assert_eq!(capabilities.kitty_keyboard, Supported::Silent);
    assert_eq!(capabilities.synchronized_output, Supported::Silent);
    // Still usable values, each carrying the fact that nobody measured it.
    assert_eq!(
        capabilities.cell_size,
        CellSize {
            width: 8,
            height: 16,
            source: CellSizeSource::Assumed,
        }
    );
    assert_eq!(
        capabilities.theme,
        ThemeHint {
            theme: Theme::Dark,
            source: ThemeSource::Assumed,
        }
    );
}

/// Silence that came back fenced is an answer: the terminal was listening and
/// did not implement the query. That is a different fact from a timeout, and
/// the two are never collapsed.
#[test]
fn silence_inside_a_fenced_round_counts_as_a_no() {
    let mut host = FakeHost::answering(&[ONLY_THE_FENCE]);

    let capabilities = detect(&terminal(), &mut host);

    assert_eq!(capabilities.probe, ProbeOutcome::Answered);
    assert_eq!(capabilities.kitty_graphics, Supported::No);
    assert_eq!(capabilities.sixel, Supported::No);
    assert_eq!(capabilities.kitty_keyboard, Supported::No);
    assert_eq!(capabilities.synchronized_output, Supported::No);
}

#[test]
fn a_silent_terminal_is_given_no_longer_than_the_local_budget() {
    let start = Instant::now();
    let mut host = FakeHost::silent();

    let capabilities = detect(&terminal(), &mut host);

    assert_eq!(capabilities.probe_budget, LOCAL_PROBE_BUDGET);
    assert!(within_budget(host.longest_wait(start), LOCAL_PROBE_BUDGET));
}

#[test]
fn ssh_buys_the_terminal_an_extra_round_trip() {
    let start = Instant::now();
    let environment = Environment {
        over_ssh: true,
        ..terminal()
    };
    let mut host = FakeHost::silent();

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.probe_budget, SSH_PROBE_BUDGET);
    assert!(capabilities.over_ssh);
    assert!(within_budget(host.longest_wait(start), SSH_PROBE_BUDGET));
}

#[test]
fn the_kernel_window_size_answers_when_the_terminal_will_not() {
    let mut host = FakeHost::silent().with_window(100, 30, 1600, 600);

    assert_eq!(
        detect(&terminal(), &mut host).cell_size,
        CellSize {
            width: 16,
            height: 20,
            source: CellSizeSource::WindowSize,
        }
    );
}

/// Many terminals fill in the character grid and leave the pixel fields at
/// zero. That is not a size, and dividing by it would be a lie (or a panic).
#[test]
fn a_window_size_without_pixels_is_not_a_cell_size() {
    let mut host = FakeHost::silent().with_window(100, 30, 0, 0);

    assert_eq!(
        detect(&terminal(), &mut host).cell_size.source,
        CellSizeSource::Assumed
    );
}

#[test]
fn what_the_terminal_reports_beats_what_the_kernel_infers() {
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]).with_window(100, 30, 1600, 600);

    assert_eq!(
        detect(&terminal(), &mut host).cell_size,
        CellSize {
            width: 17,
            height: 37,
            source: CellSizeSource::Reported,
        }
    );
}

/// A Sixel-only terminal with no truecolor — here, a 256-colour `xterm` —
/// draws ASCII. ADR-0016 ranks Sixel above ASCII, but keeps no Sixel encoder,
/// so a pixel tier would have nothing behind it. The protocol is still
/// reported: it is a fact about the terminal.
#[test]
fn a_sixel_only_terminal_without_truecolor_draws_ascii_not_an_empty_pixel_tier() {
    let mut host = FakeHost::answering(&[SIXEL_ONLY]);

    let capabilities = detect(&terminal(), &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Ascii);
    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Sixel));
    assert_eq!(capabilities.sixel, Supported::Yes);
    assert_eq!(capabilities.kitty_graphics, Supported::No);
}

/// The same terminal with truecolor draws Unicode instead: an unproven image
/// protocol does not displace the tier the user actually reads (ADR-0016).
#[test]
fn sixel_does_not_take_the_tier_from_unicode() {
    let environment = Environment {
        colorterm: Some("truecolor".to_owned()),
        ..terminal()
    };
    let mut host = FakeHost::answering(&[SIXEL_ONLY]);

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Unicode);
    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Sixel));
}

/// Kitty is the one protocol that outranks Unicode, and it does so on a
/// terminal that would otherwise have drawn characters (ADR-0016).
#[test]
fn kitty_takes_the_tier_from_unicode() {
    let environment = Environment {
        colorterm: Some("truecolor".to_owned()),
        ..terminal()
    };
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]);

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Pixel);
    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Kitty));
}

/// DESIGN.md §4.2: Kitty first when more than one protocol answers.
#[test]
fn kitty_wins_over_sixel_when_both_answer() {
    let mut host = FakeHost::answering(&[b"\x1b_Gi=31;OK\x1b\\\x1b[?62;4;22c"]);

    let capabilities = detect(&terminal(), &mut host);

    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Kitty));
    assert_eq!(capabilities.sixel, Supported::Yes);
}

/// The one capability allowed to rest on an environment variable, because
/// iTerm2 offers nothing to ask (DESIGN.md §4.1 step 4).
///
/// It is reported, and it does not take the tier: ADR-0016 ranks iTerm2 below
/// Unicode until a checklist row says its pixels beat one. The protocol is on
/// the struct anyway, because `graph.renderer = "pixel"` has to know what to
/// reach for.
#[test]
fn iterm2_is_taken_from_the_environment_alone_and_does_not_take_the_tier() {
    let environment = Environment {
        term_program: Some("iTerm.app".to_owned()),
        term_program_version: Some("3.5.11".to_owned()),
        ..terminal()
    };
    let mut host = FakeHost::answering(&[ONLY_THE_FENCE]);

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Unicode);
    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Iterm2));
    assert!(capabilities.iterm2_images);
    assert_eq!(capabilities.kitty_graphics, Supported::No);
}

#[test]
fn an_iterm2_that_does_not_report_its_version_is_not_trusted_with_images() {
    let environment = Environment {
        term_program: Some("iTerm.app".to_owned()),
        ..terminal()
    };
    let mut host = FakeHost::answering(&[ONLY_THE_FENCE]);

    let capabilities = detect(&environment, &mut host);

    assert!(!capabilities.iterm2_images);
    assert_eq!(capabilities.tier, RenderingTier::Unicode);
}

#[test]
fn tmux_too_old_to_pass_anything_through_falls_back_to_unicode() {
    let environment = Environment {
        term_program: Some("tmux".to_owned()),
        term_program_version: Some("3.2a".to_owned()),
        ..inside_tmux()
    };
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]);

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Unicode);
    assert_eq!(
        capabilities.tmux,
        TmuxStatus::TooOld {
            version: "3.2a".to_owned()
        }
    );
    // Whatever answered the first round was tmux, not the terminal that would
    // have to draw the image, so it says nothing about graphics.
    assert_eq!(capabilities.kitty_graphics, Supported::Silent);
    assert_eq!(capabilities.sixel, Supported::Silent);
    assert_eq!(
        host.written.len(),
        1,
        "no point re-probing through old tmux"
    );
}

#[test]
fn tmux_with_passthrough_reaches_the_outer_terminal() {
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING, GRAPHICS_THROUGH_TMUX])
        .with_tmux_version("tmux 3.4\n");

    let capabilities = detect(&inside_tmux(), &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Pixel);
    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Kitty));
    assert_eq!(
        capabilities.tmux,
        TmuxStatus::Passthrough {
            version: "3.4".to_owned()
        }
    );
    // The re-probe goes out wrapped, or the outer terminal never sees it.
    assert_eq!(
        host.written.get(1).map(Vec::as_slice),
        Some(probe::tmux_passthrough(&probe::graphics_round()).as_slice())
    );
}

#[test]
fn tmux_that_forwards_nothing_back_falls_back_to_unicode() {
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]).with_tmux_version("tmux 3.4");

    let capabilities = detect(&inside_tmux(), &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Unicode);
    assert_eq!(
        capabilities.tmux,
        TmuxStatus::PassthroughSilent {
            version: "3.4".to_owned()
        }
    );
    assert_eq!(capabilities.kitty_graphics, Supported::Silent);
    assert_eq!(host.written.len(), 2, "the wrapped re-probe still goes out");
}

/// The M0 review's Minor 2. The tmux re-probe re-asks only the graphics
/// queries, so its failure is evidence about graphics and about nothing else.
/// A fenced first round already established that this terminal has neither the
/// Kitty keyboard protocol nor DEC 2026 — a spoken "no", not silence — and the
/// second round's failure must not walk that back.
#[test]
fn a_failed_tmux_reprobe_does_not_unanswer_the_first_round() {
    let mut host = FakeHost::answering(&[ONLY_THE_FENCE])
        .with_tmux_version("tmux 3.4")
        .failing_to_write_on_round(1);

    let capabilities = detect(&inside_tmux(), &mut host);

    assert_eq!(capabilities.kitty_keyboard, Supported::No);
    assert_eq!(capabilities.synchronized_output, Supported::No);
    // The failure itself is still reported: it is a fact about this terminal.
    let ProbeOutcome::Failed(message) = &capabilities.probe else {
        panic!("expected a reported failure, got {:?}", capabilities.probe);
    };
    assert!(message.contains("scripted write failure"), "{message}");
    // And graphics, which the second round *was* asking about, stay unproven.
    assert_eq!(capabilities.kitty_graphics, Supported::Silent);
    assert_eq!(capabilities.tier, RenderingTier::Unicode);
}

/// The M0 review's Minor 2, the other half. An outer terminal can stay silent
/// unfenced and answer through tmux's passthrough — rare, but then the tier
/// *was* measured, and the status screen must not read "pixel, assumed
/// (terminal silent)" beside it.
#[test]
fn a_tmux_reprobe_that_answers_is_not_reported_as_silence() {
    let mut host =
        FakeHost::answering(&[SAYS_NOTHING, GRAPHICS_THROUGH_TMUX]).with_tmux_version("tmux 3.4");

    let capabilities = detect(&inside_tmux(), &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Pixel);
    assert_eq!(capabilities.pixel_protocol, Some(PixelProtocol::Kitty));
    assert_eq!(capabilities.probe, ProbeOutcome::Answered);
    // Round two re-asks only the graphics queries, so the two the silent first
    // round asked about are still unproven. Answered is about the tier.
    assert_eq!(capabilities.kitty_keyboard, Supported::Silent);
    assert_eq!(capabilities.synchronized_output, Supported::Silent);
}

/// And the reverse still holds: a silent first round with a tmux that forwards
/// nothing stays silence, because nothing measured anything.
#[test]
fn two_silent_rounds_are_still_silence() {
    let mut host = FakeHost::answering(&[SAYS_NOTHING]).with_tmux_version("tmux 3.4");

    let capabilities = detect(&inside_tmux(), &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Unicode);
    assert_eq!(capabilities.probe, ProbeOutcome::TimedOut);
}

/// An unknown version is treated as too old. Hoping and drawing a broken
/// screen is exactly what §4.3 rules out.
#[test]
fn tmux_of_unknown_version_is_treated_as_too_old() {
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]);

    let capabilities = detect(&inside_tmux(), &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Unicode);
    assert_eq!(capabilities.tmux, TmuxStatus::VersionUnknown);
    assert_eq!(host.written.len(), 1);
}

/// Even a truecolor-less tmux gets Unicode: §4.3 is the more specific rule,
/// and tmux draws box characters whatever its colour depth.
#[test]
fn tmux_does_not_fall_all_the_way_to_ascii() {
    let environment = Environment {
        term: Some("screen".to_owned()),
        ..inside_tmux()
    };
    let mut host = FakeHost::silent();

    assert_eq!(detect(&environment, &mut host).tier, RenderingTier::Unicode);
}

#[test]
fn iterm2_inline_images_do_not_survive_a_tmux_that_blocks_them() {
    let environment = Environment {
        term_program: Some("iTerm.app".to_owned()),
        term_program_version: Some("3.5.11".to_owned()),
        ..inside_tmux()
    };
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]).with_tmux_version("tmux 3.4");

    let capabilities = detect(&environment, &mut host);

    assert!(!capabilities.iterm2_images);
    assert_eq!(capabilities.tier, RenderingTier::Unicode);
}

#[test]
fn output_that_is_not_a_terminal_is_never_probed() {
    let environment = Environment {
        stdout_is_tty: false,
        ..terminal()
    };
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]);

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Ascii);
    assert_eq!(capabilities.probe, ProbeOutcome::Skipped);
    assert_eq!(capabilities.color_depth, ColorDepth::None);
    assert!(!capabilities.is_tty);
    assert_eq!(
        detect(
            &Environment {
                stdout_is_tty: false,
                ..inside_tmux()
            },
            &mut FakeHost::silent()
        )
        .tmux,
        TmuxStatus::VersionUnknown,
        "reporting `Outside` from inside tmux would be a plain untruth"
    );
    // Probe bytes down a pipe would land in whatever the user redirected to.
    assert!(host.written.is_empty());
}

#[test]
fn a_dumb_terminal_is_never_probed() {
    let environment = Environment {
        term: Some("dumb".to_owned()),
        ..terminal()
    };
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]);

    let capabilities = detect(&environment, &mut host);

    assert_eq!(capabilities.tier, RenderingTier::Ascii);
    assert_eq!(capabilities.probe, ProbeOutcome::Skipped);
    assert_eq!(capabilities.color_depth, ColorDepth::None);
    assert!(host.written.is_empty());
}

/// A terminal Graft cannot write to is reported, not quietly turned into an
/// unexplainable ASCII session.
#[test]
fn a_write_failure_is_reported_rather_than_swallowed() {
    let mut host = FakeHost::answering(&[ANSWERS_EVERYTHING]).failing_to_write();

    let capabilities = detect(&terminal(), &mut host);

    let ProbeOutcome::Failed(message) = &capabilities.probe else {
        panic!("expected a reported failure, got {:?}", capabilities.probe);
    };
    assert!(message.contains("scripted write failure"), "{message}");
    assert_eq!(capabilities.kitty_graphics, Supported::Silent);
    assert_eq!(capabilities.cell_size.source, CellSizeSource::Assumed);
}

#[test]
fn no_color_is_recorded_without_touching_the_tier() {
    let environment = Environment {
        colorterm: Some("truecolor".to_owned()),
        no_color: true,
        ..terminal()
    };
    let mut host = FakeHost::silent();

    let capabilities = detect(&environment, &mut host);

    assert!(capabilities.no_color);
    assert_eq!(capabilities.color_depth, ColorDepth::TrueColor);
    assert_eq!(capabilities.tier, RenderingTier::Unicode);
}

#[test]
fn colour_depth_comes_from_what_the_environment_names() {
    let depth = |term: &str, colorterm: Option<&str>| {
        Environment {
            term: Some(term.to_owned()),
            colorterm: colorterm.map(str::to_owned),
            ..terminal()
        }
        .color_depth()
    };

    assert_eq!(
        depth("xterm-256color", Some("truecolor")),
        ColorDepth::TrueColor
    );
    assert_eq!(depth("xterm-direct", None), ColorDepth::TrueColor);
    assert_eq!(depth("xterm-256color", None), ColorDepth::Palette256);
    assert_eq!(depth("xterm", None), ColorDepth::Palette16);
}

#[test]
fn a_light_background_gives_the_light_theme() {
    let mut host = FakeHost::answering(&[b"\x1b]11;rgb:ffff/ffff/f0f0\x1b\\\x1b[?62;22c"]);

    let capabilities = detect(&terminal(), &mut host);

    assert_eq!(capabilities.theme.theme, Theme::Light);
    assert_eq!(
        capabilities.theme.source,
        ThemeSource::Background(Rgb {
            red: 0xff,
            green: 0xff,
            blue: 0xf0,
        })
    );
}

#[test]
fn versions_parse_from_whatever_shape_they_arrive_in() {
    assert_eq!(version("3.4"), Some((3, 4)));
    assert_eq!(version("tmux 3.5a"), Some((3, 5)));
    assert_eq!(version("next-3.6"), Some((3, 6)));
    assert_eq!(version("3.5.0-beta"), Some((3, 5)));
    assert_eq!(version("3"), Some((3, 0)));
    assert_eq!(version(""), None);
    assert_eq!(version("openbsd"), None);
}

mod redetection {
    use graft_core::capabilities::{CellSize, CellSizeSource, Rgb, Theme, ThemeHint, ThemeSource};

    use super::*;

    const BUDGET: Duration = Duration::from_millis(200);

    fn measured() -> CellSize {
        CellSize {
            width: 9,
            height: 18,
            source: CellSizeSource::Reported,
        }
    }

    fn dark_background() -> ThemeHint {
        ThemeHint {
            theme: Theme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0x1d,
                green: 0x1f,
                blue: 0x21,
            }),
        }
    }

    #[test]
    fn a_resize_asks_only_for_the_cell_size_and_the_fence() {
        let mut host = FakeHost::silent();

        remeasure_cell_size(&mut host, measured(), BUDGET).expect("a fake cannot fail");

        assert_eq!(host.written, vec![b"\x1b[16t\x1b[c".to_vec()]);
    }

    #[test]
    fn a_reported_cell_size_replaces_the_previous_one() {
        let mut host = FakeHost::answering(&[b"\x1b[6;40;20t\x1b[?62;22c"]);

        let size = remeasure_cell_size(&mut host, measured(), BUDGET).expect("a fake cannot fail");

        assert_eq!(
            size,
            CellSize {
                width: 20,
                height: 40,
                source: CellSizeSource::Reported,
            }
        );
    }

    #[test]
    fn an_unanswered_query_falls_back_to_the_window_size() {
        let mut host = FakeHost::answering(&[ONLY_THE_FENCE]).with_window(100, 50, 1100, 1100);

        let size = remeasure_cell_size(&mut host, measured(), BUDGET).expect("a fake cannot fail");

        assert_eq!(
            size,
            CellSize {
                width: 11,
                height: 22,
                source: CellSizeSource::WindowSize,
            }
        );
    }

    #[test]
    fn a_silent_resize_keeps_the_previous_size_but_marks_it_assumed() {
        let mut host = FakeHost::silent();

        let size = remeasure_cell_size(&mut host, measured(), BUDGET).expect("a fake cannot fail");

        assert_eq!(
            size,
            CellSize {
                source: CellSizeSource::Assumed,
                ..measured()
            }
        );
    }

    #[test]
    fn a_window_with_fewer_pixels_than_cells_is_no_measurement() {
        // 1×1 and 0×0 ptys, and a window claiming 10 px across 80 columns:
        // none of them may produce a zero-pixel cell.
        for (columns, rows, width, height) in [(0, 0, 0, 0), (1, 1, 0, 0), (80, 24, 10, 10)] {
            let mut host = FakeHost::silent().with_window(columns, rows, width, height);

            let size =
                remeasure_cell_size(&mut host, measured(), BUDGET).expect("a fake cannot fail");

            assert_eq!(size.source, CellSizeSource::Assumed);
            assert!(size.width > 0 && size.height > 0);
        }
    }

    #[test]
    fn a_write_failure_is_returned_not_absorbed() {
        let mut resized = FakeHost::silent().failing_to_write();
        let mut refocused = FakeHost::silent().failing_to_write();

        assert!(remeasure_cell_size(&mut resized, measured(), BUDGET).is_err());
        assert!(requery_theme(&mut refocused, dark_background(), BUDGET).is_err());
    }

    #[test]
    fn focus_regain_asks_only_for_the_background_and_the_fence() {
        let mut host = FakeHost::silent();

        requery_theme(&mut host, dark_background(), BUDGET).expect("a fake cannot fail");

        assert_eq!(host.written, vec![b"\x1b]11;?\x1b\\\x1b[c".to_vec()]);
    }

    #[test]
    fn a_reported_light_background_switches_the_hint() {
        let mut host = FakeHost::answering(&[b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\\x1b[?62;22c"]);

        let hint = requery_theme(&mut host, dark_background(), BUDGET).expect("a fake cannot fail");

        assert_eq!(hint.theme, Theme::Light);
        assert_eq!(
            hint.source,
            ThemeSource::Background(Rgb {
                red: 255,
                green: 255,
                blue: 255,
            })
        );
    }

    #[test]
    fn a_silent_terminal_leaves_the_theme_unchanged() {
        for answer in [SAYS_NOTHING, ONLY_THE_FENCE] {
            let mut host = FakeHost::answering(&[answer]);

            let hint =
                requery_theme(&mut host, dark_background(), BUDGET).expect("a fake cannot fail");

            assert_eq!(hint, dark_background());
        }
    }

    #[test]
    fn a_re_query_waits_no_longer_than_its_budget() {
        let mut host = FakeHost::silent();
        let start = Instant::now();

        requery_theme(&mut host, dark_background(), SSH_PROBE_BUDGET).expect("a fake cannot fail");

        assert!(within_budget(host.longest_wait(start), SSH_PROBE_BUDGET));
    }
}
