//! A put-back's line and tags (m7-power-tools/24-a).

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_ops::operation::{RecoverParams, RecoverTarget};
use graft_ui::graph_screen::RowTag;

use super::{done_status, put_back_tags};
use crate::update::state::Status;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn params(target: RecoverTarget) -> RecoverParams {
    RecoverParams {
        target,
        because: "reset: moving to HEAD~1".to_owned(),
    }
}

fn move_main() -> RecoverParams {
    params(RecoverTarget::MoveRef {
        name: "main".to_owned(),
        to: oid(4),
        expected: oid(3),
    })
}

fn done(what: &str, also: Option<&str>) -> Status {
    Status::Done {
        what: what.to_owned(),
        also: also.map(str::to_owned),
        keys: Some("u undoes".to_owned()),
    }
}

#[test]
fn a_moved_branch_says_it_has_the_commit_again() {
    let said = done_status(&move_main(), |_| Some("Add retry".to_owned()));
    assert_eq!(said, done("put back", Some("main has \"Add retry\" again")));
}

#[test]
fn a_subject_nobody_holds_is_the_sha() {
    let said = done_status(&move_main(), |_| None);
    assert_eq!(said, done("put back", Some("main has 0404040 again")));
}

#[test]
fn a_recreated_branch_is_back_at_its_commit() {
    let target = RecoverTarget::Recreate {
        name: "spike".to_owned(),
        at: oid(7),
    };
    let said = done_status(&params(target), |_| Some("Try it".to_owned()));
    assert_eq!(said, done("spike is back at \"Try it\"", None));
}

#[test]
fn files_put_back_say_which() {
    let files = |paths: &[&str]| {
        params(RecoverTarget::Files {
            entry: graft_core::journal::EntryId::new(1, [0; 10]),
            backup_tree: oid(8),
            paths: paths.iter().map(PathBuf::from).collect(),
        })
    };
    assert_eq!(
        done_status(&files(&["src/a.rs"]), |_| None),
        done("your edit to src/a.rs is back", None)
    );
    assert_eq!(
        done_status(&files(&["src/a.rs", "b", "c"]), |_| None),
        done("your edits to src/a.rs +2 more are back", None)
    );
}

#[test]
fn a_moved_branch_tags_what_came_back_and_where_it_was() {
    assert_eq!(
        put_back_tags(&move_main()),
        vec![
            (oid(4), RowTag::PutBack),
            (
                oid(3),
                RowTag::WasHere {
                    branch: "main".to_owned()
                }
            ),
        ]
    );
}

#[test]
fn a_recreated_branch_tags_its_commit_and_files_tag_nothing() {
    let recreate = params(RecoverTarget::Recreate {
        name: "spike".to_owned(),
        at: oid(7),
    });
    assert_eq!(put_back_tags(&recreate), vec![(oid(7), RowTag::PutBack)]);
    let files = params(RecoverTarget::Files {
        entry: graft_core::journal::EntryId::new(1, [0; 10]),
        backup_tree: oid(8),
        paths: vec![PathBuf::from("src/a.rs")],
    });
    assert!(put_back_tags(&files).is_empty());
}
