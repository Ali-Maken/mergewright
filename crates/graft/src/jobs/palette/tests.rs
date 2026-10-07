#![allow(
    clippy::expect_used,
    reason = "fixture names and ids are literals; a bad one is a broken test"
)]

use std::cell::Cell;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::switch_refs::{RefListing, SwitchRefs};
use graft_ops::switch_list::{Relation, SwitchKind, SwitchList, SwitchRow, filter_switch_rows};

use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full).expect("a valid ref name")
}

fn listing(full: &str, tip: u8, when: i64) -> RefListing {
    RefListing {
        name: name(full),
        tip: oid(tip),
        subject: String::new(),
        author: String::new(),
        when,
        upstream: None,
    }
}

/// `feat/login` (old), `sara/feat` (new, the hit is inside a segment only
/// after `/`), `origin/feat/login` (remote, newest), `fix-feat` (inside),
/// `main` (no match).
fn refs() -> SwitchRefs {
    SwitchRefs {
        refs: vec![
            listing("refs/heads/feat/login", 1, 10),
            listing("refs/heads/sara/feat", 2, 30),
            listing("refs/heads/main", 3, 50),
            listing("refs/heads/refeat", 4, 60),
            listing("refs/remotes/origin/feat/login", 5, 40),
        ],
        previous: None,
    }
}

fn tags() -> RefBadges {
    crate::refs::badges(
        &[
            (name("refs/tags/v2.1.0"), oid(7)),
            (name("refs/tags/v2.0.0"), oid(8)),
            (name("refs/tags/release"), oid(9)),
        ],
        &graft_git::repo::HeadState::Detached(oid(3)),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk")
}

fn request(query: &str) -> NamesRequest {
    NamesRequest {
        query: Arc::from(query),
        branches: Some(Arc::new(refs())),
        tags: Some(Arc::new(tags())),
        files: None,
    }
}

fn names(hits: &Ranked<RefHit>) -> Vec<String> {
    hits.top
        .iter()
        .map(|hit| String::from_utf8_lossy(&hit.name).into_owned())
        .collect()
}

/// Segment start first; among those local before remote, newer tip first;
/// an inside hit after; totals count every match.
#[test]
fn names_rank_and_total() {
    let hits = match_names(&request("feat"), &|| false).expect("not stopped");
    let branches = hits.branches.expect("branches asked");
    assert_eq!(
        names(&branches),
        ["sara/feat", "feat/login", "origin/feat/login", "refeat"],
    );
    assert_eq!(branches.total, 4);
    assert!(branches.top[2].remote, "{branches:?}");
    assert_eq!(branches.top[1].tip, oid(1));
    assert_eq!(hits.tags.map(|t| t.total), Some(0), "no tag says feat");
    let tags = match_names(&request("v2"), &|| false)
        .and_then(|hits| hits.tags)
        .expect("tags asked");
    assert_eq!(names(&tags), ["v2.0.0", "v2.1.0"], "tier, then name");
    let tags = match_names(&request("v2.1"), &|| false)
        .and_then(|hits| hits.tags)
        .expect("tags");
    assert_eq!(names(&tags), ["v2.1.0"]);
    assert_eq!(tags.top[0].tip, oid(7));
}

/// A source the request did not carry is not answered, so its group still
/// says `reading…`.
#[test]
fn names_leave_unread_sources_unanswered() {
    let req = NamesRequest {
        branches: None,
        ..request("v")
    };
    let hits = match_names(&req, &|| false).expect("not stopped");
    assert_eq!(hits.branches, None);
    assert!(hits.tags.is_some());
}

/// More matches than [`TOP`]: the best [`TOP`] in order, the total all.
#[test]
fn names_keep_the_best_fifty_and_count_the_rest() {
    let refs = SwitchRefs {
        refs: (0..120)
            .map(|at| listing(&format!("refs/heads/b{at:03}"), 1, i64::from(at)))
            .collect(),
        previous: None,
    };
    let req = NamesRequest {
        query: Arc::from("b"),
        branches: Some(Arc::new(refs)),
        tags: None,
        files: None,
    };
    let branches = match_names(&req, &|| false)
        .and_then(|hits| hits.branches)
        .expect("answered");
    assert_eq!(branches.total, 120);
    assert_eq!(branches.top.len(), TOP);
    assert_eq!(names(&branches)[0], "b119", "newest first");
    assert_eq!(names(&branches)[TOP - 1], "b070");
}

/// `stop` is looked at while matching, every [`STOP_EVERY`] names, and the
/// answer is dropped once it says so.
#[test]
fn names_stop_when_cancelled() {
    let refs = SwitchRefs {
        refs: (0..10_000)
            .map(|at| listing(&format!("refs/heads/b{at}"), 1, 0))
            .collect(),
        previous: None,
    };
    let req = NamesRequest {
        query: Arc::from("b"),
        branches: Some(Arc::new(refs)),
        tags: None,
        files: None,
    };
    let asked = Cell::new(0);
    let stop = || {
        asked.set(asked.get() + 1);
        true
    };
    assert_eq!(match_names(&req, &stop), None);
    assert_eq!(asked.get(), 1, "stopped at the first look, not at the end");
}

/// Local rows on one fixture: the palette's BRANCHES order is the `C`
/// picker's (H4) — one query orders alike in both.
#[test]
fn palette_and_picker_order_branches_alike() {
    let local: Vec<RefListing> = refs()
        .refs
        .into_iter()
        .filter(|r| r.name.as_bytes().starts_with(b"refs/heads/"))
        .collect();
    let mut picker_rows: Vec<SwitchRow> = local
        .iter()
        .map(|r| SwitchRow {
            name: r.name.clone(),
            kind: SwitchKind::Local,
            tip: r.tip,
            subject: String::new(),
            author: String::new(),
            when: r.when,
            relation: Relation::None,
            current: false,
        })
        .collect();
    // The picker's drawn order: newest first.
    picker_rows.sort_by_key(|r| std::cmp::Reverse(r.when));
    let list = SwitchList {
        previous: None,
        rows: picker_rows,
    };
    let req = NamesRequest {
        query: Arc::from("feat"),
        branches: Some(Arc::new(SwitchRefs {
            refs: local,
            previous: None,
        })),
        tags: None,
        files: None,
    };
    let palette = names(
        &match_names(&req, &|| false)
            .and_then(|h| h.branches)
            .expect("answered"),
    );
    let picker: Vec<String> = filter_switch_rows(&list, "feat")
        .iter()
        .map(|r| String::from_utf8_lossy(r.name.shorthand()).into_owned())
        .collect();
    assert_eq!(palette, picker);
}

/// The answer's cap is the widened group's.
#[test]
fn top_is_the_widened_cap() {
    assert_eq!(TOP, crate::update::palette::WIDE_ROWS);
}

/// FILES (12b): `fuzzy_score` first, then the shorter path, then index
/// order; a path that does not match is left out; `total` counts all.
#[test]
fn files_rank_by_score_then_shorter_then_index_order() {
    let paths = TrackedPaths::from_nul_separated(
        b"docs/rebase/notes.md\0src/rebase.rs\0README.md\0lib/rebase.rs\0src/x/rebase.rs\0",
    );
    let req = NamesRequest {
        query: Arc::from("rebase"),
        branches: None,
        tags: None,
        files: Some(Arc::new(paths.clone())),
    };
    let files = match_names(&req, &|| false)
        .and_then(|hits| hits.files)
        .expect("answered");
    let names: Vec<&[u8]> = files
        .top
        .iter()
        .map(|ix| paths.get(*ix as usize).expect("an index into the read"))
        .collect();
    let score = |p: &str| fuzzy_score("rebase", p).expect("matches");
    // A stable sort keeps index order for an equal key.
    let mut want = [
        "docs/rebase/notes.md",
        "src/rebase.rs",
        "lib/rebase.rs",
        "src/x/rebase.rs",
    ];
    want.sort_by_key(|p| (score(p), p.len()));
    assert_eq!(names, want.iter().map(|p| p.as_bytes()).collect::<Vec<_>>());
    assert_eq!(files.total, 4);
}

/// The read's failure words: the cap in his terms.
#[test]
fn tracked_failure_words_say_too_many_to_list() {
    assert_eq!(
        tracked_failure_words(&StatusError::TooLarge { bytes: 1 }),
        "too many to list"
    );
}
