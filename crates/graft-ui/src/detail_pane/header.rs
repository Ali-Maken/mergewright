//! The pane's header region: subject, author, parents/against, contained-in
//! and radar lines, compressed per breakpoint.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::detail::{
    CompareHeader, CompareState, ContainedIn, DetailFocus, DetailHeader, DetailView, RadarLine,
    Subject,
};
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};
use graft_term::sanitize::SanitizedText;
use unicode_width::UnicodeWidthStr;

use super::layout::DetailLayout;
use super::line_render::draw_line;

pub(crate) fn title_text(subject: &Subject, keys: &Keymap) -> String {
    match subject {
        Subject::None => String::new(),
        Subject::Commit { sha } => format!("COMMIT {}", sha.as_str()),
        Subject::Compare(compare) => compare_title(compare, usize::MAX, keys),
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

/// A compare's first line in words (m7-14), fitted to `width` cells by a
/// ladder: drop `· F files +a −d`, then `· N commits`, then cut the head's
/// name with `…`. A number is never cut. On the base itself, `· x clears
/// it` spells `graph.compare` from `keys`, left out when unbound (m8/09d).
pub(crate) fn compare_title(compare: &CompareHeader, width: usize, keys: &Keymap) -> String {
    let base = compare.base.as_str();
    let head = compare.head.as_str();
    let fits = |text: &String| text.width() <= width;
    match &compare.state {
        CompareState::IsBase => keys.after(
            "this is the compare base",
            ActionId("graph.compare"),
            "clears it",
        ),
        CompareState::Reading => fit_head(width, head, |c| {
            format!("comparing {c} with {base} \u{2026}")
        }),
        CompareState::AlreadyIn => fit_head(width, head, |c| {
            format!("{c} is already in {base} \u{b7} nothing of its own")
        }),
        CompareState::NoSharedHistory => fit_head(width, head, |c| {
            format!("{c} and {base} share no history \u{b7} showing {base}..{c}")
        }),
        CompareState::Failed(why) => format!("couldn't compare: {}", why.as_str()),
        CompareState::Since { commits, tally } => {
            let lead = |c: &str| format!("changes on {c} since it left {base}");
            let plural = if *commits == 1 { "" } else { "s" };
            let count = format!(" \u{b7} {commits} commit{plural}");
            if let Some(tally) = tally {
                let files = if tally.files == 1 { "file" } else { "files" };
                let full = format!(
                    "{}{count} \u{b7} {} {files} +{} \u{2212}{}",
                    lead(head),
                    tally.files,
                    tally.added,
                    tally.removed
                );
                if fits(&full) {
                    return full;
                }
            }
            let counted = format!("{}{count}", lead(head));
            if fits(&counted) {
                return counted;
            }
            fit_head(width, head, lead)
        }
    }
}

/// `words(head)` if it fits `width`; else `words` with `head` cut to fit,
/// ending in `…`. Only the name is cut.
fn fit_head(width: usize, head: &str, words: impl Fn(&str) -> String) -> String {
    let whole = words(head);
    if whole.width() <= width {
        return whole;
    }
    let chars: Vec<char> = head.chars().collect();
    for keep in (0..chars.len()).rev() {
        let cut: String = chars[..keep].iter().chain(['\u{2026}'].iter()).collect();
        let line = words(&cut);
        if line.width() <= width {
            return line;
        }
    }
    words("\u{2026}")
}

pub(crate) fn radar_text(line: &RadarLine) -> String {
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
        RadarLine::NotChecked(why) => format!(
            "radar: not checked \u{2014} {} \u{00b7} ctrl+r to try again",
            why.as_str()
        ),
    }
}

/// All of the header's radar lines, joined into the one slot the header
/// layouts reserve for them (`header_lines`) — every [`RadarLine`] the pane
/// carries shows, not just the first (`m2-details-and-diffs/13c`).
pub(crate) fn radar_lines_text(lines: &[RadarLine]) -> String {
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
fn header_lines(view: &DetailView, keys: &Keymap) -> [String; 5] {
    let header = &view.header;
    let title = title_text(&header.subject, keys);

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

pub(crate) fn title_style(view: &DetailView, theme: &Theme) -> Style {
    if view.focus == DetailFocus::Graph {
        foreground(theme, Token::TextMuted)
    } else {
        foreground(theme, Token::Accent)
    }
}

pub(crate) fn render_header(
    view: &DetailView,
    layout: DetailLayout,
    area: Rect,
    theme: &Theme,
    keys: &Keymap,
    buf: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mut lines = header_lines(view, keys);
    if let Subject::Compare(compare) = &view.header.subject {
        lines[0] = compare_title(compare, usize::from(area.width), keys);
    }
    let displayed = displayed_lines(view, layout, &lines);

    let title_style = title_style(view, theme);
    let text_style = foreground(theme, Token::Text);
    // `w` on: the title gives up the tail's cells, so a long title is cut
    // with `…` rather than the tail dropped — unless the row is too narrow
    // to keep a readable title beside it.
    let tail = whitespace_tail_width(view, area);
    for (index, line) in displayed.iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        let style = if index == 0 { title_style } else { text_style };
        let row_area = if index == 0 {
            Rect {
                width: area.width - tail,
                ..area
            }
        } else {
            area
        };
        draw_line(buf, row_area, offset, line, style);
    }
    if tail > 0
        && let Some(title) = displayed.first()
    {
        let used = title.trim_end().width().min(usize::from(area.width - tail));
        draw_whitespace_tail(area, used, theme, buf);
    }
    if view.following {
        draw_following(area, theme, buf);
    }
}

/// The header's rows for this breakpoint, from the five canonical lines.
fn displayed_lines(view: &DetailView, layout: DetailLayout, lines: &[String; 5]) -> Vec<String> {
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

    match layout {
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
    }
}

/// The cells the title row keeps for `· whitespace ignored`: none with `w`
/// off, or when fewer than [`TITLE_MIN`] cells of title would be left.
fn whitespace_tail_width(view: &DetailView, area: Rect) -> u16 {
    let width = u16::try_from(crate::blame::WHITESPACE_IGNORED.width()).unwrap_or(u16::MAX);
    if view.options.ignore_whitespace && area.width >= width.saturating_add(TITLE_MIN) {
        width
    } else {
        0
    }
}

/// The narrowest title worth keeping beside the whitespace tail.
const TITLE_MIN: u16 = 12;

/// `· whitespace ignored`, muted, right after the title's text while `w` is
/// on (m8-polish-and-ship/17 item 8); cut at the area's edge.
fn draw_whitespace_tail(area: Rect, used: usize, theme: &Theme, buf: &mut Buffer) {
    let Ok(used) = u16::try_from(used) else {
        return;
    };
    if used >= area.width || area.height == 0 {
        return;
    }
    let room = usize::from(area.width - used);
    buf.set_stringn(
        area.x + used,
        area.y,
        crate::blame::WHITESPACE_IGNORED,
        room,
        foreground(theme, Token::TextMuted),
    );
}

/// What the title row ends in while the pane waits for the cursor to rest.
pub(crate) const FOLLOWING: &str = "reading\u{2026}";

/// `reading…`, muted, at the title row's right end — over the title's tail
/// when the row is too narrow for both.
fn draw_following(area: Rect, theme: &Theme, buf: &mut Buffer) {
    let width = FOLLOWING.width();
    let Ok(cells) = u16::try_from(width) else {
        return;
    };
    if area.width <= cells || area.height == 0 {
        return;
    }
    let x = area.x + area.width - cells;
    // One blank cell before it, so it never reads as part of the title.
    buf.set_stringn(x - 1, area.y, " ", 1, foreground(theme, Token::TextMuted));
    buf.set_stringn(
        x,
        area.y,
        FOLLOWING,
        width,
        foreground(theme, Token::TextMuted),
    );
}
