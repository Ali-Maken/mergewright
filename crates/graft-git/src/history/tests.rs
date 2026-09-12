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

use graft_core::oid::Oid;
use graft_fixtures::Shape;

use super::node::{MapNodes, Node, Nodes};
use super::simple::{Dated, Follow};
use super::topological::Topological;
use super::{CHUNK, Chunk, Order, Walk, WalkError, read_commit};
use crate::repo::SharedRepo;

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
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

    assert_eq!(all(&mut walk), rev_list(&path, &["--date-order", "--all"]));
}

#[test]
fn commit_date_order_agrees_with_git_on_the_octopus_fixture() {
    let path = graft_fixtures::borrow(Shape::OctopusMerge);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

    assert_eq!(all(&mut walk), rev_list(&path, &["--date-order", "--all"]));
}

#[test]
fn the_linear_fixture_comes_out_newest_first() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::Topological).expect("the walk starts");

    assert_eq!(walk.order_in_force(), Order::Topological);
    // Every fixture's timestamps strictly increase with its stream order, so
    // there is exactly one date order and it is topological: git's
    // `--date-order` is a sound oracle for this walk on this corpus.
    assert_eq!(
        all(&mut walk),
        rev_list(scratch.path(), &["--date-order", "--all"])
    );
}

#[test]
fn topological_order_agrees_with_git_on_the_octopus_fixture() {
    let scratch = with_commit_graph(Shape::OctopusMerge);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let handle = shared.handle();
    let mut walk = Walk::all_refs(&handle, Order::Topological).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::Topological).expect("the walk starts");

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
    let mut walk = Walk::from_tips(&handle, Order::FirstParent, &[tip]).expect("the walk starts");

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
    let mut trunk = Walk::from_tips(&handle, Order::FirstParent, &[tip]).expect("the walk starts");
    let mut whole = Walk::from_tips(&handle, Order::CommitDate, &[tip]).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::Topological).expect("the walk starts");

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
    let walk = Walk::all_refs(&handle, Order::Topological).expect("the walk starts");

    assert!(walk.has_commit_graph());
    assert_eq!(walk.order_in_force(), Order::Topological);
    assert!(!walk.is_provisional());
}

#[test]
fn asking_for_date_order_is_never_provisional() {
    let path = graft_fixtures::borrow(Shape::Linear);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let handle = shared.handle();
    let walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

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
        .commit_tips()
        .expect("the fixture's refs can be read");
    let mut walk = Walk::full_topological(&handle, &tips).expect("the walk starts");

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
        .commit_tips()
        .expect("the fixture's refs can be read");
    let mut walk = Walk::full_topological(&handle, &tips).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::Topological).expect("the walk starts");
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

    let mut whole = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");
    let expected = all(&mut whole);

    let mut chunked = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");
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
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

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
    let mut walk = Walk::all_refs(&handle, Order::CommitDate).expect("the walk starts");

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

// ---------------------------------------------------------------------------
// In-memory graphs: the questions the corpus cannot answer.
// ---------------------------------------------------------------------------

/// A recognisable object id: `nn` repeated to full length.
fn oid(n: u8) -> Oid {
    Oid::from_bytes(&[n; Oid::SHA1_BYTES]).expect("20 bytes is a sha1")
}

/// A graph from `(id, parents, time, generation)` tuples.
fn graph(rows: &[(u8, &[u8], i64, u64)]) -> MapNodes {
    let mut nodes = HashMap::new();
    for &(id, parents, time, generation) in rows {
        nodes.insert(
            oid(id),
            Node {
                id: oid(id),
                parents: parents.iter().map(|&p| oid(p)).collect(),
                time,
                generation,
            },
        );
    }
    MapNodes(nodes)
}

fn dated(nodes: &impl Nodes, tips: &[Oid], follow: Follow) -> Vec<Oid> {
    let mut reads = 0;
    let mut walk = Dated::new(nodes, tips, follow, &mut reads).expect("the walk starts");
    let mut ids = Vec::new();
    while let Some(id) = walk.next(nodes, &mut reads).expect("the graph is complete") {
        ids.push(id);
    }
    ids
}

fn topological(nodes: &impl Nodes, tips: &[Oid]) -> Vec<Oid> {
    let mut reads = 0;
    let mut sort = Topological::new(nodes, tips, &mut reads).expect("the walk starts");
    let mut ids = Vec::new();
    while let Some(id) = sort.next(nodes, &mut reads).expect("the graph is complete") {
        ids.push(id);
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
