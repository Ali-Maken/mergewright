//! The card's colours below 180 columns (ADR-0044 §2, "Card"): no glyph
//! changes; an open block's sides in their tints, labels in their hue,
//! BOLD; a taken block in the taken side's tint with its name in that hue.
//! Asserted on the buffer; how it looks is unverified by capture (CLAUDE.md
//! rule 5).

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use super::*;
use crate::paint::color;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(ColorDepth::TrueColor, false),
    )
}

fn row_text(buf: &Buffer, y: u16, columns: u16) -> Vec<String> {
    (0..columns)
        .map(|x| buf[(x, y)].symbol().to_owned())
        .collect()
}

/// The x of the first cell of `word` on row `y`.
fn find(buf: &Buffer, y: u16, columns: u16, word: &str) -> u16 {
    let cells = row_text(buf, y, columns);
    let n = word.chars().count();
    let at = (0..cells.len().saturating_sub(n))
        .find(|&x| cells[x..x + n].concat() == word)
        .unwrap_or_else(|| panic!("{word:?} not on row {y}: {}", cells.concat()));
    u16::try_from(at).expect("small")
}

/// The screen row of the first layout row matching `want`.
fn row_of(laid: &Layout, want: impl Fn(&ScreenRow) -> bool) -> u16 {
    let i = laid
        .rows
        .iter()
        .position(want)
        .expect("the layout has the row");
    u16::try_from(i).expect("small") + HEADER_ROWS
}

fn fg(theme: &Theme, t: Token) -> Color {
    color(theme.token(t))
}

fn tint(theme: &Theme, t: Token) -> Color {
    color(theme.tint(t).expect("truecolor dark has tints"))
}

fn bold(cell: &ratatui::buffer::Cell) -> bool {
    cell.modifier.contains(Modifier::BOLD)
}

/// Cells well inside the left and the right half of a 120-column card.
const LEFT: u16 = GUTTER + 4;
const RIGHT: u16 = 116;

#[test]
fn card_tints_follow_the_block_state() {
    let theme = theme();
    let doc = three_block_doc(false);
    // 120 columns: block 0 open, paired; block 1 took theirs.
    let laid = layout(&doc, Arrangement::Card, 120);
    let buf = draw(&doc, &laid, 0, 120, 40, false);
    open_block_paired(&theme, &laid, &buf);
    taken_block_tinted(&theme, &laid, &buf);
    // 80 columns: stacked.
    let laid = layout(&doc, Arrangement::Card, 80);
    let buf = draw(&doc, &laid, 0, 80, 40, false);
    open_block_stacked(&theme, &laid, &buf);
}

/// Labels in their hue, BOLD; each half in its tint; the header's `open`.
fn open_block_paired(theme: &Theme, laid: &Layout, buf: &Buffer) {
    let labels = row_of(laid, |r| matches!(r, ScreenRow::Labels { block: 0 }));
    assert_eq!(
        buf[(GUTTER, labels)].fg,
        fg(theme, Token::Ours),
        "left label"
    );
    assert!(bold(&buf[(GUTTER, labels)]));
    let x = find(buf, labels, 120, "theirs");
    assert_eq!(buf[(x, labels)].fg, fg(theme, Token::Theirs), "right label");
    assert!(bold(&buf[(x, labels)]));

    let pair = row_of(laid, |r| matches!(r, ScreenRow::Pair { block: 0, .. }));
    assert_eq!(
        buf[(LEFT, pair)].bg,
        tint(theme, Token::OursTint),
        "left half"
    );
    assert_eq!(
        buf[(RIGHT, pair)].bg,
        tint(theme, Token::TheirsTint),
        "right half"
    );
    assert_eq!(
        buf[(LEFT, pair)].fg,
        fg(theme, Token::Text),
        "text keeps its colour"
    );

    let header = row_of(laid, |r| matches!(r, ScreenRow::Header { block: 0 }));
    let x = find(buf, header, 120, "open");
    assert_eq!(
        buf[(x, header)].fg,
        fg(theme, Token::Open),
        "the header's `open`"
    );
    assert!(
        buf[(x, header)].modifier.contains(Modifier::REVERSED),
        "cursor"
    );
}

/// A taken block: its name in the taken side's hue, its lines on its tint;
/// a plain row untinted.
fn taken_block_tinted(theme: &Theme, laid: &Layout, buf: &Buffer) {
    let header = row_of(laid, |r| matches!(r, ScreenRow::Header { block: 1 }));
    let x = find(buf, header, 120, "theirs · feature");
    assert_eq!(
        buf[(x, header)].fg,
        fg(theme, Token::Theirs),
        "took theirs · feature"
    );
    let taken = row_of(laid, |r| {
        matches!(
            r,
            ScreenRow::Side {
                block: 1,
                side: SideKind::Theirs,
                ..
            }
        )
    });
    assert_eq!(
        buf[(LEFT, taken)].bg,
        tint(theme, Token::TheirsTint),
        "taken line"
    );
    let plain = row_of(laid, |r| matches!(r, ScreenRow::Plain { .. }));
    assert_eq!(buf[(LEFT, plain)].bg, Color::Reset);
}

/// Stacked: each side's rows in its own tint, labels in their hue, BOLD.
fn open_block_stacked(theme: &Theme, laid: &Layout, buf: &Buffer) {
    let side = |which: SideKind| {
        row_of(
            laid,
            |r| matches!(r, ScreenRow::Side { block: 0, side, .. } if *side == which),
        )
    };
    for (label, token) in [
        (LabelOf::Ours, Token::Ours),
        (LabelOf::Theirs, Token::Theirs),
    ] {
        let y = side(SideKind::Label(label));
        assert_eq!(buf[(GUTTER, y)].fg, fg(theme, token));
        assert!(bold(&buf[(GUTTER, y)]));
    }
    assert_eq!(
        buf[(LEFT, side(SideKind::Ours))].bg,
        tint(theme, Token::OursTint)
    );
    assert_eq!(
        buf[(LEFT, side(SideKind::Theirs))].bg,
        tint(theme, Token::TheirsTint)
    );
}

#[test]
fn card_has_no_tint_at_sixteen_colours() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, Arrangement::Card, 120);
    let mut buf = Buffer::empty(Rect::new(0, 0, 120, 40));
    let theme = Theme::new(
        ThemeKind::Dark,
        &test_capabilities(ColorDepth::Palette16, false),
    );
    ResolverScreen {
        doc: &doc,
        layout: &laid,
        top: 0,
        placement: Placement::Strip { rows: 0 },
        theme: &theme,
        ascii: false,
    }
    .render(buf.area, &mut buf);
    for cell in buf.content() {
        assert_eq!(cell.bg, Color::Reset);
    }
}
