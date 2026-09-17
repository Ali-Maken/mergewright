//! Fixture round-trip for `diff::parse_unified`/`parse_file_list`: real git
//! output, for every commit of every named shape in the corpus
//! (02-diff-parser's fixture-round-trip acceptance criterion).

use std::collections::HashMap;
use std::path::Path;

use graft_fixtures::Shape;
use graft_fixtures::git::run;
use graft_git::diff::{parse_file_list, parse_unified};

/// The empty tree's well-known object id, valid in every repository: diffing
/// it against a commit lists that commit's whole tree as additions, with no
/// need for `--root` (which only applies to an actual root commit).
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[test]
fn parse_unified_matches_numstat_for_every_shape() {
    for &shape in Shape::ALL {
        let Ok(dir) = graft_fixtures::try_borrow(shape) else {
            continue; // e.g. non-utf8-ref-name, skipped where the filesystem can't hold it
        };
        let log = run(&dir, &["log", "--format=%H", "--all"])
            .unwrap_or_else(|why| panic!("{shape}: git log: {why}"));
        for sha in String::from_utf8_lossy(&log).lines() {
            check_commit(shape, &dir, sha);
        }
    }
}

fn check_commit(shape: Shape, dir: &Path, sha: &str) {
    let patch = run(
        dir,
        &[
            "diff-tree",
            "-p",
            "-M",
            "-C",
            "--no-color",
            "--no-ext-diff",
            "--root",
            sha,
        ],
    )
    .unwrap_or_else(|why| panic!("{shape} {sha}: diff-tree -p: {why}"));
    let diffs =
        parse_unified(&patch).unwrap_or_else(|why| panic!("{shape} {sha}: parse_unified: {why:?}"));

    let list = file_list(dir, &["--root", sha])
        .unwrap_or_else(|why| panic!("{shape} {sha}: parse_file_list: {why:?}"));
    let by_path: HashMap<&[u8], (Option<u32>, Option<u32>)> = list
        .files
        .iter()
        .map(|f| (f.path.as_slice(), (f.added, f.removed)))
        .collect();

    for diff in &diffs {
        if diff.binary.is_some() {
            continue;
        }
        let name = || String::from_utf8_lossy(&diff.entry.path).into_owned();
        let (added, removed) = by_path
            .get(diff.entry.path.as_slice())
            .unwrap_or_else(|| panic!("{shape} {sha}: {:?} not in --numstat output", name()));
        assert_eq!(
            diff.entry.added,
            *added,
            "{shape} {sha}: added count for {:?}",
            name()
        );
        assert_eq!(
            diff.entry.removed,
            *removed,
            "{shape} {sha}: removed count for {:?}",
            name()
        );
    }
}

/// Runs `diff-tree --raw -z` and `--numstat -z` with the same trailing
/// arguments and joins them with [`parse_file_list`].
fn file_list(
    dir: &Path,
    args: &[&str],
) -> Result<graft_git::diff::FileList, graft_git::diff::ParseError> {
    let base = ["diff-tree", "-z", "-M", "-C", "-r", "--no-commit-id"];
    let raw_args: Vec<&str> = base
        .iter()
        .chain(["--raw"].iter())
        .chain(args)
        .copied()
        .collect();
    let numstat_args: Vec<&str> = base
        .iter()
        .chain(["--numstat"].iter())
        .chain(args)
        .copied()
        .collect();
    let raw = run(dir, &raw_args).unwrap_or_else(|why| panic!("diff-tree --raw: {why}"));
    let numstat =
        run(dir, &numstat_args).unwrap_or_else(|why| panic!("diff-tree --numstat: {why}"));
    parse_file_list(&raw, &numstat)
}

#[test]
fn parse_file_list_paths_match_ls_tree_for_unicode_and_non_utf8_names() {
    for shape in [Shape::UnicodeNames, Shape::NonUtf8Path] {
        let dir = graft_fixtures::borrow(shape);
        let head_ref = if shape == Shape::NonUtf8Path {
            "latin1"
        } else {
            "HEAD"
        };
        let listed = run(&dir, &["ls-tree", "-r", "-z", "--name-only", head_ref])
            .unwrap_or_else(|why| panic!("{shape}: ls-tree: {why}"));
        let mut expected: Vec<Vec<u8>> = listed
            .split(|&b| b == 0)
            .filter(|p| !p.is_empty())
            .map(<[u8]>::to_vec)
            .collect();
        expected.sort();

        let sha = String::from_utf8_lossy(
            &run(&dir, &["rev-parse", head_ref])
                .unwrap_or_else(|why| panic!("{shape}: rev-parse: {why}")),
        )
        .trim()
        .to_string();
        let list = file_list(&dir, &[EMPTY_TREE, &sha])
            .unwrap_or_else(|why| panic!("{shape}: parse_file_list: {why:?}"));
        let mut found: Vec<Vec<u8>> = list.files.iter().map(|f| f.path.clone()).collect();
        found.sort();
        assert_eq!(
            found, expected,
            "{shape}: paths from parse_file_list vs ls-tree"
        );
    }
}
