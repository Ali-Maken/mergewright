//! `jobs::conflict` unit tests: the open route, the card glimpse, the base
//! refusal words.

use graft_git::conflict::{BaseMismatch, Diff3Error};
use graft_ops::conflict::BaseError;

use super::{CARD_BYTES, CARD_LINES, Opening, base_refusal, glimpse, open};

/// Ticket 25: the job picks the route from the status it reads itself.
/// The same path is an unmerged document before `git add` and a done
/// file after it.
#[test]
fn open_routes_on_the_status_it_reads() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::StoppedMerge);
    let store = crate::repos::RepoStore::new(scratch.path().to_path_buf(), None);
    let path = std::path::Path::new("both.txt");
    match open(&store, path, &()).expect("it opens") {
        Opening::Unmerged(opened) => assert!(opened.file.is_some(), "a parsed document"),
        Opening::Done(done, _) => panic!("still unmerged, got {done:?}"),
    }
    std::fs::write(scratch.path().join(path), b"settled\n").expect("the file is written");
    let added = std::process::Command::new("git")
        .current_dir(scratch.path())
        .args(["add", "both.txt"])
        .status()
        .expect("git runs");
    assert!(added.success());
    match open(&store, path, &()).expect("it opens") {
        Opening::Done(done, _) => assert!(
            matches!(
                *done,
                graft_ops::conflict::DoneFile::Text {
                    unstaged: false,
                    ..
                }
            ),
            "{done:?}"
        ),
        Opening::Unmerged(opened) => panic!("git add made it done, got {opened:?}"),
    }
}

/// A file of `lines` numbered lines, and its size.
fn numbered(lines: usize) -> (Vec<u8>, u64) {
    let bytes: Vec<u8> = (0..lines)
        .flat_map(|n| format!("line {n}\n").into_bytes())
        .collect();
    let size = u64::try_from(bytes.len()).expect("fits");
    (bytes, size)
}

#[test]
fn a_short_file_is_read_whole() {
    let (bytes, size) = numbered(3);
    let seen = glimpse(bytes.as_slice(), size, false).expect("reads");
    assert_eq!(
        &*seen.head,
        [b"line 0".to_vec(), b"line 1".to_vec(), b"line 2".to_vec()]
    );
    assert!(!seen.cut);
    assert_eq!(seen.size, size);
}

/// Many short lines: the line cap is what stops it.
#[test]
fn many_lines_stop_at_the_line_cap() {
    let (bytes, size) = numbered(CARD_LINES * 3);
    let seen = glimpse(bytes.as_slice(), size, false).expect("reads");
    assert_eq!(seen.head.len(), CARD_LINES);
    assert!(seen.cut);
}

/// A few long lines: the byte cap is what stops it, and the fragment it
/// cuts through is not shown as though it were a line.
#[test]
fn long_lines_stop_at_the_byte_cap() {
    let line = vec![b'x'; 10_000];
    let mut bytes = Vec::new();
    for _ in 0..20 {
        bytes.extend_from_slice(&line);
        bytes.push(b'\n');
    }
    let size = u64::try_from(bytes.len()).expect("fits");
    let seen = glimpse(bytes.as_slice(), size, false).expect("reads");
    let held: usize = seen.head.iter().map(Vec::len).sum();
    assert!(u64::try_from(held).expect("fits") <= CARD_BYTES);
    assert!(
        seen.head.iter().all(|kept| kept.len() == 10_000),
        "no fragment"
    );
    assert!(seen.cut);
}

/// A binary file is not read at all: its size is the whole card.
#[test]
fn a_binary_file_reads_nothing() {
    let (bytes, size) = numbered(10);
    let seen = glimpse(bytes.as_slice(), size, true).expect("reads");
    assert!(seen.head.is_empty());
    assert_eq!(seen.size, size);
}

#[test]
fn no_base_and_mismatch_are_facts_not_problems() {
    assert!(!base_refusal(&BaseError::NoBase).1);
    assert!(!base_refusal(&BaseError::Mismatch(BaseMismatch { block: 0 })).1);
}

#[test]
fn a_diff3_failure_is_a_problem() {
    let error = BaseError::Diff3(Diff3Error::Io(std::io::Error::other("disk full")));
    let (words, problem) = base_refusal(&error);
    assert!(problem, "git itself failed");
    assert!(words.contains("disk full"), "{words}");
}

/// A merge stopped on a modify/delete: `main` changed `gone.txt`, `side`
/// deleted it, so it stays on disk under a whole-file card
/// (`DeletedByThem`).
fn modify_delete() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .current_dir(dir.path())
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success() || args[0] == "merge",
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(dir.path().join("gone.txt"), b"one\n").expect("write");
    git(&["add", "gone.txt"]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["checkout", "-q", "-b", "side"]);
    git(&["rm", "-q", "gone.txt"]);
    git(&["commit", "-q", "-m", "delete"]);
    git(&["checkout", "-q", "main"]);
    std::fs::write(dir.path().join("gone.txt"), b"two\n").expect("write");
    git(&["commit", "-q", "-am", "change"]);
    git(&["merge", "-q", "side"]);
    dir
}

fn whole_file_card(dir: &std::path::Path) -> std::sync::Arc<super::OpenedFile> {
    let store = crate::repos::RepoStore::new(dir.to_path_buf(), None);
    match open(&store, std::path::Path::new("gone.txt"), &()).expect("it opens") {
        Opening::Unmerged(opened) => opened,
        Opening::Done(done, _) => panic!("still unmerged, got {done:?}"),
    }
}

/// `KNOWN_GAPS` :572: a file that is there but cannot be read says so in the
/// card's words, rather than drawing as an empty file.
#[test]
fn a_permission_error_shows_in_the_card_not_empty() {
    use std::os::unix::fs::PermissionsExt;
    let dir = modify_delete();
    let file = dir.path().join("gone.txt");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    if std::fs::File::open(&file).is_ok() {
        // Root reads a 000 file; the case this test is about cannot be made.
        return;
    }
    let opened = whole_file_card(dir.path());
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("chmod back");
    assert_eq!(opened.on_disk, None);
    let words = opened
        .unreadable
        .as_deref()
        .unwrap_or_else(|| panic!("no words: {opened:?}"));
    assert!(words.starts_with("can't read gone.txt: "), "{words}");
}

/// The card's glimpse is read from the workdir, not the process's cwd,
/// and a file that is simply not there is no error.
#[test]
fn a_relative_path_reads_from_the_workdir() {
    let dir = modify_delete();
    let opened = whole_file_card(dir.path());
    assert_eq!(opened.unreadable, None);
    let on_disk = opened.on_disk.as_ref().expect("the file is read");
    assert_eq!(&*on_disk.head, [b"two".to_vec()]);
    std::fs::remove_file(dir.path().join("gone.txt")).expect("rm");
    let opened = whole_file_card(dir.path());
    assert_eq!(
        (opened.on_disk.as_ref(), opened.unreadable.as_ref()),
        (None, None)
    );
}
