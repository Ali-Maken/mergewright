//! His `[theme.<kind>]` colours through the session: on screen at startup,
//! and still his after the theme is rebuilt (m8-polish-and-ship/16).

use graft_core::capabilities::{Rgb, Theme as TerminalTheme, ThemeHint, ThemeSource};
use graft_ui::theme::{ThemeKind, Token, TokenColor};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

use super::for_tests::capabilities;
use super::{Session, TerminalChange};
use crate::config::{Config, Loaded, ThemeOverrides};
use crate::update::{AppState, Size};

const DARK_SELECTION: Rgb = Rgb {
    red: 0x3A,
    green: 0x3F,
    blue: 0x4B,
};
const LIGHT_SELECTION: Rgb = Rgb {
    red: 0xC0,
    green: 0xD0,
    blue: 0xE0,
};

fn with_theme(theme: ThemeOverrides) -> Loaded {
    Loaded {
        config: Config {
            theme,
            ..Config::default()
        },
        ..Loaded::default()
    }
}

fn both_selections() -> Session {
    Session::settle(
        &with_theme(ThemeOverrides {
            dark: vec![(Token::Selection, DARK_SELECTION)],
            light: vec![(Token::Selection, LIGHT_SELECTION)],
            ..ThemeOverrides::default()
        }),
        capabilities(),
    )
}

fn light_hint() -> ThemeHint {
    ThemeHint {
        theme: TerminalTheme::Light,
        source: ThemeSource::Background(Rgb {
            red: 250,
            green: 250,
            blue: 250,
        }),
    }
}

fn frame(session: &Session, state: &AppState) -> Buffer {
    let (columns, rows) = (state.size.columns, state.size.rows);
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| crate::view::frame::view(session, state, frame))
        .expect("TestBackend cannot fail to draw");
    terminal.backend().buffer().clone()
}

fn graph() -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        size: Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    }
}

fn colour(rgb: Rgb) -> Color {
    Color::Rgb(rgb.red, rgb.green, rgb.blue)
}

/// The row whose text holds `needle`, as its cells.
fn row_with<'a>(buffer: &'a Buffer, needle: &str) -> Vec<&'a ratatui::buffer::Cell> {
    let area = buffer.area;
    (area.top()..area.bottom())
        .map(|y| {
            (area.left()..area.right())
                .map(|x| &buffer[(x, y)])
                .collect::<Vec<_>>()
        })
        .find(|cells| {
            cells
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
                .contains(needle)
        })
        .unwrap_or_else(|| panic!("no row holds {needle:?}"))
}

#[test]
fn j10_the_cursor_row_is_filled_with_his_selection() {
    let session = Session::settle(
        &crate::config::tests::loaded_from_a_file("[theme.dark]\nselection = \"#3A3F4B\"\n"),
        capabilities(),
    );
    let buffer = frame(&session, &graph());
    let cursor = row_with(&buffer, "Commit 5");
    let filled = cursor
        .iter()
        .filter(|cell| cell.bg == colour(DARK_SELECTION))
        .count();
    assert!(
        filled > 20,
        "the cursor row's fill is his colour on {filled} cells"
    );
}

#[test]
fn retheme_keeps_each_kinds_overrides_across_a_switch() {
    let mut session = both_selections();
    assert!(session.retheme(ThemeKind::Light));
    assert_eq!(
        session.theme.token(Token::Selection),
        TokenColor::Rgb(LIGHT_SELECTION)
    );
    assert_eq!(session.rendering.theme, ThemeKind::Light);
    assert!(session.retheme(ThemeKind::Dark));
    assert_eq!(
        session.theme.token(Token::Selection),
        TokenColor::Rgb(DARK_SELECTION)
    );
    assert!(!session.retheme(ThemeKind::Dark), "no change twice");
}

#[test]
fn a_re_detected_background_rebuilds_through_retheme() {
    let mut session = both_selections();
    assert!(session.apply(TerminalChange::Theme(light_hint())));
    assert_eq!(session.rendering.theme, ThemeKind::Light);
    assert_eq!(
        session.theme.token(Token::Selection),
        TokenColor::Rgb(LIGHT_SELECTION),
        "the light theme with light's overrides, not the built-in"
    );
}

#[test]
fn an_overridden_bg_fills_the_commit_editor_and_never_the_graph() {
    let ground = Rgb {
        red: 0x20,
        green: 0x10,
        blue: 0x30,
    };
    let session = Session::settle(
        &with_theme(ThemeOverrides {
            dark: vec![(Token::Bg, ground)],
            ..ThemeOverrides::default()
        }),
        capabilities(),
    );

    let graph = frame(&session, &graph());
    for cell in row_with(&graph, "Commit 3") {
        assert_eq!(
            cell.bg,
            Color::Reset,
            "ADR-0003: the graph keeps the ground"
        );
    }
    assert!(
        graph.content().iter().all(|cell| cell.bg != colour(ground)),
        "nothing on the bare graph paints bg"
    );

    let mut editing = self::graph();
    editing.commit_editor = Some(crate::update::commit_editor::CommitEditor::new());
    let editor = frame(&session, &editing);
    let filled = editor
        .content()
        .iter()
        .filter(|cell| cell.bg == colour(ground))
        .count();
    assert!(filled > 0, "the commit editor's fill is his bg");
}

#[test]
fn no_color_draws_the_same_tokens_with_or_without_overrides() {
    let mut caps = capabilities();
    caps.no_color = true;
    let plain = Session::settle(&Loaded::default(), caps.clone());
    let his = Session::settle(
        &with_theme(ThemeOverrides {
            dark: vec![(Token::Selection, DARK_SELECTION)],
            ..ThemeOverrides::default()
        }),
        caps,
    );
    for token in Token::ALL {
        assert_eq!(his.theme.token(token), plain.theme.token(token));
    }
}
