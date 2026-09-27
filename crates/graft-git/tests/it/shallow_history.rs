//! A shallow clone's boundary commits are parentless, in every walk
//! (m7-power-tools/32). Git's rule: a commit listed in `<common_dir>/shallow`
//! has no parents. Every answer here is checked against `git rev-list` on the
//! same clone.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::commit_graph::{self, CommitGraphStatus};
use graft_git::history::{
    CommitTable, Order, Walk, WalkError, ahead_behind, first_parent_chain, first_parent_merges,
    read_commit,
};
use graft_git::repo::RepoHandle;

use crate::session::{git_ok, open, rev};

/// One commit and its parents, first parent first.
type Edge = (Oid, Vec<Oid>);

/// Which refs a clone fetches.
#[derive(Clone, Copy)]
enum Branches {
    /// `main` only (`--depth` implies `--single-branch`).
    Main,
    /// Every branch, each cut at the depth from its own tip.
    All,
}

/// A `--depth <depth>` clone of `shape` at `<temp>/clone`. `file://` so git
/// honours `--depth`.
fn clone(shape: Shape, depth: u32, branches: Branches) -> tempfile::TempDir {
    let source = graft_fixtures::borrow(shape);
    let temp = tempfile::tempdir().unwrap_or_else(|why| panic!("a temp dir: {why}"));
    let url = format!("file://{}", source.display());
    let depth = depth.to_string();
    let which = match branches {
        Branches::Main => ["--branch", "main"],
        Branches::All => ["--no-single-branch", "--no-tags"],
    };
    let mut args = vec!["clone", "-q", "--depth", &depth];
    args.extend(which);
    args.extend([url.as_str(), "clone"]);
    git_ok(temp.path(), &args);
    temp
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|why| panic!("git printed non-UTF-8: {why}"))
}

fn hex(word: &str) -> Oid {
    Oid::from_hex(word).unwrap_or_else(|why| panic!("{word:?}: {why}"))
}

/// The ids the clone's `shallow` file lists.
fn shallow_ids(dir: &Path) -> BTreeSet<Oid> {
    let path = dir.join(".git/shallow");
    let bytes = std::fs::read(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    text(bytes).lines().map(hex).collect()
}

/// The oracle: `git rev-list --parents --all`.
fn git_edges(dir: &Path) -> BTreeSet<Edge> {
    text(git_ok(dir, &["rev-list", "--parents", "--all"]))
        .lines()
        .map(|line| {
            let mut words = line.split_whitespace().map(hex);
            let id = words
                .next()
                .unwrap_or_else(|| panic!("an empty rev-list line"));
            (id, words.collect())
        })
        .collect()
}

/// The ids `git rev-list` prints for `args`, in its order.
fn git_ids(dir: &Path, args: &[&str]) -> Vec<Oid> {
    text(git_ok(dir, args)).lines().map(hex).collect()
}

fn table(repo: &RepoHandle) -> Arc<CommitTable> {
    Arc::new(CommitTable::open(repo).unwrap_or_else(|why| panic!("opening the table: {why}")))
}

/// Every `(id, parents)` `next_links` hands out, walk to the end.
fn link_edges(mut walk: Walk<'_>) -> BTreeSet<Edge> {
    let table = Arc::clone(walk.table());
    let mut edges = BTreeSet::new();
    loop {
        let chunk = walk
            .next_links(64, &|| false)
            .unwrap_or_else(|why| panic!("next_links: {why}"));
        for link in chunk.links {
            let parents = link.parents.iter().map(|p| table.id_of(p)).collect();
            edges.insert((table.id_of(link.commit), parents));
        }
        if chunk.done {
            return edges;
        }
    }
}

/// Every `(id, parents)` of `next_chunk`'s rows, walk to the end.
fn row_edges(mut walk: Walk<'_>) -> BTreeSet<Edge> {
    let mut edges = BTreeSet::new();
    loop {
        let chunk = walk
            .next_chunk(64, &|| false)
            .unwrap_or_else(|why| panic!("next_chunk: {why}"));
        for commit in chunk.commits {
            edges.insert((commit.id, commit.parents));
        }
        if chunk.done {
            return edges;
        }
    }
}

fn all_refs(repo: &RepoHandle, order: Order) -> Walk<'_> {
    Walk::all_refs(repo, order, table(repo)).unwrap_or_else(|why| panic!("all_refs: {why}"))
}

/// The walk oracle: links and rows, in every ordering, equal git's edges;
/// at least one boundary row (a parentless commit git lists as shallow) so
/// the check is not vacuous.
fn assert_walks_match_git(dir: &Path) {
    let repo = open(dir);
    let expected = git_edges(dir);
    assert!(
        shallow_ids(dir)
            .iter()
            .any(|id| expected.contains(&(*id, Vec::new()))),
        "no boundary row in git's own answer: the oracle would be vacuous"
    );

    // First-parent walks a different set; `first_parent_chain` is checked
    // on its own below.
    for order in [Order::CommitDate, Order::Topological] {
        assert_eq!(
            link_edges(all_refs(&repo, order)),
            expected,
            "next_links, {order:?}"
        );
        assert_eq!(
            row_edges(all_refs(&repo, order)),
            expected,
            "next_chunk, {order:?}"
        );
    }
    let tips = repo
        .commit_tips(&*table(&repo))
        .unwrap_or_else(|why| panic!("tips: {why}"));
    let full = Walk::full_topological(&repo, &tips, table(&repo))
        .unwrap_or_else(|why| panic!("full_topological: {why}"));
    assert_eq!(link_edges(full), expected, "full_topological links");
}

#[test]
fn a_depth_2_clone_walks() {
    let temp = crate::session::shallow_clone(2);
    let dir = temp.path().join("clone");
    let repo = open(&dir);
    let walk = Walk::all_refs(&repo, Order::CommitDate, table(&repo));
    let mut walk = walk.unwrap_or_else(|why| panic!("all_refs on a depth-2 clone: {why}"));
    let chunk = walk.next_chunk(64, &|| false);
    assert!(chunk.is_ok(), "walking a depth-2 clone: {:?}", chunk.err());
}

#[test]
fn walks_match_git_on_a_depth_1_session_clone() {
    let temp = crate::session::shallow_clone(1);
    assert_walks_match_git(&temp.path().join("clone"));
}

#[test]
fn walks_match_git_on_a_depth_2_session_clone() {
    let temp = crate::session::shallow_clone(2);
    assert_walks_match_git(&temp.path().join("clone"));
}

/// `feature-branches` at depth 3, `main` only: `main~1` is `Merge branch
/// 'feature/one'`, whose second parent `one: finish` is a boundary (its
/// parent `one: start` was never fetched). The assertion below proves the
/// cut is there, not assumed.
#[test]
fn walks_match_git_on_a_depth_3_feature_branches_clone_with_a_cut_merge() {
    let temp = clone(Shape::FeatureBranches, 3, Branches::Main);
    let dir = temp.path().join("clone");
    let edges = git_edges(&dir);
    let shallow = shallow_ids(&dir);
    let cut_merge = edges.iter().any(|(_, parents)| {
        parents
            .get(1)
            .is_some_and(|second| shallow.contains(second))
    });
    assert!(cut_merge, "no merge with a cut second parent: {edges:#?}");
    assert_walks_match_git(&dir);
}

#[test]
fn first_parent_reads_match_git_on_a_shallow_clone() {
    let temp = clone(Shape::FeatureBranches, 3, Branches::Main);
    let dir = temp.path().join("clone");
    let repo = open(&dir);
    let main = rev(&dir, "origin/main");

    let table = table(&repo);
    let chain: Vec<Oid> = first_parent_chain(&repo, &table, main, &())
        .unwrap_or_else(|why| panic!("first_parent_chain: {why}"))
        .into_iter()
        .map(|handle| table.id_of(handle))
        .collect();
    assert_eq!(
        chain,
        git_ids(&dir, &["rev-list", "--first-parent", "origin/main"])
    );

    let merges =
        first_parent_merges(&repo, main).unwrap_or_else(|why| panic!("first_parent_merges: {why}"));
    let count = text(git_ok(
        &dir,
        &[
            "rev-list",
            "--first-parent",
            "--merges",
            "--count",
            "origin/main",
        ],
    ));
    assert_eq!(merges.to_string(), count.trim());
    // `branches::merges_on` counted 0 here before 32 (its `Missing` arm).
    assert!(
        merges > 0,
        "the chain holds merges; a count of 0 proves nothing"
    );
}

#[test]
fn ahead_behind_matches_git_across_a_shallow_boundary() {
    let temp = clone(Shape::FeatureBranches, 2, Branches::All);
    let dir = temp.path().join("clone");
    let repo = open(&dir);
    let branches = text(git_ok(
        &dir,
        &["for-each-ref", "--format=%(refname)", "refs/remotes/origin"],
    ));
    let mut checked = 0;
    for branch in branches.lines().filter(|name| !name.ends_with("/HEAD")) {
        let (a, b) = (rev(&dir, "origin/main"), rev(&dir, branch));
        let counted = ahead_behind(&repo, a, b).unwrap_or_else(|why| panic!("{branch}: {why}"));
        let range = format!("origin/main...{branch}");
        let git = text(git_ok(
            &dir,
            &["rev-list", "--left-right", "--count", &range],
        ));
        assert_eq!(
            format!("{}\t{}", counted.ahead, counted.behind),
            git.trim(),
            "{range}"
        );
        checked += 1;
    }
    assert!(checked > 1, "only {checked} branches compared");
}

#[test]
fn read_commit_of_a_boundary_has_no_parents() {
    let temp = crate::session::shallow_clone(2);
    let dir = temp.path().join("clone");
    let repo = open(&dir);
    let boundary = rev(&dir, "main~1");
    let commit = read_commit(&repo, boundary).unwrap_or_else(|why| panic!("read_commit: {why}"));
    assert_eq!(commit.parents, Vec::<Oid>::new());
    let tip = read_commit(&repo, rev(&dir, "main")).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(
        tip.parents,
        vec![boundary],
        "a commit above the cut keeps its parent"
    );
}

/// Git's own test technique: a full clone with a commit-graph, then a
/// hand-written `shallow`. Git stops using the graph (`commit-graph.c`
/// `commit_graph_compatible`); so must the table.
#[test]
fn a_shallow_file_over_a_commit_graph_binds_no_graph() {
    let temp = clone(Shape::FeatureBranches, 1000, Branches::All);
    let dir = temp.path().join("clone");
    git_ok(&dir, &["commit-graph", "write", "--reachable"]);
    let repo = open(&dir);
    assert!(
        matches!(
            commit_graph::status(&repo),
            Ok(CommitGraphStatus::Present { .. })
        ),
        "the graph is there before the shallow file"
    );

    // A table opened before the clone became shallow keeps its graph (the
    // app holds one across `git fetch --depth`); its walk must still cut.
    let stale = table(&repo);
    assert!(stale.has_commit_graph(), "the stale table has a graph");

    let cut = text(git_ok(&dir, &["rev-parse", "origin/main~2"]));
    std::fs::write(dir.join(".git/shallow"), cut).unwrap_or_else(|why| panic!("{why}"));
    let repo = open(&dir);

    let through_the_graph = Walk::all_refs(&repo, Order::Topological, stale)
        .unwrap_or_else(|why| panic!("all_refs over the stale table: {why}"));
    assert_eq!(
        link_edges(through_the_graph),
        git_edges(&dir),
        "links read from the graph file"
    );

    assert!(!table(&repo).has_commit_graph(), "the table bound a graph");
    assert!(
        matches!(commit_graph::status(&repo), Ok(CommitGraphStatus::Absent)),
        "status: {:?}",
        commit_graph::status(&repo)
    );
    assert_walks_match_git(&dir);
}

#[test]
fn an_unreadable_shallow_file_fails_the_walk() {
    let temp = crate::session::shallow_clone(2);
    let dir = temp.path().join("clone");
    let shallow = dir.join(".git/shallow");
    std::fs::remove_file(&shallow).unwrap_or_else(|why| panic!("{why}"));
    std::fs::create_dir(&shallow).unwrap_or_else(|why| panic!("{why}"));
    let repo = open(&dir);

    let walk = Walk::all_refs(
        &repo,
        Order::CommitDate,
        Arc::new(CommitTable::without_a_commit_graph(Oid::SHA1_BYTES)),
    );
    assert!(
        matches!(walk, Err(WalkError::Shallow(_))),
        "all_refs: {:?}",
        walk.err()
    );
    let main = rev(&dir, "main");
    assert!(matches!(
        first_parent_merges(&repo, main),
        Err(WalkError::Shallow(_))
    ));
    assert!(matches!(
        read_commit(&repo, main),
        Err(WalkError::Shallow(_))
    ));
}
