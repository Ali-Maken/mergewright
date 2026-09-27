//! `recover_for`: what `⏎` puts back, per row.

use std::collections::BTreeMap;

use graft_core::journal::{HeadRef, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_git::status::WorkingTreeStatus;
use graft_git::version::GitVersion;

use super::{entry, list, list_with_edits, lost, oid, record, run};
use crate::journal_list::{JournalList, ListRow, LostEdit, Recover, recover_for, recover_scope};
use crate::operation::{NoPutBack, RecoverParams, RecoverTarget, Refusal, RepoFacts};

/// The repository now, with these branches (short name, tip).
fn facts(branches: &[(&str, u8)]) -> RepoFacts {
    let refs: BTreeMap<String, Option<Oid>> = branches
        .iter()
        .map(|(name, tip)| (format!("refs/heads/{name}"), Some(oid(*tip))))
        .collect();
    RepoFacts {
        bare: false,
        head: HeadRef::Branch("refs/heads/main".to_owned()),
        in_progress: None,
        status: WorkingTreeStatus { entries: vec![] },
        stashes: vec![],
        git: GitVersion {
            major: 2,
            minor: 54,
            patch: 0,
        },
        remotes: vec![],
        upstream: None,
        refs,
        markers: vec![],
        incoming: None,
        edit_stop: None,
    }
}

fn row_of(list: &JournalList, wanted: impl Fn(&ListRow) -> bool) -> usize {
    list.rows
        .iter()
        .position(wanted)
        .unwrap_or_else(|| panic!("no such row in {list:?}"))
}

fn lost_row(list: &JournalList) -> usize {
    row_of(list, |row| matches!(row, ListRow::Lost { .. }))
}

/// The shell's `reset --hard HEAD~1` on `main`, D (4) lost under it.
fn shell_reset() -> JournalList {
    let outside = [
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 3, 4, 1004, "commit: D"),
    ];
    list(&[], &outside, Some(&lost(&[run(4, 0)])), &[])
}

#[test]
fn checked_out_main_is_moved_back() {
    let list = shell_reset();
    let facts = facts(&[("main", 3)]);

    let lost_at = lost_row(&list);
    assert_eq!(
        recover_for(&list, lost_at, &[], &facts),
        Ok(Recover::Run(RecoverParams {
            target: RecoverTarget::MoveRef {
                name: "main".to_owned(),
                to: oid(4),
                expected: oid(3),
            },
            because: "the shell's reset".to_owned(),
        }))
    );
    // The `$` row puts the same ref back to before the step.
    assert_eq!(
        recover_for(&list, 0, &[], &facts),
        Ok(Recover::Run(RecoverParams {
            target: RecoverTarget::MoveRef {
                name: "main".to_owned(),
                to: oid(4),
                expected: oid(3),
            },
            because: "the shell's reset".to_owned(),
        }))
    );
    assert_eq!(
        recover_scope(&list, lost_at).refs[..4],
        [
            "refs/heads/main".to_owned(),
            "refs/remotes/main".to_owned(),
            "refs/tags/main".to_owned(),
            "refs/heads/main-restored".to_owned()
        ]
    );
}

#[test]
fn a_gone_branch_is_recreated() {
    let outside = [
        record("HEAD", 5, 3, 1008, "checkout: moving from spike to main"),
        record("HEAD", 3, 5, 1007, "commit: E"),
    ];
    let list = list(&[], &outside, Some(&lost(&[run(5, 0)])), &["spike"]);

    assert_eq!(
        recover_for(&list, lost_row(&list), &[], &facts(&[("main", 3)])),
        Ok(Recover::Run(RecoverParams {
            target: RecoverTarget::Recreate {
                name: "spike".to_owned(),
                at: oid(5),
            },
            because: "the shell's branch -D".to_owned(),
        }))
    );
    // The switch row itself is the app's switch, not a put-back.
    assert_eq!(
        recover_for(&list, 0, &[], &facts(&[("main", 3)])),
        Err(Refusal::NoPutBack(NoPutBack::Switch {
            back_to: "spike".to_owned()
        }))
    );
}

#[test]
fn moved_on_offers_main_restored() {
    let list = shell_reset();
    // main moved on to 7 since the reset left it at 3.
    let facts = facts(&[("main", 7)]);

    assert_eq!(
        recover_for(&list, lost_row(&list), &[], &facts),
        Ok(Recover::OfferBranch {
            at: oid(4),
            name: "main-restored".to_owned(),
            why: "main moved on since (now 0707070)".to_owned(),
        })
    );
}

#[test]
fn an_existing_restored_name_gets_a_suffix() {
    let list = shell_reset();
    let facts = facts(&[("main", 7), ("main-restored", 1), ("main-restored-2", 1)]);

    let Ok(Recover::OfferBranch { name, .. }) = recover_for(&list, lost_row(&list), &[], &facts)
    else {
        panic!("an offer");
    };
    assert_eq!(name, "main-restored-3");
}

/// Review m7-22 M1, defence in depth: a gone branch is never made again
/// under a name a remote-tracking branch or a tag has. `restored` instead.
#[test]
fn a_name_a_remote_branch_or_tag_has_is_never_made_a_branch() {
    for (other, why) in [
        (
            "refs/remotes/origin/main",
            "origin/main is a remote branch, not one of yours",
        ),
        (
            "refs/tags/origin/main",
            "origin/main is a tag, not a branch",
        ),
    ] {
        let outside = [
            record(
                "refs/heads/origin/main",
                4,
                3,
                1005,
                "reset: moving to HEAD~1",
            ),
            record("refs/heads/origin/main", 3, 4, 1004, "commit: D"),
        ];
        let list = list(&[], &outside, Some(&lost(&[run(4, 0)])), &[]);
        let mut facts = facts(&[("main", 1)]);
        facts.refs.insert(other.to_owned(), Some(oid(2)));
        assert!(
            recover_scope(&list, 0).refs.contains(&other.to_owned()),
            "the scope reads {other}"
        );

        assert_eq!(
            recover_for(&list, 0, &[], &facts),
            Ok(Recover::OfferBranch {
                at: oid(4),
                name: "restored".to_owned(),
                why: why.to_owned(),
            }),
            "the $ row, against {other}"
        );
        assert_eq!(
            recover_for(&list, lost_row(&list), &[], &facts),
            Ok(Recover::OfferBranch {
                at: oid(4),
                name: "restored".to_owned(),
                why: why.to_owned(),
            }),
            "the ✗ row, against {other}"
        );
    }
}

#[test]
fn a_head_only_commit_offers_restored() {
    let detached = "03".repeat(20);
    let outside = [
        record(
            "HEAD",
            5,
            3,
            1008,
            &format!("checkout: moving from {detached} to main"),
        ),
        record("HEAD", 3, 5, 1007, "commit: E"),
    ];
    let list = list(&[], &outside, Some(&lost(&[run(5, 0)])), &[]);

    assert_eq!(
        recover_for(
            &list,
            lost_row(&list),
            &[],
            &facts(&[("main", 3), ("restored", 1)])
        ),
        Ok(Recover::OfferBranch {
            at: oid(5),
            name: "restored-2".to_owned(),
            why: "no branch held it, only HEAD".to_owned(),
        })
    );
}

#[test]
fn a_graft_row_is_refused() {
    let fetch = entry(1000, OperationKind::Fetch, "fetch origin", &[]);
    let list = list(std::slice::from_ref(&fetch), &[], None, &[]);

    assert_eq!(
        recover_for(&list, 0, &[fetch], &facts(&[])),
        Err(Refusal::NoPutBack(NoPutBack::Graft))
    );
    assert_eq!(
        recover_for(&list, 9, &[], &facts(&[])),
        Err(Refusal::NoPutBack(NoPutBack::Nothing))
    );
}

#[test]
fn a_lost_edit_recovers_its_files() {
    let discard = entry(1000, OperationKind::Discard, "discard src/a.rs", &[]);
    let mut undo: JournalEntry = entry(1005, OperationKind::Undo, "undo: discard src/a.rs", &[]);
    undo.reverses = Some(discard.id.clone());
    let edit = LostEdit {
        entry: undo.id.clone(),
        backup_tree: oid(8),
        paths: vec!["src/a.rs".into()],
    };
    let entries = [discard, undo.clone()];
    let list = list_with_edits(&entries, &[], None, &[], &[edit]);

    let row = row_of(&list, |row| matches!(row, ListRow::LostEdit { .. }));
    assert_eq!(
        recover_for(&list, row, &entries, &facts(&[])),
        Ok(Recover::Run(RecoverParams {
            target: RecoverTarget::Files {
                entry: undo.id,
                backup_tree: oid(8),
                paths: vec!["src/a.rs".into()],
            },
            because: r#"undo of "discard src/a.rs""#.to_owned(),
        }))
    );
}

/// A lost commit a branch holds again by the time `⏎` is pressed.
#[test]
fn a_commit_held_again_is_refused() {
    let list = shell_reset();
    assert_eq!(
        recover_for(&list, lost_row(&list), &[], &facts(&[("main", 4)])),
        Err(Refusal::HeldAgain {
            commit: oid(4),
            by: "main".to_owned()
        })
    );
}

/// Under a Graft reset, the branch is expected where the entry left it.
#[test]
fn under_a_graft_row_main_is_moved_back_from_its_after_value() {
    let reset = entry(
        1005,
        OperationKind::Reset,
        "reset main to 0303030",
        &[("refs/heads/main", 4, 3)],
    );
    let entries = [reset];
    let list = list(&entries, &[], Some(&lost(&[run(4, 0)])), &[]);

    assert_eq!(
        recover_for(&list, lost_row(&list), &entries, &facts(&[("main", 3)])),
        Ok(Recover::Run(RecoverParams {
            target: RecoverTarget::MoveRef {
                name: "main".to_owned(),
                to: oid(4),
                expected: oid(3),
            },
            because: "reset main to 0303030".to_owned(),
        }))
    );
}
