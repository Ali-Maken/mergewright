//! The screen Graft opens on: what it opened, and what it is drawing with.
//!
//! M0's only screen, and the shape every later one follows. It renders a
//! [`StatusView`] — plain data that `app` assembles — and holds no repository,
//! no capabilities and no config of its own. Every string that came out of a
//! repository is typed [`SanitizedText`], so a branch name with an escape
//! sequence in it cannot reach the terminal as anything but visible text
//! (DESIGN.md §8.2).
//!
//! Deliberately ASCII, at every tier: this screen is what a user sees when
//! they are finding out what Graft detected, and a box-drawing character in
//! the wrong terminal would be the first thing they had to discount.

#[cfg(test)]
mod tests;

use graft_core::capabilities::{CellSize, CellSizeSource, ColorDepth, RenderingTier};
use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::paint::foreground;
use crate::theme::{Theme, ThemeKind, Token};

/// How wide the label column is, in cells. Wide enough for the longest label
/// with a gap after it.
const LABEL_WIDTH: usize = 12;

/// Everything the status screen draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusView {
    /// The repository, once it is open. `None` while the job that opens it is
    /// still running — the first frame is drawn before it answers.
    pub repository: Option<RepositoryView>,
    /// What Graft decided to draw with, and how sure it is.
    pub rendering: RenderingView,
    /// Advisories: a git that is too old, a config key nobody recognises.
    /// Empty means there is nothing to say, and nothing is drawn.
    pub notes: Vec<SanitizedText>,
    /// Something went wrong and has not been superseded. Absent when nothing
    /// is wrong — there is no permanent error field on this screen (CLAUDE.md).
    pub problem: Option<SanitizedText>,
    /// The history walk, once one has been started. `None` before that, and
    /// nothing is drawn for it.
    pub history: Option<HistoryView>,
}

/// How far the history walk has got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryView {
    /// Where the walk is.
    pub state: HistoryState,
    /// Rows the layout engine has laid out so far.
    pub rows: usize,
}

/// Where a history walk is. Each state is named for itself, so a walk that is
/// fine never reads as an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryState {
    /// Rows are still arriving.
    Streaming,
    /// Every commit has a row.
    Complete,
    /// The user stopped it; the rows already laid out stay.
    Cancelled,
    /// It broke, and this is why.
    Failed(SanitizedText),
}

/// The repository Graft opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryView {
    /// The working tree's root, or the repository's own directory when it is
    /// bare.
    pub path: SanitizedText,
    /// No working tree.
    pub bare: bool,
    /// Where `HEAD` points.
    pub head: HeadView,
}

/// Where `HEAD` points, worded for a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadView {
    /// On a branch, with commits on it. The short name: `main`.
    Branch(SanitizedText),
    /// Detached: a commit id, abbreviated.
    Detached(SanitizedText),
    /// On a branch that has no commits yet.
    Unborn(SanitizedText),
    /// `HEAD` could not be read, and this is why. A repository can be open and
    /// still have an unreadable `HEAD`, and saying nothing would leave the row
    /// blank with no explanation.
    Unreadable(SanitizedText),
}

/// What Graft is drawing with, and where each answer came from.
///
/// Provenance is shown rather than hidden: a guessed cell size and a terminal
/// that never reported its background are the two things that silently produce
/// a wrong-looking screen, and a user comparing terminals needs to see which
/// of them is a measurement (DESIGN.md §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderingView {
    /// The tier in effect — detection's answer, unless config overrode it.
    pub tier: RenderingTier,
    /// Where that tier came from, and — when nothing was measured — why not.
    pub tier_source: TierSource,
    /// The size of one character cell, and whether anything measured it.
    pub cell_size: CellSize,
    /// The theme in effect.
    pub theme: ThemeKind,
    /// The theme came from `ui.theme`, not from the terminal's background.
    pub theme_forced: bool,
    /// The terminal never said what its background is, so the theme is a
    /// default rather than a match.
    pub theme_unknown: bool,
    /// How much colour the terminal has.
    pub color_depth: ColorDepth,
    /// `NO_COLOR` is set: the chrome is drawn without colour whatever the
    /// terminal can do.
    pub no_color: bool,
}

/// Where the rendering tier in effect came from.
///
/// The tier is a conclusion, and a conclusion drawn from silence is not the
/// same as one drawn from an answer. Every variant below that is not
/// [`Detected`](TierSource::Detected) means the terminal never told Graft
/// anything: the tier is then a fallback from environment hints, and the
/// screen says so rather than printing `detected` over a guess (CLAUDE.md's
/// status-field rule). Mirrors [`CellSizeSource`] deliberately — the two
/// answer the same question about two different fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TierSource {
    /// `graph.renderer` in the config file set it. Detection did not decide,
    /// and may well have concluded something else.
    Forced,
    /// The probe round finished — the queries went out and the fence came
    /// back — so the tier rests on what the terminal actually answered.
    Detected,
    /// The terminal was asked and said nothing before the deadline. The tier
    /// is a fallback from environment hints, not a measurement.
    Silent,
    /// The terminal could not be written to or read from, so the probe never
    /// got an answer to fall back *from*. The error itself is a note on the
    /// screen; this only records that the tier is a fallback.
    ProbeFailed,
    /// Nothing was asked: output is not a terminal, or `TERM=dumb`.
    NotProbed,
}

/// The status screen, ready to render.
pub struct StatusScreen<'a> {
    view: &'a StatusView,
    theme: &'a Theme,
}

impl<'a> StatusScreen<'a> {
    /// Draw `view` in `theme`.
    #[must_use]
    pub fn new(view: &'a StatusView, theme: &'a Theme) -> Self {
        Self { view, theme }
    }

    /// Every line of the screen, in order.
    fn lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(Span::styled(
                "Tributary",
                foreground(self.theme, Token::Accent),
            )),
            Line::default(),
        ];

        match &self.view.repository {
            Some(repository) => {
                lines.push(self.row("Repository", &location(repository), Token::Text));
                lines.push(self.row("HEAD", &head(&repository.head), Token::Text));
            }
            None => {
                // Not blank, and not an error: the job that opens the
                // repository is still running, and this is the first frame.
                lines.push(self.row("Repository", "opening", Token::TextMuted));
            }
        }
        if let Some(history) = &self.view.history {
            lines.push(history_row(self, history));
        }
        lines.push(self.row("Rendering", &rendering(&self.view.rendering), Token::Text));
        lines.push(self.row("Theme", &theming(&self.view.rendering), Token::Text));

        if !self.view.notes.is_empty() {
            lines.push(Line::default());
            for note in &self.view.notes {
                lines.push(Line::from(Span::styled(
                    note.as_str().to_owned(),
                    foreground(self.theme, Token::Info),
                )));
            }
        }

        if let Some(problem) = &self.view.problem {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                problem.as_str().to_owned(),
                foreground(self.theme, Token::Warning),
            )));
        }

        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "q  quit",
            foreground(self.theme, Token::TextMuted),
        )));
        lines
    }

    /// One labelled row: a muted label in a fixed column, then the value.
    fn row(&self, label: &str, value: &str, token: Token) -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("{label:LABEL_WIDTH$}"),
                foreground(self.theme, Token::TextMuted),
            ),
            Span::styled(value.to_owned(), foreground(self.theme, token)),
        ])
    }
}

/// `History     streaming, 412 rows laid out`.
fn history_row(screen: &StatusScreen<'_>, history: &HistoryView) -> Line<'static> {
    let (state, token) = match &history.state {
        HistoryState::Streaming => ("streaming".to_owned(), Token::Text),
        HistoryState::Complete => ("complete".to_owned(), Token::Text),
        HistoryState::Cancelled => ("cancelled".to_owned(), Token::TextMuted),
        HistoryState::Failed(reason) => (format!("failed: {}", reason.as_str()), Token::Warning),
    };
    let value = format!("{state}, {} rows laid out", history.rows);
    screen.row("History", &value, token)
}

impl Widget for StatusScreen<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            // A pty that has not been sized yet (ADR-0012). Nothing to draw
            // into, and nothing wrong with that.
            return;
        }

        Paragraph::new(self.lines())
            // Wrapped rather than truncated: on a narrow terminal a cut-off
            // path is worse than a path over two lines, and this screen
            // exists to be read.
            .wrap(Wrap { trim: false })
            .render(area, buffer);
    }
}

/// The repository row's value: where it is, and whether it has a working tree.
fn location(repository: &RepositoryView) -> String {
    if repository.bare {
        format!("{} (bare)", repository.path)
    } else {
        repository.path.as_str().to_owned()
    }
}

/// The `HEAD` row's value.
fn head(head: &HeadView) -> String {
    match head {
        HeadView::Branch(name) => name.as_str().to_owned(),
        HeadView::Detached(id) => format!("detached at {id}"),
        HeadView::Unborn(name) => format!("{name} (no commits yet)"),
        HeadView::Unreadable(why) => format!("unreadable: {why}"),
    }
}

/// The rendering row's value: the tier, where it came from, and the cell size
/// the pixel tier would measure against.
fn rendering(view: &RenderingView) -> String {
    let source = match view.tier_source {
        TierSource::Forced => "set by graph.renderer",
        TierSource::Detected => "detected",
        // Not "detected": nothing was. Each of these is a fallback worked out
        // from environment hints, and the wording says which wall was hit so
        // that a user comparing terminals can tell a silent terminal from one
        // Graft never got to ask.
        TierSource::Silent => "assumed (terminal silent)",
        TierSource::ProbeFailed => "assumed (probe failed)",
        TierSource::NotProbed => "assumed (terminal not asked)",
    };
    let CellSize {
        width,
        height,
        source: measured,
    } = view.cell_size;
    let measured = match measured {
        CellSizeSource::Reported => "reported",
        CellSizeSource::WindowSize => "from the window size",
        CellSizeSource::Assumed => "assumed",
    };
    format!(
        "{}, {source}, cell {width}x{height} ({measured})",
        tier_name(view.tier)
    )
}

/// The theme row's value: which theme, why that one, and how much colour it
/// has to work with.
fn theming(view: &RenderingView) -> String {
    let source = if view.theme_forced {
        "set by ui.theme"
    } else if view.theme_unknown {
        // The terminal answered nothing, so this is a default rather than a
        // match — and the fix is a line in the config file.
        "terminal background unknown"
    } else {
        "from the terminal background"
    };
    let color = if view.no_color {
        "no colour (NO_COLOR)"
    } else {
        depth_name(view.color_depth)
    };
    format!("{}, {source}, {color}", view.theme.name())
}

/// How a tier is spelled on screen — the same word the config file uses.
fn tier_name(tier: RenderingTier) -> &'static str {
    match tier {
        RenderingTier::Pixel => "pixel",
        RenderingTier::Unicode => "unicode",
        RenderingTier::Ascii => "ascii",
    }
}

/// How much colour the terminal has, in words.
fn depth_name(depth: ColorDepth) -> &'static str {
    match depth {
        ColorDepth::TrueColor => "truecolor",
        ColorDepth::Palette256 => "256 colours",
        ColorDepth::Palette16 => "16 colours",
        ColorDepth::None => "no colour",
    }
}
