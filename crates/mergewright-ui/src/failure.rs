//! The one failure mark (ease-pass 26, ADR-0067): `⊗ <why>`, ASCII
//! `(x) <why>`. Producers and view models carry the words only; whatever
//! draws a failure calls here, so no screen can place the mark apart or
//! spell it its own way. `✗` stays a commit no branch holds (J), `!` needs
//! attention, `⚠` warns: none of them is a failure.

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

fn check_unmarked(why: &str) {
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
mod tests;
