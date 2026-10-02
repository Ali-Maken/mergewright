//! The header row every screen with a repository open draws (DESIGN.md §1.0,
//! ADR-0058 §4): ` ◆ <repo>  <~/path> · <branch>` on the left, the
//! informational tail right-aligned, one cell of right margin.
//!
//! What gives first when the row is short: the path (cut from the **left**,
//! so the end of it — the folder he knows — survives), then the branch, then
//! the tail, and the repository's name last of all.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The product's name, said where the app names itself (the empty state, the
/// status screen) and never in a repository's header. One line to change
/// when the real name is chosen (G-rename).
pub const APP_NAME: &str = "graft";

/// Everything the header row says, as plain text the caller has already
/// sanitised.
#[derive(Clone, Debug)]
pub struct HeaderView<'a> {
    /// The repository's name: the last component of its path.
    pub repo: &'a str,
    /// Its path, `~`-abbreviated by the caller (or the practice repo's
    /// words, ADR-0056).
    pub path: &'a str,
    /// The branch `HEAD` is on, or `detached <sha7>`.
    pub branch: &'a str,
    /// The informational tail, right-aligned: walk state, filters, `gfx`,
    /// `+N lanes` — or the operation banner, which replaces it. Already
    /// fitted by the caller to [`tail_room`]; cut again here if it is not.
    pub tail: Option<Line<'static>>,
    /// The practice repository (G-tour): the glyph is `◇`, not `◆`.
    pub practice: bool,
}

/// The glyph before the repository's name.
fn glyph(practice: bool) -> &'static str {
    if practice { "\u{25c7}" } else { "\u{25c6}" }
}

/// Cells between the left part and the tail.
const TAIL_GAP: usize = 2;
/// The cell of right margin every row keeps (ADR-0058 §4).
const RIGHT_MARGIN: usize = 1;

/// ` ◆ <repo>`: the part that is cut last. Before a repository is open there
/// is no name to mark, and the row is the one cell of indent.
fn head(view: &HeaderView<'_>) -> String {
    if view.repo.is_empty() {
        return " ".to_owned();
    }
    format!(" {} {}", glyph(view.practice), view.repo)
}

/// Cells the tail may take at `width`: what the repository's name and the
/// margins leave. The caller fits its tail to this (a banner ladders down
/// to it), so the tail never pushes the name off the row.
#[must_use]
pub fn tail_room(view: &HeaderView<'_>, width: u16) -> u16 {
    let room = usize::from(width)
        .saturating_sub(RIGHT_MARGIN)
        .saturating_sub(head(view).width())
        .saturating_sub(TAIL_GAP);
    u16::try_from(room).unwrap_or(u16::MAX)
}

/// The header row at `width` cells.
#[must_use]
pub fn header_line(view: &HeaderView<'_>, width: u16, theme: &Theme) -> Line<'static> {
    let width = usize::from(width);
    let usable = width.saturating_sub(RIGHT_MARGIN);
    let accent = foreground(theme, Token::Accent).add_modifier(Modifier::BOLD);
    let muted = foreground(theme, Token::TextMuted);
    let text = foreground(theme, Token::Text);

    let head = head(view);
    let head_width = head.width().min(usable);
    let mut spans = vec![Span::styled(fit(&head, head_width), accent)];

    let tail_width = view.tail.as_ref().map_or(0, Line::width).min(
        usable
            .saturating_sub(head_width)
            .saturating_sub(TAIL_GAP),
    );
    let gap = if tail_width > 0 { TAIL_GAP } else { 0 };
    let left_room = usable
        .saturating_sub(head_width)
        .saturating_sub(tail_width + gap);

    let location = location_spans(view, left_room, muted, text);
    let left_width: usize = head_width + location.iter().map(Span::width).sum::<usize>();
    spans.extend(location);

    if let Some(tail) = &view.tail
        && tail_width > 0
    {
        let pad = usable.saturating_sub(left_width + tail_width);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.extend(fit_spans(tail, tail_width));
    }
    Line::from(spans)
}

/// `  <path> · <branch>` in at most `room` cells: whole if it fits; else
/// the path cut from the left, keeping the branch; else ` · <branch>`
/// alone; else nothing.
fn location_spans(
    view: &HeaderView<'_>,
    room: usize,
    muted: Style,
    text: Style,
) -> Vec<Span<'static>> {
    let branch = (!view.branch.is_empty()).then(|| {
        [
            Span::styled(" \u{b7} ", muted),
            Span::styled(view.branch.to_owned(), text),
        ]
    });
    let branch_width = branch
        .as_ref()
        .map_or(0, |spans| spans.iter().map(Span::width).sum());
    // `  ` before the path, and at least `…` plus one cell of it.
    let path_room = room.saturating_sub(branch_width).saturating_sub(2);
    let mut spans = Vec::new();
    if !view.path.is_empty() && path_room >= 2 {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(cut_left(view.path, path_room), muted));
    }
    if let Some(branch) = branch
        && branch_width <= room.saturating_sub(spans.iter().map(Span::width).sum())
    {
        spans.extend(branch);
    }
    spans
}

/// `text` in at most `width` cells, cut from the **left** with `…`: the end
/// of a path is the part that says which folder it is.
fn cut_left(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let mut kept: Vec<char> = Vec::new();
    let mut used = 1;
    for character in text.chars().rev() {
        let cells = character.width().unwrap_or(0);
        if used + cells > width {
            break;
        }
        used += cells;
        kept.push(character);
    }
    let mut out = String::from("\u{2026}");
    out.extend(kept.into_iter().rev());
    out
}

/// `line`'s spans cut to `width` cells in all, styles kept.
fn fit_spans(line: &Line<'static>, width: usize) -> Vec<Span<'static>> {
    if line.width() <= width {
        return line.spans.clone();
    }
    let mut out = Vec::new();
    let mut left = width;
    for span in &line.spans {
        if left == 0 {
            break;
        }
        let cells = span.width();
        if cells < left {
            out.push(span.clone());
            left -= cells;
        } else {
            out.push(Span::styled(fit(&span.content, left), span.style));
            left = 0;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use graft_core::capabilities::ColorDepth;

    use super::*;
    use crate::test_support::terminal;
    use crate::theme::ThemeKind;

    fn theme() -> Theme {
        Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false))
    }

    fn header<'a>(path: &'a str, branch: &'a str, tail: Option<&str>) -> HeaderView<'a> {
        HeaderView {
            repo: "billing-api",
            path,
            branch,
            tail: tail.map(|words| Line::raw(words.to_owned())),
            practice: false,
        }
    }

    fn drawn(view: &HeaderView<'_>, width: u16) -> String {
        let line = header_line(view, width, &theme());
        line.spans.iter().map(|span| span.content.as_ref()).collect()
    }

    const LONG: &str = "~/code/clients/acme/payments/services/billing/billing-api-with-a-name-that-goes-on";

    #[test]
    fn the_header_names_the_repo_path_and_branch_and_never_the_product() {
        let row = drawn(&header("~/code/billing-api", "main", None), 120);
        assert!(row.starts_with(" \u{25c6} billing-api  ~/code/billing-api \u{b7} main"));
        assert!(!row.contains(&format!(" {APP_NAME} ")), "{row:?}");
        assert!(row.width() <= 119, "one cell of right margin: {row:?}");
    }

    /// Acceptance: the path is cut from the left with `…` at 60, 100 and 187
    /// columns, and the repository's name is never cut while any path is
    /// left.
    #[test]
    fn a_long_path_is_cut_from_the_left_and_the_repo_name_survives() {
        for width in [60u16, 100, 187] {
            let row = drawn(&header(LONG, "main", Some("1,204 commits")), width);
            assert!(row.width() < usize::from(width), "{width}: {row:?}");
            assert!(
                row.starts_with(" \u{25c6} billing-api  "),
                "{width}: {row:?}"
            );
            if LONG.width() + 40 > usize::from(width) {
                assert!(row.contains("  \u{2026}"), "{width}: cut from the left: {row:?}");
            }
            assert!(row.contains("goes-on \u{b7} main"), "{width}: the path's end survives: {row:?}");
            assert!(row.trim_end().ends_with("1,204 commits"), "{width}: {row:?}");
        }
        // So narrow that no path is left: the name is still whole.
        let row = drawn(&header(LONG, "main", None), 22);
        assert!(row.starts_with(" \u{25c6} billing-api"), "{row:?}");
        assert!(!row.contains('~'), "{row:?}");
    }

    #[test]
    fn a_detached_head_says_so_with_its_sha() {
        let row = drawn(&header("~/code/billing-api", "detached 9f2c1ab", None), 100);
        assert!(row.contains("~/code/billing-api \u{b7} detached 9f2c1ab"), "{row:?}");
    }

    #[test]
    fn the_tail_is_right_aligned_inside_the_margin() {
        for width in [80u16, 120, 187] {
            let row = drawn(&header("~/code/billing-api", "main", Some("reading 4,210 rows")), width);
            assert_eq!(row.width(), usize::from(width) - 1, "{width}: {row:?}");
            assert!(row.ends_with("reading 4,210 rows"), "{width}: {row:?}");
        }
    }

    #[test]
    fn the_practice_repo_draws_a_hollow_glyph() {
        let mut view = header("practice \u{b7} /tmp/x \u{b7} a throwaway repo, not yours", "main", None);
        view.practice = true;
        let row = drawn(&view, 120);
        assert!(row.starts_with(" \u{25c7} billing-api"), "{row:?}");
        assert!(!row.contains('\u{25c6}'), "{row:?}");
    }

    #[test]
    fn tail_room_leaves_the_name_and_margins() {
        let view = header("~/x", "main", None);
        // ` ◆ billing-api` is 14 cells; one margin, two gap.
        assert_eq!(tail_room(&view, 100), 100 - 14 - 1 - 2);
        assert_eq!(tail_room(&view, 10), 0);
    }
}
