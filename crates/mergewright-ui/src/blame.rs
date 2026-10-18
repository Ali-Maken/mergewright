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
use ratatui::style::{Modifier, Style};

use crate::keymap::{ActionId, Keymap};
use crate::whitespace_tail::{put_whitespace_tail, whitespace_tail};
#[cfg(test)]
use line::trailer;
use line::{Gutter, Line};

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
    /// `w`: whitespace-only changes are ignored; the title's tail says so,
    /// muted (m8-polish-and-ship/17 item 8).
    pub whitespace_ignored: bool,
    /// Blame's own whitespace default (mergewright-git `BlameOptions::default`),
    /// handed in so the title names `w`'s state only when it departs from it.
    pub default_ignored: bool,
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
    let title = title(view, keys);
    put(
        buf,
        area.x,
        area.y,
        area.width,
        &title,
        Style::default().add_modifier(Modifier::BOLD),
    );
    // The tail names `w`'s state only when it departs from blame's own
    // default, which the view carries.
    if let Some(tail) = whitespace_tail(view.whitespace_ignored, view.default_ignored) {
        put_whitespace_tail(buf, area, &title, tail, put);
    }
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

/// Writes `text` at (`x`, `y`), clipped to `width` cells.
fn put(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    buf.set_stringn(x, y, text, usize::from(width), style);
}

mod line;

#[cfg(test)]
mod tests;
