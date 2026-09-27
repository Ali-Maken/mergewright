//! `blame::blame` against git's own `git blame`, on the `session` fixture
//! (m7-power-tools/18).

use std::path::Path;
use std::sync::atomic::AtomicBool;

use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::blame::{Blame, BlameError, BlameOptions, LineMark, Rev, blame};

use crate::session::{FILE, OLD_FILE, commits, git, git_ok, open, rev};

fn blame_ok(dir: &Path, rev: Rev, path: &str, opts: BlameOptions) -> Blame {
    blame(&open(dir), &rev, Path::new(path), &opts, &())
        .unwrap_or_else(|why| panic!("blaming {path}: {why}"))
}

/// The oracle: `git blame --line-porcelain` with the same flags, read by a
/// parser of its own — each entry's first line, `<id> <orig> <final>`.
fn oracle(dir: &Path, rev: Option<Oid>, path: &str) -> Vec<(Option<Oid>, u32)> {
    let rev = rev.map(|id| id.to_string());
    let mut args = vec![
        "blame",
        "--line-porcelain",
        "-w",
        "-M",
        "--ignore-revs-file",
        ".git-blame-ignore-revs",
    ];
    if let Some(rev) = &rev {
        args.push(rev);
    }
    args.extend(["--", path]);
    let out = git_ok(dir, &args);
    out.split(|&b| b == b'\n')
        .filter(|line| !line.starts_with(b"\t"))
        .filter_map(|line| {
            let text = std::str::from_utf8(line).ok()?;
            let mut fields = text.split(' ');
            let id = fields.next()?;
            if id.len() != 40 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let orig: u32 = fields.next()?.parse().ok()?;
            let id = Oid::from_hex(id).ok()?;
            let committed = id.as_bytes().iter().any(|&b| b != 0);
            Some((committed.then_some(id), orig))
        })
        .collect()
}

fn ours(blame: &Blame) -> Vec<(Option<Oid>, u32)> {
    blame
        .lines
        .iter()
        .map(|line| (line.commit, line.orig_line))
        .collect()
}

#[test]
fn blame_matches_git_blame_line_for_line() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    for (rev, path) in [(ids.you, FILE), (ids.priya, FILE), (ids.lina, OLD_FILE)] {
        let blamed = blame_ok(dir, Rev::Commit(rev), path, BlameOptions::default());
        let expected = oracle(dir, Some(rev), path);
        assert!(!expected.is_empty(), "the oracle read nothing at {rev}");
        assert_eq!(ours(&blamed), expected, "blame of {path} at {rev}");
        for (index, line) in blamed.lines.iter().enumerate() {
            assert_eq!(line.final_line as usize, index + 1);
        }
    }

    // The working tree, with one line changed and one added.
    let file = dir.join(FILE);
    let text = std::fs::read_to_string(&file).expect("the fixture file reads");
    let text =
        text.replace("let mut attempt = 0;", "let mut attempt = 1;") + "// a line not committed\n";
    std::fs::write(&file, text).expect("the fixture file writes");
    let blamed = blame_ok(dir, Rev::WorkingTree, FILE, BlameOptions::default());
    assert_eq!(ours(&blamed), oracle(dir, None, FILE), "working-tree blame");
}

#[test]
fn ignore_revs_skips_the_listed_commit_and_counts_it() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let blamed = blame_ok(&dir, Rev::Commit(ids.you), FILE, BlameOptions::default());
    assert!(
        blamed
            .lines
            .iter()
            .all(|line| line.commit != Some(ids.rustfmt)),
        "no line is the rustfmt commit's"
    );
    let found = blamed.ignore_revs.expect("the fixture lists one revision");
    assert_eq!(found.file, Path::new(".git-blame-ignore-revs"));
    assert_eq!(found.revisions, 1);
    assert!(found.applied);
}

#[test]
fn ignore_revs_false_blames_the_rustfmt_commit() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let opts = BlameOptions {
        ignore_revs: false,
        ..BlameOptions::default()
    };
    let blamed = blame_ok(&dir, Rev::Commit(ids.you), FILE, opts);
    let rustfmt: Vec<u32> = blamed
        .lines
        .iter()
        .filter(|line| line.commit == Some(ids.rustfmt))
        .map(|line| line.final_line)
        .collect();
    assert_eq!(rustfmt, [31, 32, 33, 35, 36, 37]);
    assert!(blamed.lines.iter().all(|line| line.mark == LineMark::Plain));
    let found = blamed.ignore_revs.expect("still found, not applied");
    assert!(!found.applied);
    assert_eq!(found.revisions, 1);
}

#[test]
fn ignore_revs_false_clears_a_configured_list_too() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    git_ok(
        dir,
        &["config", "blame.ignoreRevsFile", ".git-blame-ignore-revs"],
    );
    let opts = BlameOptions {
        ignore_revs: false,
        ..BlameOptions::default()
    };
    let blamed = blame_ok(dir, Rev::Commit(ids.you), FILE, opts);
    assert_eq!(blamed.lines[30].commit, Some(ids.rustfmt), "line 31");
}

/// m7-power-tools/18d: an empty `--ignore-revs-file=` does not clear a
/// configured file git cannot open; `--no-ignore-revs-file` does.
#[test]
fn ignore_revs_false_skips_a_configured_file_that_is_missing() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    git_ok(dir, &["config", "blame.ignoreRevsFile", "no-such-file"]);
    let opts = BlameOptions {
        ignore_revs: false,
        ..BlameOptions::default()
    };
    let blamed = blame_ok(dir, Rev::Commit(ids.you), FILE, opts);
    assert_eq!(blamed.lines[30].commit, Some(ids.rustfmt), "line 31");
}

/// The `(None, true)` arm: nothing at the root, a configured file that is
/// missing. git's own fatal comes back, not a quiet blame without the list.
#[test]
fn ignore_revs_true_with_a_missing_configured_file_is_gits_error() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    std::fs::remove_file(dir.join(".git-blame-ignore-revs")).expect("removes");
    git_ok(dir, &["config", "blame.ignoreRevsFile", "no-such-file"]);
    let result = blame(
        &open(dir),
        &Rev::Commit(ids.you),
        Path::new(FILE),
        &BlameOptions::default(),
        &(),
    );
    let why = result.expect_err("git refuses").to_string();
    assert!(why.contains("could not open object name list"), "{why}");
}

#[test]
fn ignore_whitespace_is_on_by_default() {
    assert!(BlameOptions::default().ignore_whitespace);
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    let file = dir.join(FILE);
    let text = std::fs::read_to_string(&file).expect("reads");
    // Line 40, Priya's, re-indented: a whitespace-only change.
    let text = text.replace(
        "        self.age() > self.ttl\n",
        "          self.age()   >   self.ttl\n",
    );
    std::fs::write(&file, text).expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "reindent"]);
    let head = crate::session::rev(dir, "HEAD");

    let blamed = blame_ok(dir, Rev::Commit(head), FILE, BlameOptions::default());
    assert_eq!(
        blamed.lines[39].commit,
        Some(ids.priya),
        "-w passes it back"
    );
    let opts = BlameOptions {
        ignore_whitespace: false,
        ..BlameOptions::default()
    };
    let blamed = blame_ok(dir, Rev::Commit(head), FILE, opts);
    assert_eq!(
        blamed.lines[39].commit,
        Some(head),
        "without -w it is the reindent's"
    );
}

#[test]
fn a_skipped_line_is_marked_skipped() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let blamed = blame_ok(&dir, Rev::Commit(ids.you), FILE, BlameOptions::default());
    let line = &blamed.lines[30];
    assert_eq!(line.text, b"    pub fn token(&self) -> &str {");
    assert_eq!(line.mark, LineMark::Skipped);
    assert_eq!(line.commit, Some(ids.ana), "git passed it to Ana");
    assert_eq!(blamed.lines[29].mark, LineMark::Plain, "line 30 is plain");
}

/// m7-power-tools/18e: an ignored commit that **added** a line leaves git
/// no one to pass it to. On every supported git the line reads unblamable
/// (2.38.5's porcelain has no `unblamable` line; blame reads the human
/// format's `*`).
#[test]
fn an_unblamable_line_is_marked_unblamable() {
    let temp = tempfile::tempdir().expect("a temp dir");
    let dir = temp.path();
    git_ok(dir, &["init", "-q", "-b", "main"]);
    std::fs::write(dir.join("a.rs"), "fn one() {}\nfn two() {}\n").expect("writes");
    git_ok(dir, &["add", "a.rs"]);
    git_ok(dir, &["commit", "-q", "-m", "add"]);
    std::fs::write(
        dir.join("a.rs"),
        "fn one() {}\nfn added_by_the_ignored_commit() {}\nfn two() {}\n",
    )
    .expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "reformat"]);
    let ignored = rev(dir, "HEAD");
    std::fs::write(dir.join(".git-blame-ignore-revs"), format!("{ignored}\n")).expect("writes");
    git_ok(dir, &["add", ".git-blame-ignore-revs"]);
    git_ok(dir, &["commit", "-q", "-m", "list it"]);

    let blamed = blame_ok(
        dir,
        Rev::Commit(rev(dir, "HEAD")),
        "a.rs",
        BlameOptions::default(),
    );
    let marks: Vec<LineMark> = blamed.lines.iter().map(|line| line.mark).collect();
    assert_eq!(
        marks,
        [LineMark::Plain, LineMark::Unblamable, LineMark::Plain]
    );
    assert_eq!(blamed.lines[1].commit, Some(ignored), "git keeps it there");
}

#[test]
fn previous_names_the_parent_and_old_path_across_the_rename() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let blamed = blame_ok(&dir, Rev::Commit(ids.you), FILE, BlameOptions::default());
    assert_eq!(blamed.lines[1].commit, Some(ids.priya), "line 2 is Priya's");
    let priya = &blamed.commits[&ids.priya];
    assert_eq!(priya.author, "Priya");
    assert_eq!(priya.summary, "Move session TTL to config");
    assert_eq!(
        priya.previous,
        Some((ids.rustfmt, OLD_FILE.into())),
        "the parent and the name before the rename"
    );
    let you = &blamed.commits[&ids.you];
    assert_eq!(you.previous, Some((ids.priya, FILE.into())));
    assert!(blamed.commits[&ids.ana].boundary);
}

#[test]
fn working_tree_blame_leaves_new_lines_uncommitted() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let ids = commits(dir);
    let file = dir.join(FILE);
    let text = std::fs::read_to_string(&file).expect("reads");
    let text = text.replace("        loop {\n", "        // try twice\n        loop {\n");
    std::fs::write(&file, text).expect("writes");

    let blamed = blame_ok(dir, Rev::WorkingTree, FILE, BlameOptions::default());
    assert_eq!(blamed.rev, Rev::WorkingTree);
    assert_eq!(blamed.lines.len(), 61);
    assert_eq!(blamed.lines[19].text, b"        // try twice");
    assert_eq!(blamed.lines[19].commit, None, "not committed yet");
    assert_eq!(
        blamed.lines[20].commit,
        Some(ids.tom),
        "`loop {{` is still Tom's"
    );
    assert_eq!(blamed.lines[20].final_line, 21);
    assert_eq!(blamed.lines[20].orig_line, 18, "line 18 in Tom's commit");
    assert!(
        blamed
            .commits
            .keys()
            .all(|id| *id != Oid::from_sha1([0; 20]))
    );
}

#[test]
fn binary_is_refused_before_spawning() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    for rev in [Rev::Commit(ids.you), Rev::WorkingTree] {
        let refused = blame(
            &open(&dir),
            &rev,
            Path::new("assets/pic.bin"),
            &BlameOptions::default(),
            &(),
        );
        assert!(
            matches!(refused, Err(BlameError::Binary)),
            "{rev:?}: {refused:?}"
        );
    }
}

#[test]
fn untracked_is_refused() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    assert!(
        dir.join("src/new.rs").is_file(),
        "the fixture has the untracked file"
    );
    for (rev, path) in [
        (Rev::WorkingTree, "src/new.rs"),
        (Rev::Commit(ids.you), "src/new.rs"),
        // The old name is not in the newest commit.
        (Rev::Commit(ids.you), OLD_FILE),
    ] {
        let refused = blame(
            &open(&dir),
            &rev,
            Path::new(path),
            &BlameOptions::default(),
            &(),
        );
        assert!(
            matches!(refused, Err(BlameError::Untracked)),
            "{path} at {rev:?}: {refused:?}"
        );
        assert_eq!(
            refused.map(|_| ()).map_err(|why| why.to_string()),
            Err("the file isn't committed yet".to_owned())
        );
    }
}

#[test]
fn blame_honours_stop() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let stop = AtomicBool::new(true);
    let result = blame(
        &open(&dir),
        &Rev::WorkingTree,
        Path::new(FILE),
        &BlameOptions::default(),
        &stop,
    );
    assert!(matches!(result, Err(BlameError::Stopped)), "{result:?}");
    // git itself still answers once the stop is lifted: nothing was left
    // behind (a lock, say) by the killed child.
    assert!(git(&dir, &["status", "--porcelain"]).status.success());
}

/// The oracle for paths: each line's `filename` from `git blame
/// --line-porcelain`, which repeats every header on every line.
fn oracle_paths(dir: &Path, rev: Oid, path: &str) -> Vec<String> {
    let rev = rev.to_string();
    let out = git_ok(
        dir,
        &[
            "blame",
            "--line-porcelain",
            "-w",
            "-M",
            "--ignore-revs-file",
            ".git-blame-ignore-revs",
            &rev,
            "--",
            path,
        ],
    );
    out.split(|&b| b == b'\n')
        .filter_map(|line| line.strip_prefix(b"filename "))
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect()
}

/// The session fixture renames `OLD_FILE` → `FILE` (Priya's commit), so
/// blaming `FILE` names both paths.
#[test]
fn blame_commit_path_follows_a_rename() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let ids = commits(&dir);
    let blamed = blame_ok(&dir, Rev::Commit(ids.you), FILE, BlameOptions::default());
    let ours: Vec<String> = blamed
        .lines
        .iter()
        .map(|line| {
            let id = line.commit.expect("every line at a commit is committed");
            blamed.commits[&id].path.to_string_lossy().into_owned()
        })
        .collect();
    assert_eq!(ours, oracle_paths(&dir, ids.you, FILE));
    assert!(
        ours.iter().any(|p| p == OLD_FILE),
        "a line from before the rename"
    );
    assert!(
        ours.iter().any(|p| p == FILE),
        "a line from after the rename"
    );
    assert_eq!(blamed.commits[&ids.priya].path, Path::new(FILE));
    assert_eq!(blamed.commits[&ids.ana].path, Path::new(OLD_FILE));
}

/// Whether plain `git blame` (no `-M`) names `id` on some line: the premise
/// that makes the `-M` test below able to fail.
fn plain_blame_names(dir: &Path, path: &str, id: Oid) -> bool {
    let out = git_ok(dir, &["blame", "--line-porcelain", "HEAD", "--", path]);
    let hex = id.to_string();
    out.split(|&b| b == b'\n')
        .any(|line| line.starts_with(hex.as_bytes()))
}

/// Review 18/19/27 Minor 7 (mutation M15): a block moved within the file
/// stays with the commit that wrote it, because blame runs with `-M`.
#[test]
fn a_block_moved_within_the_file_keeps_its_author() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();
    let path = "src/moved.rs";
    let first = "fn first_block_that_moves() {\n    let alpha_value = compute_alpha(1, 2, 3);\n    let beta_value = compute_beta(alpha_value);\n    store_result(alpha_value, beta_value);\n}\n";
    let second = "fn second_block_that_stays() {\n    let gamma_value = compute_gamma(4, 5, 6);\n    let delta_value = compute_delta(gamma_value);\n    store_other(gamma_value, delta_value);\n}\n";
    std::fs::write(dir.join(path), format!("{first}{second}")).expect("writes");
    git_ok(dir, &["add", path]);
    git_ok(dir, &["commit", "-q", "-m", "write both blocks"]);
    let wrote = rev(dir, "HEAD");
    std::fs::write(dir.join(path), format!("{second}{first}")).expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "swap the blocks"]);
    let moved = rev(dir, "HEAD");
    assert!(
        plain_blame_names(dir, path, moved),
        "the premise: without -M, some moved line is the swap's"
    );

    let blamed = blame_ok(dir, Rev::Commit(moved), path, BlameOptions::default());
    assert_eq!(blamed.lines.len(), 10);
    for line in &blamed.lines {
        assert_eq!(
            line.commit,
            Some(wrote),
            "{:?}",
            String::from_utf8_lossy(&line.text)
        );
    }
}

/// Review 18/19/27 Minor 8 (mutation M23): for the working tree, the file on
/// disk decides "binary", not the committed one — both ways round.
#[test]
fn working_tree_binary_is_the_file_on_disk() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Session);
    let dir = scratch.path();

    // Text in HEAD, binary on disk: refused.
    std::fs::write(dir.join(FILE), b"fn a() {}\n\0\x01\x02 binary now\n").expect("writes");
    let refused = blame(
        &open(dir),
        &Rev::WorkingTree,
        Path::new(FILE),
        &BlameOptions::default(),
        &(),
    );
    assert!(
        matches!(refused, Err(BlameError::Binary)),
        "binary on disk: {refused:?}"
    );

    // Binary in HEAD, text on disk: blamed, every line not committed yet.
    let pic = "assets/pic.bin";
    std::fs::write(dir.join(pic), b"now plain text\nsecond line\n").expect("writes");
    let blamed = blame_ok(dir, Rev::WorkingTree, pic, BlameOptions::default());
    assert_eq!(blamed.lines.len(), 2);
    assert!(blamed.lines.iter().all(|line| line.commit.is_none()));
}

/// The final line numbers of every line credited to `id`.
fn lines_of(blamed: &Blame, id: Oid) -> Vec<u32> {
    blamed
        .lines
        .iter()
        .filter(|line| line.commit == Some(id))
        .map(|line| line.final_line)
        .collect()
}

/// m7-power-tools/20d: in a `--depth 2` clone git credits the lines it can
/// go no further back on to the boundary; `blame` marks that commit
/// `shallow` from git's own `<common_dir>/shallow` list.
#[test]
fn blame_marks_lines_from_the_shallow_boundary() {
    let temp = crate::session::shallow_clone(2);
    let dir = temp.path().join("clone");
    let boundary = rev(&dir, "HEAD~1");
    let listed = std::fs::read_to_string(dir.join(".git").join("shallow")).expect("shallow list");
    assert_eq!(listed.trim(), boundary.to_string(), "the shallow file's id");
    let blamed = blame_ok(
        &dir,
        Rev::Commit(rev(&dir, "HEAD")),
        FILE,
        BlameOptions::default(),
    );
    // git's own blame is the oracle for which lines land on the boundary.
    let expected: Vec<u32> = oracle(&dir, Some(rev(&dir, "HEAD")), FILE)
        .iter()
        .zip(1..)
        .filter(|((id, _), _)| *id == Some(boundary))
        .map(|(_, line)| line)
        .collect();
    let credited = lines_of(&blamed, boundary);
    assert_eq!(credited, expected);
    for line in (17..=25).chain(27..=30) {
        assert!(credited.contains(&line), "the probe's line {line}");
    }
    for commit in blamed.commits.values() {
        assert_eq!(commit.shallow, commit.id == boundary, "{commit:?}");
    }
}

/// A full clone's root is parentless like a shallow cut, but not shallow:
/// its author did write the lines.
#[test]
fn a_full_clones_root_commit_is_not_shallow() {
    let dir = graft_fixtures::borrow(Shape::Session);
    let root = rev(&dir, "main~5");
    let blamed = blame_ok(
        &dir,
        Rev::Commit(rev(&dir, "main")),
        FILE,
        BlameOptions::default(),
    );
    let commit = blamed.commits.get(&root).expect("the root names lines");
    assert!(!commit.shallow);
    assert!(blamed.commits.values().all(|commit| !commit.shallow));
    // Recorded in 20d's NOTES: what git's `boundary` says on a real root.
    eprintln!("root boundary = {}", commit.boundary);
}

#[test]
fn an_unreadable_shallow_file_fails_blame() {
    let temp = crate::session::shallow_clone(2);
    let dir = temp.path().join("clone");
    let shallow = dir.join(".git").join("shallow");
    std::fs::remove_file(&shallow).expect("removing shallow");
    std::fs::create_dir(&shallow).expect("shallow as a directory");
    let result = blame(
        &open(&dir),
        &Rev::Commit(rev(&dir, "HEAD")),
        Path::new(FILE),
        &BlameOptions::default(),
        &(),
    );
    match result {
        Err(BlameError::Shallow { path, .. }) => assert_eq!(path, shallow),
        other => panic!("expected Err(Shallow), got {other:?}"),
    }
}
