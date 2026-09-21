//! Pane boundaries, asserted as a class.
//!
//! Every other layout check in this tree is hand-keyed: a label, in a widget,
//! at one or two fixed sizes. That technique cannot express the fault this
//! file was written for. At 160 columns the graph's short-SHA column and the
//! changes pane's first column were flush against one another, so `a82960b`
//! and `CONFLICTS (2)` read as the single token `a82960bCONFLICTS` — and
//! there is no extra text on a known row to go looking for. Both panes were
//! drawing exactly what they should, one column too close together. The
//! *boundary* is what is wrong, so the boundary is what is asserted here: the
//! `Rect`s the layout functions hand out, swept across a range of widths and
//! heights rather than sampled at two. A property swept over a range cannot
//! be satisfied by agreeing with one arrangement.
//!
//! The properties, at every size in the sweep:
//!
//! - no two panes drawn at the same time overlap;
//! - panes that share any row leave **at least one column** between them, so
//!   the last cell of one and the first cell of the next never abut;
//! - the header's row, the banner's rows, the graph's body rows, the
//!   commit-graph row and the footer row are all disjoint, whatever the
//!   banner's height and whether or not the offer is on screen;
//! - the Ghost Preview box docks inside the frame it is given and never over
//!   the footer row.
//!
//! Empty rects are skipped throughout: a pane with no width or no height
//! draws nothing, so it can neither overlap its neighbour nor crowd it.

mod support;

use graft_core::capabilities::ColorDepth;
use graft_fixtures::Shape;
use graft_term::sanitize_line;
use graft_ui::detail::DetailFocus;
use graft_ui::detail_pane::{DetailLayout, detail_areas, split_area};
use graft_ui::graph_screen::{CommitGraphLine, GraphScreen};
use graft_ui::preview_box::{PREVIEW_MIN, PreviewBox, preview_box_area};
use graft_ui::theme::ThemeKind;
use ratatui::layout::Rect;

use support::corpus::lay_out;
use support::{screen, theme};

/// The widths the sweep covers: narrower than any breakpoint, through both
/// of them (100 and 160), and well past the wide one. Every width, not a
/// sample — the 160-column gutter fault is invisible at 120, which is exactly
/// what a two-size test misses.
fn widths() -> impl Iterator<Item = u16> {
    20..=240
}

/// Heights the sweep covers: one below the chrome's own row budget, one
/// where the banner and the offer between them leave almost no body, and the
/// two the captures are taken at.
const HEIGHTS: [u16; 5] = [3, 8, 24, 40, 48];

/// A rectangle something draws into, with a name a failure can print.
#[derive(Clone, Copy, Debug)]
struct Pane {
    name: &'static str,
    rect: Rect,
}

/// `rect` as a drawn pane, or `None` when it has no cells to draw in.
fn drawn(name: &'static str, rect: Rect) -> Option<Pane> {
    (rect.width > 0 && rect.height > 0).then_some(Pane { name, rect })
}

/// Collect the non-empty rects into a pane list.
fn panes<const N: usize>(all: [(&'static str, Rect); N]) -> Vec<Pane> {
    all.into_iter()
        .filter_map(|(name, rect)| drawn(name, rect))
        .collect()
}

/// `rect` is wholly within `frame`.
fn inside(rect: Rect, frame: Rect) -> bool {
    rect.x >= frame.x
        && rect.y >= frame.y
        && rect.right() <= frame.right()
        && rect.bottom() <= frame.bottom()
}

/// The first boundary fault among `panes`, in words, or `None` when every
/// pair is clear of every other.
///
/// Two faults are one property: panes that overlap fight over cells, and
/// panes that merely abut run their text together. Both are "this boundary
/// does not exist", so both are checked in one pass.
fn boundary_fault(panes: &[Pane]) -> Option<String> {
    for (index, first) in panes.iter().enumerate() {
        for second in &panes[index + 1..] {
            if first.rect.intersects(second.rect) {
                return Some(format!(
                    "{} {:?} overlaps {} {:?}",
                    first.name, first.rect, second.name, second.rect
                ));
            }
            let shares_rows =
                first.rect.y < second.rect.bottom() && second.rect.y < first.rect.bottom();
            if !shares_rows {
                // Stacked, not side by side: a row boundary needs no gutter,
                // text on two rows cannot run together.
                continue;
            }
            // They share rows and do not overlap, so they are disjoint in x.
            let (left, right) = if first.rect.x <= second.rect.x {
                (first, second)
            } else {
                (second, first)
            };
            if right.rect.x <= left.rect.right() {
                return Some(format!(
                    "{} ends at column {} and {} starts at column {}: no gutter between them",
                    left.name,
                    left.rect.right(),
                    right.name,
                    right.rect.x
                ));
            }
        }
    }
    None
}

#[test]
fn the_graph_and_the_detail_pane_never_abut_at_any_width() {
    for height in HEIGHTS {
        for width in widths() {
            for focus in [DetailFocus::Graph, DetailFocus::Files, DetailFocus::Diff] {
                for override_cells in [None, Some(48), Some(100), Some(width)] {
                    let layout = DetailLayout::compute(width, height, focus, override_cells);
                    let frame = Rect::new(0, 0, width, height);
                    let (graph, pane) = split_area(layout, frame);
                    let drawn = panes([
                        ("the graph", graph.unwrap_or_default()),
                        ("the detail pane", pane.unwrap_or_default()),
                    ]);
                    assert!(
                        boundary_fault(&drawn).is_none(),
                        "{width}x{height}, {layout:?}, override {override_cells:?}: {}",
                        boundary_fault(&drawn).unwrap_or_default()
                    );
                    for pane in &drawn {
                        assert!(
                            inside(pane.rect, frame),
                            "{width}x{height}, {layout:?}: {} {:?} leaves the frame",
                            pane.name,
                            pane.rect
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_detail_pane_s_own_regions_never_abut_the_graph_or_each_other() {
    for height in HEIGHTS {
        for width in widths() {
            for focus in [DetailFocus::Graph, DetailFocus::Files, DetailFocus::Diff] {
                for files_count in [0usize, 1, 3, 40] {
                    let layout = DetailLayout::compute(width, height, focus, None);
                    let frame = Rect::new(0, 0, width, height);
                    let (graph, pane) = split_area(layout, frame);
                    let regions = detail_areas(pane.unwrap_or_default(), layout, files_count);
                    let drawn = panes([
                        ("the graph", graph.unwrap_or_default()),
                        ("the pane header", regions.header),
                        ("the file list", regions.files),
                        ("the diff body", regions.body),
                    ]);
                    assert!(
                        boundary_fault(&drawn).is_none(),
                        "{width}x{height}, {layout:?}, {files_count} files: {}",
                        boundary_fault(&drawn).unwrap_or_default()
                    );
                }
            }
        }
    }
}

#[test]
fn the_banner_never_takes_a_row_another_part_of_the_screen_draws() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    for height in HEIGHTS {
        for banner_lines in 0..=3u16 {
            for offer in [None, Some(CommitGraphLine::Offered)] {
                let mut view = screen::view(&lines, usize::from(height), 0);
                view.banner = (0..banner_lines)
                    .map(|index| sanitize_line(&format!("banner line {index}")))
                    .collect();
                view.commit_graph = offer;
                let screen = GraphScreen::new(&view, &theme);
                for width in widths() {
                    let where_ = format!(
                        "{width}x{height}, {banner_lines} banner lines, offer {}",
                        offer.is_some()
                    );
                    check_carve(
                        screen,
                        Rect::new(0, 0, width, height),
                        banner_lines,
                        &where_,
                    );
                }
            }
        }
    }
}

/// The carve at one size: the five parts are disjoint, inside the screen, in
/// reading order, and in the places the design fixes them to.
fn check_carve(screen: GraphScreen<'_>, area: Rect, banner_lines: u16, where_: &str) {
    let rows = screen.screen_rows(area);
    let drawn = panes([
        ("the header", rows.header),
        ("the banner", rows.banner),
        ("the graph body", rows.body),
        ("the commit-graph offer", rows.offer),
        ("the hint bar", rows.footer),
    ]);
    assert!(
        boundary_fault(&drawn).is_none(),
        "{where_}: {}",
        boundary_fault(&drawn).unwrap_or_default()
    );
    for pane in &drawn {
        assert!(
            inside(pane.rect, area),
            "{where_}: {} {:?} leaves the screen",
            pane.name,
            pane.rect
        );
    }
    // Where they sit, not merely that they are apart: the header has the
    // first row, the hint bar the last, and the rest are between them in
    // reading order.
    assert_eq!(
        (rows.header.y, rows.header.height),
        (area.y, 1.min(area.height)),
        "{where_}: the header is not the screen's first row"
    );
    assert_eq!(
        (rows.footer.height, rows.footer.bottom()),
        (u16::from(area.height > 1), area.bottom()),
        "{where_}: the hint bar is not the last row of a screen with two"
    );
    let order: Vec<u16> = drawn.iter().map(|pane| pane.rect.y).collect();
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "{where_}: the screen is not in reading order: {order:?}"
    );
    // The banner outranks the commit-graph offer: a modal line never gives
    // up its row to a suggestion.
    assert!(
        rows.offer.height == 0 || rows.banner.height == banner_lines,
        "{where_}: the offer took a row the banner wanted"
    );
    // The body the fold and the pixel tier are handed is this same carve's.
    let body = screen.graph_area(area);
    assert_eq!(
        (body.y, body.height),
        (rows.body.y, rows.body.height),
        "{where_}: the graph column's rows and the carve's disagree"
    );
}

#[test]
fn the_preview_box_docks_inside_the_frame_and_above_the_footer() {
    let mut view = PreviewBox::new(sanitize_line("Rebase feature onto main"));
    view.lines = (0..6)
        .map(|index| sanitize_line(&format!("pick {index} a commit")))
        .collect();
    view.commands = vec![sanitize_line("git rebase --onto main")];

    for height in HEIGHTS {
        for width in widths() {
            let frame = Rect::new(0, 0, width, height);
            let box_area = preview_box_area(frame, &view);
            assert!(
                inside(box_area, frame),
                "{width}x{height}: the preview box {box_area:?} leaves the frame"
            );
            let footer = Rect {
                y: frame.bottom().saturating_sub(1),
                height: u16::from(height > 1),
                ..frame
            };
            let drawn = panes([("the preview box", box_area), ("the screen footer", footer)]);
            assert!(
                boundary_fault(&drawn).is_none(),
                "{width}x{height}: {}",
                boundary_fault(&drawn).unwrap_or_default()
            );
            if width >= PREVIEW_MIN.0 && height >= PREVIEW_MIN.1 {
                assert!(
                    box_area.height > 1,
                    "{width}x{height}: a frame at or over {PREVIEW_MIN:?} docks a real box, \
                     not the too-small line"
                );
            }
        }
    }
}
