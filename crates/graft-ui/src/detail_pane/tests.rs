use std::sync::Arc;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::header::{compare_title, radar_lines_text, radar_text, title_style, title_text};
use super::layout::percent;
use super::unified::gutter_width;
use super::*;
use crate::detail::{
    BodyView, ContainedIn, DetailFocus, DetailHeader, DetailView, DiffMode, DiffOptionsView,
    DiffView, FileListView, FileRow, FileRowKind, FileStatusView, HunkView, LineKindView, LineView,
    RadarLine, Subject, SubmoduleCommitView, SubmoduleLogView, SubmoduleView,
};
use crate::paint::foreground;
use crate::test_support::terminal;
use crate::theme::{Theme, ThemeKind, Token};

fn theme(depth: ColorDepth) -> Theme {
    Theme::new(ThemeKind::Dark, &terminal(depth, false))
}

fn draw(view: &DetailView, layout: DetailLayout, columns: u16, rows: u16) -> Terminal<TestBackend> {
    let theme = theme(ColorDepth::TrueColor);
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).expect("test backend opens");
    terminal
        .draw(|frame| {
            frame.render_widget(
                DetailPane {
                    view,
                    layout: &layout,
                    theme: &theme,
                },
                frame.area(),
            );
        })
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

#[allow(clippy::too_many_lines)] // one hand-built view-model fixture, not logic
fn commit_view() -> DetailView {
    DetailView {
        header: DetailHeader {
            subject: Subject::Commit {
                sha: sanitize_line("a1b2c3d"),
            },
            summary: sanitize_line("Fix token refresh race"),
            author: sanitize_line("Rui Mehdi"),
            author_date: sanitize_line("2026-09-13 14:02 +0500"),
            relative: sanitize_line("2 min ago"),
            committer: None,
            co_authors: vec![],
            parents: vec![sanitize_line("9f8e7d6")],
            against: None,
            contained: ContainedIn::Known {
                branches: vec![sanitize_line("main")],
                tags: vec![],
                more: 0,
            },
            radar: vec![],
        },
        files: Some(Arc::new(FileListView {
            rows: vec![
                FileRow {
                    depth: 0,
                    name: sanitize_line("src/auth/token.rs"),
                    kind: FileRowKind::File {
                        status: FileStatusView::Modified,
                        added: Some(12),
                        removed: Some(4),
                    },
                    entry: Some(0),
                },
                FileRow {
                    depth: 0,
                    name: sanitize_line("src/auth/session.rs"),
                    kind: FileRowKind::File {
                        status: FileStatusView::Added,
                        added: Some(3),
                        removed: Some(0),
                    },
                    entry: Some(1),
                },
                FileRow {
                    depth: 0,
                    name: sanitize_line("assets/logo.png"),
                    kind: FileRowKind::File {
                        status: FileStatusView::Modified,
                        added: None,
                        removed: None,
                    },
                    entry: Some(2),
                },
            ],
            cursor: 0,
            top: 0,
            files: 3,
            added: 15,
            removed: 4,
            message: vec![],
            compare: None,
        })),
        body: BodyView::Diff(Arc::new(DiffView {
            path: sanitize_line("src/auth/token.rs"),
            old_path: None,
            status: FileStatusView::Modified,
            hunks: vec![HunkView {
                heading: sanitize_line("@@ -40,7 +40,15 @@ fn refresh()"),
                lines: vec![
                    LineView {
                        kind: LineKindView::Context,
                        old_no: Some(40),
                        new_no: Some(40),
                        text: sanitize_line("let token = self.cache;"),
                        words: vec![],
                        syntax: vec![],
                        pair: None,
                    },
                    LineView {
                        kind: LineKindView::Removed,
                        old_no: Some(41),
                        new_no: None,
                        text: sanitize_line("if token.is_expired() {"),
                        words: vec![],
                        syntax: vec![],
                        pair: Some(2),
                    },
                    LineView {
                        kind: LineKindView::Added,
                        old_no: None,
                        new_no: Some(41),
                        text: sanitize_line("if token.is_expired()"),
                        words: vec![],
                        syntax: vec![],
                        pair: Some(1),
                    },
                    LineView {
                        kind: LineKindView::Added,
                        old_no: None,
                        new_no: Some(42),
                        text: sanitize_line("    && !refreshing {"),
                        words: vec![],
                        syntax: vec![],
                        pair: None,
                    },
                ],
            }],
            binary: None,
            truncated: None,
            untracked: false,
        })),
        body_top: 0,
        body_cursor: None,
        focus: DetailFocus::Diff,
        mode: DiffMode::Unified,
        options: DiffOptionsView {
            context: 3,
            ignore_whitespace: false,
            word_tint: true,
        },
        explain: None,
        following: false,
    }
}

fn working_tree_view(staged: u32, unstaged: u32, untracked: u32) -> DetailView {
    let mut view = commit_view();
    view.header.subject = Subject::WorkingTree {
        staged,
        unstaged,
        untracked,
    };
    view.header.summary = sanitize_line("Uncommitted changes");
    view.header.author = sanitize_line("");
    view.header.author_date = sanitize_line("");
    view.header.relative = sanitize_line("");
    view.header.parents = vec![];
    view.header.against = Some(sanitize_line("HEAD a1b2c3d"));
    view.header.contained = ContainedIn::NotAsked;
    view.header.radar = vec![];
    view.focus = DetailFocus::Files;
    view
}

#[test]
fn compute_picks_the_breakpoint_by_width() {
    assert!(matches!(
        DetailLayout::compute(160, 40, DetailFocus::Graph, None),
        DetailLayout::Right { .. }
    ));
    assert!(matches!(
        DetailLayout::compute(159, 40, DetailFocus::Graph, None),
        DetailLayout::Below { .. }
    ));
    assert!(matches!(
        DetailLayout::compute(99, 40, DetailFocus::Graph, None),
        DetailLayout::Tab { shown: false }
    ));
    assert!(matches!(
        DetailLayout::compute(99, 40, DetailFocus::Files, None),
        DetailLayout::Tab { shown: true }
    ));
}

#[test]
fn override_cells_replaces_the_percentage_but_keeps_the_minimum() {
    match DetailLayout::compute(200, 40, DetailFocus::Graph, Some(10)) {
        DetailLayout::Right { columns } => assert_eq!(columns, 48),
        other => panic!("expected Right, got {other:?}"),
    }
    match DetailLayout::compute(200, 40, DetailFocus::Graph, Some(90)) {
        DetailLayout::Right { columns } => assert_eq!(columns, 90),
        other => panic!("expected Right, got {other:?}"),
    }
    match DetailLayout::compute(120, 40, DetailFocus::Graph, Some(2)) {
        DetailLayout::Below { rows } => assert_eq!(rows, 8),
        other => panic!("expected Below, got {other:?}"),
    }
}

#[test]
fn areas_stack_files_above_body_in_right_and_tab() {
    let layout = DetailLayout::Right { columns: 60 };
    let pane = Rect::new(0, 0, 60, 30);
    let areas = detail_areas(pane, layout, 3);
    assert_eq!(areas.header, Rect::new(0, 0, 60, 5));
    assert_eq!(areas.files.y, 5);
    assert_eq!(areas.body.y, areas.files.y + areas.files.height);
    assert!(areas.files.height <= 3);
}

#[test]
fn areas_never_go_negative_with_no_files() {
    let layout = DetailLayout::Tab { shown: true };
    let pane = Rect::new(0, 0, 80, 3);
    let areas = detail_areas(pane, layout, 0);
    assert_eq!(areas.files.height, 0);
    assert_eq!(areas.body.y, areas.files.y);
}

#[test]
fn areas_cap_files_height_even_with_huge_file_counts() {
    let layout = DetailLayout::Right { columns: 60 };
    let pane = Rect::new(0, 0, 60, 30);
    let areas = detail_areas(pane, layout, 100_000);
    let rest_height = pane.height - 5;
    assert_eq!(areas.files.height, percent(rest_height, 40));
    assert_eq!(areas.body.height, rest_height - areas.files.height);
}

#[test]
fn below_splits_files_and_body_horizontally_beside_a_two_line_header() {
    let layout = DetailLayout::Below { rows: 12 };
    let pane = Rect::new(0, 0, 120, 12);
    let areas = detail_areas(pane, layout, 3);
    assert_eq!(areas.header, Rect::new(0, 0, 120, 2));
    // 29, not 30: the thirtieth column is the gutter, so a file name that
    // fills the list does not run into the diff beside it (m5/19). The
    // body's own x is unchanged — the gutter comes out of the list's width,
    // which is what keeps `graft::update::geometry`'s hit-testing mirror
    // landing on the same cells.
    assert_eq!(areas.files, Rect::new(0, 2, 29, 10));
    assert_eq!(areas.body, Rect::new(30, 2, 90, 10));
}

#[test]
fn empty_state_reads_select_a_commit() {
    let mut view = commit_view();
    view.body = BodyView::Empty(sanitize_line("select a commit"));
    view.files = None;
    let layout = DetailLayout::Right { columns: 60 };
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(
        screen.iter().any(|line| line.contains("select a commit")),
        "{screen:#?}"
    );
}

#[test]
fn loading_state_reads_reading() {
    let mut view = commit_view();
    view.body = BodyView::Loading(sanitize_line("reading\u{2026}"));
    let layout = DetailLayout::Right { columns: 60 };
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(
        screen.iter().any(|line| line.contains("reading\u{2026}")),
        "{screen:#?}"
    );
}

#[test]
fn failed_state_shows_the_first_stderr_line() {
    let mut view = commit_view();
    view.body = BodyView::Failed(sanitize_line("git diff-tree: fatal: bad object"));
    let layout = DetailLayout::Right { columns: 60 };
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(
        screen
            .iter()
            .any(|line| line.contains("git diff-tree: fatal: bad object")),
        "{screen:#?}"
    );
}

#[test]
fn message_state_shows_the_commit_body() {
    let mut view = commit_view();
    view.body = BodyView::Message {
        lines: vec![sanitize_line("Fix token refresh race"), sanitize_line("")],
    };
    let layout = DetailLayout::Right { columns: 60 };
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(
        screen
            .iter()
            .any(|line| line.contains("Fix token refresh race")),
        "{screen:#?}"
    );
}

#[test]
fn diff_state_draws_signs_and_hunk_heading() {
    let view = commit_view();
    let layout = DetailLayout::Right { columns: 60 };
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(
        screen
            .iter()
            .any(|line| line.contains("@@ -40,7 +40,15 @@")),
        "{screen:#?}"
    );
    assert!(
        screen
            .iter()
            .any(|line| line.contains("+ if token.is_expired()")),
        "{screen:#?}"
    );
    assert!(
        screen
            .iter()
            .any(|line| line.contains("- if token.is_expired()")),
        "{screen:#?}"
    );
}

#[test]
fn focused_region_title_paints_accent_others_muted() {
    let mut view = commit_view();
    view.focus = DetailFocus::Files;
    let layout = DetailLayout::Right { columns: 60 };
    let theme = theme(ColorDepth::TrueColor);
    assert_eq!(
        title_style(&view, &theme),
        foreground(&theme, Token::Accent)
    );

    view.focus = DetailFocus::Graph;
    assert_eq!(
        title_style(&view, &theme),
        foreground(&theme, Token::TextMuted)
    );
    let _ = layout;
}

#[test]
fn working_tree_header_reads_the_uncommitted_summary() {
    let view = working_tree_view(2, 1, 1);
    let layout = DetailLayout::Right { columns: 60 };
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(
        screen[0].contains("UNCOMMITTED \u{b7} 2 staged 1 unstaged 1 untracked"),
        "{screen:#?}"
    );
    assert!(screen[1].contains("diff vs HEAD a1b2c3d"), "{screen:#?}");
    assert!(!screen[2].contains("Reza"), "{screen:#?}");
    assert!(!screen[3].contains("Contained"), "{screen:#?}");
}

#[test]
fn working_tree_header_omits_zero_counts() {
    let title = title_text(&Subject::WorkingTree {
        staged: 3,
        unstaged: 0,
        untracked: 0,
    });
    assert_eq!(title, "UNCOMMITTED \u{b7} 3 staged");
}

#[test]
fn diff_body_returns_the_total_content_rows() {
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let theme = theme(ColorDepth::TrueColor);
    let area = Rect::new(0, 0, 40, 2);
    let mut buffer = Buffer::empty(area);
    let total = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    assert_eq!(total, 5); // one heading + four lines
}

#[test]
fn palette16_diff_lines_use_bold_dim_and_no_background() {
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let theme = theme(ColorDepth::Palette16);
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    // rows: 0 heading, 1 context, 2 removed, 3 added, 4 added. Gutter is
    // "old new " (3 + 1 + 3 + 1 = 8 cells) before the sign cell.
    for x in 0..area.width {
        for y in 0..area.height {
            assert_eq!(
                buffer[(x, y)].bg,
                ratatui::style::Color::Reset,
                "({x},{y}) carries a background"
            );
        }
    }
    let sign_cell = &buffer[(8, 3)];
    assert!(
        sign_cell.modifier.contains(ratatui::style::Modifier::BOLD),
        "{sign_cell:?}"
    );
    let removed_sign = &buffer[(8, 2)];
    assert!(
        removed_sign
            .modifier
            .contains(ratatui::style::Modifier::DIM),
        "{removed_sign:?}"
    );
}

#[test]
fn no_color_theme_also_drops_backgrounds_and_tint() {
    let theme = theme(ColorDepth::None);
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    for x in 0..area.width {
        for y in 0..area.height {
            assert_eq!(buffer[(x, y)].bg, ratatui::style::Color::Reset);
        }
    }
}

#[test]
fn gutter_width_matches_the_widest_line_number() {
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    // Fixture's widest number is 42 (two digits), so gutter stays at the
    // min-3 floor: "old new " is 3 + 1 + 3 + 1 = 8 cells.
    assert_eq!(gutter_width(&diff.hunks), 3);
}

#[test]
fn a_line_missing_one_side_leaves_that_gutter_half_blank() {
    let theme = theme(ColorDepth::TrueColor);
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    let screen = (0..area.width)
        .map(|x| buffer[(x, 2)].symbol().to_string())
        .collect::<String>();
    // Row 2 is the removed line: old_no 41, new_no None.
    assert!(screen.starts_with(" 41     -"), "{screen:?}");
}

#[test]
fn word_tint_is_a_stronger_background_than_the_soft_row_background() {
    let mut view = commit_view();
    let BodyView::Diff(diff) = &mut view.body else {
        panic!("expected a diff body");
    };
    let diff = std::sync::Arc::make_mut(diff);
    #[allow(clippy::single_range_in_vec_init)] // one word range is the point of the test
    let words = vec![0..2]; // "if" on the added line
    diff.hunks[0].lines[2].words = words;
    let theme = theme(ColorDepth::TrueColor);
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    // Gutter "old new " = 8 cells, sign + space = 2, so text starts at 10.
    let tinted = buffer[(10, 3)].bg;
    let plain = buffer[(15, 3)].bg;
    assert_ne!(tinted, plain, "word tint should stand out from the row bg");
    assert_eq!(tinted, ratatui::style::Color::Rgb(0x5D, 0xB8, 0x7A)); // Token::Added, full strength
    assert_eq!(plain, ratatui::style::Color::Rgb(0x1B, 0x1E, 0x24)); // Token::Surface, soft
}

#[test]
fn word_tint_false_leaves_the_word_at_the_plain_row_background() {
    // Review M1 (m2-details-and-diffs round 1b): `W` used to flip a flag
    // nothing read. `word_tint: false` must now actually turn the tint off.
    let mut view = commit_view();
    let BodyView::Diff(diff) = &mut view.body else {
        panic!("expected a diff body");
    };
    let diff = std::sync::Arc::make_mut(diff);
    #[allow(clippy::single_range_in_vec_init)]
    let words = vec![0..2]; // "if" on the added line
    diff.hunks[0].lines[2].words = words;
    let theme = theme(ColorDepth::TrueColor);
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: false,
            cursor: None,
        },
    );
    let word_cell = buffer[(10, 3)].bg;
    let plain = buffer[(15, 3)].bg;
    assert_eq!(
        word_cell, plain,
        "word_tint: false should paint the word like any other cell on the line"
    );
}

#[test]
fn unfocused_diff_body_keeps_its_added_removed_colour() {
    // Review M2 (m2-details-and-diffs round 1b): the body used to draw all
    // `text-muted` until `Tab` reached the Diff region. Focus is now shown
    // on the gutter only; the body keeps its colour either way.
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let theme = theme(ColorDepth::TrueColor);
    let area = Rect::new(0, 0, 40, 5);

    let mut focused_buf = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut focused_buf,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    let mut unfocused_buf = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut unfocused_buf,
        &theme,
        DiffFocus {
            focused: false,
            word_tint: true,
            cursor: None,
        },
    );
    // Row 2 is the removed line's sign+text; its fg must match focused or
    // not (the added/removed colour, not `text-muted`).
    assert_eq!(focused_buf[(10, 2)].fg, unfocused_buf[(10, 2)].fg);
    assert_ne!(
        unfocused_buf[(10, 2)].fg,
        foreground(&theme, Token::TextMuted).fg.unwrap()
    );
    // The gutter is the one place focus still shows.
    assert_ne!(focused_buf[(0, 2)].fg, unfocused_buf[(0, 2)].fg);
}

#[test]
fn syntax_role_paints_a_keyword_differently_from_plain_text() {
    use crate::syntax::{SyntaxRole, SyntaxSpan};

    let mut view = commit_view();
    let BodyView::Diff(diff) = &mut view.body else {
        panic!("expected a diff body");
    };
    let diff = std::sync::Arc::make_mut(diff);
    diff.hunks[0].lines[0].syntax = vec![SyntaxSpan {
        range: 0..3, // "let"
        role: SyntaxRole::Keyword,
    }];
    let theme = theme(ColorDepth::TrueColor);
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    // Row 1 is the context line "let token = self.cache;"; text starts at
    // gutter(8) + sign/space(2) = 10.
    let keyword_fg = buffer[(10, 1)].fg;
    let plain_fg = buffer[(16, 1)].fg; // inside "token", past the keyword span
    assert_ne!(keyword_fg, plain_fg);
    assert_eq!(keyword_fg, foreground(&theme, Token::Accent).fg.unwrap());
}

#[test]
fn scrolling_within_a_hunk_pins_its_heading_until_the_next_one_shows() {
    let theme = theme(ColorDepth::TrueColor);
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let area = Rect::new(0, 0, 40, 2);

    // top = 1 (context row) and top = 2 (removed row): both still inside
    // the one hunk, both must pin the same heading text at row 0.
    for top in [1, 2] {
        let mut buffer = Buffer::empty(area);
        let _rows = diff_body(
            &diff.hunks,
            top,
            None,
            area,
            &mut buffer,
            &theme,
            DiffFocus {
                focused: true,
                word_tint: true,
                cursor: None,
            },
        );
        let row0 = (0..area.width)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect::<String>();
        assert!(row0.contains("@@ -40,7 +40,15 @@"), "top={top}: {row0:?}");
    }

    // top = 0 lands exactly on the heading row: it's naturally visible, no
    // pin needed, so scrolling one row further away from it draws normal
    // content, not a duplicate heading.
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    let row0 = (0..area.width)
        .map(|x| buffer[(x, 0)].symbol().to_string())
        .collect::<String>();
    assert!(row0.contains("@@ -40,7 +40,15 @@"), "{row0:?}");
    let row1 = (0..area.width)
        .map(|x| buffer[(x, 1)].symbol().to_string())
        .collect::<String>();
    assert!(!row1.contains("@@"), "{row1:?}");
}

#[test]
fn the_pinned_heading_never_hides_the_line_cursors_own_row() {
    // m3/04 review C1. `k` past a hunk boundary leaves `body_top ==
    // body_cursor`; the pin used to take that slot, so `Space` and `d` aimed
    // at a row nobody could see. The cursor row must be on screen, with its
    // own text, not merely inside `[top, top + height)`.
    let theme = theme(ColorDepth::TrueColor);
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let area = Rect::new(0, 0, 40, 2);

    // Body row 2 is the removed line — inside the hunk, so a pin applies.
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        2,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: Some(2),
        },
    );
    let drawn: Vec<String> = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    let marked = drawn
        .iter()
        .find(|row| row.contains(super::line_render::CURSOR_GLYPH))
        .unwrap_or_else(|| panic!("the cursor row was not drawn at all: {drawn:?}"));
    assert!(
        marked.contains("if token.is_expired() {"),
        "the marked row is not body row 2: {marked:?}"
    );

    // The cursor one row below still gets its pin: only the row the pin
    // would cover is exempt.
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        2,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: Some(3),
        },
    );
    let row0 = (0..area.width)
        .map(|x| buffer[(x, 0)].symbol().to_string())
        .collect::<String>();
    assert!(row0.contains("@@ -40,7 +40,15 @@"), "{row0:?}");
    let row1 = (0..area.width)
        .map(|x| buffer[(x, 1)].symbol().to_string())
        .collect::<String>();
    assert!(
        row1.contains(super::line_render::CURSOR_GLYPH) && row1.contains("if token.is_expired()"),
        "{row1:?}"
    );
}

#[test]
fn truncated_diff_ends_with_the_more_lines_footer() {
    let theme = theme(ColorDepth::TrueColor);
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let area = Rect::new(0, 0, 40, 6);
    let mut buffer = Buffer::empty(area);
    let total = diff_body(
        &diff.hunks,
        0,
        Some((5, 105)),
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    assert_eq!(total, 6); // 5 real rows + the footer
    let footer = (0..area.width)
        .map(|x| buffer[(x, 5)].symbol().to_string())
        .collect::<String>();
    assert!(
        footer.contains("\u{2026} 100 more lines \u{b7} L load all"),
        "{footer:?}"
    );
}

#[test]
fn a_non_truncated_diff_has_no_footer_row() {
    let theme = theme(ColorDepth::TrueColor);
    let view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        panic!("expected a diff body");
    };
    let area = Rect::new(0, 0, 40, 6);
    let mut buffer = Buffer::empty(area);
    let total = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    assert_eq!(total, 5);
    let last_row = (0..area.width)
        .map(|x| buffer[(x, 4)].symbol().to_string())
        .collect::<String>();
    assert!(!last_row.contains("more lines"), "{last_row:?}");
}

#[test]
fn crlf_line_shows_the_glyph_after_the_text_outside_the_gutter() {
    let theme = theme(ColorDepth::TrueColor);
    let mut view = commit_view();
    let BodyView::Diff(diff) = &mut view.body else {
        panic!("expected a diff body");
    };
    let diff = std::sync::Arc::make_mut(diff);
    diff.hunks[0].lines[0].text = sanitize_line("let token = self.cache;\u{240D}");
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body(
        &diff.hunks,
        0,
        None,
        area,
        &mut buffer,
        &theme,
        DiffFocus {
            focused: true,
            word_tint: true,
            cursor: None,
        },
    );
    let row1 = (0..area.width)
        .map(|x| buffer[(x, 1)].symbol().to_string())
        .collect::<String>();
    assert!(row1.trim_end().ends_with("self.cache;\u{240D}"), "{row1:?}");
}

fn side_line(
    kind: LineKindView,
    old_no: Option<u32>,
    new_no: Option<u32>,
    text: &str,
    pair: Option<u32>,
) -> LineView {
    LineView {
        kind,
        old_no,
        new_no,
        text: sanitize_line(text),
        words: vec![],
        syntax: vec![],
        pair,
    }
}

fn half_text(buffer: &Buffer, y: u16, x0: u16, width: u16) -> String {
    (x0..x0 + width)
        .map(|x| buffer[(x, y)].symbol().to_string())
        .collect()
}

/// A hunk with one 2-removed/2-added paired run, plus an unpaired removed and
/// an unpaired added line, bracketed by context.
fn side_by_side_hunk() -> HunkView {
    HunkView {
        heading: sanitize_line("@@ -1,6 +1,6 @@"),
        lines: vec![
            side_line(LineKindView::Context, Some(1), Some(1), "ctx head", None),
            side_line(LineKindView::Removed, Some(2), None, "old a", Some(3)),
            side_line(LineKindView::Removed, Some(3), None, "old b", Some(4)),
            side_line(LineKindView::Added, None, Some(2), "new a", Some(1)),
            side_line(LineKindView::Added, None, Some(3), "new b", Some(2)),
            side_line(LineKindView::Removed, Some(4), None, "old c", None),
            side_line(LineKindView::Added, None, Some(4), "new d", None),
            side_line(LineKindView::Context, Some(5), Some(5), "ctx tail", None),
        ],
    }
}

#[test]
fn paired_rows_share_a_screen_row_and_unpaired_rows_leave_a_blank_half() {
    let theme = theme(ColorDepth::TrueColor);
    let hunks = vec![side_by_side_hunk()];
    let area = Rect::new(0, 0, 41, 8);
    let mut buffer = Buffer::empty(area);
    let total = diff_body_side_by_side(&hunks, 0, area, &mut buffer, &theme, true, true);
    // heading + ctx + 2 paired rows + unpaired-removed row + unpaired-added row + ctx = 7
    assert_eq!(total, 7);

    let left_width = 20;
    let right_x = 21;
    let right_width = 20;

    // Row 1: the leading context line, mirrored on both halves.
    assert!(half_text(&buffer, 1, 0, left_width).starts_with("  1   ctx head"));
    assert!(half_text(&buffer, 1, right_x, right_width).starts_with("  1   ctx head"));

    // Rows 2-3: the paired removed/added run, aligned row for row.
    assert!(half_text(&buffer, 2, 0, left_width).starts_with("  2 - old a"));
    assert!(half_text(&buffer, 2, right_x, right_width).starts_with("  2 + new a"));
    assert!(half_text(&buffer, 3, 0, left_width).starts_with("  3 - old b"));
    assert!(half_text(&buffer, 3, right_x, right_width).starts_with("  3 + new b"));

    // Row 4: an unpaired removed line — right half stays blank, not shifted.
    assert!(half_text(&buffer, 4, 0, left_width).starts_with("  4 - old c"));
    assert_eq!(
        half_text(&buffer, 4, right_x, right_width),
        " ".repeat(usize::from(right_width))
    );

    // Row 5: an unpaired added line — left half stays blank.
    assert_eq!(
        half_text(&buffer, 5, 0, left_width),
        " ".repeat(usize::from(left_width))
    );
    assert!(half_text(&buffer, 5, right_x, right_width).starts_with("  4 + new d"));

    // Row 6: the trailing context line, mirrored again.
    assert!(half_text(&buffer, 6, 0, left_width).starts_with("  5   ctx tail"));
    assert!(half_text(&buffer, 6, right_x, right_width).starts_with("  5   ctx tail"));
}

#[test]
fn word_tint_syntax_and_palette16_match_diff_bodys_look() {
    use crate::syntax::{SyntaxRole, SyntaxSpan};

    let mut view = commit_view();
    let BodyView::Diff(diff) = &mut view.body else {
        panic!("expected a diff body");
    };
    let diff = std::sync::Arc::make_mut(diff);
    #[allow(clippy::single_range_in_vec_init)]
    let words = vec![0..2]; // "if" on the added line, same fixture as diff_body's test
    diff.hunks[0].lines[2].words = words;
    diff.hunks[0].lines[0].syntax = vec![SyntaxSpan {
        range: 0..3, // "let"
        role: SyntaxRole::Keyword,
    }];

    let true_color = theme(ColorDepth::TrueColor);
    let area = Rect::new(0, 0, 60, 5);
    let mut buffer = Buffer::empty(area);
    let _rows = diff_body_side_by_side(&diff.hunks, 0, area, &mut buffer, &true_color, true, true);

    // Row 2 is the removed/added pair; the tinted word sits on the right
    // half's added line, same colour `diff_body`'s equivalent test asserts.
    let tinted = buffer[(36, 2)].bg;
    assert_eq!(tinted, ratatui::style::Color::Rgb(0x5D, 0xB8, 0x7A)); // Token::Added, full strength

    // Row 1 is the context line, mirrored on the left half; the keyword span
    // paints the same accent `diff_body` uses for `SyntaxRole::Keyword`.
    let keyword_fg = buffer[(6, 1)].fg;
    assert_eq!(
        keyword_fg,
        foreground(&true_color, Token::Accent).fg.unwrap()
    );

    // 16-colour fallback: no backgrounds anywhere, same rule as `diff_body`.
    let theme16 = theme(ColorDepth::Palette16);
    let mut buffer16 = Buffer::empty(area);
    let _rows = diff_body_side_by_side(&diff.hunks, 0, area, &mut buffer16, &theme16, true, true);
    for x in 0..area.width {
        for y in 0..area.height {
            assert_eq!(
                buffer16[(x, y)].bg,
                ratatui::style::Color::Reset,
                "({x},{y}) carries a background"
            );
        }
    }
}

#[test]
fn radar_text_covers_every_variant() {
    use graft_core::mistake_radar::Severity;

    assert_eq!(
        radar_text(&RadarLine::Finding {
            severity: Severity::Warning,
            title: sanitize_line("foxtrot merge"),
        }),
        "foxtrot merge"
    );
    assert_eq!(
        radar_text(&RadarLine::EvilUnchecked),
        "evil merge: unchecked"
    );
    assert_eq!(
        radar_text(&RadarLine::EvilChecking),
        "evil merge: checking\u{2026}"
    );
    assert_eq!(radar_text(&RadarLine::EvilClean), "evil merge: clean");
    assert_eq!(
        radar_text(&RadarLine::EvilNotCheckable(sanitize_line("octopus merge"))),
        "evil merge: not checkable \u{2014} octopus merge"
    );
    assert_eq!(
        radar_text(&RadarLine::ForcePushNotCheckable(sanitize_line(
            "no remote-tracking reflog"
        ))),
        "force-pushed: not checkable \u{2014} no remote-tracking reflog"
    );
    assert_eq!(
        radar_text(&RadarLine::NotChecked(sanitize_line(
            "commit b32274b is not in the repository"
        ))),
        "radar: not checked \u{2014} commit b32274b is not in the repository \u{b7} ctrl+r to try again"
    );
}

#[test]
fn radar_lines_text_joins_every_line_not_just_the_first() {
    use graft_core::mistake_radar::Severity;

    let lines = vec![
        RadarLine::Finding {
            severity: Severity::Warning,
            title: sanitize_line("foxtrot merge"),
        },
        RadarLine::EvilUnchecked,
    ];
    assert_eq!(
        radar_lines_text(&lines),
        "foxtrot merge \u{b7} evil merge: unchecked"
    );
    assert_eq!(radar_lines_text(&[]), "");
}

#[test]
fn header_shows_every_radar_line_in_the_right_layout() {
    use graft_core::mistake_radar::Severity;

    let mut view = commit_view();
    view.header.radar = vec![
        RadarLine::Finding {
            severity: Severity::Warning,
            title: sanitize_line("foxtrot merge"),
        },
        RadarLine::EvilUnchecked,
    ];
    let layout = DetailLayout::compute(160, 40, DetailFocus::Graph, None);
    let terminal = draw(&view, layout, 160, 40);
    let rows = text(&terminal);
    // The header's radar row (index 4 of the five canonical lines) carries
    // both lines joined, not just the finding.
    assert!(rows[4].contains("foxtrot merge"));
    assert!(rows[4].contains("evil merge: unchecked"));
}

// --- CONFLICTS section (m5-history-and-ghost-preview/04) ---

/// A working-tree view whose file list opens with a `CONFLICTS` section
/// (one unmerged file), followed by an ordinary `Staged` section — the
/// caller's row order, which `render_files` draws as given
/// (`WorkingTreeSection::Unmerged` first, per scope item 6).
fn conflicted_working_tree_view() -> DetailView {
    use crate::detail::WorkingTreeSection;
    let mut view = working_tree_view(1, 0, 0);
    view.files = Some(Arc::new(FileListView {
        rows: vec![
            FileRow {
                depth: 0,
                name: sanitize_line(WorkingTreeSection::Unmerged.title()),
                kind: FileRowKind::Section {
                    section: WorkingTreeSection::Unmerged,
                    count: 1,
                },
                entry: None,
            },
            FileRow {
                depth: 1,
                name: sanitize_line("src/auth/token.rs"),
                kind: FileRowKind::File {
                    status: FileStatusView::Unmerged {
                        how: sanitize_line("both modified"),
                    },
                    added: None,
                    removed: None,
                },
                entry: Some(0),
            },
            FileRow {
                depth: 0,
                name: sanitize_line("Staged"),
                kind: FileRowKind::Section {
                    section: WorkingTreeSection::Staged,
                    count: 1,
                },
                entry: None,
            },
            FileRow {
                depth: 1,
                name: sanitize_line("src/auth/session.rs"),
                kind: FileRowKind::File {
                    status: FileStatusView::Added,
                    added: Some(3),
                    removed: Some(0),
                },
                entry: Some(1),
            },
        ],
        cursor: 0,
        top: 0,
        files: 2,
        added: 3,
        removed: 0,
        message: vec![],
        compare: None,
    }));
    view
}

/// The `CONFLICTS` section draws first in the changes pane, and its file
/// row shows the porcelain pair as words (`both modified`), never a letter
/// and add/remove counts — the row-text half of this is also covered at the
/// `file_row_text` level by
/// `detail_pane::files::tests::an_unmerged_row_shows_how_as_words_not_a_letter_and_counts`;
/// this test is the widget-level check, through `DetailPane` end to end.
#[test]
fn conflicts_section_draws_first_with_words_not_a_letter() {
    let view = conflicted_working_tree_view();
    let layout = DetailLayout::compute(80, 24, DetailFocus::Files, None);
    let terminal = draw(&view, layout, 80, 24);
    let rows = text(&terminal);
    let conflicts_row = rows
        .iter()
        .position(|r| r.contains("CONFLICTS"))
        .unwrap_or_else(|| panic!("no CONFLICTS row: {rows:#?}"));
    let staged_row = rows
        .iter()
        .position(|r| r.contains("Staged"))
        .unwrap_or_else(|| panic!("no Staged row: {rows:#?}"));
    assert!(
        conflicts_row < staged_row,
        "CONFLICTS must draw before Staged: {rows:#?}"
    );
    let unmerged_row = &rows[conflicts_row + 1];
    assert!(unmerged_row.contains("both modified"), "{unmerged_row:?}");
    assert!(!unmerged_row.contains(" U "), "{unmerged_row:?}");
}

/// Not an assertion — dumps the CONFLICTS-first changes pane at 80×24 and
/// 160×48. Run by hand to regenerate: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft-ui detail_pane::tests::capture_for_docs_shots`.
#[test]
fn capture_for_docs_shots() {
    let mut shots = graft_fixtures::Captures::new();
    let view = conflicted_working_tree_view();
    let mut out = String::new();
    out.push_str(
        "# docs/shots/m5-ui-conflicts.txt — CONFLICTS section, changes pane\n\
         #\n\
         # What right looks like: `CONFLICTS (n)` heads the file list, ahead\n\
         # of Staged/Unstaged/Untracked. Its row shows the porcelain pair as\n\
         # words (`both modified`) in the trailing column, never a single\n\
         # letter and add/remove counts — there is no single \"the diff\"\n\
         # while both sides are still in the index.\n\
         \n",
    );
    for (label, w, h) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let layout = DetailLayout::compute(w, h, DetailFocus::Files, None);
        out.push_str(&format!("=== {label} ===\n"));
        for line in text(&draw(&view, layout, w, h)) {
            if !line.trim().is_empty() {
                out.push_str(line.trim_end());
                out.push('\n');
            }
        }
        out.push('\n');
    }
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m5-ui-conflicts.txt"
        ),
        out,
    );
    shots.finish();
}

/// `commit_view`'s diff as an untracked file's (m6-conflicts/81), with
/// `hunks` kept or emptied.
fn untracked_view(keep_hunks: bool) -> DetailView {
    let mut view = commit_view();
    let BodyView::Diff(diff) = &view.body else {
        unreachable!("commit_view's body is a diff")
    };
    let mut diff = (**diff).clone();
    diff.untracked = true;
    if !keep_hunks {
        diff.hunks.clear();
    }
    view.body = BodyView::Diff(Arc::new(diff));
    view
}

#[test]
fn an_untracked_file_with_lines_draws_its_lines() {
    let view = untracked_view(true);
    let screen = text(&draw(&view, DetailLayout::Right { columns: 80 }, 80, 30));
    assert!(
        screen.iter().any(|line| line.contains("&& !refreshing {")),
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains("no diff yet")),
        "{screen:#?}"
    );
}

#[test]
fn an_empty_untracked_file_says_empty_file() {
    let view = untracked_view(false);
    let screen = text(&draw(&view, DetailLayout::Right { columns: 80 }, 80, 30));
    assert!(
        screen.iter().any(|line| line.contains("empty file")),
        "{screen:#?}"
    );
}

// m7-14: a compare's first line, in words.

fn compare(
    head: &str,
    base: &str,
    state: crate::detail::CompareState,
) -> crate::detail::CompareHeader {
    crate::detail::CompareHeader {
        base: sanitize_line(base),
        head: sanitize_line(head),
        state,
    }
}

fn since(
    commits: usize,
    files: usize,
    added: usize,
    removed: usize,
) -> crate::detail::CompareState {
    crate::detail::CompareState::Since {
        commits,
        tally: Some(crate::detail::FileTally {
            files,
            added,
            removed,
        }),
    }
}

#[test]
fn the_header_says_changes_since_it_left() {
    let header = compare("feature-a", "main", since(3, 2, 10, 4));
    assert_eq!(
        compare_title(&header, usize::MAX),
        "changes on feature-a since it left main \u{b7} 3 commits \u{b7} 2 files +10 \u{2212}4"
    );
    let one = compare("feature-a", "main", since(1, 1, 1, 0));
    assert_eq!(
        compare_title(&one, usize::MAX),
        "changes on feature-a since it left main \u{b7} 1 commit \u{b7} 1 file +1 \u{2212}0"
    );
    assert_eq!(
        title_text(&Subject::Compare(header)),
        "changes on feature-a since it left main \u{b7} 3 commits \u{b7} 2 files +10 \u{2212}4"
    );
}

#[test]
fn an_ancestor_says_nothing_of_its_own() {
    let header = compare("old", "main", crate::detail::CompareState::AlreadyIn);
    assert_eq!(
        compare_title(&header, usize::MAX),
        "old is already in main \u{b7} nothing of its own"
    );
    let base = compare("main", "main", crate::detail::CompareState::IsBase);
    assert_eq!(
        compare_title(&base, usize::MAX),
        "this is the compare base \u{b7} x clears it"
    );
}

#[test]
fn no_shared_history_says_two_dot_in_words() {
    let header = compare(
        "orphan",
        "main",
        crate::detail::CompareState::NoSharedHistory,
    );
    assert_eq!(
        compare_title(&header, usize::MAX),
        "orphan and main share no history \u{b7} showing main..orphan"
    );
}

#[test]
fn a_failed_compare_says_why() {
    let header = compare(
        "x",
        "main",
        crate::detail::CompareState::Failed(sanitize_line("bad object")),
    );
    assert_eq!(
        compare_title(&header, usize::MAX),
        "couldn't compare: bad object"
    );
    let reading = compare("feature-a", "main", crate::detail::CompareState::Reading);
    assert_eq!(
        compare_title(&reading, usize::MAX),
        "comparing feature-a with main \u{2026}"
    );
}

#[test]
fn the_header_drops_the_tally_first_when_narrow() {
    let header = compare("feature-a", "main", since(3, 2, 10, 4));
    let counted = "changes on feature-a since it left main \u{b7} 3 commits";
    assert_eq!(compare_title(&header, 60), counted, "tally dropped first");
    assert_eq!(
        compare_title(&header, 45),
        "changes on feature-a since it left main",
        "then the count"
    );
    assert_eq!(
        compare_title(&header, 36),
        "changes on featu\u{2026} since it left main",
        "then the head's name, cut"
    );
    // Drawn: at 40 columns the header shows the ladder's step, not the count.
    let mut view = commit_view();
    view.header.subject = Subject::Compare(header);
    let screen = text(&draw(&view, DetailLayout::Right { columns: 40 }, 40, 12));
    assert!(
        screen.iter().any(|line| line.contains("changes on")),
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains("3 commits")),
        "{screen:#?}"
    );
}

/// A submodule's facts, ids as the capture shows them (m7-power-tools/13).
fn submodule(moved: bool, changed: bool, untracked: bool) -> SubmoduleView {
    SubmoduleView {
        from: Some(sanitize_line("3b2a1f0")),
        to: Some(sanitize_line(if moved { "9c8d7e6" } else { "3b2a1f0" })),
        changed_inside: changed,
        untracked_inside: untracked,
        not_set_up: false,
        log: None,
    }
}

/// The changes pane with one `UNSTAGED` section holding `libs/parser`.
fn submodule_row_view(view: SubmoduleView) -> DetailView {
    let mut detail = working_tree_view(0, 1, 0);
    detail.files = Some(Arc::new(FileListView {
        rows: vec![
            FileRow {
                depth: 0,
                name: sanitize_line("UNSTAGED"),
                kind: FileRowKind::Section {
                    section: crate::detail::WorkingTreeSection::Unstaged,
                    count: 1,
                },
                entry: None,
            },
            FileRow {
                depth: 1,
                name: sanitize_line("libs/parser"),
                kind: FileRowKind::File {
                    status: FileStatusView::Submodule(Box::new(view)),
                    added: None,
                    removed: None,
                },
                entry: Some(0),
            },
        ],
        cursor: 1,
        files: 1,
        ..FileListView::default()
    }));
    detail.body = BodyView::Empty(sanitize_line(""));
    detail
}

/// Draws the pane as the frame places it at `columns` x 40.
fn at_width(view: &DetailView, columns: u16) -> Vec<String> {
    let layout = DetailLayout::compute(columns, 40, DetailFocus::Files, None);
    let width = match layout {
        DetailLayout::Right { columns } => columns,
        DetailLayout::Below { .. } | DetailLayout::Tab { .. } => columns,
    };
    text(&draw(view, layout, width, 40))
}

/// The row at 187 columns (the pane is 78 wide): `▣`, the whole path, the
/// word `submodule`, then each true fact — never a letter or counts.
#[test]
fn a_submodule_row_says_its_facts_at_187_columns() {
    let screen = at_width(&submodule_row_view(submodule(true, true, false)), 187);
    let want = "\u{25a3} libs/parser  submodule \u{b7} pointer moved 3b2a1f0 \u{2192} 9c8d7e6 \u{b7} changed inside";
    assert!(screen.iter().any(|line| line.contains(want)), "{screen:#?}");
    assert!(
        !screen.iter().any(|line| line.contains("untracked inside")),
        "a false fact is not drawn: {screen:#?}"
    );
}

/// At 80 columns every fact still shows: the ids go first.
#[test]
fn a_submodule_row_drops_its_ids_before_a_fact_at_80_columns() {
    let screen = at_width(&submodule_row_view(submodule(true, true, true)), 80);
    let want = "\u{25a3} libs/parser  submodule \u{b7} pointer moved \u{b7} changed inside \u{b7} untracked inside";
    assert!(screen.iter().any(|line| line.contains(want)), "{screen:#?}");
}

/// Each fact alone, and not set up.
#[test]
fn each_submodule_fact_is_its_own_word() {
    assert_eq!(
        submodule(false, false, true).row_words(true),
        "submodule \u{b7} untracked inside"
    );
    assert_eq!(
        submodule(false, true, false).row_words(true),
        "submodule \u{b7} changed inside"
    );
    let not_set_up = SubmoduleView {
        not_set_up: true,
        ..SubmoduleView::default()
    };
    assert_eq!(not_set_up.row_words(true), "submodule \u{b7} not set up");
    let screen = at_width(&submodule_row_view(not_set_up), 187);
    assert!(
        screen
            .iter()
            .any(|line| line.contains("\u{25a3} libs/parser  submodule \u{b7} not set up")),
        "{screen:#?}"
    );
}

fn commit(pulled_in: bool, id: &str, subject: &str) -> SubmoduleCommitView {
    SubmoduleCommitView {
        pulled_in,
        id: sanitize_line(id),
        subject: sanitize_line(subject),
    }
}

/// The diff pane on a submodule path, `log` set.
fn submodule_diff_view(log: SubmoduleLogView) -> DetailView {
    let mut detail = submodule_row_view(submodule(true, false, false));
    let mut sub = submodule(true, false, false);
    sub.log = Some(log);
    detail.body = BodyView::Diff(Arc::new(DiffView {
        path: sanitize_line("libs/parser"),
        old_path: None,
        status: FileStatusView::Submodule(Box::new(sub)),
        hunks: vec![],
        binary: None,
        truncated: None,
        untracked: false,
    }));
    detail.focus = DetailFocus::Diff;
    detail
}

/// The header, then one row per commit: `+` pulled in, `−` rewound.
#[test]
fn the_diff_pane_on_a_submodule_lists_its_commits() {
    let screen = at_width(
        &submodule_diff_view(SubmoduleLogView::Commits(vec![
            commit(true, "9c8d7e6", "parse the new header"),
            commit(true, "5e4d3c2", "lexer: keep spans"),
            commit(false, "1a2b3c4", "an experiment"),
        ])),
        187,
    );
    for want in [
        "libs/parser \u{b7} submodule \u{b7} 3b2a1f0 \u{2192} 9c8d7e6",
        "+ 9c8d7e6 parse the new header",
        "+ 5e4d3c2 lexer: keep spans",
        "\u{2212} 1a2b3c4 an experiment",
    ] {
        assert!(
            screen.iter().any(|line| line.contains(want)),
            "{want}: {screen:#?}"
        );
    }
}

/// Past 50 commits: 50 rows, then `+N more`.
#[test]
fn the_diff_pane_caps_a_submodules_commits_at_50() {
    let commits = (0..53)
        .map(|n| commit(true, &format!("{n:07}"), &format!("commit {n}")))
        .collect();
    let view = submodule_diff_view(SubmoduleLogView::Commits(commits));
    let layout = DetailLayout::Tab { shown: true };
    let screen = text(&draw(&view, layout, 80, 90));
    assert!(
        screen
            .iter()
            .any(|line| line.contains("+ 0000049 commit 49")),
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains("commit 50")),
        "{screen:#?}"
    );
    assert!(
        screen
            .iter()
            .any(|line| line.trim_end().ends_with("+3 more")),
        "{screen:#?}"
    );
}

/// Not set up, or the commits missing: one line says so.
#[test]
fn the_diff_pane_says_when_a_submodules_commits_are_not_here() {
    let screen = at_width(&submodule_diff_view(SubmoduleLogView::NotHere), 187);
    assert!(
        screen
            .iter()
            .any(|line| line.contains("the submodule's commits aren't here \u{b7} not set up")),
        "{screen:#?}"
    );
}

/// `submodule_row_view` with the row named `name`.
fn submodule_row_named(name: &str, view: SubmoduleView) -> DetailView {
    let mut detail = submodule_row_view(view);
    let mut files = detail.files.as_deref().cloned().expect("a file list");
    files.rows[1].name = sanitize_line(name);
    detail.files = Some(Arc::new(files));
    detail
}

/// The diff pane on `view` (its `log` as given), focus on the diff; the
/// row under the cursor carries the same facts, as the app's does.
fn submodule_diff_of(view: SubmoduleView) -> DetailView {
    let mut detail = submodule_row_view(SubmoduleView {
        log: None,
        ..view.clone()
    });
    detail.focus = DetailFocus::Diff;
    detail.body = BodyView::Diff(Arc::new(DiffView {
        path: sanitize_line("libs/parser"),
        old_path: None,
        status: FileStatusView::Submodule(Box::new(view)),
        hunks: vec![],
        binary: None,
        truncated: None,
        untracked: false,
    }));
    detail
}

/// The file list's drawn area in `at_width(_, columns)`'s frame.
fn files_area(columns: u16) -> Rect {
    let layout = DetailLayout::compute(columns, 40, DetailFocus::Files, None);
    let width = match layout {
        DetailLayout::Right { columns } => columns,
        DetailLayout::Below { .. } | DetailLayout::Tab { .. } => columns,
    };
    detail_areas(Rect::new(0, 0, width, 40), layout, 2).files
}

/// The screen row drawing the submodule row, cut to the list's area.
fn submodule_row_on_screen(screen: &[String], area: Rect) -> String {
    let line = screen
        .iter()
        .find(|line| line.contains('\u{25a3}'))
        .unwrap_or_else(|| panic!("a submodule row: {screen:#?}"));
    line.chars()
        .skip(usize::from(area.x))
        .take(usize::from(area.width))
        .collect()
}

/// m7-power-tools/30i: at 120 (Below, a 30-column list) the row keeps every
/// fact as a mark, right-aligned where a file's `+3 −2` sits.
#[test]
fn a_submodule_row_keeps_every_fact_at_120_columns() {
    let screen = at_width(&submodule_row_view(submodule(true, true, true)), 120);
    let row = submodule_row_on_screen(&screen, files_area(120));
    assert!(row.contains("\u{25a3} libs/parser"), "{row:?}");
    assert!(
        row.ends_with("\u{2192} \u{b1} ?"),
        "right-aligned at the list's last drawn column: {row:?}"
    );
    assert!(!row.contains("submodule"), "{row:?}");
}

/// m7-power-tools/30i: the diff pane's second line says what the
/// submodule's own tree holds; moved-only has no second line and no blank.
#[test]
fn the_diff_pane_says_what_changed_inside_a_submodule() {
    let with_log = |mut view: SubmoduleView| {
        view.log = Some(SubmoduleLogView::Commits(vec![commit(
            true,
            "9c8d7e6",
            "parse the new header",
        )]));
        submodule_diff_of(view)
    };
    let header_at = |screen: &[String]| {
        screen
            .iter()
            .position(|line| line.contains("libs/parser \u{b7} submodule \u{b7}"))
            .expect("the header")
    };

    let screen = at_width(&with_log(submodule(true, true, true)), 187);
    let header = header_at(&screen);
    assert!(
        screen[header + 1].contains("changed inside \u{b7} untracked inside"),
        "{screen:#?}"
    );
    assert!(
        screen[header + 2].contains("+ 9c8d7e6 parse the new header"),
        "{screen:#?}"
    );

    let screen = at_width(&with_log(submodule(true, false, false)), 187);
    let header = header_at(&screen);
    assert!(
        screen[header + 1].contains("+ 9c8d7e6 parse the new header"),
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains("inside")),
        "{screen:#?}"
    );
}

/// The diff pane on `view` at `columns` draws `words` whole.
fn assert_line_two_whole(view: &SubmoduleView, words: &str, columns: u16) {
    let mut sub = view.clone();
    sub.log = Some(SubmoduleLogView::Commits(vec![]));
    let screen = at_width(&submodule_diff_of(sub), columns);
    assert!(
        screen.iter().any(|line| line.contains(words)),
        "{columns}: {screen:#?}"
    );
}

/// The sweep's four views: all three facts; changed only; not set up;
/// moved + untracked under a long path.
fn sweep_views() -> [(&'static str, SubmoduleView); 4] {
    let zlib = SubmoduleView {
        from: Some(sanitize_line("3b2a1f0")),
        to: Some(sanitize_line("9c8d7e6")),
        untracked_inside: true,
        ..SubmoduleView::default()
    };
    let not_set_up = SubmoduleView {
        not_set_up: true,
        ..SubmoduleView::default()
    };
    [
        ("libs/parser", submodule(true, true, true)),
        ("libs/parser", submodule(false, true, false)),
        ("libs/parser", not_set_up),
        ("vendor/third_party/zlib-ng", zlib),
    ]
}

/// m7-power-tools/30i: at every width the row draws the first rung of its
/// ladder that fits; no fact word or mark is cut; the path is cut only on
/// the last rung. The diff pane's line 2 is whole at every width too.
#[test]
fn a_submodule_row_degrades_by_its_ladder_only_80_to_187() {
    use super::files::{SubmoduleTail, submodule_tails};
    let mut rungs_seen = std::collections::BTreeSet::new();
    for (name, view) in sweep_views() {
        let tails = submodule_tails(&view);
        assert!(
            matches!(tails.last(), Some(SubmoduleTail::Marks(_))),
            "{tails:?}"
        );
        let row_view = submodule_row_named(name, view.clone());
        let inside = super::body::submodule_inside_words(&view);
        for columns in 80..=187_u16 {
            let area = files_area(columns);
            let width = usize::from(area.width);
            let row = submodule_row_on_screen(&at_width(&row_view, columns), area);
            let lead = format!("  \u{25a3} {name}  ");
            let (rung, chosen) = tails
                .iter()
                .enumerate()
                .find(|(_, tail)| match tail {
                    SubmoduleTail::Words(words) => {
                        lead.chars().count() + words.chars().count() <= width
                    }
                    SubmoduleTail::Marks(_) => true,
                })
                .expect("the last rung always fits");
            rungs_seen.insert(rung);
            match chosen {
                SubmoduleTail::Words(words) => {
                    assert_eq!(row.trim_end(), format!("{lead}{words}"), "{columns}");
                }
                SubmoduleTail::Marks(marks) => {
                    assert!(row.ends_with(marks.as_str()), "{columns}: {row:?}");
                    assert!(
                        row.contains(&format!("\u{25a3} {name} ")) || row.contains('\u{2026}'),
                        "the path whole, or cut with `…`: {columns}: {row:?}"
                    );
                }
            }
            if let Some(words) = &inside {
                assert_line_two_whole(&view, words, columns);
            }
        }
    }
    assert!(
        rungs_seen.len() >= 3,
        "the sweep crosses rungs: {rungs_seen:?}"
    );
}

#[test]
fn a_pane_waiting_for_the_cursor_to_rest_ends_its_title_row_in_reading() {
    let mut view = commit_view();
    let layout = DetailLayout::Right { columns: 60 };
    let still = text(&draw(&view, layout, 60, 20));
    assert!(!still[0].contains("reading\u{2026}"), "{still:#?}");
    view.following = true;
    let screen = text(&draw(&view, layout, 60, 20));
    assert!(screen[0].starts_with("COMMIT a1b2c3d"), "{screen:#?}");
    assert!(screen[0].ends_with(" reading\u{2026}"), "{screen:#?}");
}
