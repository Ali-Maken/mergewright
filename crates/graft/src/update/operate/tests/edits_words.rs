//! Where a set-aside's edits are, as the status line says it (m6-conflicts/74,
//! moved here and reworded by /84): a clean put-back rides `also` after the
//! tick; a failure is sticky, git's own failure first, then the one place
//! `graft_ops` says the edits are (`EditsAt`) — never a stash-list read of
//! the app's own.

use graft_ops::EditsAt;
use graft_ui::graph_screen::hints::fit_status_line;

use super::*;

/// The `Stash{n}` put-back — the one sticky shape 74 shipped.
fn put_back() -> Operation {
    Operation::DropPutBack {
        stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
        why: graft_core::journal::PutBackWhy::SetAside,
    }
}

fn failed_with(state: &mut AppState, error: &str, edits: Option<EditsAt>) {
    let mut commands = Vec::new();
    launch(state, &mut commands, put_back(), &mut ids()).expect("nothing else is running");
    let id = state.operating.expect("launched");
    operated(
        state,
        id,
        Operated::Failed {
            error: error.to_owned(),
            refresh: Some(Refresh::Everything),
            remote: None,
            refused: None,
            edits,
        },
        &mut ids(),
    );
}

/// m6-conflicts/74: a rebase that set edits aside answered `Done` — the
/// put-back ran clean. Its own line, the tick, then `your edits are back`,
/// carried in `also` (not `keys`, which a key-driven path replaces — review
/// m6-final-fixes-74 M4). The same rebase on a clean tree words as before.
#[test]
fn a_clean_put_back_says_your_edits_are_back() {
    let mut done = applied("rebase main onto origin/main", Refresh::Everything);
    done.entry.put_back = Some(graft_core::journal::PutBack {
        stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let mut state = AppState::default();
    let mut commands = Vec::new();
    launch(&mut state, &mut commands, put_back(), &mut ids()).expect("nothing else is running");
    let id = state.operating.expect("launched");
    operated(&mut state, id, Operated::Done(done), &mut ids());
    assert_eq!(
        state.status,
        Status::Done {
            what: "rebase main onto origin/main".to_owned(),
            also: Some("your edits are back".to_owned()),
            keys: None,
        }
    );
    assert_eq!(
        crate::view::status::done_line(
            "rebase main onto origin/main",
            Some("your edits are back"),
            None
        ),
        "rebase main onto origin/main \u{2713} \u{b7} your edits are back"
    );

    let mut clean = AppState::default();
    let mut commands = Vec::new();
    launch(&mut clean, &mut commands, put_back(), &mut ids()).expect("nothing else is running");
    let id = clean.operating.expect("launched");
    let plain = applied("rebase main onto origin/main", Refresh::Everything);
    operated(&mut clean, id, Operated::Done(plain), &mut ids());
    assert_eq!(clean.status, Status::done("rebase main onto origin/main"));
}

/// m6-conflicts/74, /84: the put-back failed for a reason other than a
/// clash and the copy is listed: sticky, git's failure then where the edits
/// are and the key that brings them back.
#[test]
fn a_put_back_failure_names_the_stash_and_s_p() {
    let mut state = AppState::default();
    failed_with(
        &mut state,
        "checkout fix: error: cannot lock ref",
        Some(EditsAt::Stash { n: 0 }),
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "checkout fix: error: cannot lock ref".to_owned(),
            keys: Some("your edits are in stash@{0} \u{b7} S p puts them back".to_owned()),
        }
    );
}

/// Review m6-final-fixes-74 M1: every place `graft_ops` can say the edits
/// are gets its own words, and only `Stash` offers `S p` — the edits back
/// in the tree must never be told to be put back again.
#[test]
fn each_place_the_edits_can_be_is_worded() {
    let failure = "rebase main onto origin/main: error: cannot lock ref";
    for (at, clause) in [
        (
            EditsAt::Stash { n: 2 },
            "your edits are in stash@{2} \u{b7} S p puts them back",
        ),
        (
            EditsAt::Back { copy: Some(1) },
            "your edits are back \u{b7} a copy stays in stash@{1}",
        ),
        (
            EditsAt::Back { copy: None },
            "your edits are back \u{b7} a copy may still be in S",
        ),
        (
            EditsAt::Backup { copy: Some(0) },
            "u restores the backup \u{b7} a copy is in stash@{0}",
        ),
        (EditsAt::Backup { copy: None }, "u restores the backup"),
    ] {
        let mut state = AppState::default();
        failed_with(&mut state, failure, Some(at));
        assert_eq!(
            state.status,
            Status::Sticky {
                reason: failure.to_owned(),
                keys: Some(clause.to_owned()),
            },
            "{at:?}"
        );
        if !matches!(at, EditsAt::Stash { .. }) {
            assert!(!clause.contains("S p"), "{at:?} offers S p: {clause}");
        }
    }
}

/// Review m6-final-fixes-74 M1: at 80 columns the failure is cut and the
/// clause stays whole — it is the `keys` half of the line.
#[test]
fn the_clause_is_kept_whole_at_80() {
    let failure = "rebase main onto origin/main: error: cannot lock ref 'refs/heads/main': \
                   unable to create lock file";
    for at in [
        EditsAt::Stash { n: 0 },
        EditsAt::Back { copy: Some(0) },
        EditsAt::Backup { copy: Some(0) },
    ] {
        let Status::Sticky {
            reason,
            keys: Some(keys),
        } = edits_status(failure, at)
        else {
            panic!("a failure with edits is sticky with keys");
        };
        let line = fit_status_line(&reason, Some(&keys), 80);
        assert!(line.chars().count() <= 80, "{line}");
        assert!(line.ends_with(&format!(" \u{b7} {keys}")), "{line}");
        assert!(line.contains('\u{2026}'), "the failure is cut: {line}");
    }
}

/// 24-0: the last clashed file done launches the drop with the clash's
/// why, so the journal row can name a set-aside's copy as the done line
/// does. A set-aside clash → `why: SetAside`; a pop clash → `why: Pop`.
#[test]
fn the_drop_after_a_clash_carries_its_why() {
    use graft_core::journal::PutBackWhy;
    use graft_ops::clash::StashClash;

    use crate::update::{ClashState, LiveOrigin};

    for why in [PutBackWhy::SetAside, PutBackWhy::Pop] {
        let entry = crate::update::clash::tests::pop_entry();
        let stash = entry.pins[0];
        let mut state = AppState {
            clash: Some(ClashState {
                clash: StashClash {
                    entry: entry.id.clone(),
                    stash,
                    index: Some(0),
                    why,
                },
                origin: LiveOrigin::ThisRun,
            }),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        crate::update::resolver::close_after_last(&mut state, &mut commands, &mut ids());
        let launched: Vec<&Operation> = commands
            .iter()
            .filter_map(|command| match command {
                Command::Operate(_, op) => Some(op.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(
            launched,
            [&Operation::DropPutBack { stash, why }],
            "{why:?}"
        );
    }
}
