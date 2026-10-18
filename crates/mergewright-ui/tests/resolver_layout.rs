//! Golden renders of `conflict_resolver::ResolverScreen`, over the fixture
//! this ticket's §1.5 diff used (`docs/design/01-view-mockups.md`'s
//! `src/auth/token.rs` rebase scenario: block 0 open at `first_line: 39`,
//! block 1 `Took(Theirs)` at `first_line: 57`).
//!
//! **What this pins, and what it does not.** These two files
//! (`tests/shots/resolver-120.txt`, `tests/shots/resolver-80.txt`) pin the
//! widget's own output against drift — a change to `layout`/`ResolverScreen`
//! that alters a cell has to touch a committed file, the way every other
//! capture in this crate works (`mergewright_fixtures::capture`, `docs/shots/`
//! module doc). They do **not** claim the render matches §1.5: measured
//! during m6-conflicts/02 (see that ticket's NOTES), §1.5's own frames are
//! hand-drawn for the markdown page — interior row widths there run 77–79
//! cells under a "120×40" label, never 118, and disagree with each other
//! row to row — so no renderer reproduces them byte-for-byte, correct or
//! not. Whether this render *looks* right is CLAUDE.md rule 6's call, by
//! eye, on G7's acceptance capture — not this file's, and not this
//! session's. The two named line-number defects this render fixes (a
//! block's `Header` row carrying its own line; a taken block's THEIRS lines
//! carrying theirs once on disk) were eyeballed against §1.5's structure
//! and text content, not its column widths — see the ticket for what still
//! differs beyond width.
//!
//! Regenerate: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright-ui --test resolver_layout`.

mod support;

use std::path::Path;

use mergewright_core::capabilities::ColorDepth;
use mergewright_term::sanitize_line;
use mergewright_ui::conflict_resolver::{
    Arrangement, BlockNumbers, BlockState, BlockView, CombinedList, DocView, FileListView, FileRow,
    FileRowLook, ListTail, Placement, RegionView, ResolverScreen, SideNames, SideRoles, TakeView,
    arrangement, layout, placement, resolver_areas,
};
use mergewright_ui::theme::ThemeKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use support::theme;

fn text(s: &str) -> mergewright_term::SanitizedText {
    sanitize_line(s)
}

fn lines(items: &[&str]) -> Vec<mergewright_term::SanitizedText> {
    items.iter().map(|s| text(s)).collect()
}

/// The scenario's file list: `token.rs` open, `routes.rs` not yet read.
fn mockup_files() -> FileListView {
    FileListView {
        rows: vec![
            FileRow {
                name: text("src/auth/token.rs"),
                state: FileRowLook::Open { open: 2, of: 3 },
            },
            FileRow {
                name: text("src/api/routes.rs"),
                state: FileRowLook::Unread,
            },
        ],
        cursor: 0,
        unavailable: false,
        tail: ListTail::None,
        combined: CombinedList::NotMerge,
    }
}

/// The §1.5 token.rs scenario: two blocks visible, one open with its OURS
/// and THEIRS sides given, one already `Took(Theirs)`.
/// The same line number on every side.
fn at(line: usize) -> BlockNumbers {
    BlockNumbers {
        ours: line,
        theirs: line,
        result: line,
    }
}

fn mockup_doc() -> DocView {
    let block0 = BlockView {
        index: 0,
        of: 3,
        state: BlockState::Open,
        ours: lines(&["if expired() {", "self.renew();"]),
        base: None,
        theirs: lines(&["if is_expired() {", "self.do_refresh();"]),
        first_line: 39,
        numbers: at(39),
        kept: Vec::new(),
    };
    let block1 = BlockView {
        index: 1,
        of: 3,
        state: BlockState::Took(TakeView::Theirs),
        ours: lines(&["old_way();"]),
        base: None,
        theirs: lines(&["let tok = self.store.get();"]),
        first_line: 42,
        numbers: at(42),
        kept: lines(&["let tok = self.store.get();"]),
    };
    DocView {
        title: text("src/auth/token.rs · 3 conflicts · 2 open"),
        files: mockup_files(),
        names: SideNames {
            ours: text("upstream · main"),
            theirs: text("yours · 3f2a1b7 Add OAuth provider"),
            roles: SideRoles::Rebase,
        },
        regions: vec![
            RegionView::Plain {
                numbers: at(38),
                lines: lines(&["fn refresh(&mut self) {"]),
            },
            RegionView::Block(block0),
            RegionView::Plain {
                numbers: at(41),
                lines: lines(&["}"]),
            },
            RegionView::Block(block1),
        ],
        cursor_block: Some(0),
        base_shown: false,
        notice: None,
        finish: None,
    }
}

fn render(columns: u16, rows: u16) -> String {
    render_in(arrangement(columns), columns, rows)
}

/// The mockup drawn in `arrangement`, whatever `columns` would pick.
fn render_in(arrangement: Arrangement, columns: u16, rows: u16) -> String {
    let doc = mockup_doc();
    let placed = placement(&doc.files, columns);
    let areas = resolver_areas(ratatui::layout::Rect::new(0, 0, columns, rows), placed);
    let laid = layout(&doc, arrangement, areas.doc.width);
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| {
            frame.render_widget(
                ResolverScreen {
                    doc: &doc,
                    layout: &laid,
                    top: 0,
                    placement: placed,
                    theme: &theme,
                    ascii: false,
                    keys: mergewright_ui::keymap::Keymap::shipped(),
                },
                frame.area(),
            );
        })
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..rows {
        let line: String = (0..columns).map(|x| buffer[(x, y)].symbol()).collect();
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn shot(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/shots")
        .join(name)
}

/// Pins the 120×40 render. Not a §1.5 match — see the module doc.
#[test]
fn resolver_layout_is_pinned_120() {
    mergewright_fixtures::capture(shot("resolver-120.txt"), render(120, 40));
}

/// Pins the 80×24 render. Not a §1.5 match — see the module doc.
#[test]
fn resolver_layout_is_pinned_80() {
    mergewright_fixtures::capture(shot("resolver-80.txt"), render(80, 24));
}

/// The inline document through its arrangement by name, not only through what
/// `placement` picks: the two pinned files, byte for byte, after the
/// three-pane arm and `RegionView::Plain::numbers` (m6-conflicts/14 S1).
#[test]
fn card_rows_are_unchanged() {
    mergewright_fixtures::capture(
        shot("resolver-120.txt"),
        render_in(Arrangement::Inline, 120, 40),
    );
    mergewright_fixtures::capture(
        shot("resolver-80.txt"),
        render_in(Arrangement::Card, 80, 24),
    );
}

/// ease-pass/11b: three panes retired. At 150, 180 and 200 columns the
/// list is the 30-cell column and the open clash draws its sides side by
/// side on one row, as at 120.
#[test]
fn wide_frames_render_inline_beside_the_30_column_list() {
    for columns in [150u16, 180, 200] {
        assert_eq!(
            placement(&mockup_doc().files, columns),
            Placement::Column { width: 30 },
            "{columns}"
        );
        assert_eq!(arrangement(columns), Arrangement::Inline, "{columns}");
        let drawn = render(columns, 40);
        for (y, line) in drawn.lines().enumerate() {
            assert_eq!(line.chars().nth(30), Some('│'), "{columns} row {y}: {line}");
        }
        assert!(
            drawn
                .lines()
                .any(|l| l.contains("? upstream · main") && l.contains("│yours · 3f2a1b7")),
            "{columns}: sides side by side\n{drawn}"
        );
        assert!(
            !drawn.contains("result"),
            "{columns}: no result pane\n{drawn}"
        );
    }
}
