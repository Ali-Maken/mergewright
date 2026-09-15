//! Tests for the three walks.
//!
//! Two kinds, because they answer different questions. The fixture tests ask
//! "does this agree with git on a repository someone can describe?", with
//! `git rev-list` as the oracle — it is the only authority on what an order
//! *means*. The in-memory tests ask "is this the incremental algorithm or
//! just a date sort?", which the corpus cannot answer: every fixture is
//! written by `fast-import` with strictly increasing timestamps, so date
//! order is a valid topological order on all of them and the two walks agree
//! by construction. Only a graph with dates out of order tells them apart,
//! and no corpus shape has one (KNOWN_GAPS.md).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_fixtures::Shape;

use super::node::{MapNodes, Nodes};
use super::simple::{Dated, Follow};
use super::topological::Topological;
use super::{CHUNK, Chunk, CommitTable, Order, Walk, WalkError, read_commit};
use crate::repo::SharedRepo;

/// The repository's id table, as the app shares it.
fn table(handle: &crate::repo::RepoHandle) -> Arc<CommitTable> {
    Arc::new(CommitTable::open(handle).expect("the table opens"))
}

/// Nothing asks these walks to stop.
fn never() -> bool {
    false
}

/// Every commit the walk produces, in order, as ids.
fn all(walk: &mut Walk<'_>) -> Vec<Oid> {
    let mut ids = Vec::new();
    loop {
        let chunk = walk
            .next_chunk(CHUNK, &never)
            .expect("the fixture's commits can be read");
        ids.extend(chunk.commits.iter().map(|commit| commit.id));
        if chunk.done {
            return ids;
        }
    }
}

/// What `git rev-list <args>` says, as ids.
fn rev_list(repo: &Path, args: &[&str]) -> Vec<Oid> {
    let mut command = vec!["rev-list"];
    command.extend_from_slice(args);
    let stdout = graft_fixtures::git::run(repo, &command).expect("git rev-list runs on a fixture");
    String::from_utf8(stdout)
        .expect("git prints object ids as ASCII")
        .lines()
        .map(|line| Oid::from_hex(line).expect("git prints full object ids"))
        .collect()
}

/// A corpus fixture with a commit-graph file written into a scratch copy.
///
/// No shape in the corpus ships one: `fast-import` does not write it and the
/// corpus is byte-identical by digest, so adding a file to it would change
/// every fixture's identity. A writable copy is the honest way to test the
/// path that needs one.
fn with_commit_graph(shape: Shape) -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(shape);
    graft_fixtures::git::run(scratch.path(), &["commit-graph", "write", "--reachable"])
        .expect("git writes a commit-graph file");
    scratch
}

#[test]
fn commit_date_order_agrees_with_git_on_the_criss_cross_fixture() {
    let path = graft_fixtures::borrow(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    assert_eq!(all(&mut walk), rev_list(&path, &["--date-order", "--all"]));
}

#[test]
fn commit_date_order_agrees_with_git_on_the_octopus_fixture() {
    let path = graft_fixtures::borrow(Shape::OctopusMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    assert_eq!(all(&mut walk), rev_list(&path, &["--date-order", "--all"]));
}

#[test]
fn the_linear_fixture_comes_out_newest_first() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    let ids = all(&mut walk);
    assert_eq!(ids, rev_list(&path, &["--all"]));
    assert_eq!(ids.len(), 200, "the linear shape is 200 commits");
}

#[test]
fn topological_order_agrees_with_git_on_the_criss_cross_fixture() {
    // Written with a commit-graph file so the topological path is the one
    // actually under test; without one ADR-0008 serves commit date instead.
    let scratch = with_commit_graph(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");

    assert_eq!(walk.order_in_force(), Order::Topological);
    // Every fixture's timestamps strictly increase with its stream order, so
    // there is exactly one date order and it is topological: git's
    // `--date-order` is a sound oracle for this walk on this corpus.
    assert_eq!(
        all(&mut walk),
        rev_list(scratch.path(), &["--date-order", "--all"])
    );
}

/// Makes a commit with `git commit-tree` on `parents` and returns its id.
fn commit_tree(dir: &Path, parents: &[&str], message: &str) -> String {
    let mut args = vec!["commit-tree", "HEAD^{tree}"];
    for parent in parents {
        args.extend_from_slice(&["-p", parent]);
    }
    args.extend_from_slice(&["-m", message]);
    let stdout = graft_fixtures::git::run(dir, &args).expect("git makes a commit");
    String::from_utf8(stdout)
        .expect("git prints ids as ASCII")
        .trim()
        .to_owned()
}

/// Two commits the commit-graph does not know about, on a new branch: a
/// child of `HEAD`, then a merge of that child with the root commit, which
/// the graph does hold. Dated now, after every fixture commit, so git's date order is
/// still the one topological order.
fn commits_after_the_graph(dir: &Path) {
    let roots = graft_fixtures::git::run(dir, &["rev-list", "--max-parents=0", "HEAD"])
        .expect("git lists the root commits");
    let roots = String::from_utf8(roots).expect("ASCII");
    let root = roots.lines().next().expect("a history has a root");
    let child = commit_tree(dir, &["HEAD"], "after the graph");
    let merge = commit_tree(dir, &[&child, root], "merge after the graph");
    graft_fixtures::git::run(dir, &["update-ref", "refs/heads/late", &merge])
        .expect("git moves a ref");
}

#[test]
fn commits_made_after_the_commit_graph_walk_in_gits_order() {
    // The overflow path: the graph covers the fixture, not the two commits
    // made since. Those are read from their objects with an unknown
    // generation, so they are explored before anything below them is
    // confirmed.
    let scratch = with_commit_graph(Shape::CrissCrossMerge);
    commits_after_the_graph(scratch.path());
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let table = table(&handle);
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, Arc::clone(&table)).expect("the walk starts");
    assert_eq!(walk.order_in_force(), Order::Topological);

    assert_eq!(
        all(&mut walk),
        rev_list(scratch.path(), &["--date-order", "--all"])
    );
    assert_eq!(table.overflow_len(), 2, "only the two late commits");
}

#[test]
fn a_two_layer_commit_graph_chain_walks_in_gits_order() {
    let scratch = graft_fixtures::copy_into_temp(Shape::CrissCrossMerge);
    let dir = scratch.path();
    graft_fixtures::git::run(dir, &["commit-graph", "write", "--reachable", "--split"])
        .expect("git writes the base layer");
    commits_after_the_graph(dir);
    graft_fixtures::git::run(
        dir,
        &["commit-graph", "write", "--reachable", "--split=no-merge"],
    )
    .expect("git writes a second layer");
    let chain = graft_fixtures::git::run(
        dir,
        &[
            "rev-parse",
            "--git-path",
            "objects/info/commit-graphs/commit-graph-chain",
        ],
    )
    .expect("git names the chain file");
    let chain = dir.join(String::from_utf8(chain).expect("ASCII").trim());
    let layers = std::fs::read_to_string(&chain).expect("the chain file is there");
    assert_eq!(layers.lines().count(), 2, "two layers, not one merged file");

    let shared = SharedRepo::open(dir).expect("the fixture opens");
    let handle = shared.handle();
    let table = table(&handle);
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, Arc::clone(&table)).expect("the walk starts");
    assert_eq!(walk.order_in_force(), Order::Topological);
    assert_eq!(all(&mut walk), rev_list(dir, &["--date-order", "--all"]));
    assert_eq!(table.overflow_len(), 0, "both layers cover every commit");
}

#[test]
fn topological_order_agrees_with_git_on_the_octopus_fixture() {
    let scratch = with_commit_graph(Shape::OctopusMerge);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");

    assert_eq!(
        all(&mut walk),
        rev_list(scratch.path(), &["--date-order", "--all"])
    );
}

#[test]
fn every_commit_of_the_octopus_fixture_comes_before_its_parents() {
    let scratch = with_commit_graph(Shape::OctopusMerge);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");

    let mut seen: Vec<Oid> = Vec::new();
    loop {
        let chunk = walk.next_chunk(CHUNK, &never).expect("commits can be read");
        for commit in &chunk.commits {
            for parent in &commit.parents {
                assert!(
                    !seen.contains(parent),
                    "{} was emitted before its child {}",
                    parent,
                    commit.id,
                );
            }
            seen.push(commit.id);
        }
        if chunk.done {
            break;
        }
    }
    assert_eq!(seen.len(), rev_list(scratch.path(), &["--all"]).len());
}

#[test]
fn the_first_parent_view_of_the_foxtrot_fixture_is_the_default_branchs_chain() {
    // The shape exists for exactly this: the merge's *first* parent is the
    // feature branch, so first-parent history walks off the default branch
    // and a walk that quietly followed every parent would look right on any
    // other fixture.
    let path = graft_fixtures::borrow(Shape::FoxtrotMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();

    let expected = rev_list(&path, &["--first-parent", "refs/heads/main"]);
    let tip = *expected.first().expect("main has commits");
    let mut walk = Walk::from_tips(&handle, Order::FirstParent, &[tip], table(&handle))
        .expect("the walk starts");

    assert_eq!(all(&mut walk), expected);
}

#[test]
fn first_parent_leaves_out_what_a_full_walk_includes() {
    let path = graft_fixtures::borrow(Shape::FoxtrotMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();

    let tip = *rev_list(&path, &["--first-parent", "refs/heads/main"])
        .first()
        .expect("main has commits");
    let mut trunk = Walk::from_tips(&handle, Order::FirstParent, &[tip], table(&handle))
        .expect("the walk starts");
    let mut whole = Walk::from_tips(&handle, Order::CommitDate, &[tip], table(&handle))
        .expect("the walk starts");

    assert!(
        all(&mut trunk).len() < all(&mut whole).len(),
        "a foxtrot merge has a second parent to leave out",
    );
}

#[test]
fn a_repository_with_no_commit_graph_file_falls_to_date_order_and_says_so() {
    // Read straight out of the corpus, which never has one.
    let path = graft_fixtures::borrow(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");

    assert!(!walk.has_commit_graph());
    assert_eq!(walk.requested_order(), Order::Topological);
    assert_eq!(walk.order_in_force(), Order::CommitDate);
    assert!(
        walk.is_provisional(),
        "the UI has to say the order is not final"
    );
    assert_eq!(all(&mut walk), rev_list(&path, &["--date-order", "--all"]));
}

#[test]
fn a_repository_with_a_commit_graph_file_runs_the_topological_path() {
    let scratch = with_commit_graph(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");

    assert!(walk.has_commit_graph());
    assert_eq!(walk.order_in_force(), Order::Topological);
    assert!(!walk.is_provisional());
}

#[test]
fn asking_for_date_order_is_never_provisional() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let walk = Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    assert!(!walk.has_commit_graph());
    assert!(!walk.is_provisional(), "date order is what was asked for");
}

#[test]
fn the_full_topological_walk_needs_no_commit_graph_file_and_is_never_provisional() {
    // ADR-0008's background job, on the corpus (which never has one).
    let path = graft_fixtures::borrow(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let tips = handle
        .commit_tips(&())
        .expect("the fixture's refs can be read");
    let mut walk = Walk::full_topological(&handle, &tips, table(&handle)).expect("the walk starts");

    assert!(!walk.has_commit_graph());
    assert_eq!(walk.requested_order(), Order::Topological);
    assert_eq!(walk.order_in_force(), Order::Topological);
    assert!(!walk.is_provisional());
    // Strictly increasing fixture timestamps make date order a topological
    // order too, so git's date order is the oracle here as it is above.
    assert_eq!(all(&mut walk), rev_list(&path, &["--date-order", "--all"]));
}

#[test]
fn the_full_topological_walk_reads_the_whole_range_before_it_emits() {
    // The price of ADR-0008's background job, stated: without generation
    // numbers the first commit out costs one read per commit in the range.
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let tips = handle
        .commit_tips(&())
        .expect("the fixture's refs can be read");
    let mut walk = Walk::full_topological(&handle, &tips, table(&handle)).expect("the walk starts");

    let first = walk.next_chunk(1, &never).expect("commits can be read");
    assert_eq!(first.commits.len(), 1);
    assert_eq!(walk.explorations(), 200);
    assert_eq!(all(&mut walk).len(), 199);
    assert_eq!(walk.explorations(), 200, "and no more than that");
}

#[test]
fn read_commit_returns_exactly_what_the_walk_emitted() {
    let path = graft_fixtures::borrow(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    let chunk = walk.next_chunk(CHUNK, &never).expect("commits can be read");
    assert!(chunk.done, "the fixture fits in one chunk");
    for emitted in &chunk.commits {
        let read = read_commit(&handle, emitted.id).expect("an emitted commit reads back");
        assert_eq!(&read, emitted);
    }
}

#[test]
fn reading_a_commit_the_repository_does_not_have_is_an_error() {
    let path = graft_fixtures::borrow(Shape::Trivial);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();

    let missing = read_commit(&handle, oid(0xab));
    assert!(
        matches!(missing, Err(WalkError::Commit(_))),
        "an absent object must be reported as one, got {missing:?}"
    );
}

#[test]
fn the_topological_walk_emits_before_it_has_walked_the_range() {
    // The 150 ms first paint in one assertion: the first row must not cost a
    // read per commit in the repository. 200 commits in, a handful out.
    let scratch = with_commit_graph(Shape::Linear);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");
    assert_eq!(walk.order_in_force(), Order::Topological);

    let first = walk.next_chunk(1, &never).expect("commits can be read");
    assert_eq!(first.commits.len(), 1);
    assert!(!first.done);
    assert!(
        walk.explorations() <= 4,
        "the first commit cost {} reads; a whole-range sort would cost 200",
        walk.explorations(),
    );

    // And the whole walk still costs one read per commit, not more.
    let rest = all(&mut walk);
    assert_eq!(rest.len(), 199);
    assert_eq!(walk.explorations(), 200);
}

#[test]
fn a_walk_carries_on_where_the_last_chunk_stopped() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();

    let mut whole =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");
    let expected = all(&mut whole);

    let mut chunked =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");
    let mut ids = Vec::new();
    let mut chunks = 0;
    loop {
        let chunk = chunked.next_chunk(30, &never).expect("commits can be read");
        assert!(chunk.commits.len() <= 30);
        chunks += 1;
        ids.extend(chunk.commits.iter().map(|commit| commit.id));
        if chunk.done {
            break;
        }
    }
    assert_eq!(ids, expected);
    assert!(chunks > 1, "200 commits do not fit in one 30-commit chunk");
}

#[test]
fn a_walk_that_is_told_to_stop_says_so_and_keeps_what_it_had() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    // Stops once three commits are out: what a cancelled job looks like from
    // in here.
    let taken = std::cell::Cell::new(0usize);
    let stop = || {
        let seen = taken.get();
        taken.set(seen + 1);
        seen >= 3
    };
    let chunk = walk.next_chunk(CHUNK, &stop).expect("commits can be read");

    assert_eq!(chunk.commits.len(), 3);
    assert!(
        chunk.stopped,
        "the caller has to be able to tell this apart"
    );
    assert!(!chunk.done, "a stopped walk is not a finished walk");
}

#[test]
fn a_finished_walk_stays_finished() {
    let path = graft_fixtures::borrow(Shape::Trivial);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    assert!(!all(&mut walk).is_empty());
    assert_eq!(
        walk.next_chunk(CHUNK, &never).expect("commits can be read"),
        Chunk {
            commits: Vec::new(),
            done: true,
            stopped: false,
        },
    );
}

#[test]
fn a_commits_fields_survive_the_walk() {
    let path = graft_fixtures::borrow(Shape::Trivial);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, table(&handle)).expect("the walk starts");

    let chunk = walk.next_chunk(1, &never).expect("commits can be read");
    let commit = chunk.commits.first().expect("the fixture has commits");
    assert_eq!(commit.author.email, b"fixture@example.invalid");
    assert_eq!(commit.committer.email, b"fixture@example.invalid");
    assert!(!commit.message.is_empty());
    assert_eq!(
        commit.id.to_string(),
        rev_list(&path, &["-1", "--all"])[0].to_string()
    );
}

/// A repository whose every commit shares one timestamp, with branches and
/// merges and no commit-graph file (m1-the-graph/67).
///
/// Plain `git` in this crate's fixtures pins both dates to `FIXED_DATE`, so
/// the tie is exact, as it is in history written by a script, a rebase or
/// `git am`. The corpus cannot hold this shape: `fast-import` streams there
/// carry strictly increasing times.
fn tied_dates() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path();
    let git = |args: &[&str]| {
        graft_fixtures::git::run(path, args).expect("git builds the tied-dates fixture");
    };
    let commit = |message: &str| git(&["commit", "-q", "--allow-empty", "-m", message]);
    let merge =
        |branch: &str, message: &str| git(&["merge", "-q", "--no-ff", "-m", message, branch]);

    graft_fixtures::git::init_repo(path).expect("git init");
    commit("initial");
    commit("main: set up");
    git(&["branch", "topic"]);
    git(&["branch", "side"]);
    for round in 1..=3 {
        commit(&format!("main: work before merge {round}"));
        git(&["checkout", "-q", "topic"]);
        commit(&format!("topic: change {round}a"));
        commit(&format!("topic: change {round}b"));
        git(&["checkout", "-q", "main"]);
        merge("topic", &format!("merge topic {round}"));
    }
    git(&["checkout", "-q", "side"]);
    commit("side: one");
    merge("main", "side: catch up with main");
    commit("side: two");
    git(&["checkout", "-q", "main"]);
    commit("main: after the merges");
    dir
}

/// Panics naming the first parent that comes out before one of its children.
fn assert_children_before_parents(what: &str, commits: &[graft_core::commit::Commit]) {
    let row: HashMap<Oid, usize> = commits
        .iter()
        .enumerate()
        .map(|(row, commit)| (commit.id, row))
        .collect();
    for (child_row, commit) in commits.iter().enumerate() {
        for parent in &commit.parents {
            let parent_row = row[parent];
            assert!(
                child_row < parent_row,
                "{what}: parent {parent} (row {parent_row}) is above its child {} (row {child_row})",
                commit.id,
            );
        }
    }
}

#[test]
fn tied_dates_never_put_a_parent_above_its_child_before_or_after_promotion() {
    let fixture = tied_dates();
    let shared = SharedRepo::open(fixture.path()).expect("the fixture opens");
    let handle = shared.handle();

    // First paint: no commit-graph, so the provisional date walk.
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table(&handle)).expect("the walk starts");
    assert!(walk.is_provisional());
    let first = walk.next_chunk(CHUNK, &never).expect("the commits read");
    assert!(first.done, "the whole fixture fits in the first chunk");
    assert_children_before_parents("first chunk", &first.commits);

    // Promotion: the full topological walk ADR-0008's background job runs.
    let tips = handle.commit_tips(&()).expect("the refs read");
    let mut sort = Walk::full_topological(&handle, &tips, table(&handle)).expect("the walk starts");
    let promoted = sort.next_chunk(CHUNK, &never).expect("the commits read");
    assert!(promoted.done);
    assert_eq!(promoted.commits.len(), first.commits.len());
    assert_children_before_parents("after promotion", &promoted.commits);
}

// ---------------------------------------------------------------------------
// In-memory graphs: the questions the corpus cannot answer.
// ---------------------------------------------------------------------------

/// A recognisable object id: `nn` repeated to full length.
fn oid(n: u8) -> Oid {
    Oid::from_bytes(&[n; Oid::SHA1_BYTES]).expect("20 bytes is a sha1")
}

/// A graph from `(id, parents, time, generation)` tuples.
fn graph(rows: &[(u8, &[u8], i64, u32)]) -> MapNodes {
    let mut nodes = HashMap::new();
    for &(id, parents, time, generation) in rows {
        nodes.insert(
            oid(id),
            (parents.iter().map(|&p| oid(p)).collect(), time, generation),
        );
    }
    MapNodes::new(nodes)
}

fn dated(nodes: &impl Nodes, tips: &[Oid], follow: Follow) -> Vec<Oid> {
    let mut reads = 0;
    let tips: Vec<_> = tips.iter().map(|tip| nodes.handle(tip)).collect();
    let mut walk = Dated::new(nodes, &tips, follow, &mut reads).expect("the walk starts");
    let mut ids = Vec::new();
    while let Some((handle, _)) = walk.next(nodes, &mut reads).expect("the graph is complete") {
        ids.push(nodes.id(handle));
    }
    ids
}

fn topological(nodes: &impl Nodes, tips: &[Oid]) -> Vec<Oid> {
    let mut reads = 0;
    let tips: Vec<_> = tips.iter().map(|tip| nodes.handle(tip)).collect();
    let mut sort = Topological::new(nodes, &tips, &mut reads).expect("the walk starts");
    let mut ids = Vec::new();
    while let Some((handle, _)) = sort.next(nodes, &mut reads).expect("the graph is complete") {
        ids.push(nodes.id(handle));
    }
    ids
}

/// A commit with two children, one of them dated *before* it.
///
/// Rebases, cherry-picks and imported history all produce this, and it is
/// the case the two walks disagree on: a date walk emits the parent as soon
/// as its newer child is out, while a topological walk holds it until both
/// children are.
fn skewed() -> MapNodes {
    graph(&[
        (0x0c, &[], 100, 1),
        (0x01, &[0x0c], 1000, 2),
        (0x02, &[0x0c], 50, 2),
    ])
}

#[test]
fn a_date_walk_can_emit_a_commit_before_one_of_its_children() {
    let nodes = skewed();
    assert_eq!(
        dated(&nodes, &[oid(0x01), oid(0x02)], Follow::AllParents),
        vec![oid(0x01), oid(0x0c), oid(0x02)],
    );
}

#[test]
fn the_topological_walk_holds_a_commit_until_every_child_is_out() {
    let nodes = skewed();
    assert_eq!(
        topological(&nodes, &[oid(0x01), oid(0x02)]),
        vec![oid(0x01), oid(0x02), oid(0x0c)],
    );
}

#[test]
fn a_date_walk_holds_a_tied_parent_until_its_unseen_child_is_out() {
    // One second throughout. The tip's parents are 0x01 and 0x0e, and 0x0e is
    // also a child of 0x01; the smaller id alone would emit 0x01 first, above
    // a child the walk has not reached yet (m1-the-graph/67).
    let nodes = graph(&[
        (0x0f, &[0x01, 0x0e], 5, 3),
        (0x0e, &[0x01], 5, 2),
        (0x01, &[], 5, 1),
    ]);
    let tips = [oid(0x0f), oid(0x0e)];
    assert_eq!(
        dated(&nodes, &tips, Follow::AllParents),
        vec![oid(0x0f), oid(0x0e), oid(0x01)],
    );
    // First parent only: 0x0f's edge to 0x0e is not followed, so the smaller
    // id goes first, but 0x01 still waits for both of its first-parent
    // children (before the fix it came out between them).
    assert_eq!(
        dated(&nodes, &tips, Follow::FirstParentOnly),
        vec![oid(0x0e), oid(0x0f), oid(0x01)],
    );
}

#[test]
fn an_exact_date_tie_is_broken_by_the_smaller_id() {
    // Two roots, same second. Without a tie-break the heap's answer depends
    // on insertion order, and the same repository draws two different graphs.
    let nodes = graph(&[(0x0a, &[], 7, 1), (0x0b, &[], 7, 1)]);
    let forwards = dated(&nodes, &[oid(0x0a), oid(0x0b)], Follow::AllParents);
    let backwards = dated(&nodes, &[oid(0x0b), oid(0x0a)], Follow::AllParents);

    assert_eq!(forwards, vec![oid(0x0a), oid(0x0b)]);
    assert_eq!(forwards, backwards);
}

#[test]
fn first_parent_follows_one_parent_and_a_full_walk_follows_both() {
    let nodes = graph(&[
        (0x0c, &[], 10, 1),
        (0x01, &[0x0c], 20, 2),
        (0x02, &[0x0c], 30, 2),
        (0x0d, &[0x01, 0x02], 40, 3),
    ]);
    assert_eq!(
        dated(&nodes, &[oid(0x0d)], Follow::FirstParentOnly),
        vec![oid(0x0d), oid(0x01), oid(0x0c)],
    );
    assert_eq!(
        dated(&nodes, &[oid(0x0d)], Follow::AllParents),
        vec![oid(0x0d), oid(0x02), oid(0x01), oid(0x0c)],
    );
}

#[test]
fn a_walk_over_no_tips_is_empty_rather_than_an_error() {
    let nodes = graph(&[]);
    assert!(topological(&nodes, &[]).is_empty());
    assert!(dated(&nodes, &[], Follow::AllParents).is_empty());
}

#[test]
fn a_commit_reachable_two_ways_is_emitted_once() {
    let nodes = graph(&[
        (0x0c, &[], 10, 1),
        (0x01, &[0x0c], 20, 2),
        (0x02, &[0x0c], 30, 2),
        (0x03, &[0x01, 0x02], 40, 3),
    ]);
    assert_eq!(topological(&nodes, &[oid(0x03)]).len(), 4);
    assert_eq!(dated(&nodes, &[oid(0x03)], Follow::AllParents).len(), 4);
}

/// Every link a walk produces in batches of `limit`, checking each batch's
/// contract on the way.
fn links_in_batches(walk: &mut Walk<'_>, limit: usize) -> Vec<graft_core::commit_handle::Link> {
    let mut links = Vec::new();
    loop {
        let chunk = walk.next_links(limit, &never).expect("the walk reads");
        assert!(!chunk.stopped, "nothing asked the walk to stop");
        assert!(chunk.links.len() <= limit, "a batch holds at most `limit`");
        if !chunk.done {
            assert_eq!(chunk.links.len(), limit, "only the last batch is short");
        }
        links.extend(chunk.links);
        if chunk.done {
            return links;
        }
    }
}

/// Every commit a walk produces in batches of `limit`, as whole commits.
fn commits_in_batches(walk: &mut Walk<'_>, limit: usize) -> Vec<super::Commit> {
    let mut commits = Vec::new();
    loop {
        let chunk = walk.next_chunk(limit, &never).expect("the walk reads");
        commits.extend(chunk.commits);
        if chunk.done {
            return commits;
        }
    }
}

/// `next_links` and `next_chunk` agree commit for commit, parents included,
/// in every order, at a batch size that splits every shape.
fn assert_links_match_commits(repo: &Path, what: &str) {
    let shared = SharedRepo::open(repo).expect("the fixture opens");
    let handle = shared.handle();
    let tips = handle
        .commit_tips(&())
        .expect("the fixture's refs can be read");
    let table = table(&handle);
    for order in [Order::Topological, Order::CommitDate, Order::FirstParent] {
        let mut commits =
            Walk::from_tips(&handle, order, &tips, Arc::clone(&table)).expect("the walk starts");
        let mut linked =
            Walk::from_tips(&handle, order, &tips, Arc::clone(&table)).expect("the walk starts");
        let commits = commits_in_batches(&mut commits, 3);
        let links = links_in_batches(&mut linked, 3);
        assert_eq!(
            links.len(),
            commits.len(),
            "{what} {order:?}: one link per commit"
        );
        for (link, commit) in links.iter().zip(&commits) {
            assert_eq!(
                table.id_of(link.commit),
                commit.id,
                "{what} {order:?}: same sequence"
            );
            let parents: Vec<Oid> = link
                .parents
                .iter()
                .map(|parent| table.id_of(parent))
                .collect();
            assert_eq!(
                parents, commit.parents,
                "{what} {order:?}: parents as walked, untrimmed"
            );
        }
    }
}

/// The fixture, or `None` for a shape this machine cannot check out (a
/// non-UTF-8 ref name on APFS), said rather than passed over.
fn available<T, E: std::fmt::Display>(shape: Shape, fixture: Result<T, E>) -> Option<T> {
    match fixture {
        Ok(fixture) => Some(fixture),
        Err(why) => {
            let reason = why.to_string();
            assert!(reason.contains("not valid UTF-8"), "{shape}: {reason}");
            eprintln!("skipped: {shape} — {reason}");
            None
        }
    }
}

#[test]
fn links_come_out_as_whole_commits_do_on_every_shape() {
    for &shape in Shape::ALL {
        let Some(repo) = available(shape, graft_fixtures::try_borrow(shape)) else {
            continue;
        };
        assert_links_match_commits(&repo, &format!("{shape:?}"));
    }
}

#[test]
fn links_come_out_as_whole_commits_do_with_a_commit_graph() {
    for &shape in Shape::ALL {
        let Some(scratch) = available(shape, graft_fixtures::try_copy_into_temp(shape)) else {
            continue;
        };
        graft_fixtures::git::run(scratch.path(), &["commit-graph", "write", "--reachable"])
            .expect("git writes a commit-graph");
        assert_links_match_commits(scratch.path(), &format!("{shape:?} with a commit-graph"));
    }
}

#[test]
fn a_link_walk_asked_to_stop_says_so_and_resumes() {
    let path = graft_fixtures::borrow(Shape::CrissCrossMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let table = table(&handle);
    let mut whole =
        Walk::all_refs(&handle, Order::CommitDate, Arc::clone(&table)).expect("the walk starts");
    let expected = all(&mut whole);
    let mut walk =
        Walk::all_refs(&handle, Order::CommitDate, Arc::clone(&table)).expect("the walk starts");

    let stopped = walk.next_links(CHUNK, &|| true).expect("the walk reads");
    assert!(stopped.stopped);
    assert!(!stopped.done);
    assert!(stopped.links.is_empty());

    // Stop after two commits, mid-batch: the two are real, and the next
    // call carries on from the third.
    let asked = std::cell::Cell::new(0);
    let after_two = || {
        asked.set(asked.get() + 1);
        asked.get() > 2
    };
    let partial = walk.next_links(CHUNK, &after_two).expect("the walk reads");
    assert!(partial.stopped);
    assert!(!partial.done);
    assert_eq!(partial.links.len(), 2);
    let mut walked: Vec<Oid> = partial
        .links
        .iter()
        .map(|link| table.id_of(link.commit))
        .collect();
    walked.extend(
        links_in_batches(&mut walk, CHUNK)
            .iter()
            .map(|link| table.id_of(link.commit)),
    );
    assert_eq!(walked, expected, "a stopped walk resumes where it stopped");
}

#[cfg(feature = "heavy-tests")]
mod heavy {
    use graft_fixtures::synthetic::{self, Scale};

    use super::assert_links_match_commits;

    #[test]
    fn links_come_out_as_whole_commits_do_on_the_five_thousand_commit_pair() {
        for scale in [Scale::of(5_000), Scale::of(5_000).with_commit_graph()] {
            let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
            assert_links_match_commits(&repo, &scale.name());
        }
    }
}
