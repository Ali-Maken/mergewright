//! The detail pane's regions, breakpoints and states.
//!
//! Draws a [`DetailView`] (`crate::detail`, view-model only) beside, below or
//! in place of the graph, per DESIGN.md §1.1-1.3's breakpoints. [`diff_body`]
//! (`diff-widget`, m2-details-and-diffs/08) is the real unified diff
//! renderer: gutter, soft/word backgrounds, syntax fg, a pinned hunk heading,
//! `␍`, and a truncation footer.
//!
//! Deviations from the ticket's literal signatures, both reported as spec
//! gaps rather than silently worked around:
//! - [`DetailPane`] also carries `theme` (07's gap): every other widget in
//!   this crate is handed a `&Theme` the same way, and there is no other
//!   colour source at render time.
//! - [`diff_body`] takes a `truncated: Option<(usize, usize)>` the ticket's
//!   signature didn't list: the truncation footer it must draw is
//!   [`crate::detail::DiffView`]'s field, not the hunks', and there is no
//!   other way to hand it over under the same argument list.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use graft_core::capabilities::ColorDepth;
use graft_core::mistake_radar::Severity;
use graft_term::sanitize::SanitizedText;

use crate::detail::{
    BodyView, ContainedIn, DetailFocus, DetailHeader, DetailView, DiffMode, ExplainView, FileRow,
    FileRowKind, FileStatusView, HunkView, LineKindView, LineView, RadarLine, Subject,
};
use crate::fit::fit;
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

/// Where the detail pane sits, per DESIGN.md §1.1-1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailLayout {
    /// 160+ columns: a right column.
    Right {
        /// Column width, 42% of the screen width, min 48.
        columns: u16,
    },
    /// 100..160 columns: a strip below the graph.
    Below {
        /// Strip height, 40% of the screen height, min 8.
        rows: u16,
    },
    /// Under 100 columns: a `[Graph] Detail` tab strip.
    Tab {
        /// Follows `focus != DetailFocus::Graph` (no separate "which tab"
        /// state).
        shown: bool,
    },
}

impl DetailLayout {
    const RIGHT_BREAKPOINT: u16 = 160;
    const BELOW_BREAKPOINT: u16 = 100;
    const RIGHT_MIN: u16 = 48;
    const BELOW_MIN: u16 = 8;

    /// The layout for a screen of `width` by `height` cells. `override_cells`
    /// (a divider drag or `<`/`>`, `detail-keys-and-focus`) replaces the
    /// percentage figure, still clamped to the breakpoint's minimum.
    #[must_use]
    pub fn compute(
        width: u16,
        height: u16,
        focus: DetailFocus,
        override_cells: Option<u16>,
    ) -> DetailLayout {
        if width >= Self::RIGHT_BREAKPOINT {
            let pct = percent(width, 42);
            DetailLayout::Right {
                columns: override_cells.unwrap_or(pct).max(Self::RIGHT_MIN),
            }
        } else if width >= Self::BELOW_BREAKPOINT {
            let pct = percent(height, 40);
            DetailLayout::Below {
                rows: override_cells.unwrap_or(pct).max(Self::BELOW_MIN),
            }
        } else {
            DetailLayout::Tab {
                shown: focus != DetailFocus::Graph,
            }
        }
    }
}

/// `cells * pct / 100`, saturating rather than overflowing for any `u16`.
fn percent(cells: u16, pct: u32) -> u16 {
    u16::try_from(u32::from(cells) * pct / 100).unwrap_or(u16::MAX)
}

/// `area` split into the graph rectangle and the pane rectangle for
/// `layout`, either of which is `None` in `Tab` mode (only the shown side
/// gets space). Pure geometry — no drawing — so both the frame that draws
/// from it (`view/frame.rs::split_for_detail`, which also renders the `Tab`
/// strip this doesn't know about) and mouse hit-testing (`update/mod.rs`,
/// divider drag and wheel routing, `detail-resize-and-persistence`) split
/// the pane the same way.
#[must_use]
pub fn split_area(layout: DetailLayout, area: Rect) -> (Option<Rect>, Option<Rect>) {
    match layout {
        DetailLayout::Right { columns } => {
            let columns = columns.min(area.width);
            let graph_area = Rect {
                width: area.width - columns,
                ..area
            };
            let pane_area = Rect {
                x: area.x + graph_area.width,
                width: columns,
                ..area
            };
            (Some(graph_area), Some(pane_area))
        }
        DetailLayout::Below { rows } => {
            let rows = rows.min(area.height);
            let graph_area = Rect {
                height: area.height - rows,
                ..area
            };
            let pane_area = Rect {
                y: area.y + graph_area.height,
                height: rows,
                ..area
            };
            (Some(graph_area), Some(pane_area))
        }
        DetailLayout::Tab { shown } => {
            let strip = 1.min(area.height);
            let content = Rect {
                y: area.y + strip,
                height: area.height - strip,
                ..area
            };
            if shown {
                (None, Some(content))
            } else {
                (Some(content), None)
            }
        }
    }
}

/// The pane's three regions: header (fixed rows), files (a list) and body
/// (message or diff).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailAreas {
    /// Fixed-height rows at the top: subject, author, parents, contained,
    /// radar (as many as the breakpoint keeps).
    pub header: Rect,
    /// The file list.
    pub files: Rect,
    /// The message or diff.
    pub body: Rect,
}

/// Split `pane_area` into [`DetailAreas`] for `layout`. `files_count` caps
/// how much of the files region the file list actually needs (`Right`/`Tab`
/// only): it never grows past that even when there is room, and never goes
/// negative when there are no files.
#[must_use]
pub fn detail_areas(pane_area: Rect, layout: DetailLayout, files_count: usize) -> DetailAreas {
    let header_rows = match layout {
        DetailLayout::Right { .. } => 5,
        DetailLayout::Below { .. } => 2,
        DetailLayout::Tab { .. } => 3,
    };
    let header = Rect {
        height: header_rows.min(pane_area.height),
        ..pane_area
    };
    let rest = Rect {
        y: pane_area.y.saturating_add(header.height),
        height: pane_area.height.saturating_sub(header.height),
        ..pane_area
    };

    if matches!(layout, DetailLayout::Below { .. }) {
        let files_width = 30.min(rest.width);
        let files = Rect {
            width: files_width,
            ..rest
        };
        let body = Rect {
            x: rest.x.saturating_add(files_width),
            width: rest.width.saturating_sub(files_width),
            ..rest
        };
        DetailAreas {
            header,
            files,
            body,
        }
    } else {
        let cap = percent(rest.height, 40);
        let files_height = cap
            .min(u16::try_from(files_count).unwrap_or(u16::MAX))
            .min(rest.height);
        let files = Rect {
            height: files_height,
            ..rest
        };
        let body = Rect {
            y: rest.y.saturating_add(files_height),
            height: rest.height.saturating_sub(files_height),
            ..rest
        };
        DetailAreas {
            header,
            files,
            body,
        }
    }
}

/// A row `diff_body` draws: a hunk's own heading, or one of its lines.
enum DiffRow<'a> {
    Heading(&'a SanitizedText),
    Line(&'a LineView),
}

/// One row per hunk heading, then its content lines. A `HunkHeading`-kind
/// line inside `hunk.lines` (real assembled diffs put one at `lines[0]`,
/// `crates/graft/src/jobs/detail.rs`) is dropped here rather than drawn a
/// second time: `hunk.heading` already stands for it, and [`pin_heading`]
/// and [`gutter_width`] both key off this same filtered shape.
fn flatten(hunks: &[HunkView]) -> Vec<DiffRow<'_>> {
    let mut rows = Vec::new();
    for hunk in hunks {
        rows.push(DiffRow::Heading(&hunk.heading));
        rows.extend(
            hunk.lines
                .iter()
                .filter(|line| line.kind != LineKindView::HunkHeading)
                .map(DiffRow::Line),
        );
    }
    rows
}

/// Gutter width: the widest old/new line number across every hunk, min 3.
fn gutter_width(hunks: &[HunkView]) -> usize {
    let widest = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKindView::HunkHeading)
        .flat_map(|line| [line.old_no, line.new_no])
        .flatten()
        .max()
        .unwrap_or(0);
    widest.to_string().len().max(3)
}

/// The hunk heading to pin at the body's top row, when `top` lands inside
/// that hunk's own lines rather than on its heading row (which is already
/// visible at row 0 in that case — normal scroll resumes).
fn pin_heading<'a>(
    hunks: &'a [HunkView],
    rows: &[DiffRow<'a>],
    top: usize,
) -> Option<&'a SanitizedText> {
    if matches!(rows.get(top), None | Some(DiffRow::Heading(_))) {
        return None;
    }
    let mut consumed = 0usize;
    for hunk in hunks {
        let hunk_rows = 1 + hunk
            .lines
            .iter()
            .filter(|line| line.kind != LineKindView::HunkHeading)
            .count();
        if top < consumed + hunk_rows {
            return Some(&hunk.heading);
        }
        consumed += hunk_rows;
    }
    None
}

/// Draw `hunks` from content row `top`: gutter numbers, sign cell, soft
/// added/removed background, a stronger background over changed words,
/// syntax foreground, a pinned hunk heading, and — when `truncated` is
/// `Some((shown, total))` — a `… N more lines · L load all` footer row.
/// Returns the total content rows including that footer, for scroll
/// clamping. At [`ColorDepth::Palette16`] or under `NO_COLOR`
/// ([`ColorDepth::None`]): sign glyph plus bold (added) / dim (removed)
/// only, no backgrounds, no syntax or word tint (m1/12c rule).
#[must_use]
pub fn diff_body(
    hunks: &[HunkView],
    top: usize,
    truncated: Option<(usize, usize)>,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    focus: DiffFocus,
) -> usize {
    let rows = flatten(hunks);
    let total = rows.len() + usize::from(truncated.is_some());
    if area.width == 0 || area.height == 0 {
        return total;
    }

    let gutter = gutter_width(hunks);
    let palette16 = matches!(
        theme.color_depth(),
        ColorDepth::Palette16 | ColorDepth::None
    );
    let style_ctx = DiffRowStyle {
        focused: focus.focused,
        palette16,
        word_tint: focus.word_tint,
    };

    let pinned = pin_heading(hunks, &rows, top);
    let mut used: u16 = 0;
    if let Some(heading) = pinned {
        let slot = RowSlot {
            x: area.x,
            y: area.y,
            width: area.width,
        };
        draw_heading(heading, slot, buf, theme);
        used = 1;
    }
    let content_top = top.saturating_add(usize::from(pinned.is_some()));
    let remaining = area.height.saturating_sub(used);

    for offset in 0..remaining {
        let idx = content_top.saturating_add(usize::from(offset));
        let slot = RowSlot {
            x: area.x,
            y: area.y.saturating_add(used).saturating_add(offset),
            width: area.width,
        };
        if let Some(row) = rows.get(idx) {
            draw_diff_row(row, gutter, slot, buf, theme, style_ctx);
        } else if let (true, Some((shown, of_total))) = (idx == rows.len(), truncated) {
            draw_truncation_row(shown, of_total, slot, buf, theme);
        }
    }

    total
}

/// One row of the side-by-side body: a full-width hunk heading, or a pair of
/// half-width lines. A `None` half is a run's unmatched line — blank on that
/// side, not shifted.
enum SideRow<'a> {
    Heading(&'a SanitizedText),
    Pair {
        left: Option<&'a LineView>,
        right: Option<&'a LineView>,
    },
}

/// `hunk`'s lines as side-by-side rows: context lines mirror both halves;
/// a removed/added line with a [`LineView::pair`] shares a row with its
/// partner (whichever side set the index, the other is marked consumed so it
/// is not emitted twice); an unpaired line leaves the other half blank.
fn side_rows_for_hunk(hunk: &HunkView) -> Vec<SideRow<'_>> {
    let mut rows = Vec::new();
    let mut consumed = vec![false; hunk.lines.len()];
    for (index, line) in hunk.lines.iter().enumerate() {
        if consumed[index] {
            continue;
        }
        match line.kind {
            LineKindView::HunkHeading => {}
            LineKindView::Context => rows.push(SideRow::Pair {
                left: Some(line),
                right: Some(line),
            }),
            LineKindView::Removed => {
                if let Some(pair) = line.pair {
                    let partner = usize::try_from(pair).ok();
                    if let Some(partner) = partner {
                        consumed[partner] = true;
                    }
                    rows.push(SideRow::Pair {
                        left: Some(line),
                        right: partner.and_then(|p| hunk.lines.get(p)),
                    });
                } else {
                    rows.push(SideRow::Pair {
                        left: Some(line),
                        right: None,
                    });
                }
            }
            LineKindView::Added => {
                if let Some(pair) = line.pair {
                    let partner = usize::try_from(pair).ok();
                    if let Some(partner) = partner {
                        consumed[partner] = true;
                    }
                    rows.push(SideRow::Pair {
                        left: partner.and_then(|p| hunk.lines.get(p)),
                        right: Some(line),
                    });
                } else {
                    rows.push(SideRow::Pair {
                        left: None,
                        right: Some(line),
                    });
                }
            }
        }
    }
    rows
}

fn flatten_side_by_side(hunks: &[HunkView]) -> Vec<SideRow<'_>> {
    let mut rows = Vec::new();
    for hunk in hunks {
        rows.push(SideRow::Heading(&hunk.heading));
        rows.extend(side_rows_for_hunk(hunk));
    }
    rows
}

/// The widest old (`old == true`) or new line number across every hunk, min 3.
fn gutter_width_side(hunks: &[HunkView], old: bool) -> usize {
    let widest = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKindView::HunkHeading)
        .filter_map(|line| if old { line.old_no } else { line.new_no })
        .max()
        .unwrap_or(0);
    widest.to_string().len().max(3)
}

/// [`pin_heading`]'s side-by-side twin: same rule, counted over
/// [`side_rows_for_hunk`] rows instead of the unified flatten.
fn pin_heading_side<'a>(
    hunks: &'a [HunkView],
    rows: &[SideRow<'a>],
    top: usize,
) -> Option<&'a SanitizedText> {
    if matches!(rows.get(top), None | Some(SideRow::Heading(_))) {
        return None;
    }
    let mut consumed = 0usize;
    for hunk in hunks {
        let hunk_rows = 1 + side_rows_for_hunk(hunk).len();
        if top < consumed + hunk_rows {
            return Some(&hunk.heading);
        }
        consumed += hunk_rows;
    }
    None
}

/// One half of a side-by-side row: `{number:>gutter} {sign}{text}`, or a
/// blank half when `line` is `None` (the other side's run ran longer).
fn draw_side_line(
    line: Option<&LineView>,
    old: bool,
    gutter: usize,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    let RowSlot { x, y, width } = slot;
    let Some(line) = line else {
        let blank = " ".repeat(usize::from(width));
        buf.set_stringn(x, y, &blank, usize::from(width), Style::default());
        return;
    };
    let (sign, kind_token) = match line.kind {
        LineKindView::Added => ('+', Some(Token::Added)),
        LineKindView::Removed => ('-', Some(Token::Removed)),
        LineKindView::Context | LineKindView::HunkHeading => (' ', None),
    };
    let number = if old { line.old_no } else { line.new_no };
    let num_str = number.map_or(String::new(), |n| n.to_string());
    let prefix = format!("{num_str:>gutter$} ");
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
    let sign_style = line_style_at(line, usize::MAX, theme, style_ctx, kind_token);
    let sign_x = x + prefix_fits;
    buf.set_stringn(sign_x, y, sign.to_string(), 1, sign_style);
    if prefix_fits.saturating_add(1) >= width {
        return;
    }
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

/// The `v`-toggled two-column diff over the same [`HunkView`] data
/// [`diff_body`] draws: old numbers and text on the left, new on the right,
/// a `│` rule between (plain at [`ColorDepth::Palette16`]/`NO_COLOR`, same
/// rule as the sign styling there). Rows come from [`side_rows_for_hunk`]:
/// [`LineView::pair`] keeps a removed/added run aligned; an unpaired line
/// leaves the other half blank on that row, never shifted. The hunk heading
/// pins full-width above both halves exactly as it does in [`diff_body`].
/// Word tint, syntax fg, `␍` and the 16-colour fallback are the same
/// [`line_style_at`]/[`draw_styled_text`] [`diff_body`] uses, so both
/// renderers match on the same fixture by construction.
#[must_use]
pub fn diff_body_side_by_side(
    hunks: &[HunkView],
    top: usize,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    focused: bool,
    word_tint: bool,
) -> usize {
    let rows = flatten_side_by_side(hunks);
    let total = rows.len();
    if area.width == 0 || area.height == 0 {
        return total;
    }

    let columns = SideColumns::new(hunks, area, theme, focused, word_tint);
    let pinned = pin_heading_side(hunks, &rows, top);
    let mut used: u16 = 0;
    if let Some(heading) = pinned {
        let slot = RowSlot {
            x: area.x,
            y: area.y,
            width: area.width,
        };
        draw_heading(heading, slot, buf, theme);
        used = 1;
    }
    let content_top = top.saturating_add(usize::from(pinned.is_some()));
    let remaining = area.height.saturating_sub(used);

    for offset in 0..remaining {
        let idx = content_top.saturating_add(usize::from(offset));
        let y = area.y.saturating_add(used).saturating_add(offset);
        draw_side_row(rows.get(idx), area, y, &columns, buf, theme);
    }

    total
}

/// Everything [`diff_body_side_by_side`]'s row loop needs beyond the row
/// itself: both gutters, the halves' x-ranges, the divider and the
/// [`DiffRowStyle`] both halves draw with.
struct SideColumns {
    old_gutter: usize,
    new_gutter: usize,
    style_ctx: DiffRowStyle,
    left_width: u16,
    right_width: u16,
    divider_x: u16,
    right_x: u16,
    divider: u16,
    divider_glyph: &'static str,
}

impl SideColumns {
    fn new(hunks: &[HunkView], area: Rect, theme: &Theme, focused: bool, word_tint: bool) -> Self {
        let palette16 = matches!(
            theme.color_depth(),
            ColorDepth::Palette16 | ColorDepth::None
        );
        let divider = u16::from(area.width >= 3);
        let usable = area.width.saturating_sub(divider);
        let left_width = usable / 2;
        let right_width = usable.saturating_sub(left_width);
        let divider_x = area.x + left_width;
        Self {
            old_gutter: gutter_width_side(hunks, true),
            new_gutter: gutter_width_side(hunks, false),
            style_ctx: DiffRowStyle {
                focused,
                palette16,
                word_tint,
            },
            left_width,
            right_width,
            divider_x,
            right_x: divider_x + divider,
            divider,
            divider_glyph: if palette16 { "|" } else { "\u{2502}" },
        }
    }
}

/// One content row: a full-width heading, a paired-line row split across
/// [`SideColumns`]'s two halves with the divider between, or nothing past
/// the last row.
fn draw_side_row(
    row: Option<&SideRow<'_>>,
    area: Rect,
    y: u16,
    columns: &SideColumns,
    buf: &mut Buffer,
    theme: &Theme,
) {
    match row {
        Some(SideRow::Heading(heading)) => {
            let slot = RowSlot {
                x: area.x,
                y,
                width: area.width,
            };
            draw_heading(heading, slot, buf, theme);
        }
        Some(SideRow::Pair { left, right }) => {
            draw_side_line(
                *left,
                true,
                columns.old_gutter,
                RowSlot {
                    x: area.x,
                    y,
                    width: columns.left_width,
                },
                buf,
                theme,
                columns.style_ctx,
            );
            if columns.divider > 0 {
                buf.set_stringn(
                    columns.divider_x,
                    y,
                    columns.divider_glyph,
                    1,
                    foreground(theme, Token::TextMuted),
                );
            }
            draw_side_line(
                *right,
                false,
                columns.new_gutter,
                RowSlot {
                    x: columns.right_x,
                    y,
                    width: columns.right_width,
                },
                buf,
                theme,
                columns.style_ctx,
            );
        }
        None => {}
    }
}

/// [`diff_body`]/[`diff_body_side_by_side`]'s two independent flags,
/// bundled into one parameter to keep both functions under clippy's
/// argument-count limit.
#[derive(Clone, Copy, Debug)]
pub struct DiffFocus {
    /// The diff region has the keys (`Tab`); see [`DiffRowStyle::focused`]
    /// for what this does and does not change.
    pub focused: bool,
    /// Changed words get the stronger tint background (`W`).
    pub word_tint: bool,
}

/// The flags [`draw_diff_row`] needs beyond the row itself and the theme,
/// bundled to keep the function under clippy's argument-count limit.
#[derive(Clone, Copy)]
struct DiffRowStyle {
    /// The diff region has the keys. Marks focus on the gutter only
    /// (`gutter_style`); line/word/syntax colour never depends on it
    /// (m2-details-and-diffs review, M2: a body dimmed to `text-muted`
    /// until `Tab` reached it was unreadable by default).
    focused: bool,
    /// The 16-colour rule (or `NO_COLOR`) is in force: bold/dim, no
    /// background, no syntax or word tint.
    palette16: bool,
    /// Changed words get the stronger tint background (`W`, review M1: the
    /// flag used to reach nothing past the view model).
    word_tint: bool,
}

/// The gutter's line-number colour: accent when this region has the keys,
/// muted otherwise. The only focus cue left on the diff body (M2 fix).
fn gutter_style(style_ctx: DiffRowStyle, theme: &Theme) -> Style {
    if style_ctx.focused {
        foreground(theme, Token::Accent)
    } else {
        foreground(theme, Token::TextMuted)
    }
}

/// One screen row's position and width, bundled so drawing a row doesn't
/// need three separate `u16` arguments (clippy's argument-count limit).
#[derive(Clone, Copy)]
struct RowSlot {
    x: u16,
    y: u16,
    width: u16,
}

fn draw_diff_row(
    row: &DiffRow<'_>,
    gutter: usize,
    slot: RowSlot,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    match row {
        DiffRow::Heading(heading) => draw_heading(heading, slot, buf, theme),
        DiffRow::Line(line) => draw_line_row(line, gutter, slot, buf, theme, style_ctx),
    }
}

fn draw_heading(heading: &SanitizedText, slot: RowSlot, buf: &mut Buffer, theme: &Theme) {
    let style = foreground(theme, Token::TextMuted);
    let fitted = fit(heading.as_str(), usize::from(slot.width));
    buf.set_stringn(slot.x, slot.y, fitted, usize::from(slot.width), style);
}

fn draw_truncation_row(shown: usize, total: usize, slot: RowSlot, buf: &mut Buffer, theme: &Theme) {
    let style = foreground(theme, Token::TextMuted);
    let more = total.saturating_sub(shown);
    let text = format!("\u{2026} {more} more lines \u{b7} L load all");
    let fitted = fit(&text, usize::from(slot.width));
    buf.set_stringn(slot.x, slot.y, fitted, usize::from(slot.width), style);
}

/// One diff line: `{old:>gutter} {new:>gutter} {sign}{text}`, the gutter
/// blank on whichever side has no number for this line.
fn draw_line_row(
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
    let prefix = format!("{old:>gutter$} {new:>gutter$} ");
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
fn line_style_at(
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
fn draw_styled_text(
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

/// The detail pane widget: header, files and body, drawn for one frame.
pub struct DetailPane<'a> {
    /// What to draw.
    pub view: &'a DetailView,
    /// Where the pane's regions land.
    pub layout: &'a DetailLayout,
    /// Not in the ticket's literal signature; see the module doc.
    pub theme: &'a Theme,
}

impl Widget for DetailPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let files_count = self.view.files.as_ref().map_or(0, |files| files.rows.len());
        let areas = detail_areas(area, *self.layout, files_count);
        render_header(self.view, *self.layout, areas.header, self.theme, buf);
        if let Some(explain) = &self.view.explain {
            // Same area the files+body split otherwise occupies, at every
            // breakpoint (`m2-details-and-diffs/17`).
            render_explain(explain, areas.files.union(areas.body), self.theme, buf);
        } else {
            render_files(self.view, areas.files, self.theme, buf);
            render_body(self.view, areas.body, self.theme, buf);
        }
    }
}

fn draw_line(buf: &mut Buffer, area: Rect, offset: u16, text: &str, style: Style) {
    if offset >= area.height || area.width == 0 {
        return;
    }
    let y = area.y.saturating_add(offset);
    let fitted = fit(text, usize::from(area.width));
    buf.set_stringn(area.x, y, fitted, usize::from(area.width), style);
}

fn title_text(subject: &Subject) -> String {
    match subject {
        Subject::None => String::new(),
        Subject::Commit { sha } => format!("COMMIT {}", sha.as_str()),
        Subject::Compare { base, head } => {
            format!("COMPARE {} \u{2192} {}", base.as_str(), head.as_str())
        }
        Subject::WorkingTree {
            staged,
            unstaged,
            untracked,
        } => {
            let mut parts = Vec::new();
            if *staged > 0 {
                parts.push(format!("{staged} staged"));
            }
            if *unstaged > 0 {
                parts.push(format!("{unstaged} unstaged"));
            }
            if *untracked > 0 {
                parts.push(format!("{untracked} untracked"));
            }
            if parts.is_empty() {
                "UNCOMMITTED".to_string()
            } else {
                format!("UNCOMMITTED \u{b7} {}", parts.join(" "))
            }
        }
    }
}

fn radar_text(line: &RadarLine) -> String {
    match line {
        RadarLine::Finding { title, .. } => title.as_str().to_string(),
        RadarLine::EvilUnchecked => "evil merge: unchecked".to_string(),
        RadarLine::EvilChecking => "evil merge: checking\u{2026}".to_string(),
        RadarLine::EvilClean => "evil merge: clean".to_string(),
        RadarLine::EvilResolved { files } => {
            let plural = if *files == 1 { "" } else { "s" };
            format!("evil merge: clean ({files} file{plural} hand-resolved)")
        }
        RadarLine::EvilNotCheckable(reason) => {
            format!("evil merge: not checkable \u{2014} {}", reason.as_str())
        }
        RadarLine::ForcePushNotCheckable(reason) => {
            format!("force-pushed: not checkable \u{2014} {}", reason.as_str())
        }
    }
}

/// All of the header's radar lines, joined into the one slot the header
/// layouts reserve for them (`header_lines`) — every [`RadarLine`] the pane
/// carries shows, not just the first (`m2-details-and-diffs/13c`).
fn radar_lines_text(lines: &[RadarLine]) -> String {
    lines
        .iter()
        .map(radar_text)
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// The author line: `name <email> · date · relative`, plus `committed by`
/// and `with:` (co-authors) when the header carries them.
fn author_line(header: &DetailHeader) -> String {
    let mut line = String::new();
    if !header.author.as_str().is_empty() {
        line.push_str(header.author.as_str());
        line.push_str(" \u{b7} ");
        line.push_str(header.author_date.as_str());
        line.push_str(" \u{b7} ");
        line.push_str(header.relative.as_str());
    }
    if let Some(committer) = &header.committer {
        line.push_str(" \u{b7} committed by ");
        line.push_str(committer.as_str());
    }
    if !header.co_authors.is_empty() {
        line.push_str(" \u{b7} with: ");
        let names: Vec<&str> = header
            .co_authors
            .iter()
            .map(SanitizedText::as_str)
            .collect();
        line.push_str(&names.join(", "));
    }
    line
}

/// The header's five canonical lines (title, author, parents/against,
/// contained, radar), before a narrower breakpoint compresses them.
fn header_lines(view: &DetailView) -> [String; 5] {
    let header = &view.header;
    let title = title_text(&header.subject);

    if let Subject::WorkingTree { .. } = header.subject {
        let against = header.against.as_ref().map_or("", |sha| sha.as_str());
        return [
            title,
            format!("diff vs {against}"),
            String::new(),
            String::new(),
            String::new(),
        ];
    }

    let author_line = author_line(header);

    let parents_line = if header.parents.is_empty() {
        header.against.as_ref().map_or(String::new(), |against| {
            format!("diff vs {}", against.as_str())
        })
    } else {
        let names: Vec<&str> = header.parents.iter().map(SanitizedText::as_str).collect();
        format!("parents {} \u{b7} diff vs first parent", names.join(", "))
    };

    let contained_line = match &header.contained {
        ContainedIn::NotAsked => String::new(),
        ContainedIn::Checking => "Contained in: checking\u{2026}".to_string(),
        ContainedIn::Known {
            branches,
            tags,
            more,
        } => {
            let mut names: Vec<&str> = branches.iter().map(SanitizedText::as_str).collect();
            names.extend(tags.iter().map(SanitizedText::as_str));
            let mut line = format!("Contained in: {}", names.join(", "));
            if *more > 0 {
                line.push_str(&format!(" +{more} more"));
            }
            line
        }
        ContainedIn::Failed(message) => format!("Contained in: {}", message.as_str()),
    };

    let radar_line = radar_lines_text(&header.radar);

    [title, author_line, parents_line, contained_line, radar_line]
}

fn title_style(view: &DetailView, theme: &Theme) -> Style {
    if view.focus == DetailFocus::Graph {
        foreground(theme, Token::TextMuted)
    } else {
        foreground(theme, Token::Accent)
    }
}

fn render_header(
    view: &DetailView,
    layout: DetailLayout,
    area: Rect,
    theme: &Theme,
    buf: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let lines = header_lines(view);
    let is_working_tree = matches!(view.header.subject, Subject::WorkingTree { .. });
    let meta = [&lines[1], &lines[2], &lines[3]]
        .into_iter()
        .filter(|line| !line.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    let mut subject_radar = view.header.summary.as_str().to_string();
    if !lines[4].is_empty() {
        subject_radar.push_str(" \u{b7} ");
        subject_radar.push_str(&lines[4]);
    }

    let displayed: Vec<String> = match layout {
        DetailLayout::Right { .. } => lines.to_vec(),
        DetailLayout::Below { .. } => {
            if is_working_tree {
                vec![lines[0].clone(), lines[1].clone()]
            } else {
                let meta_line = if meta.is_empty() {
                    lines[0].clone()
                } else {
                    format!("{} \u{b7} {meta}", lines[0])
                };
                vec![meta_line, subject_radar]
            }
        }
        DetailLayout::Tab { .. } => {
            if is_working_tree {
                vec![lines[0].clone(), lines[1].clone(), String::new()]
            } else {
                vec![lines[0].clone(), meta, subject_radar]
            }
        }
    };

    let title_style = title_style(view, theme);
    let text_style = foreground(theme, Token::Text);
    for (index, line) in displayed.iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        let style = if index == 0 { title_style } else { text_style };
        draw_line(buf, area, offset, line, style);
    }
}

fn status_letter(status: FileStatusView) -> char {
    match status {
        FileStatusView::Added => 'A',
        FileStatusView::Deleted => 'D',
        FileStatusView::Modified => 'M',
        FileStatusView::Renamed => 'R',
        FileStatusView::Copied => 'C',
        FileStatusView::TypeChanged => 'T',
        FileStatusView::Unmerged => 'U',
        FileStatusView::Untracked => '?',
    }
}

fn file_row_text(row: &FileRow, width: usize) -> String {
    let indent = "  ".repeat(usize::from(row.depth));
    let (left, right) = match &row.kind {
        FileRowKind::Message { lines } => {
            (format!("\u{2709} message ({lines} lines)"), String::new())
        }
        FileRowKind::Dir { collapsed } => {
            let marker = if *collapsed { "\u{25b8}" } else { "\u{25be}" };
            (
                format!("{indent}{marker} {}", row.name.as_str()),
                String::new(),
            )
        }
        FileRowKind::File {
            status,
            added,
            removed,
        } => {
            let letter = status_letter(*status);
            let counts = match (added, removed) {
                (Some(added), Some(removed)) => format!("+{added} \u{2212}{removed}"),
                _ => "bin".to_string(),
            };
            (format!("{indent}{letter} {}", row.name.as_str()), counts)
        }
    };
    if right.is_empty() {
        left
    } else {
        let gap = width
            .saturating_sub(left.chars().count() + right.chars().count())
            .max(1);
        format!("{left}{}{right}", " ".repeat(gap))
    }
}

fn render_files(view: &DetailView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let Some(files) = view.files.as_ref() else {
        return;
    };
    let text_style = if view.focus == DetailFocus::Files {
        foreground(theme, Token::Accent)
    } else {
        foreground(theme, Token::TextMuted)
    };
    let cursor_bg = color(theme.token(Token::SurfaceRaised));

    for (offset, (index, row)) in (0_u16..).zip(files.visible().skip(files.top)) {
        if offset >= area.height {
            break;
        }
        let style = if index == files.cursor {
            text_style.bg(cursor_bg)
        } else {
            text_style
        };
        let text = file_row_text(row, usize::from(area.width));
        draw_line(buf, area, offset, &text, style);
    }
}

fn render_body(view: &DetailView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let focused = view.focus == DetailFocus::Diff;
    let muted = foreground(theme, Token::TextMuted);
    let warning = foreground(theme, Token::Warning);
    let text_style = if focused {
        foreground(theme, Token::Text)
    } else {
        muted
    };

    match &view.body {
        BodyView::Empty(text) => draw_line(buf, area, 0, text.as_str(), muted),
        BodyView::Loading(text) => draw_line(buf, area, 0, text.as_str(), text_style),
        BodyView::Failed(text) => draw_line(buf, area, 0, text.as_str(), warning),
        BodyView::Message { lines } => {
            for (index, line) in lines.iter().enumerate() {
                let Ok(offset) = u16::try_from(index) else {
                    break;
                };
                draw_line(buf, area, offset, line.as_str(), text_style);
            }
        }
        BodyView::Diff(diff) => render_diff_body(view, diff, focused, area, theme, buf, muted),
    }
}

/// The [`BodyView::Diff`] arm of [`render_body`], split out to keep that
/// function under clippy's line-count limit.
fn render_diff_body(
    view: &DetailView,
    diff: &crate::detail::DiffView,
    focused: bool,
    area: Rect,
    theme: &Theme,
    buf: &mut Buffer,
    muted: Style,
) {
    if diff.untracked {
        draw_line(buf, area, 0, "untracked \u{b7} no diff yet (M3)", muted);
        return;
    }
    if let Some(binary) = &diff.binary {
        draw_line(
            buf,
            area,
            0,
            &format!("binary \u{b7} {}", binary.as_str()),
            muted,
        );
        return;
    }
    let focus = DiffFocus {
        focused,
        word_tint: view.options.word_tint,
    };
    let _rows = match view.mode {
        DiffMode::Unified => diff_body(
            &diff.hunks,
            view.body_top,
            diff.truncated,
            area,
            buf,
            theme,
            focus,
        ),
        DiffMode::SideBySide => diff_body_side_by_side(
            &diff.hunks,
            view.body_top,
            area,
            buf,
            theme,
            focus.focused,
            focus.word_tint,
        ),
    };
}

/// The Explain panel: title (severity glyph + `title`, severity colour),
/// paragraphs wrapped to width, a blank line, the sketch verbatim, footer
/// `Esc close` (`m2-details-and-diffs/17`; design note "Explain panel").
fn render_explain(explain: &ExplainView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (glyph, severity_token) = match explain.severity {
        Severity::Warning => ("\u{26a0} ", Token::Warning),
        Severity::Info => ("\u{24d8} ", Token::TextMuted),
    };
    let title_style = foreground(theme, severity_token);
    let text_style = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    let width = usize::from(area.width);

    let mut offset: u16 = 0;
    draw_line(
        buf,
        area,
        offset,
        &format!("{glyph}{}", explain.title.as_str()),
        title_style,
    );
    offset += 1;

    'paragraphs: for paragraph in &explain.paragraphs {
        for line in wrap_to_width(paragraph.as_str(), width) {
            if offset >= area.height {
                break 'paragraphs;
            }
            draw_line(buf, area, offset, &line, text_style);
            offset += 1;
        }
    }

    offset += 1; // blank line between the paragraphs and the sketch.

    for line in &explain.sketch {
        if offset >= area.height {
            return;
        }
        draw_line(buf, area, offset, line.as_str(), text_style);
        offset += 1;
    }

    if offset < area.height {
        draw_line(buf, area, offset, "Esc close", muted);
    }
}

/// Greedy word wrap at `width` cells (`unicode-width`, matching `fit`'s own
/// notion of a cell). An empty `text` wraps to no lines.
fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let extra = usize::from(!current.is_empty());
        if !current.is_empty() && current.width() + extra + word.width() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests;
