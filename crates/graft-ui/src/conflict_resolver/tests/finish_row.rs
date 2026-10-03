//! The finish row (m6-conflicts/60): on the notice row once every file is
//! done, full words where they fit, short ones where not, cut with `…`
//! below that; `accent` BOLD on a `selection` fill, REVERSED with no colour.

use graft_core::capabilities::ColorDepth;
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;

use super::super::*;
use crate::paint::color;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

const FULL: &str = "✓ every file done — c review and finish the merge";
const SHORT: &str = "✓ every file done — c review & finish";

/// Two files, both done, the resolver on the second: a done document of
/// two plain lines.
fn every_file_done(finish: bool) -> DocView {
    let text = |s: &str| graft_term::sanitize_line(s);
    DocView {
        title: text("notes.txt · done ✓ · d starts it over"),
        files: FileListView {
            rows: vec![
                FileRow {
                    name: text("gone.txt"),
                    state: FileState::Done,
                },
                FileRow {
                    name: text("notes.txt"),
                    state: FileState::Done,
                },
            ],
            cursor: 1,
            unavailable: false,
        },
        names: SideNames {
            ours: text("main"),
            theirs: text("feature"),
            roles: SideRoles::Merge,
        },
        regions: vec![RegionView::Plain {
            numbers: BlockNumbers {
                ours: 1,
                theirs: 1,
                result: 1,
            },
            lines: vec![text("one"), text("TWO from feature")],
        }],
        cursor_block: None,
        base_shown: false,
        notice: Some((text("gone.txt done ✓ · ▸ notes.txt"), false)),
        finish: finish.then(|| FinishRow {
            full: text(FULL),
            short: text(SHORT),
        }),
    }
}

fn theme(depth: ColorDepth, no_color: bool) -> Theme {
    Theme::new(ThemeKind::Dark, &test_capabilities(depth, no_color))
}

/// The resolver drawn at `columns` x 20 in its own placement; the buffer
/// and where the notice row is.
fn drawn(doc: &DocView, columns: u16, theme: &Theme) -> (Buffer, Rect) {
    let area = Rect::new(0, 0, columns, 20);
    let placed = placement(&doc.files, columns);
    let areas = resolver_areas(area, placed);
    let laid = layout(doc, Arrangement::Card, areas.doc.width);
    let mut buf = Buffer::empty(area);
    ResolverScreen {
        doc,
        layout: &laid,
        top: 0,
        placement: placed,
        theme,
        ascii: false,
    }
    .render(area, &mut buf);
    let row = Rect {
        y: areas.doc.y + 1,
        height: 1,
        ..areas.doc
    };
    (buf, row)
}

fn text_of(buf: &Buffer, row: Rect) -> String {
    (row.x..row.right())
        .map(|x| buf[(x, row.y)].symbol().to_owned())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// Width-driven: the full words wherever the notice row holds them, the
/// short ones where not, then cut with `…`. At 187 the list is a column
/// beside the document; at 120 and 80 a strip above it, so the row is the
/// frame's width and the full words (49 cells) fit.
#[test]
fn finish_row_words_by_width() {
    let doc = every_file_done(true);
    let theme = theme(ColorDepth::TrueColor, false);
    for columns in [187, 120, 80] {
        let (buf, row) = drawn(&doc, columns, &theme);
        assert_eq!(text_of(&buf, row), FULL, "{columns}");
    }
    // The row narrower than the full words: the short ones.
    let mut buf = Buffer::empty(Rect::new(0, 0, 70, 20));
    let placed = Placement::Column { width: 24 };
    let area = Rect::new(0, 0, 70, 20);
    let areas = resolver_areas(area, placed);
    assert!(areas.doc.width < 49 && areas.doc.width >= 37, "{areas:?}");
    let laid = layout(&doc, Arrangement::Card, areas.doc.width);
    ResolverScreen {
        doc: &doc,
        layout: &laid,
        top: 0,
        placement: placed,
        theme: &theme,
        ascii: false,
    }
    .render(area, &mut buf);
    let row = Rect {
        y: areas.doc.y + 1,
        height: 1,
        ..areas.doc
    };
    assert_eq!(text_of(&buf, row), SHORT);
    // Narrower still: the short words, cut.
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 3));
    let tiny = Rect::new(0, 0, 30, 3);
    ResolverScreen {
        doc: &doc,
        layout: &laid,
        top: 0,
        placement: Placement::Strip { rows: 0 },
        theme: &theme,
        ascii: false,
    }
    .draw_finish_row(&mut buf, tiny, doc.finish.as_ref().expect("set"));
    let cut = text_of(
        &buf,
        Rect {
            y: 1,
            height: 1,
            ..tiny
        },
    );
    assert!(cut.ends_with('…'), "{cut:?}");
    assert!(SHORT.starts_with(cut.trim_end_matches('…')), "{cut:?}");
}

/// `accent` BOLD on a full-width `selection` fill; with no colour at all
/// (`NO_COLOR`), REVERSED BOLD instead. Colour is **unverified** by eye.
#[test]
fn finish_row_style() {
    let doc = every_file_done(true);
    let theme = theme(ColorDepth::TrueColor, false);
    let (buf, row) = drawn(&doc, 120, &theme);
    let accent = color(theme.token(crate::theme::Token::Accent));
    let selection = color(theme.token(crate::theme::Token::Selection));
    for x in row.x..row.right() {
        let cell = &buf[(x, row.y)];
        assert_eq!(cell.bg, selection, "the fill spans the row, x {x}");
        assert!(cell.modifier.contains(Modifier::BOLD), "x {x}");
    }
    // `accent`, lifted to stay legible on the fill (ADR-0058 §3).
    assert_eq!(
        buf[(row.x, row.y)].fg,
        color(theme.on_selection(theme.token(crate::theme::Token::Accent)))
    );
    assert_ne!(
        buf[(row.x, row.y)].fg,
        accent,
        "dark B's accent needs the lift"
    );
    let checked = crate::test_support::assert_legible_on_selection(&buf, &theme);
    assert!(
        checked > 20,
        "the finish row's words are on the fill ({checked})"
    );

    let bare = theme_none();
    let (buf, row) = drawn(&doc, 120, &bare);
    for x in row.x..row.right() {
        let cell = &buf[(x, row.y)];
        assert!(cell.modifier.contains(Modifier::REVERSED), "x {x}");
    }
}

fn theme_none() -> Theme {
    theme(ColorDepth::None, true)
}

/// No finish row: the notice draws as before, muted and unfilled.
#[test]
fn without_a_finish_row_the_notice_draws() {
    let doc = every_file_done(false);
    let theme = theme(ColorDepth::TrueColor, false);
    let (buf, row) = drawn(&doc, 120, &theme);
    assert_eq!(text_of(&buf, row), "gone.txt done ✓ · ▸ notes.txt");
    assert!(!buf[(row.x, row.y)].modifier.contains(Modifier::BOLD));
}

/// A click on the finish row is `c`; on the title row, or with no finish
/// row, it is not.
#[test]
fn finish_row_at_hits_only_the_row() {
    let doc = every_file_done(true);
    for columns in [187, 120, 80] {
        let area = Rect::new(0, 2, columns, 30);
        let placed = placement(&doc.files, columns);
        let page = resolver_areas(area, placed).doc;
        assert!(
            finish_row_at(&doc, placed, area, page.x, page.y + 1),
            "{columns}"
        );
        assert!(
            finish_row_at(&doc, placed, area, page.right() - 1, page.y + 1),
            "{columns}"
        );
        assert!(
            !finish_row_at(&doc, placed, area, page.x, page.y),
            "{columns}: title"
        );
        assert!(
            !finish_row_at(&doc, placed, area, page.x, page.y + 2),
            "{columns}: body"
        );
        if page.x > 0 {
            assert!(
                !finish_row_at(&doc, placed, area, 0, page.y + 1),
                "{columns}: list"
            );
        }
        assert!(
            !finish_row_at(&every_file_done(false), placed, area, page.x, page.y + 1),
            "{columns}: none drawn"
        );
    }
}
