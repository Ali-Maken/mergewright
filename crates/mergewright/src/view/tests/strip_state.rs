//! The overview strip capture's fixture state (split out of `view::tests`).

use mergewright_core::oid::Oid;

use super::{oid, refs_with_head};
use crate::history::{History, Phase};
use crate::update::{AppState, Size};

/// Two feature branches, each merged into `main`, `main`'s tip as `HEAD`:
/// mirrors `mergewright_fixtures::Shape::FeatureBranches`'s shape without a real
/// checkout (`view::strip::tests::feature_branches_state`'s twin — this
/// module cannot see that private `#[cfg(test)]` item, so it is rebuilt
/// here rather than shared).
pub(super) fn strip_capture_state() -> AppState {
    use mergewright_core::commit::{Commit, Identity, Timestamp};
    use mergewright_graph::layout::layout;
    use mergewright_graph::seeds::Seeds;

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
    fn commit(id: u8, parents: Vec<Oid>, message: &str) -> Commit {
        Commit {
            id: oid(id),
            parents,
            author: who(i64::from(id)),
            committer: who(i64::from(id)),
            message: message.as_bytes().to_vec(),
        }
    }

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
        window: Some(crate::history::Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            walked: rows.len(),
            done: true,
            rows: rows.into(),
            ..Default::default()
        }),
        rows: row_count,
        phase: Phase::Complete,
        cursor: 0,
        reanchor: None,
        ..History::started(crate::config::MergedBranches::Folded)
    };
    AppState {
        history: Some(history),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(9))))),
        size: Size {
            columns: 80,
            rows: 24,
        },
        strip_shown: true,
        ..AppState::default()
    }
}
