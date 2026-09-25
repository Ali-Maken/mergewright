//! Golden renders of `conflict_resolver::ResolverScreen`, over the fixture
//! this ticket's §1.5 diff used (`docs/design/01-view-mockups.md`'s
//! `src/auth/token.rs` rebase scenario: block 0 open at `first_line: 39`,
//! block 1 `Took(Theirs)` at `first_line: 57`).
//!
//! **What this pins, and what it does not.** These two files
//! (`tests/shots/resolver-120.txt`, `tests/shots/resolver-80.txt`) pin the
//! widget's own output against drift — a change to `layout`/`ResolverScreen`
//! that alters a cell has to touch a committed file, the way every other
//! capture in this crate works (`graft_fixtures::capture`, `docs/shots/`
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
//! Regenerate: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft-ui --test resolver_layout`.

mod support;

use std::path::Path;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
use graft_ui::conflict_resolver::{
    Arrangement, BlockNumbers, BlockState, BlockView, DocView, FileListView, FileRow, FileState,
    RegionView, ResolverScreen, SideNames, SideRoles, TakeView, layout, placement, resolver_areas,
};
use graft_ui::theme::ThemeKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use support::theme;

fn text(s: &str) -> graft_term::SanitizedText {
    sanitize_line(s)
}

fn lines(items: &[&str]) -> Vec<graft_term::SanitizedText> {
    items.iter().map(|s| text(s)).collect()
}

/// The scenario's file list: `token.rs` open, `routes.rs` not yet read.
fn mockup_files() -> FileListView {
    FileListView {
        rows: vec![
            FileRow {
                name: text("src/auth/token.rs"),
                state: FileState::Open { open: 2, of: 3 },
            },
            FileRow {
                name: text("src/api/routes.rs"),
                state: FileState::Unread,
            },
        ],
        cursor: 0,
        unavailable: false,
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
    let placed = placement(&mockup_doc().files, columns);
    render_in(placed.arrangement(), columns, rows)
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
    graft_fixtures::capture(shot("resolver-120.txt"), render(120, 40));
}

/// Pins the 80×24 render. Not a §1.5 match — see the module doc.
#[test]
fn resolver_layout_is_pinned_80() {
    graft_fixtures::capture(shot("resolver-80.txt"), render(80, 24));
}

/// The card through `Arrangement::Card` by name, not only through what
/// `placement` picks: the two pinned files, byte for byte, after the
/// three-pane arm and `RegionView::Plain::numbers` (m6-conflicts/14 S1).
#[test]
fn card_rows_are_unchanged() {
    graft_fixtures::capture(
        shot("resolver-120.txt"),
        render_in(Arrangement::Card, 120, 40),
    );
    graft_fixtures::capture(
        shot("resolver-80.txt"),
        render_in(Arrangement::Card, 80, 24),
    );
}

/// Pins the 200×50 three-pane render (m6-conflicts/14). Generated from the
/// mockup's data, not a match of any mockup's drawing: the file's first
/// line says so.
#[test]
fn resolver_layout_is_pinned_three_200() {
    let placed = placement(&mockup_doc().files, 200);
    assert_eq!(placed.arrangement(), Arrangement::ThreePane);
    let header = "# generated by resolver_layout_is_pinned_three_200 from the token.rs data; \
                  not a mockup match\n";
    graft_fixtures::capture(
        shot("resolver-three-200.txt"),
        format!("{header}{}", render(200, 50)),
    );
}
