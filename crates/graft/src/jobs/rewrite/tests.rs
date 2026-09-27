//! The put-in's words and row offsets, over plain data. What `plan` and the
//! map decide is graft-ops' and proved against real git there; the walks
//! in `tests/it/m7_walk/amend_into.rs` run this job end to end.

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_ops::absorb::{AbsorbMap, StagedHunk, StaysWhy};
use graft_ops::operation::Tip;
use graft_ops::rewrite::{BranchLine, RewriteParams, TodoAction, TodoLine};

use super::*;

fn oid(n: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0] = n;
    Oid::from_sha1(bytes)
}

fn row(action: TodoAction, n: u8) -> TodoLine {
    TodoLine {
        action,
        commit: oid(n),
        subject: format!("commit {n}"),
    }
}

fn params(todo: Vec<TodoLine>, made: Vec<Oid>) -> RewriteParams {
    RewriteParams {
        head: Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(9),
        },
        onto: oid(0),
        todo,
        line: BranchLine::default(),
        messages: BTreeMap::new(),
        made,
        index_base: None,
    }
}

fn hunk(path: &str, old: std::ops::Range<u32>, new: std::ops::Range<u32>) -> StagedHunk {
    StagedHunk {
        path: PathBuf::from(path),
        old,
        new,
    }
}

/// `a` on the third commit back: its row, then the made fixup, then two
/// kept rows above — two first-parent steps below `HEAD`.
#[test]
fn a_targets_offset_counts_the_kept_rows_above_it() {
    let todo = vec![
        row(TodoAction::Pick, 1),
        row(TodoAction::Fixup, 50),
        row(TodoAction::Pick, 2),
        row(TodoAction::Pick, 3),
    ];
    assert_eq!(offsets(&params(todo, vec![oid(50)])), vec![2]);
}

/// Absorb into two commits; a target the plan drops (its pieces undo it)
/// keeps no row and gets no offset; an `R`-style fixup of his own is not a
/// target.
#[test]
fn absorb_offsets_skip_a_dropped_target_and_his_own_fixups() {
    let todo = vec![
        row(TodoAction::Pick, 1),
        row(TodoAction::Fixup, 50),
        row(TodoAction::Drop, 2),
        row(TodoAction::Pick, 3),
        row(TodoAction::Fixup, 4),
        row(TodoAction::Pick, 5),
        row(TodoAction::Fixup, 51),
    ];
    assert_eq!(offsets(&params(todo, vec![oid(50), oid(51)])), vec![2, 0]);
}

#[test]
fn numstat_reads_counts_paths_and_binaries() {
    let gains = parse_numstat(b"4\t1\tsrc/auth.rs\x002\t0\tsrc/cb.rs\x00-\t-\tlogo.png\x00");
    assert_eq!(
        gains,
        vec![
            Gain {
                path: "src/auth.rs".to_owned(),
                counts: Some((4, 1)),
            },
            Gain {
                path: "src/cb.rs".to_owned(),
                counts: Some((2, 0)),
            },
            Gain {
                path: "logo.png".to_owned(),
                counts: None,
            },
        ]
    );
}

/// Note R2's first box: the title names the target, the `gains` row lists
/// each staged file's counts; the landing names the subject.
#[test]
fn amend_into_words_name_the_target_and_what_it_gains() {
    let gains = parse_numstat(b"4\t1\tsrc/auth.rs\x002\t0\tsrc/cb.rs\x00");
    let target = Oid::from_hex("3b2a1f0000000000000000000000000000000000").expect("hex");
    let (done, shown) = amend_into_words(target, "Add OAuth callback", &gains);
    assert_eq!(
        shown.title,
        "put your staged changes into 3b2a1f0 \"Add OAuth callback\""
    );
    assert_eq!(shown.confirm, "put it in");
    assert_eq!(
        shown.rows,
        vec![
            "3b2a1f0  Add OAuth callback  gains  src/auth.rs +4 \u{2212}1 \u{b7} src/cb.rs +2"
                .to_owned()
        ]
    );
    assert_eq!(done.what, "put into \"Add OAuth callback\"");
    assert_eq!(done.also, None);
}

/// Note R2's second box: placed rows `→ <sha>  <subject>`, stays `stays
/// staged · <why>`, columns aligned; the landing counts hunks, commits and
/// stays.
#[test]
fn absorb_words_map_each_hunk_and_count_the_landing() {
    let a = Oid::from_hex("3b2a1f0000000000000000000000000000000000").expect("hex");
    let b = Oid::from_hex("7f6e5d4000000000000000000000000000000000").expect("hex");
    let map = AbsorbMap {
        placed: vec![
            (hunk("src/auth.rs", 40..45, 40..45), a),
            (hunk("src/auth.rs", 88..91, 88..91), b),
        ],
        stays: vec![(hunk("src/cfg.rs", 3..4, 3..4), StaysWhy::NoCommitChangedIt)],
    };
    let subjects = BTreeMap::from([(a, "Add OAuth callback"), (b, "Handle expired tokens")]);
    let (done, shown) = absorb_words(&map, &subjects);
    assert_eq!(
        shown.title,
        "put staged changes into the commits they belong to"
    );
    assert_eq!(shown.confirm, "put them in");
    assert_eq!(
        shown.rows,
        vec![
            "src/auth.rs  lines 40\u{2013}44  \u{2192} 3b2a1f0  Add OAuth callback".to_owned(),
            "src/auth.rs  lines 88\u{2013}90  \u{2192} 7f6e5d4  Handle expired tokens".to_owned(),
            "src/cfg.rs   line 3       stays staged \u{b7} no commit on your branch changed these lines"
                .to_owned(),
        ]
    );
    assert_eq!(done.what, "put 2 hunks into 2 commits");
    assert_eq!(done.also.as_deref(), Some("1 stays staged"));
}

#[test]
fn absorb_landing_words_count_one_of_each_in_the_singular() {
    let a = oid(1);
    let map = AbsorbMap {
        placed: vec![(hunk("a.rs", 1..2, 1..2), a)],
        stays: Vec::new(),
    };
    let (done, _) = absorb_words(&map, &BTreeMap::new());
    assert_eq!(done.what, "put 1 hunk into 1 commit");
    assert_eq!(done.also, None, "nothing stays: nothing said");
}

/// A hunk that only removes lines is named by what it removes.
#[test]
fn a_removal_is_named_by_the_lines_it_removes() {
    assert_eq!(lines_words(&hunk("a.rs", 5..8, 5..5)), "lines 5\u{2013}7");
}

/// One row per [`StaysWhy`], each worded (06c and 06e carries): a stay
/// never reads as a bare `stays staged`.
fn stays_row(why: StaysWhy) -> String {
    let map = AbsorbMap {
        placed: Vec::new(),
        stays: vec![(hunk("f.rs", 1..2, 1..2), why)],
    };
    let (done, shown) = absorb_words(&map, &BTreeMap::new());
    assert_eq!(done.also.as_deref(), Some("1 stays staged"));
    shown.rows.into_iter().next().expect("one row")
}

#[test]
fn stays_no_commit_changed_it() {
    assert!(
        stays_row(StaysWhy::NoCommitChangedIt)
            .ends_with("stays staged \u{b7} no commit on your branch changed these lines")
    );
}

#[test]
fn stays_two_commits_changed_it() {
    assert!(
        stays_row(StaysWhy::TwoCommitsChangedIt)
            .ends_with("stays staged \u{b7} more than one commit changed these lines")
    );
}

#[test]
fn stays_new_file() {
    assert!(
        stays_row(StaysWhy::NewFile)
            .ends_with("stays staged \u{b7} a new file \u{b7} no commit has it yet")
    );
}

#[test]
fn stays_before_your_branch() {
    assert!(
        stays_row(StaysWhy::BeforeYourBranch)
            .ends_with("stays staged \u{b7} these lines are older than your branch")
    );
}

#[test]
fn stays_renamed() {
    assert!(
        stays_row(StaysWhy::Renamed).ends_with("stays staged \u{b7} a renamed file stays whole")
    );
}

#[test]
fn stays_submodule() {
    assert!(
        stays_row(StaysWhy::Submodule)
            .ends_with("stays staged \u{b7} a submodule change has no lines to put in")
    );
}

#[test]
fn stays_outside_your_commits() {
    assert!(
        stays_row(StaysWhy::OutsideYourCommits)
            .ends_with("stays staged \u{b7} these lines are older than the commits absorb reaches")
    );
}

/// 06e's ruling, in the planner's words.
#[test]
fn stays_already_pushed() {
    assert!(
        stays_row(StaysWhy::AlreadyPushed)
            .ends_with("stays staged \u{b7} the commit that changed these lines is already pushed")
    );
}

/// 09b A2: `git --version` cost `a` on `HEAD` one spawn (14.5 ms of the
/// 30.2 ms, `docs/perf/09b.txt` phase 2) before the dependents check, on
/// every press. Read once per process, as blame's is.
#[test]
fn the_git_version_is_read_at_most_once_per_process() {
    let first = git_version().expect("git answers --version");
    let second = git_version().expect("git answers --version");
    assert_eq!(first, second);
    // Another test on this thread may have read it first: at most one read.
    assert!(VERSION_READS.with(std::cell::Cell::get) <= 1);
}
