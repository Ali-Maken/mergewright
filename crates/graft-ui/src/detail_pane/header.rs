//! The pane's header region: subject, author, parents/against, contained-in
//! and radar lines, compressed per breakpoint.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::detail::{ContainedIn, DetailFocus, DetailHeader, DetailView, RadarLine, Subject};
use crate::paint::foreground;
use crate::theme::{Theme, Token};
use graft_term::sanitize::SanitizedText;

use super::layout::DetailLayout;
use super::line_render::draw_line;

pub(crate) fn title_text(subject: &Subject) -> String {
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
