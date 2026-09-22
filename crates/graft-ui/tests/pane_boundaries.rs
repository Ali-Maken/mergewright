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
use graft_ui::confirm::{ConfirmView, confirm_area, confirm_area_at};
use graft_ui::detail::{
    ContainedIn, DetailFocus, DetailHeader, DetailView, FileListView, FileRow, FileRowKind,
    FileStatusView, Subject, WorkingTreeSection,
};
use graft_ui::detail_pane::{DetailLayout, DetailPane, detail_areas, split_area};
use graft_ui::graph_screen::{
    BannerView, CommitGraphLine, GraphScreen, banner_line, snap_to_columns,
};
use graft_ui::preview_box::{PREVIEW_MIN, PreviewBox, preview_box_area};
use graft_ui::theme::ThemeKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
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

/// The text on one row of a drawn terminal, cell by cell, trailing blanks
/// trimmed.
fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    let text: String = (buffer.area.x..buffer.area.right())
        .map(|x| buffer[(x, y)].symbol().to_owned())
        .collect();
    text.trim_end().to_owned()
}

/// The first and last column something other than a space is drawn on,
/// for one row of a drawn terminal.
fn inked_span(terminal: &Terminal<TestBackend>, y: u16) -> Option<(u16, u16)> {
    let buffer = terminal.backend().buffer();
    let inked: Vec<u16> = (buffer.area.x..buffer.area.right())
        .filter(|&x| buffer[(x, y)].symbol() != " ")
        .collect();
    Some((*inked.first()?, *inked.last()?))
}

/// The operation banner shares the header row with the repository path,
/// and its room comes out **before** the path's (m5/16, review C1): at every
/// width that can hold the banner's own words, the state is on the row in
/// full — never `MERG…` — focused or not, with or without a step count, and
/// the conflicted file it carries for the hint bar is never drawn
/// (m5-history-and-ghost-preview/11, carried item 10).
///
/// **Mutation that breaks it:** in `graph_screen::layout`, hand the banner
/// the width left *after* the location instead of reserving its own — at
/// 80 columns the scratch path wins and the state is cut.
#[test]
fn the_banner_keeps_its_state_word_at_every_width() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);
    let conflict = "src/auth/token.rs";
    // What the header spends before anything else: the app's own name. Read
    // off a wide render with no banner rather than hard-coded, so the sweep
    // follows the header if that prefix ever changes.
    let prefix = {
        let view = screen::view(&lines, 24, 0);
        let mut terminal = Terminal::new(TestBackend::new(240, 24)).expect("test backend opens");
        terminal
            .draw(|frame| frame.render_widget(GraphScreen::new(&view, &theme), frame.area()))
            .expect("draws");
        let header = row_text(&terminal, 0);
        header
            .find("~/code/api")
            .unwrap_or_else(|| panic!("no location on the header row: {header:?}"))
    };

    for height in HEIGHTS {
        for focused in [false, true] {
            for step in [None, Some((3, 7))] {
                let mut view = screen::view(&lines, usize::from(height), 0);
                view.banner_op = Some(BannerView {
                    kind: sanitize_line("REBASING"),
                    step,
                    conflict: Some(sanitize_line(conflict)),
                    focused,
                    skippable: true,
                });
                let want = banner_line(view.banner_op.as_ref().expect("set above"));
                // The banner is drawn behind two spaces of its own, after
                // the header's prefix.
                let needs = prefix + 2 + want.chars().count();
                for width in widths() {
                    let mut terminal =
                        Terminal::new(TestBackend::new(width, height)).expect("test backend opens");
                    terminal
                        .draw(|frame| {
                            frame.render_widget(GraphScreen::new(&view, &theme), frame.area());
                        })
                        .expect("draws");
                    let header = row_text(&terminal, 0);
                    let where_ = format!("{width}x{height}, focused {focused}, step {step:?}");
                    if usize::from(width) >= needs {
                        assert!(
                            header.contains(&want),
                            "{where_}: the header row lost the banner: {header:?}"
                        );
                    }
                    // Narrower than that, the banner may be cut — but never
                    // in favour of the path: a header with no whole banner
                    // has no whole path either.
                    if !header.contains(&want) {
                        assert!(
                            !header.contains("~/code/api"),
                            "{where_}: the state was cut for the path: {header:?}"
                        );
                    }
                    assert!(
                        !header.contains("token.rs"),
                        "{where_}: the banner names the file: {header:?}"
                    );
                }
            }
        }
    }
}

/// The rows of a changes pane with one conflict: `CONFLICTS (1)` first,
/// its file row under it, then a staged file.
fn conflicted_rows() -> Vec<FileRow> {
    vec![
        FileRow {
            depth: 0,
            name: sanitize_line(WorkingTreeSection::Unmerged.title()),
            kind: FileRowKind::Section {
                section: WorkingTreeSection::Unmerged,
                count: 1,
            },
            entry: None,
        },
        FileRow {
            depth: 1,
            name: sanitize_line("src/auth/token.rs"),
            kind: FileRowKind::File {
                status: FileStatusView::Unmerged {
                    how: sanitize_line("both modified"),
                },
                added: None,
                removed: None,
            },
            entry: Some(0),
        },
        FileRow {
            depth: 0,
            name: sanitize_line("Staged"),
            kind: FileRowKind::Section {
                section: WorkingTreeSection::Staged,
                count: 1,
            },
            entry: None,
        },
        FileRow {
            depth: 1,
            name: sanitize_line("src/auth/session.rs"),
            kind: FileRowKind::File {
                status: FileStatusView::Added,
                added: Some(3),
                removed: Some(0),
            },
            entry: Some(1),
        },
    ]
}

/// A changes pane on the working tree, listing [`conflicted_rows`].
fn conflicted_view() -> DetailView {
    let rows = conflicted_rows();
    DetailView {
        header: DetailHeader {
            subject: Subject::WorkingTree {
                staged: 1,
                unstaged: 0,
                untracked: 0,
            },
            summary: sanitize_line("Uncommitted changes"),
            author: sanitize_line(""),
            author_date: sanitize_line(""),
            relative: sanitize_line(""),
            committer: None,
            co_authors: vec![],
            parents: vec![],
            against: Some(sanitize_line("HEAD a1b2c3d")),
            contained: ContainedIn::NotAsked,
            radar: vec![],
        },
        files: Some(std::sync::Arc::new(FileListView {
            rows,
            cursor: 0,
            top: 0,
            files: 2,
            added: 3,
            removed: 0,
            message: vec![],
        })),
        body: graft_ui::detail::BodyView::Empty(sanitize_line("")),
        body_top: 0,
        body_cursor: None,
        focus: DetailFocus::Files,
        mode: graft_ui::detail::DiffMode::Unified,
        options: graft_ui::detail::DiffOptionsView::default(),
        explain: None,
    }
}

/// The `CONFLICTS` section is drawn inside the files region at every width
/// and height the pane can take: its heading row and its file row put ink
/// only between the region's left and right edges, the heading reads
/// `CONFLICTS (1)` in full wherever the region is wide enough to hold it,
/// and the file row's `both modified` never spills past the edge
/// (m5-history-and-ghost-preview/11, carried item 10).
#[test]
fn the_conflicts_section_stays_inside_the_files_region_at_every_width() {
    let view = conflicted_view();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);
    let rows = view.files.as_ref().map_or(0, |files| files.rows.len());
    let mut checked = 0;

    for height in HEIGHTS {
        for width in widths() {
            let frame = Rect::new(0, 0, width, height);
            let layout = DetailLayout::compute(width, height, DetailFocus::Files, None);
            let Some(pane) = split_area(layout, frame).1 else {
                continue;
            };
            let areas = detail_areas(pane, layout, rows);
            if areas.files.height == 0 || areas.files.width == 0 {
                continue;
            }
            let mut terminal =
                Terminal::new(TestBackend::new(width, height)).expect("test backend opens");
            terminal
                .draw(|frame| {
                    frame.render_widget(
                        DetailPane {
                            view: &view,
                            layout: &layout,
                            theme: &theme,
                        },
                        pane,
                    );
                })
                .expect("draws");
            let where_ = format!("{width}x{height}, files {:?}", areas.files);
            let heading_y = (areas.files.y..areas.files.bottom())
                .find(|&y| row_text(&terminal, y).contains("CONFLICTS"))
                .unwrap_or_else(|| panic!("{where_}: no CONFLICTS row in the files region"));
            for y in [heading_y, heading_y + 1] {
                if y >= areas.files.bottom() {
                    continue;
                }
                let Some((first, last)) = inked_span(&terminal, y) else {
                    continue;
                };
                assert!(
                    first >= areas.files.x && last < areas.files.right(),
                    "{where_}: row {y} is inked at {first}..={last}, outside the files region"
                );
            }
            let heading = row_text(&terminal, heading_y);
            if usize::from(areas.files.width) > "CONFLICTS (1)".len() {
                assert!(
                    heading.contains("CONFLICTS (1)"),
                    "{where_}: the heading is cut: {heading:?}"
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked > 100,
        "the sweep drew {checked} panes — too few to mean anything"
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

/// A box drawn over the graph covers whole row columns or it does not stand
/// on them at all (ADR-0040) — swept over the same widths and heights as
/// everything else in this file.
///
/// The fault this pins is geometric, not textual. At 80 columns the
/// credential prompt's right border landed inside the short-SHA column and
/// left ` b8c5`, `68ff`, `00bd` beside it: four-character hex tokens that
/// read as valid short identifiers and are not. The confirm box beside it
/// survived only because "Push main → origin/main" happens to be the length
/// it is. Same surface class, opposite outcome, decided by a string — so the
/// rule is asserted against the boundaries the rows are drawn to, at every
/// width, rather than against either box's wording.
///
/// **Mutation that breaks it:** drop the `snap_to_columns` call in
/// `graft::view::frame::draw_confirm` and draw `natural` — at 80 the box's
/// left edge sits at column 8, mid-subject, and this fails naming it.
#[test]
fn no_box_drawn_over_the_graph_splits_a_row_column() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    let mut confirm = ConfirmView::new(
        sanitize_line("Push main → origin/main"),
        vec![sanitize_line(
            "1 commit · can't be undone from here — origin keeps what you send",
        )],
        sanitize_line("git push --progress origin refs/heads/main:refs/heads/main"),
    );
    confirm = confirm.with_labels("Cancel", "Push");

    for height in HEIGHTS {
        for width in widths() {
            let frame = Rect::new(0, 0, width, height);
            let view = screen::view(&lines, usize::from(height), 0);
            let boundaries = GraphScreen::new(&view, &theme).column_boundaries(frame);

            let natural = confirm_area(frame, &confirm).intersection(frame);
            if natural.is_empty() {
                continue;
            }
            let Some(snapped) = snap_to_columns(natural, &boundaries, frame) else {
                // No snap position fits: `draw_confirm` clears the band
                // instead, so no column survives beside the box to be split.
                continue;
            };
            let rect =
                confirm_area_at(frame, &confirm, snapped.x, snapped.width).intersection(frame);
            if rect.is_empty() {
                continue;
            }
            assert!(
                boundaries.contains(&rect.x),
                "{width}x{height}: the confirm box starts at column {} and the row columns \
                 begin at {boundaries:?} — it splits the one it starts inside",
                rect.x
            );
            // The right edge rests on the **gutter** before a column's
            // text, not on the text itself (ADR-0040's 2026-09-21
            // amendment), so the position to look for is one cell short of
            // a boundary — except at the frame's own edge, where nothing is
            // beside the box and no gutter is owed.
            assert!(
                rect.right() == frame.right()
                    || boundaries.contains(&(rect.right().saturating_add(1))),
                "{width}x{height}: the confirm box ends at column {} and the row columns \
                 begin at {boundaries:?} — it splits the one it ends inside, or stands \
                 flush against it",
                rect.right()
            );
        }
    }
}

/// **And it leaves a gutter.** A non-clearing box leaves at least one blank
/// column between its border and any content beside it (ADR-0040, amended
/// 2026-09-21 by m5-history-and-ghost-preview/29). Covering whole columns
/// is not enough on its own: at 80 columns the confirm box's right border
/// landed on the first cell of the short-SHA column and the row read
/// `╮60af0eb` — one token, the vertical form of the `8443─────` fault the
/// same ticket fixed on the horizontal edge.
///
/// Asserted against the **drawn** rows rather than against the geometry:
/// the question is whether a character sits next to the border, and only
/// the buffer can answer it. `draw_confirm` tints `rect` and nothing else
/// when the box snaps, so every cell outside `rect` still holds the graph.
///
/// **Mutation that reproduces `╮60af0eb`:** drop the `- 1` gutter step in
/// `snap_to_columns` (snap the right edge to the boundary itself) and this
/// fails at 80x24 naming the character it touched.
#[test]
fn no_box_drawn_over_the_graph_touches_the_content_beside_it() {
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget as _;

    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    let mut confirm = ConfirmView::new(
        sanitize_line("Push main → origin/main"),
        vec![sanitize_line(
            "1 commit · can't be undone from here — origin keeps what you send",
        )],
        sanitize_line("git push --progress origin refs/heads/main:refs/heads/main"),
    );
    confirm = confirm.with_labels("Cancel", "Push");

    for height in HEIGHTS {
        for width in widths() {
            let frame = Rect::new(0, 0, width, height);
            let view = screen::view(&lines, usize::from(height), 0);
            let screen = GraphScreen::new(&view, &theme);
            let boundaries = screen.column_boundaries(frame);

            let natural = confirm_area(frame, &confirm).intersection(frame);
            if natural.is_empty() {
                continue;
            }
            // No snap position fits: `draw_confirm` clears the band instead,
            // so nothing survives beside the box to be touched.
            let Some(snapped) = snap_to_columns(natural, &boundaries, frame) else {
                continue;
            };
            let rect =
                confirm_area_at(frame, &confirm, snapped.x, snapped.width).intersection(frame);
            if rect.is_empty() {
                continue;
            }

            let mut drawn = Buffer::empty(frame);
            let body = screen.screen_rows(frame).body;
            screen.render(frame, &mut drawn);
            // The rows the column geometry describes. The header and the
            // footer are single lines with no columns in them, so a box
            // standing on one is not what this rule is about.
            for y in rect.top().max(body.top())..rect.bottom().min(body.bottom()) {
                if rect.x > frame.x {
                    let beside = drawn[(rect.x - 1, y)].symbol().to_owned();
                    assert!(
                        beside.trim().is_empty(),
                        "{width}x{height}: the box starts at column {} and {beside:?} sits \
                         against its left border on row {y}",
                        rect.x
                    );
                }
                if rect.right() < frame.right() {
                    let beside = drawn[(rect.right(), y)].symbol().to_owned();
                    assert!(
                        beside.trim().is_empty(),
                        "{width}x{height}: the box ends at column {} and {beside:?} sits \
                         against its right border on row {y}",
                        rect.right()
                    );
                }
            }
        }
    }
}

/// Where the gutter cannot be honoured the box **clears** — it never
/// shrinks its content to reach a position (ADR-0040 clause 2, unchanged by
/// the 2026-09-21 amendment).
///
/// Two halves. The first is the rule itself, on numbers: with the frame's
/// own right edge out of the boundary set there is no gutter-legal position
/// beyond the last column, and [`snap_to_columns`] answers `None` rather
/// than handing back a rect narrower than the one it was given.
///
/// The second is the measurement the amendment owes: **which width first
/// forces the confirm box to clear.** Ticket 28 recorded that no width in
/// 20..=240 reached that branch; the gutter costs a cell, so it could have.
/// This sweeps the same range and prints the first width that clears, or
/// says none does — the answer goes in the ticket's report either way.
#[test]
fn where_the_gutter_cannot_be_honoured_the_box_clears() {
    let bounds = Rect::new(0, 0, 80, 24);
    // A single column start at 40 and no frame edge in the set: a rect
    // reaching past 39 has nowhere left to go that leaves a gutter.
    let past_the_last_column = Rect::new(4, 0, 40, 6);
    assert!(
        snap_to_columns(past_the_last_column, &[0, 40], bounds).is_none(),
        "no gutter-legal position past the last column: the caller clears"
    );
    // One cell narrower and the gutter at 39 covers it, so it snaps.
    let inside = Rect::new(4, 0, 35, 6);
    assert_eq!(
        snap_to_columns(inside, &[0, 40], bounds).map(|rect| (rect.x, rect.right())),
        Some((0, 39)),
        "the right edge rests on the gutter before column 40, not on 40"
    );

    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);
    let mut confirm = ConfirmView::new(
        sanitize_line("Push main → origin/main"),
        vec![sanitize_line(
            "1 commit · can't be undone from here — origin keeps what you send",
        )],
        sanitize_line("git push --progress origin refs/heads/main:refs/heads/main"),
    );
    confirm = confirm.with_labels("Cancel", "Push");

    let mut clears: Vec<(u16, u16)> = Vec::new();
    for height in HEIGHTS {
        for width in widths() {
            let frame = Rect::new(0, 0, width, height);
            let view = screen::view(&lines, usize::from(height), 0);
            let boundaries = GraphScreen::new(&view, &theme).column_boundaries(frame);
            let natural = confirm_area(frame, &confirm).intersection(frame);
            if natural.is_empty() {
                continue;
            }
            if snap_to_columns(natural, &boundaries, frame).is_none() {
                clears.push((width, height));
            }
        }
    }
    // Not an assertion that it never clears — clearing is correct where it
    // happens. The measurement is the point, so it is printed and the shape
    // of the answer is pinned: the frame's own edges are in the boundary
    // set, so an outward position always exists there.
    assert!(
        clears.is_empty(),
        "the confirm box clears at {clears:?} — the amendment changed the \
         measurement ticket 28 recorded, and the ticket's report must say so"
    );
}

/// Every boundary the snap can land on is one the rows are actually drawn
/// to: the graph column's end, the marker slot's, the subject's, the gap's,
/// the author chip's, the date's and the SHA's, plus the frame's own edges.
///
/// Without this, `no_box_drawn_over_the_graph_splits_a_row_column` could be
/// satisfied by a boundary list that named every column — the box would land
/// on a "boundary" that is the middle of the subject and the assertion would
/// still hold. This pins the list itself against the two ends it must have
/// and against being dense.
#[test]
fn the_row_columns_are_the_ones_the_rows_are_drawn_to() {
    let dir = graft_fixtures::borrow(Shape::FeatureBranches);
    let laid = lay_out(Shape::FeatureBranches, &dir);
    let lines = laid.lines();
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    for width in widths() {
        let frame = Rect::new(0, 0, width, 24);
        let view = screen::view(&lines, 24, 0);
        let boundaries = GraphScreen::new(&view, &theme).column_boundaries(frame);

        assert_eq!(
            boundaries.first().copied(),
            Some(frame.x),
            "{width}: the frame's left edge is a boundary"
        );
        assert_eq!(
            boundaries.last().copied(),
            Some(frame.right()),
            "{width}: the frame's right edge is a boundary"
        );
        assert!(
            boundaries.windows(2).all(|pair| pair[0] < pair[1]),
            "{width}: boundaries must be ascending and deduped: {boundaries:?}"
        );
        // Seven columns at most (graph, marker, subject, gap, author, date,
        // sha) means eight boundaries at most. A denser list would let the
        // snap stop in the middle of a column.
        assert!(
            boundaries.len() <= 8,
            "{width}: {} boundaries is more than the columns there are: {boundaries:?}",
            boundaries.len()
        );
    }
}
