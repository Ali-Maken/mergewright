//! The conflict resolver as a pure widget (`docs/design-notes/m6-conflicts.md`
//! Decision 1, "New UI").
//!
//! One file, drawn as a document: plain lines as they are, each conflict
//! block between git's `<<<<<<<`/`>>>>>>>` drawn as the merge mock's clash
//! (ease-pass/11): an open clash marked `?`, **yours and theirs side by
//! side** from [`INLINE_FROM`] frame columns or **stacked** below, its
//! lines unnumbered; the cursor's clash outlined. BASE is a stacked section
//! inside it, shown on demand ([`DocView::base_shown`]) at every width. A
//! taken block shrinks to the lines the pick wrote, numbered as plain
//! lines and marked `◀ kept yours` / `▶ took theirs` / `◆ kept both`.
//!
//! The three panes are retired (ease-pass/11b): from [`INLINE_FROM`] the
//! document is inline at every width; [`arrangement`] says which.
//!
//! [`layout`] is pure and total: it lays out `doc` into [`ScreenRow`]s for
//! any `columns >= 1`, and [`ResolverScreen`] — the [`Widget`] — draws only
//! `layout.rows[top..top + height]`, nothing outside its `Rect`. The app
//! scrolls and hit-tests against the same rows the widget draws
//! ([`row_at`]); there is no second copy of the layout.
//!
//! **Line numbers are per side** (ADR-0043 §5): each side is numbered
//! contiguously by its own projection (`mergewright_git::conflict::Projection`).
//! An open card's yours lines count from [`BlockNumbers::ours`], its theirs
//! lines from [`BlockNumbers::theirs`]; plain and taken lines carry the
//! result's number. BASE and every marker row (header, labels, end, note)
//! carry none. The number column is as wide as the widest number drawn.
//!
//! **Names** come from [`SideNames`], one per side (`yours · main`); git's
//! marker labels are never drawn.
//!
//! **What this ticket does not build** (`m6-conflicts/02-ui-conflict-resolver.md`
//! scope; G4/G5's job): key handling, scrolling state, hit-testing beyond
//! [`row_at`], colour/theme entries beyond `theme.cursor`/`theme.frame`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use mergewright_term::SanitizedText;

use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

mod arrange;
mod draw;
mod files;
mod finish_row;
mod layout;
mod mark;
mod names;

pub use arrange::{Arrangement, INLINE_FROM, LIST_COLUMN, arrangement};
use draw::set_text;
pub use files::{
    CombinedList, FileList, FileListView, FileRow, FileRowLook, ListTail, Placement, ResolverAreas,
    file_at, placement, resolver_areas, review_row_at,
};
pub use finish_row::{FinishRow, finish_row_at};
pub use layout::{Layout, ScreenRow, layout, row_at};
pub use mark::Mark;
pub use names::{BlockNumbers, SideNames, SideRoles, number_width};

/// Screen rows the widget reserves above the document itself: the title row
/// and one more for the notice (blank while none is set).
pub const HEADER_ROWS: u16 = 2;

/// Cells left of every row's text (ease-pass/11): the outline's `│` round
/// the cursor's clash, the clash's mark (`?` `◀` `▶` `◆`), and a space
/// before the line number — on every row, so the text stays put whichever
/// clash has the cursor.
const GUTTER: u16 = 3;

/// The single line the widget draws when it has no room to draw anything
/// else. `layout` still returns rows at this size — the app decides no key
/// acts, this widget only decides what is legible.
const TOO_SMALL: &str = "resolver needs 60x10";
/// The narrowest the resolver draws its document at; below this it draws
/// [`TOO_SMALL`] instead.
///
/// Public because the sentence above [`TOO_SMALL`] makes it the app's
/// business: *the app decides no key acts.* It cannot decide that against a
/// number it cannot see, and a second copy of `60` in `update::resolver`
/// would be the widget's floor and the keyboard's floor drifting apart
/// (m6-conflicts/04).
pub const MIN_COLUMNS: u16 = 60;
/// The shortest the resolver draws its document at, on the same terms as
/// [`MIN_COLUMNS`]. Counted over the rows the resolver actually gets, not
/// over the terminal.
pub const MIN_ROWS: u16 = 10;

/// Everything the resolver draws for one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocView {
    /// The file identity row, `notes.txt · 2 conflicts · 1 open` — the app
    /// builds the wording; the widget draws it after its own `▸`, which
    /// repeats the list's.
    pub title: SanitizedText,
    /// The file list drawn beside or above the document.
    pub files: FileListView,
    /// Each side's one name (card labels, took words, header keys).
    pub names: SideNames,
    /// The document's regions, in file order.
    pub regions: Vec<RegionView>,
    /// The block the cursor is on, if any: its header draws reversed and
    /// every row of its card carries the gutter rule.
    pub cursor_block: Option<usize>,
    /// `B` toggled BASE on: every open block with a base draws it.
    pub base_shown: bool,
    /// Drawn on the row under the title while set: `(text, is_error)`.
    /// `is_error` is not drawn differently (ADR-0041 — the words carry the
    /// distinction, not colour).
    pub notice: Option<(SanitizedText, bool)>,
    /// Every file is done and the summary is not open (m6-conflicts/60):
    /// drawn on the notice row **instead of** [`Self::notice`], as the one
    /// next action. The app leaves it `None` while a problem notice stands,
    /// which outranks it.
    pub finish: Option<FinishRow>,
}

/// One region of the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionView {
    /// Lines outside any conflict block, drawn as-is.
    Plain {
        /// This region's first line in each projection: the document
        /// draws `numbers.result`.
        numbers: BlockNumbers,
        /// The lines themselves.
        lines: Vec<SanitizedText>,
    },
    /// A conflict block: open, or taken and kept until the resolver closes.
    Block(BlockView),
    /// A whole-file conflict shape (UD/DU/AU/UA/DD, binary): one card, then
    /// the file's lines (or none) as plain text.
    WholeFile {
        /// `deleted by them`, `added by us`, … — the shape, worded.
        how: SanitizedText,
        /// The keys that apply here (`o keeps the file  t deletes it`).
        keys: SanitizedText,
        /// The file as it is (empty for a delete) — as far as the app read
        /// it, which for a large file is its head only.
        lines: Vec<SanitizedText>,
        /// One line under them, muted and **unnumbered** (m6-05 review,
        /// Major 4): how much of a long file was left unread, or a binary
        /// file's size in place of its bytes. Not a line of the file, so
        /// it never carries a line number that would claim it is one.
        note: Option<SanitizedText>,
    },
    /// After the last block of a file with none left open, the file
    /// stays on screen and says so (the merge mock, M9): `✓ Every clash in
    /// <name> is solved.`, then `next`, the way on in the app's words.
    Solved {
        /// The file's last path segment.
        name: SanitizedText,
        /// What to do now: `n next file, 2 clashes left`, `y review the
        /// merge`; empty for none.
        next: SanitizedText,
    },
}

/// One conflict block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockView {
    /// 0-based position among every block in the file.
    pub index: usize,
    /// How many blocks the file has.
    pub of: usize,
    /// Open, or taken.
    pub state: BlockState,
    /// Our side's lines.
    pub ours: Vec<SanitizedText>,
    /// The base's label and lines, when attached (`attach_base`); `None`
    /// when unavailable (the notice line says why).
    pub base: Option<(SanitizedText, Vec<SanitizedText>)>,
    /// Their side's lines.
    pub theirs: Vec<SanitizedText>,
    /// The block's position in the file, 1-based: while `Open`, the line
    /// its `<<<<<<<` marker sits on; once `Took`, its first taken line
    /// (matching `mergewright_git::conflict::BlockAt::first_line`'s own doc —
    /// the marker line no longer exists once resolved). Where `e` opens
    /// the editor; never drawn.
    pub first_line: usize,
    /// Where each side's lines are numbered from.
    pub numbers: BlockNumbers,
    /// Once taken, the lines the pick wrote, as the app's resolver state
    /// holds them: the widget draws them and never derives them from the
    /// sides. Empty while open.
    pub kept: Vec<SanitizedText>,
}

/// Whether a block still has a decision to make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockState {
    /// Not yet resolved.
    Open,
    /// Resolved, kept on screen with its header until the resolver closes.
    Took(TakeView),
}

/// Which side a taken block kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TakeView {
    /// `o`.
    Ours,
    /// `t`.
    Theirs,
    /// `b`: ours then theirs, git's smerge order.
    Both,
}

/// Which side a [`ScreenRow::Side`] row draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideKind {
    /// A line of `BlockView::ours`.
    Ours,
    /// A line of the base.
    Base,
    /// A line of `BlockView::theirs`.
    Theirs,
    /// The label row leading a stacked section (`yours · main`, …).
    Label(LabelOf),
}

/// Which section a [`SideKind::Label`] row names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelOf {
    /// [`SideNames::ours`].
    Ours,
    /// `BASE · merged common ancestor`.
    Base,
    /// [`SideNames::theirs`].
    Theirs,
}

/// The resolver, ready to render.
pub struct ResolverScreen<'a> {
    /// What to draw.
    pub doc: &'a DocView,
    /// `doc` laid out at this screen's width.
    pub layout: &'a Layout,
    /// The first row index on screen.
    pub top: usize,
    /// Where the file list goes ([`placement`]); the document gets the
    /// rest ([`resolver_areas`]).
    pub placement: Placement,
    /// The theme to draw in.
    pub theme: &'a Theme,
    /// Draw box-drawing glyphs with their ASCII fallback.
    pub ascii: bool,
    /// His keymap: the too-small line's key is spelled from it.
    pub keys: &'a Keymap,
}

impl Widget for ResolverScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        if area.width < MIN_COLUMNS || area.height < MIN_ROWS {
            // `resolver needs 60x10 · esc closes`; the ASCII tier's
            // separator is ` - `. No key when `app.back` is unbound.
            let close = self.keys.key_verb(&[ActionId("app.back")], "closes");
            let too_small = match close {
                Some(close) if self.ascii => format!("{TOO_SMALL} - {close}"),
                Some(close) => format!("{TOO_SMALL} \u{b7} {close}"),
                None => TOO_SMALL.to_owned(),
            };
            set_text(
                buf,
                area.x,
                area.y,
                area,
                &too_small,
                foreground(self.theme, Token::TextMuted),
            );
            return;
        }
        let areas = resolver_areas(area, self.placement);
        FileList {
            view: &self.doc.files,
            placement: self.placement,
            theme: self.theme,
            ascii: self.ascii,
        }
        .render(areas.list, buf);
        self.draw_separator(buf, area, areas);
        let area = areas.doc;
        if area.is_empty() {
            return;
        }
        self.draw_page_header(buf, area);
        if area.height <= HEADER_ROWS {
            return;
        }
        let body = Rect {
            y: area.y.saturating_add(HEADER_ROWS),
            height: area.height - HEADER_ROWS,
            ..area
        };
        for offset in 0..body.height {
            let Some(row) = row_at(self.layout, self.top, offset) else {
                break;
            };
            let y = body.y + offset;
            self.draw_row(buf, body, y, row);
        }
    }
}

impl ResolverScreen<'_> {
    /// The rule row under a strip, or the `│` column beside a column.
    fn draw_separator(&self, buf: &mut Buffer, area: Rect, areas: ResolverAreas) {
        let style = foreground(self.theme, Token::Border);
        match self.placement {
            Placement::Strip { .. } => {
                let y = areas.list.bottom();
                if y < areas.doc.y {
                    let rule = if self.ascii { "-" } else { "─" };
                    set_text(
                        buf,
                        area.x,
                        y,
                        area,
                        &rule.repeat(usize::from(area.width)),
                        style,
                    );
                }
            }
            Placement::Column { .. } => {
                let x = areas.list.right();
                if x < areas.doc.x {
                    let bar = if self.ascii { "|" } else { "│" };
                    for y in area.y..area.bottom() {
                        set_text(buf, x, y, area, bar, style);
                    }
                }
            }
        }
    }

    /// The title row, then the notice (if any) on the row below it —
    /// the two `HEADER_ROWS` the body draws under. Not to be confused with
    /// [`Self::draw_header`], a block's own card header.
    fn draw_page_header(&self, buf: &mut Buffer, area: Rect) {
        let mark = if self.ascii { ">" } else { "▸" };
        set_text(
            buf,
            area.x,
            area.y,
            area,
            &format!("{mark} {}", self.doc.title.as_str()),
            foreground(self.theme, Token::Text),
        );
        if area.height == 1 {
            return;
        }
        if let Some(finish) = &self.doc.finish {
            self.draw_finish_row(buf, area, finish);
        } else if let Some((text, is_error)) = &self.doc.notice {
            let token = if *is_error {
                Token::Warning
            } else {
                Token::TextMuted
            };
            set_text(
                buf,
                area.x,
                area.y + 1,
                area,
                text.as_str(),
                foreground(self.theme, token),
            );
        }
    }
}

#[cfg(test)]
mod tests;
