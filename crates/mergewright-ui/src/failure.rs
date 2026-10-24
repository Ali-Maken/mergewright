//! The one failure mark (ease-pass 26, ADR-0067): `⊗ <why>`, ASCII
//! `(x) <why>`. Producers and view models carry the words only; whatever
//! draws a failure calls here, so no screen can place the mark apart or
//! spell it its own way. `✗` stays a commit no branch holds (J), `!` needs
//! attention, `⚠` warns: none of them is a failure.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::graph_screen::{ProblemKind, ProblemLine, fit_status_line};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The failure mark, Unicode tier.
pub const FAILED_MARK: &str = "\u{2297}";
/// The failure mark, ASCII tier.
pub const FAILED_MARK_ASCII: &str = "(x)";
/// The mark's ink. The one place: an `error` token, if PROMPT §5 ever gains
/// one, replaces this line. Colour ships unverified; the glyph is the carrier.
pub const FAILED_INK: Token = Token::Removed;

/// Marks the words of a failure must not already start with.
const FOREIGN_MARKS: [&str; 5] = [FAILED_MARK, FAILED_MARK_ASCII, "\u{2717}", "\u{26a0}", "!"];

/// The mark for a tier.
#[must_use]
pub fn mark(ascii: bool) -> &'static str {
    if ascii {
        FAILED_MARK_ASCII
    } else {
        FAILED_MARK
    }
}

/// Cells the mark and its space take: what a caller subtracts from its
/// width before fitting the words, so the words are cut and the mark kept.
#[must_use]
pub fn lead_width(ascii: bool) -> usize {
    mark(ascii).width() + 1
}

pub(crate) fn check_unmarked(why: &str) {
    debug_assert!(
        !FOREIGN_MARKS.iter().any(|m| why.starts_with(m)),
        "failure words must be unmarked, the renderer adds the mark: {why:?}"
    );
}

/// `⊗ <why>` (ASCII `(x) <why>`): the one way a failure reads.
/// Debug-asserts that `why` does not start with a mark.
#[must_use]
pub fn marked(why: &str, ascii: bool) -> String {
    check_unmarked(why);
    format!("{} {why}", mark(ascii))
}

/// [`marked`] when `failed`, else `why` as it is.
#[must_use]
pub fn marked_if(failed: bool, why: &str, ascii: bool) -> String {
    if failed {
        marked(why, ascii)
    } else {
        why.to_owned()
    }
}

/// The same line as styled spans: the mark in [`FAILED_INK`], a space, then
/// `why` in `words_style`. Under a colourless theme the mark has no colour
/// and still draws.
#[must_use]
pub fn spans<'a>(why: &'a str, ascii: bool, theme: &Theme, words_style: Style) -> Vec<Span<'a>> {
    check_unmarked(why);
    vec![
        Span::styled(mark(ascii), foreground(theme, FAILED_INK)),
        Span::styled(" ", words_style),
        Span::styled(why, words_style),
    ]
}

/// Paints the failure mark in [`FAILED_INK`] wherever a body drew it inside
/// `area`. A body that draws its words as one string in one style calls this
/// once at the end of its render: `⊗` is no other mark (the glyph test), so
/// every one found is a failure's. At the ASCII tier the mark is `(x)`
/// followed by a space. Under a colourless theme the glyph keeps no colour.
pub fn ink_marks(buf: &mut Buffer, area: Rect, theme: &Theme, ascii: bool) {
    let ink = foreground(theme, FAILED_INK);
    let area = area.intersection(*buf.area());
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let hit = if ascii {
                x + 3 < area.right()
                    && buf[(x, y)].symbol() == "("
                    && buf[(x + 1, y)].symbol() == "x"
                    && buf[(x + 2, y)].symbol() == ")"
                    && buf[(x + 3, y)].symbol() == " "
            } else {
                buf[(x, y)].symbol() == FAILED_MARK
            };
            if hit {
                let cells = if ascii { 3 } else { 1 };
                for at in x..x + cells {
                    buf[(at, y)].set_style(ink);
                }
            }
        }
    }
}

/// [`spans`] as one owned line, for a screen that builds its lines up.
#[must_use]
pub fn line(why: &str, ascii: bool, theme: &Theme, words_style: Style) -> Line<'static> {
    Line::from(
        spans(why, ascii, theme, words_style)
            .into_iter()
            .map(|span| Span::styled(span.content.into_owned(), span.style))
            .collect::<Vec<_>>(),
    )
}

/// A bar row's problem, one row `width` cells wide: a leading space, then
/// for a [`ProblemKind::Failure`] the mark and a space, then the reason cut
/// to what is left (`fit_status_line`: the reason first, the keys whole).
/// The words are cut and the mark is kept. A `Notice` draws its words alone.
#[must_use]
pub fn status_line(
    problem: &ProblemLine,
    ascii: bool,
    width: u16,
    theme: &Theme,
    words_style: Style,
) -> Line<'static> {
    let inner = usize::from(width.saturating_sub(1));
    let keys = problem
        .keys
        .as_ref()
        .map(mergewright_term::SanitizedText::as_str);
    let room = |taken: usize| u16::try_from(inner.saturating_sub(taken)).unwrap_or(u16::MAX);
    match problem.kind {
        ProblemKind::Notice => Line::from(vec![
            Span::styled(" ", words_style),
            Span::styled(
                fit_status_line(problem.reason.as_str(), keys, room(0)),
                words_style,
            ),
        ]),
        ProblemKind::Failure => {
            check_unmarked(problem.reason.as_str());
            let words = fit_status_line(problem.reason.as_str(), keys, room(lead_width(ascii)));
            Line::from(vec![
                Span::styled(" ", words_style),
                Span::styled(mark(ascii), foreground(theme, FAILED_INK)),
                Span::styled(" ", words_style),
                Span::styled(words, words_style),
            ])
        }
    }
}

#[cfg(test)]
mod body_tests;
#[cfg(test)]
mod tests;

/// The shared check of every failed-body test (boxes B1, B2): `grid` holds
/// exactly one `⊗ <words>` (ASCII: `(x) <words>` and no `⊗`).
#[cfg(test)]
pub(crate) fn assert_marked(grid: &str, words: &str, ascii: bool) {
    let want = format!("{} {words}", mark(ascii));
    assert_eq!(
        grid.matches(&want).count(),
        1,
        "one `{want}` expected in:\n{grid}"
    );
    if ascii {
        assert!(!grid.contains(FAILED_MARK), "ASCII tier drew ⊗:\n{grid}");
    }
}

/// `draw` run on a `width` x `height` test frame; its rows joined by `\n`.
#[cfg(test)]
pub(crate) fn frame_text(
    width: u16,
    height: u16,
    draw: impl FnOnce(&mut ratatui::Frame),
) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
        .expect("a test backend opens");
    terminal.draw(draw).expect("a test backend draws");
    let buf = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
