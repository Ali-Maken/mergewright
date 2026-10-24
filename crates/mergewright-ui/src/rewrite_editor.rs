//! The rewrite editor (`R`, m7-power-tools/08, note R1): his branch as rows,
//! newest first, git's letters on them, and beside (or below) them what the
//! branches will be after `y`, with the message the rewrite will write.
//!
//! Owns no state and reads no repository. `mergewright::update::rewrite_editor`
//! holds the state and `mergewright::view::rewrite_editor` turns it into the
//! [`RewriteEditorView`] this draws. The layout is one pure function over
//! the view and the area ([`layout`]); the drawing and the app's mouse
//! ([`rewrite_row_at`]) both ask it, so what is drawn and what a click lands
//! on cannot disagree.
//!
//! Every distinction is carried by **characters** (R1's distinctions table):
//! the action word and its glyph (`↓ ✎ ⏸ ✗`), `↕ moved`, `─` on the base
//! row, the `│` rule between now and after (≥180 columns), `○ new` / `● new`
//! on the AFTER rows, the `┊` gutter of the message. A capture records the
//! grid and nothing else (CLAUDE.md rule 5); colour ships unverified.
//!
//! Breakpoints (R1):
//! - ≥180: rows left ([`LEFT_WIDTH`]), `│`, AFTER the rest; the message
//!   under the rows.
//! - 120–179: rows full width; AFTER below them, labelled; the message
//!   below that.
//! - <120: rows only, or AFTER only while `Tab` has swapped to it; the
//!   message under the rows.
//!
//! The last row of the area is the notice row: a refusal in words, above
//! the app's hint bar.

use mergewright_term::SanitizedText;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::keymap::Keymap;
use crate::paint::foreground;
use crate::text_input::{FieldView as InputView, FieldWidget};
use crate::theme::{Theme, Token};

mod draw;
mod layout;
mod right;
mod view;
use draw::{Glyphs, put};
pub use draw::{RewriteEditorWidget, draw_rewrite_editor};
pub use layout::rewrite_row_at;
use layout::{RowLine, band, layout, row_lines};
use right::{draw_message, draw_right};
pub use view::{
    AfterFact, AfterMark, AfterRow, AfterView, ChangesView, EditorRow, FactTone, FieldView,
    MessageBlock, RewriteEditorView, RightSide, RowMark,
};

#[cfg(test)]
pub(crate) mod tests;

/// From this width the AFTER side stands right of the rows, past a `│`.
pub const WIDE_MIN: u16 = 180;

/// Below this width the AFTER side is a card `Tab` swaps to.
pub const NARROW_BELOW: u16 = 120;

/// The rows' width at [`WIDE_MIN`] and above; the `│` is the next column.
pub const LEFT_WIDTH: u16 = 96;

/// At this many area rows or fewer the message shows
/// [`SHORT_MESSAGE_LINES`] lines and `…`.
pub const SHORT_HEIGHT: u16 = 24;

/// The message lines shown on a short screen.
pub const SHORT_MESSAGE_LINES: usize = 4;

/// Cells before the action word: a space, the cursor (`▸`) or the base's
/// `─`, a space.
const LEAD: usize = 3;

/// Cells the action word and its glyph take, gap included (`squash ↓  `).
const ACTION_WIDTH: usize = 10;

/// Cells a short sha takes, gap included.
const SHORT_WIDTH: usize = 9;

/// The subject column's bounds: as wide as the longest subject, within
/// these.
const SUBJECT_MIN: usize = 12;
const SUBJECT_MAX: usize = 30;

/// The rows that keep their natural height before anything below them does:
/// fewer than this and the rows scroll.
const ROWS_MIN: u16 = 3;
