use std::path::Path;

use graft_core::oid::Oid;
use graft_fixtures::Shape;

use super::*;
use crate::diff::{DiffSpec, FileEntry, FileStatus, Hunk, file_diff, list_files};
use crate::fixture::Fixture;
use crate::repo::{RepoHandle, SharedRepo};

fn root_commit(repo_path: &Path) -> Option<Oid> {
    let stdout = graft_fixtures::git::run(repo_path, &["rev-list", "--max-parents=0", "--all"])
        .expect("git rev-list runs");
    let text = String::from_utf8(stdout).expect("git prints hex ids as ascii");
    let first_line = text.lines().next()?;
    Some(Oid::from_hex(first_line.trim()).expect("git prints a full object id"))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len().max(1)).any(|w| w == needle)
}

/// Everything from the hunk's own `@@ -...@@` header onward: the part
/// [`hunk_patch`] shares with git's own `diff-tree -p` output, once the
/// `diff --git`/mode/`---`/`+++` lines built independently are stripped off.
fn hunk_portion(patch: &[u8]) -> &[u8] {
    let marker = b"\n@@ -";
    let start = patch
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("a built patch has a hunk header")
        + 1;
    &patch[start..]
}

/// Checks every hunk of every file in `shape`'s root commit, if it has one
/// and its fixture is available on this machine.
fn check_shape_whole_hunks(shape: Shape) {
    let path = match graft_fixtures::try_borrow(shape) {
        Ok(path) => path,
        Err(why) => {
            assert!(
                why.to_string().contains("not valid UTF-8"),
                "{shape}: {why}"
            );
            return;
        }
    };
    let Some(root) = root_commit(&path) else {
        return;
    };
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let spec = DiffSpec::Commit {
        commit: root,
        against: None,
    };
    let oracle = graft_fixtures::git::run(
        &path,
        &[
            "diff-tree",
            "-p",
            "-M",
            "-C",
            "--no-color",
            "--no-ext-diff",
            "--root",
            &root.to_string(),
        ],
    )
    .expect("git diff-tree runs");

    let list = list_files(&handle, &spec, &crate::diff::DiffOptions::default(), &())
        .expect("list_files succeeds");
    for entry in &list.files {
        let diff = file_diff(
            &handle,
            &spec,
            entry,
            &crate::diff::DiffOptions::default(),
            usize::MAX,
            &(),
        )
        .expect("file_diff succeeds");
        if diff.binary.is_some() {
            continue;
        }
        for hunk in 0..diff.hunks.len() {
            let built = hunk_patch(&diff, hunk, &Selection::WholeHunk)
                .expect("a whole-hunk selection always builds");
            let portion = hunk_portion(&built);
            assert!(
                contains(&oracle, portion),
                "{shape}: hunk {hunk} of {:?} not found verbatim in git's own diff-tree -p output",
                String::from_utf8_lossy(&entry.path)
            );
        }
    }
}

#[test]
fn whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shapes_root_commit() {
    for &shape in Shape::ALL {
        check_shape_whole_hunks(shape);
    }
}

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

#[test]
fn lines_selection_stages_exactly_the_selected_line() {
    let fixture = Fixture::with_two_commits();
    std::fs::write(fixture.root().join("f.txt"), "one\ntwo\nthree\n").expect("file written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("add f.txt");
    std::fs::write(fixture.root().join("f.txt"), "one\ntwoo\nthree\nfour\n")
        .expect("file rewritten");
    let repo = open(&fixture);

    let entry = FileEntry {
        path: b"f.txt".to_vec(),
        old_path: None,
        status: FileStatus::Modified,
        added: None,
        removed: None,
    };
    let diff = file_diff(
        &repo,
        &DiffSpec::Unstaged,
        &entry,
        &crate::diff::DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");
    assert_eq!(diff.hunks.len(), 1, "one hunk covers this whole change");
    let hunk = &diff.hunks[0];
    // Lines, in order: " one", "-two", "+twoo", " three", "+four". Select
    // only the trailing "+four" addition.
    let four_index = hunk
        .lines
        .iter()
        .position(|line| line.kind == LineKind::Added && line.text == b"four")
        .expect("the hunk has a +four line");

    let patch = hunk_patch(&diff, 0, &Selection::Lines(vec![four_index]))
        .expect("a one-line selection builds");

    std::fs::write(fixture.root().join("patch.diff"), &patch).expect("patch written");
    fixture.git(&["apply", "--cached", "patch.diff"]);

    let staged = graft_fixtures::git::run(fixture.root(), &["diff", "--cached", "--no-color"])
        .expect("git diff --cached runs");
    let staged = String::from_utf8_lossy(&staged);
    assert!(staged.contains("+four"), "{staged}");
    assert!(!staged.contains("twoo"), "{staged}");
    assert!(!staged.contains("-two"), "{staged}");
}

fn sample_hunk() -> Hunk {
    Hunk {
        old_start: 1,
        old_lines: 2,
        new_start: 1,
        new_lines: 2,
        heading: Vec::new(),
        lines: vec![
            DiffLine {
                kind: LineKind::Context,
                text: b"unchanged".to_vec(),
                old_no: Some(1),
                new_no: Some(1),
                no_newline_at_eof: false,
            },
            DiffLine {
                kind: LineKind::Removed,
                text: b"old".to_vec(),
                old_no: Some(2),
                new_no: None,
                no_newline_at_eof: false,
            },
            DiffLine {
                kind: LineKind::Added,
                text: b"new".to_vec(),
                old_no: None,
                new_no: Some(2),
                no_newline_at_eof: false,
            },
        ],
    }
}

fn sample_diff(hunks: Vec<Hunk>, binary: Option<crate::diff::BinaryNote>) -> FileDiff {
    FileDiff {
        entry: FileEntry {
            path: b"f.txt".to_vec(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(1),
            removed: Some(1),
        },
        hunks,
        binary,
        truncated: None,
        old_mode: None,
        new_mode: None,
    }
}

#[test]
fn no_such_hunk_is_reported() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(&diff, 1, &Selection::WholeHunk),
        Err(PatchError::NoSuchHunk { hunk: 1 })
    );
}

#[test]
fn no_such_line_is_reported() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(&diff, 0, &Selection::Lines(vec![9])),
        Err(PatchError::NoSuchLine { line: 9 })
    );
}

#[test]
fn a_context_line_is_not_a_changed_line() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(&diff, 0, &Selection::Lines(vec![0])),
        Err(PatchError::NotAChangedLine { line: 0 })
    );
}

#[test]
fn a_binary_diff_has_no_hunks_to_select() {
    let diff = sample_diff(Vec::new(), Some(crate::diff::BinaryNote::Differ));
    assert_eq!(
        hunk_patch(&diff, 0, &Selection::WholeHunk),
        Err(PatchError::Binary)
    );
}

#[test]
fn selecting_nothing_is_reported() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(&diff, 0, &Selection::Lines(Vec::new())),
        Err(PatchError::NothingSelected)
    );
}

#[test]
fn whole_hunk_selection_keeps_every_line() {
    let diff = sample_diff(vec![sample_hunk()], None);
    let patch = hunk_patch(&diff, 0, &Selection::WholeHunk).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(text.contains("@@ -1,2 +1,2 @@\n"));
    assert!(text.contains(" unchanged\n"));
    assert!(text.contains("-old\n"));
    assert!(text.contains("+new\n"));
}

#[test]
fn added_and_deleted_files_use_dev_null() {
    let mut hunk = sample_hunk();
    hunk.old_start = 0;
    hunk.old_lines = 0;
    let mut diff = sample_diff(vec![hunk], None);
    diff.entry.status = FileStatus::Added;
    let patch = hunk_patch(&diff, 0, &Selection::WholeHunk).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(text.contains("--- /dev/null\n"));
    assert!(text.contains("+++ b/f.txt\n"));

    diff.entry.status = FileStatus::Deleted;
    let patch = hunk_patch(&diff, 0, &Selection::WholeHunk).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(text.contains("--- a/f.txt\n"));
    assert!(text.contains("+++ /dev/null\n"));
}

#[test]
fn mode_lines_appear_only_when_the_mode_changed() {
    let mut diff = sample_diff(vec![sample_hunk()], None);
    diff.old_mode = Some(0o100_644);
    diff.new_mode = Some(0o100_755);
    let patch = hunk_patch(&diff, 0, &Selection::WholeHunk).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(text.contains("old mode 100644\n"));
    assert!(text.contains("new mode 100755\n"));
}
