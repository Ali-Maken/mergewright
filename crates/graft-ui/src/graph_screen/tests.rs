use graft_core::capabilities::ColorDepth;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::layout::layout;
use graft_graph::row::Row;
use graft_graph::seeds::Seeds;
use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::terminal;
use crate::theme::ThemeKind;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

/// A linear history of `count` commits, newest first.
fn linear(count: u8) -> Vec<Row> {
    let commits: Vec<Commit> = (1..=count)
        .rev()
        .map(|id| {
            let who = Identity {
                name: b"Test".to_vec(),
                email: b"test@example.invalid".to_vec(),
                when: Timestamp {
                    seconds: i64::from(id),
                    offset_seconds: 0,
                },
            };
            Commit {
                id: oid(id),
                parents: if id == 1 { vec![] } else { vec![oid(id - 1)] },
                author: who.clone(),
                committer: who,
                message: format!("Commit {id}").into_bytes(),
            }
        })
        .collect();
    layout(&commits, Seeds::default())
}

fn view(rows: &[Row], walk: WalkView, cursor: usize) -> GraphView<'_> {
    GraphView {
        location: Some(sanitize_line("/code/api")),
        walk,
        problem: None,
        rows: rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mut line = GraphLine::plain(row);
                if index == cursor {
                    line.paint = RowPaint::Highlighted;
                }
                GraphEntry {
                    line,
                    subject: sanitize_line(&String::from_utf8_lossy(row.commit.subject())),
                    author: AuthorChip {
                        initials: sanitize_line(&initials(&String::from_utf8_lossy(
                            &row.commit.author.name,
                        ))),
                        identity: BranchIdentity::of_name(&row.commit.author.email),
                    },
                    when: "2h".to_owned(),
                    sha: short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH)),
                }
            })
            .collect(),
    }
}

fn draw(view: &GraphView<'_>, columns: u16, rows: u16) -> Terminal<TestBackend> {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(view, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    terminal
}

fn text(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn rows_sit_between_the_header_and_the_key_hints() {
    let rows = linear(3);
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 3 }, 0),
        40,
        6,
    ));

    assert!(screen[0].contains("/code/api"), "{screen:#?}");
    assert!(screen[0].contains("3 commits"), "{screen:#?}");
    assert!(screen[1].contains("Commit 3"), "{screen:#?}");
    assert!(screen[3].contains("Commit 1"), "{screen:#?}");
    assert!(screen[5].contains("q quit"), "{screen:#?}");
    assert_eq!(usize::from(CHROME_ROWS), 2);
}

#[test]
fn the_cursor_row_is_raised_and_the_others_are_not() {
    let rows = linear(3);
    let terminal = draw(&view(&rows, WalkView::Complete { rows: 3 }, 1), 40, 6);
    let buffer = terminal.backend().buffer();
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps());
    let raised = color(theme.token(Token::SurfaceRaised));

    assert_eq!(buffer[(30, 2)].bg, raised);
    assert_ne!(buffer[(30, 1)].bg, raised);
    assert!(buffer[(10, 2)].modifier.contains(Modifier::BOLD));
}

fn terminal_caps() -> graft_core::capabilities::Capabilities {
    terminal(ColorDepth::TrueColor, false)
}

#[test]
fn a_jump_in_progress_says_how_far_it_has_got_and_how_to_back_out() {
    let rows = linear(2);
    let screen = text(&draw(
        &view(&rows, WalkView::Jumping { laid_out: 4500 }, 0),
        60,
        5,
    ));

    assert!(screen[0].contains("laying out… 4500 rows"), "{screen:#?}");
    assert!(screen[0].contains("Esc cancels"), "{screen:#?}");
}

#[test]
fn a_walk_that_is_fine_never_reads_as_an_error() {
    for walk in [
        WalkView::Opening,
        WalkView::Reading { laid_out: 0 },
        WalkView::Reading { laid_out: 1000 },
        WalkView::Complete { rows: 0 },
        WalkView::Cancelled { laid_out: 500 },
    ] {
        let (words, token) = walk_text(&walk);
        assert!(
            !words.contains("fail") && !words.contains("error"),
            "{words}"
        );
        assert_ne!(token, Token::Warning, "{walk:?}");
    }
    let (words, token) = walk_text(&WalkView::Failed(sanitize_line("bad object")));
    assert!(words.contains("bad object"));
    assert_eq!(token, Token::Warning);
}

#[test]
fn a_problem_takes_the_footers_place() {
    let rows = linear(2);
    let mut view = view(&rows, WalkView::Complete { rows: 2 }, 0);
    view.problem = Some(sanitize_line("job 3 failed: not a git repository"));
    let screen = text(&draw(&view, 60, 5));

    assert!(screen[4].contains("not a git repository"), "{screen:#?}");
    assert!(!screen[4].contains("q quit"), "{screen:#?}");
}

#[test]
fn more_rows_than_fit_are_cut_at_the_footer() {
    let rows = linear(10);
    let screen = text(&draw(
        &view(&rows, WalkView::Reading { laid_out: 10 }, 0),
        40,
        5,
    ));

    assert!(screen[3].contains("Commit 8"), "{screen:#?}");
    assert!(screen[4].contains("q quit"), "{screen:#?}");
}

#[test]
fn the_marker_slot_is_the_same_width_empty_or_filled() {
    let rows = linear(2);
    let empty = view(&rows, WalkView::Complete { rows: 2 }, 0);
    let empty_screen = text(&draw(&empty, 120, 5));

    let mut marked = rows.clone();
    marked[0].marker = Some(graft_core::mistake_radar::MistakeRadarKind::EvilMerge);
    let filled = view(&marked, WalkView::Complete { rows: 2 }, 0);
    let filled_screen = text(&draw(&filled, 120, 5));

    // Whether or not row 0 carries a marker, the subject starts at the same
    // column: the marker glyph and its neighbour's blank both take
    // `MARKER_WIDTH` cells, so later columns never shift.
    let subject_at = |line: &str| line.find("Commit").expect("the subject is drawn");
    assert_eq!(subject_at(&empty_screen[1]), subject_at(&filled_screen[1]));
    assert_ne!(empty_screen[1], filled_screen[1]);
}

#[test]
fn the_author_chip_drops_below_the_breakpoint_and_returns_above_it() {
    let rows = linear(2);
    let view = view(&rows, WalkView::Complete { rows: 2 }, 0);

    let narrow = text(&draw(&view, AUTHOR_BREAKPOINT - 1, 5));
    let wide = text(&draw(&view, AUTHOR_BREAKPOINT, 5));

    assert!(!narrow[1].contains("TE"), "{narrow:#?}");
    assert!(wide[1].contains("TE"), "{wide:#?}");
    // The date and SHA survive at both widths — only the author chip drops.
    assert!(narrow[1].contains("2h"), "{narrow:#?}");
    assert!(wide[1].contains("2h"), "{wide:#?}");
}

#[test]
fn tiny_and_empty_terminals_draw_without_panicking() {
    let rows = linear(3);
    let view = view(&rows, WalkView::Complete { rows: 3 }, 0);
    for (columns, height) in [(0, 0), (1, 1), (2, 2), (80, 1), (3, 24)] {
        let terminal = draw(&view, columns, height);
        assert_eq!(terminal.backend().buffer().area.height, height);
    }
}
