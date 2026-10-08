//! One body row of the blame place: the gutter at its three widths, the
//! code, and the cursor row's trailer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

use super::{BlameRowView, BlameView, BlockPos, RowMark, put};
use crate::fit::fit;
use crate::graph_screen::{initials, relative_date, relative_date_long};

/// The gap between the code's last character and the trailer's `◂`.
const TRAILER_GAP: usize = 4;

/// The name field's width in the two wide gutters.
const NAME_CELLS: usize = 5;

/// The gutter's three widths (pass 1b's breakpoint table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Gutter {
    /// ≥180 columns: 46 cells, subject included.
    Wide,
    /// 120–179: 22 cells, subject dropped (the trailer carries it).
    Middle,
    /// <120: 10 cells, bracket + initials + age.
    Narrow,
}

impl Gutter {
    pub(super) fn for_width(width: u16) -> Gutter {
        match width {
            180.. => Gutter::Wide,
            120..=179 => Gutter::Middle,
            _ => Gutter::Narrow,
        }
    }

    fn cells(self) -> usize {
        match self {
            Gutter::Wide => 46,
            Gutter::Middle => 22,
            Gutter::Narrow => 10,
        }
    }
}

/// One body row being drawn.
pub(super) struct Line<'r> {
    pub(super) row: &'r BlameRowView,
    pub(super) cursor: bool,
    pub(super) gutter: Gutter,
    pub(super) number_cells: usize,
}

impl Line<'_> {
    pub(super) fn draw(&self, buf: &mut Buffer, x: u16, y: u16, width: u16, view: &BlameView) {
        let base = if self.cursor {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        if self.cursor {
            buf.set_style(
                Rect {
                    x,
                    y,
                    width,
                    height: 1,
                },
                base,
            );
        }
        let gutter = self.gutter_text(view.now);
        let mut gutter_style = base.add_modifier(Modifier::DIM);
        if view.heatmap
            && let Some(who) = &self.row.who
        {
            gutter_style = gutter_style.fg(heat(view.now - who.author_time));
        }
        let cells = self.gutter.cells();
        put(buf, x, y, width, &gutter, gutter_style);
        let code = format!(
            "│ {:>w$}  {}",
            self.row.line_no,
            self.row.text,
            w = self.number_cells
        );
        let code_x = x.saturating_add(u16::try_from(cells).unwrap_or(u16::MAX));
        let code_room = width.saturating_sub(u16::try_from(cells).unwrap_or(u16::MAX));
        put(buf, code_x, y, code_room, &code, base);
        if self.cursor {
            let used = cells + code.width();
            let start = used + TRAILER_GAP;
            let room = usize::from(width).saturating_sub(start);
            if let Some(trailer) = trailer(self.row, view.now, room) {
                let trailer_x = x.saturating_add(u16::try_from(start).unwrap_or(u16::MAX));
                put(
                    buf,
                    trailer_x,
                    y,
                    u16::try_from(room).unwrap_or(u16::MAX),
                    &trailer,
                    base.add_modifier(Modifier::DIM),
                );
            }
        }
    }

    /// The gutter's text, exactly `self.gutter.cells()` wide.
    fn gutter_text(&self, now: i64) -> String {
        let bracket = if self.cursor {
            '▸'
        } else {
            match self.row.block {
                BlockPos::Single => '─',
                BlockPos::First => '┌',
                BlockPos::Middle => '│',
                BlockPos::Last => '└',
            }
        };
        let first = matches!(self.row.block, BlockPos::First | BlockPos::Single);
        let cells = self.gutter.cells();
        let mut text = format!(" {bracket} ");
        if self.row.mark == RowMark::Shallow {
            if let Some(who) = &self.row.who
                && (first || self.cursor)
            {
                text.push_str(&shallow_gutter(self.gutter, &who.sha7));
            }
            return pad(&text, cells);
        }
        if let Some(who) = &self.row.who {
            let named = first || self.cursor;
            match self.gutter {
                Gutter::Narrow => {
                    if named {
                        let age = relative_date(who.author_time, now);
                        text.push_str(&format!("{:<2} {age:>4}", initials(&who.name)));
                    }
                }
                Gutter::Wide | Gutter::Middle => {
                    if first {
                        let age = relative_date(who.author_time, now);
                        text.push_str(&format!(
                            "{} {} {age:>4}",
                            who.sha7,
                            pad(&who.name, NAME_CELLS)
                        ));
                        if self.gutter == Gutter::Wide {
                            text.push_str("  ");
                            let room = cells.saturating_sub(text.width() + 1);
                            text.push_str(&fit(&who.subject, room));
                        }
                    } else if self.cursor {
                        // Mid-block cursor row: the name, where the first row
                        // puts it, so the eye finds him without looking up.
                        text.push_str(&format!("{} {}", " ".repeat(7), pad(&who.name, NAME_CELLS)));
                    }
                }
            }
        }
        pad(&text, cells)
    }
}

/// A shallow-boundary row's gutter words after the bracket: git's `^` form
/// of the sha (`^` + 6 hex, so the width holds) and words in place of the
/// name, which is not known.
fn shallow_gutter(gutter: Gutter, sha7: &str) -> String {
    let marked: String = std::iter::once('^').chain(sha7.chars().take(6)).collect();
    match gutter {
        Gutter::Wide => format!("{marked} shallow clone · can't say who"),
        Gutter::Middle => format!("{marked} shallow"),
        Gutter::Narrow => "shallow".to_owned(),
    }
}

/// The trailer for the cursor row, fitted to `room` cells, or `None` when
/// even `◂ Name · age` would not fit (the gutter already names him).
pub(super) fn trailer(row: &BlameRowView, now: i64, room: usize) -> Option<String> {
    let fixed = |text: &str| (text.width() <= room).then(|| text.to_owned());
    match row.mark {
        RowMark::NotCommitted => return fixed("◂ not committed yet"),
        RowMark::Unblamable => return fixed("◂ git can't say who"),
        RowMark::Shallow => {
            let sha7 = row.who.as_ref().map_or("", |who| who.sha7.as_str());
            let full = format!("◂ can't say who · this clone starts at {sha7}");
            return fixed(&full).or_else(|| fixed("◂ can't say who"));
        }
        RowMark::Plain | RowMark::Skipped => {}
    }
    let who = row.who.as_ref()?;
    let skipped = if row.mark == RowMark::Skipped {
        "(skipped commit) "
    } else {
        ""
    };
    let head = format!(
        "◂ {skipped}{} · {}",
        who.name,
        relative_date_long(who.author_time, now)
    );
    let full = format!("{head} · {} · {}", who.subject, who.sha7);
    if full.width() <= room {
        return Some(full);
    }
    let unhashed = format!("{head} · {}", who.subject);
    if unhashed.width() <= room {
        return Some(unhashed);
    }
    let prefix = format!("{head} · ");
    if let Some(cut) = cut_at_word(&who.subject, room.saturating_sub(prefix.width())) {
        return Some(format!("{prefix}{cut}"));
    }
    fixed(&head)
}

/// `subject` cut at a word boundary so that it and a trailing `…` fit in
/// `room` cells; `None` if not even its first word fits.
fn cut_at_word(subject: &str, room: usize) -> Option<String> {
    let mut kept = String::new();
    for word in subject.split_whitespace() {
        let next = if kept.is_empty() {
            word.to_owned()
        } else {
            format!("{kept} {word}")
        };
        if next.width() + 1 > room {
            break;
        }
        kept = next;
    }
    (!kept.is_empty()).then(|| format!("{kept}…"))
}

/// `text` fitted and padded to exactly `cells`.
fn pad(text: &str, cells: usize) -> String {
    let fitted = fit(text, cells);
    let short = cells.saturating_sub(fitted.width());
    format!("{fitted}{}", " ".repeat(short))
}

/// The heatmap's gutter colour by age: bright for recent, dim for old
/// (DESIGN §1.12). Colour only; the printed age carries the same fact.
fn heat(elapsed: i64) -> Color {
    const WEEK: i64 = 7 * 86_400;
    const MONTH: i64 = 30 * 86_400;
    const HALF_YEAR: i64 = 180 * 86_400;
    const YEAR: i64 = 365 * 86_400;
    if elapsed < WEEK {
        Color::LightYellow
    } else if elapsed < MONTH {
        Color::Yellow
    } else if elapsed < HALF_YEAR {
        Color::LightRed
    } else if elapsed < YEAR {
        Color::Magenta
    } else {
        Color::DarkGray
    }
}
