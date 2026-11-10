//! `snapshot_reads` against real `git` (m7-power-tools/30r, ADR-0052): the
//! in-process reads `raw_record` makes instead of `diff-tree` and `config`
//! must give git's own answers, because a wrong "no" loses his bytes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::AtomicBool;

use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::snapshot_reads::{ChangedFile, ReadError, autocrlf_converts, changed_files};

fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

/// Runs `git` hermetically in `dir`, `stdin` fed and `env` added.
fn git_with(dir: &Path, args: &[&str], stdin: &[u8], env: &[(&str, &Path)]) -> Output {
    use std::io::Write;

    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap_or_else(|why| {
        panic!("running git (mergewright-git does not run without it): {why}")
    });
    let Some(mut input) = child.stdin.take() else {
        panic!("git's stdin is piped");
    };
    input
        .write_all(stdin)
        .unwrap_or_else(|why| panic!("git {args:?} reads its stdin: {why}"));
    drop(input);
    child
        .wait_with_output()
        .unwrap_or_else(|why| panic!("git {args:?} finishes: {why}"))
}

/// [`git_with`] with nothing extra, asserting success; stdout.
fn git_ok(dir: &Path, args: &[&str]) -> Vec<u8> {
    ok(dir, args, git_with(dir, args, b"", &[]))
}

fn ok(dir: &Path, args: &[&str], output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn oid(printed: &[u8]) -> Oid {
    let hex = String::from_utf8_lossy(printed);
    Oid::from_hex(hex.trim()).unwrap_or_else(|why| panic!("{hex:?} is not an id: {why}"))
}

/// A blank directory with a fresh repository in `name` under it.
fn fresh(name: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap_or_else(|why| panic!("a temp directory: {why}"));
    let dir = root.path().join(name);
    std::fs::create_dir_all(&dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
    git_ok(&dir, &["init", "-q", "-b", "main"]);
    (root, dir)
}

// ---------------------------------------------------------------------------
// changed_files

/// What a tree holds at a path.
enum Item {
    File(&'static [u8]),
    Exec(&'static [u8]),
    Link(&'static [u8]),
    Gitlink(&'static str),
}

use Item::{Exec, File, Gitlink, Link};

/// A tree of exactly `items`, written by git (`update-index --index-info` on
/// a scratch index, then `write-tree`), so a path needs no file on disk: a
/// non-UTF-8 name is refused by APFS, not by git.
fn tree(dir: &Path, items: &[(&[u8], Item)]) -> Oid {
    let index = dir.join(".git/snapshot-reads-index");
    if index.exists() {
        std::fs::remove_file(&index)
            .unwrap_or_else(|why| panic!("the old scratch index goes: {why}"));
    }
    let mut info = Vec::new();
    for (path, item) in items {
        let (mode, id) = match item {
            File(bytes) => ("100644", blob(dir, bytes)),
            Exec(bytes) => ("100755", blob(dir, bytes)),
            Link(target) => ("120000", blob(dir, target)),
            Gitlink(hex) => ("160000", oid(hex.as_bytes())),
        };
        info.extend_from_slice(format!("{mode} {id}\t").as_bytes());
        info.extend_from_slice(path);
        info.push(0);
    }
    let env = [("GIT_INDEX_FILE", index.as_path())];
    let args = ["update-index", "-z", "--index-info"];
    ok(dir, &args, git_with(dir, &args, &info, &env));
    let written = oid(&ok(
        dir,
        &["write-tree"],
        git_with(dir, &["write-tree"], b"", &env),
    ));
    std::fs::remove_file(&index).unwrap_or_else(|why| panic!("the scratch index goes: {why}"));
    written
}

fn blob(dir: &Path, bytes: &[u8]) -> Oid {
    let args = ["hash-object", "-w", "--stdin"];
    oid(&ok(dir, &args, git_with(dir, &args, bytes, &[])))
}

fn commit(dir: &Path, tree: Oid) -> Oid {
    oid(&git_ok(dir, &["commit-tree", &tree.to_string(), "-m", "c"]))
}

/// git's answer: `diff-tree -r -z --no-renames` (from the empty tree when
/// unborn), kept to regular files on the new side, sorted by path bytes.
fn oracle(dir: &Path, head: Option<Oid>, to: Oid) -> Vec<ChangedFile> {
    let empty_tree = || {
        let args = ["hash-object", "-t", "tree", "--stdin"];
        oid(&ok(dir, &args, git_with(dir, &args, b"", &[]))).to_string()
    };
    let from = head.map_or_else(empty_tree, |head| head.to_string());
    let stdout = git_ok(
        dir,
        &[
            "diff-tree",
            "-r",
            "-z",
            "--no-renames",
            &from,
            &to.to_string(),
        ],
    );
    let fields: Vec<&[u8]> = stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect();
    let mut files = Vec::new();
    for pair in fields.chunks(2) {
        let [meta, path] = pair else {
            panic!("diff-tree printed a torn record: {pair:?}");
        };
        let words: Vec<&[u8]> = meta.split(|byte| *byte == b' ').collect();
        let [_, new_mode, _, new_oid, _] = words.as_slice() else {
            panic!("diff-tree printed {:?}", String::from_utf8_lossy(meta));
        };
        if *new_mode == b"100644" || *new_mode == b"100755" {
            files.push(ChangedFile {
                path: path.to_vec(),
                executable: *new_mode == b"100755",
                blob: oid(new_oid),
            });
        }
    }
    files.sort_by(|one, other| one.path.cmp(&other.path));
    files
}

fn paths(files: &[ChangedFile]) -> Vec<Vec<u8>> {
    files.iter().map(|file| file.path.clone()).collect()
}

/// `changed_files` from `before` (committed as `HEAD`) to `after`, the handle
/// opened before either tree is written (as `raw_record`'s is): git's answer,
/// and exactly `expected`'s paths, so an oracle that agreed on nothing could
/// not pass. Returns the read.
fn agrees(
    before: &[(&[u8], Item)],
    after: &[(&[u8], Item)],
    expected: &[&[u8]],
) -> Vec<ChangedFile> {
    let (_root, dir) = fresh("repo");
    let handle = open(&dir);
    let head = commit(&dir, tree(&dir, before));
    let to = tree(&dir, after);
    let read = changed_files(&handle, Some(head), to, &())
        .unwrap_or_else(|why| panic!("changed_files: {why}"));
    assert_eq!(read, oracle(&dir, Some(head), to), "agrees with diff-tree");
    assert_eq!(paths(&read), expected, "the paths");
    read
}

const C1: &str = "1111111111111111111111111111111111111111";
const C2: &str = "2222222222222222222222222222222222222222";

#[test]
fn changed_files_agrees_with_diff_tree_on_adds_edits_and_deletes() {
    agrees(
        &[(b"k", File(b"k")), (b"m", File(b"m1")), (b"d", File(b"d"))],
        &[
            (b"k", File(b"k")),
            (b"m", File(b"m2")),
            (b"new", File(b"n")),
        ],
        &[b"m", b"new"],
    );
}

#[test]
fn changed_files_agrees_with_diff_tree_on_a_mode_change_alone() {
    let read = agrees(
        &[(b"x.sh", File(b"x")), (b"y.sh", Exec(b"y"))],
        &[(b"x.sh", Exec(b"x")), (b"y.sh", File(b"y"))],
        &[b"x.sh", b"y.sh"],
    );
    assert!(read[0].executable && !read[1].executable, "{read:?}");
}

/// A file where a symlink or a directory was is named, even on the same
/// blob; a symlink where a file was is not; a directory where a file was
/// names the files under it.
#[test]
fn changed_files_agrees_with_diff_tree_on_type_changes() {
    agrees(
        &[(b"s", Link(b"t")), (b"f", File(b"f"))],
        &[(b"s", File(b"t")), (b"f", Link(b"f"))],
        &[b"s"],
    );
    agrees(
        &[(b"dir/x", File(b"x")), (b"file", File(b"f"))],
        &[(b"dir", File(b"d")), (b"file/y", File(b"y"))],
        &[b"dir", b"file/y"],
    );
}

#[test]
fn changed_files_agrees_with_diff_tree_on_gitlinks() {
    agrees(
        &[
            (b"sub", Gitlink(C1)),
            (b"g2", File(b"g")),
            (b"h", Gitlink(C1)),
        ],
        &[
            (b"sub", Gitlink(C2)),
            (b"g2", Gitlink(C1)),
            (b"g3", Gitlink(C1)),
            (b"h", File(b"h")),
        ],
        &[b"h"],
    );
}

#[test]
fn changed_files_agrees_with_diff_tree_under_unchanged_subtrees() {
    agrees(
        &[
            (b"deep/a/b/c.txt", File(b"c")),
            (b"deep/a/other.txt", File(b"o1")),
            (b"deep/z/q.txt", File(b"q")),
            (b"top", File(b"t")),
        ],
        &[
            (b"deep/a/b/c.txt", File(b"c")),
            (b"deep/a/other.txt", File(b"o2")),
            (b"deep/z/q.txt", File(b"q")),
            (b"top", File(b"t")),
        ],
        &[b"deep/a/other.txt"],
    );
}

/// Paths are bytes: a newline and a non-UTF-8 name come back as git wrote
/// them.
#[test]
fn changed_files_agrees_with_diff_tree_on_odd_paths() {
    agrees(
        &[(b"dir/odd\nname.rs", File(b"1"))],
        &[
            (b"dir/odd\nname.rs", File(b"2")),
            (b"new\nline", File(b"x")),
        ],
        &[b"dir/odd\nname.rs", b"new\nline"],
    );
    agrees(
        &[(b"caf\xe9.txt", File(b"1"))],
        &[(b"caf\xe9.txt", File(b"2")), (b"\xff\xfe", File(b"x"))],
        &[b"caf\xe9.txt", b"\xff\xfe"],
    );
}

/// Unborn: every regular file, never a link or a gitlink.
#[test]
fn changed_files_agrees_with_diff_tree_when_unborn() {
    let (_root, dir) = fresh("repo");
    let to = tree(
        &dir,
        &[
            (b"a", File(b"a")),
            (b"l", Link(b"a")),
            (b"g", Gitlink(C1)),
            (b"d/e", Exec(b"e")),
        ],
    );
    let read = changed_files(&open(&dir), None, to, &()).expect("unborn reads");
    assert_eq!(read, oracle(&dir, None, to), "unborn");
    assert_eq!(paths(&read), [b"a".as_slice(), b"d/e"], "unborn");
    assert!(read[1].executable, "d/e is 100755");
}

/// `raw_record` is handed a tree `git write-tree` wrote a moment ago, after
/// the handle opened: the in-process read must find it.
#[test]
fn a_tree_written_after_the_handle_opened_is_read() {
    let (_root, dir) = fresh("repo");
    let head = commit(&dir, tree(&dir, &[(b"a.rs", File(b"a\n"))]));
    let handle = open(&dir);

    std::fs::write(dir.join("a.rs"), b"a\nchanged\n").expect("a.rs writes");
    std::fs::write(dir.join("b.rs"), b"b\n").expect("b.rs writes");
    git_ok(&dir, &["add", "-A"]);
    let to = oid(&git_ok(&dir, &["write-tree"]));

    let read = changed_files(&handle, Some(head), to, &()).expect("the new tree reads");
    assert_eq!(read, oracle(&dir, Some(head), to));
    assert_eq!(paths(&read), [b"a.rs".as_slice(), b"b.rs"]);
}

/// An object that is not there is an error, never "nothing changed".
#[test]
fn a_missing_tree_is_an_error_not_an_empty_list() {
    let (_root, dir) = fresh("repo");
    let head = commit(&dir, tree(&dir, &[(b"a", File(b"a"))]));
    let missing = Oid::from_hex(C2).expect("an id");

    let read = changed_files(&open(&dir), Some(head), missing, &());
    assert!(
        matches!(read, Err(ReadError::Object { oid, .. }) if oid == missing),
        "{read:?}"
    );
}

/// A stop asked before the walk is a stop, not a partial list.
#[test]
fn a_stop_is_a_stop() {
    let (_root, dir) = fresh("repo");
    let head = commit(&dir, tree(&dir, &[(b"a", File(b"a"))]));
    let to = tree(&dir, &[(b"a", File(b"b"))]);

    let read = changed_files(&open(&dir), Some(head), to, &AtomicBool::new(true));
    assert!(matches!(read, Err(ReadError::Stopped)), "{read:?}");
}

// ---------------------------------------------------------------------------
// autocrlf_converts

/// git's reading, in the environment the in-process read sees: the test
/// process's own, so global and system config are **not** isolated here
/// (both readers see the same ones). Unset (exit 1) is `false`; otherwise
/// git's own boolean parser decides (`--type=bool`), and a value it refuses
/// (`input`, `maybe`) is not git-false, so `true`.
fn git_reads_autocrlf(dir: &Path) -> bool {
    let as_mergewright = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("LC_ALL", "C")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_COMMON_DIR")
            .output()
            .unwrap_or_else(|why| panic!("git {args:?} runs: {why}"))
    };
    let raw = as_mergewright(&["config", "--get", "core.autocrlf"]);
    match raw.status.code() {
        Some(1) => return false,
        Some(0) => {}
        other => panic!(
            "config --get exited {other:?}: {}",
            String::from_utf8_lossy(&raw.stderr)
        ),
    }
    let typed = as_mergewright(&["config", "--type=bool", "--get", "core.autocrlf"]);
    if typed.status.success() {
        String::from_utf8_lossy(&typed.stdout).trim() == "true"
    } else {
        true
    }
}

/// Appends `text` to the repository's own config file, for shapes
/// `git config` does not write (a bare key).
fn append_config(dir: &Path, text: &str) {
    use std::io::Write;
    let config = dir.join(".git/config");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&config)
        .unwrap_or_else(|why| panic!("{}: {why}", config.display()));
    file.write_all(text.as_bytes())
        .unwrap_or_else(|why| panic!("{}: {why}", config.display()));
}

/// A config file outside `.git` that turns `core.autocrlf` on; its path.
fn extra_config(root: &Path) -> String {
    let extra = root.join("extra.gitconfig");
    std::fs::write(&extra, "[core]\n\tautocrlf = true\n")
        .unwrap_or_else(|why| panic!("{}: {why}", extra.display()));
    extra.to_string_lossy().into_owned()
}

/// One case in a repository of its own, named so an `includeIf gitdir:` can
/// match it; `setup` gets the repository and the directory above it.
/// `expected` is `None` where the answer rests on his global or system config
/// (unset), so only agreement with git is asserted there.
fn autocrlf_agrees(name: &str, setup: &dyn Fn(&Path, &Path), expected: Option<bool>) {
    let (root, dir) = fresh("mwr-30r-repo");
    setup(&dir, root.path());
    let read = autocrlf_converts(&open(&dir)).unwrap_or_else(|why| panic!("{name}: {why}"));
    assert_eq!(read, git_reads_autocrlf(&dir), "{name}: agrees with git");
    if let Some(expected) = expected {
        assert_eq!(read, expected, "{name}");
    }
}

#[test]
fn autocrlf_agrees_with_git_config_on_each_value() {
    autocrlf_agrees("unset", &|_, _| {}, None);
    for (value, expected) in [
        ("true", true),
        ("input", true),
        ("false", false),
        ("no", false),
        ("off", false),
        ("0", false),
        ("", false),
        ("TRUE", true),
        ("yes", true),
        ("maybe", true),
    ] {
        let set = |dir: &Path, _: &Path| {
            git_ok(dir, &["config", "core.autocrlf", value]);
        };
        autocrlf_agrees(value, &set, Some(expected));
    }
    let bare = |dir: &Path, _: &Path| append_config(dir, "[core]\n\tautocrlf\n");
    autocrlf_agrees("a bare key", &bare, Some(true));
    for (first, last) in [("true", false), ("false", true)] {
        let twice = |dir: &Path, _: &Path| {
            git_ok(dir, &["config", "--add", "core.autocrlf", first]);
            git_ok(
                dir,
                &["config", "--add", "core.autocrlf", &last.to_string()],
            );
        };
        autocrlf_agrees(&format!("two definitions, {last} last"), &twice, Some(last));
    }
}

#[test]
fn autocrlf_agrees_with_git_config_through_includes_and_worktree_config() {
    let include = |dir: &Path, root: &Path| {
        git_ok(dir, &["config", "include.path", &extra_config(root)]);
    };
    autocrlf_agrees("include.path outside .git", &include, Some(true));
    let matching = |dir: &Path, root: &Path| {
        let key = "includeIf.gitdir:mwr-30r-repo/.path";
        git_ok(dir, &["config", key, &extra_config(root)]);
    };
    autocrlf_agrees("includeIf gitdir: that matches", &matching, Some(true));
    let elsewhere = |dir: &Path, root: &Path| {
        let key = "includeIf.gitdir:elsewhere-30r/.path";
        git_ok(dir, &["config", key, &extra_config(root)]);
    };
    autocrlf_agrees("includeIf gitdir: that does not match", &elsewhere, None);
    let worktree = |dir: &Path, _: &Path| {
        git_ok(dir, &["config", "extensions.worktreeConfig", "true"]);
        git_ok(dir, &["config", "--worktree", "core.autocrlf", "true"]);
        let written = std::fs::read_to_string(dir.join(".git/config.worktree"))
            .expect("git wrote config.worktree");
        assert!(written.contains("autocrlf"), "{written}");
    };
    autocrlf_agrees("config.worktree", &worktree, Some(true));
}

/// A linked worktree's own `config.worktree` is read from its own git
/// directory: on in the linked one, not in the main one.
#[test]
fn a_linked_worktrees_own_config_is_read() {
    let (root, dir) = fresh("main");
    std::fs::write(dir.join("a"), b"a\n").expect("a writes");
    git_ok(&dir, &["add", "-A"]);
    git_ok(&dir, &["commit", "-q", "-m", "first"]);
    git_ok(&dir, &["config", "core.autocrlf", "false"]);
    let linked = root.path().join("linked");
    git_ok(&dir, &["worktree", "add", "-q", &linked.to_string_lossy()]);
    git_ok(&dir, &["config", "extensions.worktreeConfig", "true"]);
    git_ok(&linked, &["config", "--worktree", "core.autocrlf", "true"]);

    let in_linked = autocrlf_converts(&open(&linked)).expect("the linked worktree reads");
    assert_eq!(in_linked, git_reads_autocrlf(&linked));
    assert!(in_linked);
    let in_main = autocrlf_converts(&open(&dir)).expect("the main worktree reads");
    assert_eq!(in_main, git_reads_autocrlf(&dir));
    assert!(!in_main);
}

/// The handle is opened once for the whole run; he sets `core.autocrlf` in
/// his shell after that. The read must see it (the handle's own config
/// snapshot is from launch).
#[test]
fn a_config_change_after_the_handle_opened_is_seen() {
    let (_root, dir) = fresh("repo");
    git_ok(&dir, &["config", "core.autocrlf", "false"]);
    let handle = open(&dir);
    assert!(!autocrlf_converts(&handle).expect("the config reads"));

    git_ok(&dir, &["config", "core.autocrlf", "true"]);
    assert!(autocrlf_converts(&handle).expect("the config reads again"));
}

// ---------------------------------------------------------------------------
// autocrlf_converts, config sources outside the repository
// (m8-polish-and-ship/21 items 4 and 5)
//
// The in-process read sees the test process's own environment, and the
// workspace denies `unsafe` (so no `set_var`): each case runs the ignored
// child below in a fresh copy of this test binary, with a fully controlled
// config environment, and compares Mergewright's answer with git's under it.

const CHILD_REPO: &str = "MWR_AUTOCRLF_CHILD_REPO";
const CHILD_OUT: &str = "MWR_AUTOCRLF_CHILD_OUT";

/// The child half: Mergewright's read and git's, `mwr git` in `CHILD_OUT`.
#[test]
#[ignore = "run by autocrlf_in(), in an environment it controls"]
fn child_reads_autocrlf() {
    let repo = PathBuf::from(std::env::var_os(CHILD_REPO).expect("the parent names the repo"));
    let out = PathBuf::from(std::env::var_os(CHILD_OUT).expect("the parent names the output"));
    let mwr = autocrlf_converts(&open(&repo)).expect("the config reads");
    let git = git_reads_autocrlf(&repo);
    std::fs::write(out, format!("{mwr} {git}")).expect("the answer is written");
}

/// `(mwr, git)` for a repository with no `core.autocrlf` of its own, in a
/// process whose only config outside the repository comes from `env`:
/// `HOME` and `XDG_CONFIG_HOME` point at an empty directory, the global file
/// is `/dev/null` and the system file is off, unless `env` says otherwise.
fn autocrlf_in(env: &[(&str, String)]) -> (bool, bool) {
    let (root, dir) = fresh("repo");
    let home = root.path().join("home");
    std::fs::create_dir_all(&home).unwrap_or_else(|why| panic!("the home is made: {why}"));
    let out = root.path().join("out");
    let binary = std::env::current_exe().unwrap_or_else(|why| panic!("the test binary: {why}"));
    let mut child = Command::new(binary);
    child
        .args([
            "--exact",
            "snapshot_reads::child_reads_autocrlf",
            "--ignored",
            "--test-threads=1",
        ])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env(CHILD_REPO, &dir)
        .env(CHILD_OUT, &out);
    for (key, value) in env {
        child.env(key, value);
    }
    let output = child
        .output()
        .unwrap_or_else(|why| panic!("the child runs: {why}"));
    assert!(
        output.status.success(),
        "the child failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let answer = std::fs::read_to_string(&out)
        .unwrap_or_else(|why| panic!("the child wrote its answer: {why}"));
    let (mwr, git) = answer
        .split_once(' ')
        .unwrap_or_else(|| panic!("two answers: {answer:?}"));
    (mwr == "true", git == "true")
}

/// A config file holding `core.autocrlf = <value>`, kept alive with the
/// returned directory.
fn config_file(value: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temp directory: {why}"));
    let file = dir.path().join("gitconfig");
    std::fs::write(&file, format!("[core]\n\tautocrlf = {value}\n"))
        .unwrap_or_else(|why| panic!("the file writes: {why}"));
    let path = file.to_string_lossy().into_owned();
    (dir, path)
}

/// Item 4: the global file (`GIT_CONFIG_GLOBAL`), both values, the system
/// file off. Mergewright and git agree, and both see the value.
#[test]
fn autocrlf_from_the_global_config_is_read_isolated() {
    for (value, expected) in [("true", true), ("false", false)] {
        let (_keep, global) = config_file(value);
        let read = autocrlf_in(&[("GIT_CONFIG_GLOBAL", global)]);
        assert_eq!(read, (expected, expected), "global {value}: (mwr, git)");
    }
}

/// Item 4: the system file (`GIT_CONFIG_SYSTEM`), both values, the global
/// file empty.
#[test]
fn autocrlf_from_the_system_config_is_read_isolated() {
    for (value, expected) in [("true", true), ("false", false)] {
        let (_keep, system) = config_file(value);
        let read = autocrlf_in(&[
            ("GIT_CONFIG_NOSYSTEM", "0".to_owned()),
            ("GIT_CONFIG_SYSTEM", system),
        ]);
        assert_eq!(read, (expected, expected), "system {value}: (mwr, git)");
    }
}

/// Item 5: config given in the environment, which git (and so every git
/// spawn) honours: `GIT_CONFIG_COUNT`/`_KEY_n`/`_VALUE_n`, and
/// `GIT_CONFIG_PARAMETERS` (what `git -c` passes to its children).
#[test]
fn autocrlf_from_environment_config_agrees_with_git() {
    for (value, expected) in [("true", true), ("false", false)] {
        let count = autocrlf_in(&[
            ("GIT_CONFIG_COUNT", "1".to_owned()),
            ("GIT_CONFIG_KEY_0", "core.autocrlf".to_owned()),
            ("GIT_CONFIG_VALUE_0", value.to_owned()),
        ]);
        assert_eq!(count, (expected, expected), "GIT_CONFIG_COUNT {value}");
        let parameters = autocrlf_in(&[(
            "GIT_CONFIG_PARAMETERS",
            format!("'core.autocrlf'='{value}'"),
        )]);
        assert_eq!(
            parameters,
            (expected, expected),
            "GIT_CONFIG_PARAMETERS {value}"
        );
        // Both set, disagreeing: git takes `GIT_CONFIG_PARAMETERS` last.
        let both = autocrlf_in(&[
            ("GIT_CONFIG_COUNT", "1".to_owned()),
            ("GIT_CONFIG_KEY_0", "core.autocrlf".to_owned()),
            ("GIT_CONFIG_VALUE_0", (!expected).to_string()),
            (
                "GIT_CONFIG_PARAMETERS",
                format!("'core.autocrlf'='{value}'"),
            ),
        ]);
        assert_eq!(both, (expected, expected), "both, PARAMETERS {value}");
    }
    let bare = autocrlf_in(&[("GIT_CONFIG_PARAMETERS", "'core.autocrlf'".to_owned())]);
    assert_eq!(bare, (true, true), "a bare key in GIT_CONFIG_PARAMETERS");
    let old_style = autocrlf_in(&[(
        "GIT_CONFIG_PARAMETERS",
        "'user.name=x' 'Core.AutoCRLF=input'".to_owned(),
    )]);
    assert_eq!(old_style, (true, true), "old style, mixed-case key, input");
}
