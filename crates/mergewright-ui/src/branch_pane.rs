//! The branch pane (ease-pass G12): right of the graph, the focused branch's
//! four answers — pushed? merged? behind what it was built on? who and when?
//! — then the route it was built on, trunk first.
//!
//! The pane is drawn only at [`PANE_FROM`] columns and wider
//! ([`pane_width`]). Its area is the content width plus one column on its
//! left for the `│` that parts it from the graph: the widget draws that rule
//! itself, so the frame reserves `pane_width + 1` columns.

use mergewright_graph::role::LaneRole;
use mergewright_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::graph_column::role_token;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The narrowest terminal that gets the pane.
pub const PANE_FROM: u16 = 100;
/// The pane's content width at 120 columns and wider.
pub const PANE_WIDE: u16 = 36;
/// The pane's content width from [`PANE_FROM`] to 119 columns.
pub const PANE_MIDDLE: u16 = 30;
/// Where [`PANE_WIDE`] starts.
const WIDE_FROM: u16 = 120;

/// The pane's content width for a terminal `columns` wide, without its `│`
/// rule; `None` below [`PANE_FROM`], where the cursor row's second line
/// says who, when and which branch instead.
#[must_use]
pub fn pane_width(columns: u16) -> Option<u16> {
    if columns >= WIDE_FROM {
        Some(PANE_WIDE)
    } else if columns >= PANE_FROM {
        Some(PANE_MIDDLE)
    } else {
        None
    }
}

/// One answer's mark: `!` needs you, `✓` fine, `·` plain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// `!`: something for you to do; mark and words in `warning`.
    Attention,
    /// `✓`: fine as it is.
    Fine,
    /// `·`: worth knowing, nothing to do.
    Plain,
}

impl Mark {
    /// The glyph the line starts with.
    #[must_use]
    pub const fn glyph(self) -> char {
        match self {
            Self::Attention => '!',
            Self::Fine => '✓',
            Self::Plain => '·',
        }
    }

    const fn token(self) -> Token {
        match self {
            Self::Attention => Token::Warning,
            Self::Fine => Token::RoleThis,
            Self::Plain => Token::TextMuted,
        }
    }
}

/// One of the four answer lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneLine {
    /// The read has not landed: `reading…`, muted.
    Reading,
    /// The answer.
    Known {
        /// Its mark.
        mark: Mark,
        /// Its words, mark excluded.
        words: SanitizedText,
    },
    /// The read failed: `can't tell: <reason>`, on this line only.
    Failed(SanitizedText),
}

/// One stop on the route: a branch name in its role's colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteStop {
    /// The short branch name.
    pub name: SanitizedText,
    /// Its role to the focused branch.
    pub role: LaneRole,
}

/// What the pane shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchPaneView {
    /// The cursor row is on a branch.
    Branch {
        /// The branch's short name.
        heading: SanitizedText,
        /// Its role (the focus is `This`; the trunk focused reads `This` too).
        role: LaneRole,
        /// It is `HEAD`'s branch: `(you're here)`.
        here: bool,
        /// Trunk first, the branch itself last; `None` until the read lands.
        route: Option<Vec<RouteStop>>,
        /// pushed, merged, behind, who.
        lines: [PaneLine; 4],
    },
    /// The cursor row is on a commit no branch holds.
    NotOnBranch {
        /// Its seven-character id.
        sha7: SanitizedText,
    },
}

/// The pane widget: the `│` rule in its first column, the view right of it.
pub struct BranchPane<'a> {
    /// What to draw.
    pub view: &'a BranchPaneView,
    /// Colours.
    pub theme: &'a Theme,
    /// The ASCII tier: a failed line's mark is `(x)`.
    pub ascii: bool,
}

/// Between a stop and the one before it on the route.
pub const ROUTE_JOIN: &str = " ─◆ ";
/// The words before the route.
pub const BUILT_ON: &str = "built on";
/// A line whose read has not landed.
pub const READING: &str = "reading…";

impl Widget for BranchPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 2 || area.height == 0 {
            return;
        }
        let rule = foreground(self.theme, Token::Border);
        for y in area.top()..area.bottom() {
            buf.set_string(area.x, y, "│", rule);
        }
        let content = Rect {
            x: area.x + 2,
            width: area.width.saturating_sub(2),
            ..area
        };
        let mut writer = Writer {
            buf,
            area: content,
            row: 0,
        };
        let muted = foreground(self.theme, Token::TextMuted);
        match self.view {
            BranchPaneView::NotOnBranch { sha7 } => {
                writer.line(&[(&format!("{} · not on a branch", sha7.as_str()), muted)]);
            }
            BranchPaneView::Branch {
                heading,
                role,
                here,
                route,
                lines,
            } => {
                let head = foreground(self.theme, role_token(*role)).add_modifier(Modifier::BOLD);
                let mut spans: Vec<(&str, Style)> = vec![(heading.as_str(), head)];
                if *here {
                    spans.push(("  (you're here)", muted));
                }
                writer.line(&spans);
                for line in lines {
                    self.answer(&mut writer, line);
                }
                if let Some(stops) = route {
                    writer.row += 1;
                    writer.line(&[(BUILT_ON, muted)]);
                    for (nth, stop) in stops.iter().enumerate() {
                        let style = foreground(self.theme, role_token(stop.role))
                            .add_modifier(Modifier::BOLD);
                        let join = if nth == 0 { "" } else { ROUTE_JOIN };
                        writer.line(&[("  ", muted), (join, style), (stop.name.as_str(), style)]);
                    }
                }
            }
        }
    }
}

impl BranchPane<'_> {
    fn answer(&self, writer: &mut Writer<'_>, line: &PaneLine) {
        let muted = foreground(self.theme, Token::TextMuted);
        match line {
            PaneLine::Reading => writer.line(&[(READING, muted)]),
            PaneLine::Known { mark, words } => {
                let ink = foreground(self.theme, mark.token());
                let words_ink = if *mark == Mark::Attention {
                    ink
                } else {
                    foreground(self.theme, Token::Text)
                };
                let glyph = format!("{} ", mark.glyph());
                // Long answers wrap under their words, not under the mark:
                // the mock's `development has 4 new commits since it left`
                // is wider than the pane.
                let room = usize::from(writer.area.width.saturating_sub(2));
                let mut pieces = wrap(words.as_str(), room).into_iter();
                let first = pieces.next().unwrap_or_default();
                writer.line(&[(&glyph, ink), (&first, words_ink)]);
                for rest in pieces {
                    writer.line(&[("  ", words_ink), (&rest, words_ink)]);
                }
            }
            PaneLine::Failed(reason) => {
                // The mark stands in the slot `!`/`✓`/`·` take on a known line.
                let warn = foreground(self.theme, Token::Warning);
                let mark = format!("{} ", crate::failure::mark(self.ascii));
                let words = format!("can't tell: {}", reason.as_str());
                crate::failure::check_unmarked(&words);
                writer.line(&[
                    (&mark, foreground(self.theme, crate::failure::FAILED_INK)),
                    (&words, warn),
                ]);
            }
        }
    }
}

/// `text` broken at spaces into lines of at most `room` cells. A word wider
/// than `room` stays whole on its own line and is cut with `…` when drawn.
fn wrap(text: &str, room: usize) -> Vec<String> {
    let width = |s: &str| {
        s.chars()
            .filter_map(unicode_width::UnicodeWidthChar::width)
            .sum::<usize>()
    };
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        if current.is_empty() {
            current.push_str(word);
        } else if width(&current) + 1 + width(word) <= room {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    lines.push(current);
    lines
}

/// Lines written top-down, each cut to the content width.
struct Writer<'b> {
    buf: &'b mut Buffer,
    area: Rect,
    row: u16,
}

impl Writer<'_> {
    fn line(&mut self, spans: &[(&str, Style)]) {
        if self.row >= self.area.height || self.area.width == 0 {
            self.row += 1;
            return;
        }
        let y = self.area.y + self.row;
        let mut x = self.area.x;
        let right = self.area.right();
        for (text, style) in spans {
            let room = usize::from(right.saturating_sub(x));
            if room == 0 {
                break;
            }
            // `fit` pads to `room`; the next span starts after the text.
            let wide = text
                .chars()
                .filter_map(unicode_width::UnicodeWidthChar::width)
                .sum::<usize>();
            let used = wide.min(room);
            self.buf.set_stringn(x, y, fit(text, used), used, *style);
            x = x.saturating_add(u16::try_from(used).unwrap_or(u16::MAX));
        }
        self.row += 1;
    }
}

#[cfg(test)]
mod tests;
