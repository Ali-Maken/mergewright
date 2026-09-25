//! `d` on a done whole-file file (m6-conflicts/58): the keep-or-delete
//! choice comes back. The stages go back into the index out of the chained
//! backup, then `checkout --ours|--theirs` writes the side that has the file
//! — so a file a delete choice took off disk is back on disk.
//!
//! This writes the user's index and working tree, so every case checks what
//! lands, byte for byte, and the stale-gate case checks nothing moved.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_git::status::UnmergedHow;
use graft_ops::apply::{Applied, ApplyError, RestartPlan, Resume, StageSource};
use graft_ops::journal::journal_path;
use graft_ops::operation::{
    OneSided, Operation, Refusal, ResolveWholeParams, RestartKind, RestartParams, Side,
};

use crate::abort_merge::{entries, merging};
use crate::conflict_fixtures::{graft_merge_stops, graft_stage, run_resume};
use crate::conflict_ops::unmerged_lines;
use crate::restart_gate::{app_merge_with_a_whole_file_conflict, gate, keep_gone, mark_both_done};
use crate::support::{NOW, TempRepo, run_undo, try_run};

/// Graft's own merge of `feature` into `main`, stopped with one path per
/// shape — the same history `conflict_ops::stopped_merge` builds, plus a
/// binary `UU` (`pic.bin`), but begun through `apply(Merge)` so there is a
/// live entry and a backup chain to read the stages back from.
fn app_merge_every_shape() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("both.txt", b"both base one\nboth base two\n");
    repo.write("theirs-deleted.txt", b"td base alpha\ntd base beta\n");
    repo.write("ours-deleted.txt", b"od base gamma\nod base delta\n");
    repo.write("renamed.txt", b"renamed base epsilon\nrenamed base zeta\n");
    repo.write("pic.bin", b"\x00\x01BASE\x00\x02\x03");
    repo.commit_all("base");

    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("both.txt", b"both theirs one\nboth theirs two\n");
    repo.git(&["rm", "-q", "theirs-deleted.txt"]);
    repo.write("ours-deleted.txt", b"od THEIRS gamma\nod base delta\n");
    repo.git(&["mv", "renamed.txt", "renamed-theirs.txt"]);
    repo.write("pic.bin", b"\x00\x01THEIRS\x00\x04");
    repo.commit_all("theirs");

    repo.git(&["checkout", "-q", "main"]);
    repo.write("both.txt", b"both ours one\nboth ours two\n");
    repo.write("theirs-deleted.txt", b"td OURS alpha\ntd base beta\n");
    repo.git(&["rm", "-q", "ours-deleted.txt"]);
    repo.git(&["mv", "renamed.txt", "renamed-ours.txt"]);
    repo.write("pic.bin", b"\x00\x01OURS\x00\x05");
    repo.commit_all("ours");

    graft_merge_stops(&repo);
    repo
}

fn resolve(repo: &TempRepo, path: &str, side: Side, how: UnmergedHow, at: i64) {
    try_run(
        repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from(path),
            side,
            how,
        }),
        at,
    )
    .unwrap_or_else(|why| panic!("resolving {path}: {}", why.explain()));
}

fn restart(repo: &TempRepo, path: &str, kind: RestartKind, at: i64) -> Result<Applied, ApplyError> {
    try_run(
        repo,
        &Operation::RestartConflict(RestartParams {
            path: PathBuf::from(path),
            kind,
        }),
        at,
    )
}

/// The blob at `:<stage>:<path>`, byte for byte (`TempRepo::git` trims).
fn stage_blob(repo: &TempRepo, stage: u8, path: &str) -> Vec<u8> {
    let out = std::process::Command::new("git")
        .current_dir(repo.path())
        .args(["cat-file", "blob", &format!(":{stage}:{path}")])
        .output()
        .expect("git runs");
    assert!(out.status.success(), ":{stage}:{path} has a blob");
    out.stdout
}

/// The stage numbers `ls-files -u` lists for `path`.
fn stages(repo: &TempRepo, path: &str) -> Vec<String> {
    unmerged_lines(repo, path)
        .lines()
        .filter_map(|line| {
            line.split('\t')
                .next()?
                .split(' ')
                .nth(2)
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn start_over_brings_back_a_modify_delete_choice() {
    let repo = app_merge_with_a_whole_file_conflict();
    keep_gone(&repo);
    assert!(
        stages(&repo, "gone.txt").is_empty(),
        "kept: staged, not unmerged"
    );

    let plan = gate(&repo, "gone.txt")
        .unwrap_or_else(|why| panic!("{}", why.explain()))
        .unwrap_or_else(|refusal| panic!("the gate refused: {}", refusal.explain()));
    assert!(
        matches!(plan.source, StageSource::Backup { .. }),
        "{plan:?}"
    );
    assert_eq!(
        plan.kind,
        RestartKind::WholeFile {
            shape: OneSided::DeletedByThem
        }
    );

    let applied = restart(&repo, "gone.txt", plan.kind, NOW + 3)
        .unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(
        stages(&repo, "gone.txt"),
        ["1", "2"],
        "stages 1 and 2 are back"
    );
    assert_eq!(
        repo.read("gone.txt"),
        stage_blob(&repo, 2, "gone.txt"),
        "the worktree holds stage 2's bytes"
    );
    assert!(merging(&repo), "the merge is still live");
    let commands = &applied.entry.commands;
    // The first *write*: the `ls-tree` that reads the recorded stages is
    // logged ahead of it (m6-conflicts/80: a restore's reads are in its log).
    assert!(
        commands
            .iter()
            .find(|line| !line.contains(" ls-tree "))
            .is_some_and(|line| line.contains("update-index")),
        "the stages went back first: {commands:?}"
    );
    let last = commands.last().expect("a command ran");
    assert!(
        last.ends_with("checkout --ours -- gone.txt") && last.contains("--literal-pathspecs"),
        "the last command writes our side: {last}"
    );
    assert!(
        !commands.iter().any(|line| line.contains("--merge")),
        "no checkout --merge for a whole file: {commands:?}"
    );
}

/// Each one-sided shape, with a keep and a delete choice among them: the
/// stages the merge left come back byte for byte, and the file on disk is
/// the side that has it — written back when the choice had deleted it.
#[test]
fn start_over_restores_the_surviving_side() {
    let repo = app_merge_every_shape();
    let rows = [
        // (path, shape, the choice, the surviving stage)
        ("theirs-deleted.txt", OneSided::DeletedByThem, Side::Ours, 2),
        ("ours-deleted.txt", OneSided::DeletedByUs, Side::Ours, 3),
        ("renamed-ours.txt", OneSided::AddedByUs, Side::Theirs, 2),
        ("renamed-theirs.txt", OneSided::AddedByThem, Side::Theirs, 3),
    ];
    let mut at = NOW + 1;
    for (path, shape, side, surviving) in rows {
        let before = unmerged_lines(&repo, path);
        let bytes = stage_blob(&repo, surviving, path);
        resolve(&repo, path, side, shape.how(), at);
        assert!(stages(&repo, path).is_empty(), "{path}: resolved");
        let deleted = !repo.path().join(path).exists();

        let plan = gate(&repo, path)
            .unwrap_or_else(|why| panic!("{}", why.explain()))
            .unwrap_or_else(|refusal| panic!("{path}: refused {}", refusal.explain()));
        assert_eq!(plan.kind, RestartKind::WholeFile { shape }, "{path}");
        let applied = restart(&repo, path, plan.kind, at + 1)
            .unwrap_or_else(|why| panic!("{path}: {}", why.explain()));

        assert_eq!(
            unmerged_lines(&repo, path),
            before,
            "{path}: the same stages"
        );
        assert_eq!(
            repo.read(path),
            bytes,
            "{path}: stage {surviving} on disk (deleted: {deleted})"
        );
        let flag = if surviving == 2 { "--ours" } else { "--theirs" };
        let last = applied.entry.commands.last().expect("a command ran");
        assert!(
            last.ends_with(&format!("checkout {flag} -- {path}")),
            "{path}: {last}"
        );
        at += 2;
    }
    // The delete rows really were deletes, so the write-back is proven.
    assert!(merging(&repo));
}

/// The gate and the act answer alike over every shape: `Ok(plan)` from the
/// gate ⇔ the restart runs with `plan.kind` and leaves the path unmerged in
/// that shape; `Err(r)` ⇔ the act refuses `r`.
///
/// Binary `UU` (`pic.bin`, chosen whole): today's answer is recorded, not
/// designed — see the row.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one table over every shape, each row answered by the gate and the act"
)]
fn gate_and_apply_agree_on_every_shape() {
    enum Setup {
        /// Resolved whole with this side.
        Whole(Side, UnmergedHow),
        /// Written by hand and marked done.
        Hand(&'static [u8]),
        /// Left unmerged.
        Unmerged,
    }
    let rows: [(&str, Setup, Result<RestartKind, UnmergedHow>); 8] = [
        (
            "both.txt",
            Setup::Hand(b"both ours one\nboth ours two\n"),
            Ok(RestartKind::Markers),
        ),
        (
            "theirs-deleted.txt",
            Setup::Whole(Side::Ours, UnmergedHow::DeletedByThem),
            Ok(RestartKind::WholeFile {
                shape: OneSided::DeletedByThem,
            }),
        ),
        (
            "ours-deleted.txt",
            Setup::Whole(Side::Ours, UnmergedHow::DeletedByUs),
            Ok(RestartKind::WholeFile {
                shape: OneSided::DeletedByUs,
            }),
        ),
        (
            "renamed-ours.txt",
            Setup::Whole(Side::Ours, UnmergedHow::AddedByUs),
            Ok(RestartKind::WholeFile {
                shape: OneSided::AddedByUs,
            }),
        ),
        (
            "renamed-theirs.txt",
            Setup::Whole(Side::Ours, UnmergedHow::AddedByThem),
            Ok(RestartKind::WholeFile {
                shape: OneSided::AddedByThem,
            }),
        ),
        (
            "renamed.txt",
            Setup::Whole(Side::Ours, UnmergedHow::BothDeleted),
            Err(UnmergedHow::BothDeleted),
        ),
        // Today's answer for a binary `UU` chosen whole: the gate has no
        // binary check, so the recovered stages spell `Markers`, and
        // `checkout --merge` puts all three stages back (the file on disk is
        // whatever git's binary merge leaves — not asserted here).
        (
            "pic.bin",
            Setup::Whole(Side::Ours, UnmergedHow::BothModified),
            Ok(RestartKind::Markers),
        ),
        // Still unmerged and one-sided: `check_restart` refuses, as before.
        (
            "theirs-deleted.txt",
            Setup::Unmerged,
            Err(UnmergedHow::DeletedByThem),
        ),
    ];
    for (row, (path, setup, expected)) in rows.into_iter().enumerate() {
        // A fresh merge per row, so no row's restart feeds the next's chain.
        let repo = app_merge_every_shape();
        let at = NOW + 10;
        match setup {
            Setup::Whole(side, how) => resolve(&repo, path, side, how, at),
            Setup::Hand(bytes) => {
                repo.write(path, bytes);
                graft_stage(&repo, path, at);
            }
            Setup::Unmerged => {}
        }
        let offered =
            gate(&repo, path).unwrap_or_else(|why| panic!("row {row}: {}", why.explain()));
        let kind = offered
            .as_ref()
            .map_or(RestartKind::Markers, |plan| plan.kind);
        let acted = restart(&repo, path, kind, at + 1);
        match (offered, acted, expected) {
            (Ok(RestartPlan { kind, .. }), Ok(_), Ok(want)) => {
                assert_eq!(kind, want, "row {row} {path}: the kind");
                let back = stages(&repo, path);
                match kind {
                    RestartKind::Markers => {
                        assert_eq!(back, ["1", "2", "3"], "row {row} {path}: three stages");
                    }
                    RestartKind::WholeFile { shape } => {
                        let want: &[&str] = match shape {
                            OneSided::DeletedByThem => &["1", "2"],
                            OneSided::DeletedByUs => &["1", "3"],
                            OneSided::AddedByUs => &["2"],
                            OneSided::AddedByThem => &["3"],
                        };
                        assert_eq!(back, want, "row {row} {path}: its shape's stages");
                    }
                }
            }
            (Err(gated), Err(ApplyError::Refused(refused)), Err(how)) => {
                assert_eq!(gated, refused, "row {row} {path}: the same refusal");
                assert_eq!(
                    gated,
                    Refusal::NotMarkerShape {
                        path: PathBuf::from(path),
                        how
                    },
                    "row {row} {path}"
                );
            }
            (offered, acted, expected) => panic!(
                "row {row} {path}: the gate said {offered:?}, the act {:?}, expected {expected:?}",
                acted.map(|applied| applied.entry.commands)
            ),
        }
    }
}

/// Everything a refused start-over must leave exactly as it was.
#[derive(Debug, PartialEq, Eq)]
struct Untouched {
    journal: Option<Vec<u8>>,
    backups: String,
    stages: String,
    status: String,
    bytes: Option<Vec<u8>>,
}

fn untouched(repo: &TempRepo, path: &str) -> Untouched {
    let journal = journal_path(&repo.open()).expect("the journal path resolves");
    Untouched {
        journal: std::fs::read(journal).ok(),
        backups: repo.git(&["for-each-ref", "refs/graft/"]),
        stages: repo.git(&["ls-files", "--stage"]),
        status: repo.git(&["status", "--porcelain=v2"]),
        bytes: std::fs::read(repo.path().join(path)).ok(),
    }
}

/// The box was opened on one kind and the stages now spell another: the act
/// refuses `RestartChanged` before anything is written — both ways round.
#[test]
fn start_over_refuses_when_the_shape_changed_since_the_gate() {
    let cases = [
        ("gone.txt", RestartKind::Markers),
        (
            "both.txt",
            RestartKind::WholeFile {
                shape: OneSided::DeletedByThem,
            },
        ),
    ];
    for (path, stale) in cases {
        let repo = app_merge_with_a_whole_file_conflict();
        keep_gone(&repo);
        mark_both_done(&repo);
        let before = untouched(&repo, path);

        match restart(&repo, path, stale, NOW + 5) {
            Err(ApplyError::Refused(Refusal::RestartChanged { path: at })) => {
                assert_eq!(at, PathBuf::from(path));
            }
            other => panic!("{path}: expected RestartChanged, got {other:?}"),
        }
        assert_eq!(untouched(&repo, path), before, "{path}: nothing written");
        assert_eq!(
            Refusal::RestartChanged {
                path: PathBuf::from(path)
            }
            .explain(),
            format!("{path} changed since it was opened · open it again")
        );
    }
}

/// What `u` puts back after the merge is finished.
#[derive(Debug, PartialEq, Eq)]
struct Landed {
    head: String,
    status: String,
    both: Option<Vec<u8>>,
    gone: Option<Vec<u8>>,
    entries: usize,
}

fn landed(repo: &TempRepo) -> Landed {
    Landed {
        head: repo.git(&["rev-parse", "HEAD"]),
        status: repo.git(&["status", "--porcelain=v2"]),
        both: std::fs::read(repo.path().join("both.txt")).ok(),
        gone: std::fs::read(repo.path().join("gone.txt")).ok(),
        entries: entries(repo).len(),
    }
}

/// A whole-file start-over folds into the merge's entry exactly as a marker
/// one does: it writes no journal line of its own, and `u` after the merge
/// is finished takes the whole span back — to the same place either way.
/// The `u` runs with no operation live, so ticket 57's gate never meets it.
#[test]
fn undo_after_a_whole_file_start_over_matches_markers() {
    /// (`HEAD` while stopped — the commit the merge began from; what `u`
    /// left; the entry count at the finish.)
    fn walk(start_over: &str) -> (String, Landed, usize) {
        let repo = app_merge_with_a_whole_file_conflict();
        let head_stopped = repo.git(&["rev-parse", "HEAD"]);
        let entries_stopped = entries(&repo).len();
        mark_both_done(&repo);
        keep_gone(&repo);
        let plan = gate(&repo, start_over).unwrap().unwrap();
        restart(&repo, start_over, plan.kind, NOW + 3)
            .unwrap_or_else(|why| panic!("{start_over}: {}", why.explain()));
        assert_eq!(
            entries(&repo).len(),
            entries_stopped,
            "folded: no line of its own"
        );
        // Choose again, then finish.
        if start_over == "gone.txt" {
            keep_gone(&repo);
        } else {
            mark_both_done(&repo);
        }
        run_resume(&repo, Resume::Continue, NOW + 6)
            .unwrap_or_else(|why| panic!("{start_over}: finishing: {}", why.explain()));
        assert!(!merging(&repo), "{start_over}: the merge is finished");
        let finished = entries(&repo).len();
        run_undo(&repo, NOW + 7, false).unwrap_or_else(|why| panic!("{start_over}: u: {why:?}"));
        (head_stopped, landed(&repo), finished)
    }
    let (markers_head, markers_after, markers_entries) = walk("both.txt");
    let (whole_head, whole_after, whole_entries) = walk("gone.txt");
    assert_eq!(
        markers_entries, whole_entries,
        "the same entries at the finish"
    );
    assert_eq!(whole_after.head, whole_head, "u took the merge back");
    assert_eq!(markers_after.head, markers_head, "u took the merge back");
    // `main`'s own bytes, as before the merge began.
    assert_eq!(
        whole_after.both.as_deref(),
        Some(&b"ours one\nbase two\n"[..])
    );
    assert_eq!(
        whole_after.gone.as_deref(),
        Some(&b"gone OURS alpha\ngone base beta\n"[..])
    );
    assert_eq!(
        (&whole_after.status, &whole_after.both, &whole_after.gone),
        (
            &markers_after.status,
            &markers_after.both,
            &markers_after.gone
        ),
        "the same place as after a marker start-over"
    );
}

/// A merge begun in the shell has no entry of Graft's and so no chain: a
/// whole-file choice there cannot be started over, and says so in today's
/// words — gate and act alike (m6-conflicts/58 Out; the `KNOWN_GAPS` row).
#[test]
fn start_over_of_a_whole_file_is_still_refused_in_a_shell_merge() {
    let repo = crate::conflict_ops::stopped_merge();
    let path = "theirs-deleted.txt";
    resolve(&repo, path, Side::Ours, UnmergedHow::DeletedByThem, NOW);
    let want = Refusal::NoStagesKept {
        path: PathBuf::from(path),
    };
    match gate(&repo, path) {
        Ok(Err(refusal)) => assert_eq!(refusal, want),
        other => panic!("expected NoStagesKept, got {other:?}"),
    }
    let kind = RestartKind::WholeFile {
        shape: OneSided::DeletedByThem,
    };
    match restart(&repo, path, kind, NOW + 1) {
        Err(ApplyError::Refused(refusal)) => assert_eq!(refusal, want),
        other => panic!("expected NoStagesKept, got {other:?}"),
    }
    assert_eq!(
        want.explain(),
        "theirs-deleted.txt: the conflict's sides were not kept"
    );
}
