//! A put-back's line and tags (m7-power-tools/24-a).

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_ops::operation::{FilesEffect, RecoverParams, RecoverTarget};
use graft_ui::graph_screen::RowTag;

use super::{done_status, files_words, preview_lines, preview_title, put_back_tags};
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

// ── the put-back box (24-a2) ────────────────────────────────────────────

/// J1's names: `main` back from oid 3 to oid 4.
fn subjects(oid: Oid) -> Option<String> {
    match oid.as_bytes().first() {
        Some(3) => Some("Add metrics endpoint".to_owned()),
        Some(4) => Some("Add rate limiter".to_owned()),
        Some(7) => Some("Try it".to_owned()),
        _ => None,
    }
}

fn changes(come_back: &[&str], go: &[&str], set_aside: usize) -> FilesEffect {
    FilesEffect::Changes {
        come_back: come_back.iter().map(PathBuf::from).collect(),
        go: go.iter().map(PathBuf::from).collect(),
        set_aside,
        replaced: Vec::new(),
        still_staged: Vec::new(),
    }
}

fn edit_to(paths: &[&str]) -> RecoverParams {
    params(RecoverTarget::Files {
        entry: graft_core::journal::EntryId::new(1, [0; 10]),
        backup_tree: oid(8),
        paths: paths.iter().map(PathBuf::from).collect(),
    })
}

#[test]
fn the_put_back_box_says_now_and_after() {
    assert_eq!(
        preview_title(&move_main(), subjects),
        "Preview: put main back to 0404040 \"Add rate limiter\""
    );
    let files = changes(&["src/api/limit.rs"], &[], 1);
    assert_eq!(
        preview_lines(&move_main(), Some(&files), subjects),
        vec![
            "now       main \u{2192} 0303030  Add metrics endpoint",
            "after y   main \u{2192} 0404040  Add rate limiter",
            "your files: src/api/limit.rs comes back \u{b7} 1 unsaved file set aside, put back after",
            "u undoes it",
        ]
    );
}

#[test]
fn the_box_says_unchanged_when_not_checked_out() {
    let files = FilesEffect::Unchanged {
        not_checked_out: "main isn't checked out".to_owned(),
    };
    let lines = preview_lines(&move_main(), Some(&files), subjects);
    assert_eq!(
        lines.get(2).map(String::as_str),
        Some("your files: unchanged (main isn't checked out)")
    );
}

#[test]
fn a_make_again_box_says_the_branch_is_gone() {
    let recreate = params(RecoverTarget::Recreate {
        name: "spike".to_owned(),
        at: oid(7),
    });
    assert_eq!(
        preview_title(&recreate, subjects),
        "Preview: make spike again at 0707070 \"Try it\""
    );
    let files = FilesEffect::Unchanged {
        not_checked_out: "spike isn't checked out".to_owned(),
    };
    assert_eq!(
        preview_lines(&recreate, Some(&files), subjects),
        vec![
            "now       spike is gone",
            "after y   spike \u{2192} 0707070  Try it",
            "your files: unchanged (spike isn't checked out)",
            "u undoes it",
        ]
    );
}

#[test]
fn a_file_put_back_box_says_only_your_files() {
    assert_eq!(
        preview_title(&edit_to(&["src/a.rs"]), subjects),
        "Preview: put back your edit to src/a.rs"
    );
    assert_eq!(
        preview_title(&edit_to(&["src/a.rs", "b", "c"]), subjects),
        "Preview: put back your edits to src/a.rs +2 more"
    );
    let files = changes(&["src/a.rs"], &[], 0);
    assert_eq!(
        preview_lines(&edit_to(&["src/a.rs"]), Some(&files), subjects),
        vec!["your files: src/a.rs comes back", "u undoes it"]
    );
}

#[test]
fn the_box_names_the_sha_when_the_subject_is_unknown() {
    assert_eq!(
        preview_title(&move_main(), |_| None),
        "Preview: put main back to 0404040"
    );
    assert_eq!(
        preview_lines(&move_main(), None, |_| None),
        vec![
            "now       main \u{2192} 0303030",
            "after y   main \u{2192} 0404040",
            "u undoes it",
        ]
    );
}

#[test]
fn files_words_joins_the_clauses_that_apply() {
    let cases: [(FilesEffect, &str); 8] = [
        (changes(&["a"], &[], 0), "your files: a comes back"),
        (
            changes(&["a", "b", "c"], &[], 0),
            "your files: a +2 more come back",
        ),
        (changes(&[], &["a"], 0), "your files: a goes"),
        (changes(&[], &["a", "b"], 0), "your files: a +1 more go"),
        (
            changes(&[], &[], 1),
            "your files: 1 unsaved file set aside, put back after",
        ),
        (
            changes(&[], &[], 3),
            "your files: 3 unsaved files set aside, put back after",
        ),
        (
            changes(&["a"], &["b"], 2),
            "your files: a comes back \u{b7} b goes \u{b7} 2 unsaved files set aside, put back after",
        ),
        (changes(&[], &[], 0), "your files: unchanged"),
    ];
    for (effect, words) in cases {
        assert_eq!(files_words(&effect), words, "{effect:?}");
    }
}

/// 30o R1: a file put-back over the file as it is now says so before `y`.
/// Clause order: come back · go · replaced · set aside.
#[test]
fn files_words_names_what_is_replaced() {
    let replacing = |replaced: &[&str]| FilesEffect::Changes {
        come_back: vec![PathBuf::from("src/a.rs")],
        go: vec![],
        set_aside: 0,
        replaced: replaced.iter().map(PathBuf::from).collect(),
        still_staged: Vec::new(),
    };
    assert_eq!(
        files_words(&replacing(&["src/a.rs"])),
        "your files: src/a.rs comes back \u{b7} what's in src/a.rs now is replaced"
    );
    assert_eq!(
        files_words(&replacing(&["src/a.rs", "src/b.rs", "src/c.rs"])),
        "your files: src/a.rs comes back \u{b7} what's in src/a.rs +2 more now is replaced"
    );
    let every = FilesEffect::Changes {
        come_back: vec![PathBuf::from("a")],
        go: vec![PathBuf::from("b")],
        set_aside: 1,
        replaced: vec![PathBuf::from("c")],
        still_staged: Vec::new(),
    };
    assert_eq!(
        files_words(&every),
        "your files: a comes back \u{b7} b goes \u{b7} what's in c now is replaced \u{b7} 1 unsaved file set aside, put back after"
    );
}

/// 30p R2: a replaced path his index holds apart from `HEAD` stays staged,
/// and the box says so. Clause order: come back · go · replaced · still
/// staged · set aside.
#[test]
fn files_words_says_a_staged_path_stays_staged() {
    let staged = |paths: &[&str]| FilesEffect::Changes {
        come_back: vec![PathBuf::from("src/a.rs")],
        go: vec![],
        set_aside: 0,
        replaced: paths.iter().map(PathBuf::from).collect(),
        still_staged: paths.iter().map(PathBuf::from).collect(),
    };
    assert_eq!(
        files_words(&staged(&["src/a.rs"])),
        "your files: src/a.rs comes back \u{b7} what's in src/a.rs now is replaced \u{b7} src/a.rs stays staged as you had it"
    );
    assert_eq!(
        files_words(&staged(&["src/a.rs", "src/b.rs", "src/c.rs"])),
        "your files: src/a.rs comes back \u{b7} what's in src/a.rs +2 more now is replaced \u{b7} src/a.rs +2 more stay staged as you had them"
    );
    let every = FilesEffect::Changes {
        come_back: vec![PathBuf::from("a")],
        go: vec![PathBuf::from("b")],
        set_aside: 1,
        replaced: vec![PathBuf::from("c")],
        still_staged: vec![PathBuf::from("c")],
    };
    assert_eq!(
        files_words(&every),
        "your files: a comes back \u{b7} b goes \u{b7} what's in c now is replaced \u{b7} c stays staged as you had it \u{b7} 1 unsaved file set aside, put back after"
    );
}

#[test]
fn no_files_line_when_the_plan_has_none() {
    let lines = preview_lines(&move_main(), None, subjects);
    assert!(
        lines.iter().all(|line| !line.starts_with("your files")),
        "{lines:?}"
    );
    assert_eq!(lines.last().map(String::as_str), Some("u undoes it"));
}
