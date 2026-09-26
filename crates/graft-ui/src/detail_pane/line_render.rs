//! Drawing helpers shared by [`super::unified`] and [`super::side_by_side`]:
//! one row's worth of gutter, sign, background and text styling. Neither
//! diff renderer inlines these — that is what keeps the two bodies matching
//! on the same fixture by construction (`diff-widget`, m2-details-and-diffs).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthChar;

use crate::detail::{LineKindView, LineView};
use crate::fit::fit;
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};
use graft_term::sanitize::SanitizedText;

/// [`super::diff_body`]/[`super::diff_body_side_by_side`]'s two independent
/// flags, bundled into one parameter to keep both functions under clippy's
/// argument-count limit.
#[derive(Clone, Copy, Debug)]
pub struct DiffFocus {
    /// The diff region has the keys (`Tab`); see [`DiffRowStyle::focused`]
    /// for what this does and does not change.
    pub focused: bool,
    /// Changed words get the stronger tint background (`W`).
    pub word_tint: bool,
    /// The line cursor's body row, when the body has one
    /// (m3-working-tree-undo-and-stash/04): the row `Space` and `d` act on.
    /// Every diff has one since m7-power-tools/20b (a commit's is what `B`
    /// blames); only the changes pane writes from it.
    pub cursor: Option<usize>,
}

/// The flags [`draw_diff_row`] needs beyond the row itself and the theme,
/// bundled to keep the function under clippy's argument-count limit.
#[derive(Clone, Copy)]
pub(crate) struct DiffRowStyle {
    /// The diff region has the keys. Marks focus on the gutter only
    /// (`gutter_style`); line/word/syntax colour never depends on it
    /// (m2-details-and-diffs review, M2: a body dimmed to `text-muted`
    /// until `Tab` reached it was unreadable by default).
    pub(crate) focused: bool,
    /// The 16-colour rule (or `NO_COLOR`) is in force: bold/dim, no
    /// background, no syntax or word tint.
    pub(crate) palette16: bool,
    /// Changed words get the stronger tint background (`W`, review M1: the
    /// flag used to reach nothing past the view model).
    pub(crate) word_tint: bool,
    /// This row is the line cursor's (m3-working-tree-undo-and-stash/04):
    /// `\u{258e}` in the cell left of the sign, and the whole row bold.
    ///
    /// The marker goes beside the sign rather than over it because the sign
    /// is what says whether the row would be added or removed — the very
    /// thing the reader needs before pressing `Space`. Bold alone would not
    /// do: a capture records characters, not attributes, so without a glyph
    /// no capture could ever show where the cursor is.
    pub(crate) cursor: bool,
}

/// The glyph marking the line cursor's row.
pub(crate) const CURSOR_GLYPH: char = '\u{258e}';

/// `style` with the cursor row's bold added, when this is that row.
pub(crate) fn with_cursor(style: Style, style_ctx: DiffRowStyle) -> Style {
    if style_ctx.cursor {
        style.add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        style
    }
}

/// The gutter's line-number colour: accent when this region has the keys,
/// muted otherwise. The only focus cue left on the diff body (M2 fix).
pub(crate) fn gutter_style(style_ctx: DiffRowStyle, theme: &Theme) -> Style {
    let base = if style_ctx.focused {
        foreground(theme, Token::Accent)
    } else {
        foreground(theme, Token::TextMuted)
    };
    with_cursor(base, style_ctx)
}

/// One screen row's position and width, bundled so drawing a row doesn't
/// need three separate `u16` arguments (clippy's argument-count limit).
#[derive(Clone, Copy)]
pub(crate) struct RowSlot {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) width: u16,
}

/// A row [`super::diff_body`] draws: a hunk's own heading, or one of its
/// lines.
pub(crate) enum DiffRow<'a> {
    Heading(&'a SanitizedText),
    Line(&'a LineView),
}

pub(crate) fn draw_diff_row(
    row: &DiffRow<'_>,
    gutter: usize,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    match row {
        DiffRow::Heading(heading) => draw_heading_row(heading, slot, buf, theme, style_ctx),
        DiffRow::Line(line) => draw_line_row(line, gutter, slot, buf, theme, style_ctx),
    }
}

/// A hunk heading drawn plainly: the pinned copy at the body's top row,
/// which is never the cursor's.
pub(crate) fn draw_heading(
    heading: &SanitizedText,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
) {
    draw_heading_row(
        heading,
        slot,
        buf,
        theme,
        DiffRowStyle {
            focused: false,
            palette16: false,
            word_tint: false,
            cursor: false,
        },
    );
}

/// A hunk heading in its own row, marked when the cursor is on it.
fn draw_heading_row(
    heading: &SanitizedText,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    let style = with_cursor(foreground(theme, Token::TextMuted), style_ctx);
    let text = if style_ctx.cursor {
        format!("{CURSOR_GLYPH}{}", heading.as_str())
    } else {
        heading.as_str().to_owned()
    };
    let fitted = fit(&text, usize::from(slot.width));
    buf.set_stringn(slot.x, slot.y, fitted, usize::from(slot.width), style);
}

pub(crate) fn draw_truncation_row(
    shown: usize,
    total: usize,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
) {
    let style = foreground(theme, Token::TextMuted);
    let more = total.saturating_sub(shown);
    let text = format!("\u{2026} {more} more lines \u{b7} L load all");
    let fitted = fit(&text, usize::from(slot.width));
    buf.set_stringn(slot.x, slot.y, fitted, usize::from(slot.width), style);
}

/// One diff line: `{old:>gutter} {new:>gutter} {sign}{text}`, the gutter
/// blank on whichever side has no number for this line.
pub(crate) fn draw_line_row(
    line: &LineView,
    gutter: usize,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    let RowSlot { x, y, width } = slot;
    let (sign, kind_token) = match line.kind {
        LineKindView::Added => ('+', Some(Token::Added)),
        LineKindView::Removed => ('-', Some(Token::Removed)),
        LineKindView::Context | LineKindView::HunkHeading => (' ', None),
    };

    let old = line
        .old_no
        .map_or(String::new(), |number| number.to_string());
    let new = line
        .new_no
        .map_or(String::new(), |number| number.to_string());
    // The prefix's trailing space is the cell left of the sign: the cursor's
    // marker goes there, so `+`/`-` stays readable (see [`DiffRowStyle::cursor`]).
    let marker = if style_ctx.cursor { CURSOR_GLYPH } else { ' ' };
    let prefix = format!("{old:>gutter$} {new:>gutter$}{marker}");
    let prefix_cells = u16::try_from(prefix.chars().count()).unwrap_or(u16::MAX);
    let prefix_fits = prefix_cells.min(width);
    buf.set_stringn(
        x,
        y,
        &prefix,
        usize::from(prefix_fits),
        gutter_style(style_ctx, theme),
    );
    if prefix_fits >= width {
        return;
    }

    // `usize::MAX` never falls inside a real word or syntax range, so this
    // gives the sign cell the row's base colour without matching either.
    let sign_style = line_style_at(line, usize::MAX, theme, style_ctx, kind_token);
    let sign_x = x + prefix_fits;
    buf.set_stringn(sign_x, y, sign.to_string(), 1, sign_style);
    if prefix_fits.saturating_add(1) >= width {
        return;
    }
    // A space between sign and text, in the sign's style: the soft/strong
    // background reads as one continuous bar under sign and text alike.
    buf.set_stringn(sign_x + 1, y, " ", 1, sign_style);

    let text_x = sign_x.saturating_add(2);
    let drawn = prefix_fits.saturating_add(2);
    if drawn >= width {
        return;
    }
    let text_width = width - drawn;
    draw_styled_text(
        buf,
        text_x,
        y,
        text_width,
        line.text.as_str(),
        sign_style,
        |index| line_style_at(line, index, theme, style_ctx, kind_token),
    );
}

/// The style for one char of `line`'s text (or, at `char_index ==
/// usize::MAX`, for its sign cell): word tint over the syntax/kind
/// foreground, which is itself over the row's soft background — reversed at
/// [`DiffRowStyle::palette16`], where none of that exists.
pub(crate) fn line_style_at(
    line: &LineView,
    char_index: usize,
    theme: &Theme,
    style_ctx: DiffRowStyle,
    kind_token: Option<Token>,
) -> Style {
    with_cursor(
        base_line_style_at(line, char_index, theme, style_ctx, kind_token),
        style_ctx,
    )
}

/// [`line_style_at`] without the cursor row's bold.
fn base_line_style_at(
    line: &LineView,
    char_index: usize,
    theme: &Theme,
    style_ctx: DiffRowStyle,
    kind_token: Option<Token>,
) -> Style {
    if style_ctx.palette16 {
        let base = foreground(theme, Token::Text);
        return match kind_token {
            Some(Token::Added) => base.add_modifier(ratatui::style::Modifier::BOLD),
            Some(Token::Removed) => base.add_modifier(ratatui::style::Modifier::DIM),
            _ => base,
        };
    }

    let word_tinted =
        style_ctx.word_tint && line.words.iter().any(|range| range.contains(&char_index));
    if let (true, Some(token)) = (word_tinted, kind_token) {
        // The word background wins over syntax; `Bg` on `Added`/`Removed` is
        // exactly the pairing `theme::chrome` already guarantees >= 4.5:1
        // contrast for, in every theme.
        return Style::new()
            .fg(color(theme.token(Token::Bg)))
            .bg(color(theme.token(token)));
    }

    let role = line
        .syntax
        .iter()
        .find(|span| span.range.contains(&char_index))
        .map(|span| span.role);
    let fg = role.map_or_else(
        || theme.token(kind_token.unwrap_or(Token::Text)),
        |role| theme.syntax(role),
    );
    let mut style = Style::new().fg(color(fg));
    if kind_token.is_some() {
        style = style.bg(color(theme.token(Token::Surface)));
    }
    style
}

/// `text` drawn cell by cell so [`line_style_at`] can style each char, fit
/// to `width` (cut with `…`, padded with `default_style` spaces). Combining
/// marks are dropped rather than stacked on their base — `fit` does that for
/// whole-line text; a diff line only needs one cell per visible character to
/// stay aligned with its gutter and word ranges.
pub(crate) fn draw_styled_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    text: &str,
    default_style: Style,
    mut style_at: impl FnMut(usize) -> Style,
) {
    let mut col: u16 = 0;
    let mut cut = false;
    for (index, character) in text.chars().enumerate() {
        let Some(cells) = character.width() else {
            continue;
        };
        if cells == 0 {
            continue;
        }
        let cells = u16::try_from(cells).unwrap_or(1);
        if col + cells > width {
            cut = true;
            break;
        }
        buf.set_stringn(
            x + col,
            y,
            character.to_string(),
            usize::from(cells),
            style_at(index),
        );
        col += cells;
    }
    if cut && width > 0 {
        let at = col.min(width - 1);
        buf.set_stringn(x + at, y, "…", 1, default_style);
        col = at + 1;
    }
    while col < width {
        buf.set_stringn(x + col, y, " ", 1, default_style);
        col += 1;
    }
}

/// A single line of plain text, fit to `area`'s width and drawn at `offset`
/// rows down from its top. Shared by the header, files and body regions.
pub(crate) fn draw_line(buf: &mut Buffer, area: Rect, offset: u16, text: &str, style: Style) {
    if offset >= area.height || area.width == 0 {
        return;
    }
    let y = area.y.saturating_add(offset);
    let fitted = fit(text, usize::from(area.width));
    buf.set_stringn(area.x, y, fitted, usize::from(area.width), style);
}
