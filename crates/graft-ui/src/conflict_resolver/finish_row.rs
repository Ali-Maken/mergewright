//! The finish row (m6-conflicts/60): once every file is done, the notice
//! row offers `c` as the one next action. Its words, where a click on it
//! lands, and how it is drawn.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use graft_term::SanitizedText;

use super::draw::set_text;
use super::{DocView, MIN_COLUMNS, MIN_ROWS, Placement, ResolverScreen, resolver_areas};
use crate::paint::foreground;
use crate::theme::Token;

/// The finish row's words, full and short: `✓ every file done — c review
/// and finish the merge` / `✓ every file done — c review & finish`. The
/// app words them (it knows the operation); the widget picks by width and
/// cuts the short one with `…` when even that does not fit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinishRow {
    /// The words when they fit the row.
    pub full: SanitizedText,
    /// The words when [`Self::full`] does not.
    pub short: SanitizedText,
}

/// Whether (`col`, `row`) is on the finish row of a resolver drawn in
/// `area` with `placement` — a click there does what `c` does. `false`
/// when no finish row is drawn.
#[must_use]
pub fn finish_row_at(doc: &DocView, placement: Placement, area: Rect, col: u16, row: u16) -> bool {
    if doc.finish.is_none() || area.width < MIN_COLUMNS || area.height < MIN_ROWS {
        return false;
    }
    let page = resolver_areas(area, placement).doc;
    page.height > 1 && row == page.y + 1 && col >= page.x && col < page.right()
}

impl ResolverScreen<'_> {
    /// The finish row on the notice row: `accent` BOLD on a full-width
    /// `selection` fill, the strongest fill on the screen because it is the
    /// next action; REVERSED where there is no colour to fill with
    /// (`NO_COLOR`). The full words when they fit, else the short ones, cut
    /// with `…` when even those do not.
    pub(super) fn draw_finish_row(&self, buf: &mut Buffer, area: Rect, finish: &FinishRow) {
        use ratatui::style::Modifier;
        let width = usize::from(area.width);
        let full = finish.full.as_str();
        let words = if crate::frame_box::cells(full) <= width {
            full
        } else {
            finish.short.as_str()
        };
        let style = if self.theme.color_depth() == graft_core::capabilities::ColorDepth::None {
            ratatui::style::Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            // `accent`, lifted to stay legible on the fill (ADR-0058 §3).
            crate::paint::on_selection(self.theme, foreground(self.theme, Token::Accent))
                .bg(crate::paint::color(self.theme.token(Token::Selection)))
                .add_modifier(Modifier::BOLD)
        };
        set_text(
            buf,
            area.x,
            area.y + 1,
            area,
            &crate::fit::fit(words, width),
            style,
        );
    }
}
