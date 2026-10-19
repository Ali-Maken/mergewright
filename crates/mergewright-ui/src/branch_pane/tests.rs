//! What the branch pane's grid says (ease-pass 09, mock G12).

use mergewright_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::*;
use crate::paint::color;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(mergewright_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn known(mark: Mark, words: &str) -> PaneLine {
    PaneLine::Known {
        mark,
        words: sanitize_line(words),
    }
}

fn stop(name: &str, role: LaneRole) -> RouteStop {
    RouteStop {
        name: sanitize_line(name),
        role,
    }
}

fn sara() -> BranchPaneView {
    BranchPaneView::Branch {
        heading: sanitize_line("sara/fix-toc"),
        role: LaneRole::This,
        here: false,
        route: Some(vec![
            stop("main", LaneRole::Trunk),
            stop("wallet-release", LaneRole::Under),
            stop("sara/fix-toc", LaneRole::This),
        ]),
        lines: [
            known(Mark::Fine, "pushed"),
            known(Mark::Plain, "not merged"),
            known(
                Mark::Attention,
                "wallet-release has 1 new commit since it left",
            ),
            known(Mark::Plain, "Sara, last commit 2w ago"),
        ],
    }
}

fn buffer(view: &BranchPaneView, columns: u16, rows: u16) -> Buffer {
    let theme = theme();
    let area = Rect::new(0, 0, columns, rows);
    let mut buf = Buffer::empty(area);
    BranchPane {
        view,
        theme: &theme,
    }
    .render(area, &mut buf);
    buf
}

fn grid(buf: &Buffer) -> Vec<String> {
    let area = buf.area;
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

#[test]
fn the_pane_is_36_wide_from_120_30_from_100_and_gone_below() {
    assert_eq!(pane_width(120), Some(36));
    assert_eq!(pane_width(200), Some(36));
    assert_eq!(pane_width(100), Some(30));
    assert_eq!(pane_width(119), Some(30));
    assert_eq!(pane_width(99), None);
}

#[test]
fn a_branch_reads_heading_four_marked_lines_then_its_route_trunk_first() {
    let lines = grid(&buffer(&sara(), 37, 12));
    assert_eq!(
        lines,
        [
            "│ sara/fix-toc",
            "│ ✓ pushed",
            "│ · not merged",
            "│ ! wallet-release has 1 new commit",
            "│   since it left",
            "│ · Sara, last commit 2w ago",
            "│",
            "│ built on",
            "│   main",
            "│    ─◆ wallet-release",
            "│    ─◆ sara/fix-toc",
            "│",
        ]
    );
}

#[test]
fn heading_and_route_names_draw_in_their_roles_and_attention_in_warning() {
    let theme = theme();
    let buf = buffer(&sara(), 37, 12);
    assert_eq!(buf[(2, 0)].fg, color(theme.token(Token::RoleThis)));
    assert_eq!(buf[(4, 8)].fg, color(theme.token(Token::RoleTrunk)));
    assert_eq!(buf[(8, 9)].fg, color(theme.token(Token::RoleUnder)));
    assert_eq!(buf[(2, 3)].fg, color(theme.token(Token::Warning)));
    assert_eq!(buf[(4, 3)].fg, color(theme.token(Token::Warning)));
    assert_eq!(buf[(2, 2)].fg, color(theme.token(Token::TextMuted)));
}

#[test]
fn heads_branch_says_youre_here() {
    let BranchPaneView::Branch {
        heading,
        role,
        route,
        lines,
        ..
    } = sara()
    else {
        unreachable!("sara() is a branch");
    };
    let view = BranchPaneView::Branch {
        heading,
        role,
        here: true,
        route,
        lines,
    };
    assert_eq!(
        grid(&buffer(&view, 37, 1)),
        ["│ sara/fix-toc  (you're here)"]
    );
}

#[test]
fn before_the_read_each_line_reads_reading_and_a_failure_stays_on_its_line() {
    let view = BranchPaneView::Branch {
        heading: sanitize_line("x"),
        role: LaneRole::This,
        here: false,
        route: None,
        lines: [
            PaneLine::Reading,
            PaneLine::Failed(sanitize_line("no repo")),
            PaneLine::Reading,
            PaneLine::Reading,
        ],
    };
    assert_eq!(
        grid(&buffer(&view, 37, 6)),
        [
            "│ x",
            "│ reading…",
            "│ can't tell: no repo",
            "│ reading…",
            "│ reading…",
            "│",
        ]
    );
}

#[test]
fn a_commit_on_no_branch_says_so() {
    let view = BranchPaneView::NotOnBranch {
        sha7: sanitize_line("abc1234"),
    };
    assert_eq!(grid(&buffer(&view, 37, 1)), ["│ abc1234 · not on a branch"]);
}
