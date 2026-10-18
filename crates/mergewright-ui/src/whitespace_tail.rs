//! The muted title tail that names the whitespace mode (`w`) when it departs
//! from a screen's default — blame, the detail pane and history all draw it
//! (m8-polish-and-ship/17b decision 3).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

/// The title tail of a screen whose default counts whitespace, while `w`
/// ignores it (detail, history).
pub(crate) const WHITESPACE_IGNORED: &str = " \u{b7} whitespace ignored";

/// The title tail of a screen whose default ignores whitespace, while `w`
/// counts it (blame).
pub(crate) const WHITESPACE_COUNTED: &str = " \u{b7} whitespace counted";

/// The title tail naming the whitespace mode, only when it departs from the
/// screen's own default (m8-polish-and-ship/17b decision 3): `None` when
/// `ignored == default_ignored`, so a title at its default says nothing.
#[must_use]
pub fn whitespace_tail(ignored: bool, default_ignored: bool) -> Option<&'static str> {
    match (ignored, default_ignored) {
        (true, false) => Some(WHITESPACE_IGNORED),
        (false, true) => Some(WHITESPACE_COUNTED),
        _ => None,
    }
}

/// `tail`, muted (DIM), right after `title` on `area`'s first row, cut at
/// the area's edge. `put` is the caller's cell writer.
pub(crate) fn put_whitespace_tail(
    buf: &mut Buffer,
    area: Rect,
    title: &str,
    tail: &str,
    put: fn(&mut Buffer, u16, u16, u16, &str, Style),
) {
    let used = u16::try_from(unicode_width::UnicodeWidthStr::width(title)).unwrap_or(u16::MAX);
    if used >= area.width {
        return;
    }
    put(
        buf,
        area.x + used,
        area.y,
        area.width - used,
        tail,
        Style::default().add_modifier(Modifier::DIM),
    );
}
