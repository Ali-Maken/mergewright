//! m7-power-tools/30q-a (ADR-0052): what the before-snapshot keeps raw.
//!
//! A path is kept raw when it is uncommitted, git would convert it, and git's
//! own checkout of the cleaned blob would not give back its bytes on disk.
//! Each test builds that on real git and reads the raw tree back with
//! `ls-tree` and `cat-file`, never from a returned value alone.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, JournalEntry};
use graft_core::oid::Oid;
use graft_ops::apply::ApplyError;
use graft_ops::operation::{Operation, Target};
use graft_ops::snapshot;

use crate::support::{NOW, Never, TempRepo, run};

/// `HEAD`'s commit, `None` while unborn.
fn head_of(repo: &TempRepo) -> Option<Oid> {
    repo.try_git(&["rev-parse", "-q", "--verify", "HEAD"])
        .ok()
        .map(|hex| Oid::from_hex(&hex).expect("HEAD is an id"))
}

/// `raw_record` over the repository as it stands: the worktree tree first,
/// as `take` has it, then `HEAD` (absent when unborn). The scratch directory
/// is left empty: nothing the record writes there survives it.
fn raw_of(repo: &TempRepo) -> Option<Oid> {
    recorded(repo).0
}

/// [`raw_of`], with the `git` subcommands `raw_record` ran, in order
/// (m7-power-tools/30r: each one is a spawn, about 6 ms on the perf machine).
fn recorded(repo: &TempRepo) -> (Option<Oid>, Vec<String>) {
    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let tree = snapshot::worktree_tree(&handle, &scratch, &Never).expect("the worktree tree");
    let mut commands = Vec::new();
    let raw = snapshot::raw_record(
        &handle,
        &scratch,
        &Never,
        head_of(repo),
        tree,
        &mut commands,
    )
    .expect("the raw record is taken");
    let left: Vec<PathBuf> = std::fs::read_dir(&scratch)
        .expect("the scratch reads")
        .map(|entry| entry.expect("the entry reads").path())
        .collect();
    assert!(left.is_empty(), "the scratch is left empty: {left:?}");
    (raw, subcommands(&commands))
}

/// `git check-attr -z …` → `check-attr`.
fn subcommands(commands: &[String]) -> Vec<String> {
    commands
        .iter()
        .map(|line| {
            let mut words = line.split(' ');
            assert_eq!(words.next(), Some("git"), "{line}");
            words.next().expect("a subcommand").to_owned()
        })
        .collect()
}

/// The before-snapshot, as `begin` takes it.
fn taken(repo: &TempRepo) -> snapshot::Snapshot {
    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let id = EntryId::new(NOW, [7; 10]);
    let head = head_of(repo);
    snapshot::take(&handle, &scratch, &id, "main", "probe", &[], head, &Never)
        .expect("the snapshot is taken")
}

/// The names at the top of a backup commit's tree, in git's order.
fn top_names(repo: &TempRepo, commit: Oid) -> Vec<String> {
    repo.git(&["ls-tree", "--name-only", &commit.to_string()])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Stage `path` through `apply` (a full-scope operation, so a before-snapshot
/// is taken), and hand back the entry and its pending line as written.
fn staged(repo: &TempRepo, path: &str) -> (JournalEntry, String) {
    let entry = run(
        repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from(path)])),
        NOW,
    );
    let journal = std::fs::read_to_string(repo.path().join(".git/graft/undo-journal.jsonl"))
        .expect("the journal reads");
    let pending = journal.lines().next().expect("a pending line").to_owned();
    assert!(pending.contains("\"status\":\"pending\""), "{pending}");
    (entry, pending)
}

/// `git <args>`'s stdout as bytes, untrimmed.
fn git_bytes(repo: &TempRepo, args: &[&str]) -> Vec<u8> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// `ls-tree -r` of the raw tree as `(mode, path)`, in git's order.
fn listed(repo: &TempRepo, raw: Oid) -> Vec<(String, String)> {
    repo.git(&["ls-tree", "-r", &raw.to_string()])
        .lines()
        .map(|line| {
            let (head, path) = line.split_once('\t').expect("a tab before the path");
            let mode = head.split(' ').next().expect("a mode");
            (mode.to_owned(), path.to_owned())
        })
        .collect()
}

/// The blob the raw tree holds at `path`.
fn kept(repo: &TempRepo, raw: Oid, path: &str) -> Vec<u8> {
    git_bytes(repo, &["cat-file", "blob", &format!("{raw}:{path}")])
}

fn executable(repo: &TempRepo, path: &str) {
    std::fs::set_permissions(
        repo.path().join(path),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("the exec bit is set");
}

fn symlink(repo: &TempRepo, path: &str, to: &str) {
    let at = repo.path().join(path);
    if std::fs::symlink_metadata(&at).is_ok() {
        std::fs::remove_file(&at).expect("the old link goes");
    }
    std::os::unix::fs::symlink(to, at).expect("the link is made");
}

/// No attributes, `core.autocrlf=false` (the scratch repo's own config), two
/// changed files: git converts nothing, so nothing is kept. The backup commit
/// keeps today's layout and the pending line today's keys.
#[test]
fn a_snapshot_with_nothing_to_convert_keeps_nothing_raw() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"a\n");
    repo.write("b.rs", b"b\n");
    repo.commit_all("first");
    repo.write("a.rs", b"a changed\n");
    repo.write("b.rs", b"b changed\r\n");

    assert_eq!(raw_of(&repo), None);

    let snapshot = taken(&repo);
    assert_eq!(snapshot.worktree_raw, None);
    assert_eq!(top_names(&repo, snapshot.commit), ["index", "worktree"]);

    let (entry, pending) = staged(&repo, "a.rs");
    assert_eq!(entry.worktree_raw_before, None);
    assert!(!pending.contains("worktree_raw_before"), "{pending}");
}

/// `core.autocrlf=true`: only the changed LF files are kept. The CRLF file
/// comes back from git's checkout as it is, the unchanged file is in
/// history, and a symlink is never filtered.
#[test]
fn a_snapshot_keeps_raw_only_what_would_not_round_trip() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("c.rs", b"c\n");
    repo.write("l.rs", b"l\n");
    repo.write("run.sh", b"#!/bin/sh\n");
    repo.write("old.rs", b"old\n");
    symlink(&repo, "link", "c.rs");
    repo.commit_all("first");

    repo.write("c.rs", b"c\r\nchanged\r\n");
    repo.write("l.rs", b"l\nchanged\n");
    repo.write("run.sh", b"#!/bin/sh\necho changed\n");
    executable(&repo, "run.sh");
    symlink(&repo, "link", "l.rs");

    let raw = raw_of(&repo).expect("something is kept");
    assert_eq!(
        listed(&repo, raw),
        vec![
            ("100644".to_owned(), "l.rs".to_owned()),
            ("100755".to_owned(), "run.sh".to_owned()),
        ]
    );
    assert_eq!(kept(&repo, raw, "l.rs"), repo.read("l.rs"));
    assert_eq!(kept(&repo, raw, "run.sh"), repo.read("run.sh"));

    // `take` joins it as `raw/` beside an unchanged `worktree/`.
    let snapshot = taken(&repo);
    assert_eq!(snapshot.worktree_raw, Some(raw));
    assert_eq!(
        top_names(&repo, snapshot.commit),
        ["index", "raw", "worktree"]
    );
    let at = |name: &str| repo.git(&["rev-parse", &format!("{}:{name}", snapshot.commit)]);
    assert_eq!(at("raw"), raw.to_string());
    assert_eq!(at("worktree"), snapshot.worktree_tree.to_string());

    // `begin` records it on the pending line.
    let (entry, pending) = staged(&repo, "c.rs");
    assert_eq!(entry.worktree_raw_before, Some(raw));
    assert!(
        pending.contains(&format!("\"worktree_raw_before\":\"{raw}\"")),
        "{pending}"
    );
}

/// A clean filter that throws a line away and a smudge that is `cat`: the
/// line is only on disk, so the file is kept, line and all.
#[test]
fn a_one_way_clean_filter_is_kept_raw() {
    let repo = TempRepo::init();
    std::fs::write(
        repo.path().join(".git/info/attributes"),
        b"nb.txt filter=strip\n",
    )
    .expect("the attributes are written");
    repo.git(&["config", "filter.strip.clean", "sed '/^#out/d'"]);
    repo.git(&["config", "filter.strip.smudge", "cat"]);
    repo.write("nb.txt", b"cell 1\n");
    repo.commit_all("first");
    repo.write("nb.txt", b"cell 1\n#out 42\ncell 2\n");

    let raw = raw_of(&repo).expect("the notebook is kept");
    assert_eq!(
        listed(&repo, raw),
        vec![("100644".to_owned(), "nb.txt".to_owned())]
    );
    assert_eq!(kept(&repo, raw, "nb.txt"), b"cell 1\n#out 42\ncell 2\n");
}

/// `core.autocrlf=input` cleans CRLF to LF and writes LF back: a changed CRLF
/// file is kept.
#[test]
fn autocrlf_input_keeps_a_crlf_file_raw() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "input"]);
    repo.write("x.rs", b"x\n");
    repo.commit_all("first");
    repo.write("x.rs", b"x\r\nchanged\r\n");

    let raw = raw_of(&repo).expect("the CRLF file is kept");
    assert_eq!(
        listed(&repo, raw),
        vec![("100644".to_owned(), "x.rs".to_owned())]
    );
    assert_eq!(kept(&repo, raw, "x.rs"), b"x\r\nchanged\r\n");
}

/// 30p-a test 7's `rot` filter: clean and smudge are each other's inverse, so
/// git's checkout gives the bytes back and nothing is kept.
#[test]
fn an_inverse_filter_keeps_nothing() {
    let repo = TempRepo::init();
    std::fs::write(
        repo.path().join(".git/info/attributes"),
        b"a.txt filter=rot\n",
    )
    .expect("the attributes are written");
    let rot = "tr 'A-Za-z' 'N-ZA-Mn-za-m'";
    repo.git(&["config", "filter.rot.clean", rot]);
    repo.git(&["config", "filter.rot.smudge", rot]);
    repo.write("a.txt", b"Hello\n");
    repo.commit_all("first");
    repo.write("a.txt", b"Hello, changed\n");

    assert_eq!(raw_of(&repo), None);
}

/// Before the first commit every file is uncommitted: an LF file under
/// `core.autocrlf=true` is kept, a CRLF one is not.
#[test]
fn an_unborn_head_keeps_an_lf_file_raw() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("new.rs", b"new\n");
    repo.write("crlf.rs", b"crlf\r\n");

    let raw = raw_of(&repo).expect("the LF file is kept");
    assert_eq!(
        listed(&repo, raw),
        vec![("100644".to_owned(), "new.rs".to_owned())]
    );
    assert_eq!(kept(&repo, raw, "new.rs"), b"new\n");
}

/// A path is moved whole through every step, never parsed as a line: a name
/// with a newline and a space in it is kept like any other.
#[test]
fn a_path_with_a_newline_in_it_is_kept_raw() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("dir/odd\nname .rs", b"odd\n");
    repo.commit_all("first");
    repo.write("dir/odd\nname .rs", b"odd\nchanged\n");

    let raw = raw_of(&repo).expect("the odd path is kept");
    let names = git_bytes(
        &repo,
        &["ls-tree", "-r", "-z", "--name-only", &raw.to_string()],
    );
    assert_eq!(names, b"dir/odd\nname .rs\0");
    assert_eq!(kept(&repo, raw, "dir/odd\nname .rs"), b"odd\nchanged\n");
}

/// `raw_record`'s paths are relative to the worktree root: run from a
/// subdirectory's handle, the same file is found and kept.
#[test]
fn a_handle_opened_in_a_subdirectory_keeps_the_same_path() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/deep/a.rs", b"a\n");
    repo.commit_all("first");
    repo.write("src/deep/a.rs", b"a\nchanged\n");

    let handle = repo.open_at("src");
    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let tree = snapshot::worktree_tree(&handle, &scratch, &Never).expect("the worktree tree");
    let head = Oid::from_hex(&repo.git(&["rev-parse", "HEAD"])).expect("HEAD is an id");
    let raw = snapshot::raw_record(&handle, &scratch, &Never, Some(head), tree, &mut Vec::new())
        .expect("the raw record is taken")
        .expect("the file is kept");

    assert_eq!(
        listed(&repo, raw),
        vec![("100644".to_owned(), "src/deep/a.rs".to_owned())]
    );
    assert_eq!(
        kept(&repo, raw, "src/deep/a.rs"),
        std::fs::read(Path::new(repo.path()).join("src/deep/a.rs")).expect("the file reads")
    );
}

// m7-power-tools/30r: what `raw_record` spawns. The gate is about one `git`
// in the common case (`docs/perf/30q-snapshot.txt`); a count is the same on
// every machine, where a time is not.

/// Five changed files, no attributes, `core.autocrlf` unset: only
/// `check-attr` runs (git's own answer on attributes), and nothing is kept.
#[test]
fn a_changed_worktree_with_nothing_to_convert_spawns_one_git() {
    let repo = TempRepo::init();
    repo.git(&["config", "--unset", "core.autocrlf"]);
    let names = ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"];
    for name in names {
        repo.write(name, b"one\n");
    }
    repo.commit_all("first");
    for name in names {
        repo.write(name, b"one\nchanged\n");
    }

    let (raw, spawned) = recorded(&repo);
    assert_eq!(raw, None);
    assert_eq!(spawned, ["check-attr"]);
}

/// A worktree that matches `HEAD` asks nothing of git.
#[test]
fn a_clean_worktree_spawns_no_git() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"a\n");
    repo.write("dir/b.rs", b"b\n");
    repo.commit_all("first");

    let (raw, spawned) = recorded(&repo);
    assert_eq!(raw, None);
    assert!(spawned.is_empty(), "{spawned:?}");
}

/// 30q-a's `l.rs`: a changed LF file under `core.autocrlf=true`. Each of the
/// three questions' `git` runs once, then the raw tree's three.
#[test]
fn a_conversion_candidate_spawns_only_what_it_must() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("l.rs", b"l\n");
    repo.commit_all("first");
    repo.write("l.rs", b"l\nchanged\n");

    let (raw, spawned) = recorded(&repo);
    let raw = raw.expect("the LF file is kept");
    assert_eq!(
        listed(&repo, raw),
        [("100644".to_owned(), "l.rs".to_owned())]
    );
    assert_eq!(
        spawned,
        [
            "check-attr",
            "update-index",
            "checkout-index",
            "hash-object",
            "update-index",
            "write-tree",
        ]
    );
}

/// The handle is opened once for the whole run (`RepoStore`); he sets
/// `core.autocrlf` in his shell after that. `take` must read it as git does
/// now, not as it was at launch: a stale "off" would lose his bytes.
#[test]
fn autocrlf_set_after_the_handle_opened_is_honoured() {
    let repo = TempRepo::init();
    repo.write("l.rs", b"l\n");
    repo.commit_all("first");
    repo.write("l.rs", b"l\nchanged\n");
    let handle = repo.open();
    repo.git(&["config", "core.autocrlf", "true"]);

    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let id = EntryId::new(NOW, [9; 10]);
    let head = head_of(&repo);
    let snapshot = snapshot::take(&handle, &scratch, &id, "main", "probe", &[], head, &Never)
        .expect("the snapshot is taken");

    let raw = snapshot.worktree_raw.expect("the LF file is kept raw");
    assert_eq!(
        listed(&repo, raw),
        [("100644".to_owned(), "l.rs".to_owned())]
    );
    assert_eq!(kept(&repo, raw, "l.rs"), b"l\nchanged\n");
}

/// m8-polish-and-ship/21 item 2: the raw blobs are hashed from paths fed on
/// stdin (`hash-object --stdin-paths`), never one argument per path, so a
/// large kept set cannot pass `ARG_MAX`. 1 000 kept files with 209-byte paths:
/// every one is in the raw tree, and no command line `raw_record` ran is
/// longer than 1 KiB.
#[test]
fn a_large_kept_set_is_hashed_from_stdin_not_the_command_line() {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    let dir = format!("{}/{}", "d".repeat(100), "e".repeat(100));
    let paths: Vec<String> = (0..1000).map(|n| format!("{dir}/{n:04}.rs")).collect();
    for path in &paths {
        repo.write(path, b"a\n");
    }
    repo.commit_all("first");
    for path in &paths {
        repo.write(path, b"a\nchanged\n");
    }

    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let tree = snapshot::worktree_tree(&handle, &scratch, &Never).expect("the worktree tree");
    let mut commands = Vec::new();
    let raw = snapshot::raw_record(
        &handle,
        &scratch,
        &Never,
        head_of(&repo),
        tree,
        &mut commands,
    )
    .expect("the raw record is taken")
    .expect("every path is kept");

    let longest = commands.iter().map(String::len).max().unwrap_or(0);
    assert!(longest <= 1024, "a command line of {longest} bytes");
    assert!(
        commands
            .iter()
            .any(|line| line.contains("hash-object") && line.contains("--stdin-paths")),
        "{commands:?}"
    );
    let listed = listed(&repo, raw);
    assert_eq!(listed.len(), paths.len());
    assert_eq!(kept(&repo, raw, &paths[999]), b"a\nchanged\n");
}

/// A tree the in-process read cannot find is `SnapshotError::Read`, never
/// "nothing to keep"; a refused operation shows it as one line, as it does a
/// `Run` failure (`PrepareError::Snapshot` becomes `ApplyError::Snapshot`).
#[test]
fn an_unreadable_tree_is_a_read_error_in_one_line() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"a\n");
    repo.commit_all("first");
    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let missing = Oid::from_hex(&"2".repeat(40)).expect("an id");

    let mut commands = Vec::new();
    let failed = snapshot::raw_record(
        &handle,
        &scratch,
        &Never,
        head_of(&repo),
        missing,
        &mut commands,
    );
    let Err(why @ snapshot::SnapshotError::Read(_)) = failed else {
        panic!("a read error, got {failed:?}");
    };
    assert!(commands.is_empty(), "{commands:?}");

    let line = ApplyError::Snapshot(why).explain(&crate::support::Letters);
    assert!(!line.contains('\n'), "{line}");
    assert!(
        line.starts_with(&format!("cannot read object {missing}: ")),
        "{line}"
    );
}
