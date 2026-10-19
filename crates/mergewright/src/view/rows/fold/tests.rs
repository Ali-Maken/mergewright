//! A folding merge's tail and the opened merge a row sits inside, read from
//! the fold facts a window carries (ease-pass 06 boxes 4 and 5).

use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_graph::Row;
use mergewright_graph::layout::layout;
use mergewright_graph::seeds::Seeds;
use mergewright_term::sanitize_line;
use mergewright_ui::graph_screen::{FoldCount, FoldMark};

use crate::history::{History, Inside, Phase, RowFold, View, Window};
use crate::update::AppState;
use crate::update::fold_count::FoldCount as Asked;
use crate::view::tests::{draw, oid};

use super::{InsideMerge, fold_tail, inside_merge};

/// `main ← wallet-release ← sara/fix-toc`, as a folding view draws it with
/// the release's PR merge opened: the PR merge (row 0, folds, opened), the
/// named merge inside it (row 1, folds, closed), a sync merge (row 2,
/// folds nothing) and a plain commit on main (row 3).
fn wallet() -> (History, [Oid; 3]) {
    let commits = wallet_commits();
    let rows = layout(&commits, Seeds::default());
    let (pr, named, sync) = (oid(9), oid(8), oid(7));
    let folds = |folds| RowFold {
        folds: Some(folds),
        inside: None,
    };
    let fold = [
        folds(true),
        RowFold {
            folds: Some(true),
            inside: Some(Inside {
                merge: pr,
                label: Some(b"PR 16021".as_slice().into()),
            }),
        },
        folds(false),
        RowFold::default(),
    ];
    let mut view = View::folded();
    view.inclusion
        .opened_mut()
        .expect("a folding view")
        .open(pr);
    let history = History {
        window: Some(Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            done: true,
            rows: rows.into(),
            promoted: false,
            fold: fold.into(),
            sought: None,
        }),
        rows: commits.len(),
        phase: Phase::Complete,
        view,
        ..History::started(crate::config::MergedBranches::Folded)
    };
    (history, [pr, named, sync])
}

/// The four commits [`wallet`] draws, newest first.
fn wallet_commits() -> Vec<Commit> {
    let who = |seconds| Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    };
    let commit = |id: u8, parents: Vec<Oid>, message: &str| Commit {
        id: oid(id),
        parents,
        author: who(i64::from(id)),
        committer: who(i64::from(id)),
        message: message.as_bytes().to_vec(),
    };
    vec![
        commit(
            9,
            vec![oid(5), oid(8)],
            "Merged PR 16021: wallet release into main",
        ),
        commit(
            8,
            vec![oid(7), oid(6)],
            "Merge branch 'sara/fix-toc' into wallet-release",
        ),
        commit(
            7,
            vec![oid(5), oid(4)],
            "Merge branch 'main' into wallet-release",
        ),
        commit(5, vec![oid(4)], "Bump the version"),
    ]
}

fn row(history: &History, at: usize) -> Row {
    history.row_at(at).expect("on hand").clone()
}

#[test]
fn a_folding_merge_tails_with_its_count_and_the_name_its_subject_gives() {
    let (history, [pr, named, _]) = wallet();
    let mut state = AppState::default();
    state.fold_counts.insert(pr, Asked::Counted(4));
    state.fold_counts.insert(named, Asked::Counted(1));

    assert_eq!(
        fold_tail(&state, &history, &row(&history, 0), 0),
        Some(FoldMark {
            open: true,
            count: FoldCount::Known(4),
            from: Some(sanitize_line("PR 16021")),
        }),
        "an Azure `Merged PR <n>: …` gives `PR <n>`; opened, it reads ▾"
    );
    assert_eq!(
        fold_tail(&state, &history, &row(&history, 1), 1),
        Some(FoldMark {
            open: false,
            count: FoldCount::Known(1),
            from: Some(sanitize_line("sara/fix-toc")),
        }),
        "a named merge gives its branch in full"
    );
}

#[test]
fn the_count_is_pending_until_it_lands_and_failed_when_it_failed() {
    let (history, [pr, ..]) = wallet();
    let merge = row(&history, 0);
    let count = |state: &AppState| fold_tail(state, &history, &merge, 0).expect("a tail").count;

    let mut state = AppState::default();
    assert_eq!(count(&state), FoldCount::Pending, "never asked");
    state.fold_counts.insert(pr, Asked::Counting);
    assert_eq!(count(&state), FoldCount::Pending, "asked");
    state.fold_counts.insert(pr, Asked::Failed);
    assert_eq!(count(&state), FoldCount::Failed);
}

#[test]
fn no_tail_on_a_sync_merge_a_plain_commit_or_a_row_the_fold_has_no_fact_for() {
    let (history, _) = wallet();
    let state = AppState::default();
    assert_eq!(
        fold_tail(&state, &history, &row(&history, 2), 2),
        None,
        "sync merge"
    );
    assert_eq!(
        fold_tail(&state, &history, &row(&history, 3), 3),
        None,
        "plain"
    );

    let mut unknown = history.clone();
    if let Some(window) = unknown.window.as_mut() {
        window.fold = [].into();
    }
    assert_eq!(
        fold_tail(&state, &unknown, &row(&history, 0), 0),
        None,
        "no fact yet"
    );
}

#[test]
fn a_row_inside_an_opened_merge_knows_the_merge_and_its_name() {
    let (history, [pr, ..]) = wallet();
    assert_eq!(
        inside_merge(&history, 1),
        Some(InsideMerge {
            merge: pr,
            label: Some(b"PR 16021".to_vec()),
        })
    );
    assert_eq!(inside_merge(&history, 0), None, "the merge itself");
}

/// Box 5, on screen: the merges' rows end with their tails, as mock G4
/// draws them, and nothing behind a closed merge is drawn.
#[test]
fn the_graph_draws_each_folding_merge_with_its_tail() {
    let (history, [pr, named, _]) = wallet();
    let mut state = AppState {
        history: Some(history),
        size: crate::update::Size {
            columns: 120,
            rows: 16,
        },
        refs: Some(std::sync::Arc::new(crate::refs::RefBadges::default())),
        ..AppState::default()
    };
    state.fold_counts.insert(pr, Asked::Counted(4));
    state.fold_counts.insert(named, Asked::Counting);

    let screen = draw(&state, 120, 16);
    let line = |needle: &str| {
        screen
            .lines()
            .find(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("no {needle:?} in\n{screen}"))
            .to_owned()
    };
    assert!(
        line("Merged PR 16").ends_with("▾ 4 commits from PR 16021")
            || line("Merged PR 16").contains("▾ 4 commits from PR 16021"),
        "{screen}"
    );
    assert!(
        // The subject is cut beside the branch pane (ease-pass 09): the
        // row is found by its tail, which has no number before `commits`.
        line("commits from sara/fix-toc").contains("▸ commits from sara/fix-toc"),
        "pending: no number yet\n{screen}"
    );
    assert!(
        !line("Merge branch 'main'").contains("commits"),
        "a sync merge has no tail\n{screen}"
    );
}

/// G4: on a row inside an opened merge, the lanes are coloured for the
/// branch the merge's row was made on, not the row's own.
#[test]
fn inside_an_opened_merge_the_focus_is_the_merges_branch() {
    let (mut history, _) = wallet();
    let merge_row = row(&history, 0);
    history.cursor = 1;
    let focus = crate::view::entries::focus(&history);
    assert_eq!(focus, mergewright_graph::made_on(&merge_row));
    assert_ne!(
        focus,
        mergewright_graph::made_on(&row(&history, 1)),
        "the fixture tells the two apart"
    );
    history.cursor = 3;
    assert_eq!(
        crate::view::entries::focus(&history),
        mergewright_graph::made_on(&row(&history, 3)),
        "outside the opened merge, the row's own branch"
    );
}

/// A lane with no branch behind it draws as another branch's; one role per
/// real column.
#[test]
fn an_unnamed_lane_is_other() {
    use mergewright_graph::identity::BranchIdentity;
    use mergewright_graph::{LaneRole, Route};
    let (history, _) = wallet();
    let main = BranchIdentity::of_name(b"main");
    let mut row = row(&history, 0);
    row.lanes = vec![Some(main), None, None];
    let roles = crate::view::entries::lane_roles(&row, &Route::unread(main, None));
    assert_eq!(roles, [LaneRole::Trunk, LaneRole::Other, LaneRole::Other]);
}
