//! The band between the three panes (ADR-0044 §2): on a block's rows, the
//! two one-cell separators and each pane's tint say whether the block is
//! open or which side it took.
//!
//! | block       | left sep          | right sep         | yours | result          | theirs |
//! |-------------|-------------------|-------------------|-------|-----------------|--------|
//! | open        | `?` / `┃` open    | `?` / `┃` open    | ours  | open            | theirs |
//! | took yours  | `▶` / `┃` ours    | `┊` muted         | ours  | ours            | —      |
//! | took theirs | `┊` muted         | `◀` / `┃` theirs  | —     | theirs          | theirs |
//! | took both   | `▶` / `┃` ours    | `◀` / `┃` theirs  | ours  | ours, then theirs | theirs |
//!
//! The glyph before the slash is the rule row's, the one after it every row
//! below. A kept side's separator sits on that side's tint, so the band
//! runs unbroken from the side into the result. Tints come from
//! [`Theme::tint`], which answers `None` at 16 colours, in high contrast
//! and under `NO_COLOR`: the glyphs and hues then carry the state alone.
//! Colour is asserted by buffer tests; how it looks is unverified by
//! capture (CLAUDE.md rule 5).

use mergewright_core::capabilities::ColorDepth;
use ratatui::style::{Color, Modifier, Style};

use super::super::{BlockState, TakeView};
use super::Pane;
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

/// Which of the two separators: between yours and the result, or between
/// the result and theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

/// A block's rule row, or one of the rows below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowKind {
    Rule,
    Body,
}

/// Where a side pane stands in its block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stand {
    /// The block is open: nothing is taken yet.
    Open,
    /// This side's lines are in the result.
    Kept,
    /// The other side was taken: this one is muted.
    NotTaken,
}

/// Where `pane` stands in a block in `state`. The result pane is always
/// [`Stand::Kept`] once taken.
pub(super) fn stand(state: BlockState, pane: Pane) -> Stand {
    match (state, pane) {
        (BlockState::Open, _) => Stand::Open,
        (BlockState::Took(TakeView::Ours), Pane::Theirs)
        | (BlockState::Took(TakeView::Theirs), Pane::Yours) => Stand::NotTaken,
        (BlockState::Took(_), _) => Stand::Kept,
    }
}

/// The hue of a side pane: `ours` on the left, `theirs` on the right, by
/// position (ADR-0044 §1), whoever's side it is in this operation.
pub(super) fn side_hue(pane: Pane) -> Token {
    match pane {
        Pane::Yours => Token::Ours,
        Pane::Result => Token::Text,
        Pane::Theirs => Token::Theirs,
    }
}

/// The separator's glyph and style on a block's row.
pub(super) fn separator(
    state: BlockState,
    side: Side,
    row: RowKind,
    cursor: bool,
    theme: &Theme,
    ascii: bool,
) -> (char, Style) {
    let pane = match side {
        Side::Left => Pane::Yours,
        Side::Right => Pane::Theirs,
    };
    let (glyph, style) = match stand(state, pane) {
        Stand::Open => {
            let glyph = match (row, ascii) {
                (RowKind::Rule, _) => '?',
                (RowKind::Body, false) => '┃',
                (RowKind::Body, true) => '|',
            };
            (glyph, foreground(theme, Token::Open))
        }
        Stand::Kept => {
            let glyph = match (row, side, ascii) {
                (RowKind::Rule, Side::Left, false) => '▶',
                (RowKind::Rule, Side::Left, true) => '>',
                (RowKind::Rule, Side::Right, false) => '◀',
                (RowKind::Rule, Side::Right, true) => '<',
                (RowKind::Body, _, false) => '┃',
                (RowKind::Body, _, true) => '|',
            };
            let mut style = foreground(theme, side_hue(pane));
            if let Some(tint) = side_tint(pane).and_then(|t| theme.tint(t)) {
                style = style.bg(color(tint));
            }
            (glyph, style)
        }
        Stand::NotTaken => (if ascii { ':' } else { '┊' }, not_taken(theme)),
    };
    if cursor {
        (glyph, style.add_modifier(Modifier::BOLD))
    } else {
        (glyph, style)
    }
}

/// The tint behind `pane` on a block's row, or `None` for no tint (the side
/// not taken, or a theme that drops tints). `result_row` is the row's index
/// in the result's kept lines: a `both` take's first `ours_len` rows came
/// from the left, the rest from the right, in git's order.
pub(super) fn row_tint(
    state: BlockState,
    pane: Pane,
    result_row: usize,
    ours_len: usize,
    theme: &Theme,
) -> Option<Color> {
    let token = match (state, pane) {
        (BlockState::Open, Pane::Result) => Token::OpenTint,
        (BlockState::Took(TakeView::Ours), Pane::Result) => Token::OursTint,
        (BlockState::Took(TakeView::Theirs), Pane::Result) => Token::TheirsTint,
        (BlockState::Took(TakeView::Both), Pane::Result) => {
            if result_row < ours_len {
                Token::OursTint
            } else {
                Token::TheirsTint
            }
        }
        (_, side) if stand(state, side) == Stand::NotTaken => return None,
        (_, side) => side_tint(side)?,
    };
    theme.tint(token).map(color)
}

fn side_tint(pane: Pane) -> Option<Token> {
    match pane {
        Pane::Yours => Some(Token::OursTint),
        Pane::Result => None,
        Pane::Theirs => Some(Token::TheirsTint),
    }
}

/// The side not taken: `text-muted`, and DIM as well where there is no
/// colour at all to mute it with (`NO_COLOR`, ADR-0044 §1).
pub(super) fn not_taken(theme: &Theme) -> Style {
    let style = foreground(theme, Token::TextMuted);
    if theme.color_depth() == ColorDepth::None {
        style.add_modifier(Modifier::DIM)
    } else {
        style
    }
}
