//! `restart_gate` against real git (m6-conflicts/28): "can `d` start this
//! path over?", asked before any box, with the same rules the act runs —
//! and without writing anything, the journal file included.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use graft_git::status::UnmergedHow;
use graft_ops::apply::{ApplyError, RestartReadError, StageSource, restart_gate};
use graft_ops::facts::read_facts;
use graft_ops::journal::journal_path;
use graft_ops::operation::{
    FactsScope, Operation, Refusal, ResolveWholeParams, RestartParams, Side,
};

use crate::conflict_fixtures::{graft_merge_stops, graft_stage};
use crate::conflict_ops::{stopped_merge, unmerged_lines};
use crate::support::{NOW, Never, TempRepo, git_version, try_run};

type Gate = Result<Result<StageSource, Refusal>, RestartReadError>;

/// Graft's own merge of `feature` into `main`, stopped on `both.txt`
/// (`UU`, markers) and `gone.txt` (`UD`: main edited it, feature deleted it).
fn app_merge_with_a_whole_file_conflict() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("both.txt", b"base one\nbase two\n");
    repo.write("gone.txt", b"gone base alpha\ngone base beta\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("both.txt", b"ours one\nbase two\n");
    repo.write("gone.txt", b"gone OURS alpha\ngone base beta\n");
    repo.commit_all("ours");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("both.txt", b"theirs one\nbase two\n");
    repo.git(&["rm", "-q", "gone.txt"]);
    repo.commit_all("theirs");
    repo.git(&["switch", "-q", "main"]);
    graft_merge_stops(&repo);
    repo
}

/// `both.txt` resolved by hand and marked done the resolver's way.
fn mark_both_done(repo: &TempRepo) {
    repo.write("both.txt", b"ours one\nbase two\n");
    graft_stage(repo, "both.txt", NOW + 1);
}

/// `gone.txt` kept with `o`: the resolver's whole-file take.
fn keep_gone(repo: &TempRepo) {
    try_run(
        repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("gone.txt"),
            side: Side::Ours,
            how: UnmergedHow::DeletedByThem,
        }),
        NOW + 2,
    )
    .unwrap_or_else(|why| panic!("keeping gone.txt: {}", why.explain()));
}

/// Everything the gate must not touch: the journal's bytes, Graft's backup
/// refs, and git's view of the working tree.
#[derive(Debug, PartialEq, Eq)]
struct Untouched {
    journal: Option<Vec<u8>>,
    backups: String,
    status: String,
}

fn untouched(repo: &TempRepo) -> Untouched {
    let path = journal_path(&repo.open()).expect("the journal path resolves");
    Untouched {
        journal: std::fs::read(path).ok(),
        backups: repo.git(&["for-each-ref", "refs/graft/backup/"]),
        status: repo.git(&["status", "--porcelain=v1"]),
    }
}

/// Ask the gate with fresh facts, and prove it wrote nothing.
fn gate(repo: &TempRepo, path: &str) -> Gate {
    let before = untouched(repo);
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let answer = restart_gate(&handle, &facts, Path::new(path));
    assert_eq!(untouched(repo), before, "the gate wrote nothing for {path}");
    answer
}

#[test]
fn an_unmerged_marker_file_is_index() {
    let repo = app_merge_with_a_whole_file_conflict();
    assert!(matches!(
        gate(&repo, "both.txt"),
        Ok(Ok(StageSource::Index))
    ));
}

#[test]
fn an_unmerged_whole_file_is_not_marker_shape() {
    let repo = app_merge_with_a_whole_file_conflict();
    match gate(&repo, "gone.txt") {
        Ok(Err(Refusal::NotMarkerShape { path, how })) => {
            assert_eq!(path, PathBuf::from("gone.txt"));
            assert_eq!(how, UnmergedHow::DeletedByThem);
        }
        other => panic!("expected NotMarkerShape, got {other:?}"),
    }
}

#[test]
fn a_marker_file_done_in_an_app_merge_is_backup() {
    let repo = app_merge_with_a_whole_file_conflict();
    mark_both_done(&repo);
    assert!(matches!(
        gate(&repo, "both.txt"),
        Ok(Ok(StageSource::Backup { .. }))
    ));
}

#[test]
fn a_whole_file_done_in_an_app_merge_is_not_marker_shape() {
    let repo = app_merge_with_a_whole_file_conflict();
    keep_gone(&repo);
    match gate(&repo, "gone.txt") {
        Ok(Err(Refusal::NotMarkerShape { path, how })) => {
            assert_eq!(path, PathBuf::from("gone.txt"));
            assert_eq!(how, UnmergedHow::DeletedByThem);
        }
        other => panic!("expected NotMarkerShape, got {other:?}"),
    }
}

#[test]
fn a_done_file_in_a_shell_merge_is_no_stages_kept() {
    let repo = stopped_merge();
    repo.write("both.txt", b"both ours one\nboth ours two\n");
    graft_stage(&repo, "both.txt", NOW);
    match gate(&repo, "both.txt") {
        Ok(Err(Refusal::NoStagesKept { path })) => assert_eq!(path, PathBuf::from("both.txt")),
        other => panic!("expected NoStagesKept, got {other:?}"),
    }
}

/// A crash mid-append leaves a torn last line. `Journal::open` would cut it —
/// a write — so the gate reads around it and leaves the bytes as they were
/// (`gate` itself asserts the bytes).
#[test]
fn a_torn_journal_tail_is_read_not_cut() {
    let repo = app_merge_with_a_whole_file_conflict();
    mark_both_done(&repo);
    let path = journal_path(&repo.open()).expect("the journal path resolves");
    let mut bytes = std::fs::read(&path).expect("the journal exists");
    bytes.extend_from_slice(br#"{"pending":{"id":"torn"#);
    std::fs::write(&path, &bytes).expect("the torn tail is written");

    assert!(matches!(
        gate(&repo, "both.txt"),
        Ok(Ok(StageSource::Backup { .. }))
    ));
    assert_eq!(
        std::fs::read(&path).expect("still there"),
        bytes,
        "the torn tail is still on disk"
    );
}

/// Graft's merge with `both.txt` resolved and marked done.
fn done_marker_app() -> TempRepo {
    let repo = app_merge_with_a_whole_file_conflict();
    mark_both_done(&repo);
    repo
}

/// Graft's merge with `gone.txt` kept with `o`.
fn done_whole_app() -> TempRepo {
    let repo = app_merge_with_a_whole_file_conflict();
    keep_gone(&repo);
    repo
}

/// A shell merge with `both.txt` edited and staged through Graft.
fn done_shell() -> TempRepo {
    let repo = stopped_merge();
    repo.write("both.txt", b"both ours one\nboth ours two\n");
    graft_stage(&repo, "both.txt", NOW);
    repo
}

/// The offer and the act answer alike: `Ok(Err(r))` from the gate ⇔ the
/// restart refuses with `r`; `Ok(Ok(_))` ⇔ the restart runs and the path is
/// unmerged again with markers.
#[test]
fn the_gate_and_the_restart_agree() {
    type Setup = fn() -> TempRepo;
    let cases: [(&str, Setup, &str); 5] = [
        (
            "unmerged marker",
            app_merge_with_a_whole_file_conflict,
            "both.txt",
        ),
        (
            "unmerged whole",
            app_merge_with_a_whole_file_conflict,
            "gone.txt",
        ),
        ("done marker, app merge", done_marker_app, "both.txt"),
        ("done whole, app merge", done_whole_app, "gone.txt"),
        ("done, shell merge", done_shell, "both.txt"),
    ];
    for (name, setup, path) in cases {
        let repo = setup();
        let offered = gate(&repo, path).unwrap_or_else(|why| panic!("{name}: {}", why.explain()));
        let acted = try_run(
            &repo,
            &Operation::RestartConflict(RestartParams {
                path: PathBuf::from(path),
            }),
            NOW + 10,
        );
        match (offered, acted) {
            (Err(gated), Err(ApplyError::Refused(refused))) => {
                assert_eq!(gated, refused, "{name}: the same refusal");
            }
            (Ok(_), Ok(_)) => {
                let stages = unmerged_lines(&repo, path);
                assert!(
                    stages.lines().count() >= 2,
                    "{name}: {path} is unmerged again: {stages:?}"
                );
                let back = String::from_utf8_lossy(&repo.read(path)).into_owned();
                assert!(back.contains("<<<<<<<"), "{name}: markers are back: {back}");
            }
            (offered, acted) => panic!("{name}: the gate said {offered:?}, the act {acted:?}"),
        }
    }
}
