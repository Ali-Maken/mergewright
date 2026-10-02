//! A finished write lands the graph cursor on the new `HEAD` exactly when
//! its launch said it moves `HEAD` (`Operation::moves_head`,
//! m7-power-tools/03). Decided at launch, carried beside `operating`, spent
//! by `operated`'s `Done` through `banner::finished_lands_on_head`.

use graft_core::job::JobId;
use graft_ops::apply::Refresh;
use graft_ops::operation::{
    CommitParams, Operation, ResetMode, ResetParams, ResetTarget, Target, Tip,
};

use super::applied;
use crate::jobs::operate::Operated;
use crate::update::events::Command;
use crate::update::operate::{launch, operated, write_job_ended};
use crate::update::state::AppState;

fn ids() -> impl FnMut() -> JobId {
    let mut next = 200;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn commit() -> Operation {
    Operation::Commit(CommitParams {
        message: "m".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
    })
}

fn reset(target: ResetTarget) -> Operation {
    Operation::Reset(ResetParams {
        target,
        to: graft_core::oid::Oid::from_bytes(&[0x0d; 20]).expect("20 bytes"),
    })
}

fn tip(name: &str) -> Tip {
    Tip {
        name: name.to_owned(),
        oid: graft_core::oid::Oid::from_bytes(&[0x0a; 20]).expect("20 bytes"),
    }
}

/// Launches `op`, answers it `Done` with `Everything` re-read, and returns
/// the state and that answer's commands.
fn launched_and_done(op: Operation) -> (AppState, Vec<Command>) {
    let mut state = AppState::default();
    let mut next_id = ids();
    let mut commands = Vec::new();
    launch(&mut state, &mut commands, op, &mut next_id).expect("no write is running");
    let id = state.operating.expect("the launch took the slot");
    let commands = operated(
        &mut state,
        id,
        Operated::Done(applied("the write", Refresh::Everything)),
        &mut next_id,
    );
    (state, commands)
}

#[test]
fn a_done_write_that_moves_head_owes_the_landing_on_its_refresh() {
    for op in [
        commit(),
        reset(ResetTarget::Head {
            branch: tip("refs/heads/main"),
            mode: ResetMode::Hard,
        }),
    ] {
        let (state, commands) = launched_and_done(op);
        assert!(state.land_on_head, "{commands:?}");
        let refs = commands.iter().find_map(|command| match command {
            Command::ReadRefs(id) => Some(*id),
            _ => None,
        });
        let walk = commands.iter().find_map(|command| match command {
            Command::ReadHistory(id, _) => Some(*id),
            _ => None,
        });
        assert!(refs.is_some() && walk.is_some(), "{commands:?}");
        assert_eq!(
            state.head_landing.refs, refs,
            "waits on this refresh's refs"
        );
        assert_eq!(state.head_landing.walk, walk, "and on its walk");
        assert_eq!(state.lands, None, "spent by the answer");
    }
}

/// `branch -f` of a branch `HEAD` is not on: the cursor stays.
#[test]
fn a_branch_reset_leaves_the_cursor() {
    let (state, _) = launched_and_done(reset(ResetTarget::Branch(tip("refs/heads/feat"))));
    assert!(!state.land_on_head);
}

#[test]
fn a_stage_leaves_the_cursor() {
    let (state, _) = launched_and_done(Operation::Stage(Target::Paths(vec!["a".into()])));
    assert!(!state.land_on_head);
}

/// A write that dies without answering owes nothing: the next write's
/// `Done` must not inherit it.
#[test]
fn a_write_that_died_owes_no_landing() {
    let mut state = AppState::default();
    let mut next_id = ids();
    let mut commands = Vec::new();
    launch(&mut state, &mut commands, commit(), &mut next_id).expect("no write is running");
    let id = state.operating.expect("the launch took the slot");
    assert_eq!(
        state.lands,
        Some((id, crate::update::operate::Landing::Head))
    );
    let _refresh = write_job_ended(&mut state, id, &mut next_id);
    assert_eq!(state.lands, None);
    assert!(!state.land_on_head);
}

/// An `Incomplete` answer ran all the same: it lands as `Done` does
/// (m8-polish-and-ship/13, ruling 3), spending the launch's landing, and
/// keeps its own warning on the status row.
#[test]
fn an_incomplete_answer_lands_and_keeps_its_warning() {
    let mut state = AppState::default();
    let mut next_id = ids();
    let mut commands = Vec::new();
    launch(&mut state, &mut commands, commit(), &mut next_id).expect("no write is running");
    let id = state.operating.expect("the launch took the slot");
    let _refresh = operated(
        &mut state,
        id,
        Operated::Incomplete {
            applied: applied("the write", Refresh::Everything),
            error: "could not read the after-state".to_owned(),
        },
        &mut next_id,
    );
    assert!(state.land_on_head, "lands as Done does");
    assert_eq!(state.head_landing.on, crate::update::operate::Landing::Head);
    assert_eq!(state.lands, None, "the launch's landing is spent");
    assert_eq!(
        state.status,
        crate::update::state::Status::sticky(
            "the write \u{2014} could not confirm it fully: could not read the after-state"
        )
    );
}
