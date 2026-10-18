//! Every ref state a badge has to tell apart, without a repository: the
//! counts come from a closure standing in for the walk.

use std::sync::Arc;

use super::*;

fn oid(n: u8) -> Oid {
    Oid::from_bytes(&[n; Oid::SHA1_BYTES]).expect("20 bytes is a sha1")
}

fn name(text: &str) -> RefName {
    RefName::new(text.as_bytes().to_vec()).expect("a valid ref name")
}

fn refs(pairs: &[(&str, u8)]) -> Vec<(RefName, Oid)> {
    pairs
        .iter()
        .map(|&(text, id)| (name(text), oid(id)))
        .collect()
}

fn tracking(pairs: &[(&str, &str)]) -> HashMap<RefName, RefName> {
    pairs
        .iter()
        .map(|&(local, upstream)| (name(local), name(upstream)))
        .collect()
}

fn on_main() -> HeadState {
    HeadState::Branch(name("refs/heads/main"))
}

/// Badges where every branch/upstream pair is `counts` apart, and no
/// stashes. `with_stashes` is the one that has them.
fn with_counts(
    refs: &[(RefName, Oid)],
    head: &HeadState,
    upstreams: &HashMap<RefName, RefName>,
    counts: AheadBehind,
) -> RefBadges {
    with_stashes(refs, head, upstreams, counts, &[])
}

/// The same, with `stashes` as `mergewright_git::stash::commits` would read them
/// off `refs/stash`'s reflog (m3-working-tree-undo-and-stash/06).
fn with_stashes(
    refs: &[(RefName, Oid)],
    head: &HeadState,
    upstreams: &HashMap<RefName, RefName>,
    counts: AheadBehind,
    stashes: &[(Oid, usize)],
) -> RefBadges {
    badges(refs, head, upstreams, stashes, |_, _| Ok(counts)).expect("counting cannot fail here")
}

fn branch(name: &str, current: bool, upstream: Upstream) -> RefLabel {
    RefLabel::Branch {
        name: name.as_bytes().to_vec(),
        current,
        upstream,
    }
}

const LEVEL: AheadBehind = AheadBehind {
    ahead: 0,
    behind: 0,
};

#[test]
fn in_sync_merges_the_remote_into_one_badge() {
    let refs = refs(&[("refs/heads/main", 1), ("refs/remotes/origin/main", 1)]);
    let up = tracking(&[("refs/heads/main", "refs/remotes/origin/main")]);

    let found = with_counts(&refs, &on_main(), &up, LEVEL);

    assert_eq!(
        found.on(oid(1)),
        [branch(
            "main",
            true,
            Upstream::InSync {
                remote: b"origin".to_vec()
            }
        )]
    );
    assert_eq!(found.head(), Some(oid(1)));
}

#[test]
fn ahead_only_keeps_the_remote_badge_on_its_own_commit() {
    let refs = refs(&[("refs/heads/main", 2), ("refs/remotes/origin/main", 1)]);
    let up = tracking(&[("refs/heads/main", "refs/remotes/origin/main")]);
    let ahead = AheadBehind {
        ahead: 1,
        behind: 0,
    };

    let found = with_counts(&refs, &on_main(), &up, ahead);

    assert_eq!(
        found.on(oid(2)),
        [branch("main", true, Upstream::Apart(ahead))]
    );
    assert_eq!(
        found.on(oid(1)),
        [RefLabel::Remote(b"origin/main".to_vec())]
    );
}

#[test]
fn behind_only() {
    let refs = refs(&[("refs/heads/main", 1), ("refs/remotes/origin/main", 2)]);
    let up = tracking(&[("refs/heads/main", "refs/remotes/origin/main")]);
    let behind = AheadBehind {
        ahead: 0,
        behind: 1,
    };

    let found = with_counts(&refs, &on_main(), &up, behind);

    assert_eq!(
        found.on(oid(1)),
        [branch("main", true, Upstream::Apart(behind))]
    );
    assert_eq!(
        found.on(oid(2)),
        [RefLabel::Remote(b"origin/main".to_vec())]
    );
}

#[test]
fn diverged_passes_both_counts_and_asks_with_the_right_tips() {
    let refs = refs(&[("refs/heads/topic", 3), ("refs/remotes/origin/topic", 4)]);
    let up = tracking(&[("refs/heads/topic", "refs/remotes/origin/topic")]);
    let apart = AheadBehind {
        ahead: 2,
        behind: 1,
    };
    let mut asked = Vec::new();

    let found = badges(&refs, &on_main(), &up, &[], |local, upstream| {
        asked.push((local, upstream));
        Ok(apart)
    })
    .expect("counting cannot fail here");

    assert_eq!(asked, [(oid(3), oid(4))]);
    assert_eq!(
        found.on(oid(3)),
        [branch("topic", false, Upstream::Apart(apart))]
    );
    assert_eq!(found.head(), None, "HEAD's branch has no tip in these refs");
}

#[test]
fn detached_head_gets_its_own_badge_first() {
    let refs = refs(&[("refs/heads/main", 1)]);

    let found = with_counts(&refs, &HeadState::Detached(oid(1)), &HashMap::new(), LEVEL);

    assert_eq!(
        found.on(oid(1)),
        [
            RefLabel::DetachedHead,
            branch("main", false, Upstream::None)
        ]
    );
    assert_eq!(found.head(), Some(oid(1)));
}

#[test]
fn a_tag_only_commit_carries_just_the_tag() {
    let refs = refs(&[("refs/heads/main", 1), ("refs/tags/archived", 2)]);

    let found = with_counts(&refs, &on_main(), &HashMap::new(), LEVEL);

    assert_eq!(found.on(oid(2)), [RefLabel::Tag(b"archived".to_vec())]);
}

#[test]
fn every_stash_entry_is_badged_by_its_own_index() {
    // `refs/stash` names only the newest; the older entries reach the
    // badges through the reflog read, which is the whole point of the
    // change (`KNOWN_GAPS.md`, m3-working-tree-undo-and-stash/06).
    let refs = refs(&[("refs/heads/main", 1), ("refs/stash", 5)]);

    let found = with_stashes(
        &refs,
        &on_main(),
        &HashMap::new(),
        LEVEL,
        &[(oid(5), 0), (oid(6), 1)],
    );

    assert_eq!(found.on(oid(5)), [RefLabel::Stash(0)]);
    assert_eq!(found.on(oid(6)), [RefLabel::Stash(1)]);
    assert_eq!(found.stash(), Some(oid(5)), "`stash()` is still the newest");
    assert_eq!(found.stashes(), [(oid(5), 0), (oid(6), 1)]);
}

#[test]
fn a_repository_with_no_stash_has_no_stash_badges() {
    let refs = refs(&[("refs/heads/main", 1)]);

    let found = with_counts(&refs, &on_main(), &HashMap::new(), LEVEL);

    assert_eq!(found.stash(), None);
    assert!(found.stashes().is_empty());
}

#[test]
fn an_unborn_branch_marks_no_head_and_badges_the_rest() {
    let refs = refs(&[("refs/heads/main", 1)]);

    let found = with_counts(
        &refs,
        &HeadState::Unborn(name("refs/heads/fresh")),
        &HashMap::new(),
        LEVEL,
    );

    assert_eq!(found.head(), None);
    assert_eq!(found.on(oid(1)), [branch("main", false, Upstream::None)]);
}

#[test]
fn an_upstream_that_does_not_exist_is_no_upstream() {
    let refs = refs(&[("refs/heads/main", 1)]);
    let up = tracking(&[("refs/heads/main", "refs/remotes/origin/main")]);

    let found = badges(&refs, &on_main(), &up, &[], |_, _| {
        panic!("nothing to count against a missing ref")
    })
    .expect("no count is asked for");

    assert_eq!(found.on(oid(1)), [branch("main", true, Upstream::None)]);
}

#[test]
fn a_remote_head_pointer_is_not_a_badge() {
    let refs = refs(&[
        ("refs/remotes/origin/HEAD", 1),
        ("refs/remotes/origin/main", 1),
    ]);

    let found = with_counts(&refs, &on_main(), &HashMap::new(), LEVEL);

    assert_eq!(
        found.on(oid(1)),
        [RefLabel::Remote(b"origin/main".to_vec())]
    );
}

#[test]
fn the_cache_walks_a_pair_once_and_again_when_a_tip_moves() {
    let cache = AheadBehindCache::default();
    let walks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let walk = || {
        walks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(LEVEL)
    };

    cache.get_or_count(oid(1), oid(2), walk).expect("walks");
    cache.get_or_count(oid(1), oid(2), walk).expect("cached");
    assert_eq!(walks.load(std::sync::atomic::Ordering::SeqCst), 1);

    cache
        .get_or_count(oid(3), oid(2), walk)
        .expect("the branch moved");
    cache
        .get_or_count(oid(3), oid(4), walk)
        .expect("the upstream moved");
    assert_eq!(walks.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn reading_the_ref_states_fixture_counts_through_the_real_walk() {
    let dir = mergewright_fixtures::borrow(mergewright_fixtures::Shape::RefStates);
    let store = RepoStore::new(dir, None);
    let cache = AheadBehindCache::default();

    let found = read(&store, &cache).expect("the fixture reads");

    let labels: Vec<&RefLabel> = found.by_commit.values().flatten().collect();
    assert!(labels.contains(&&branch(
        "ahead",
        false,
        Upstream::Apart(AheadBehind {
            ahead: 1,
            behind: 0
        })
    )));
    assert!(labels.contains(&&branch(
        "behind",
        false,
        Upstream::Apart(AheadBehind {
            ahead: 0,
            behind: 1
        })
    )));
    assert!(
        labels.iter().any(|label| matches!(
            label,
            RefLabel::Branch { name, upstream: Upstream::InSync { remote }, .. }
                if name == b"main" && remote == b"origin"
        )),
        "{labels:?}"
    );
    assert!(labels.contains(&&RefLabel::Tag(b"archived".to_vec())));
}
