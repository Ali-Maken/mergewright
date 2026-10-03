//! The Map view (`M`, DESIGN.md §5.1): one line per local branch, plotted as
//! a density sparkline against a shared time axis (`map-view-screen`,
//! m2-details-and-diffs/19).
//!
//! Draws whatever [`crate::graft::view::map::map_view`] (a later crate)
//! builds from [`crate::graft::map::MapData`] — this module owns no I/O and
//! no state, only the widget.
//!
//! Text glyphs at every tier in M2 (design note, Proposed changes): the
//! pixel tier's real density curve is out of scope here (M8 polish gap).
//! Mouse is also out of scope (M2 gap row) — every interaction here is a
//! key, `update::map::map_key`'s.
//!
//! **Undocumented choices, not fully pinned by the design note:**
//! - The header's day/month wording switches at 60 days (`span_words`):
//!   below it reads `last N days`, at or above `last N months` (rounded to
//!   the nearest month, at least one).
//! - The ASCII tier's tag mark (`┴` in Unicode) draws `+`, mirroring
//!   `graph_column`'s ASCII junction glyph — the design note gives no ASCII
//!   form for it.
//! - When a Radar mark and a Tag mark land in the same bucket (both name
//!   the same bucket index), the later one in `MapRowView::marks` wins the
//!   glyph; unspecified by the design note, and rare (a tag and a finding on
//!   the same commit).

use graft_core::mistake_radar::Severity;
use graft_graph::identity::BranchIdentity;
use graft_term::SanitizedText;

use crate::theme::Theme;

mod render;

/// A mark on one bucket of one row, once sanitised for drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapMarkView {
    /// A tag's short name.
    Tag(SanitizedText),
    /// The loudest Mistake Radar finding in the bucket.
    Radar(Severity),
}

/// One row: a branch's density sparkline, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRowView {
    /// The branch's name, sanitised.
    pub name: SanitizedText,
    /// Lane colour: the same identity the main graph's lanes use.
    pub identity: BranchIdentity,
    /// Commits per bucket; [`MapView::buckets`] long.
    pub counts: Vec<u32>,
    /// `HEAD` is on this branch: prefixed `▸`, suffixed ` ●`.
    pub is_current: bool,
    /// `Some((row, bucket))`: this branch merged into the row at `row`, its
    /// own line ending at `bucket` — where the elbow (`●╮`) draws.
    pub merged_into: Option<(usize, usize)>,
    /// Tag and Radar marks, by bucket.
    pub marks: Vec<(usize, MapMarkView)>,
    /// The tip is in the newest bucket and this row is not `merged_into`
    /// anything: draws ` (active)`. Never set on the current row (it draws
    /// ` ●` instead).
    pub active: bool,
    /// This row's own walk did not hit its cap; `false` colours nothing here
    /// — only [`MapView::complete`] draws the footer's ` · partial`.
    pub complete: bool,
}

/// What the Map view is doing right now, in place of its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapStatus {
    /// Still walking branches: `done` of `total` have a row so far.
    Loading {
        /// Rows built so far.
        done: u32,
        /// Rows the build expects to visit.
        total: u32,
    },
    /// No local branches at all.
    Empty,
    /// The build failed; the line to show, already sanitised.
    Failed(SanitizedText),
    /// Rows are ready to draw.
    Ready,
}

/// Everything the Map view draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapView {
    /// The branch named in the header: current if there is one, else the
    /// default branch.
    pub title_branch: SanitizedText,
    /// The shared time span's width, in days, for the header's wording.
    pub span_days: u32,
    /// Rows, in the order they draw (current first, then default, then the
    /// rest by tip date).
    pub rows: Vec<MapRowView>,
    /// The largest bucket count over every row: what a bar's glyphs scale
    /// against.
    pub max_count: u32,
    /// Columns every row's bar has.
    pub buckets: usize,
    /// The row under the cursor.
    pub selected: usize,
    /// Local branches beyond what `rows` holds; the footer's `+N more`.
    pub more: usize,
    /// `false` if any row's walk hit its cap; the footer's ` · partial`.
    pub complete: bool,
    /// What to draw instead of, or alongside, the rows.
    pub status: MapStatus,
    /// Draw bars and marks with the ASCII glyph table rather than Unicode
    /// block density (`graph.renderer`, mirroring [`GraphView::ascii`]).
    ///
    /// [`GraphView::ascii`]: crate::graph_screen::GraphView::ascii
    pub ascii: bool,
}

/// The Map view, ready to render.
#[derive(Clone, Copy, Debug)]
pub struct MapScreen<'a> {
    /// What to draw.
    pub view: &'a MapView,
    /// The theme to draw it in.
    pub theme: &'a Theme,
}

#[cfg(test)]
mod tests {
    use graft_core::capabilities::ColorDepth;
    use graft_term::sanitize_line;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::render::ramp_glyph;
    use super::*;
    use crate::test_support::terminal as test_capabilities;
    use crate::theme::ThemeKind;

    fn draw(view: &MapView, columns: u16, rows: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(columns, rows))
            .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
        let capabilities = test_capabilities(ColorDepth::TrueColor, false);
        let theme = Theme::new(ThemeKind::Dark, &capabilities);
        terminal
            .draw(|frame| {
                frame.render_widget(
                    MapScreen {
                        view,
                        theme: &theme,
                    },
                    frame.area(),
                );
            })
            .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
        terminal.backend().to_string()
    }

    fn row(name: &str, identity: u8, counts: Vec<u32>) -> MapRowView {
        MapRowView {
            name: sanitize_line(name),
            identity: BranchIdentity::of_name(&[identity]),
            counts,
            is_current: false,
            merged_into: None,
            marks: Vec::new(),
            active: false,
            complete: true,
        }
    }

    fn sample_view(ascii: bool) -> MapView {
        let mut main = row("main", 1, vec![0, 1, 2, 7, 0, 0, 3, 0, 1, 0]);
        main.is_current = true;
        let mut feature = row("feature/auth", 2, vec![0, 0, 3, 4, 0, 0, 0, 0, 0, 0]);
        feature.merged_into = Some((0, 3));
        let mut release = row("release/2.3", 3, vec![0, 0, 0, 0, 0, 0, 0, 1, 2, 0]);
        release.marks = vec![
            (7, MapMarkView::Tag(sanitize_line("v1.0"))),
            (8, MapMarkView::Radar(Severity::Warning)),
        ];
        MapView {
            title_branch: sanitize_line("main"),
            span_days: 40,
            rows: vec![main, feature, release],
            max_count: 7,
            buckets: 10,
            selected: 0,
            more: 0,
            complete: true,
            status: MapStatus::Ready,
            ascii,
        }
    }

    #[test]
    fn ramp_glyph_zero_is_blank_never_for_a_nonzero_count() {
        assert_eq!(ramp_glyph(0, 1000, false), ' ');
        assert_eq!(ramp_glyph(0, 1000, true), ' ');
        assert_eq!(ramp_glyph(1000, 1000, false), '█');
        assert_eq!(ramp_glyph(1000, 1000, true), '@');
        assert_eq!(ramp_glyph(1, 1000, false), '▁');
        assert_eq!(ramp_glyph(1, 1000, true), '.');
        for count in 1..=1000u32 {
            assert_ne!(ramp_glyph(count, 1000, false), ' ');
            assert_ne!(ramp_glyph(count, 1000, true), ' ');
        }
    }

    /// ADR-0058 §3: on the selected row's `selection` fill, the name, the
    /// branch-coloured bar, the tag and the radar mark clear 4.5:1 in dark B.
    #[test]
    fn text_on_the_cursor_row_is_legible() {
        let theme = Theme::new(
            ThemeKind::Dark,
            &test_capabilities(ColorDepth::TrueColor, false),
        );
        for selected in 0..3 {
            let view = MapView {
                selected,
                ..sample_view(false)
            };
            let mut terminal = Terminal::new(TestBackend::new(120, 24)).expect("test backend");
            terminal
                .draw(|frame| {
                    frame.render_widget(
                        MapScreen {
                            view: &view,
                            theme: &theme,
                        },
                        frame.area(),
                    );
                })
                .expect("draws");
            let checked = crate::test_support::assert_legible_on_selection(
                terminal.backend().buffer(),
                &theme,
            );
            assert!(checked > 5, "row {selected}: drawn ({checked})");
        }
    }

    #[test]
    fn ready_snapshot_80x24_unicode() {
        insta::assert_snapshot!(draw(&sample_view(false), 80, 24));
    }

    #[test]
    fn ready_snapshot_80x24_ascii() {
        insta::assert_snapshot!(draw(&sample_view(true), 80, 24));
    }

    #[test]
    fn ready_snapshot_120x40_unicode() {
        insta::assert_snapshot!(draw(&sample_view(false), 120, 40));
    }

    #[test]
    fn loading_snapshot() {
        let mut view = sample_view(false);
        view.status = MapStatus::Loading { done: 2, total: 5 };
        insta::assert_snapshot!(draw(&view, 80, 24));
    }

    #[test]
    fn empty_snapshot() {
        let mut view = sample_view(false);
        view.rows = Vec::new();
        view.status = MapStatus::Empty;
        insta::assert_snapshot!(draw(&view, 80, 24));
    }

    #[test]
    fn failed_snapshot() {
        let mut view = sample_view(false);
        view.status = MapStatus::Failed(sanitize_line("git for-each-ref: not a repository"));
        insta::assert_snapshot!(draw(&view, 80, 24));
    }
}
