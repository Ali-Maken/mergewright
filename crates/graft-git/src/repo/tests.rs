//! The awkward repositories, not the happy path: opening one of these wrongly
//! is what makes a git tool untrustworthy (PROMPT.md §6.3).
//!
//! Not covered here: a path that is not valid UTF-8. APFS and HFS+ reject
//! filenames that are not valid UTF-8, so such a fixture cannot be created on
//! the platform these tests run on. The API takes that case seriously anyway —
//! every path in and out of this module is a `Path`, never a `String` — and
//! the behaviour itself belongs in a Linux run of the heavy tier.

use std::path::Path;

use super::*;
use crate::fixture::Fixture;

/// Compares two paths by their canonical form, so that a symlinked temporary
/// directory does not fail a test that is otherwise right.
#[track_caller]
fn assert_same_path(actual: &Path, expected: &Path) {
    let canonical = |path: &Path| {
        path.canonicalize()
            .unwrap_or_else(|error| panic!("{} cannot be resolved: {error}", path.display()))
    };
    assert_eq!(canonical(actual), canonical(expected));
}

/// The full ref name of a `HEAD` that is on a branch, born or unborn.
#[track_caller]
fn branch_name(head: &HeadState) -> &[u8] {
    match head {
        HeadState::Branch(name) | HeadState::Unborn(name) => name.as_bytes(),
        HeadState::Detached(id) => panic!("expected a branch, HEAD is detached at {id}"),
    }
}

#[test]
fn opens_a_repository_at_its_root() {
    let fixture = Fixture::with_two_commits();

    let repo = RepoHandle::open(fixture.root()).expect("a repository with two commits opens");

    assert_same_path(repo.git_dir(), &fixture.root().join(".git"));
    assert_same_path(repo.workdir().expect("not bare"), fixture.root());
    assert!(!repo.is_bare());
    assert_eq!(
        branch_name(&repo.head().expect("HEAD is readable")),
        b"refs/heads/main"
    );
}

#[test]
fn discovers_the_repository_from_a_subdirectory() {
    let fixture = Fixture::with_two_commits();
    let nested = fixture.subdirectory("src/deeply/nested");

    let repo = RepoHandle::open(&nested).expect("discovery walks up to the repository");

    assert_same_path(repo.git_dir(), &fixture.root().join(".git"));
    assert_same_path(repo.workdir().expect("not bare"), fixture.root());
}

#[test]
fn an_empty_repository_has_an_unborn_head() {
    let fixture = Fixture::empty_repo();

    let repo = RepoHandle::open(fixture.root()).expect("a repository with no commits opens");
    let head = repo
        .head()
        .expect("HEAD is readable before the first commit");

    // The branch does not exist yet, but its name does: it is where the first
    // commit will land, and it is what the status line has to show.
    assert_eq!(
        head,
        HeadState::Unborn(RefName::new("refs/heads/main").expect("valid"))
    );
    assert_same_path(repo.workdir().expect("not bare"), fixture.root());
}

#[test]
fn a_detached_head_reports_the_commit_it_points_at() {
    let fixture = Fixture::with_two_commits();
    let first = fixture.rev_parse("HEAD~1");
    fixture.git(&["checkout", "--detach", &first]);

    let repo = RepoHandle::open(fixture.root()).expect("a detached repository opens");
    let head = repo.head().expect("HEAD is readable while detached");

    assert_eq!(
        head,
        HeadState::Detached(Oid::from_hex(&first).expect("git printed hex"))
    );
}

#[test]
fn a_bare_repository_has_no_working_tree() {
    let fixture = Fixture::bare_repo();

    let repo = RepoHandle::open(fixture.root()).expect("a bare repository opens");

    assert!(repo.is_bare());
    assert_eq!(repo.workdir(), None);
    assert_same_path(repo.git_dir(), fixture.root());
}

#[test]
fn a_linked_worktree_keeps_its_own_head_and_shares_the_repository() {
    let fixture = Fixture::with_two_commits();
    let worktree = fixture.root().join("linked");
    fixture.git(&["worktree", "add", "-b", "side", "linked"]);

    let repo = RepoHandle::open(&worktree).expect("a linked worktree opens");

    assert_same_path(repo.workdir().expect("not bare"), &worktree);
    // Its own git dir under the main repository's, but the same common dir —
    // which is how a worktree is told apart from an ordinary repository.
    assert_same_path(
        repo.git_dir(),
        &fixture.root().join(".git").join("worktrees").join("linked"),
    );
    assert_same_path(repo.common_dir(), &fixture.root().join(".git"));
    // `HEAD` is per-worktree: the main checkout is still on `main`.
    assert_eq!(
        branch_name(&repo.head().expect("HEAD is readable")),
        b"refs/heads/side"
    );

    let main = RepoHandle::open(fixture.root()).expect("the main worktree still opens");
    assert_eq!(
        branch_name(&main.head().expect("HEAD is readable")),
        b"refs/heads/main"
    );
}

#[test]
fn a_submodules_working_directory_opens_the_submodule_not_its_parent() {
    let inner = Fixture::with_two_commits();
    let outer = Fixture::with_two_commits();
    // `protocol.file.allow` is `user` by default since git 2.38.1, so a
    // submodule from a local path has to be allowed explicitly. Still no
    // network: the source is a directory on this machine.
    outer.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        &inner.root().to_string_lossy(),
        "vendor/inner",
    ]);
    let submodule = outer.root().join("vendor").join("inner");

    let repo = RepoHandle::open(&submodule).expect("a submodule working directory opens");

    // A submodule's `.git` is a file pointing into the parent's `.git/modules`.
    // Opening its working directory must follow that pointer rather than
    // walking up to the parent repository.
    assert_same_path(repo.workdir().expect("not bare"), &submodule);
    assert_same_path(
        repo.git_dir(),
        &outer
            .root()
            .join(".git")
            .join("modules")
            .join("vendor")
            .join("inner"),
    );
}

#[test]
fn a_plain_directory_is_not_a_repository() {
    let fixture = Fixture::empty_dir();

    let error = RepoHandle::open(fixture.root()).expect_err("a bare directory is not a repository");

    assert!(
        matches!(error, OpenError::NotARepo { .. }),
        "expected NotARepo, got {error:?}"
    );
    assert_same_path(error.path(), fixture.root());
}

#[test]
fn a_repository_below_the_path_is_not_found() {
    // Discovery walks up, never down: a directory of checkouts is not itself
    // a repository, and answering with the first one found inside it would
    // silently operate on the wrong repository.
    let fixture = Fixture::empty_dir();
    let child = fixture.subdirectory("checkout");
    fixture.git_in(&child, &["init", "-b", "main"]);

    let error = RepoHandle::open(fixture.root()).expect_err("the repository is below, not above");

    assert!(
        matches!(error, OpenError::NotARepo { .. }),
        "expected NotARepo, got {error:?}"
    );
}

#[test]
fn a_path_that_does_not_exist_is_reported_as_such() {
    let fixture = Fixture::empty_dir();
    let missing = fixture.root().join("no-such-directory");

    let error = RepoHandle::open(&missing).expect_err("a missing path cannot be opened");

    assert!(
        matches!(error, OpenError::Inaccessible { .. }),
        "expected Inaccessible, got {error:?}"
    );
    assert_eq!(error.path(), missing);
}

#[test]
fn every_open_error_names_the_path_it_was_given() {
    // The message drives the exit screen, so the path has to survive into it.
    let fixture = Fixture::empty_dir();

    let error = RepoHandle::open(fixture.root()).expect_err("not a repository");

    assert!(
        error
            .to_string()
            .contains(&fixture.root().display().to_string()),
        "{error} should name {}",
        fixture.root().display()
    );
}

/// Runs `git` in `dir`, failing the test on anything but success.
#[track_caller]
fn git_in(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap_or_else(|why| panic!("git runs: {why}"));
    assert!(status.success(), "git {args:?} failed");
}

/// The ref-states shape, whose `main`, `ahead` and `behind` all track
/// `origin`, with `behind`'s upstream setting removed.
fn ref_states_with_upstreams() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::RefStates);
    git_in(
        scratch.path(),
        &["config", "--unset", "branch.behind.merge"],
    );
    scratch
}

fn name(text: &str) -> RefName {
    RefName::new(text.as_bytes().to_vec()).expect("a valid ref name")
}

#[test]
fn a_configured_upstream_resolves_through_the_fetch_refspec() {
    let scratch = ref_states_with_upstreams();
    let repo = RepoHandle::open(scratch.path()).expect("the fixture opens");

    assert_eq!(
        repo.upstream(&name("refs/heads/ahead"))
            .expect("config reads"),
        Some(name("refs/remotes/origin/ahead"))
    );
    assert_eq!(
        repo.upstream(&name("refs/heads/behind"))
            .expect("config reads"),
        None,
        "no branch.behind.merge setting, so no upstream"
    );
}

#[test]
fn annotated_tags_peel_to_their_commit() {
    let scratch = ref_states_with_upstreams();
    let repo = RepoHandle::open(scratch.path()).expect("the fixture opens");

    let peeled = repo.peeled_references(&()).expect("refs read");
    let tips = repo.commit_tips(&()).expect("tips read");
    for (ref_name, id) in &peeled {
        assert!(tips.contains(id), "{ref_name:?} peels to a tip");
    }
    assert!(
        peeled
            .iter()
            .any(|(ref_name, _)| ref_name.as_bytes() == b"refs/tags/archived")
    );
}

#[test]
fn ahead_behind_counts_the_ref_states_branches() {
    use crate::history::{AheadBehind, ahead_behind};
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::RefStates);
    let repo = RepoHandle::open(&dir).expect("the fixture opens");
    let peeled = repo.peeled_references(&()).expect("refs read");
    let tip = |wanted: &str| {
        peeled
            .iter()
            .find(|(ref_name, _)| ref_name.as_bytes() == wanted.as_bytes())
            .map_or_else(|| panic!("{wanted} exists"), |&(_, id)| id)
    };
    let count = |local: &str, upstream: &str| {
        ahead_behind(&repo, tip(local), tip(upstream)).expect("both sides read")
    };

    assert!(count("refs/heads/main", "refs/remotes/origin/main").in_sync());
    assert_eq!(
        count("refs/heads/ahead", "refs/remotes/origin/ahead"),
        AheadBehind {
            ahead: 1,
            behind: 0
        }
    );
    assert_eq!(
        count("refs/heads/behind", "refs/remotes/origin/behind"),
        AheadBehind {
            ahead: 0,
            behind: 1
        }
    );
    assert_eq!(
        count("refs/heads/ahead", "refs/remotes/origin/behind"),
        AheadBehind {
            ahead: 2,
            behind: 2
        }
    );
}

/// m1-the-graph/55: the one-pass reading is exactly `references` and the
/// expected tips.
fn snapshot_is_both_halves(repo: &RepoHandle, expected_tips: &[String], packed: bool) {
    let snapshot = repo.ref_snapshot(&()).expect("refs read");
    stats_with_nothing_known_are_exact(repo, packed);
    assert_eq!(
        snapshot.refs,
        repo.references().expect("refs read"),
        "snapshot refs, packed: {packed}"
    );
    let tips: Vec<String> = snapshot.tips.iter().map(Oid::to_string).collect();
    assert_eq!(tips, expected_tips, "snapshot tips, packed: {packed}");
    assert_eq!(
        snapshot.peeled,
        repo.peeled_references(&()).expect("refs read"),
        "snapshot peeled, packed: {packed}"
    );
}

/// What `commit_tips` and `peeled_references` return when many refs share a
/// commit and some refs name no commit at all — loose, then packed with
/// git's peeled lines. Pins the answer so reading refs faster cannot change it
/// (m1-the-graph/30d).
#[test]
fn refs_sharing_commits_or_naming_no_commit_peel_the_same_loose_or_packed() {
    let fixture = Fixture::with_two_commits();
    let head = fixture.rev_parse("HEAD");
    let parent = fixture.rev_parse("HEAD~1");
    let tree = fixture.rev_parse("HEAD^{tree}");
    std::fs::write(fixture.root().join("loose.txt"), "a blob\n").expect("file writes");
    let blob = fixture.git(&["hash-object", "-w", "loose.txt"]);
    let current = fixture.git(&["symbolic-ref", "HEAD"]);
    for (ref_name, target) in [
        ("refs/heads/twin-a", &head),
        ("refs/heads/twin-b", &head),
        ("refs/remotes/origin/twin-a", &head),
        ("refs/heads/older", &parent),
        ("refs/tags/on-tree", &tree),
        ("refs/tags/on-blob", &blob),
    ] {
        fixture.git(&["update-ref", ref_name, target]);
    }
    annotated_tags(&fixture, &parent, &tree);
    fixture.git(&["symbolic-ref", "refs/heads/alias", "refs/heads/older"]);

    let hex = |id: &Oid| id.to_string();
    let expected_peeled: Vec<(String, String)> = std::collections::BTreeMap::from([
        ("refs/heads/alias".to_owned(), parent.clone()),
        (current, head.clone()),
        ("refs/heads/older".to_owned(), parent.clone()),
        ("refs/heads/twin-a".to_owned(), head.clone()),
        ("refs/heads/twin-b".to_owned(), head.clone()),
        ("refs/remotes/origin/twin-a".to_owned(), head.clone()),
        ("refs/tags/outer".to_owned(), parent.clone()),
        ("refs/tags/release".to_owned(), parent.clone()),
    ])
    .into_iter()
    .collect();
    let mut expected_tips = vec![head.clone(), parent.clone()];
    expected_tips.sort();

    for packed in [false, true] {
        if packed {
            fixture.git(&["pack-refs", "--all"]);
        }
        let repo = RepoHandle::open(fixture.root()).expect("the fixture opens");
        let tips: Vec<String> = repo
            .commit_tips(&())
            .expect("tips read")
            .iter()
            .map(hex)
            .collect();
        assert_eq!(tips, expected_tips, "tips, packed: {packed}");
        snapshot_is_both_halves(&repo, &expected_tips, packed);
        assert_eq!(
            peeled_pairs(&repo, &()),
            expected_peeled,
            "peeled refs, packed: {packed}"
        );
        assert_same_with_a_commit_graph_table(
            &fixture,
            &repo,
            &expected_tips,
            &expected_peeled,
            packed,
        );
    }
}

/// `release` on `parent`, `tree-tag` on `tree`, and `outer`, a tag of the
/// `release` tag: git peels it through both (m1-the-graph/58b).
fn annotated_tags(fixture: &Fixture, parent: &str, tree: &str) {
    fixture.git(&["tag", "-a", "-m", "release", "release", parent]);
    fixture.git(&["tag", "-a", "-m", "a tree", "tree-tag", tree]);
    let release_tag = fixture.rev_parse("refs/tags/release");
    fixture.git(&["tag", "-a", "-m", "outer", "outer", &release_tag]);
}

/// m1-the-graph/58b: what one pass reads for the corpus above with nothing
/// known.
fn stats_with_nothing_known_are_exact(repo: &RepoHandle, packed: bool) {
    // Seven distinct objects: two commits, a tree, a blob, three tags.
    // With nothing known each is header-read once; the tree and the
    // blob are then read whole, and so are the tags unless packed-refs
    // already peeled them (m1-the-graph/58b).
    let stats = repo.ref_snapshot(&()).expect("refs read").stats;
    let expected = if packed {
        TipStats {
            refs: 11,
            known: 0,
            tags: 3,
            peeled_packed: 3,
            objects_read: 2,
            find_header: 4,
        }
    } else {
        TipStats {
            refs: 11,
            known: 0,
            tags: 3,
            peeled_packed: 0,
            objects_read: 5,
            find_header: 7,
        }
    };
    assert_eq!(
        stats, expected,
        "stats with nothing known, packed: {packed}"
    );
}

/// `peeled_references`, as `(ref name text, commit hex)` pairs.
fn peeled_pairs(repo: &RepoHandle, known: &dyn crate::repo::KnownCommits) -> Vec<(String, String)> {
    repo.peeled_references(known)
        .expect("refs read")
        .iter()
        .map(|(ref_name, id)| {
            (
                String::from_utf8_lossy(ref_name.as_bytes()).into_owned(),
                id.to_string(),
            )
        })
        .collect()
}

/// m1-the-graph/58: with a commit-graph the table already knows every tip
/// here (a tree/blob ref and a tag object are not commits, so the table has
/// no opinion and today's header-parse-then-peel path still decides them).
/// Tips and peeled refs must not change.
fn assert_same_with_a_commit_graph_table(
    fixture: &Fixture,
    repo: &RepoHandle,
    expected_tips: &[String],
    expected_peeled: &[(String, String)],
    packed: bool,
) {
    fixture.git(&["commit-graph", "write", "--reachable"]);
    let table = crate::history::CommitTable::open(repo).expect("the commit-graph opens");
    assert!(table.has_commit_graph(), "packed: {packed}");
    let tips_with_table: Vec<String> = repo
        .commit_tips(&table)
        .expect("tips read")
        .iter()
        .map(Oid::to_string)
        .collect();
    assert_eq!(
        tips_with_table, expected_tips,
        "tips with the table, packed: {packed}"
    );
    assert_eq!(
        peeled_pairs(repo, &table),
        expected_peeled,
        "peeled refs with the table, packed: {packed}"
    );
    // m1-the-graph/58b: the table answers both commits, so no header is
    // read. Loose, each tag object is read once, whole, and its target
    // judged by the table: objects read = tags + the tree and the blob.
    // Packed, every tag resolves from its peeled line and only the tree and
    // the blob are read — no tag object at all.
    let stats = repo.ref_snapshot(&table).expect("refs read").stats;
    assert_eq!(stats.find_header, 0, "packed: {packed}");
    assert_eq!(stats.known, 2, "packed: {packed}");
    assert_eq!(stats.tags, 3, "packed: {packed}");
    if packed {
        assert_eq!(stats.peeled_packed, stats.tags);
        assert_eq!(stats.objects_read, 2);
    } else {
        assert_eq!(stats.peeled_packed, 0);
        assert_eq!(stats.objects_read, stats.tags + 2);
    }
}

#[test]
fn worktree_name_is_main_in_the_main_worktree() {
    let fixture = Fixture::with_two_commits();
    let repo = RepoHandle::open(fixture.root()).expect("a fixture repository opens");
    assert_eq!(repo.worktree_name(), "main");
}

#[test]
fn worktree_name_is_the_linked_worktrees_own_name() {
    let fixture = Fixture::with_two_commits();
    let worktree = fixture.root().join("linked");
    fixture.git(&["worktree", "add", "-b", "side", "linked"]);
    let repo = RepoHandle::open(&worktree).expect("a linked worktree opens");
    assert_eq!(repo.worktree_name(), "linked");
}

#[test]
fn git_path_resolves_under_the_git_directory_in_the_main_worktree() {
    let fixture = Fixture::with_two_commits();
    let repo = RepoHandle::open(fixture.root()).expect("a fixture repository opens");
    let resolved = repo
        .git_path(Path::new("graft/undo-journal.jsonl"))
        .expect("git rev-parse --git-path runs");
    assert!(resolved.is_absolute(), "{}", resolved.display());
    assert_eq!(resolved.file_name().unwrap(), "undo-journal.jsonl");
    let graft_dir = resolved.parent().expect("a parent directory");
    assert_eq!(graft_dir.file_name().unwrap(), "graft");
    assert_same_path(
        graft_dir.parent().expect("the git directory"),
        repo.git_dir(),
    );
}

#[test]
fn git_path_resolves_under_the_linked_worktrees_own_directory() {
    let fixture = Fixture::with_two_commits();
    let worktree = fixture.root().join("linked");
    fixture.git(&["worktree", "add", "-b", "side", "linked"]);
    let repo = RepoHandle::open(&worktree).expect("a linked worktree opens");

    let resolved = repo
        .git_path(Path::new("graft/undo-journal.jsonl"))
        .expect("git rev-parse --git-path runs");

    assert!(resolved.is_absolute(), "{}", resolved.display());
    assert_eq!(resolved.file_name().unwrap(), "undo-journal.jsonl");
    let graft_dir = resolved.parent().expect("a parent directory");
    assert_eq!(graft_dir.file_name().unwrap(), "graft");
    // A linked worktree's own git dir, not the common one: `graft/` under it
    // is per-worktree, exactly like `worktree_name`'s own split.
    assert_same_path(
        graft_dir.parent().expect("the git directory"),
        repo.git_dir(),
    );
}

/// C1 (m3-07 review): a backup chain used to be seeded into the history
/// walk, drawing two synthetic commits above `HEAD` for every undoable
/// write. `ref_snapshot` is the repository as Graft draws it, so Graft's own
/// refs are not in it — while `references` and `reference`, which undo and
/// the journal use, still answer for them.
#[test]
fn grafts_own_refs_are_not_drawn_but_are_still_readable() {
    let fixture = Fixture::with_two_commits();
    let head = fixture.rev_parse("HEAD");
    let parent = fixture.rev_parse("HEAD~1");
    let backup = "refs/graft/backup/main/01M2XA1AWED99JDS7KERF0V5X1";
    // A backup chain's commits are reachable from nothing else: `parent` here
    // stands in for the before-commit that only the backup ref holds.
    fixture.git(&["update-ref", backup, &parent]);
    fixture.git(&["update-ref", "refs/heads/kept", &head]);
    let repo = RepoHandle::open(fixture.root()).expect("the repository opens");

    let snapshot = repo.ref_snapshot(&()).expect("the ref store reads");

    let drawn: Vec<String> = snapshot
        .refs
        .iter()
        .map(|entry| String::from_utf8_lossy(entry.name.as_bytes()).into_owned())
        .collect();
    assert!(
        !drawn.iter().any(|name| name.starts_with("refs/graft/")),
        "a graft ref reached the drawn snapshot: {drawn:?}"
    );
    assert!(
        drawn.iter().any(|name| name == "refs/heads/kept"),
        "the user's own refs are still there: {drawn:?}"
    );
    let peeled: Vec<&RefName> = snapshot.peeled.iter().map(|(name, _)| name).collect();
    assert!(
        !peeled.iter().any(|name| name.is_graft_internal()),
        "a graft ref reached the peeled pairs"
    );
    // The before-commit is reachable only from the backup ref, so if the
    // walk were still seeded from it, this tip would be here.
    let parent_id = Oid::from_hex(&parent).expect("a commit id");
    assert!(
        !snapshot.tips.contains(&parent_id),
        "a backup-only commit is still seeding the walk"
    );

    // Undo's own path is untouched: by exact name, and in the full listing.
    let by_name = repo
        .reference(&name(backup))
        .expect("the ref store reads")
        .expect("the backup ref is still there");
    assert_eq!(by_name, RefTarget::Object(parent_id));
    let every: Vec<String> = repo
        .references()
        .expect("the ref store reads")
        .iter()
        .map(|entry| String::from_utf8_lossy(entry.name.as_bytes()).into_owned())
        .collect();
    assert!(
        every.iter().any(|found| found == backup),
        "`references` is the one that still says everything: {every:?}"
    );
}
