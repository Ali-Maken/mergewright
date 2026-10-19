//! `RefBadges::live_branch` (ease-pass/07b): which live branch a merged
//! branch's name means, from the index `badges` builds.

use super::*;

fn oid(n: u8) -> Oid {
    Oid::from_bytes(&[n; Oid::SHA1_BYTES]).expect("20 bytes is a sha1")
}

fn read(names: &[&str], head: &str) -> RefBadges {
    let refs: Vec<(RefName, Oid)> = names
        .iter()
        .zip(1u8..)
        .map(|(text, id)| {
            (
                RefName::new(text.as_bytes().to_vec()).expect("a valid ref name"),
                oid(id),
            )
        })
        .collect();
    let head = HeadState::Branch(RefName::new(head.as_bytes().to_vec()).expect("a valid ref name"));
    badges(&refs, &head, &HashMap::new(), &[], |_, _| {
        Err("no upstreams here".to_owned())
    })
    .expect("nothing is counted")
}

fn local(name: &str, current: bool) -> RefLabel {
    RefLabel::Branch {
        name: name.as_bytes().to_vec(),
        current,
        upstream: Upstream::None,
    }
}

#[test]
fn a_local_branch_of_exactly_that_name_is_found() {
    let refs = read(
        &[
            "refs/heads/main",
            "refs/heads/sara/fix-toc",
            "refs/remotes/origin/sara/fix-toc",
        ],
        "refs/heads/main",
    );
    assert_eq!(
        refs.live_branch(b"sara/fix-toc"),
        Some(local("sara/fix-toc", false))
    );
}

#[test]
fn a_remote_only_branch_is_found_under_its_remote() {
    let refs = read(
        &["refs/heads/main", "refs/remotes/origin/sara/fix-toc"],
        "refs/heads/main",
    );
    assert_eq!(
        refs.live_branch(b"sara/fix-toc"),
        Some(RefLabel::Remote(b"origin/sara/fix-toc".to_vec()))
    );
    assert_eq!(
        refs.live_branch(b"origin/sara/fix-toc"),
        Some(RefLabel::Remote(b"origin/sara/fix-toc".to_vec()))
    );
}

#[test]
fn a_deleted_branch_is_none() {
    let refs = read(
        &["refs/heads/main", "refs/remotes/origin/main"],
        "refs/heads/main",
    );
    assert_eq!(refs.live_branch(b"sara/fix-toc"), None);
    assert_eq!(refs.live_branch(b"fix-toc"), None);
}

#[test]
fn heads_own_branch_is_found_as_current() {
    let refs = read(&["refs/heads/main", "refs/heads/feat"], "refs/heads/feat");
    assert_eq!(refs.live_branch(b"feat"), Some(local("feat", true)));
}

#[test]
fn origin_head_is_not_a_branch() {
    let refs = read(
        &["refs/heads/main", "refs/remotes/origin/HEAD"],
        "refs/heads/main",
    );
    assert_eq!(refs.live_branch(b"HEAD"), None);
    assert_eq!(refs.live_branch(b"origin/HEAD"), None);
}
