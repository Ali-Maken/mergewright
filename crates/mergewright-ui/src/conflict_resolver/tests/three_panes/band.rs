//! The band between the panes (ADR-0044 §2): per block state, the two
//! separator glyphs, their colour and weight, and each pane's tint — and
//! what is left of it at 16 colours, in high contrast, under `NO_COLOR`
//! and at the ASCII tier. Colour is asserted on the buffer; how it looks
//! is unverified by capture (CLAUDE.md rule 5).

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use super::super::*;
use super::fixtures::doc_of;
use crate::paint::color;

/// Four blocks, the last with one ours line and two theirs lines, so a
/// `both` take has a split result.
const FOUR: &[u8] = b"a\n<<<<<<< HEAD\nb\n=======\nB\n>>>>>>> t\nc\n<<<<<<< HEAD\nd\n=======\nD\n>>>>>>> t\ne\n<<<<<<< HEAD\nf\n=======\nF\n>>>>>>> t\ng\n<<<<<<< HEAD\nh\n=======\nH1\nH2\n>>>>>>> t\ni\n";

const WIDTH: u16 = 187;

/// Block 0 open, 1 took yours, 2 took theirs, 3 took both; cursor on 0.
fn four_states() -> DocView {
    let mut file = mergewright_git::conflict::parse(FOUR).expect("parses");
    file.take(1, mergewright_git::conflict::Take::Ours).expect("1");
    file.take(2, mergewright_git::conflict::Take::Theirs).expect("2");
    file.take(3, mergewright_git::conflict::Take::Both).expect("3");
    doc_of(&file, false)
}

fn theme_of(kind: ThemeKind, depth: ColorDepth, no_color: bool) -> Theme {
    Theme::new(kind, &test_capabilities(depth, no_color))
}

fn dark() -> Theme {
    theme_of(ThemeKind::Dark, ColorDepth::TrueColor, false)
}

/// The drawn buffer, the three panes, and the y of layout row 0.
struct Drawn {
    buf: Buffer,
    panes: [Rect; 3],
    y0: u16,
    laid: Layout,
}

impl Drawn {
    fn at(&self, x: u16, row: usize) -> &ratatui::buffer::Cell {
        &self.buf[(x, self.y0 + u16::try_from(row).expect("small"))]
    }
    fn left_sep(&self, row: usize) -> &ratatui::buffer::Cell {
        self.at(self.panes[0].right(), row)
    }
    fn right_sep(&self, row: usize) -> &ratatui::buffer::Cell {
        self.at(self.panes[1].right(), row)
    }
    /// A cell well inside pane `p` (past its mark and number columns).
    fn inside(&self, p: usize, row: usize) -> &ratatui::buffer::Cell {
        self.at(self.panes[p].x + 6, row)
    }
    /// The first letter of pane `p`'s text on `row`, past its mark and
    /// number columns.
    fn first_text(&self, p: usize, row: usize) -> &ratatui::buffer::Cell {
        let pane = self.panes[p];
        let x = (pane.x + 1..pane.right())
            .find(|&x| {
                let s = self.at(x, row).symbol();
                !s.chars().all(|c| c == ' ' || c.is_ascii_digit())
            })
            .expect("the row has text");
        self.at(x, row)
    }
    fn rule(&self, block: usize) -> usize {
        self.laid.block_row[block]
    }
}

fn draw_with(doc: &DocView, theme: &Theme, ascii: bool) -> Drawn {
    let laid = layout(doc, Arrangement::ThreePane, WIDTH);
    let height = u16::try_from(laid.rows.len()).expect("small") + HEADER_ROWS;
    let area = Rect::new(0, 0, WIDTH, height.max(MIN_ROWS));
    let placement = Placement::Column { width: 22 };
    let mut buf = Buffer::empty(area);
    ResolverScreen {
        doc,
        layout: &laid,
        top: 0,
        placement,
        theme,
        ascii,
        keys: crate::keymap::Keymap::shipped(),
    }
    .render(area, &mut buf);
    let areas = resolver_areas(area, placement);
    Drawn {
        buf,
        panes: three_panes(areas.doc),
        y0: areas.doc.y + HEADER_ROWS,
        laid,
    }
}

fn fg(theme: &Theme, token: Token) -> Color {
    color(theme.token(token))
}

fn tint(theme: &Theme, token: Token) -> Color {
    color(theme.tint(token).expect("truecolor dark has tints"))
}

/// `(symbol, fg, bold)` of a cell.
fn glyph(cell: &ratatui::buffer::Cell) -> (String, Color, bool) {
    (
        cell.symbol().to_owned(),
        cell.fg,
        cell.modifier.contains(Modifier::BOLD),
    )
}

/// The dark truecolor theme's colours the band is checked against.
struct Hues {
    ours: Color,
    theirs: Color,
    open: Color,
    muted: Color,
    border: Color,
    ours_tint: Color,
    theirs_tint: Color,
    open_tint: Color,
}

impl Hues {
    fn of(theme: &Theme) -> Hues {
        Hues {
            ours: fg(theme, Token::Ours),
            theirs: fg(theme, Token::Theirs),
            open: fg(theme, Token::Open),
            muted: fg(theme, Token::TextMuted),
            border: fg(theme, Token::Border),
            ours_tint: tint(theme, Token::OursTint),
            theirs_tint: tint(theme, Token::TheirsTint),
            open_tint: tint(theme, Token::OpenTint),
        }
    }
}

fn s(glyph: &str) -> String {
    glyph.to_owned()
}

#[test]
fn band_follows_the_block_state() {
    let doc = four_states();
    let theme = dark();
    let d = draw_with(&doc, &theme, false);
    let h = Hues::of(&theme);
    plain_rows_keep_the_bar(&d, &h);
    open_block(&d, &h);
    took_yours(&d, &h);
    took_theirs(&d, &h);
    took_both(&d, &h);
}

/// Titles row and a plain row: `│` in border, no tint.
fn plain_rows_keep_the_bar(d: &Drawn, h: &Hues) {
    for sep in [d.left_sep(0), d.right_sep(0)] {
        assert_eq!(glyph(sep), (s("│"), h.border, false), "titles row");
        assert_eq!(sep.bg, Color::Reset, "titles row");
    }
    // Row 1 is the plain line `a`.
    assert_eq!(glyph(d.left_sep(1)), (s("│"), h.border, false), "plain row");
    assert_eq!(d.inside(1, 1).bg, Color::Reset, "plain row");
}

/// Block 0, open, the cursor block: `?` then `┃` in `open`, BOLD.
fn open_block(d: &Drawn, h: &Hues) {
    let r = d.rule(0);
    assert_eq!(glyph(d.left_sep(r)), (s("?"), h.open, true), "rule, left");
    assert_eq!(glyph(d.right_sep(r)), (s("?"), h.open, true), "rule, right");
    assert_eq!(glyph(d.left_sep(r + 1)), (s("┃"), h.open, true), "body");
    assert_eq!(glyph(d.right_sep(r + 1)), (s("┃"), h.open, true), "body");
    assert_eq!(
        [0, 1, 2].map(|p| d.inside(p, r + 1).bg),
        [h.ours_tint, h.open_tint, h.theirs_tint],
        "open body tints"
    );
}

/// Block 1, took yours: `▶`/`┃` in `ours` on `ours-tint`; `┊` muted.
fn took_yours(d: &Drawn, h: &Hues) {
    let r = d.rule(1);
    assert_eq!(glyph(d.left_sep(r)), (s("▶"), h.ours, false), "yours rule");
    assert_eq!(d.left_sep(r).bg, h.ours_tint);
    assert_eq!(glyph(d.left_sep(r + 1)), (s("┃"), h.ours, false));
    assert_eq!(d.left_sep(r + 1).bg, h.ours_tint);
    assert_eq!(glyph(d.right_sep(r)), (s("┊"), h.muted, false));
    assert_eq!(glyph(d.right_sep(r + 1)), (s("┊"), h.muted, false));
    assert_eq!(
        [0, 1, 2].map(|p| d.inside(p, r + 1).bg),
        [h.ours_tint, h.ours_tint, Color::Reset],
        "took yours tints"
    );
    assert_eq!(d.first_text(2, r + 1).fg, h.muted, "not taken: muted");
}

/// Block 2, took theirs: the mirror.
fn took_theirs(d: &Drawn, h: &Hues) {
    let r = d.rule(2);
    assert_eq!(glyph(d.left_sep(r)), (s("┊"), h.muted, false));
    assert_eq!(glyph(d.left_sep(r + 1)), (s("┊"), h.muted, false));
    assert_eq!(glyph(d.right_sep(r)), (s("◀"), h.theirs, false), "rule");
    assert_eq!(d.right_sep(r).bg, h.theirs_tint);
    assert_eq!(glyph(d.right_sep(r + 1)), (s("┃"), h.theirs, false));
    assert_eq!(d.right_sep(r + 1).bg, h.theirs_tint);
    assert_eq!(
        [0, 1, 2].map(|p| d.inside(p, r + 1).bg),
        [Color::Reset, h.theirs_tint, h.theirs_tint],
        "took theirs tints"
    );
    assert_eq!(d.first_text(0, r + 1).fg, h.muted, "not taken: muted");
}

/// Block 3, took both: `▶`/`┃` in ours, `◀`/`┃` in theirs; the result's
/// first `ours.len()` (1) rows ours-tint, then theirs-tint.
fn took_both(d: &Drawn, h: &Hues) {
    let r = d.rule(3);
    assert_eq!(glyph(d.left_sep(r)), (s("▶"), h.ours, false));
    assert_eq!(glyph(d.right_sep(r)), (s("◀"), h.theirs, false));
    assert_eq!(glyph(d.left_sep(r + 1)), (s("┃"), h.ours, false));
    assert_eq!(glyph(d.right_sep(r + 1)), (s("┃"), h.theirs, false));
    assert_eq!(
        [1, 2, 3].map(|k| d.inside(1, r + k).bg),
        [h.ours_tint, h.theirs_tint, h.theirs_tint],
        "took both: result rows 0, 1, 2"
    );
    assert_eq!(d.inside(0, r + 1).bg, h.ours_tint);
    assert_eq!(d.inside(2, r + 1).bg, h.theirs_tint);
}

#[test]
fn cursor_rule_row_keeps_its_reverse_over_a_tint() {
    let doc = four_states();
    let theme = dark();
    let d = draw_with(&doc, &theme, false);
    let r = d.rule(0);
    for p in 0..3 {
        let cell = d.inside(p, r);
        assert!(cell.modifier.contains(Modifier::REVERSED), "pane {p}");
    }
    // The cursor mark `▏` still in the yours pane's mark column.
    assert_eq!(d.at(d.panes[0].x, r + 1).symbol(), "\u{258f}");
    // Other blocks' rule rows are not reversed.
    assert!(!d.inside(1, d.rule(1)).modifier.contains(Modifier::REVERSED));
}

/// No cell of the document area carries a background.
fn no_background(d: &Drawn) -> Result<(), String> {
    let x0 = d.panes[0].x;
    let x1 = d.panes[2].right();
    for row in 0..d.laid.rows.len() {
        for x in x0..x1 {
            let bg = d.at(x, row).bg;
            if bg != Color::Reset {
                return Err(format!("row {row}, x {x}: bg {bg:?}"));
            }
        }
    }
    Ok(())
}

#[test]
fn band_at_sixteen_colours_has_no_background() {
    let doc = four_states();
    let theme = theme_of(ThemeKind::Dark, ColorDepth::Palette16, false);
    let d = draw_with(&doc, &theme, false);
    no_background(&d).unwrap_or_else(|why| panic!("16 colours: {why}"));
    // The hue stays on the glyphs.
    assert_eq!(d.left_sep(d.rule(1)).fg, fg(&theme, Token::Ours));
    assert_eq!(d.right_sep(d.rule(2)).fg, fg(&theme, Token::Theirs));
    assert_eq!(d.left_sep(d.rule(0)).fg, fg(&theme, Token::Open));
}

#[test]
fn band_in_high_contrast_has_no_background() {
    let doc = four_states();
    let theme = theme_of(ThemeKind::HighContrast, ColorDepth::TrueColor, false);
    let d = draw_with(&doc, &theme, false);
    no_background(&d).unwrap_or_else(|why| panic!("high contrast: {why}"));
    assert_eq!(d.left_sep(d.rule(1)).fg, fg(&theme, Token::Ours));
}

#[test]
fn band_under_no_color_is_glyphs_and_modifiers() {
    let doc = four_states();
    let theme = theme_of(ThemeKind::Dark, ColorDepth::TrueColor, true);
    let d = draw_with(&doc, &theme, false);
    no_background(&d).unwrap_or_else(|why| panic!("NO_COLOR: {why}"));
    for row in 0..d.laid.rows.len() {
        for x in d.panes[0].x..d.panes[2].right() {
            assert_eq!(d.at(x, row).fg, Color::Reset, "row {row}, x {x}");
        }
    }
    // Glyphs as in the table.
    assert_eq!(d.left_sep(d.rule(1)).symbol(), "▶");
    assert_eq!(d.right_sep(d.rule(1)).symbol(), "┊");
    assert_eq!(d.left_sep(d.rule(0)).symbol(), "?");
    // DIM on the side not taken, not on the kept one.
    let r = d.rule(1);
    let dim = |p: usize, row: usize| d.first_text(p, row).modifier.contains(Modifier::DIM);
    assert!(dim(2, r + 1), "theirs, not taken");
    assert!(!dim(0, r + 1), "yours, taken");
    assert!(dim(2, r), "`not taken`");
    let r = d.rule(2);
    assert!(dim(0, r + 1), "yours, not taken");
    // BOLD on `? open`.
    let placeholder = d.inside(1, d.rule(0) + 1);
    assert!(placeholder.modifier.contains(Modifier::BOLD), "? open");
}

#[test]
fn band_ascii_glyphs() {
    let mut doc = four_states();
    doc.names.ours = text("yours - main");
    doc.names.theirs = text("theirs - feature");
    let theme = dark();
    let d = draw_with(&doc, &theme, true);
    let sym = |c: &ratatui::buffer::Cell| c.symbol().to_owned();
    assert_eq!(sym(d.left_sep(0)), "|", "titles");
    assert_eq!(sym(d.left_sep(d.rule(0))), "?");
    assert_eq!(sym(d.left_sep(d.rule(0) + 1)), "|");
    assert_eq!(sym(d.left_sep(d.rule(1))), ">");
    assert_eq!(sym(d.right_sep(d.rule(1))), ":");
    assert_eq!(sym(d.right_sep(d.rule(2))), "<");
    assert_eq!(sym(d.right_sep(d.rule(2) + 1)), "|");
    // Nothing the widget draws in the panes is above U+007F.
    for row in 0..d.laid.rows.len() {
        for x in d.panes[0].x..d.panes[2].right() {
            let s = sym(d.at(x, row));
            assert!(s.is_ascii(), "row {row}, x {x}: {s:?}");
        }
    }
}

#[test]
fn titles_and_rule_words_take_their_side_hue() {
    let doc = four_states();
    let theme = dark();
    let d = draw_with(&doc, &theme, false);
    let bold = |c: &ratatui::buffer::Cell| c.modifier.contains(Modifier::BOLD);
    // Titles: yours in ours BOLD, result in text BOLD, theirs in theirs BOLD.
    let title = |p: usize| d.at(d.panes[p].x + 1, 0);
    assert_eq!(title(0).fg, fg(&theme, Token::Ours));
    assert_eq!(title(1).fg, fg(&theme, Token::Text));
    assert_eq!(title(2).fg, fg(&theme, Token::Theirs));
    assert!((0..3).all(|p| bold(title(p))), "titles are bold");

    let word = |p: usize, block: usize| d.inside(p, d.rule(block)).fg;
    // Open: `o take` ours, `i of n · open` open, `t take` theirs.
    assert_eq!(
        [0, 1, 2].map(|p| word(p, 0)),
        [
            fg(&theme, Token::Ours),
            fg(&theme, Token::Open),
            fg(&theme, Token::Theirs)
        ]
    );
    // Took yours: `taken ✓` ours, `took yours` ours, `not taken` muted.
    assert_eq!(
        [0, 1, 2].map(|p| word(p, 1)),
        [
            fg(&theme, Token::Ours),
            fg(&theme, Token::Ours),
            fg(&theme, Token::TextMuted)
        ]
    );
    // Took theirs: the mirror.
    assert_eq!(
        [0, 1, 2].map(|p| word(p, 2)),
        [
            fg(&theme, Token::TextMuted),
            fg(&theme, Token::Theirs),
            fg(&theme, Token::Theirs)
        ]
    );
    // Took both: both sides in their hue, the result in text.
    assert_eq!(
        [0, 1, 2].map(|p| word(p, 3)),
        [
            fg(&theme, Token::Ours),
            fg(&theme, Token::Text),
            fg(&theme, Token::Theirs)
        ]
    );
    // `? open · o t b` in open, BOLD, on open-tint.
    let placeholder = d.inside(1, d.rule(0) + 1);
    assert_eq!(placeholder.fg, fg(&theme, Token::Open));
    assert!(bold(placeholder));
    assert_eq!(placeholder.bg, tint(&theme, Token::OpenTint));
}
