//! ease-pass/07a: every local branch's ahead count, from one refs read.
//! The counts come from a closure standing in for the walk, so each test
//! also sees exactly which pairs a read asks to count.

use std::cell::Cell;
use std::sync::Arc;

use super::*;
use crate::update::AppState;
use crate::update::commit_editor::push_words;

fn oid(n: u16) -> Oid {
    let mut bytes = [0xab; Oid::SHA1_BYTES];
    bytes[..2].copy_from_slice(&n.to_be_bytes());
    Oid::from_bytes(&bytes).expect("20 bytes is a sha1")
}

fn name(text: &str) -> RefName {
    RefName::new(text.as_bytes().to_vec()).expect("a valid ref name")
}

/// `ahead` (2 ahead of its upstream, HEAD on it), `level` (in sync) and
/// `loose` (no upstream), read once. The walk answers per local tip.
fn three_branches() -> RefBadges {
    let refs = vec![
        (name("refs/heads/ahead"), oid(1)),
        (name("refs/heads/level"), oid(2)),
        (name("refs/heads/loose"), oid(3)),
        (name("refs/remotes/origin/ahead"), oid(10)),
        (name("refs/remotes/origin/level"), oid(2)),
    ];
    let upstreams: HashMap<RefName, RefName> = [
        ("refs/heads/ahead", "refs/remotes/origin/ahead"),
        ("refs/heads/level", "refs/remotes/origin/level"),
    ]
    .iter()
    .map(|&(local, up)| (name(local), name(up)))
    .collect();
    let head = HeadState::Branch(name("refs/heads/ahead"));
    badges(&refs, &head, &upstreams, &[], |local, _| {
        Ok(AheadBehind {
            ahead: if local == oid(1) { 2 } else { 0 },
            behind: 0,
        })
    })
    .expect("counting cannot fail here")
}

fn ahead_of(found: &RefBadges, branch: &str) -> Option<AheadOfUpstream> {
    let (id, ..) = found
        .branches()
        .find(|(_, name, _, _)| *name == branch.as_bytes())?;
    found
        .on(id)
        .iter()
        .find(|label| matches!(label, RefLabel::Branch { name, .. } if name == branch.as_bytes()))
        .and_then(RefLabel::ahead_of_upstream)
}

/// The settle test: path A or B. One read already holds every tracked
/// branch's count, not only HEAD's.
#[test]
fn every_tracked_local_branch_has_its_ahead_count() {
    let found = three_branches();
    let upstream_of = |branch: &str| {
        found
            .branches()
            .find(|(_, name, _, _)| *name == branch.as_bytes())
            .map(|(_, _, _, up)| up.clone())
    };
    assert_eq!(
        upstream_of("ahead"),
        Some(Upstream::Apart(AheadBehind {
            ahead: 2,
            behind: 0
        }))
    );
    assert_eq!(
        upstream_of("level"),
        Some(Upstream::InSync {
            remote: b"origin".to_vec()
        })
    );
    assert_eq!(upstream_of("loose"), Some(Upstream::None));
}

#[test]
fn ahead_of_upstream_counts_the_ahead_branch() {
    assert_eq!(
        ahead_of(&three_branches(), "ahead"),
        Some(AheadOfUpstream::By(2))
    );
}

#[test]
fn ahead_of_upstream_is_zero_in_sync() {
    assert_eq!(
        ahead_of(&three_branches(), "level"),
        Some(AheadOfUpstream::By(0))
    );
}

#[test]
fn ahead_of_upstream_without_an_upstream_is_not_pushed() {
    assert_eq!(
        ahead_of(&three_branches(), "loose"),
        Some(AheadOfUpstream::NotPushed)
    );
}

#[test]
fn ahead_of_upstream_is_none_off_a_local_branch() {
    for label in [
        RefLabel::DetachedHead,
        RefLabel::Remote(b"origin/main".to_vec()),
        RefLabel::Tag(b"v1".to_vec()),
        RefLabel::Stash(0),
    ] {
        assert_eq!(label.ahead_of_upstream(), None, "{label:?}");
    }
}

/// 500 local branches, 200 tracked: one count per tracked branch, none
/// for the rest. A second read with nothing moved, through the run's
/// cache, walks nothing.
#[test]
fn the_ahead_read_asks_once_per_tracked_branch() {
    let mut refs = Vec::new();
    let mut upstreams = HashMap::new();
    for n in 0..500u16 {
        let local = name(&format!("refs/heads/b{n}"));
        refs.push((local.clone(), oid(n)));
        if n < 200 {
            let up = name(&format!("refs/remotes/origin/b{n}"));
            refs.push((up.clone(), oid(1000 + n)));
            upstreams.insert(local, up);
        }
    }
    let head = HeadState::Branch(name("refs/heads/b0"));
    let cache = AheadBehindCache::default();
    let walks = Cell::new(0usize);
    let read = || {
        badges(&refs, &head, &upstreams, &[], |local, up| {
            cache.get_or_count(local, up, || {
                walks.set(walks.get() + 1);
                Ok(AheadBehind {
                    ahead: 1,
                    behind: 0,
                })
            })
        })
        .expect("counting cannot fail here")
    };

    let first = read();
    assert_eq!(walks.get(), 200, "cold read: one walk per tracked branch");
    let tracked = first
        .branches()
        .filter(|(.., up)| !matches!(up, Upstream::None))
        .count();
    assert_eq!(tracked, 200);

    read();
    assert_eq!(
        walks.get(),
        200,
        "warm re-read with nothing moved walks nothing"
    );
}

/// HEAD's count from the accessor equals the header's `N commits to push`:
/// both read the same refs read.
#[test]
fn heads_count_equals_the_headers() {
    let found = three_branches();
    let Some(AheadOfUpstream::By(n)) = ahead_of(&found, "ahead") else {
        panic!("HEAD's branch is tracked");
    };
    let state = AppState {
        refs: Some(Arc::new(found)),
        ..AppState::default()
    };
    assert_eq!(
        push_words(&state, true),
        Some(format!("{n} commits to push"))
    );
}
