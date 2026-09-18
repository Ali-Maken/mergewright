use std::sync::Arc;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::detail::{DetailHeader, DiffMode, DiffOptionsView, DiffView, FileListView};
use crate::test_support::terminal;
use crate::theme::ThemeKind;

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
        focus: DetailFocus::Diff,
        mode: DiffMode::Unified,
        options: DiffOptionsView {
            context: 3,
            ignore_whitespace: false,
            word_tint: true,
        },
        explain: None,
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
    assert_eq!(areas.files, Rect::new(0, 2, 30, 10));
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
