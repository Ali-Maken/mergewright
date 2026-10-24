//! What the status screen has to show, and what it must never let through.
//!
//! The snapshots are the point: this screen is the one a user reads when they
//! are working out why Mergewright looks the way it does, and a wording change that
//! loses the provenance of a guess is exactly the kind of regression that
//! reviews miss.

use mergewright_core::capabilities::{CellSize, CellSizeSource, ColorDepth, RenderingTier};
use mergewright_term::{SanitizedText, sanitize_line};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::{
    HeadView, HistoryState, HistoryView, RenderingView, RepositoryView, StatusScreen, StatusView,
    TierSource,
};
use crate::graph_screen::{ProblemKind, ProblemLine};
use crate::test_support::terminal as capabilities;
use crate::theme::{Theme, ThemeKind};

/// Draw `view` into a terminal of exactly this size and return what the cells
/// ended up holding.
fn draw(view: &StatusView, columns: u16, rows: u16) -> String {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    let theme = Theme::new(ThemeKind::Dark, &capabilities(ColorDepth::TrueColor, false));

    terminal
        .draw(|frame| frame.render_widget(StatusScreen::new(view, &theme), frame.area()))
        .expect("TestBackend cannot fail to draw");

    terminal.backend().to_string()
}

fn text(value: &str) -> SanitizedText {
    sanitize_line(value)
}

/// A terminal that measured everything it was asked and needed no overrides.
fn measured() -> RenderingView {
    RenderingView {
        tier: RenderingTier::Unicode,
        tier_source: TierSource::Detected,
        cell_size: CellSize {
            width: 9,
            height: 18,
            source: CellSizeSource::Reported,
        },
        theme: ThemeKind::Dark,
        theme_forced: false,
        theme_unknown: false,
        color_depth: ColorDepth::TrueColor,
        no_color: false,
    }
}

/// An open repository on a branch, nothing to report.
fn healthy() -> StatusView {
    StatusView {
        repository: Some(RepositoryView {
            path: text("/code/api"),
            bare: false,
            head: HeadView::Branch(text("main")),
        }),
        rendering: measured(),
        notes: Vec::new(),
        problem: None,
        history: None,
    }
}

#[test]
fn an_open_repository_on_a_branch() {
    insta::assert_snapshot!(draw(&healthy(), 60, 10));
}

#[test]
fn nothing_is_reported_when_there_is_nothing_to_report() {
    // The status-field rule (CLAUDE.md): no notes and no problem means no
    // notes section and no problem line, not an empty one.
    let screen = draw(&healthy(), 60, 10);

    assert!(!screen.contains("error"), "{screen}");
    assert!(!screen.contains("warning"), "{screen}");
}

#[test]
fn a_branch_name_with_an_escape_in_it_renders_inert() {
    // A branch really can be named this, and the name reaches the screen only
    // as `SanitizedText`. What the terminal must receive is the *picture* of
    // an escape, never the escape.
    let raw = "ma\u{1b}[31min\u{7}";
    let mut view = healthy();
    view.repository = Some(RepositoryView {
        path: text("/code/api"),
        bare: false,
        head: HeadView::Branch(text(raw)),
    });

    let screen = draw(&view, 60, 10);

    assert!(!screen.contains('\u{1b}'), "an escape reached the cells");
    assert!(!screen.contains('\u{7}'), "a bell reached the cells");
    assert!(screen.contains('\u{241b}'), "{screen}");
    insta::assert_snapshot!(screen);
}

#[test]
fn a_path_with_an_escape_in_it_renders_inert() {
    // Directory names are attacker-controlled in exactly the same way a clone
    // is, and the path is drawn before `HEAD` is even read.
    let mut view = healthy();
    view.repository = Some(RepositoryView {
        path: text("/tmp/\u{1b}]0;pwned\u{7}/api"),
        bare: false,
        head: HeadView::Branch(text("main")),
    });

    let screen = draw(&view, 60, 10);

    assert!(!screen.contains('\u{1b}'), "an escape reached the cells");
    insta::assert_snapshot!(screen);
}

#[test]
fn a_detached_head() {
    let mut view = healthy();
    view.repository = Some(RepositoryView {
        path: text("/code/api"),
        bare: false,
        head: HeadView::Detached(text("4f1a9c02be77")),
    });

    insta::assert_snapshot!(draw(&view, 60, 10));
}

#[test]
fn a_repository_with_no_commits_yet() {
    let mut view = healthy();
    view.repository = Some(RepositoryView {
        path: text("/code/fresh"),
        bare: false,
        head: HeadView::Unborn(text("main")),
    });

    insta::assert_snapshot!(draw(&view, 60, 10));
}

#[test]
fn a_bare_repository_with_an_unreadable_head() {
    let mut view = healthy();
    view.repository = Some(RepositoryView {
        path: text("/srv/api.git"),
        bare: true,
        head: HeadView::Unreadable(text("HEAD points at a ref that does not exist")),
    });

    insta::assert_snapshot!(draw(&view, 60, 10));
}

#[test]
fn the_first_frame_before_the_repository_is_open() {
    let mut view = healthy();
    view.repository = None;

    insta::assert_snapshot!(draw(&view, 60, 10));
}

#[test]
fn every_guess_says_it_is_a_guess() {
    let mut view = healthy();
    view.rendering = RenderingView {
        tier: RenderingTier::Pixel,
        tier_source: TierSource::Forced,
        cell_size: CellSize {
            width: 8,
            height: 16,
            source: CellSizeSource::Assumed,
        },
        theme: ThemeKind::HighContrast,
        theme_forced: true,
        theme_unknown: true,
        color_depth: ColorDepth::Palette256,
        no_color: false,
    };

    insta::assert_snapshot!(draw(&view, 72, 10));
}

#[test]
fn a_tier_nothing_measured_never_reads_as_detected() {
    // The M0 review's Major 2. Every source that is not `Detected` produced
    // the tier from environment hints, and the row has to say so — a user
    // comparing terminals is reading this exact line for whether the probe
    // worked.
    for source in [
        TierSource::Silent,
        TierSource::ProbeFailed,
        TierSource::NotProbed,
    ] {
        let mut view = healthy();
        view.rendering.tier_source = source;

        let screen = draw(&view, 72, 10);

        assert!(
            !screen.contains("detected"),
            "{source:?} rendered as a measurement: {screen}"
        );
        assert!(screen.contains("assumed ("), "{source:?}: {screen}");
    }
}

#[test]
fn a_terminal_that_said_nothing_about_itself() {
    let mut view = healthy();
    view.rendering.tier_source = TierSource::Silent;
    view.rendering.cell_size = CellSize {
        width: 8,
        height: 16,
        source: CellSizeSource::Assumed,
    };
    view.rendering.theme_unknown = true;

    insta::assert_snapshot!(draw(&view, 72, 10));
}

#[test]
fn a_terminal_the_user_turned_the_colour_off_in() {
    let mut view = healthy();
    view.rendering.no_color = true;
    view.rendering.theme_unknown = true;

    insta::assert_snapshot!(draw(&view, 72, 10));
}

#[test]
fn notes_and_a_problem() {
    let mut view = healthy();
    view.notes = vec![
        text("git 2.30.1 is older than the 2.38 Mergewright needs"),
        text("unknown config key `ui.colour`, ignored"),
    ];
    view.problem = Some(ProblemLine {
        kind: ProblemKind::Failure,
        reason: text("could not read HEAD: permission denied"),
        keys: None,
    });

    insta::assert_snapshot!(draw(&view, 60, 16));
}

#[test]
fn a_narrow_terminal_wraps_rather_than_truncating() {
    // A cut-off path is worse than a path over two lines.
    insta::assert_snapshot!(draw(&healthy(), 24, 14));
}

#[test]
fn a_terminal_too_short_for_everything() {
    insta::assert_snapshot!(draw(&healthy(), 60, 3));
}

#[test]
fn a_terminal_with_no_cells_at_all() {
    // ADR-0012's degenerate pty. The point is that drawing returns at all;
    // rendering a zero-sized `TestBackend` to text is what panics, so there is
    // no snapshot.
    let mut terminal = Terminal::new(TestBackend::new(0, 0)).expect("TestBackend reports its size");
    let theme = Theme::new(ThemeKind::Dark, &capabilities(ColorDepth::TrueColor, false));
    let view = healthy();

    terminal
        .draw(|frame| frame.render_widget(StatusScreen::new(&view, &theme), frame.area()))
        .expect("TestBackend cannot fail to draw");

    assert_eq!(terminal.backend().buffer().area.area(), 0);
}

#[test]
fn history_says_where_the_walk_is_and_how_many_rows_it_has() {
    let states = [
        (HistoryState::Streaming, "streaming, 412 rows laid out"),
        (HistoryState::Complete, "complete, 412 rows laid out"),
        (HistoryState::Cancelled, "cancelled, 412 rows laid out"),
        (
            HistoryState::Failed(text("the pack is corrupt")),
            "failed: the pack is corrupt, 412 rows laid out",
        ),
    ];
    for (state, expected) in states {
        let mut view = healthy();
        view.history = Some(HistoryView { state, rows: 412 });
        let screen = draw(&view, 72, 10);
        assert!(
            screen.contains(&format!("History     {expected}")),
            "{expected}:\n{screen}"
        );
    }
    assert!(
        !draw(&healthy(), 72, 10).contains("History"),
        "no walk started, no history row"
    );
}
