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
    BlockNumbers, BlockState, BlockView, DocView, RegionView, ResolverScreen, SideNames, SideRoles,
    TakeView, layout,
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

/// The §1.5 token.rs scenario: two blocks visible, one open with its OURS
/// and THEIRS sides given, one already `Took(Theirs)`.
fn mockup_doc() -> DocView {
    let block0 = BlockView {
        index: 0,
        of: 3,
        state: BlockState::Open,
        ours: lines(&["if expired() {", "self.renew();"]),
        base: None,
        theirs: lines(&["if is_expired() {", "self.do_refresh();"]),
        first_line: 39,
        numbers: BlockNumbers {
            ours: 39,
            theirs: 39,
            result: 39,
        },
    };
    let block1 = BlockView {
        index: 1,
        of: 3,
        state: BlockState::Took(TakeView::Theirs),
        ours: lines(&["old_way();"]),
        base: None,
        theirs: lines(&["let tok = self.store.get();"]),
        first_line: 42,
        numbers: BlockNumbers {
            ours: 42,
            theirs: 42,
            result: 42,
        },
    };
    DocView {
        title: text("src/auth/token.rs · file 1 of 2 · 2 of 3 conflicts open"),
        names: SideNames {
            ours: text("upstream · main"),
            theirs: text("yours · 3f2a1b7 Add OAuth provider"),
            roles: SideRoles::Rebase,
        },
        regions: vec![
            RegionView::Plain {
                first_line: 38,
                lines: lines(&["fn refresh(&mut self) {"]),
            },
            RegionView::Block(block0),
            RegionView::Plain {
                first_line: 41,
                lines: lines(&["}"]),
            },
            RegionView::Block(block1),
        ],
        cursor_block: Some(0),
        base_shown: false,
        notice: None,
    }
}

fn render(columns: u16, rows: u16) -> String {
    let doc = mockup_doc();
    let laid = layout(&doc, columns);
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
