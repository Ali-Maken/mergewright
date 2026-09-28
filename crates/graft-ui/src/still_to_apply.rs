//! The still-to-apply block: at a rewrite stop, a box between the banner and
//! the graph rows naming the steps git has not applied yet — **not commits
//! yet**, so never drawn as graph rows (field notes :38, "ghost rows").
//!
//! One row per pending pick-like line, newest first (`action sha7 subject`),
//! then one row per dependent branch still to come:
//! `<branch> (<author of its tip>): N commits follow, on top`. The caller caps
//! the pick rows at six and counts the rest in [`PendingView::more`]; a rect
//! too short for everything cuts further, ending in `+N more`
//! (design note R4; ticket 07c).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::frame_box::{border_row, content_row};

/// The box's title, in its top border.
pub const TITLE: &str = "still to apply · not commits yet";

/// Pick rows shown before the rest is counted as `+N more`.
pub const MAX_ROWS: usize = 6;

/// One pending todo line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRowView {
    /// `pick`, `edit`, `reword`, …
    pub action: String,
    /// The commit's id, seven characters.
    pub sha7: String,
    /// Its subject line.
    pub subject: String,
}

/// What the block draws. `rows` newest first, at most [`MAX_ROWS`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingView {
    /// The pending rows shown, newest first.
    pub rows: Vec<PendingRowView>,
    /// Pending lines beyond `rows`, not drawn.
    pub more: usize,
    /// `(branch, author of its tip, commits)` per dependent still to come.
    pub followers: Vec<(String, String, usize)>,
}

impl PendingView {
    /// Nothing pending: no block.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.more == 0 && self.followers.is_empty()
    }
}

/// Rows the whole block wants: two borders plus its content. Zero when
/// there is nothing to draw.
#[must_use]
pub fn still_to_apply_height(view: &PendingView) -> u16 {
    if view.is_empty() {
        return 0;
    }
    let content = content_lines(view).len();
    u16::try_from(content + 2).unwrap_or(u16::MAX)
}

fn follower_line(branch: &str, author: &str, commits: usize) -> String {
    let noun = if commits == 1 { "commit" } else { "commits" };
    format!("{branch} ({author}): {commits} {noun} follow, on top")
}

/// Every content line, uncut: picks, `+N more` when the caller capped them,
/// then the followers.
fn content_lines(view: &PendingView) -> Vec<String> {
    let mut lines: Vec<String> = view
        .rows
        .iter()
        .map(|row| format!("{} {} {}", row.action, row.sha7, row.subject))
        .collect();
    if view.more > 0 {
        lines.push(format!("+{} more", view.more));
    }
    lines.extend(
        view.followers
            .iter()
            .map(|(branch, author, commits)| follower_line(branch, author, *commits)),
    );
    lines
}

/// The content lines cut to `capacity`: when they don't fit, the last shown
/// line is `+N more`, N counting every pending line (pick or follower) that
/// is hidden, plus the caller's own `more`.
fn fitted_lines(view: &PendingView, capacity: usize) -> Vec<String> {
    let all = content_lines(view);
    if all.len() <= capacity {
        return all;
    }
    if capacity == 0 {
        return Vec::new();
    }
    let shown_picks = view.rows.len().min(capacity - 1);
    let hidden = view.rows.len() - shown_picks + view.more;
    let mut lines: Vec<String> = all[..shown_picks].to_vec();
    let room = capacity - 1 - shown_picks;
    let followers: Vec<String> = view
        .followers
        .iter()
        .map(|(branch, author, commits)| follower_line(branch, author, *commits))
        .collect();
    // Picks first; followers only if the picks all fit.
    let shown_followers = if shown_picks == view.rows.len() {
        room.min(followers.len())
    } else {
        0
    };
    // Always > 0 here: the lines did not fit, so something is hidden.
    let hidden = hidden + followers.len() - shown_followers;
    lines.push(format!("+{hidden} more"));
    lines.extend(followers.into_iter().take(shown_followers));
    lines
}

/// The block as plain text rows, exactly `area` sized (rows beyond the box
/// are not produced).
#[must_use]
pub fn still_to_apply_lines(view: &PendingView, width: u16, height: u16) -> Vec<String> {
    if view.is_empty() || height < 3 || width < 4 {
        return Vec::new();
    }
    let width = usize::from(width);
    let capacity = usize::from(height) - 2;
    let body = fitted_lines(view, capacity);
    let mut rows = Vec::with_capacity(body.len() + 2);
    rows.push(border_row('╭', '╮', TITLE, width));
    rows.extend(body.iter().map(|line| content_row(line, width)));
    rows.push(border_row('╰', '╯', "", width));
    rows
}

/// Draws the block into `area` (the rows between the banner and the graph).
pub fn draw_still_to_apply(frame: &mut Frame, area: Rect, view: &PendingView) {
    let lines: Vec<Line<'static>> = still_to_apply_lines(view, area.width, area.height)
        .into_iter()
        .map(Line::from)
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(n: usize) -> PendingRowView {
        PendingRowView {
            action: "pick".into(),
            sha7: format!("abc{n:04}"),
            subject: format!("change {n}"),
        }
    }

    fn view(picks: usize, more: usize, followers: usize) -> PendingView {
        PendingView {
            rows: (0..picks).map(pick).collect(),
            more,
            followers: (0..followers)
                .map(|n| (format!("feature-{n}"), "Sara".into(), 2))
                .collect(),
        }
    }

    #[test]
    fn still_to_apply_is_newest_first_with_followers() {
        let rows = still_to_apply_lines(&view(2, 0, 1), 60, 20);
        assert!(rows[0].contains(TITLE), "{rows:?}");
        assert!(rows[1].contains("pick abc0000 change 0"));
        assert!(rows[2].contains("pick abc0001 change 1"));
        assert!(rows[3].contains("feature-0 (Sara): 2 commits follow, on top"));
        assert!(rows[4].starts_with('╰'));
        assert_eq!(rows.len(), 5);
        assert_eq!(still_to_apply_height(&view(2, 0, 1)), 5);
    }

    #[test]
    fn still_to_apply_caps_at_six_then_more() {
        // The caller caps at six and counts the rest.
        let rows = still_to_apply_lines(&view(MAX_ROWS, 3, 0), 60, 20);
        assert_eq!(rows.len(), MAX_ROWS + 3);
        assert!(rows[MAX_ROWS + 1].contains("+3 more"), "{rows:?}");
        // A short rect cuts further: 2 content rows → 1 pick + "+N more".
        let rows = still_to_apply_lines(&view(MAX_ROWS, 3, 1), 60, 4);
        assert_eq!(rows.len(), 4);
        assert!(rows[1].contains("change 0"));
        assert!(rows[2].contains("+9 more"), "{rows:?}");
    }

    #[test]
    fn no_block_when_nothing_is_pending() {
        let empty = PendingView::default();
        assert!(still_to_apply_lines(&empty, 80, 10).is_empty());
        assert_eq!(still_to_apply_height(&empty), 0);
    }
}
