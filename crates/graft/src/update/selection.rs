//! The uncommitted-changes row's one-bit selection.
//!
//! The row is synthesised by `view` (m1-the-graph/12b), not a position of
//! `History`'s cursor — making it one would shift every row index (windows,
//! requests, search hits, promotion anchors). So the selection lives beside
//! `History`'s cursor instead: `AppState::on_working_tree`. This module holds
//! the pure rules for setting, clearing and following it
//! (`docs/design-notes/m2-details-and-diffs.md`, "Spec fix (2026-09-17,
//! working-tree row)").

use graft_core::job::JobId;

use crate::history::Motion;

use super::detail::DetailSpec;
use super::events::Command;
use super::state::AppState;

/// A graph motion key, before `History::move_cursor` sees it: the motion
/// `History` should still apply, or `None` when the flag alone answered it
/// and nothing more happens this key.
///
/// Not on the row: `Up(1)` from the viewport's top row (`History::cursor ==
/// History::top`), while the row is shown, lands on it instead of scrolling
/// — `History` is asked for nothing. Any other motion passes through
/// unchanged (a `g`/`PageUp` that lands on row 0 stops there; one more `k`
/// reaches the virtual row).
///
/// On the row: `Down(n)` leaves it (`Down(0)` needs nothing more — `j`'s
/// `Down(1)` lands exactly on `HEAD`'s row, where the history cursor already
/// sits); `PageDown`/`Bottom` leave it and apply as usual; `Up`/`PageUp`/`Top`
/// do nothing at the very top of everything (`History::top == 0`), else
/// leave the row (the viewport scrolls up, the row goes with `HEAD`).
pub fn move_from_graph(state: &mut AppState, motion: Motion) -> Option<Motion> {
    if !state.on_working_tree {
        let at_top = state
            .history
            .as_ref()
            .is_some_and(|history| history.cursor == history.top);
        if matches!(motion, Motion::Up(1)) && at_top && state.uncommitted_row_shown() {
            state.on_working_tree = true;
            return None;
        }
        return Some(motion);
    }
    match motion {
        Motion::Down(n) => {
            state.on_working_tree = false;
            // `Down(1)` (`j`) lands exactly on `HEAD`'s row, where the
            // history cursor already sits — `Down(0)`'s "nothing more"
            // reading, so it collapses to `None` too.
            (n > 1).then(|| Motion::Down(n - 1))
        }
        Motion::PageDown | Motion::Bottom => {
            state.on_working_tree = false;
            Some(motion)
        }
        Motion::Up(_) | Motion::PageUp | Motion::Top => {
            let top = state.history.as_ref().map_or(0, |history| history.top);
            if top == 0 {
                None
            } else {
                state.on_working_tree = false;
                Some(motion)
            }
        }
    }
}

/// Called once at the end of every event, with `History::cursor`'s value
/// from the start of it. Clears the flag when the row is no longer shown
/// (the tree went clean on `Ctrl+R`, refs moved `HEAD`, the wheel scrolled
/// `HEAD` off the top, a resize) or the history cursor moved under it (a
/// search hit, a click, a `G` landing). Returns whether it cleared.
pub fn reconcile(state: &mut AppState, cursor_before: usize) -> bool {
    if !state.on_working_tree {
        return false;
    }
    let cursor_now = state.history.as_ref().map_or(0, |history| history.cursor);
    if !state.uncommitted_row_shown() || cursor_now != cursor_before {
        state.on_working_tree = false;
        true
    } else {
        false
    }
}

/// After any event: a shown pane follows the selection, the working-tree row
/// included. A no-op while hidden, or when the selection did not change
/// (`DetailState::follow`'s own check).
///
/// While a compare base is pinned (`AppState::compare_base`,
/// m2-details-and-diffs/11), the cursor moving onto another commit opens a
/// fresh `DetailSpec::Compare` against it instead of `Commit`; onto the
/// working-tree row it follows to `WorkingTree` as usual, the base kept
/// (`x` is a no-op there — `update::keys`); back onto the base's own row it
/// does nothing (comparing a commit with itself is not a spec). With no base
/// pinned, a pane already showing a `Compare` (e.g. from the context menu,
/// `context-actions-menu`) does not follow the cursor at all.
pub fn follow_selection(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    if !state.detail.shown {
        return Vec::new();
    }
    if let Some(base) = state.compare_base {
        let spec = match state.selected_spec() {
            Some(DetailSpec::Commit(oid)) if oid != base.oid => DetailSpec::Compare {
                base: base.oid,
                head: oid,
            },
            Some(spec @ DetailSpec::WorkingTree) => spec,
            _ => return Vec::new(),
        };
        return state.detail.follow(spec, next_id);
    }
    if matches!(state.detail.spec, Some(DetailSpec::Compare { .. })) {
        return Vec::new();
    }
    let Some(spec) = state.selected_spec() else {
        return Vec::new();
    };
    state.detail.follow(spec, next_id)
}

#[cfg(test)]
mod tests {
    use graft_core::job::JobId;
    use graft_git::status::WorkingTreeCounts;

    use super::*;
    use crate::history::{History, Motion};
    use crate::refs::RefBadges;
    use crate::update::detail::DetailSpec;
    use crate::update::state::{AppState, Harness, Size};

    fn counts() -> WorkingTreeCounts {
        WorkingTreeCounts {
            staged: 2,
            unstaged: 1,
            untracked: 1,
        }
    }

    /// A state with refs answered, `HEAD` at row 0, and a dirty tree — the
    /// row is shown, cursor at the top.
    fn shown_state() -> AppState {
        let mut history = History::started();
        history.rows = 3;
        history.top = 0;
        history.cursor = 0;
        let mut state = AppState {
            size: Size {
                columns: 80,
                rows: 30,
            },
            harness: Harness::default(),
            history: Some(history),
            working_tree: Some(counts()),
            ..Default::default()
        };
        // `HEAD` unborn reads as topmost with no refs to compare against, so
        // an empty `RefBadges` (no head, no branches) is enough to make the
        // row attach without a real repository.
        state.refs = Some(std::sync::Arc::new(RefBadges::default()));
        state
    }

    #[test]
    fn up_one_from_the_top_row_selects_the_virtual_row_and_asks_history_for_nothing() {
        let mut state = shown_state();
        let motion = move_from_graph(&mut state, Motion::Up(1));
        assert!(state.on_working_tree);
        assert_eq!(motion, None);
    }

    #[test]
    fn up_one_from_the_top_row_scrolls_as_in_m1_when_the_row_is_hidden() {
        let mut state = shown_state();
        state.working_tree = Some(WorkingTreeCounts {
            staged: 0,
            unstaged: 0,
            untracked: 0,
        });
        let motion = move_from_graph(&mut state, Motion::Up(1));
        assert!(!state.on_working_tree);
        assert_eq!(motion, Some(Motion::Up(1)));
    }

    #[test]
    fn down_one_from_the_row_selects_heads_row_and_asks_history_for_nothing() {
        let mut state = shown_state();
        state.on_working_tree = true;
        let motion = move_from_graph(&mut state, Motion::Down(1));
        assert!(!state.on_working_tree);
        assert_eq!(motion, None);
    }

    #[test]
    fn down_three_from_the_row_leaves_down_two_for_history() {
        let mut state = shown_state();
        state.on_working_tree = true;
        let motion = move_from_graph(&mut state, Motion::Down(3));
        assert!(!state.on_working_tree);
        assert_eq!(motion, Some(Motion::Down(2)));
    }

    #[test]
    fn page_down_from_the_row_clears_the_flag_and_applies() {
        let mut state = shown_state();
        state.on_working_tree = true;
        let motion = move_from_graph(&mut state, Motion::PageDown);
        assert!(!state.on_working_tree);
        assert_eq!(motion, Some(Motion::PageDown));
    }

    #[test]
    fn bottom_from_the_row_clears_the_flag_and_applies() {
        let mut state = shown_state();
        state.on_working_tree = true;
        let motion = move_from_graph(&mut state, Motion::Bottom);
        assert!(!state.on_working_tree);
        assert_eq!(motion, Some(Motion::Bottom));
    }

    #[test]
    fn up_or_pageup_or_top_from_the_row_does_nothing_at_the_very_top() {
        let mut state = shown_state();
        state.on_working_tree = true;
        assert_eq!(move_from_graph(&mut state, Motion::Up(1)), None);
        assert!(state.on_working_tree);
        assert_eq!(move_from_graph(&mut state, Motion::PageUp), None);
        assert!(state.on_working_tree);
        assert_eq!(move_from_graph(&mut state, Motion::Top), None);
        assert!(state.on_working_tree);
    }

    #[test]
    fn up_from_the_row_scrolls_up_one_when_the_viewport_is_not_at_the_very_top() {
        let mut state = shown_state();
        state.history.as_mut().unwrap().top = 5;
        state.on_working_tree = true;
        let motion = move_from_graph(&mut state, Motion::Up(1));
        assert!(!state.on_working_tree);
        assert_eq!(motion, Some(Motion::Up(1)));
    }

    #[test]
    fn reconcile_clears_when_the_tree_goes_clean() {
        let mut state = shown_state();
        state.on_working_tree = true;
        state.working_tree = Some(WorkingTreeCounts {
            staged: 0,
            unstaged: 0,
            untracked: 0,
        });
        assert!(reconcile(&mut state, 0));
        assert!(!state.on_working_tree);
    }

    #[test]
    fn reconcile_clears_when_the_history_cursor_moved() {
        let mut state = shown_state();
        state.on_working_tree = true;
        state.history.as_mut().unwrap().cursor = 1;
        assert!(reconcile(&mut state, 0));
        assert!(!state.on_working_tree);
    }

    #[test]
    fn reconcile_keeps_the_flag_when_the_row_still_shows_and_the_cursor_did_not_move() {
        let mut state = shown_state();
        state.on_working_tree = true;
        assert!(!reconcile(&mut state, 0));
        assert!(state.on_working_tree);
    }

    #[test]
    fn reconcile_is_a_no_op_when_the_flag_is_already_off() {
        let mut state = shown_state();
        assert!(!reconcile(&mut state, 0));
        assert!(!state.on_working_tree);
    }

    fn detail_shown_state() -> AppState {
        let mut state = shown_state();
        let mut ids = 0u64;
        let mut next_id = || {
            ids += 1;
            JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
        };
        state
            .detail
            .open(DetailSpec::Commit(head_id()), &mut next_id);
        state
    }

    fn head_id() -> graft_core::oid::Oid {
        graft_core::oid::Oid::from_bytes(&[1u8; 20]).unwrap()
    }

    fn identity() -> graft_core::commit::Identity {
        graft_core::commit::Identity {
            name: b"Test".to_vec(),
            email: b"test@example.invalid".to_vec(),
            when: graft_core::commit::Timestamp {
                seconds: 0,
                offset_seconds: 0,
            },
        }
    }

    fn commit_row(byte: u8) -> graft_graph::Row {
        graft_graph::Row {
            commit: graft_core::commit::Commit {
                id: graft_core::oid::Oid::from_bytes(&[byte; 20]).unwrap(),
                parents: Vec::new(),
                author: identity(),
                committer: identity(),
                message: b"a commit".to_vec(),
            },
            lane: 0,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            lanes: Vec::new(),
            marker: None,
        }
    }

    /// `shown_state` with a two-row window on hand (rows 0 and 1, distinct
    /// commits), so `selected_spec` names a real commit as the cursor moves
    /// — what the compare-follow tests below need
    /// (m2-details-and-diffs/11).
    fn state_with_rows() -> AppState {
        let mut state = shown_state();
        let history = state.history.as_mut().expect("shown_state sets one");
        history.window = Some(crate::history::Window {
            request: 1,
            first: 0,
            rows: vec![commit_row(1), commit_row(2)].into(),
            laid_out: 2,
            done: false,
            promoted: false,
        });
        state
    }

    #[test]
    fn follow_selection_follows_onto_the_working_tree_row() {
        let mut state = detail_shown_state();
        state.on_working_tree = true;
        let mut ids = 100u64;
        let commands = follow_selection(&mut state, &mut || {
            ids += 1;
            JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
        });
        assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
        assert!(!commands.is_empty());
    }

    #[test]
    fn follow_selection_is_a_no_op_while_hidden() {
        let mut state = shown_state();
        state.on_working_tree = true;
        let mut ids = 100u64;
        let commands = follow_selection(&mut state, &mut || {
            ids += 1;
            JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
        });
        assert!(commands.is_empty());
        assert_eq!(state.detail.spec, None);
    }

    /// While a base is pinned, moving the cursor opens a fresh `Compare`
    /// against the new row each time — never `Commit`
    /// (m2-details-and-diffs/11).
    #[test]
    fn follow_selection_opens_a_fresh_compare_as_the_cursor_moves() {
        let mut state = state_with_rows();
        let mut ids = 0u64;
        let mut next_id = || {
            ids += 1;
            JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
        };
        let base_oid = commit_row(1).commit.id;
        state
            .detail
            .open(DetailSpec::Commit(base_oid), &mut next_id);
        state.compare_base = Some(super::super::detail::CompareBase {
            oid: base_oid,
            row: 0,
        });
        state.history.as_mut().unwrap().cursor = 1;

        let commands = follow_selection(&mut state, &mut next_id);

        assert!(!commands.is_empty());
        assert_eq!(
            state.detail.spec,
            Some(DetailSpec::Compare {
                base: base_oid,
                head: commit_row(2).commit.id,
            })
        );
        assert_eq!(
            state.compare_base,
            Some(super::super::detail::CompareBase {
                oid: base_oid,
                row: 0
            }),
            "the base stays set as the compare follows"
        );
    }

    /// Onto the working-tree row, a pinned base is left alone and the pane
    /// follows to `WorkingTree` as usual (m2-details-and-diffs/11).
    #[test]
    fn follow_selection_onto_the_working_tree_row_keeps_the_base() {
        let mut state = state_with_rows();
        let mut ids = 0u64;
        let mut next_id = || {
            ids += 1;
            JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
        };
        let base_oid = commit_row(1).commit.id;
        state
            .detail
            .open(DetailSpec::Commit(base_oid), &mut next_id);
        state.compare_base = Some(super::super::detail::CompareBase {
            oid: base_oid,
            row: 0,
        });
        state.on_working_tree = true;

        let commands = follow_selection(&mut state, &mut next_id);

        assert!(!commands.is_empty());
        assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
        assert!(state.compare_base.is_some(), "the base stays set");
    }

    /// Back onto the base's own row, there is nothing to compare it with:
    /// no command, no spec change (m2-details-and-diffs/11).
    #[test]
    fn follow_selection_back_onto_the_base_row_does_nothing() {
        let mut state = state_with_rows();
        let mut ids = 0u64;
        let mut next_id = || {
            ids += 1;
            JobId::new(super::super::detail::LOCAL_JOB_BIT | ids)
        };
        let base_oid = commit_row(1).commit.id;
        let head_oid = commit_row(2).commit.id;
        state.detail.open(
            DetailSpec::Compare {
                base: base_oid,
                head: head_oid,
            },
            &mut next_id,
        );
        state.compare_base = Some(super::super::detail::CompareBase {
            oid: base_oid,
            row: 0,
        });
        state.history.as_mut().unwrap().cursor = 0;

        let commands = follow_selection(&mut state, &mut next_id);

        assert!(commands.is_empty());
        assert_eq!(
            state.detail.spec,
            Some(DetailSpec::Compare {
                base: base_oid,
                head: head_oid,
            })
        );
    }
}
