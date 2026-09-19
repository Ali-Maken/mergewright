use super::*;
use crate::fixture::Fixture;

fn open(fixture: &Fixture) -> RepoHandle {
    RepoHandle::open(fixture.root()).expect("a fixture repository opens")
}

fn path(fixture: &Fixture) -> &str {
    fixture.root().to_str().expect("fixture paths are UTF-8")
}

/// A work repository whose `main` was pushed with `-u` to a real bare
/// remote: real `origin`, real `branch.main.*`, real tracking ref.
fn pushed() -> (Fixture, Fixture) {
    let bare = Fixture::bare_repo();
    let work = Fixture::with_two_commits();
    work.git(&["remote", "add", "origin", path(&bare)]);
    work.git(&["push", "--quiet", "-u", "origin", "main"]);
    (bare, work)
}

#[test]
fn list_names_origin_first() {
    let (bare, work) = pushed();
    work.git(&["remote", "add", "zeta", "https://example.invalid/z.git"]);
    work.git(&["remote", "add", "alpha", "https://example.invalid/a.git"]);
    work.git(&[
        "remote",
        "set-url",
        "--push",
        "alpha",
        "ssh://example.invalid/a.git",
    ]);

    let remotes = list(&open(&work)).expect("remotes read");

    assert_eq!(
        remotes,
        vec![
            Remote {
                name: "origin".to_owned(),
                fetch_url: path(&bare).to_owned(),
                push_url: None,
            },
            Remote {
                name: "alpha".to_owned(),
                fetch_url: "https://example.invalid/a.git".to_owned(),
                push_url: Some("ssh://example.invalid/a.git".to_owned()),
            },
            Remote {
                name: "zeta".to_owned(),
                fetch_url: "https://example.invalid/z.git".to_owned(),
                push_url: None,
            },
        ]
    );
}

#[test]
fn list_is_empty_without_remotes() {
    let work = Fixture::with_two_commits();
    assert_eq!(list(&open(&work)).expect("remotes read"), vec![]);
}

#[test]
fn list_skips_a_remote_with_no_url() {
    let work = Fixture::with_two_commits();
    work.git(&[
        "config",
        "remote.ghost.fetch",
        "+refs/heads/*:refs/remotes/ghost/*",
    ]);
    assert_eq!(list(&open(&work)).expect("remotes read"), vec![]);
}

#[test]
fn upstream_of_reads_config_and_tip() {
    let (_bare, work) = pushed();
    let tip = work.rev_parse("main");

    let upstream = upstream_of(&open(&work), "refs/heads/main")
        .expect("config reads")
        .expect("main tracks origin/main");

    assert_eq!(upstream.remote, "origin");
    assert_eq!(upstream.remote_branch, "refs/heads/main");
    assert_eq!(upstream.tracking_ref, "refs/remotes/origin/main");
    assert_eq!(
        upstream.tracking_tip.map(|id| id.to_string()),
        Some(tip),
        "the tracking ref points where main was pushed"
    );
}

#[test]
fn upstream_of_is_none_without_tracking() {
    let (_bare, work) = pushed();
    work.git(&["branch", "loose"]);
    assert_eq!(
        upstream_of(&open(&work), "refs/heads/loose").expect("config reads"),
        None
    );
}

/// The refspec decides the tracking ref, not a string pattern: a remote that
/// only fetches `other` does not map `main` anywhere.
#[test]
fn upstream_of_is_none_when_the_refspec_does_not_map_the_branch() {
    let (_bare, work) = pushed();
    work.git(&[
        "config",
        "remote.origin.fetch",
        "+refs/heads/other:refs/remotes/origin/other",
    ]);
    assert_eq!(
        upstream_of(&open(&work), "refs/heads/main").expect("config reads"),
        None
    );
}

/// And a non-standard refspec that does map it is followed, wherever it
/// lands.
#[test]
fn upstream_of_follows_a_non_standard_refspec() {
    let (_bare, work) = pushed();
    work.git(&[
        "config",
        "remote.origin.fetch",
        "+refs/heads/*:refs/remotes/mirror/*",
    ]);
    let upstream = upstream_of(&open(&work), "refs/heads/main")
        .expect("config reads")
        .expect("main still tracks through the refspec");
    assert_eq!(upstream.tracking_ref, "refs/remotes/mirror/main");
    assert_eq!(upstream.tracking_tip, None, "nothing fetched there yet");
}
