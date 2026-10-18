//! `search` tests: the query, the scan and the palette's first hits. Moved
//! out of `search.rs` (ease-pass 06c, move-only).

use super::*;

fn id(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("valid hex")
}

const ID: &str = "abcdef0123456789abcdef0123456789abcdef01";

#[test]
fn empty_query_is_none() {
    assert_eq!(Query::parse(""), None);
    assert_eq!(Query::parse("   "), None);
}

#[test]
fn matches_each_field_case_insensitively() {
    let q = Query::parse("Fix").unwrap();
    assert!(q.matches(id(ID), b"fix the thing", b"", b""));
    assert!(q.matches(id(ID), b"", b"Ann Fixer <ann@x>", b""));
    assert!(q.matches(id(ID), b"", b"", b"line one\nPREFIX\n"));
    assert!(!q.matches(id(ID), b"tidy", b"Bob <bob@x>", b"nothing"));
}

#[test]
fn matches_author_email() {
    let q = Query::parse("ann@example").unwrap();
    assert!(q.matches(id(ID), b"s", b"Ann <ann@example.com>", b""));
}

#[test]
fn sha_prefix_matches_only_as_prefix() {
    assert!(
        Query::parse("ABCDEF")
            .unwrap()
            .matches(id(ID), b"", b"", b"")
    );
    assert!(
        !Query::parse("cdef01")
            .unwrap()
            .matches(id(ID), b"", b"", b"")
    );
}

#[test]
fn non_utf8_haystack_and_non_ascii_needle() {
    let q = Query::parse("bug").unwrap();
    assert!(q.matches(id(ID), b"\xff\xfeBUG", b"", b""));
    let q = Query::parse("ÉTÉ").unwrap();
    assert!(q.matches(id(ID), "un été".as_bytes(), b"", b""));
}

#[test]
fn escape_sequence_subject_matches_literally() {
    let q = Query::parse("\u{1b}[31mred").unwrap();
    assert!(q.matches(id(ID), b"\x1b[31mred alert", b"", b""));
}

#[test]
fn states_describe_without_error_words() {
    assert_eq!(
        SearchState::Running { hits: 12 }.describe(),
        "12 matches so far"
    );
    assert_eq!(SearchState::Done { hits: 1 }.describe(), "1 match");
    assert_eq!(SearchState::Done { hits: 0 }.describe(), "no matches");
    assert_eq!(
        SearchState::Cancelled { hits: 3 }.describe(),
        "search cancelled, 3 matches found"
    );
    assert!(
        SearchState::Failed("x".into())
            .describe()
            .contains("failed")
    );
}

use std::sync::atomic::{AtomicBool, Ordering};

use graft_fixtures::Shape;

/// Every commit a walk from every ref reaches, in walk order.
fn walked(store: &RepoStore) -> Vec<Commit> {
    let repo = store.shared().expect("the fixture is a repository");
    let handle = repo.handle();
    let table = store.table().expect("the table opens");
    let mut walk = Walk::all_refs(&handle, Order::Topological, table).expect("walk starts");
    let mut commits = Vec::new();
    loop {
        let chunk = walk.next_chunk(500, &|| false).expect("walk reads");
        commits.extend(chunk.commits);
        if chunk.done {
            return commits;
        }
    }
}

/// Runs a scan two commits a chunk, collecting every batch.
fn scanned(store: &RepoStore, raw: &str) -> (Scanned, Vec<Arc<[Oid]>>) {
    let query = Query::parse(raw).expect("a query");
    let mut batches = Vec::new();
    let ended = scan(store, &query, 2, &|| false, &mut |hits| {
        batches.push(hits);
        true
    })
    .expect("the scan reads");
    (ended, batches)
}

fn found(batches: &[Arc<[Oid]>], id: Oid) -> bool {
    batches.iter().any(|batch| batch.contains(&id))
}

/// A hit in each field on a real repository, for the last commit the walk
/// reaches, so it lies well past the first two-commit chunk.
#[test]
fn a_scan_finds_the_last_commit_by_each_field() {
    let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
    let commits = walked(&store);
    assert!(commits.len() > 2, "the hit must lie past the first chunk");
    let last = commits.last().expect("commits");
    let subject = String::from_utf8_lossy(last.subject()).into_owned();
    let name = String::from_utf8_lossy(&last.author.name).into_owned();
    let email = String::from_utf8_lossy(&last.author.email).into_owned();
    let sha = last.id.to_string()[..10].to_uppercase();
    for raw in [
        subject.as_str(),
        name.as_str(),
        email.as_str(),
        sha.as_str(),
    ] {
        let (ended, batches) = scanned(&store, raw);
        assert_eq!(ended, Scanned::Finished, "{raw}");
        assert!(found(&batches, last.id), "{raw} finds the last commit");
    }
}

/// A hit in a message body, beyond the subject line: a commit with a body
/// is added on top of a writable copy of the linear shape.
#[test]
fn a_scan_finds_a_commit_by_its_body() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    graft_fixtures::git::run(
        scratch.path(),
        &[
            "-c",
            "user.name=Body Writer",
            "-c",
            "user.email=body@example.com",
            "commit",
            "--allow-empty",
            "-m",
            "a plain subject",
            "-m",
            "The body mentions a Quux-Frobnicator.",
        ],
    )
    .expect("commits");
    let store = RepoStore::new(scratch.path().to_path_buf(), None);
    let top = walked(&store)[0].id;

    let (_, batches) = scanned(&store, "quux-frobnicator");
    assert!(found(&batches, top), "the body line is searched");
    assert_eq!(batches.iter().map(|batch| batch.len()).sum::<usize>(), 1);
}

#[test]
fn a_scan_with_no_hit_finishes_with_nothing() {
    let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
    let (ended, batches) = scanned(&store, "no commit says zq-xj-vk-7431");
    assert_eq!((ended, batches.len()), (Scanned::Finished, 0));
}

/// Cancelled after its first hit: the scan stops at the next chunk and
/// hands over nothing more.
#[test]
fn a_cancelled_scan_stops_after_the_chunk_in_hand() {
    let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
    let email = String::from_utf8_lossy(&walked(&store)[0].author.email).into_owned();
    let query = Query::parse(&email).expect("a query");
    let cancelled = AtomicBool::new(false);
    let mut calls = 0;
    let ended = scan(
        &store,
        &query,
        1,
        &|| cancelled.load(Ordering::Relaxed),
        &mut |_| {
            calls += 1;
            cancelled.store(true, Ordering::Relaxed);
            true
        },
    )
    .expect("the scan reads");
    assert_eq!((ended, calls), (Scanned::Stopped, 1));
}
/// The palette's COMMITS are `/`'s first hits, same commits, same order,
/// each with its subject.
#[test]
fn palette_commit_hits_match_slash_search() {
    let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
    let email = String::from_utf8_lossy(&walked(&store)[0].author.email).into_owned();
    let (_, batches) = scanned(&store, &email);
    let slash: Vec<Oid> = batches
        .iter()
        .flat_map(|batch| batch.iter().copied())
        .collect();
    assert!(slash.len() > 9, "every commit matches its author");
    let query = Query::parse(&email).expect("a query");

    let hits = first_hits(&store, &query, 5, &|| false).expect("reads");

    assert_eq!(
        hits.iter().map(|hit| hit.id).collect::<Vec<_>>(),
        slash[..5].to_vec()
    );
    let commits = walked(&store);
    for hit in &hits {
        let commit = commits.iter().find(|c| c.id == hit.id).expect("walked");
        assert_eq!(&*hit.subject, commit.subject());
    }
}

/// Nine hits found → the walk ends there: it reads fewer chunks than
/// a full walk (`stop` is asked once per chunk).
#[test]
fn commits_job_stops_after_nine_hits() {
    let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
    let email = String::from_utf8_lossy(&walked(&store)[0].author.email).into_owned();
    let query = Query::parse(&email).expect("a query");
    let asked = std::cell::Cell::new(0_usize);
    let limit = crate::update::palette::COMMIT_HITS;

    let hits = first_hits(&store, &query, limit, &|| {
        asked.set(asked.get() + 1);
        false
    })
    .expect("reads");

    assert_eq!(hits.len(), 9);
    let full = std::cell::Cell::new(0_usize);
    let none = Query::parse("no commit says zq-xj-vk-7431").expect("a query");
    assert!(
        first_hits(&store, &none, limit, &|| {
            full.set(full.get() + 1);
            false
        })
        .expect("reads")
        .is_empty()
    );
    assert!(
        asked.get() < full.get(),
        "stopped early: {} stop checks vs {} for a full walk",
        asked.get(),
        full.get()
    );
}

/// Four finished hits, the selection on the first, `folded` of them hidden
/// by the default fold as the walk last counted.
fn four_hits(folded: usize) -> Search {
    let mut search = Search::new(Query::parse("login").expect("a query"), 0, (0, 0));
    let ids: Vec<Oid> = (1..=4).map(|byte| Oid::from_sha1([byte; 20])).collect();
    search.batches = vec![Arc::from(ids)];
    search.state = SearchState::Done { hits: 4 };
    search.current = Some(0);
    search.folded = folded;
    search
}

/// ease-pass 06c: the count stays every hit; the line says how many of
/// them are in folded merges, right after it.
#[test]
fn the_line_says_how_many_hits_are_in_folded_merges() {
    let keys = graft_ui::keymap::Keymap::shipped();
    assert_eq!(
        four_hits(3).line(keys),
        "/login  4 matches \u{b7} 3 in folded merges (1/4)  n/N next/prev  esc close"
    );
}

/// Nothing folded: the line is today's, byte for byte (status rule: the
/// suffix shows only when there is something to say).
#[test]
fn with_no_folded_hit_the_line_is_unchanged() {
    let keys = graft_ui::keymap::Keymap::shipped();
    assert_eq!(
        four_hits(0).line(keys),
        "/login  4 matches (1/4)  n/N next/prev  esc close"
    );
}
