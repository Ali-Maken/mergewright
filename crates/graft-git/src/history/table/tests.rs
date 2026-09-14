//! Tests for the id table: round trips with and without a commit-graph, the
//! overflow past the graph, prefix collisions and SHA-256 widths.

use std::path::Path;

use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;
use graft_fixtures::Shape;

use super::{CommitTable, Overflow};
use crate::repo::SharedRepo;

/// Every commit `git` says the repository holds.
fn every_commit(repo: &Path) -> Vec<Oid> {
    let stdout = graft_fixtures::git::run(repo, &["rev-list", "--all"]).expect("git rev-list runs");
    String::from_utf8(stdout)
        .expect("git prints ids as ASCII")
        .lines()
        .map(|line| Oid::from_hex(line).expect("git prints full ids"))
        .collect()
}

/// The fixture, or `None` for the one reason a shape may be missing on this
/// machine (a non-UTF-8 ref name on APFS), said rather than passed over. Any
/// other failure fails the test.
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

fn table_for(repo: &Path) -> CommitTable {
    let shared = SharedRepo::open(repo).expect("the fixture opens");
    CommitTable::open(&shared.handle()).expect("the table opens")
}

/// Interns every commit and checks each round trip, then the count.
fn assert_round_trips(table: &CommitTable, commits: &[Oid], what: &str) {
    for id in commits {
        let handle = table.intern(id);
        assert_eq!(table.id_of(handle), *id, "{what}: id_of(intern) for {id}");
        assert_eq!(
            table.handle_of(id),
            Some(handle),
            "{what}: handle_of for {id}"
        );
        assert_eq!(table.intern(id), handle, "{what}: interning twice for {id}");
    }
    assert_eq!(table.len(), commits.len(), "{what}: one entry per commit");
}

#[test]
fn every_corpus_shape_round_trips_without_a_commit_graph() {
    for &shape in Shape::ALL {
        let Some(repo) = available(shape, graft_fixtures::try_borrow(shape)) else {
            continue;
        };
        let table = table_for(&repo);
        assert!(!table.has_commit_graph(), "{shape:?} ships no commit-graph");
        assert_eq!(table.graph_count(), 0);
        assert_round_trips(&table, &every_commit(&repo), &format!("{shape:?}"));
        assert_eq!(table.overflow_len(), table.len());
    }
}

#[test]
fn every_corpus_shape_round_trips_with_a_commit_graph() {
    for &shape in Shape::ALL {
        let Some(scratch) = available(shape, graft_fixtures::try_copy_into_temp(shape)) else {
            continue;
        };
        graft_fixtures::git::run(scratch.path(), &["commit-graph", "write", "--reachable"])
            .expect("git writes a commit-graph");
        let table = table_for(scratch.path());
        let commits = every_commit(scratch.path());
        assert!(table.has_commit_graph(), "{shape:?}");
        assert_eq!(table.graph_count() as usize, commits.len(), "{shape:?}");
        for id in &commits {
            let handle = table
                .handle_of(id)
                .expect("a graph commit has a handle before interning");
            assert!(
                handle.0 < table.graph_count(),
                "{shape:?}: {id} is a graph position"
            );
        }
        assert_round_trips(&table, &commits, &format!("{shape:?}"));
        assert_eq!(
            table.overflow_len(),
            0,
            "{shape:?}: nothing leaves the graph"
        );
        assert_eq!(
            table.approximate_bytes(),
            0,
            "{shape:?}: the graph is not copied"
        );
    }
}

#[test]
fn a_commit_made_after_the_graph_interns_above_it() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    let dir = scratch.path();
    graft_fixtures::git::run(dir, &["commit-graph", "write", "--reachable"])
        .expect("git writes a commit-graph");
    let stdout = graft_fixtures::git::run(
        dir,
        &[
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            "HEAD",
            "-m",
            "after the graph",
        ],
    )
    .expect("git makes a commit");
    let late = Oid::from_hex(String::from_utf8(stdout).expect("ASCII").trim()).expect("an id");

    let table = table_for(dir);
    assert_eq!(table.handle_of(&late), None, "not interned yet");
    let handle = table.intern(&late);
    assert!(handle.0 >= table.graph_count());
    assert_eq!(table.overflow_len(), 1);
    assert_eq!(table.id_of(handle), late);
    assert_eq!(table.handle_of(&late), Some(handle));
}

/// An id of `len` bytes: `head` for the first eight, `tail` for the rest.
fn made_up(len: usize, head: u8, tail: u8) -> Oid {
    let mut bytes = vec![tail; len];
    bytes[..8].fill(head);
    Oid::from_bytes(&bytes).expect("20 or 32 bytes")
}

#[test]
fn two_ids_sharing_their_first_eight_bytes_get_two_handles() {
    let table = CommitTable::with_graph(None, Oid::SHA1_BYTES);
    let first = made_up(Oid::SHA1_BYTES, 0xab, 1);
    let second = made_up(Oid::SHA1_BYTES, 0xab, 2);
    let (a, b) = (table.intern(&first), table.intern(&second));
    assert_ne!(a, b);
    assert_eq!((table.id_of(a), table.id_of(b)), (first, second));
    assert_eq!(
        (table.handle_of(&first), table.handle_of(&second)),
        (Some(a), Some(b))
    );
    assert_eq!(table.handle_of(&made_up(Oid::SHA1_BYTES, 0xab, 3)), None);
    assert_eq!(table.len(), 2);
}

#[test]
fn the_overflow_holds_sha256_ids_at_their_own_width() {
    let table = CommitTable::with_graph(None, Oid::MAX_BYTES);
    let ids: Vec<_> = (0..40u8)
        .map(|n| made_up(Oid::MAX_BYTES, n % 4, n))
        .collect();
    let handles: Vec<_> = ids.iter().map(|id| table.intern(id)).collect();
    for (id, handle) in ids.iter().zip(&handles) {
        assert_eq!(table.id_of(*handle), *id);
    }
    assert_eq!(handles, (0..40).map(CommitHandle).collect::<Vec<_>>());
    let overflow = table.overflow.read().expect("not poisoned");
    let stored: usize = overflow.pages.iter().map(Vec::len).sum();
    assert_eq!(stored, 40 * Oid::MAX_BYTES);
}

#[test]
#[should_panic(expected = "another table")]
fn a_handle_from_nowhere_panics() {
    let table = CommitTable::with_graph(None, Oid::SHA1_BYTES);
    let _ = table.id_of(CommitHandle(3));
}

#[test]
fn the_table_can_be_shared_across_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<CommitTable>();
    let _ = Overflow::new(Oid::SHA1_BYTES);
}

#[cfg(feature = "heavy-tests")]
mod heavy {
    use graft_fixtures::synthetic::{self, Scale};

    use super::{assert_round_trips, every_commit, table_for};

    #[test]
    fn the_five_thousand_commit_pair_round_trips() {
        for scale in [Scale::of(5_000), Scale::of(5_000).with_commit_graph()] {
            let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
            let table = table_for(&repo);
            assert_round_trips(&table, &every_commit(&repo), &scale.name());
        }
    }

    #[test]
    fn the_overflow_at_a_hundred_thousand_commits_costs_under_sixty_bytes_each() {
        let repo = synthetic::borrow(Scale::HEAVY).unwrap_or_else(|why| panic!("{why}"));
        let table = table_for(&repo);
        let commits = every_commit(&repo);
        for id in &commits {
            table.intern(id);
        }
        let bytes = table.approximate_bytes();
        let per_commit = bytes.div_ceil(commits.len());
        println!(
            "overflow at {} commits, no commit-graph: {bytes} B, {per_commit} B/commit",
            commits.len()
        );
        assert_eq!(table.len(), commits.len());
        assert!(bytes <= 60 * commits.len(), "{per_commit} B/commit");
    }
}
