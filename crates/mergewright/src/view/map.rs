//! Building [`MapView`]: turning `state.map`'s [`MapData`] into the Map
//! screen's widget input (`map-view-screen`, m2-details-and-diffs/19).
//!
//! Pure, like every other `view` module: everything here is read off
//! [`AppState`], never off a repository handle. `ascii` is left `false` —
//! the caller (`view::frame`) sets it from the tier in force, mirroring
//! [`mergewright_ui::graph_screen::GraphView::ascii`].

use mergewright_term::sanitize_path_bytes;
use mergewright_ui::map_screen::{MapMarkView, MapRowView, MapStatus, MapView};

use crate::map::{MapBranch, MapData, MapMark};
use crate::update::AppState;
use crate::update::detail::Loading;

/// A day, in seconds — for turning [`MapData`]'s span into
/// [`MapView::span_days`].
const DAY_SECONDS: i64 = 86_400;

/// [`MapView`] for `state.map`'s current build, whatever stage it is at.
///
/// `now_seconds` is unused once the build has landed (its own span already
/// carries `to`); kept in the signature to mirror
/// [`super::rows::graph_view`]'s.
#[must_use]
pub fn map_view(state: &AppState, _now_seconds: i64) -> MapView {
    let map = &state.map;
    let mut view = match &map.data {
        Loading::Ready(data) => ready_view(data, map.selected),
        Loading::Pending(_) => placeholder_view(MapStatus::Loading {
            done: map.progress.map_or(0, |(done, _)| done),
            total: map.progress.map_or(0, |(_, total)| total),
        }),
        Loading::Idle => placeholder_view(MapStatus::Empty),
        Loading::Failed(message) => {
            placeholder_view(MapStatus::Failed(mergewright_term::sanitize_line(message)))
        }
    };
    // `Esc` stops a running write before the Map sees it: the footer says
    // so (m8/59 Major A).
    view.esc_cancels = crate::update::remote::esc_stops_write(state);
    view
}

/// A view with no rows, for every status but [`MapStatus::Ready`].
fn placeholder_view(status: MapStatus) -> MapView {
    MapView {
        title_branch: mergewright_term::sanitize_line(""),
        span_days: 0,
        rows: Vec::new(),
        max_count: 0,
        buckets: 0,
        selected: 0,
        more: 0,
        complete: true,
        status,
        ascii: false,
        esc_cancels: false,
    }
}

/// The view once a build has landed; [`MapStatus::Empty`] if it found no
/// local branches at all.
fn ready_view(data: &MapData, selected: usize) -> MapView {
    if data.branches.is_empty() {
        return placeholder_view(MapStatus::Empty);
    }
    let title_branch = data
        .branches
        .iter()
        .find(|branch| branch.is_current)
        .or_else(|| data.branches.iter().find(|branch| branch.is_default))
        .unwrap_or(&data.branches[0]);
    let span_days = ((data.to - data.from) / DAY_SECONDS).clamp(1, i64::from(u32::MAX));
    let newest_bucket = data.buckets.saturating_sub(1);

    MapView {
        title_branch: sanitize_path_bytes(title_branch.name.shorthand()),
        span_days: u32::try_from(span_days).unwrap_or(u32::MAX),
        rows: data
            .branches
            .iter()
            .map(|branch| row_view(branch, data, newest_bucket))
            .collect(),
        max_count: data.max_count(),
        buckets: data.buckets,
        selected,
        more: data.more,
        complete: data.complete,
        status: MapStatus::Ready,
        ascii: false,
        esc_cancels: false,
    }
}

/// One [`MapBranch`] as a [`MapRowView`]. `active` marks a row whose tip
/// lands in the newest bucket, is not the current branch and has not merged
/// into the default branch's row (DESIGN.md §5.1: ` (active)`).
fn row_view(branch: &MapBranch, data: &MapData, newest_bucket: usize) -> MapRowView {
    let tip_bucket = data.bucket_of(branch.tip_date);
    let active = !branch.is_current && branch.merged_into.is_none() && tip_bucket == newest_bucket;
    MapRowView {
        name: sanitize_path_bytes(branch.name.shorthand()),
        identity: branch.identity,
        counts: branch.counts.clone(),
        is_current: branch.is_current,
        merged_into: branch.merged_into,
        marks: branch
            .marks
            .iter()
            .map(|(bucket, mark)| (*bucket, mark_view(mark)))
            .collect(),
        active,
        complete: branch.complete,
    }
}

/// [`MapMark`] as its drawable counterpart: the tag's name sanitised, the
/// severity passed through.
fn mark_view(mark: &MapMark) -> MapMarkView {
    match mark {
        MapMark::Tag(name) => MapMarkView::Tag(sanitize_path_bytes(name)),
        MapMark::Radar(severity) => MapMarkView::Radar(*severity),
    }
}

#[cfg(test)]
mod tests {
    use mergewright_core::mistake_radar::Severity;
    use mergewright_core::ref_name::RefName;

    use super::*;
    use crate::update::detail::Loading;

    fn branch(name: &str, identity: u8, is_current: bool, is_default: bool) -> MapBranch {
        MapBranch {
            name: RefName::new(format!("refs/heads/{name}")).expect("a valid ref name"),
            identity: mergewright_graph::identity::BranchIdentity::of_name(&[identity]),
            tip: mergewright_core::oid::Oid::from_bytes(&[identity; mergewright_core::oid::Oid::SHA1_BYTES])
                .expect("20 bytes is a valid oid length"),
            tip_date: 0,
            counts: vec![0, 1, 3],
            is_current,
            is_default,
            merged_into: None,
            marks: Vec::new(),
            complete: true,
        }
    }

    fn data(branches: Vec<MapBranch>) -> MapData {
        MapData {
            branches,
            more: 0,
            from: 0,
            to: 3 * DAY_SECONDS,
            buckets: 3,
            complete: true,
        }
    }

    #[test]
    fn current_branch_titles_the_header_over_the_default() {
        let mut main = branch("main", 1, false, true);
        main.tip_date = 2 * DAY_SECONDS;
        let mut feature = branch("feature", 2, true, false);
        feature.tip_date = 2 * DAY_SECONDS;
        let view = ready_view(&data(vec![feature, main]), 0);
        assert_eq!(view.title_branch.as_str(), "feature");
        assert_eq!(view.span_days, 3);
        assert_eq!(view.rows.len(), 2);
    }

    #[test]
    fn a_tip_in_the_newest_bucket_with_no_merge_is_active() {
        let mut main = branch("main", 1, false, true);
        main.tip_date = 2 * DAY_SECONDS;
        let mut old = branch("old-topic", 2, false, false);
        old.tip_date = 0;
        let view = ready_view(&data(vec![main, old]), 0);
        assert!(view.rows[0].active, "tip in the newest bucket: active");
        assert!(!view.rows[1].active, "tip in the oldest bucket: not active");
    }

    #[test]
    fn a_merged_row_is_never_active_even_in_the_newest_bucket() {
        let mut main = branch("main", 1, false, true);
        main.tip_date = 2 * DAY_SECONDS;
        let mut merged = branch("topic", 2, false, false);
        merged.tip_date = 2 * DAY_SECONDS;
        merged.merged_into = Some((0, 2));
        let view = ready_view(&data(vec![main, merged]), 0);
        assert!(!view.rows[1].active);
    }

    #[test]
    fn marks_sanitise_the_tag_name_and_pass_the_severity_through() {
        let mut main = branch("main", 1, true, true);
        main.marks = vec![
            (0, MapMark::Tag(b"v1.0".to_vec())),
            (1, MapMark::Radar(Severity::Warning)),
        ];
        let view = ready_view(&data(vec![main]), 0);
        match &view.rows[0].marks[0].1 {
            mergewright_ui::map_screen::MapMarkView::Tag(name) => assert_eq!(name.as_str(), "v1.0"),
            mergewright_ui::map_screen::MapMarkView::Radar(severity) => {
                panic!("expected a Tag mark, got Radar({severity:?})")
            }
        }
        assert!(matches!(
            view.rows[0].marks[1].1,
            mergewright_ui::map_screen::MapMarkView::Radar(Severity::Warning)
        ));
    }

    #[test]
    fn no_branches_at_all_is_empty_not_ready() {
        let view = ready_view(&data(Vec::new()), 0);
        assert!(matches!(view.status, MapStatus::Empty));
    }

    #[test]
    fn idle_and_pending_and_failed_carry_no_rows() {
        let mut state = AppState::default();
        state.map.data = Loading::Idle;
        assert!(matches!(map_view(&state, 0).status, MapStatus::Empty));

        state.map.data = Loading::Pending(mergewright_core::job::JobId::new(1));
        state.map.progress = Some((2, 5));
        let view = map_view(&state, 0);
        assert!(matches!(
            view.status,
            MapStatus::Loading { done: 2, total: 5 }
        ));

        state.map.data = Loading::Failed("git for-each-ref: not a repository".to_owned());
        let view = map_view(&state, 0);
        assert!(matches!(view.status, MapStatus::Failed(_)));
    }
}
