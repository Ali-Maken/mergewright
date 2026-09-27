//! `update::undo` tests for edits set aside (m6-conflicts/74, moved out of
//! the inline module by /84, review m6-final-fixes-74 m7): the relaunch line
//! naming where the edits wait, and the landing keeping a done line's
//! `also` when the banner takes the keys.

use super::*;

/// m6-conflicts/74: Graft killed mid-rebase with edits set aside; the
/// relaunch's interrupted line names the stash they wait in, by its
/// position in the list now. Once the stash is gone from the list (he
/// put them back), the clause goes too.
#[test]
fn relaunch_after_a_kill_names_the_stash_the_edits_are_in() {
    let stash = graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes");
    let other = graft_core::oid::Oid::from_bytes(&[0x11; 20]).expect("20 bytes");
    let mut entry = torn_entry(0, "rebase main onto origin/main");
    entry.put_back = Some(graft_core::journal::PutBack {
        stash,
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let journal = JournalState {
        entries: Loading::Ready(Arc::new(JournalRead {
            entries: vec![entry],
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        })),
        ..JournalState::default()
    };
    let with_stashes = |stashes: &[(graft_core::oid::Oid, usize)]| {
        let main = graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec())
            .expect("a valid ref name");
        AppState {
            journal: journal.clone(),
            refs: Some(Arc::new(
                crate::refs::badges(
                    &[(main.clone(), other)],
                    &graft_git::repo::HeadState::Branch(main),
                    &std::collections::HashMap::new(),
                    stashes,
                    |_, _| Err("no upstreams in this fixture".to_owned()),
                )
                .expect("badges builds from plain data"),
            )),
            ..AppState::default()
        }
    };

    let listed = with_stashes(&[(other, 0), (stash, 1)]);
    assert_eq!(
        interrupted_notice(&listed).as_deref(),
        Some(
            "an operation was interrupted: rebase main onto origin/main \u{b7} edits in stash@{1} \u{b7} J to inspect"
        )
    );
    let gone = with_stashes(&[(other, 0)]);
    assert_eq!(
        interrupted_notice(&gone).as_deref(),
        Some("an operation was interrupted: rebase main onto origin/main \u{b7} J to inspect")
    );
}

/// 30c: the launch that closed the entry read where the edits were
/// (`Recovered::set_aside`); the notice says it in the status line's words,
/// and reads no list to guess. An entry that launch did not close keeps
/// ` · edits in stash@{n}`.
#[test]
fn the_notice_words_where_recovery_found_the_edits() {
    let stash = graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes");
    let mut entry = torn_entry(0, "checkout other");
    entry.put_back = Some(graft_core::journal::PutBack {
        stash,
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let id = entry.id.clone();
    let main =
        graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name");
    let state = |edits: Vec<(graft_core::journal::EntryId, graft_ops::EditsAt)>| AppState {
        journal: JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries: vec![entry.clone()],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            })),
            ..JournalState::default()
        },
        interrupted_edits: edits,
        refs: Some(Arc::new(
            crate::refs::badges(
                &[(main.clone(), stash)],
                &graft_git::repo::HeadState::Branch(main.clone()),
                &std::collections::HashMap::new(),
                &[(stash, 0)],
                |_, _| Err("no upstreams in this fixture".to_owned()),
            )
            .expect("badges builds from plain data"),
        )),
        ..AppState::default()
    };
    let back = state(vec![(
        id.clone(),
        graft_ops::EditsAt::Back { copy: Some(0) },
    )]);
    assert_eq!(
        interrupted_notice(&back).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} your edits are back \u{b7} a copy stays in stash@{0} \u{b7} J to inspect"
        )
    );
    let in_stash = state(vec![(id, graft_ops::EditsAt::Stash { n: 0 })]);
    assert_eq!(
        interrupted_notice(&in_stash).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} your edits are in stash@{0} \u{b7} S p puts them back \u{b7} J to inspect"
        )
    );
    // Another entry's: this one was closed by an earlier launch.
    let other = state(vec![(
        graft_core::journal::EntryId::new(9, [9; 10]),
        graft_ops::EditsAt::Back { copy: Some(0) },
    )]);
    assert_eq!(
        interrupted_notice(&other).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} edits in stash@{0} \u{b7} J to inspect"
        )
    );
}

/// Review m6-final-fixes-74 M4: `land_after_undo` replaces the done line's
/// `keys` with the banner's continue words and leaves `also` alone — a fact
/// is not a key, so the key-driven landing cannot drop it.
#[test]
fn the_landing_keeps_also_when_the_banner_takes_the_keys() {
    let (mut state, read) = finish_undone(AppState::default());
    if let Status::Done { also, .. } = &mut state.status {
        *also = Some("your edits are back".to_owned());
    }
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
    assert!(state.banner_focus, "the banner has the keyboard");
    assert_eq!(
        done_line_of(&state.status),
        "undo: finish merge of feature \u{2713} \u{b7} your edits are back \u{b7} c finishes the merge"
    );
}
