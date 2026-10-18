//! Building [`StripMarks`]: what the overview strip (`O`,
//! m2-details-and-diffs/20) draws, read off what `History` holds right now.
//!
//! Marks are a paint-time lookup like the marker column
//! (`view::radar::marker_for`, m2-details-and-diffs/13b's Proposed changes):
//! only rows currently resident get looked at, so the strip is honestly
//! incomplete while a repo is still streaming (DESIGN.md §5.3 "Cost") —
//! never a claim about rows it hasn't seen.

use std::collections::HashSet;

use mergewright_core::oid::Oid;
use mergewright_ui::overview_strip::{StripMark, StripMarks};

use crate::update::AppState;

/// The strip's marks over the rows `state.history` holds now: `Merge` where
/// a row has two or more parents, `Radar(sev)` from `view::radar::marker_for`
/// (which itself only answers once the radar context is `Ready`), `Head` on
/// the row `RefBadges::head()` names, and `SearchHit` for each search hit
/// that lands on a resident row. Empty (`rows_total == 0`, no marks, an
/// empty viewport) when there is no history yet.
#[must_use]
pub fn strip_marks(state: &AppState) -> StripMarks {
    let Some(history) = &state.history else {
        return StripMarks {
            rows_total: 0,
            viewport: 0..0,
            marks: Vec::new(),
        };
    };

    let mut marks = Vec::new();
    let head = state.refs.as_ref().and_then(|refs| refs.head());
    // Once per frame, not once per row: a search that matches thousands of
    // commits used to be compared against every resident row (M2 review
    // M10). One hit is one commit, so a set answers the same question.
    let hits: HashSet<Oid> = state.search.as_ref().map_or_else(HashSet::new, |search| {
        (0..search.hits())
            .filter_map(|index| search.hit(index))
            .collect()
    });

    if let Some(window) = &history.window {
        for (offset, row) in window.rows.iter().enumerate() {
            let at = window.first + offset;
            if crate::history::parents::is_merge(row, state.refs.as_deref()) {
                marks.push((at, StripMark::Merge));
            }
            // `subject()` borrows the message it is already holding; the
            // whole message was being decoded here for a rule that only
            // reads the first line.
            let subject = String::from_utf8_lossy(row.commit.subject());
            if let Some(marker) = super::radar::marker_for(
                state,
                row.commit.id,
                crate::history::parents::of(row, state.refs.as_deref()),
                &subject,
            ) {
                marks.push((at, StripMark::Radar(marker.severity)));
            }
            if Some(row.commit.id) == head {
                marks.push((at, StripMark::Head));
            }
            if hits.contains(&row.commit.id) {
                marks.push((at, StripMark::SearchHit));
            }
        }
    }

    let top = history.top;
    let graph_rows = state.graph_rows();
    StripMarks {
        rows_total: history.rows,
        viewport: top..top.saturating_add(graph_rows),
        marks,
    }
}

#[cfg(test)]
mod tests {
    use mergewright_core::commit::{Commit, Identity, Timestamp};
    use mergewright_graph::layout::layout;
    use mergewright_graph::seeds::Seeds;

    use super::*;
    use crate::history::{History, Phase, Window};
    use crate::update::Size;
    use crate::view::tests::{oid, refs_with_head};

    fn who(seconds: i64) -> Identity {
        Identity {
            name: b"Test".to_vec(),
            email: b"test@example.invalid".to_vec(),
            when: Timestamp {
                seconds,
                offset_seconds: 0,
            },
        }
    }

    fn commit(id: u8, parents: Vec<mergewright_core::oid::Oid>, message: &str) -> Commit {
        Commit {
            id: oid(id),
            parents,
            author: who(i64::from(id)),
            committer: who(i64::from(id)),
            message: message.as_bytes().to_vec(),
        }
    }

    /// Two feature branches, each merged into `main`: `main`'s tip (row 0)
    /// is `HEAD`, and rows 1 and 4 are the two merges. Mirrors
    /// `mergewright_fixtures::Shape::FeatureBranches`'s shape without a real
    /// checkout — a synthetic `layout()` is enough for `strip_marks`, which
    /// never reads the repository itself.
    fn feature_branches_state() -> AppState {
        let commits = vec![
            commit(9, vec![oid(8), oid(4)], "Merge branch 'topic-b'"),
            commit(8, vec![oid(7)], "topic-b work"),
            commit(7, vec![oid(4)], "topic-b start"),
            commit(4, vec![oid(3), oid(2)], "Merge branch 'topic-a'"),
            commit(3, vec![oid(1)], "topic-a work"),
            commit(2, vec![oid(1)], "topic-a start"),
            commit(1, Vec::new(), "root"),
        ];
        let rows = layout(&commits, Seeds::default());
        let row_count = rows.len();
        let history = History {
            window: Some(Window {
                request: 0,
                first: 0,
                laid_out: rows.len(),
                done: true,
                rows: rows.into(),
                promoted: false,
                fold: [].into(),
                sought: None,
            }),
            rows: row_count,
            phase: Phase::Complete,
            cursor: 0,
            reanchor: None,
            ..History::started()
        };
        AppState {
            history: Some(history),
            refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(9))))),
            size: Size {
                columns: 100,
                rows: 16,
            },
            ..AppState::default()
        }
    }

    #[test]
    fn feature_branches_marks_both_merges_and_head_and_the_viewport() {
        let state = feature_branches_state();
        let marks = strip_marks(&state);

        assert_eq!(marks.rows_total, 7);
        assert_eq!(marks.viewport, 0..state.graph_rows());
        assert!(marks.marks.contains(&(0, StripMark::Head)));

        let merge_rows: Vec<usize> = marks
            .marks
            .iter()
            .filter(|(_, mark)| *mark == StripMark::Merge)
            .map(|(row, _)| *row)
            .collect();
        assert_eq!(merge_rows.len(), 2, "{merge_rows:?}");
    }

    #[test]
    fn no_history_yet_is_empty() {
        let state = AppState::default();
        let marks = strip_marks(&state);
        assert_eq!(marks.rows_total, 0);
        assert_eq!(marks.viewport, 0..0);
        assert!(marks.marks.is_empty());
    }

    #[test]
    fn a_search_hit_on_a_resident_row_is_marked() {
        let mut state = feature_branches_state();
        let query = crate::search::Query::parse("root").expect("a non-empty query");
        let mut search = crate::search::Search::new(query, 0, (0, 0));
        search.batches = vec![std::sync::Arc::from(vec![oid(1)])];
        search.state = crate::search::SearchState::Done { hits: 1 };
        state.search = Some(search);

        let marks = strip_marks(&state);
        let row_of_oid_1 = state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .and_then(|window| window.rows.iter().position(|row| row.commit.id == oid(1)))
            .expect("root commit is a resident row");
        assert!(marks.marks.contains(&(row_of_oid_1, StripMark::SearchHit)));
    }

    /// ease-pass 06c: hits in folded merges are the search's count, not
    /// strip marks. The strip marks the one resident hit (`root`, row 6),
    /// the same whether the walk counted the other two folded or not.
    #[test]
    fn hits_in_folded_merges_leave_the_strip_marks_unchanged() {
        let mut state = feature_branches_state();
        let query = crate::search::Query::parse("totals").expect("a non-empty query");
        let mut search = crate::search::Search::new(query, 0, (0, 0));
        search.batches = vec![std::sync::Arc::from(vec![oid(1), oid(0x40), oid(0x41)])];
        search.state = crate::search::SearchState::Done { hits: 3 };
        state.search = Some(search.clone());
        let uncounted = strip_marks(&state);
        search.folded = 2;
        state.search = Some(search);

        let counted = strip_marks(&state);

        assert_eq!(counted, uncounted);
        let hits: Vec<usize> = counted
            .marks
            .iter()
            .filter(|(_, mark)| *mark == StripMark::SearchHit)
            .map(|(row, _)| *row)
            .collect();
        assert_eq!(hits, [6]);
    }
}
