//! The conflict resolver as a pure widget (`docs/design-notes/m6-conflicts.md`
//! Decision 1, "New UI").
//!
//! One file, drawn as a document: plain lines as they are, each conflict
//! block between git's `<<<<<<<`/`>>>>>>>` drawn as a labelled card with
//! **OURS and THEIRS side by side** at [`SIDE_BY_SIDE_MIN`] columns or
//! **stacked** below it. BASE is a third stacked section inside the card,
//! shown on demand ([`DocView::base_shown`]) at every width. A taken block
//! keeps its header (`took ours`/`took theirs`/`took both`) and shows only
//! the taken side's lines, full width, until the resolver closes.
//!
//! [`layout`] is pure and total: it lays out `doc` into [`ScreenRow`]s for
//! any `columns >= 1`, and [`ResolverScreen`] — the [`Widget`] — draws only
//! `layout.rows[top..top + height]`, nothing outside its `Rect`. The app
//! scrolls and hit-tests against the same rows the widget draws
//! ([`row_at`]); there is no second copy of the layout.
//!
//! **Line numbers are per side** (ADR-0043 §5): each side is numbered
//! contiguously by its own projection (`graft_git::conflict::Projection`).
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
use ratatui::style::Modifier;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use graft_term::SanitizedText;

use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// Columns at or above which a block's OURS/THEIRS draw side by side.
/// Below it they stack. Decision 1's numbers: halves at 120 columns are 58
/// cells each, readable; three full-height columns at 120 would be 38 —
/// code is unreadable there, which is why the resolver has two shapes and
/// not three.
pub const SIDE_BY_SIDE_MIN: u16 = 100;

/// Screen rows the widget reserves above the document itself: the title row
/// and one more for the notice (blank while none is set).
pub const HEADER_ROWS: u16 = 2;

/// Cells the cursor-block gutter (`▏`/`|`) and its trailing space take, on
/// every row of every block — not only the cursor block's, so the document
/// column stays put whichever block has the cursor.
const GUTTER: u16 = 2;

/// The single line the widget draws when it has no room to draw anything
/// else. `layout` still returns rows at this size — the app decides no key
/// acts, this widget only decides what is legible.
const TOO_SMALL: &str = "resolver needs 60x10 · esc closes";
/// [`TOO_SMALL`]'s ASCII form.
const TOO_SMALL_ASCII: &str = "resolver needs 60x10 - esc closes";
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

/// The role words for one operation's two sides — a closed set, so the hint
/// bar can carry them as `&'static str` (ADR-0043 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SideRoles {
    /// A merge or pull, or no operation at all: `yours` / `theirs`.
    Merge,
    /// A rebase: `upstream` / `yours` (the commit being replayed is yours).
    Rebase,
    /// A cherry-pick: `yours` / `picked`.
    CherryPick,
    /// A revert: `yours` / `reverting`.
    Revert,
    /// A stash pop: `yours` / `stashed`.
    StashPop,
}

impl SideRoles {
    /// The ours side's role word.
    #[must_use]
    pub const fn ours(self) -> &'static str {
        match self {
            SideRoles::Rebase => "upstream",
            SideRoles::Merge | SideRoles::CherryPick | SideRoles::Revert | SideRoles::StashPop => {
                "yours"
            }
        }
    }

    /// The theirs side's role word.
    #[must_use]
    pub const fn theirs(self) -> &'static str {
        match self {
            SideRoles::Merge => "theirs",
            SideRoles::Rebase => "yours",
            SideRoles::CherryPick => "picked",
            SideRoles::Revert => "reverting",
            SideRoles::StashPop => "stashed",
        }
    }
}

/// The one name each side goes by everywhere the resolver names it: role
/// word, then what it is (`yours · main`, `picked · 3f2a1b7 Add OAuth`).
/// The app builds them; this widget draws them verbatim, cut with `…` from
/// the right so the role word survives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideNames {
    /// The ours side (`yours · main`; rebase `upstream · main`).
    pub ours: SanitizedText,
    /// The theirs side (`theirs · feature`; stash pop `stashed`).
    pub theirs: SanitizedText,
    /// The role words alone, for the header keys and the hint bar.
    pub roles: SideRoles,
}

/// Where an open block's lines start in each projection, 1-based
/// (`graft_git::conflict::ConflictFile::first_lines`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockNumbers {
    /// Its first yours line, counted in the ours projection.
    pub ours: usize,
    /// Its first theirs line, counted in the theirs projection.
    pub theirs: usize,
    /// Its first line in the result: where a taken block's lines start.
    pub result: usize,
}

/// Everything the resolver draws for one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocView {
    /// `{path} · file {i} of {n} · {open} of {total} conflicts open`
    /// (≥100 columns) or the short form below it — the app builds the
    /// wording, this widget only draws it.
    pub title: SanitizedText,
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
}

/// One region of the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionView {
    /// Lines outside any conflict block, drawn as-is.
    Plain {
        /// The first line's number in the result projection.
        first_line: usize,
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
    /// (matching `graft_git::conflict::BlockAt::first_line`'s own doc —
    /// the marker line no longer exists once resolved). Where `e` opens
    /// the editor; never drawn.
    pub first_line: usize,
    /// Where each side's lines are numbered from.
    pub numbers: BlockNumbers,
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

/// One row of the document, as [`layout`] plans it. Carries indices into
/// `doc`, not text: the widget (and any other reader, e.g. a hit test)
/// looks the text up from the same `doc` it was laid out from, so the two
/// can never disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenRow {
    /// A plain line: `{line_no} {text}`, the number right-aligned.
    Plain {
        /// The file's own line number.
        line_no: usize,
        /// The line itself.
        text: SanitizedText,
    },
    /// A block's header/take-keys rule.
    Header {
        /// The block's index.
        block: usize,
    },
    /// The wide-layout labels row (`yours · main │ theirs · feature`).
    Labels {
        /// The block's index.
        block: usize,
    },
    /// A wide-layout row of the two sides, side by side. `None` pads the
    /// shorter side.
    Pair {
        /// The block's index.
        block: usize,
        /// Index into `ours`, if this row has one.
        ours: Option<usize>,
        /// Index into `theirs`, if this row has one.
        theirs: Option<usize>,
    },
    /// A stacked-layout row (narrow, or BASE at any width): a label row
    /// (`i` unused, `0`) or the `i`-th line of `side`.
    Side {
        /// The block's index.
        block: usize,
        /// Which section.
        side: SideKind,
        /// The line's index within that section.
        i: usize,
    },
    /// A block's closing rule.
    End {
        /// The block's index.
        block: usize,
    },
    /// The whole-file card's header row.
    WholeFile {
        /// The shape, worded.
        how: SanitizedText,
        /// The keys that apply.
        keys: SanitizedText,
    },
    /// A whole-file card's note under its lines: not a line of the file.
    Note {
        /// The note.
        text: SanitizedText,
    },
}

/// `doc` laid out at `columns`: every row the widget can draw, and where
/// each block's header row is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Every row, in draw order.
    pub rows: Vec<ScreenRow>,
    /// `block_row[i]` is the row index of block `i`'s [`ScreenRow::Header`].
    pub block_row: Vec<usize>,
    /// The width this layout was built for.
    pub columns: u16,
}

/// `doc` laid out at `columns`. Pure and total: returns rows for any
/// `columns >= 1`, including below [`MIN_COLUMNS`] (the widget, not the
/// layout, is what refuses to draw there).
#[must_use]
pub fn layout(doc: &DocView, columns: u16) -> Layout {
    let mut rows = Vec::new();
    let mut block_row = Vec::new();
    for region in &doc.regions {
        match region {
            RegionView::Plain { first_line, lines } => {
                for (offset, text) in lines.iter().enumerate() {
                    rows.push(ScreenRow::Plain {
                        line_no: first_line + offset,
                        text: text.clone(),
                    });
                }
            }
            RegionView::WholeFile {
                how,
                keys,
                lines,
                note,
            } => {
                rows.push(ScreenRow::WholeFile {
                    how: how.clone(),
                    keys: keys.clone(),
                });
                for (offset, text) in lines.iter().enumerate() {
                    rows.push(ScreenRow::Plain {
                        line_no: offset + 1,
                        text: text.clone(),
                    });
                }
                if let Some(text) = note {
                    rows.push(ScreenRow::Note { text: text.clone() });
                }
            }
            RegionView::Block(block) => {
                block_row.push(rows.len());
                rows.push(ScreenRow::Header { block: block.index });
                layout_block(doc, block, columns, &mut rows);
                rows.push(ScreenRow::End { block: block.index });
            }
        }
    }
    Layout {
        rows,
        block_row,
        columns,
    }
}

/// The rows between a block's header and its closing rule.
fn layout_block(doc: &DocView, block: &BlockView, columns: u16, rows: &mut Vec<ScreenRow>) {
    match block.state {
        BlockState::Took(take) => layout_taken(block, take, rows),
        BlockState::Open if columns >= SIDE_BY_SIDE_MIN => {
            rows.push(ScreenRow::Labels { block: block.index });
            let paired = block.ours.len().max(block.theirs.len());
            for i in 0..paired {
                rows.push(ScreenRow::Pair {
                    block: block.index,
                    ours: (i < block.ours.len()).then_some(i),
                    theirs: (i < block.theirs.len()).then_some(i),
                });
            }
            if let Some((_, base_lines)) = base_if_shown(doc, block) {
                push_labeled_side(
                    rows,
                    block.index,
                    LabelOf::Base,
                    SideKind::Base,
                    base_lines.len(),
                );
            }
        }
        BlockState::Open => {
            push_labeled_side(
                rows,
                block.index,
                LabelOf::Ours,
                SideKind::Ours,
                block.ours.len(),
            );
            if let Some((_, base_lines)) = base_if_shown(doc, block) {
                push_labeled_side(
                    rows,
                    block.index,
                    LabelOf::Base,
                    SideKind::Base,
                    base_lines.len(),
                );
            }
            push_labeled_side(
                rows,
                block.index,
                LabelOf::Theirs,
                SideKind::Theirs,
                block.theirs.len(),
            );
        }
    }
}

/// A taken block's rows: the taken side's lines, full width, no labels (the
/// header already says which side).
fn layout_taken(block: &BlockView, take: TakeView, rows: &mut Vec<ScreenRow>) {
    match take {
        TakeView::Ours => push_side_lines(rows, block.index, SideKind::Ours, block.ours.len()),
        TakeView::Theirs => {
            push_side_lines(rows, block.index, SideKind::Theirs, block.theirs.len());
        }
        TakeView::Both => {
            push_side_lines(rows, block.index, SideKind::Ours, block.ours.len());
            push_side_lines(rows, block.index, SideKind::Theirs, block.theirs.len());
        }
    }
}

/// `block`'s base, only when `doc.base_shown` and it is attached.
fn base_if_shown<'a>(
    doc: &DocView,
    block: &'a BlockView,
) -> Option<&'a (SanitizedText, Vec<SanitizedText>)> {
    if doc.base_shown {
        block.base.as_ref()
    } else {
        None
    }
}

/// A stacked section: its label row, then `len` content rows.
fn push_labeled_side(
    rows: &mut Vec<ScreenRow>,
    block: usize,
    label: LabelOf,
    side: SideKind,
    len: usize,
) {
    rows.push(ScreenRow::Side {
        block,
        side: SideKind::Label(label),
        i: 0,
    });
    push_side_lines(rows, block, side, len);
}

/// `len` content rows of `side`, no label.
fn push_side_lines(rows: &mut Vec<ScreenRow>, block: usize, side: SideKind, len: usize) {
    for i in 0..len {
        rows.push(ScreenRow::Side { block, side, i });
    }
}

/// The row `top + screen_row` names, or `None` past the end. The app scrolls
/// and hit-tests through this — the same indexing [`ResolverScreen`] draws
/// with, so a click and the pixel it lands on never disagree.
#[must_use]
pub fn row_at(layout: &Layout, top: usize, screen_row: u16) -> Option<&ScreenRow> {
    layout.rows.get(top + usize::from(screen_row))
}

/// The resolver, ready to render.
pub struct ResolverScreen<'a> {
    /// What to draw.
    pub doc: &'a DocView,
    /// `doc` laid out at this screen's width.
    pub layout: &'a Layout,
    /// The first row index on screen.
    pub top: usize,
    /// The theme to draw in.
    pub theme: &'a Theme,
    /// Draw box-drawing glyphs with their ASCII fallback.
    pub ascii: bool,
}

impl ResolverScreen<'_> {
    fn rule(&self) -> char {
        if self.ascii { '-' } else { '─' }
    }

    fn divider(&self) -> char {
        if self.ascii { '|' } else { '│' }
    }

    fn join(&self) -> char {
        if self.ascii { '+' } else { '┴' }
    }

    fn cursor_mark(&self) -> &'static str {
        if self.ascii { "|" } else { "\u{258f}" }
    }

    /// The separator the widget's own labels use (`OURS · HEAD`,
    /// ` conflict 1 of 3 · open`, …) — ASCII `-` at the ASCII tier, mirroring
    /// every box-drawing glyph here. App-supplied text (`doc.title`,
    /// `ours_label`, …) is drawn verbatim either way: sanitizing *that* is
    /// `term::sanitize`'s job, not this widget's.
    fn dot(&self) -> &'static str {
        if self.ascii { "-" } else { "\u{b7}" }
    }

    /// `n` right-aligned in the number column, then a space; a blank column
    /// when `None`.
    fn numbered(&self, n: Option<usize>, text: &str) -> String {
        let width = number_width(self.doc);
        match n {
            Some(n) => format!("{n:>width$} {text}"),
            None => format!("{:width$} {text}", ""),
        }
    }

    fn is_cursor_block(&self, block: usize) -> bool {
        self.doc.cursor_block == Some(block)
    }

    /// `None` when `doc` carries no card for `index` — a stale `Layout` built
    /// from a previous `DocView` (a re-read after `e`/`M` can shrink
    /// `blocks()`, `retake_replaces` shows 3 → 2). Draw call sites must stay
    /// total: they draw a blank row rather than unwrap this.
    fn block(&self, index: usize) -> Option<&BlockView> {
        self.doc.regions.iter().find_map(|region| match region {
            RegionView::Block(block) if block.index == index => Some(block),
            _ => None,
        })
    }

    /// A blank row, `width` cells, for a row whose block or line no longer
    /// exists in `doc` — the render stays total instead of panicking or
    /// reading stale data (CLAUDE.md rule 3).
    fn draw_blank(&self, buf: &mut Buffer, area: Rect, y: u16) {
        let content = Self::content_area(area);
        set_text(
            buf,
            content.x,
            y,
            content,
            &" ".repeat(usize::from(content.width)),
            foreground(self.theme, Token::TextMuted),
        );
    }

    /// `area` with the gutter's cells removed from the left, whether or not
    /// this row's block has the cursor — the document column stays put
    /// whichever block does.
    fn content_area(area: Rect) -> Rect {
        Rect {
            x: area.x.saturating_add(GUTTER),
            width: area.width.saturating_sub(GUTTER),
            ..area
        }
    }

    fn draw_gutter(&self, buf: &mut Buffer, area: Rect, y: u16, block: usize) {
        if self.is_cursor_block(block) {
            set_text(
                buf,
                area.x,
                y,
                area,
                self.cursor_mark(),
                foreground(self.theme, Token::Accent),
            );
        }
    }

    /// The plain content style: never reversed. Only [`Self::draw_header`]
    /// reverses (Major 3, m6-02-03 review) — every other row of the cursor
    /// block carries the `▏` gutter instead ([`Self::draw_gutter`]), which
    /// is the design note's own split ("the cursor block's header (reversed,
    /// `▏` rule on its lines)"). A whole reversed card is a worse version of
    /// the m5/44 problem this design already fixed once.
    fn base_style(&self) -> ratatui::style::Style {
        foreground(self.theme, Token::Text)
    }

    /// [`Self::base_style`], reversed when `cursor` — the header's own
    /// style, and the only place reversal happens.
    fn header_style(&self, cursor: bool) -> ratatui::style::Style {
        if cursor {
            self.base_style().add_modifier(Modifier::REVERSED)
        } else {
            self.base_style()
        }
    }

    fn draw_row(&self, buf: &mut Buffer, area: Rect, y: u16, row: &ScreenRow) {
        match row {
            ScreenRow::Plain { line_no, text } => {
                // Drawn through `content_area` like every block row, so the
                // number column sits in the same cells whether the row is
                // inside a card or not (Major 2, m6-02-03 review) — a plain
                // line never carries the cursor gutter, but it still leaves
                // room for one so the numbers line up.
                let content = Self::content_area(area);
                let line = self.numbered(Some(*line_no), text.as_str());
                set_text(
                    buf,
                    content.x,
                    y,
                    content,
                    &fit(&line, usize::from(content.width)),
                    foreground(self.theme, Token::Text),
                );
            }
            ScreenRow::Note { text } => {
                // In the text column, under the lines above it, with the
                // number column left blank: it is about the file, not in it.
                let content = Self::content_area(area);
                let line = self.numbered(None, text.as_str());
                set_text(
                    buf,
                    content.x,
                    y,
                    content,
                    &fit(&line, usize::from(content.width)),
                    foreground(self.theme, Token::TextMuted),
                );
            }
            ScreenRow::WholeFile { how, keys } => {
                let line = format!("{how} {} {keys}", self.dot());
                set_text(
                    buf,
                    area.x,
                    y,
                    area,
                    &fit(&line, usize::from(area.width)),
                    foreground(self.theme, Token::Warning),
                );
            }
            ScreenRow::Header { block } => self.draw_header(buf, area, y, *block),
            ScreenRow::Labels { block } => self.draw_labels(buf, area, y, *block),
            ScreenRow::Pair {
                block,
                ours,
                theirs,
            } => self.draw_pair(buf, area, y, *block, *ours, *theirs),
            ScreenRow::Side { block, side, i } => self.draw_side(buf, area, y, *block, *side, *i),
            ScreenRow::End { block } => self.draw_end(buf, area, y, *block),
        }
    }

    fn draw_header(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        let Some(block) = self.block(block_index) else {
            self.draw_blank(buf, area, y);
            return;
        };
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let names = &self.doc.names;
        let (ours, theirs) = (names.roles.ours(), names.roles.theirs());
        let (state_word, keys) = match block.state {
            BlockState::Open => ("open".to_owned(), format!("o {ours}  t {theirs}  b both")),
            BlockState::Took(TakeView::Ours) => (
                format!("took {}", names.ours),
                format!("t {theirs}  b both"),
            ),
            BlockState::Took(TakeView::Theirs) => (
                format!("took {}", names.theirs),
                format!("o {ours}  b both"),
            ),
            BlockState::Took(TakeView::Both) => {
                ("took both".to_owned(), format!("o {ours}  t {theirs}"))
            }
        };
        let rule2 = self.rule().to_string().repeat(2);
        let middle = format!(
            " conflict {} of {} {} {state_word} {rule2} {keys} ",
            block.index + 1,
            block.of,
            self.dot(),
        );
        // No number: a marker row is not a line of any side (ADR-0043 §5).
        let line = ruled_line(&middle, self.rule(), usize::from(content.width));
        let cursor = self.is_cursor_block(block_index);
        set_text(buf, content.x, y, content, &line, self.header_style(cursor));
    }

    fn draw_labels(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        if self.block(block_index).is_none() {
            self.draw_blank(buf, area, y);
            return;
        }
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let half = width.saturating_sub(1) / 2;
        let left = fit(self.doc.names.ours.as_str(), half);
        let right = fit(
            self.doc.names.theirs.as_str(),
            width.saturating_sub(half + 1),
        );
        let line = format!("{left}{}{right}", self.divider());
        set_text(buf, content.x, y, content, &line, self.side_style(false));
    }

    fn draw_pair(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        block_index: usize,
        ours: Option<usize>,
        theirs: Option<usize>,
    ) {
        let Some(block) = self.block(block_index) else {
            self.draw_blank(buf, area, y);
            return;
        };
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let half = width.saturating_sub(1) / 2;
        // `Pair` rows only exist for an `Open` block (`layout_block`): each
        // side counts in its own projection.
        let left_text = ours
            .and_then(|i| block.ours.get(i))
            .map_or("", |t| t.as_str());
        let left = fit(
            &self.numbered(ours.map(|i| block.numbers.ours + i), left_text),
            half,
        );
        let right_text = theirs
            .and_then(|i| block.theirs.get(i))
            .map_or("", |t| t.as_str());
        let right = fit(
            &self.numbered(theirs.map(|i| block.numbers.theirs + i), right_text),
            width.saturating_sub(half + 1),
        );
        let line = format!("{left}{}{right}", self.divider());
        set_text(buf, content.x, y, content, &line, self.base_style());
    }

    fn draw_side(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        block_index: usize,
        side: SideKind,
        i: usize,
    ) {
        let Some(block) = self.block(block_index) else {
            self.draw_blank(buf, area, y);
            return;
        };
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let (line, muted) = match side {
            SideKind::Label(LabelOf::Ours) => (self.doc.names.ours.as_str().to_owned(), true),
            SideKind::Label(LabelOf::Theirs) => (self.doc.names.theirs.as_str().to_owned(), true),
            SideKind::Label(LabelOf::Base) => {
                (format!("BASE {} merged common ancestor", self.dot()), true)
            }
            SideKind::Ours => {
                // Open: the ours projection. Taken: the result, where the
                // kept lines start at `numbers.result`.
                let n = match block.state {
                    BlockState::Open => block.numbers.ours + i,
                    BlockState::Took(_) => block.numbers.result + i,
                };
                let text = block.ours.get(i).map_or("", |t| t.as_str());
                (self.numbered(Some(n), text), false)
            }
            SideKind::Theirs => {
                // Open: the theirs projection. Taken: the result, after the
                // ours lines a `Both` take kept first.
                let n = match block.state {
                    BlockState::Open => block.numbers.theirs + i,
                    BlockState::Took(TakeView::Both) => block.numbers.result + block.ours.len() + i,
                    BlockState::Took(_) => block.numbers.result + i,
                };
                let text = block.theirs.get(i).map_or("", |t| t.as_str());
                (self.numbered(Some(n), text), false)
            }
            SideKind::Base => {
                let base_lines = block
                    .base
                    .as_ref()
                    .map_or(&[][..], |(_, lines)| lines.as_slice());
                (
                    self.numbered(None, base_lines.get(i).map_or("", SanitizedText::as_str)),
                    false,
                )
            }
        };
        set_text(
            buf,
            content.x,
            y,
            content,
            &fit(&line, width),
            self.side_style(muted),
        );
    }

    fn draw_end(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let rule = self.rule();
        let line = if self.layout.columns >= SIDE_BY_SIDE_MIN {
            let half = width.saturating_sub(1) / 2;
            let mut s = rule.to_string().repeat(half);
            s.push(self.join());
            s.push_str(&rule.to_string().repeat(width.saturating_sub(half + 1)));
            s
        } else {
            rule.to_string().repeat(width)
        };
        set_text(
            buf,
            content.x,
            y,
            content,
            &fit(&line, width),
            self.base_style(),
        );
    }

    fn side_style(&self, muted: bool) -> ratatui::style::Style {
        if muted {
            foreground(self.theme, Token::TextMuted)
        } else {
            foreground(self.theme, Token::Text)
        }
    }
}

/// Digits in the widest line number any projection reaches in `doc` — the
/// number column's width (at least 1).
#[must_use]
pub fn number_width(doc: &DocView) -> usize {
    let last = |first: usize, len: usize| (first + len).saturating_sub(1);
    let widest = doc
        .regions
        .iter()
        .map(|region| match region {
            RegionView::Plain { first_line, lines } => last(*first_line, lines.len()),
            RegionView::WholeFile { lines, .. } => lines.len(),
            RegionView::Block(block) => last(block.numbers.ours, block.ours.len())
                .max(last(block.numbers.theirs, block.theirs.len()))
                .max(last(
                    block.numbers.result,
                    match block.state {
                        BlockState::Open | BlockState::Took(TakeView::Ours) => block.ours.len(),
                        BlockState::Took(TakeView::Theirs) => block.theirs.len(),
                        BlockState::Took(TakeView::Both) => block.ours.len() + block.theirs.len(),
                    },
                )),
        })
        .max()
        .unwrap_or(0);
    widest.max(1).to_string().len()
}

/// `middle` centred in a rule line `width` cells wide, the rule character
/// filling what is left; cut with `…` if `middle` alone overflows `width`.
fn ruled_line(middle: &str, rule: char, width: usize) -> String {
    let mut s = format!("{rule}{rule}{middle}");
    let used = s.width();
    if used < width {
        s.push_str(&rule.to_string().repeat(width - used));
    }
    fit(&s, width)
}

/// Writes `text`, clipped to what is left of `area` from `(x, y)`; a no-op
/// past the area's edge (mirrors `map_screen::render::set_text`).
fn set_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    area: Rect,
    text: &str,
    style: ratatui::style::Style,
) {
    if x >= area.right() || y >= area.bottom() {
        return;
    }
    let width = area.right() - x;
    buf.set_stringn(x, y, text, usize::from(width), style);
}

impl Widget for ResolverScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        if area.width < MIN_COLUMNS || area.height < MIN_ROWS {
            let too_small = if self.ascii {
                TOO_SMALL_ASCII
            } else {
                TOO_SMALL
            };
            set_text(
                buf,
                area.x,
                area.y,
                area,
                too_small,
                foreground(self.theme, Token::TextMuted),
            );
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
    /// The title row, then the notice (if any) on the row below it —
    /// the two `HEADER_ROWS` the body draws under. Not to be confused with
    /// [`Self::draw_header`], a block's own card header.
    fn draw_page_header(&self, buf: &mut Buffer, area: Rect) {
        set_text(
            buf,
            area.x,
            area.y,
            area,
            self.doc.title.as_str(),
            foreground(self.theme, Token::Text),
        );
        if area.height == 1 {
            return;
        }
        if let Some((text, is_error)) = &self.doc.notice {
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
