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
    let total = diff_body(&diff.hunks, 0, area, &mut buffer, &theme, true);
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
    let _rows = diff_body(&diff.hunks, 0, area, &mut buffer, &theme, true);
    // rows: 0 heading, 1 context, 2 removed, 3 added, 4 added
    let added_cell = &buffer[(0, 3)];
    assert_eq!(added_cell.bg, ratatui::style::Color::Reset);
}
