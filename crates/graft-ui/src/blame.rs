//! The blame place's picture (m7-power-tools/20c; design note pass 1b
//! "Blame — the name at the line", pass 2b B1): a title, the ignore-revs
//! line when the repo lists revisions, then one row per line of the file —
//! gutter · `│` · line number · code, and a trailer on the cursor row that
//! says who, when and why at the end of the line he is on.
//!
//! A pure draw from [`BlameView`]; no state, no keys. The bar under it is the
//! frame's bottom row, drawn by the hint catalogue, not here.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

use crate::fit::fit;
use crate::graph_screen::{initials, relative_date, relative_date_long};
use crate::keymap::{ActionId, Keymap};

/// Which revision of the file is blamed; the title's last words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameRev {
    /// The file as it is now (on disk, or the tip).
    Now,
    /// As of a commit (sha7).
    AsOf(String),
    /// As of a commit's parent, after `^` or from a removed line (sha7).
    Before(String),
}

/// The commit a line's block belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Who {
    /// Short sha, 7 hex digits.
    pub sha7: String,
    /// Author name.
    pub name: String,
    /// Author time, Unix seconds.
    pub author_time: i64,
    /// The commit's subject line.
    pub subject: String,
}

/// Where a row sits in its commit's run of consecutive lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockPos {
    /// A one-row block: `─`.
    Single,
    /// First row: `┌`, and the block's sha, name, age and subject.
    First,
    /// Inner row: `│`.
    Middle,
    /// Last row: `└`.
    Last,
}

/// What git said about a line beyond its commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowMark {
    /// An ordinary committed line.
    Plain,
    /// Changed on disk, not committed: trails `◂ not committed yet`.
    NotCommitted,
    /// git passed it to no one because its commit is ignored (`?`/`*`
    /// marks): trails `◂ (skipped commit) Name …`.
    Skipped,
    /// git could not say who: trails `◂ git can't say who`.
    Unblamable,
    /// Its commit is where this shallow clone starts: nothing older was
    /// fetched, so who wrote the line is unknown. Only `who.sha7` is drawn,
    /// marked `^` as git marks a boundary (20's shallow ruling); the name,
    /// age and subject never are.
    Shallow,
}

/// One line of the blamed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameRowView {
    /// 1-based line number in the blamed revision.
    pub line_no: u32,
    /// The line's text, already sanitised.
    pub text: String,
    /// Its commit; `None` where there is none to name.
    pub who: Option<Who>,
    /// Its place in the block.
    pub block: BlockPos,
    /// Beyond-the-commit marks.
    pub mark: RowMark,
}

/// The body, by state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlameBody<'a> {
    /// git is still reading: the title says so, the body is empty.
    Reading,
    /// git failed; its first line.
    Failed(&'a str),
    /// The lines, the cursor's index into `rows`, and the first row shown.
    Lines {
        /// Every line of the file.
        rows: &'a [BlameRowView],
        /// The cursor row.
        cursor: usize,
        /// The first row drawn.
        top: usize,
    },
}

/// The ignore-revs line: drawn only when the repo has the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IgnoreLine {
    /// How many revisions the file lists.
    pub count: usize,
    /// Whether they are being skipped now.
    pub applied: bool,
}

/// An active find, for the title's tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindTitle<'a> {
    /// What he typed.
    pub query: &'a str,
    /// Which match the cursor is on, 1-based.
    pub at: usize,
    /// How many matches; 0 reads `no match`.
    pub of: usize,
}

/// Everything the blame place draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameView<'a> {
    /// The file's path, sanitised.
    pub path: &'a str,
    /// Which revision.
    pub rev: BlameRev,
    /// The body's state.
    pub body: BlameBody<'a>,
    /// The ignore-revs line, or `None` without the file.
    pub ignore: Option<IgnoreLine>,
    /// An active find.
    pub find: Option<FindTitle<'a>>,
    /// `space`: colour each block's gutter by age (colour only).
    pub heatmap: bool,
    /// The clock the ages are read against, Unix seconds.
    pub now: i64,
}

/// The gap between the code's last character and the trailer's `◂`.
const TRAILER_GAP: usize = 4;

/// The name field's width in the two wide gutters.
const NAME_CELLS: usize = 5;

/// The gutter's three widths (pass 1b's breakpoint table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gutter {
    /// ≥180 columns: 46 cells, subject included.
    Wide,
    /// 120–179: 22 cells, subject dropped (the trailer carries it).
    Middle,
    /// <120: 10 cells, bracket + initials + age.
    Narrow,
}

impl Gutter {
    fn for_width(width: u16) -> Gutter {
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

/// Draws the blame place into `area`, its keys spelled from `keys`.
pub fn draw_blame(frame: &mut Frame, area: Rect, view: &BlameView, keys: &Keymap) {
    draw_into(frame.buffer_mut(), area, view, keys);
}

/// Which row of `rows` sits at screen row `y`, for a click; `None` on the
/// title, the ignore-revs line, past the last row, or outside the body.
#[must_use]
pub fn blame_row_at(area: Rect, view: &BlameView, y: u16) -> Option<usize> {
    let BlameBody::Lines { rows, top, .. } = view.body else {
        return None;
    };
    let body = body_area(area, view);
    if y < body.y || y >= body.bottom() {
        return None;
    }
    let index = top + usize::from(y - body.y);
    (index < rows.len()).then_some(index)
}

/// The rows under the title and the ignore-revs line.
fn body_area(area: Rect, view: &BlameView) -> Rect {
    let header = 1 + u16::from(view.ignore.is_some());
    let header = header.min(area.height);
    Rect {
        y: area.y + header,
        height: area.height - header,
        ..area
    }
}

fn draw_into(buf: &mut Buffer, area: Rect, view: &BlameView, keys: &Keymap) {
    if area.is_empty() {
        return;
    }
    put(
        buf,
        area.x,
        area.y,
        area.width,
        &title(view, keys),
        Style::default().add_modifier(Modifier::BOLD),
    );
    if let Some(ignore) = view.ignore
        && area.height > 1
    {
        put(
            buf,
            area.x,
            area.y + 1,
            area.width,
            &ignore_words(ignore, keys),
            Style::default().add_modifier(Modifier::DIM),
        );
    }
    let body = body_area(area, view);
    match view.body {
        BlameBody::Reading => {}
        BlameBody::Failed(first) => {
            if body.height > 0 {
                let words = keys.after(&format!(" git blame stopped: {first}"), BACK, "back");
                put(buf, body.x, body.y, body.width, &words, Style::default());
            }
        }
        BlameBody::Lines { rows, cursor, top } => {
            let gutter = Gutter::for_width(area.width);
            let number_cells = number_cells(rows);
            for (offset, (index, row)) in rows
                .iter()
                .enumerate()
                .skip(top)
                .take(usize::from(body.height))
                .enumerate()
            {
                let y = body.y + u16::try_from(offset).unwrap_or(u16::MAX);
                let line = Line {
                    row,
                    cursor: index == cursor,
                    gutter,
                    number_cells,
                };
                line.draw(buf, body.x, y, body.width, view);
            }
        }
    }
}

/// `app.back`: the key every blame line names for leaving.
const BACK: ActionId = ActionId("app.back");

/// The title row, by state.
fn title(view: &BlameView, keys: &Keymap) -> String {
    if view.body == BlameBody::Reading {
        return keys.after(&format!(" blaming {}…", view.path), BACK, "cancels");
    }
    let rev = match &view.rev {
        BlameRev::Now => "as it is now".to_owned(),
        BlameRev::AsOf(sha) => format!("as of {sha}"),
        BlameRev::Before(sha) => format!("before {sha}"),
    };
    let mut title = format!(" BLAME · {} · {rev}", view.path);
    if let Some(find) = view.find {
        if find.of == 0 {
            title.push_str(&format!(" · find \"{}\" · no match", find.query));
        } else {
            title.push_str(&format!(
                " · find \"{}\" {} of {}",
                find.query, find.at, find.of
            ));
        }
    }
    title
}

/// The ignore-revs clause; `blame.ignore_revs`' key spelled from `keys`,
/// the `· <key> …` part left out when he left it unbound.
fn ignore_words(ignore: IgnoreLine, keys: &Keymap) -> String {
    let count = ignore.count;
    let key = keys.spell_keys(&[ActionId("blame.ignore_revs")]);
    if ignore.applied {
        let commits = if count == 1 { "commit" } else { "commits" };
        let skipping = format!(" skipping {count} {commits} listed in .git-blame-ignore-revs");
        match key {
            Some(key) => format!("{skipping} \u{b7} {key} shows them"),
            None => skipping,
        }
    } else {
        match key {
            Some(key) => format!(
                " showing every commit \u{b7} {key} skips the {count} in .git-blame-ignore-revs"
            ),
            None => " showing every commit".to_owned(),
        }
    }
}

/// Cells the line-number field takes: the widest number, at least two.
fn number_cells(rows: &[BlameRowView]) -> usize {
    let widest = rows.iter().map(|row| row.line_no).max().unwrap_or(0);
    widest.to_string().len().max(2)
}

/// One body row being drawn.
struct Line<'r> {
    row: &'r BlameRowView,
    cursor: bool,
    gutter: Gutter,
    number_cells: usize,
}

impl Line<'_> {
    fn draw(&self, buf: &mut Buffer, x: u16, y: u16, width: u16, view: &BlameView) {
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
fn trailer(row: &BlameRowView, now: i64, room: usize) -> Option<String> {
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

/// Writes `text` at (`x`, `y`), clipped to `width` cells.
fn put(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    buf.set_stringn(x, y, text, usize::from(width), style);
}

#[cfg(test)]
mod tests;
