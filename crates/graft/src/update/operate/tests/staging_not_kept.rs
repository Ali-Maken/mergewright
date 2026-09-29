//! The put-back's fallback line (ticket 30b, Decided 3): the op finished,
//! plus one piece of news — never "could not confirm".

use std::path::PathBuf;

use graft_core::job::JobId;
use graft_ops::apply::{AfterState, Refresh};
use graft_ops::operation::{CommitParams, Operation};

use super::applied;
use crate::jobs::operate::Operated;
use crate::update::operate::outcome::staging_not_kept_rungs;
use crate::update::operate::{launch, operated, staging_not_kept_status};
use crate::update::state::{AppState, Status};

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn staging_not_kept_rungs_cut_the_edits_back_clause_first() {
    assert_eq!(
        staging_not_kept_rungs("checkout fix", &paths(&["base.txt"])),
        vec![
            "checkout fix \u{2713} \u{b7} your edits are back \u{b7} staging not kept: base.txt \u{b7} u undoes it, staging included".to_owned(),
            "checkout fix \u{2713} \u{b7} staging not kept: base.txt \u{b7} u undoes it, staging included".to_owned(),
            "staging not kept: base.txt \u{b7} u undoes it, staging included".to_owned(),
        ]
    );
    assert_eq!(
        staging_not_kept_rungs("checkout fix", &paths(&["a.txt", "b.txt", "c.txt"])),
        vec![
            "checkout fix \u{2713} \u{b7} your edits are back \u{b7} staging not kept: a.txt +2 more \u{b7} u undoes it, staging included".to_owned(),
            "checkout fix \u{2713} \u{b7} staging not kept: a.txt +2 more \u{b7} u undoes it, staging included".to_owned(),
            "staging not kept: a.txt +2 more \u{b7} u undoes it, staging included".to_owned(),
        ]
    );
}

/// Rung 1's text after `✓ · ` is graft-ops' own words, so the two cannot
/// drift.
#[test]
fn staging_not_kept_words_match_graft_ops() {
    for names in [&["f.txt"][..], &["a.txt", "b.txt", "c.txt"][..]] {
        let paths = paths(names);
        let rungs = staging_not_kept_rungs("checkout fix", &paths);
        let tail = rungs[0]
            .strip_prefix("checkout fix \u{2713} \u{b7} ")
            .expect("rung 1 leads with the done words");
        assert_eq!(tail, AfterState::StagingNotKept { paths }.to_string());
    }
}

/// The status keeps the key half apart, so `reason · keys` is the rung
/// that fits the row: rung 1 at 187, rung 2 at 80, rung 3 at 60.
#[test]
fn staging_not_kept_status_picks_the_rung_that_fits() {
    let files = paths(&["base.txt"]);
    let rungs = staging_not_kept_rungs("checkout fix", &files);
    for (columns, rung) in [(187, 0), (80, 1), (60, 2)] {
        let Status::Sticky { reason, keys } =
            staging_not_kept_status("checkout fix", &files, columns)
        else {
            panic!("sticky");
        };
        let keys = keys.expect("the undo key");
        assert_eq!(format!("{reason} \u{b7} {keys}"), rungs[rung], "{columns}");
    }
}

/// `operated` takes it as `Done` takes it — the commit editor closed, the
/// landing owed — with the sticky line, never `could not confirm`.
#[test]
fn operated_takes_staging_not_kept_as_done_with_its_line() {
    let mut state = AppState::default();
    state.size.columns = 187;
    let mut next = 300;
    let mut next_id = move || {
        next += 1;
        JobId::new(next)
    };
    let mut commands = Vec::new();
    let commit = Operation::Commit(CommitParams {
        message: "m".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
    });
    launch(&mut state, &mut commands, commit, &mut next_id).expect("no write is running");
    let id = state.operating.expect("launched");
    let _refresh = operated(
        &mut state,
        id,
        Operated::StagingNotKept {
            applied: applied("checkout fix", Refresh::Everything),
            paths: paths(&["base.txt"]),
        },
        &mut next_id,
    );
    assert_eq!(
        state.status,
        staging_not_kept_status("checkout fix", &paths(&["base.txt"]), 187)
    );
    assert!(state.land_on_head, "lands as Done does");
    assert_eq!(state.lands, None);
}
