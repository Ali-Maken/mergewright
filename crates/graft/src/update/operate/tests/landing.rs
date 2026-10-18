//! Where a finished put-back or `b` lands (m7-power-tools/24-a): the
//! restored commit, the new branch's commit, or the changes pane on the
//! file — never `HEAD` when `HEAD` did not move.

use std::path::PathBuf;

use graft_core::job::JobId;
use graft_core::journal::EntryId;
use graft_core::oid::Oid;
use graft_ops::apply::Refresh;
use graft_ops::operation::{
    BranchCreateParams, CommitParams, OnEdits, Operation, RecoverParams, RecoverTarget, Target,
};
use graft_ui::graph_screen::RowTag;

use super::applied;
use crate::jobs::operate::Operated;
use crate::update::banner::{refs_answered, walk_started};
use crate::update::events::Command;
use crate::update::operate::{Landing, landing_of, launch, operated};
use crate::update::state::{AppState, Status};

fn ids() -> impl FnMut() -> JobId {
    let mut next = 300;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes")
}

fn recover(target: RecoverTarget) -> Operation {
    Operation::Recover(RecoverParams {
        target,
        because: "reset: moving to HEAD~1".to_owned(),
    })
}

/// `main` moved back to `0x04` from `0x03`, `main` not checked out.
fn move_main() -> Operation {
    recover(RecoverTarget::MoveRef {
        name: "main".to_owned(),
        to: oid(4),
        expected: oid(3),
    })
}

fn files() -> Operation {
    recover(RecoverTarget::Files {
        entry: EntryId::new(1, [0; 10]),
        backup_tree: oid(8),
        paths: vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")],
    })
}

fn branch_at(at: Oid) -> Operation {
    Operation::BranchCreate(BranchCreateParams {
        name: "spike-restored".to_owned(),
        at,
        checkout: false,
        on_edits: OnEdits::Refuse,
    })
}

/// A graph of five commits (`0x05` newest … `0x01`), cursor on row 0.
fn on_the_graph() -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    }
}

/// Launches `op` on `state`, answers it `Done`, and returns that answer's
/// commands.
fn run_to_done(state: &mut AppState, op: Operation) -> Vec<Command> {
    let mut next_id = ids();
    let mut commands = Vec::new();
    launch(state, &mut commands, op, &mut next_id).expect("no write is running");
    let id = state.operating.expect("the launch took the slot");
    operated(
        state,
        id,
        Operated::Done(applied("put back", Refresh::Everything)),
        &mut next_id,
    )
}

/// The refresh's refs and walk answer, as `update` would pass them on.
fn refresh_lands(state: &mut AppState) -> Vec<Command> {
    let mut commands = Vec::new();
    if let Some(refs) = state.head_landing.refs {
        refs_answered(state, &mut commands, refs);
    }
    walk_started(state, &mut commands);
    commands
}

fn cursor_commit(state: &AppState) -> Option<Oid> {
    state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| row.commit.id)
}

#[test]
fn landing_of_names_every_kind() {
    assert_eq!(landing_of(&move_main()), Some(Landing::Commit(oid(4))));
    assert_eq!(
        landing_of(&recover(RecoverTarget::Recreate {
            name: "spike".to_owned(),
            at: oid(7),
        })),
        Some(Landing::Commit(oid(7)))
    );
    assert_eq!(
        landing_of(&files()),
        Some(Landing::Changes(PathBuf::from("src/a.rs")))
    );
    assert_eq!(
        landing_of(&branch_at(oid(2))),
        Some(Landing::Commit(oid(2)))
    );
    assert_eq!(
        landing_of(&Operation::Commit(CommitParams {
            message: "m".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            include: graft_ops::operation::Include::Staged,
        })),
        Some(Landing::Head)
    );
    assert_eq!(
        landing_of(&Operation::Stage(Target::Paths(vec!["a".into()]))),
        None
    );
}

#[test]
fn put_back_lands_on_the_restored_commit_with_tags_and_words() {
    let mut state = on_the_graph();
    let _refresh = run_to_done(&mut state, move_main());
    assert!(state.land_on_head);
    assert_eq!(state.head_landing.on, Landing::Commit(oid(4)));
    assert_eq!(state.row_tags.on(oid(4)), Some(&RowTag::PutBack));
    assert_eq!(
        state.row_tags.on(oid(3)),
        Some(&RowTag::WasHere {
            branch: "main".to_owned()
        })
    );
    // The history window holds `0x04`: its subject is the words'.
    assert_eq!(
        state.status,
        Status::Done {
            what: "put back".to_owned(),
            also: Some("main has \"Commit 4\u{241b}[31m\" again".to_owned()),
            keys: Some("u undoes".to_owned()),
        }
    );
    let _seek = refresh_lands(&mut state);
    assert_eq!(
        cursor_commit(&state),
        Some(oid(4)),
        "on the restored commit"
    );
}

/// A put-back whose after-state could not all be read back lands and tags
/// as its `Done` would, and its warning stays on the status row
/// (m8-polish-and-ship/13, ruling 3).
#[test]
fn an_incomplete_put_back_lands_tags_and_keeps_its_warning() {
    let mut state = on_the_graph();
    let mut next_id = ids();
    let mut commands = Vec::new();
    launch(&mut state, &mut commands, move_main(), &mut next_id).expect("no write is running");
    let id = state.operating.expect("the launch took the slot");
    let _refresh = operated(
        &mut state,
        id,
        Operated::Incomplete {
            applied: applied("put back", Refresh::Everything),
            error: "index unreadable".to_owned(),
        },
        &mut next_id,
    );
    assert_eq!(state.row_tags.on(oid(4)), Some(&RowTag::PutBack));
    let warning = Status::sticky("put back \u{2014} could not confirm it fully: index unreadable");
    assert_eq!(state.status, warning);
    let _seek = refresh_lands(&mut state);
    assert_eq!(
        cursor_commit(&state),
        Some(oid(4)),
        "on the restored commit"
    );
    assert_eq!(state.status, warning, "the landing kept the warning");
}

/// `HEAD` is on another branch (no refs read here names it at all): the
/// cursor goes to the restored commit, not wherever `HEAD` is.
#[test]
fn a_put_back_of_an_unchecked_branch_lands_on_the_restored_commit_not_head() {
    let mut state = on_the_graph();
    let _refresh = run_to_done(
        &mut state,
        recover(RecoverTarget::Recreate {
            name: "spike".to_owned(),
            at: oid(2),
        }),
    );
    let _seek = refresh_lands(&mut state);
    assert_eq!(cursor_commit(&state), Some(oid(2)));
    assert!(!state.land_on_head, "spent");
}

/// `J` puts back a branch that was merged (ease-pass 06c): its commit sits
/// behind the merge the default fold keeps closed, so the merge opens and
/// the cursor lands on the commit.
#[test]
fn a_put_back_onto_a_merged_branch_opens_its_merge_and_lands_on_it() {
    use crate::update::tests::reveal::{find_for, on, revealed};
    let walk = JobId::new(4242);
    let (merge, at) = (oid(3), oid(9));
    let mut state = on_the_graph();
    let recreate = RecoverTarget::Recreate {
        name: "spike".to_owned(),
        at,
    };
    let _refresh = run_to_done(&mut state, recover(recreate));
    state.awaiting = Some(walk);
    let seek = refresh_lands(&mut state);
    let find = find_for(&seek, at);

    let opened: &[u8] = &[5, 4, 3, 9, 2, 1];
    let (state, _) = revealed(&state, walk, find, (merge, at), (&[5, 4, 3, 2, 1], opened));

    assert_eq!(on(&state), Some(at), "the cursor on the restored commit");
}

#[test]
fn a_file_put_back_lands_on_the_changes_pane_on_the_file() {
    let mut state = on_the_graph();
    let _refresh = run_to_done(&mut state, files());
    assert!(!state.land_on_head, "no graph landing is owed");
    assert!(state.on_working_tree, "the changes pane");
    let kept = state
        .detail
        .kept_cursor
        .as_ref()
        .expect("the cursor goes to the file");
    assert_eq!(kept.path, b"src/a.rs".to_vec());
    assert_eq!(
        kept.section,
        Some(graft_ui::detail::WorkingTreeSection::Unstaged)
    );
    assert_eq!(
        state.status,
        Status::Done {
            what: "your edits to src/a.rs +1 more are back".to_owned(),
            also: None,
            keys: Some("u undoes".to_owned()),
        }
    );
}

#[test]
fn b_from_the_graph_lands_where_it_was() {
    let mut state = on_the_graph();
    let _refresh = run_to_done(&mut state, branch_at(oid(3)));
    assert_eq!(state.head_landing.on, Landing::Commit(oid(3)));
    let _seek = refresh_lands(&mut state);
    assert_eq!(cursor_commit(&state), Some(oid(3)));
}

#[test]
fn the_tags_clear_on_the_next_operation() {
    let mut state = on_the_graph();
    let _refresh = run_to_done(&mut state, move_main());
    assert!(state.row_tags.on(oid(4)).is_some());
    let mut commands = Vec::new();
    launch(
        &mut state,
        &mut commands,
        Operation::Stage(Target::Paths(vec!["a".into()])),
        &mut ids(),
    )
    .expect("the put-back finished");
    assert_eq!(state.row_tags.on(oid(4)), None);
    assert_eq!(state.row_tags.on(oid(3)), None);
}

/// `b` in J, a name, `⏎`: J closes, the branch is made, and the cursor
/// lands on the commit it was made at — not on `HEAD`.
#[test]
fn b_from_j_lands_on_the_new_branch() {
    let mut state = on_the_graph();
    state.journal.shown = true;
    assert!(crate::update::name_prompt::open_name_prompt_with(
        &mut state,
        crate::update::name_prompt::NameKind::Branch {
            at: oid(3),
            checkout: false,
        },
        "spike-restored",
    ));
    let mut next_id = ids();
    let mut commands = Vec::new();
    crate::update::name_prompt::name_prompt_key(
        &mut state,
        &mut commands,
        graft_core::event::KeyPress::plain(graft_core::event::KeyCode::Enter),
        &mut next_id,
    );
    assert!(!state.journal.shown, "J closed on create");
    let id = state.operating.expect("the branch is being made");
    assert_eq!(state.lands, Some((id, Landing::Commit(oid(3)))));
    let _refresh = operated(
        &mut state,
        id,
        Operated::Done(applied("created", Refresh::Everything)),
        &mut next_id,
    );
    let _seek = refresh_lands(&mut state);
    assert_eq!(
        cursor_commit(&state),
        Some(oid(3)),
        "on the new branch's commit"
    );
}
