//! `trace`'s readers against git's own answers, on the `session` fixture
//! (m7-power-tools/18).

use std::path::Path;
use std::sync::atomic::AtomicBool;

use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::diff::{FileStatus, LineKind};
use graft_git::trace::{
    LineRange, SHALLOW_END, TraceCommit, TraceEnd, TraceError, enclosing_function, file_history,
    line_history, whole_file,
};

use crate::session::{FILE, OLD_FILE, commits, git_ok, open, rev};

fn function(dir: &Path, at: Oid, path: &str, line: u32) -> Option<LineRange> {
    enclosing_function(&open(dir), at, Path::new(path), line, &())
        .unwrap_or_else(|why| panic!("the function around {path}:{line}: {why}"))
}

fn trace_lines(dir: &Path, at: Oid, range: &LineRange, cap: usize) -> (Vec<TraceCommit>, TraceEnd) {
    let mut rows = Vec::new();
    let end = line_history(
        &open(dir),
        at,
        Path::new(FILE),
        range,
        cap,
        &mut |batch| rows.extend(batch),
        &(),
    )
    .unwrap_or_else(|why| panic!("line history: {why}"));
    (rows, end)
}

fn trace_file(dir: &Path, at: Oid, cap: usize) -> (Vec<TraceCommit>, TraceEnd) {
    let mut rows = Vec::new();
    let end = file_history(
        &open(dir),
        at,
        Path::new(FILE),
        cap,
        &mut |batch| rows.extend(batch),
        &(),
    )
    .unwrap_or_else(|why| panic!("file history: {why}"));
    (rows, end)
}

/// A copy of the fixture with `path` (in a directory it already has)
/// committed as `content`.
fn with_file(path: &str, content: &str) -> (graft_fixtures::Scratch, Oid) {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    std::fs::write(dir.join(path), content).unwrap_or_else(|why| panic!("writing {path}: {why}"));
    git_ok(dir, &["add", path]);
    git_ok(dir, &["commit", "-q", "-m", "add a file"]);
    let head = rev(dir, "HEAD");
    (scratch, head)
}

const REFRESH: &str = "pub fn refresh_token(&mut self, client: &Client) -> Result<()> {";

#[test]
fn a_rust_method_is_its_fn_not_the_impl() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    for line in [18, 22, 29] {
        assert_eq!(
            function(&dir, ids.you, FILE, line),
            Some(LineRange::Function {
                name: REFRESH.to_owned(),
                start: 18,
                end: 29
            }),
            "line {line}"
        );
    }
    // In Tom's commit it sat higher.
    assert_eq!(
        function(&dir, ids.tom, OLD_FILE, 20),
        Some(LineRange::Function {
            name: REFRESH.to_owned(),
            start: 17,
            end: 27
        })
    );
}

#[test]
fn a_python_def_is_found() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    assert_eq!(
        function(&dir, ids.you, "tools/rotate.py", 5),
        Some(LineRange::Function {
            name: "def rotate(keys):".to_owned(),
            start: 4,
            end: 7
        })
    );
}

#[test]
fn no_function_is_none() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    assert_eq!(
        function(&dir, ids.you, FILE, 1),
        None,
        "an import line, before any header"
    );
    assert_eq!(function(&dir, ids.you, FILE, 999), None, "past the end");
    assert_eq!(function(&dir, ids.you, "assets/pic.bin", 1), None, "binary");
}

#[test]
fn over_400_lines_is_none() {
    let mut body = String::from("fn huge() {\n");
    for n in 0..450 {
        body.push_str(&format!("    let x{n} = {n};\n"));
    }
    body.push_str("}\n\nfn small() {\n    let y = 1;\n}\n");
    let (scratch, head) = with_file("src/huge.rs", &body);
    assert_eq!(
        function(scratch.path(), head, "src/huge.rs", 200),
        None,
        "a 452-line fn"
    );
    assert_eq!(
        function(scratch.path(), head, "src/huge.rs", 455),
        Some(LineRange::Function {
            name: "fn small() {".to_owned(),
            start: 454,
            end: 456
        }),
        "the small one after it is still found"
    );
}

#[test]
fn a_user_attributes_file_is_never_overridden() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    // The user's own attributes file, naming no driver for `.rs`. Were Graft
    // to pass `-c core.attributesFile=…` over it, git would use the rust
    // driver and name the method; untouched, git's default names the impl.
    let attributes = dir.join(".git/user-attributes");
    std::fs::write(&attributes, "*.txt text\n").expect("writes");
    let value = attributes.to_str().expect("a UTF-8 temp path");
    git_ok(dir, &["config", "core.attributesFile", value]);
    assert_eq!(
        function(dir, ids.you, FILE, 22),
        Some(LineRange::Function {
            name: "impl Session {".to_owned(),
            start: 13,
            end: 42
        })
    );
}

#[test]
fn a_repository_driver_is_used() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    // A directory-scoped pattern cannot match the temporary files by
    // itself; the driver it names must still be the one used.
    std::fs::write(dir.join(".git/info/attributes"), "src/** diff=onlyimpl\n").expect("writes");
    git_ok(dir, &["config", "diff.onlyimpl.xfuncname", "^impl.*$"]);
    let found = function(dir, ids.you, FILE, 22);
    assert!(
        matches!(&found, Some(LineRange::Function { name, .. }) if name == "impl Session {"),
        "{found:?}"
    );
}

#[test]
#[allow(non_snake_case, reason = "the ticket's name: `L` is git's own flag")]
fn line_history_matches_git_log_L() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let range = LineRange::Block { start: 18, end: 29 };
    let (rows, _) = trace_lines(&dir, ids.you, &range, 1000);
    let oracle = git_ok(
        &dir,
        &[
            "log",
            "--format=%H",
            "-s",
            "-L18,29:src/auth/session.rs",
            &ids.you.to_string(),
        ],
    );
    let expected: Vec<Oid> = String::from_utf8_lossy(&oracle)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|hex| Oid::from_hex(hex).expect("an id"))
        .collect();
    assert_eq!(
        expected,
        [ids.you, ids.lina, ids.tom],
        "the oracle's own answer"
    );
    let ours: Vec<Oid> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ours, expected);
}

#[test]
fn range_at_moves_with_the_code() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let range = function(&dir, ids.you, FILE, 22).expect("refresh_token");
    let (rows, _) = trace_lines(&dir, ids.you, &range, 1000);
    let at: Vec<Option<(u32, u32)>> = rows.iter().map(|row| row.range_at).collect();
    assert_eq!(at, [Some((18, 29)), Some((17, 28)), Some((17, 27))]);
    let counts: Vec<(u32, u32)> = rows.iter().map(|row| (row.added, row.removed)).collect();
    assert_eq!(counts, [(1, 1), (3, 2), (11, 0)], "the range's lines only");
    assert_eq!(
        rows[2].path,
        Path::new(OLD_FILE),
        "Tom's commit had the old name"
    );
}

#[test]
fn file_history_follows_the_rename() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let (rows, _) = trace_file(&dir, ids.you, 1000);
    let order: Vec<Oid> = rows.iter().map(|row| row.id).collect();
    assert_eq!(
        order,
        [ids.you, ids.priya, ids.rustfmt, ids.lina, ids.tom, ids.ana]
    );
    let priya = &rows[1];
    assert_eq!(priya.renamed_from.as_deref(), Some(Path::new(OLD_FILE)));
    assert_eq!(priya.path, Path::new(FILE));
    assert_eq!(priya.author, "Priya");
    assert!(
        rows.iter()
            .filter(|row| row.id != ids.priya)
            .all(|row| row.renamed_from.is_none())
    );
    assert_eq!(rows[3].path, Path::new(OLD_FILE));
    assert_eq!(
        (rows[5].added, rows[5].removed),
        (42, 0),
        "the file's counts"
    );
}

#[test]
fn trace_end_is_start_on_the_adding_commit() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let range = LineRange::Block { start: 18, end: 29 };
    assert_eq!(
        trace_lines(&dir, ids.you, &range, 1000).1,
        TraceEnd::Start { added_by: ids.tom }
    );
    assert_eq!(
        trace_file(&dir, ids.you, 1000).1,
        TraceEnd::Start { added_by: ids.ana }
    );
}

#[test]
fn git_stopping_says_why() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let (rows, end) = trace_lines(
        &dir,
        ids.you,
        &LineRange::Block {
            start: 100,
            end: 120,
        },
        1000,
    );
    assert!(rows.is_empty());
    assert_eq!(
        end,
        TraceEnd::GitStops("file src/auth/session.rs has only 60 lines".to_owned())
    );
}

#[test]
fn trace_capped_says_how_many() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let (rows, end) = trace_file(&dir, ids.you, 2);
    assert_eq!(rows.len(), 2);
    assert_eq!(end, TraceEnd::Capped { shown: 2 });
    let (rows, end) = trace_file(&dir, ids.you, 6);
    assert_eq!(rows.len(), 6, "exactly the cap, nothing more: not capped");
    assert_eq!(end, TraceEnd::Start { added_by: ids.ana });
}

#[test]
fn stop_kills_git_and_keeps_the_rows() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let stop = AtomicBool::new(true);
    let mut rows = Vec::new();
    let end = file_history(
        &open(&dir),
        ids.you,
        Path::new(FILE),
        1000,
        &mut |batch| rows.extend(batch),
        &stop,
    )
    .expect("a stop is an end, not an error");
    assert_eq!(end, TraceEnd::Stopped);
    // The runner hands stdout over only at exit, so a stopped trace has sent
    // nothing: the rows the sink holds are exactly those sent, none.
    assert!(rows.is_empty());
    let range = LineRange::Block { start: 18, end: 29 };
    let end = line_history(
        &open(&dir),
        ids.you,
        Path::new(FILE),
        &range,
        1000,
        &mut |_| {},
        &stop,
    )
    .expect("a stop is an end");
    assert_eq!(end, TraceEnd::Stopped);
}

#[test]
fn whole_file_is_one_hunk_with_every_line() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let diff = whole_file(&open(&dir), ids.you, Path::new(FILE), None, usize::MAX, &())
        .expect("the whole file");
    assert_eq!(diff.hunks.len(), 1);
    let hunk = &diff.hunks[0];
    assert_eq!((hunk.new_start, hunk.new_lines), (1, 60));
    assert_eq!(hunk.lines.len(), 61, "60 lines + the 1 removed");
    let changed: Vec<(LineKind, Option<u32>)> = hunk
        .lines
        .iter()
        .filter(|line| line.kind != LineKind::Context)
        .map(|line| (line.kind, line.new_no))
        .collect();
    assert_eq!(
        changed,
        [
            (LineKind::Added, Some(3)),
            (LineKind::Removed, None),
            (LineKind::Added, Some(26))
        ]
    );
    assert_eq!(diff.truncated, None);

    // Priya's rename, with the old name given.
    let diff = whole_file(
        &open(&dir),
        ids.priya,
        Path::new(FILE),
        Some(Path::new(OLD_FILE)),
        usize::MAX,
        &(),
    )
    .expect("the rename");
    assert!(
        matches!(diff.entry.status, FileStatus::Renamed { .. }),
        "{:?}",
        diff.entry.status
    );
    assert_eq!(diff.entry.old_path.as_deref(), Some(OLD_FILE.as_bytes()));
    assert_eq!(diff.hunks.len(), 1);
}

#[test]
fn whole_file_of_a_root_commit_is_all_adds() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let diff = whole_file(
        &open(&dir),
        ids.ana,
        Path::new(OLD_FILE),
        None,
        usize::MAX,
        &(),
    )
    .expect("the root commit");
    assert_eq!(diff.entry.status, FileStatus::Added);
    assert_eq!(diff.hunks.len(), 1);
    assert_eq!(diff.hunks[0].lines.len(), 42);
    assert!(
        diff.hunks[0]
            .lines
            .iter()
            .all(|line| line.kind == LineKind::Added)
    );
}

#[test]
fn whole_file_of_a_merge_diffs_its_first_parent() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    git_ok(dir, &["checkout", "-q", "-b", "side"]);
    let file = dir.join(FILE);
    let text = std::fs::read_to_string(&file).expect("reads");
    std::fs::write(
        &file,
        text.replace("    use super::*;\n", "    use super::*;\n    // side\n"),
    )
    .expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "side"]);
    git_ok(dir, &["checkout", "-q", "main"]);
    std::fs::write(dir.join("tools/rotate.py"), "print('main')\n").expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "main"]);
    git_ok(dir, &["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
    let merge = rev(dir, "HEAD");

    let diff =
        whole_file(&open(dir), merge, Path::new(FILE), None, usize::MAX, &()).expect("the merge");
    let added: Vec<&[u8]> = diff.hunks[0]
        .lines
        .iter()
        .filter(|line| line.kind == LineKind::Added)
        .map(|line| line.text.as_slice())
        .collect();
    assert_eq!(
        added,
        [b"    // side".as_slice()],
        "against main, the first parent"
    );

    // A file the merge did not change against its first parent: all context.
    let diff = whole_file(
        &open(dir),
        merge,
        Path::new("tools/rotate.py"),
        None,
        usize::MAX,
        &(),
    )
    .expect("unchanged");
    assert_eq!(diff.hunks.len(), 1);
    assert!(
        diff.hunks[0]
            .lines
            .iter()
            .all(|line| line.kind == LineKind::Context)
    );
}

#[test]
fn whole_file_respects_max_lines() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let diff = whole_file(&open(&dir), ids.you, Path::new(FILE), None, 10, &()).expect("cut");
    assert_eq!(diff.hunks[0].lines.len(), 10);
    let truncated = diff.truncated.expect("cut short");
    assert_eq!((truncated.shown_lines, truncated.total_lines), (10, 61));
    assert_eq!(
        (diff.hunks[0].old_lines, diff.hunks[0].new_lines),
        (9, 10),
        "recounted"
    );
}

/// A copy of the fixture where `plain.txt` was added, then renamed to the
/// non-ASCII `café.txt` with its middle line changed (m7-power-tools/27).
/// git quotes the new name in `diff --git` and leaves the old one bare.
fn renamed_to_non_ascii() -> (graft_fixtures::Scratch, Oid) {
    let (scratch, _) = with_file("plain.txt", "one\ntwo\nthree\nfour\nfive\n");
    let dir = scratch.path();
    git_ok(dir, &["mv", "plain.txt", "café.txt"]);
    std::fs::write(dir.join("café.txt"), "one\ntwo\nTHREE\nfour\nfive\n")
        .unwrap_or_else(|why| panic!("writing café.txt: {why}"));
    git_ok(dir, &["add", "café.txt"]);
    git_ok(dir, &["commit", "-q", "-m", "rename to café"]);
    let head = rev(dir, "HEAD");
    (scratch, head)
}

#[test]
fn a_mixed_quoted_diff_git_header_parses() {
    let (scratch, head) = renamed_to_non_ascii();
    let patch = git_ok(
        scratch.path(),
        &[
            "diff",
            "-M",
            "--no-color",
            &format!("{head}~"),
            &head.to_string(),
        ],
    );
    assert!(
        patch.starts_with(b"diff --git a/plain.txt \"b/caf\\303\\251.txt\"\n"),
        "the shape under test: {}",
        String::from_utf8_lossy(&patch)
    );
    let files = graft_git::diff::parse_unified(&patch)
        .unwrap_or_else(|why| panic!("git's own rename patch: {why:?}"));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].entry.path, "café.txt".as_bytes());
    assert_eq!(files[0].entry.old_path.as_deref(), Some(&b"plain.txt"[..]));
    assert!(
        matches!(files[0].entry.status, FileStatus::Renamed { .. }),
        "{:?}",
        files[0].entry.status
    );
}

#[test]
fn line_history_crosses_a_non_ascii_rename() {
    let (scratch, head) = renamed_to_non_ascii();
    let mut rows = Vec::new();
    let end = line_history(
        &open(scratch.path()),
        head,
        Path::new("café.txt"),
        &LineRange::Block { start: 2, end: 4 },
        1000,
        &mut |batch| rows.extend(batch),
        &(),
    )
    .unwrap_or_else(|why| panic!("line history across the rename: {why}"));
    let paths: Vec<&Path> = rows.iter().map(|row| row.path.as_path()).collect();
    assert_eq!(paths, [Path::new("café.txt"), Path::new("plain.txt")]);
    assert!(matches!(end, TraceEnd::Start { .. }), "{end:?}");
}

/// A `--depth <depth>` clone of the session fixture at `<temp>/clone`
/// (m7-power-tools/18c). `file://` so git honours `--depth`: a plain local
/// path ignores it.
fn shallow_clone(depth: u32) -> tempfile::TempDir {
    let source = graft_fixtures::borrow(Shape::Session);
    let temp = tempfile::tempdir().unwrap_or_else(|why| panic!("a temp dir: {why}"));
    let url = format!("file://{}", source.display());
    let depth = depth.to_string();
    git_ok(
        temp.path(),
        &[
            "clone", "-q", "--depth", &depth, "--branch", "main", &url, "clone",
        ],
    );
    temp
}

fn block() -> LineRange {
    LineRange::Block { start: 18, end: 29 }
}

fn shallow_end() -> TraceEnd {
    TraceEnd::GitStops(SHALLOW_END.to_owned())
}

#[test]
fn a_shallow_boundary_ends_line_history_as_shallow_not_start() {
    let temp = shallow_clone(2);
    let dir = temp.path().join("clone");
    let boundary = rev(&dir, "HEAD~1");
    let (rows, end) = trace_lines(&dir, rev(&dir, "HEAD"), &block(), 1000);
    assert!(rows.iter().all(|row| row.id != boundary), "{rows:?}");
    assert_eq!(rows.len(), 1, "only the commit above the boundary");
    assert_eq!(end, shallow_end());
}

#[test]
fn a_shallow_boundary_ends_file_history_as_shallow_not_start() {
    let temp = shallow_clone(2);
    let dir = temp.path().join("clone");
    let boundary = rev(&dir, "HEAD~1");
    let (rows, end) = trace_file(&dir, rev(&dir, "HEAD"), 1000);
    assert!(rows.iter().all(|row| row.id != boundary), "{rows:?}");
    assert_eq!(rows.len(), 1, "only the commit above the boundary");
    assert_eq!(end, shallow_end());
}

#[test]
fn a_depth_one_clone_traces_no_rows_and_says_shallow() {
    let temp = shallow_clone(1);
    let dir = temp.path().join("clone");
    let head = rev(&dir, "HEAD");
    let (rows, end) = trace_lines(&dir, head, &block(), 1000);
    assert!(rows.is_empty(), "{rows:?}");
    assert_eq!(end, shallow_end());
    let (rows, end) = trace_file(&dir, head, 1000);
    assert!(rows.is_empty(), "{rows:?}");
    assert_eq!(end, shallow_end());
}

#[test]
fn a_shallow_clone_deep_enough_still_ends_at_the_add() {
    // Depth 6: Ana's root is the boundary; Tom's commit is inside.
    let temp = shallow_clone(6);
    let dir = temp.path().join("clone");
    let tom = rev(&dir, "HEAD~4");
    let (rows, end) = trace_lines(&dir, rev(&dir, "HEAD"), &block(), 1000);
    let shallow = std::fs::read_to_string(dir.join(".git").join("shallow"))
        .expect("the clone is shallow: git wrote its boundary list");
    assert_eq!(
        shallow.trim(),
        rev(&dir, "HEAD~5").to_string(),
        "Ana's commit"
    );
    assert_eq!(end, TraceEnd::Start { added_by: tom });
    assert_eq!(rows.last().map(|row| row.author.as_str()), Some("Tom"));
    let oracle = git_ok(
        &dir,
        &[
            "log",
            "--format=%H",
            "-s",
            "-L18,29:src/auth/session.rs",
            "HEAD",
        ],
    );
    let expected: Vec<Oid> = String::from_utf8_lossy(&oracle)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|hex| Oid::from_hex(hex).expect("an id"))
        .collect();
    let ours: Vec<Oid> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ours, expected);
}

#[test]
fn a_linked_worktree_of_a_shallow_clone_sees_the_boundary() {
    let temp = shallow_clone(2);
    let dir = temp.path().join("clone");
    let linked = temp.path().join("linked");
    let linked_arg = linked.to_str().expect("a UTF-8 temp path");
    git_ok(
        &dir,
        &["worktree", "add", "-q", "--detach", linked_arg, "HEAD"],
    );
    let named =
        String::from_utf8(git_ok(&linked, &["rev-parse", "--git-path", "shallow"])).expect("UTF-8");
    let named = linked.join(named.trim());
    let ours = open(&linked).common_dir().join("shallow");
    assert_eq!(
        named.canonicalize().expect("git's shallow file"),
        ours.canonicalize().expect("<common_dir>/shallow"),
        "the oracle names the same file"
    );
    let (rows, end) = trace_lines(&linked, rev(&linked, "HEAD"), &block(), 1000);
    assert_eq!(rows.len(), 1);
    assert_eq!(end, shallow_end());
}

#[test]
fn an_unreadable_shallow_file_is_an_error() {
    let temp = shallow_clone(2);
    let dir = temp.path().join("clone");
    let head = rev(&dir, "HEAD");
    let shallow = dir.join(".git").join("shallow");
    std::fs::remove_file(&shallow).expect("removing shallow");
    std::fs::create_dir(&shallow).expect("shallow as a directory");
    let result = line_history(
        &open(&dir),
        head,
        Path::new(FILE),
        &block(),
        1000,
        &mut |_| {},
        &(),
    );
    match result {
        Err(TraceError::Shallow { path, .. }) => assert_eq!(path, shallow),
        other => panic!("expected Err(Shallow), got {other:?}"),
    }
}
