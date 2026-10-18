//! After the last file: the status line's words by operation. Split from
//! `file_list.rs` to keep it under the size ratchet.

use super::*;

/// `⏎`'s stage landing on the last file not done.
fn last_file_lands(kind: Option<InProgress>) -> AppState {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.in_progress = kind.map(|kind| crate::update::resolver::fixtures::operation(kind, None));
    state.banner_focus = false;
    let staged = JobId::new(4_500);
    state.resolver.as_mut().expect("open").staging = Some(staged);
    let mut commands = Vec::new();
    let mut ids = 4_500;
    resolved_advance(&mut state, &mut commands, staged, &mut || {
        ids += 1;
        JobId::new(ids)
    });
    state
}

/// The resolver stays on the last file (m6-conflicts/31, P8): the status
/// line says `✓ <file> done  every clash is solved · y reviews the merge`. Nothing
/// opens on its own (ease-pass/05b dropped m6-conflicts/60's owed open).
#[test]
fn after_the_last_file_of_a_merge_the_status_says_every_file_done() {
    let state = last_file_lands(Some(InProgress::Merge));
    assert_eq!(super::super::open_path(&state), Path::new("conflicted.rs"));
    assert!(!state.banner_focus);
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.notice, None, "no status read out: nothing to check");
    assert_eq!(
        state.status,
        Status::Done {
            what: "conflicted.rs done".to_owned(),
            also: None,
            keys: Some("every clash is solved · y reviews the merge".to_owned()),
        }
    );
}

/// Rewritten for m6-conflicts/60 (was
/// `after_the_last_file_of_a_rebase_the_notice_says_c_continues`): no
/// summary is owed — `c` writes — so the status line names the key.
#[test]
fn after_the_last_file_of_a_rebase_the_status_says_c_continues() {
    for (kind, keys) in [
        (
            InProgress::Rebase,
            "every file done · c continues the rebase",
        ),
        (
            InProgress::CherryPick,
            "every file done · c continues the cherry-pick",
        ),
        (
            InProgress::Revert,
            "every file done · c continues the revert",
        ),
    ] {
        let state = last_file_lands(Some(kind));
        assert_eq!(super::super::open_path(&state), Path::new("conflicted.rs"));
        assert!(!state.banner_focus, "{kind:?}");
        let open = state.resolver.as_ref().expect("open");
        assert_eq!(open.notice, None, "{kind:?}");
        assert_eq!(
            state.status,
            Status::Done {
                what: "conflicted.rs done".to_owned(),
                also: None,
                keys: Some(keys.to_owned()),
            },
            "{kind:?}"
        );
    }
}

#[test]
fn after_the_last_stash_file_the_status_says_every_file_done() {
    let state = last_file_lands(None);
    assert!(state.resolver.is_none());
    assert!(!state.banner_focus, "no banner: back on the graph");
    assert_eq!(state.status, Status::done("every file done"));
}
