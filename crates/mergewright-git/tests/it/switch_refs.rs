//! `switch_refs`: local + remote branches with tip, subject, author, date and
//! upstream, and `@{-1}` (m7-power-tools/10a), against throwaway repos.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_fixtures::Scratch;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::RunError;
use mergewright_git::switch_refs::{RefListing, switch_refs};

fn git<S: AsRef<OsStr>>(dir: &Path, args: &[S]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Ann Author")
        .env("GIT_AUTHOR_EMAIL", "ann@example.invalid")
        .env("GIT_COMMITTER_NAME", "Cy Committer")
        .env("GIT_COMMITTER_EMAIL", "cy@example.invalid")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap_or_else(|why| panic!("running git: {why}"));
    assert!(
        output.status.success(),
        "git {:?}: {}",
        args.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// A fresh repository with one commit on `main`, in its own scratch dir.
fn fresh() -> (Scratch, PathBuf) {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let dir = scratch.path().join("switch-refs");
    std::fs::create_dir(&dir).unwrap_or_else(|why| panic!("mkdir: {why}"));
    git(&dir, &["init", "-q", "-b", "main"]);
    git(
        &dir,
        &["commit", "-q", "--allow-empty", "-m", "first subject"],
    );
    (scratch, dir)
}

fn open(dir: &Path) -> RepoHandle {
    RepoHandle::open(dir).unwrap_or_else(|why| panic!("opening: {why}"))
}

fn name(full: &str) -> RefName {
    RefName::new(full).unwrap_or_else(|why| panic!("{full}: {why}"))
}

fn tip(dir: &Path, rev: &str) -> Oid {
    let hex = String::from_utf8(git(dir, &["rev-parse", rev]))
        .unwrap_or_else(|why| panic!("rev-parse output: {why}"));
    Oid::from_hex(hex.trim()).unwrap_or_else(|why| panic!("{hex:?}: {why}"))
}

fn row<'a>(refs: &'a [RefListing], full: &str) -> &'a RefListing {
    refs.iter()
        .find(|r| r.name.as_bytes() == full.as_bytes())
        .unwrap_or_else(|| panic!("{full} not listed"))
}

/// `dir` with `origin/main` and `origin/HEAD` as remote-tracking refs.
fn with_remote(dir: &Path) {
    let head = tip(dir, "HEAD").to_string();
    git(dir, &["update-ref", "refs/remotes/origin/main", &head]);
    git(
        dir,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
}

#[test]
fn lists_local_and_remote_with_tip_subject_author_date() {
    let (_s, dir) = fresh();
    git(
        &dir,
        &["commit", "-q", "--allow-empty", "-m", "second subject"],
    );
    with_remote(&dir);
    let refs = switch_refs(&open(&dir), &()).expect("reads").refs;
    assert_eq!(refs.len(), 2, "{refs:?}");
    let main = row(&refs, "refs/heads/main");
    assert_eq!(main.tip, tip(&dir, "main"));
    assert_eq!(main.subject, "second subject");
    assert_eq!(main.author, "Ann Author");
    assert_eq!(main.when, 1_700_000_000);
    let remote = row(&refs, "refs/remotes/origin/main");
    assert_eq!(remote.tip, tip(&dir, "main"));
    assert_eq!(remote.upstream, None);
}

#[test]
fn leaves_out_the_remote_head_symref() {
    let (_s, dir) = fresh();
    with_remote(&dir);
    let refs = switch_refs(&open(&dir), &()).expect("reads").refs;
    assert!(
        refs.iter()
            .all(|r| r.name.as_bytes() != b"refs/remotes/origin/HEAD"),
        "{refs:?}"
    );
}

#[test]
fn upstream_is_read_for_a_local_branch() {
    let (_s, dir) = fresh();
    with_remote(&dir);
    git(&dir, &["config", "remote.origin.url", "/nowhere"]);
    git(
        &dir,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
    );
    git(&dir, &["branch", "-q", "--track", "x", "origin/main"]);
    git(&dir, &["branch", "-q", "a"]);
    git(&dir, &["branch", "-q", "--track", "y", "a"]);
    let refs = switch_refs(&open(&dir), &()).expect("reads").refs;
    assert_eq!(
        row(&refs, "refs/heads/x").upstream,
        Some(name("refs/remotes/origin/main"))
    );
    assert_eq!(
        row(&refs, "refs/heads/y").upstream,
        Some(name("refs/heads/a"))
    );
    assert_eq!(row(&refs, "refs/heads/main").upstream, None);
}

#[test]
fn previous_is_the_branch_before_the_last_switch() {
    let (_s, dir) = fresh();
    git(&dir, &["switch", "-q", "-c", "b"]);
    git(&dir, &["switch", "-q", "-c", "c"]);
    let previous = switch_refs(&open(&dir), &()).expect("reads").previous;
    assert_eq!(previous, Some(name("refs/heads/b")));
}

#[test]
fn no_previous_checkout_is_none_not_an_error() {
    let (_s, dir) = fresh();
    let previous = switch_refs(&open(&dir), &()).expect("reads").previous;
    assert_eq!(previous, None);
}

#[test]
fn a_detached_previous_is_none() {
    let (_s, dir) = fresh();
    git(&dir, &["switch", "-q", "--detach", "main"]);
    git(&dir, &["switch", "-q", "main"]);
    let previous = switch_refs(&open(&dir), &()).expect("reads").previous;
    assert_eq!(previous, None);
}

#[test]
fn a_deleted_previous_is_none() {
    let (_s, dir) = fresh();
    git(&dir, &["switch", "-q", "-c", "b"]);
    git(&dir, &["switch", "-q", "-c", "c"]);
    git(&dir, &["branch", "-q", "-D", "b"]);
    let previous = switch_refs(&open(&dir), &()).expect("reads").previous;
    assert_eq!(previous, None);
}

/// Review 18/19/27 Minor 1: with a tag named like the previous branch,
/// `rev-parse --symbolic-full-name @{-1}` prints nothing and exits 0 (seen on
/// 2.38.5 and 2.54), which read as "detached". The oracle is git's own
/// `switch -`.
#[test]
fn a_previous_branch_sharing_a_tags_name_is_still_the_branch() {
    let (_s, dir) = fresh();
    git(&dir, &["switch", "-q", "-c", "b"]);
    git(&dir, &["tag", "b"]);
    git(&dir, &["switch", "-q", "-c", "c"]);
    let previous = switch_refs(&open(&dir), &()).expect("reads").previous;
    git(&dir, &["switch", "-q", "-"]);
    let landed = git(&dir, &["symbolic-ref", "HEAD"]);
    assert_eq!(landed, b"refs/heads/b\n", "git's own `switch -`");
    assert_eq!(previous, Some(name("refs/heads/b")));
}

/// A deleted previous branch with a tag of its name left behind: `@{-1}`
/// resolves to the tag, which is not a branch to switch back to.
#[test]
fn a_deleted_previous_with_a_same_named_tag_is_none() {
    let (_s, dir) = fresh();
    git(&dir, &["switch", "-q", "-c", "t"]);
    git(&dir, &["switch", "-q", "main"]);
    git(&dir, &["branch", "-q", "-D", "t"]);
    git(&dir, &["tag", "t"]);
    let previous = switch_refs(&open(&dir), &()).expect("reads").previous;
    assert_eq!(previous, None);
}

#[test]
fn a_branch_name_with_non_utf8_bytes_round_trips() {
    let (_s, dir) = fresh();
    let raw: &[u8] = b"refs/heads/caf\xe9";
    // Written into `packed-refs`, not as a loose file: macOS filesystems
    // refuse a non-UTF-8 file name, and git reads packed refs the same.
    let head = tip(&dir, "HEAD").to_string();
    let mut packed = b"# pack-refs with: peeled fully-peeled sorted \n".to_vec();
    packed.extend_from_slice(head.as_bytes());
    packed.push(b' ');
    packed.extend_from_slice(raw);
    packed.push(b'\n');
    std::fs::write(dir.join(".git/packed-refs"), packed).expect("write packed-refs");
    let listed = git(&dir, &["for-each-ref", "--format=%(refname)", "refs/heads"]);
    assert!(listed.windows(raw.len()).any(|w| w == raw), "git lists it");
    let refs = switch_refs(&open(&dir), &()).expect("reads").refs;
    assert!(refs.iter().any(|r| r.name.as_bytes() == raw), "{refs:?}");
}

#[test]
fn switch_refs_honours_stop() {
    let (_s, dir) = fresh();
    let stop = AtomicBool::new(true);
    let result = switch_refs(&open(&dir), &stop);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
}
