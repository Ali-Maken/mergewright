use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use graft_core::oid::Oid;
use graft_fixtures::Shape;

use super::*;
use crate::diff::{DiffSpec, FileEntry, FileStatus, Hunk, file_diff, list_files};
use crate::fixture::Fixture;
use crate::repo::{RepoHandle, SharedRepo};

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

/// Checks every hunk `hunk_patch` builds for `commit` against the bytes
/// git's own `diff-tree -p` prints for the same commit, in `repo_path`.
///
/// `extra` carries whatever the commit needs beyond the fixed options —
/// `--root` for a commit with no parent.
fn check_commit_whole_hunks(label: &str, repo_path: &Path, commit: Oid, extra: &[&str]) {
    let shared = SharedRepo::open(repo_path).expect("the repository opens");
    let handle = shared.handle();
    let spec = DiffSpec::Commit {
        commit,
        against: None,
    };
    let mut args = vec!["diff-tree", "-p", "-M", "-C", "--no-color", "--no-ext-diff"];
    args.extend_from_slice(extra);
    let commit_hex = commit.to_string();
    args.push(&commit_hex);
    let oracle = graft_fixtures::git::run(repo_path, &args).expect("git diff-tree runs");

    let list = list_files(&handle, &spec, &crate::diff::DiffOptions::default(), &())
        .expect("list_files succeeds");
    let mut checked = 0usize;
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
            let built = hunk_patch(&diff, hunk, &Selection::WholeHunk, PatchDirection::Forward)
                .expect("a whole-hunk selection always builds");
            let portion = hunk_portion(&built);
            assert!(
                contains(&oracle, portion),
                "{label}: hunk {hunk} of {:?} not found verbatim in git's own diff-tree -p output:\n{}",
                String::from_utf8_lossy(&entry.path),
                String::from_utf8_lossy(portion)
            );
            checked += 1;
        }
    }
    HUNKS_CHECKED.fetch_add(checked, Ordering::Relaxed);
}

/// How many hunks the whole-hunk oracle actually compared, so a loop that
/// can skip every shape cannot pass by skipping every shape.
static HUNKS_CHECKED: AtomicUsize = AtomicUsize::new(0);

/// The corpus shape's root commit and its newest commit with a parent and no
/// merge — the first is a pile of file creations, the second the edits,
/// deletions and renames the root cannot show.
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
    if let Some(root) = single_commit(&path, &["rev-list", "--max-parents=0", "--all"]) {
        check_commit_whole_hunks(&format!("{shape} root"), &path, root, &["--root"]);
    }
    if let Some(tip) = single_commit(
        &path,
        &[
            "rev-list",
            "--no-merges",
            "--min-parents=1",
            "-n",
            "1",
            "--all",
        ],
    ) {
        check_commit_whole_hunks(&format!("{shape} tip"), &path, tip, &[]);
    }
}

#[test]
fn whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape() {
    HUNKS_CHECKED.store(0, Ordering::Relaxed);
    for &shape in Shape::ALL {
        check_shape_whole_hunks(shape);
    }
    assert!(
        HUNKS_CHECKED.load(Ordering::Relaxed) > 0,
        "the corpus oracle compared no hunk at all"
    );
}

/// A repository whose one interesting commit carries, on purpose, every diff
/// shape the corpus roots do not have: two hunks in one file with a heading
/// between them, a file with no trailing newline, a rename with an edit, a
/// mode change, a deletion and an addition.
fn awkward_shapes_fixture() -> Fixture {
    let fixture = Fixture::empty_repo();
    let root = fixture.root().to_path_buf();

    let mut base = String::new();
    base.push_str("fn head() {\n");
    for index in 1..=8 {
        base.push_str(&format!("    line {index}\n"));
    }
    base.push_str("}\n");
    base.push_str("fn tail() {\n");
    for index in 9..=16 {
        base.push_str(&format!("    line {index}\n"));
    }
    base.push_str("}\n");
    std::fs::write(root.join("multi.txt"), &base).expect("multi.txt written");
    std::fs::write(root.join("nonl.txt"), "alpha\nbeta").expect("nonl.txt written");
    std::fs::write(root.join("renamed.txt"), "r1\nr2\nr3\nr4\nr5\nr6\nr7\nr8\n")
        .expect("renamed.txt written");
    std::fs::write(root.join("mode.txt"), "chmod me\n").expect("mode.txt written");
    std::fs::write(root.join("gone.txt"), "bye\n").expect("gone.txt written");
    fixture.git(&["add", "-A"]);
    fixture.commit("base");

    let edited = base
        .replace("    line 2\n", "    line two\n")
        .replace("    line 15\n", "    line fifteen\n");
    std::fs::write(root.join("multi.txt"), &edited).expect("multi.txt rewritten");
    std::fs::write(root.join("nonl.txt"), "alpha\nbeta\ngamma").expect("nonl.txt rewritten");
    fixture.git(&["mv", "renamed.txt", "moved.txt"]);
    std::fs::write(
        root.join("moved.txt"),
        "r1\nr2\nr3\nr4\nr5\nr6\nr7\nr8 edited\n",
    )
    .expect("moved.txt rewritten");
    fixture.git(&["update-index", "--chmod=+x", "mode.txt"]);
    std::fs::remove_file(root.join("gone.txt")).expect("gone.txt removed");
    std::fs::write(root.join("fresh.txt"), "new file\n").expect("fresh.txt written");
    fixture.git(&["add", "-A"]);
    fixture.commit("every awkward shape at once");
    fixture
}

#[test]
fn whole_hunk_reproduces_gits_own_bytes_on_the_awkward_shapes() {
    let fixture = awkward_shapes_fixture();
    let before = HUNKS_CHECKED.load(Ordering::Relaxed);
    let head = Oid::from_hex(&fixture.rev_parse("HEAD")).expect("a full object id");
    check_commit_whole_hunks("awkward", fixture.root(), head, &[]);

    // The point of the fixture is that these shapes exist at all: a two-hunk
    // file with a `@@ ... @@ fn …` heading, a `\ No newline at end of file`
    // marker, a rename with an edit, a deletion and an addition.
    let oracle = graft_fixtures::git::run(
        fixture.root(),
        &[
            "diff-tree",
            "-p",
            "-M",
            "-C",
            "--no-color",
            "--no-ext-diff",
            &fixture.rev_parse("HEAD"),
        ],
    )
    .expect("git diff-tree runs");
    let text = String::from_utf8(oracle).expect("ascii fixture");
    assert!(text.contains("\\ No newline at end of file"), "{text}");
    assert!(text.contains("rename from renamed.txt"), "{text}");
    assert!(text.contains("@@ -15,6 +15,6 @@ fn tail() {"), "{text}");
    assert!(text.contains("--- /dev/null"), "{text}");
    assert!(text.contains("+++ /dev/null"), "{text}");
    assert!(
        HUNKS_CHECKED.load(Ordering::Relaxed) >= before + 6,
        "the awkward fixture must contribute several hunks, not one"
    );
}

/// The one commit `args` names, or `None` when it names none.
fn single_commit(repo_path: &Path, args: &[&str]) -> Option<Oid> {
    let stdout = graft_fixtures::git::run(repo_path, args).expect("git rev-list runs");
    let text = String::from_utf8(stdout).expect("git prints hex ids as ascii");
    let first_line = text.lines().next()?;
    Some(Oid::from_hex(first_line.trim()).expect("git prints a full object id"))
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

    let patch = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![four_index]),
        PatchDirection::Forward,
    )
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
        hunk_patch(&diff, 1, &Selection::WholeHunk, PatchDirection::Forward),
        Err(PatchError::NoSuchHunk { hunk: 1 })
    );
}

#[test]
fn no_such_line_is_reported() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(
            &diff,
            0,
            &Selection::Lines(vec![9]),
            PatchDirection::Forward
        ),
        Err(PatchError::NoSuchLine { line: 9 })
    );
}

#[test]
fn a_context_line_is_not_a_changed_line() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(
            &diff,
            0,
            &Selection::Lines(vec![0]),
            PatchDirection::Forward
        ),
        Err(PatchError::NotAChangedLine { line: 0 })
    );
}

#[test]
fn a_binary_diff_has_no_hunks_to_select() {
    let diff = sample_diff(Vec::new(), Some(crate::diff::BinaryNote::Differ));
    assert_eq!(
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward),
        Err(PatchError::Binary)
    );
}

#[test]
fn selecting_nothing_is_reported() {
    let diff = sample_diff(vec![sample_hunk()], None);
    assert_eq!(
        hunk_patch(
            &diff,
            0,
            &Selection::Lines(Vec::new()),
            PatchDirection::Forward
        ),
        Err(PatchError::NothingSelected)
    );
}

#[test]
fn whole_hunk_selection_keeps_every_line() {
    let diff = sample_diff(vec![sample_hunk()], None);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(text.contains("@@ -1,2 +1,2 @@\n"));
    assert!(text.contains(" unchanged\n"));
    assert!(text.contains("-old\n"));
    assert!(text.contains("+new\n"));
}

#[test]
fn added_and_deleted_files_use_dev_null_under_their_extended_header() {
    let mut hunk = sample_hunk();
    hunk.old_start = 0;
    hunk.old_lines = 0;
    let mut diff = sample_diff(vec![hunk], None);
    diff.entry.status = FileStatus::Added;
    diff.new_mode = Some(0o100_644);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(
        text.starts_with(
            "diff --git a/f.txt b/f.txt\nnew file mode 100644\n--- /dev/null\n+++ b/f.txt\n"
        ),
        "{text}"
    );

    diff.entry.status = FileStatus::Deleted;
    diff.new_mode = None;
    diff.old_mode = Some(0o100_755);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(
        text.starts_with(
            "diff --git a/f.txt b/f.txt\ndeleted file mode 100755\n--- a/f.txt\n+++ /dev/null\n"
        ),
        "{text}"
    );
}

#[test]
fn a_creation_or_deletion_of_unknown_mode_is_written_without_the_diff_git_line() {
    let mut diff = sample_diff(vec![sample_hunk()], None);
    for status in [FileStatus::Added, FileStatus::Deleted] {
        diff.entry.status = status;
        let patch =
            hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");
        let text = String::from_utf8(patch).expect("ascii");
        assert!(text.starts_with("--- "), "{status:?}: {text}");
        assert!(!text.contains("file mode"), "{status:?}: {text}");
    }
}

#[test]
fn mode_lines_appear_only_when_the_mode_changed() {
    let mut diff = sample_diff(vec![sample_hunk()], None);
    diff.old_mode = Some(0o100_644);
    diff.new_mode = Some(0o100_755);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");
    let text = String::from_utf8(patch).expect("ascii");
    assert!(text.contains("old mode 100644\n"));
    assert!(text.contains("new mode 100755\n"));
}

/// Writes `patch` beside the repository and hands it to `git apply` with
/// `args` (`--cached`, `-R`, …). Panics with git's own message if git refuses
/// it — a refusal is the failure this proves does not happen.
fn apply(fixture: &Fixture, name: &str, patch: &[u8], args: &[&str]) {
    std::fs::write(fixture.root().join(name), patch).expect("the patch is written");
    let mut full = vec!["apply"];
    full.extend_from_slice(args);
    full.push(name);
    fixture.git(&full);
}

/// The exact bytes of a blob: `":f.txt"` for the index, `"HEAD:f.txt"` for a
/// commit. Bytes, not lines — a fused line or a gained newline is invisible
/// to anything coarser.
fn blob(fixture: &Fixture, spec: &str) -> Vec<u8> {
    graft_fixtures::git::run(fixture.root(), &["cat-file", "blob", spec])
        .expect("git cat-file runs")
}

/// A repository whose `f.txt` is committed as `committed` and whose worktree
/// holds `working`.
fn one_file_fixture(committed: &str, working: &str) -> Fixture {
    let fixture = Fixture::empty_repo();
    std::fs::write(fixture.root().join("f.txt"), committed).expect("f.txt written");
    fixture.git(&["add", "f.txt"]);
    fixture.commit("base");
    std::fs::write(fixture.root().join("f.txt"), working).expect("f.txt rewritten");
    fixture
}

fn unstaged_diff(repo: &RepoHandle) -> FileDiff {
    let entry = FileEntry {
        path: b"f.txt".to_vec(),
        old_path: None,
        status: FileStatus::Modified,
        added: None,
        removed: None,
    };
    file_diff(
        repo,
        &DiffSpec::Unstaged,
        &entry,
        &crate::diff::DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds")
}

fn staged_diff(repo: &RepoHandle) -> FileDiff {
    let entry = FileEntry {
        path: b"f.txt".to_vec(),
        old_path: None,
        status: FileStatus::Modified,
        added: None,
        removed: None,
    };
    file_diff(
        repo,
        &DiffSpec::Staged,
        &entry,
        &crate::diff::DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds")
}

/// The index, in `hunk`, of the one `Added` line whose text is `text`.
fn added_line(hunk: &Hunk, text: &[u8]) -> usize {
    hunk.lines
        .iter()
        .position(|line| line.kind == LineKind::Added && line.text == text)
        .unwrap_or_else(|| panic!("the hunk has a +{} line", String::from_utf8_lossy(text)))
}

const TEN_LINES: &str = "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n";
const TEN_LINES_EDITED: &str = "l1\nl2\nl3\nl4\nL5\nl5b\nl6\nl7\nl8\nl9\nl10\n";

#[test]
fn a_forward_line_selection_stages_exactly_that_line_byte_for_byte() {
    let fixture = one_file_fixture(TEN_LINES, TEN_LINES_EDITED);
    let repo = open(&fixture);
    let diff = unstaged_diff(&repo);
    assert_eq!(diff.hunks.len(), 1, "one hunk covers this change");
    let selected = added_line(&diff.hunks[0], b"l5b");

    let patch = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![selected]),
        PatchDirection::Forward,
    )
    .expect("a forward one-line selection builds");
    // The hunk does not start at line 1, so the header's starts are carried
    // through, not assumed away.
    let text = String::from_utf8(patch.clone()).expect("ascii");
    assert!(text.contains("@@ -2,7 +2,8 @@"), "{text}");

    apply(&fixture, "forward.diff", &patch, &["--cached"]);

    assert_eq!(
        blob(&fixture, ":f.txt"),
        b"l1\nl2\nl3\nl4\nl5\nl5b\nl6\nl7\nl8\nl9\nl10\n".to_vec(),
        "only the selected line may reach the index"
    );
    assert_eq!(
        std::fs::read(fixture.root().join("f.txt")).expect("f.txt reads"),
        TEN_LINES_EDITED.as_bytes(),
        "staging must not touch the worktree"
    );
}

#[test]
fn a_reverse_line_selection_discards_exactly_that_line_from_the_worktree() {
    let fixture = one_file_fixture(TEN_LINES, TEN_LINES_EDITED);
    let repo = open(&fixture);
    let diff = unstaged_diff(&repo);
    let selected = added_line(&diff.hunks[0], b"l5b");

    let patch = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![selected]),
        PatchDirection::Reverse,
    )
    .expect("a reverse one-line selection builds");

    apply(&fixture, "reverse.diff", &patch, &["-R"]);

    assert_eq!(
        std::fs::read(fixture.root().join("f.txt")).expect("f.txt reads"),
        b"l1\nl2\nl3\nl4\nL5\nl6\nl7\nl8\nl9\nl10\n".to_vec(),
        "only the selected line may leave the worktree; L5 stays"
    );
}

#[test]
fn a_reverse_line_selection_unstages_exactly_that_line_from_the_index() {
    let fixture = one_file_fixture(TEN_LINES, TEN_LINES_EDITED);
    fixture.git(&["add", "f.txt"]);
    let repo = open(&fixture);
    let diff = staged_diff(&repo);
    let selected = added_line(&diff.hunks[0], b"l5b");

    let patch = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![selected]),
        PatchDirection::Reverse,
    )
    .expect("a reverse one-line selection builds");

    apply(&fixture, "unstage.diff", &patch, &["--cached", "-R"]);

    assert_eq!(
        blob(&fixture, ":f.txt"),
        b"l1\nl2\nl3\nl4\nL5\nl6\nl7\nl8\nl9\nl10\n".to_vec(),
        "only the selected line may leave the index"
    );
}

#[test]
fn a_reverse_selection_of_a_removed_line_puts_it_back() {
    // `l5` was removed and `L5` put in its place. Discarding only the removal
    // must restore `l5` and leave `L5` and `l5b` alone.
    let fixture = one_file_fixture(TEN_LINES, TEN_LINES_EDITED);
    let repo = open(&fixture);
    let diff = unstaged_diff(&repo);
    let removed = diff.hunks[0]
        .lines
        .iter()
        .position(|line| line.kind == LineKind::Removed && line.text == b"l5")
        .expect("the hunk has a -l5 line");

    let patch = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![removed]),
        PatchDirection::Reverse,
    )
    .expect("a reverse one-line selection builds");

    apply(&fixture, "restore.diff", &patch, &["-R"]);

    assert_eq!(
        std::fs::read(fixture.root().join("f.txt")).expect("f.txt reads"),
        b"l1\nl2\nl3\nl4\nl5\nL5\nl5b\nl6\nl7\nl8\nl9\nl10\n".to_vec(),
        "the removed line comes back; nothing else moves"
    );
}

#[test]
fn a_forward_patch_is_refused_in_reverse_and_the_reverse_one_is_the_answer() {
    // The finding this guards: the forward form was the only form built, and
    // `git apply -R` refuses it outright — line-level discard was impossible.
    let fixture = one_file_fixture(TEN_LINES, TEN_LINES_EDITED);
    let repo = open(&fixture);
    let diff = unstaged_diff(&repo);
    let selected = added_line(&diff.hunks[0], b"l5b");

    let forward = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![selected]),
        PatchDirection::Forward,
    )
    .expect("builds");
    let reverse = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![selected]),
        PatchDirection::Reverse,
    )
    .expect("builds");
    assert_ne!(
        forward, reverse,
        "the two directions cannot be the same patch"
    );

    std::fs::write(fixture.root().join("forward.diff"), &forward).expect("patch written");
    let refused = graft_fixtures::git::run(fixture.root(), &["apply", "-R", "forward.diff"]);
    assert!(
        refused.is_err(),
        "git must refuse the forward form in reverse; if it stops refusing, \
         this test is the wrong guard"
    );

    // And the reverse form is accepted for the same selection.
    apply(&fixture, "reverse.diff", &reverse, &["-R"]);
    assert_eq!(
        std::fs::read(fixture.root().join("f.txt")).expect("f.txt reads"),
        b"l1\nl2\nl3\nl4\nL5\nl6\nl7\nl8\nl9\nl10\n".to_vec()
    );
}

#[test]
fn a_selection_that_would_strand_a_no_newline_marker_is_refused() {
    // `one\ntwo` (no final newline) → `one\ntwo\nthree` (no final newline).
    // Git's hunk is ` one`, `-two` + marker, `+two`, `+three` + marker.
    // Selecting only `+three` forward turns the marker-carrying `-two` into
    // context with `+three` still after it; git takes that patch, exits 0 and
    // stages `one\ntwothree` — two lines fused into one.
    let fixture = one_file_fixture("one\ntwo", "one\ntwo\nthree");
    let repo = open(&fixture);
    let diff = unstaged_diff(&repo);
    let hunk = &diff.hunks[0];
    let three = added_line(hunk, b"three");
    let marked = hunk
        .lines
        .iter()
        .position(|line| line.kind == LineKind::Removed && line.no_newline_at_eof)
        .expect("git marks the removed `two` as having no newline");

    assert_eq!(
        hunk_patch(
            &diff,
            0,
            &Selection::Lines(vec![three]),
            PatchDirection::Forward
        ),
        Err(PatchError::NoNewlineBoundary { line: marked })
    );

    // The whole hunk is still stageable, and stages the right bytes.
    let whole = hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward)
        .expect("the whole hunk always builds");
    apply(&fixture, "whole.diff", &whole, &["--cached"]);
    assert_eq!(
        blob(&fixture, ":f.txt"),
        b"one\ntwo\nthree".to_vec(),
        "the whole hunk stages the file git itself would"
    );
}

#[test]
fn the_whole_hunk_of_a_no_newline_file_reverses_out_byte_for_byte() {
    let fixture = one_file_fixture("one\ntwo", "one\ntwo\nthree");
    let repo = open(&fixture);
    let diff = unstaged_diff(&repo);

    let whole = hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Reverse)
        .expect("the whole hunk always builds");
    apply(&fixture, "whole.diff", &whole, &["-R"]);

    assert_eq!(
        std::fs::read(fixture.root().join("f.txt")).expect("f.txt reads"),
        b"one\ntwo".to_vec(),
        "discarding the whole hunk restores the committed bytes, newline and all"
    );
}

// --- Whole-file creations and deletions (m7-power-tools/31) ---------------
//
// Under a `diff --git` header, `git apply` reads `/dev/null` as a path unless
// an extended header line (`deleted file mode`/`new file mode`) says the file
// goes or comes. These tests apply the built patch to a real index and read
// the index back: the text of the patch alone cannot show what git does with
// it.

/// A repository with `a.txt` and `b.txt` committed.
fn two_file_fixture() -> Fixture {
    let fixture = Fixture::empty_repo();
    std::fs::write(fixture.root().join("a.txt"), "keep\n").expect("a.txt written");
    std::fs::write(fixture.root().join("b.txt"), "bye\nfor now\n").expect("b.txt written");
    fixture.git(&["add", "-A"]);
    fixture.commit("base");
    fixture
}

/// The staged diff of `path`, whose status in the index is `status`.
fn staged_file_diff(repo: &RepoHandle, path: &str, status: FileStatus) -> FileDiff {
    let entry = FileEntry {
        path: path.as_bytes().to_vec(),
        old_path: None,
        status,
        added: None,
        removed: None,
    };
    file_diff(
        repo,
        &DiffSpec::Staged,
        &entry,
        &crate::diff::DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds")
}

/// Every path in the index, in git's order.
fn index_paths(fixture: &Fixture) -> Vec<String> {
    let stdout =
        graft_fixtures::git::run(fixture.root(), &["ls-files"]).expect("git ls-files runs");
    String::from_utf8(stdout)
        .expect("ascii paths")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_deleted_files_patch_applied_to_the_index_removes_it_and_adds_no_dev_null() {
    let fixture = two_file_fixture();
    fixture.git(&["rm", "-q", "--cached", "b.txt"]);
    let diff = staged_file_diff(&open(&fixture), "b.txt", FileStatus::Deleted);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");

    fixture.git(&["read-tree", "HEAD"]);
    apply(&fixture, "delete.diff", &patch, &["--cached"]);

    assert_eq!(
        index_paths(&fixture),
        vec!["a.txt".to_owned()],
        "b.txt goes and nothing named dev/null comes"
    );
}

#[test]
fn an_added_files_patch_applied_to_the_index_creates_it() {
    let fixture = two_file_fixture();
    std::fs::write(fixture.root().join("fresh.txt"), "new file\nsecond\n").expect("written");
    fixture.git(&["add", "fresh.txt"]);
    let diff = staged_file_diff(&open(&fixture), "fresh.txt", FileStatus::Added);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");

    fixture.git(&["read-tree", "HEAD"]);
    apply(&fixture, "create.diff", &patch, &["--cached"]);

    assert_eq!(
        index_paths(&fixture),
        vec![
            "a.txt".to_owned(),
            "b.txt".to_owned(),
            "fresh.txt".to_owned()
        ]
    );
    assert_eq!(blob(&fixture, ":fresh.txt"), b"new file\nsecond\n".to_vec());
}

#[test]
fn a_modified_files_patch_applied_to_the_index_changes_only_it() {
    let fixture = two_file_fixture();
    std::fs::write(fixture.root().join("b.txt"), "bye\nfor good\n").expect("written");
    fixture.git(&["add", "b.txt"]);
    let diff = staged_file_diff(&open(&fixture), "b.txt", FileStatus::Modified);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");

    fixture.git(&["read-tree", "HEAD"]);
    apply(&fixture, "modify.diff", &patch, &["--cached"]);

    assert_eq!(
        index_paths(&fixture),
        vec!["a.txt".to_owned(), "b.txt".to_owned()]
    );
    assert_eq!(blob(&fixture, ":b.txt"), b"bye\nfor good\n".to_vec());
}

#[test]
fn unstaging_a_staged_deletion_puts_the_file_back_in_the_index() {
    let fixture = two_file_fixture();
    fixture.git(&["rm", "-q", "--cached", "b.txt"]);
    let diff = staged_file_diff(&open(&fixture), "b.txt", FileStatus::Deleted);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Reverse).expect("builds");

    apply(&fixture, "undelete.diff", &patch, &["--cached", "-R"]);

    assert_eq!(
        index_paths(&fixture),
        vec!["a.txt".to_owned(), "b.txt".to_owned()]
    );
    assert_eq!(blob(&fixture, ":b.txt"), b"bye\nfor now\n".to_vec());
}

#[test]
fn unstaging_a_staged_addition_takes_the_file_out_of_the_index() {
    let fixture = two_file_fixture();
    std::fs::write(fixture.root().join("fresh.txt"), "new file\n").expect("written");
    fixture.git(&["add", "fresh.txt"]);
    let diff = staged_file_diff(&open(&fixture), "fresh.txt", FileStatus::Added);
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Reverse).expect("builds");

    apply(&fixture, "unadd.diff", &patch, &["--cached", "-R"]);

    assert_eq!(
        index_paths(&fixture),
        vec!["a.txt".to_owned(), "b.txt".to_owned()]
    );
}

#[test]
fn discarding_a_worktree_deletion_brings_the_file_back() {
    let fixture = two_file_fixture();
    std::fs::remove_file(fixture.root().join("b.txt")).expect("b.txt removed");
    let entry = FileEntry {
        path: b"b.txt".to_vec(),
        old_path: None,
        status: FileStatus::Deleted,
        added: None,
        removed: None,
    };
    let diff = file_diff(
        &open(&fixture),
        &DiffSpec::Unstaged,
        &entry,
        &crate::diff::DiffOptions::default(),
        usize::MAX,
        &(),
    )
    .expect("file_diff succeeds");
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Reverse).expect("builds");

    apply(&fixture, "discard.diff", &patch, &["-R"]);

    assert_eq!(
        std::fs::read(fixture.root().join("b.txt")).expect("b.txt is back"),
        b"bye\nfor now\n".to_vec()
    );
    assert!(
        !fixture.root().join("dev").exists(),
        "no dev/null is written into the worktree"
    );
}

#[test]
fn a_deletion_of_unknown_mode_still_applies_as_a_deletion() {
    let fixture = two_file_fixture();
    fixture.git(&["rm", "-q", "--cached", "b.txt"]);
    let mut diff = staged_file_diff(&open(&fixture), "b.txt", FileStatus::Deleted);
    diff.old_mode = None;
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");

    fixture.git(&["read-tree", "HEAD"]);
    apply(&fixture, "delete.diff", &patch, &["--cached"]);

    assert_eq!(index_paths(&fixture), vec!["a.txt".to_owned()]);
}

#[test]
fn a_creation_of_unknown_mode_still_applies_as_a_creation() {
    let fixture = two_file_fixture();
    std::fs::write(fixture.root().join("fresh.txt"), "new file\n").expect("written");
    fixture.git(&["add", "fresh.txt"]);
    let mut diff = staged_file_diff(&open(&fixture), "fresh.txt", FileStatus::Added);
    diff.new_mode = None;
    let patch =
        hunk_patch(&diff, 0, &Selection::WholeHunk, PatchDirection::Forward).expect("builds");

    fixture.git(&["read-tree", "HEAD"]);
    apply(&fixture, "create.diff", &patch, &["--cached"]);

    assert_eq!(
        index_paths(&fixture),
        vec![
            "a.txt".to_owned(),
            "b.txt".to_owned(),
            "fresh.txt".to_owned()
        ]
    );
    assert_eq!(blob(&fixture, ":fresh.txt"), b"new file\n".to_vec());
}

#[test]
fn staging_part_of_a_deletion_is_refused_by_git_and_leaves_the_index_alone() {
    let fixture = two_file_fixture();
    fixture.git(&["rm", "-q", "--cached", "b.txt"]);
    let diff = staged_file_diff(&open(&fixture), "b.txt", FileStatus::Deleted);
    let patch = hunk_patch(
        &diff,
        0,
        &Selection::Lines(vec![0]),
        PatchDirection::Forward,
    )
    .expect("builds");

    fixture.git(&["read-tree", "HEAD"]);
    std::fs::write(fixture.root().join("part.diff"), &patch).expect("the patch is written");
    let refused =
        graft_fixtures::git::run(fixture.root(), &["apply", "--cached", "part.diff"]).is_err();

    assert!(
        refused,
        "git must refuse a deletion that leaves lines behind"
    );
    assert_eq!(
        index_paths(&fixture),
        vec!["a.txt".to_owned(), "b.txt".to_owned()],
        "a refused patch changes nothing"
    );
}
