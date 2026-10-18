//! Writing a `docs/shots/` capture: one frame of the real `view::view`, and
//! the file it goes into. Shared, because a capture written two slightly
//! different ways is two pictures of two apps.
#![allow(
    clippy::expect_used,
    reason = "a capture that cannot be drawn or written has nothing to show; the panic names why"
)]

use std::path::Path;
use std::time::Duration;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Rgb,
    Supported, Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};

/// A plain truecolour terminal that answered every probe. `Capabilities` has
/// no `Default` on purpose (an unprobed terminal is not a neutral one), so
/// the literal lives beside the one frame writer that needs it, the way
/// `frames.rs` keeps its own.
pub fn capabilities() -> Capabilities {
    Capabilities {
        tier: RenderingTier::Unicode,
        pixel_protocol: None,
        kitty_graphics: Supported::No,
        kitty_deflate: false,
        answers_dsr: false,
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::Yes,
        cell_size: CellSize {
            width: 9,
            height: 18,
            source: CellSizeSource::Reported,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0x10,
                green: 0x10,
                blue: 0x14,
            }),
        },
        color_depth: ColorDepth::TrueColor,
        no_color: false,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
}

/// Every row of the frame `view::view` draws for `state` at this size, drawn
/// against the wall clock.
///
/// A capture must not use this: its relative dates would read a different
/// age every run. Capture writers call [`frame_at`] with
/// `Some(Clock::Fixed(…))`.
pub fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    frame_at(state, columns, rows, None)
}

/// The path every capture draws its repository at, whatever directory the
/// test actually opened.
///
/// A capture drawn from a live repository is drawn from a scratch copy in the
/// system temp directory, and the header shows where you are — so the frame
/// carried `/var/folders/…/T/graft-remote-idICFn/wor…`, a name the fixture
/// invents afresh every run. That made the file impossible to reproduce and
/// so impossible to check, and it shipped one machine's directory layout in a
/// repository that may be published (m5-history-and-ghost-preview/40).
///
/// This is the same fix m5-history-and-ghost-preview/23 made for the clock,
/// at the same seam: a capture is drawn against a fixed instant *and* a fixed
/// location. No capture loses anything — a real path would be one machine's
/// either way — and `/code/api` is the path the rest of the captures and the
/// mockups in `docs/design/01-view-mockups.md` already use.
///
/// It is set **before** the frame is drawn, never substituted into the text
/// afterwards. The header *fits* itself to the width it has, so substituting
/// left the 80-column captures showing `graft /code/api  8…` — a truncation
/// caused by a path that is not in the picture — and tied every capture to
/// this machine's temp-path length, so the same file could not be drawn on
/// Linux (M5 groups 12/13 review, m2). Two captures were fixed that way, one
/// at a time; doing it here is what makes it true of all of them.
///
/// Since m8-polish-and-ship/06 the header writes a path under `$HOME` as
/// `~/…` (ADR-0058 §4, J22), so the path is put under this machine's home:
/// every machine then draws `~/code/api`. With no `HOME` at all it stays
/// `/code/api`, and the header draws that.
const CAPTURE_REPO_PATH: &str = "/code/api";

/// [`CAPTURE_REPO_PATH`] under `$HOME`, when there is one.
fn capture_repo_path() -> std::path::PathBuf {
    match std::env::var_os("HOME") {
        Some(home) if std::path::Path::new(&home).parent().is_some() => {
            std::path::Path::new(&home).join(CAPTURE_REPO_PATH.trim_start_matches('/'))
        }
        _ => std::path::PathBuf::from(CAPTURE_REPO_PATH),
    }
}

/// [`frame`], with the instant its relative dates are drawn against. `None`
/// is the wall clock; a capture that must not age passes
/// `Some(Clock::Fixed(…))`.
///
/// The repository's location is rewritten to [`CAPTURE_REPO_PATH`] whatever
/// directory the test opened, so the frame is the same on every machine.
pub fn frame_at(state: &AppState, columns: u16, rows: u16, clock: Option<Clock>) -> Vec<String> {
    let session =
        mergewright::session::Session::settle(&mergewright::config::Loaded::default(), capabilities());
    let mut state = state.clone();
    state.size = mergewright::update::Size { columns, rows };
    if let Some(clock) = clock {
        state.harness.clock = clock;
    }
    if let Some(repository) = &mut state.repository {
        repository.path = capture_repo_path();
    }
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows))
        .expect("test backend opens");
    terminal
        .draw(|frame| mergewright::view::view(&session, &state, frame))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// The shortest prefix of an object id a frame ever shows.
const SHORT_ID: usize = 7;

/// Replaces an object id the run itself invented with zeros of **the same
/// width**, every prefix of it the frame might show.
///
/// A capture that commits before it draws cannot name a fixed id: the commit
/// is made now, so its id is new every run, and the file could never be
/// compared to anything (m5-history-and-ghost-preview/40). Zeroing it keeps
/// the file reproducible and keeps every column where it was, so the grid the
/// user judges by eye is the grid the code drew.
///
/// This is only ever for a value the run genuinely invents, and the capture's
/// own caption says so. A capture that differs for any other reason is a
/// defect, and this is not a place to hide one — hence the panic when the id
/// is not there at all.
///
/// # Panics
///
/// If `id` never appears in `body`, which means the caller is normalising
/// something the frame does not show.
pub fn without_invented_id(body: &str, id: &str) -> String {
    assert!(
        id.len() >= SHORT_ID && id.chars().all(|c| c.is_ascii_hexdigit()),
        "{id:?} is not an object id"
    );
    let mut out = body.to_owned();
    // Longest prefix first: once the 40-character form is zeroed, the shorter
    // passes cannot match inside what it left behind.
    for length in (SHORT_ID..=id.len()).rev() {
        out = out.replace(&id[..length], &"0".repeat(length));
    }
    assert_ne!(
        out, body,
        "{id} was normalised out of a capture that never showed it"
    );
    out
}

/// Asserts one capture under `docs/shots/` against what the code just drew —
/// or, under `MWR_UPDATE_SHOTS=1`, rewrites it and fails anyway.
///
/// It used to write, unconditionally, from a `#[ignore]`d test nobody re-ran,
/// which is how 42 captures drifted across four milestones without a test
/// going red (m5-history-and-ghost-preview/40). See
/// [`mergewright_fixtures::capture`] for the mechanism and `TESTING.md` for the
/// workflow.
pub fn check_shot(name: &str, body: &str) {
    mergewright_fixtures::capture(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(name),
        body,
    );
}

/// Pumps until nothing on the frame is still loading, so a capture never
/// catches a `reading…` or `sorting…` that a moment later is gone (as
/// `conflict_interim::landing_shots::settled`). Moved here from
/// `resolver::shell_begun` for the undo-over-drift capture (m6-conflicts/79).
pub fn settled_frame(app: &crate::app_harness::App, state: AppState) -> AppState {
    app.pump(state, "a frame with nothing loading", |state| {
        frame(state, 120, 40)
            .iter()
            .all(|row| !row.contains("reading\u{2026}") && !row.contains("sorting\u{2026}"))
    })
}

/// Pumps until nothing a frame draws is still moving: the walk complete,
/// its promotion done, and the radar answered.
///
/// `remote_by_key::ready` holds once the first rows land, which is what
/// `f`/`p`/`P` decide from, not what a frame shows. With no commit-graph
/// the order is provisional (ADR-0008) and the header reads `N rows so far
/// date order, sorting…` or `N commits`, whichever the job pool reached
/// first; the radar's `ⓘ` on an unpushed `main` is there or not the same
/// way (m8-polish-and-ship/30, both seen in one capture across two runs).
/// [`settled_frame`] waits out only the `sorting…`.
pub fn at_rest(app: &crate::app_harness::App, state: AppState) -> AppState {
    app.pump(state, "a complete walk and the radar's answer", |state| {
        walk_has_settled(state) && radar_has_answered(state)
    })
}

/// One resolver row split at its list column's rule (ease-pass/11): the
/// list's cell and the document's part. A row with no rule there is all
/// document — a strip list's frame, or no list at all.
pub fn split_at_list(row: &str) -> (String, String) {
    let at = usize::from(mergewright_ui::conflict_resolver::LIST_COLUMN);
    let chars: Vec<char> = row.chars().collect();
    if chars.get(at) == Some(&'│') {
        (
            chars[..at].iter().collect(),
            chars[at + 1..].iter().collect(),
        )
    } else {
        (String::new(), row.to_owned())
    }
}

/// The list column's cell naming `name`, its words one space apart
/// (`✓ gone.txt done`; the open file's starts `▏`). Empty when no cell
/// names it.
pub fn list_row(frame: &[String], name: &str) -> String {
    frame
        .iter()
        .map(|row| split_at_list(row).0)
        .find(|cell| cell.contains(name))
        .map(|cell| cell.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// Every row's document part, right of the list column's rule.
pub fn doc_rows(frame: &[String]) -> Vec<String> {
    frame.iter().map(|row| split_at_list(row).1).collect()
}

/// The walk has stopped moving: complete, its promotion done.
pub fn walk_has_settled(state: &AppState) -> bool {
    use mergewright::history::Phase;
    state
        .history
        .as_ref()
        .is_some_and(|history| history.phase == Phase::Complete && history.promotion.is_none())
}

/// Pumps until the Mistake Radar has answered: its default-branch context
/// has landed (`state.radar.context` is `Ready`, which is what builds the
/// ref findings the rows' `ⓘ` draws from), and neither probe it asks for
/// then is still running (`radar.duplicates`, every `radar.force_push`).
/// The radar is its own job, so a frame taken before it lands lacks its
/// marks; [`settled_frame`] waits only for `reading…` and `sorting…`
/// (review m6-look-finish-wave2 M3).
pub fn radar_answered(app: &crate::app_harness::App, state: AppState) -> AppState {
    app.pump(state, "the radar's answer", radar_has_answered)
}

/// [`radar_answered`]'s test, for a caller that waits on more than it does.
pub fn radar_has_answered(state: &AppState) -> bool {
    use mergewright::update::detail::Loading;
    matches!(state.radar.context, Loading::Ready(_))
        && state.radar.built_from.is_some()
        && !matches!(state.radar.duplicates, Loading::Pending(_))
        && !state
            .radar
            .force_push
            .values()
            .any(|verdict| matches!(verdict, Loading::Pending(_)))
}

#[cfg(test)]
mod tests {
    use super::without_invented_id;

    #[test]
    fn a_short_id_is_zeroed_where_it_appears() {
        let out = without_invented_id("tip 1e43bf9 pushed\n", "1e43bf9");
        assert_eq!(out, "tip 0000000 pushed\n");
    }

    #[test]
    fn every_prefix_the_frame_shows_is_zeroed_at_its_own_width() {
        let id = "1e43bf90c1d2e3f4";
        let out = without_invented_id("short 1e43bf9, long 1e43bf90c1d2e3f4.", id);
        assert_eq!(out, "short 0000000, long 0000000000000000.");
    }

    #[test]
    fn the_replacement_keeps_the_line_the_width_it_was() {
        let line = "◆    [◆ main ⇄ origin] to push     now  1e43bf9";
        let out = without_invented_id(line, "1e43bf9");
        assert_eq!(out.chars().count(), line.chars().count());
    }

    #[test]
    #[should_panic(expected = "never showed it")]
    fn normalising_an_id_the_frame_does_not_show_is_a_mistake() {
        without_invented_id("nothing here\n", "deadbee");
    }

    #[test]
    #[should_panic(expected = "is not an object id")]
    fn only_an_object_id_may_be_normalised() {
        without_invented_id("to push\n", "to push");
    }
}
