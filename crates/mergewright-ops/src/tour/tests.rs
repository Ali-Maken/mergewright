//! The practice repo: its path guard, its shape, its removal and its ticks.
//! Real temp folders and real git for the first three; synthetic journal
//! entries for the ticks.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a scratch folder or a git read that fails leaves nothing to test"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mergewright_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind};
use mergewright_core::oid::Oid;
use mergewright_core::on_disk::{DIR, Generation};
use mergewright_git::history_reads::{Forecast, forecast_merge};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation};
use mergewright_git::version::system_git;

use super::build::finish_or_clean;
use super::{
    PRACTICE_MARKER, PracticeAt, PracticeRepo, TourError, TourTask, build_practice_repo, inside,
    practice_prefix, remove_practice_repo, tasks_done, tour_progress,
};
use crate::journal::tests::{committed, reversal};
use crate::journal::{render_outcome, render_pending};

/// The app's clock for tests that don't care about dates.
const NOW: PracticeAt = PracticeAt::Now {
    now_unix_secs: 1_800_000_000,
};

// ---- helpers ----

fn names(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .expect("readable")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("utf-8")
        })
        .collect()
}

/// A temp root T with `T/under`, the folder the build is pointed at.
fn scratch() -> (tempfile::TempDir, PathBuf) {
    let top = tempfile::tempdir().expect("a scratch folder");
    let under = top.path().join("under");
    std::fs::create_dir(&under).expect("under");
    (top, under)
}

/// `git <words>` in `dir`: exit code and stdout.
fn read(dir: &Path, words: &[&str]) -> (Option<i32>, String) {
    let raw = GitInvocation::in_folder(dir)
        .args(words.iter().map(|word| GitArg::Word((*word).to_owned())))
        .run_raw(&())
        .expect("git runs");
    (
        raw.code,
        String::from_utf8(raw.output.stdout).expect("utf-8"),
    )
}

fn ok(dir: &Path, words: &[&str]) -> String {
    let (code, out) = read(dir, words);
    assert_eq!(code, Some(0), "git {words:?}");
    out
}

/// A practice repo value pointing anywhere, for the refusals.
fn claimed(root: &Path, under: &Path) -> PracticeRepo {
    PracticeRepo {
        root: root.to_path_buf(),
        work: root.join("work"),
        remote: root.join("remote.git"),
        under: under.to_path_buf(),
    }
}

/// A folder under `under` that looks like a practice root (prefix, marker).
fn fake_practice(under: &Path, name: &str) -> PathBuf {
    let root = under.canonicalize().unwrap().join(name);
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(PRACTICE_MARKER), "x").unwrap();
    std::fs::write(root.join("keep.txt"), "his file").unwrap();
    root
}

// ---- the guard ----

#[test]
fn inside_joins_a_plain_relative_path() {
    let root = Path::new("/tmp/mergewright-practice-1-2");
    assert_eq!(
        inside(root, Path::new("work/menu.txt")).unwrap(),
        root.join("work/menu.txt")
    );
}

#[test]
fn inside_refuses_an_absolute_path() {
    let answer = inside(Path::new("/tmp/p"), Path::new("/etc/passwd"));
    assert!(
        matches!(answer, Err(TourError::Outside { .. })),
        "{answer:?}"
    );
}

#[test]
fn inside_refuses_any_dot_dot() {
    for rel in ["..", "../x", "work/../../x", "work/.."] {
        let answer = inside(Path::new("/tmp/p"), Path::new(rel));
        assert!(
            matches!(answer, Err(TourError::Outside { .. })),
            "{rel}: {answer:?}"
        );
    }
}

#[test]
fn inside_refuses_empty_and_dot() {
    for rel in ["", "."] {
        let answer = inside(Path::new("/tmp/p"), Path::new(rel));
        assert!(
            matches!(answer, Err(TourError::Outside { .. })),
            "{rel:?}: {answer:?}"
        );
    }
}

// ---- the build ----

#[test]
fn building_writes_nothing_outside_its_root() {
    let (top, under) = scratch();
    let before_top = names(top.path());
    assert_eq!(names(&under), BTreeSet::new());

    let repo = build_practice_repo(&under, NOW).expect("builds");

    assert_eq!(
        names(top.path()),
        before_top,
        "the temp root's parent changed"
    );
    let made = names(&under);
    assert_eq!(made.len(), 1, "{made:?}");
    let name = made.iter().next().unwrap();
    assert!(
        name.starts_with(&practice_prefix(Generation::CURRENT)),
        "{name}"
    );
    assert_eq!(repo.root(), under.canonicalize().unwrap().join(name));
    assert_eq!(repo.work(), repo.root().join("work"));
    assert_eq!(
        names(repo.root()),
        BTreeSet::from([PRACTICE_MARKER, "hooks", "remote.git", "work"].map(str::to_owned))
    );
    assert_eq!(names(&repo.root().join("hooks")), BTreeSet::new());
}

#[test]
fn the_built_repo_has_the_tours_shape() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, NOW).expect("builds");
    let work = repo.work();

    let branches = ok(work, &["branch", "--format=%(refname:short)"]);
    assert_eq!(
        branches.lines().collect::<BTreeSet<_>>(),
        BTreeSet::from(["clash", "feature", "main"])
    );
    assert_eq!(
        ok(work, &["symbolic-ref", "--short", "HEAD"]).trim(),
        "main"
    );
    assert_eq!(ok(work, &["rev-list", "--count", "main"]).trim(), "5");
    assert_eq!(
        ok(work, &["rev-parse", "--abbrev-ref", "main@{upstream}"]).trim(),
        "origin/main"
    );
    assert_eq!(
        ok(work, &["rev-list", "--count", "origin/main..main"]).trim(),
        "1",
        "main is not one ahead of origin/main"
    );
    assert_eq!(
        ok(work, &["rev-list", "--count", "main..origin/main"]).trim(),
        "0"
    );
    assert_eq!(
        ok(work, &["rev-list", "--count", "main..feature"]).trim(),
        "1"
    );
    assert_eq!(
        ok(work, &["rev-list", "--count", "main..clash"]).trim(),
        "1"
    );

    let diff = ok(work, &["diff", "--", "notes.txt"]);
    assert_eq!(
        diff.lines().filter(|line| line.starts_with("@@")).count(),
        2,
        "{diff}"
    );
    let status = ok(work, &["status", "--porcelain"]);
    assert_eq!(
        status.lines().collect::<BTreeSet<_>>(),
        BTreeSet::from([" M notes.txt", "?? scratch.txt"]),
        "{status}"
    );

    let (clean, _) = read(work, &["merge-tree", "--write-tree", "main", "feature"]);
    assert_eq!(clean, Some(0), "feature does not merge clean");
    let (stops, out) = read(
        work,
        &["merge-tree", "--write-tree", "--name-only", "main", "clash"],
    );
    assert_eq!(stops, Some(1), "clash does not conflict");
    let conflicted: Vec<&str> = out.lines().skip(1).take_while(|l| !l.is_empty()).collect();
    assert_eq!(conflicted, ["menu.txt"], "{out}");
}

#[test]
fn the_built_repos_own_config_keeps_his_hooks_and_signing_out() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, NOW).expect("builds");
    let hooks = repo.root().join("hooks");
    let hooks_text = hooks.to_str().unwrap();
    let work = repo.work();
    for (key, value) in [
        ("user.name", "Practice"),
        ("user.email", "practice@example.invalid"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("core.hooksPath", hooks_text),
    ] {
        assert_eq!(ok(work, &["config", "--local", key]).trim(), value, "{key}");
    }
    let remote = repo.root().join("remote.git");
    assert_eq!(
        ok(&remote, &["config", "--local", "core.hooksPath"]).trim(),
        hooks_text
    );
    assert_eq!(
        ok(work, &["rev-parse", "--git-path", "hooks"]).trim(),
        hooks_text,
        "git does not look for hooks in the empty folder"
    );
    assert_eq!(
        ok(work, &["remote", "get-url", "origin"]).trim(),
        remote.to_str().unwrap()
    );
}

#[test]
fn building_under_a_missing_folder_fails_and_makes_nothing() {
    let (top, under) = scratch();
    let missing = under.join("nope");
    let answer = build_practice_repo(&missing, NOW);
    assert!(matches!(answer, Err(TourError::TempDir(_))), "{answer:?}");
    assert!(!missing.exists());
    assert_eq!(names(top.path()), BTreeSet::from(["under".to_owned()]));
}

#[cfg(unix)]
#[test]
fn building_under_a_read_only_folder_fails_and_makes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let (_top, under) = scratch();
    std::fs::set_permissions(&under, std::fs::Permissions::from_mode(0o555)).unwrap();
    let answer = build_practice_repo(&under, NOW);
    std::fs::set_permissions(&under, std::fs::Permissions::from_mode(0o755)).unwrap();
    let error = answer.expect_err("a read-only temp folder");
    assert!(matches!(error, TourError::TempDir(_)), "{error:?}");
    assert!(
        error
            .explain()
            .starts_with("couldn't make the practice folder"),
        "{}",
        error.explain()
    );
    assert_eq!(names(&under), BTreeSet::new());
}

#[test]
fn a_build_that_fails_part_way_removes_its_folder() {
    let (_top, under) = scratch();
    let root = fake_practice(&under, &current_name("9-9"));
    let repo = claimed(&root, &under.canonicalize().unwrap());

    let answer = finish_or_clean(repo, |_| {
        Err(TourError::Git {
            words: "git said no".to_owned(),
        })
    });

    let error = answer.expect_err("the fill failed");
    assert_eq!(error.explain(), "git said no");
    assert!(!root.exists(), "the half-built folder was left");
}

// ---- removal ----

#[test]
fn removing_a_built_repo_removes_its_folder_and_nothing_else() {
    let (top, under) = scratch();
    std::fs::write(under.join("neighbour.txt"), "another program's").unwrap();
    let repo = build_practice_repo(&under, NOW).expect("builds");

    remove_practice_repo(&repo).expect("removes");

    assert!(!repo.root().exists());
    assert_eq!(names(&under), BTreeSet::from(["neighbour.txt".to_owned()]));
    assert_eq!(names(top.path()), BTreeSet::from(["under".to_owned()]));
}

/// A practice root name for this build: the current prefix, then `rest`.
fn current_name(rest: &str) -> String {
    format!("{}{rest}", practice_prefix(Generation::CURRENT))
}

/// The folder the tour header shows is named from the names table (51).
#[test]
fn a_new_practice_folder_takes_the_current_name() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, NOW).expect("builds");
    let name = repo
        .root()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let current = format!("{}-practice-", Generation::CURRENT.dir());
    assert!(name.starts_with(&current), "{name}");
    assert_eq!(practice_prefix(Generation::CURRENT), current);
}

/// The removal guard accepts a root named with an earlier generation's prefix.
#[test]
fn removal_accepts_every_earlier_generations_folder() {
    let (_top, under) = scratch();
    for (n, old) in Generation::CURRENT.older().enumerate() {
        let root = fake_practice(&under, &format!("{}-practice-{n}-{n}", old.dir()));
        let repo = claimed(&root, &under.canonicalize().unwrap());
        remove_practice_repo(&repo).expect("an old practice folder removes");
        assert!(!root.exists(), "{}", old.dir());
    }
}

#[test]
fn removal_refuses_a_folder_without_the_marker() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, NOW).expect("builds");
    std::fs::remove_file(repo.root().join(PRACTICE_MARKER)).unwrap();

    let answer = remove_practice_repo(&repo);

    assert!(
        matches!(answer, Err(TourError::NotPractice(_))),
        "{answer:?}"
    );
    assert!(
        repo.work().join("menu.txt").exists(),
        "something was removed"
    );
}

#[cfg(unix)]
#[test]
fn removal_refuses_a_marker_that_is_a_link() {
    let (top, under) = scratch();
    let root = fake_practice(&under, &current_name("1-1"));
    std::fs::remove_file(root.join(PRACTICE_MARKER)).unwrap();
    let target = top.path().join("elsewhere-marker");
    std::fs::write(&target, "x").unwrap();
    std::os::unix::fs::symlink(&target, root.join(PRACTICE_MARKER)).unwrap();

    let answer = remove_practice_repo(&claimed(&root, &under));

    assert!(
        matches!(answer, Err(TourError::NotPractice(_))),
        "{answer:?}"
    );
    assert!(root.join("keep.txt").exists());
}

#[cfg(unix)]
#[test]
fn removal_refuses_a_root_that_is_a_link() {
    let (top, under) = scratch();
    // His folder, made to look like a practice root, outside `under`…
    let his = top.path().join("his-project");
    std::fs::create_dir(&his).unwrap();
    std::fs::write(his.join(PRACTICE_MARKER), "x").unwrap();
    std::fs::write(his.join("keep.txt"), "his file").unwrap();
    // …and a correctly named link to it inside `under`.
    let link = under.canonicalize().unwrap().join(current_name("2-2"));
    std::os::unix::fs::symlink(&his, &link).unwrap();

    let answer = remove_practice_repo(&claimed(&link, &under));

    assert!(
        matches!(answer, Err(TourError::NotPractice(_))),
        "{answer:?}"
    );
    assert!(his.join("keep.txt").exists(), "his folder was emptied");
    assert!(link.exists());
}

#[test]
fn removal_refuses_a_root_outside_the_folder_it_was_made_in() {
    let (top, under) = scratch();
    let other = top.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let root = fake_practice(&other, &current_name("3-3"));

    let answer = remove_practice_repo(&claimed(&root, &under));

    assert!(
        matches!(answer, Err(TourError::Outside { .. })),
        "{answer:?}"
    );
    assert!(root.join("keep.txt").exists());
}

#[test]
fn removal_refuses_a_root_with_the_wrong_name() {
    let (_top, under) = scratch();
    let root = fake_practice(&under, "my-project");

    let answer = remove_practice_repo(&claimed(&root, &under));

    assert!(
        matches!(answer, Err(TourError::NotPractice(_))),
        "{answer:?}"
    );
    assert!(root.join("keep.txt").exists());
}

#[test]
fn removal_refuses_a_recorded_root_that_is_not_canonical() {
    let (_top, under) = scratch();
    let root = fake_practice(&under, &current_name("4-4"));
    let dotted = root.join("..").join(root.file_name().unwrap());

    let answer = remove_practice_repo(&claimed(&dotted, &under));

    assert!(
        matches!(answer, Err(TourError::NotPractice(_))),
        "{answer:?}"
    );
    assert!(root.join("keep.txt").exists());
}

// ---- ticks ----

fn with_status(mut entry: JournalEntry, status: EntryStatus) -> JournalEntry {
    entry.status = status;
    entry
}

fn aborted(entry: JournalEntry) -> JournalEntry {
    with_status(
        entry,
        EntryStatus::Aborted {
            reason: "no".to_owned(),
        },
    )
}

fn done(entries: &[JournalEntry], task: TourTask) -> bool {
    let index = TourTask::ALL.iter().position(|t| *t == task).unwrap();
    tasks_done(entries, &BTreeSet::new())[index]
}

#[test]
fn nothing_is_done_in_an_empty_journal() {
    assert_eq!(tasks_done(&[], &BTreeSet::new()), [false; 8]);
}

#[test]
fn each_simple_task_ticks_on_its_committed_kind() {
    for (kind, task) in [
        (OperationKind::Stage, TourTask::StageHunk),
        (OperationKind::Commit, TourTask::Commit),
        (OperationKind::BranchCreate, TourTask::Branch),
        (OperationKind::Merge, TourTask::MergeWithPreview),
        (OperationKind::Push, TourTask::Push),
    ] {
        let entries = [committed(1, kind, 1, 2)];
        let flags = tasks_done(&entries, &BTreeSet::new());
        let expected = TourTask::ALL.map(|t| t == task);
        assert_eq!(flags, expected, "{kind:?}");
    }
}

#[test]
fn aborted_and_pending_entries_never_tick() {
    let kinds = [
        OperationKind::Stage,
        OperationKind::Commit,
        OperationKind::BranchCreate,
        OperationKind::Merge,
        OperationKind::Push,
    ];
    for kind in kinds {
        let entries = [aborted(committed(1, kind, 1, 2))];
        assert_eq!(
            tasks_done(&entries, &BTreeSet::new()),
            [false; 8],
            "aborted {kind:?}"
        );
        let entries = [with_status(committed(1, kind, 1, 2), EntryStatus::Pending)];
        assert_eq!(
            tasks_done(&entries, &BTreeSet::new()),
            [false; 8],
            "pending {kind:?}"
        );
    }
}

#[test]
fn an_amend_is_not_the_commit_task() {
    assert!(!done(
        &[committed(1, OperationKind::Amend, 1, 2)],
        TourTask::Commit
    ));
}

#[test]
fn undo_ticks_only_when_it_reverses_a_commit() {
    let commit = committed(1, OperationKind::Commit, 1, 2);
    let undo = reversal(2, OperationKind::Undo, &commit);
    assert!(done(&[commit.clone(), undo.clone()], TourTask::Undo));

    let stage = committed(1, OperationKind::Stage, 1, 2);
    let undo_stage = reversal(2, OperationKind::Undo, &stage);
    assert!(
        !done(&[stage, undo_stage], TourTask::Undo),
        "undo of a stage"
    );

    assert!(
        !done(&[commit.clone(), aborted(undo)], TourTask::Undo),
        "an aborted undo"
    );

    // Undo → redo → undo: the root is still the commit.
    let first = reversal(2, OperationKind::Undo, &commit);
    let redo = reversal(3, OperationKind::Redo, &first);
    let again = reversal(4, OperationKind::Undo, &redo);
    assert!(done(&[commit, aborted(first), redo, again], TourTask::Undo));
}

#[test]
fn stash_and_back_needs_a_push_then_a_later_apply_or_pop() {
    let push = committed(1, OperationKind::StashPush, 1, 1);
    for back_kind in [OperationKind::StashApply, OperationKind::StashPop] {
        let back = committed(2, back_kind, 1, 1);
        assert!(
            done(&[push.clone(), back.clone()], TourTask::StashAndBack),
            "{back_kind:?}"
        );
        assert!(
            !done(&[push.clone(), aborted(back)], TourTask::StashAndBack),
            "aborted {back_kind:?}"
        );
    }
    let early_pop = committed(1, OperationKind::StashPop, 1, 1);
    let late_push = committed(2, OperationKind::StashPush, 1, 1);
    assert!(
        !done(&[early_pop, late_push], TourTask::StashAndBack),
        "a pop before the push"
    );
    assert!(!done(&[push], TourTask::StashAndBack), "a push alone");
}

/// "Later" is journal order, not id order. Under a pinned clock
/// (`MWR_NOW`) every entry of a run shares one millisecond, so two ids
/// differ only in their random half and sort either way round. The pty
/// test `task_4_ticks_by_setting_aside_and_bringing_back` failed about
/// half its runs on exactly this.
#[test]
fn stash_and_back_reads_journal_order_when_ids_share_a_millisecond() {
    let mut push = committed(1, OperationKind::StashPush, 1, 1);
    push.id = EntryId::new(1, [0xff; 10]);
    let mut pop = committed(1, OperationKind::StashPop, 1, 1);
    pop.id = EntryId::new(1, [0x00; 10]);
    assert!(pop.id < push.id, "the pop's id sorts first");
    assert!(
        done(&[push.clone(), pop.clone()], TourTask::StashAndBack),
        "push written first, then pop"
    );
    assert!(
        !done(&[pop, push], TourTask::StashAndBack),
        "pop written first, then push"
    );
}

fn only(merge: &JournalEntry) -> BTreeSet<EntryId> {
    BTreeSet::from([merge.id.clone()])
}

#[test]
fn resolve_ticks_on_a_committed_merge_git_could_not_make() {
    let merge = committed(1, OperationKind::Merge, 1, 2);
    let flags = tasks_done(std::slice::from_ref(&merge), &only(&merge));
    assert_eq!(
        flags,
        TourTask::ALL.map(|t| matches!(t, TourTask::MergeWithPreview | TourTask::ResolveConflict))
    );
}

#[test]
fn a_conflicted_merge_not_committed_never_ticks_7() {
    let merge = committed(1, OperationKind::Merge, 1, 2);
    for entry in [
        with_status(merge.clone(), EntryStatus::Pending),
        aborted(merge.clone()),
    ] {
        let flags = tasks_done(std::slice::from_ref(&entry), &only(&merge));
        assert_eq!(flags, [false; 8], "{:?}", entry.status);
    }
}

#[test]
fn a_clean_merge_ticks_6_only() {
    let merge = committed(1, OperationKind::Merge, 1, 2);
    let other = committed(2, OperationKind::Merge, 2, 3);
    let flags = tasks_done(std::slice::from_ref(&merge), &only(&other));
    assert_eq!(
        flags,
        TourTask::ALL.map(|t| t == TourTask::MergeWithPreview)
    );
}

#[test]
fn a_resolve_conflict_entry_alone_ticks_nothing() {
    let resolve = committed(1, OperationKind::ResolveConflict, 1, 1);
    let flags = tasks_done(std::slice::from_ref(&resolve), &only(&resolve));
    assert_eq!(flags, [false; 8]);
}

// ---- dates and the folder name ----

const AT: PracticeAt = PracticeAt::Fixed {
    first_unix_secs: 1_700_000_000,
};

fn tip(work: &Path, rev: &str) -> String {
    ok(work, &["rev-parse", rev]).trim().to_owned()
}

#[test]
fn a_fixed_instant_builds_the_same_commits() {
    let (_one, first) = scratch();
    let (_two, second) = scratch();
    let a = build_practice_repo(&first, AT).expect("builds");
    let b = build_practice_repo(&second, AT).expect("builds");
    for rev in ["main", "feature", "clash", "origin/main"] {
        assert_eq!(tip(a.work(), rev), tip(b.work(), rev), "{rev}");
    }
}

/// `(author, committer, subject)` of every built commit, oldest first.
fn dated(work: &Path) -> Vec<(i64, i64, String)> {
    let mut rows: Vec<(i64, i64, String)> = ok(work, &["log", "--all", "--format=%at %ct %s"])
        .lines()
        .map(|line| {
            let mut parts = line.splitn(3, ' ');
            let author = parts.next().unwrap().parse().unwrap();
            let committer = parts.next().unwrap().parse().unwrap();
            (author, committer, parts.next().unwrap().to_owned())
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn every_built_commit_has_its_own_minute() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, AT).expect("builds");
    let rows = dated(repo.work());
    let subjects: Vec<&str> = rows.iter().map(|row| row.2.as_str()).collect();
    assert_eq!(
        subjects,
        [
            "Start the menu",
            "Add a side dish",
            "Add opening hours",
            "Ice cream for dessert",
            "Pie for dessert",
            "Name the chef",
            "Add today's specials",
        ],
        "build order"
    );
    for (k, (author, committer, subject)) in rows.iter().enumerate() {
        let want = 1_700_000_000 + 60 * i64::try_from(k).unwrap();
        assert_eq!((*author, *committer), (want, want), "{subject}");
    }
}

#[test]
fn now_dates_end_a_minute_before_now() {
    let (_top, under) = scratch();
    let now = 1_800_000_000;
    let repo = build_practice_repo(&under, PracticeAt::Now { now_unix_secs: now }).expect("builds");
    let rows = dated(repo.work());
    assert_eq!(rows.last().unwrap().0, now - 60);
    assert_eq!(rows.last().unwrap().1, now - 60);
    assert_eq!(rows.first().unwrap().0, now - 60 * 7);
    let name = repo.root().file_name().unwrap().to_str().unwrap();
    let rest = name
        .strip_prefix(&practice_prefix(Generation::CURRENT))
        .unwrap();
    assert!(
        rest.starts_with(&format!("{}-", std::process::id())),
        "{name}"
    );
}

#[test]
fn a_fixed_instant_names_the_folder_and_never_reuses_one() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, AT).expect("builds");
    assert_eq!(
        repo.root().file_name().unwrap(),
        "mergewright-practice-1700000000"
    );
    let main_before = tip(repo.work(), "main");

    let again = build_practice_repo(&under, AT);

    assert!(matches!(again, Err(TourError::TempDir(_))), "{again:?}");
    assert_eq!(
        names(&under),
        BTreeSet::from(["mergewright-practice-1700000000".to_owned()])
    );
    assert!(
        repo.root().join(PRACTICE_MARKER).is_file(),
        "nothing removed"
    );
    assert_eq!(tip(repo.work(), "main"), main_before);
}

// ---- task 7 against git's own answer ----

fn oid_of(work: &Path, rev: &str) -> Oid {
    Oid::from_hex(&tip(work, rev)).unwrap()
}

/// A Committed `Merge` entry whose recorded result is `result` on `main`.
fn merge_entry(at: i64, result: Oid) -> JournalEntry {
    let mut entry = committed(at, OperationKind::Merge, 1, 2);
    entry.refs_after = BTreeMap::from([("refs/heads/main".to_owned(), Some(result))]);
    entry.head_after = Some(HeadRef::Branch("refs/heads/main".to_owned()));
    entry
}

fn write_journal(work: &Path, entries: &[JournalEntry]) {
    let journal = work.join(".git").join(DIR).join("undo-journal.jsonl");
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    let mut bytes = Vec::new();
    for entry in entries {
        bytes.extend(render_pending(entry).unwrap());
        bytes.extend(render_outcome(entry).unwrap());
    }
    std::fs::write(&journal, bytes).unwrap();
}

#[test]
fn the_built_clash_forecasts_conflicts_and_feature_clean() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, AT).expect("builds");
    let handle = RepoHandle::open(repo.work()).unwrap();
    let git = system_git().unwrap().version();
    let main = oid_of(repo.work(), "main");
    let clash = forecast_merge(&handle, &git, None, main, oid_of(repo.work(), "clash"), &());
    assert_eq!(
        clash.unwrap(),
        Forecast::Conflicts {
            paths: vec![PathBuf::from("menu.txt")]
        }
    );
    let feature = forecast_merge(
        &handle,
        &git,
        None,
        main,
        oid_of(repo.work(), "feature"),
        &(),
    );
    assert_eq!(feature.unwrap(), Forecast::Clean);
}

#[test]
fn tour_progress_ticks_7_for_the_clash_merge_and_not_for_feature() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, AT).expect("builds");
    let work = repo.work();
    ok(work, &["merge", "--no-edit", "feature"]);
    let feature_result = oid_of(work, "main");
    let (code, _out) = read(work, &["merge", "--no-edit", "clash"]);
    assert_eq!(code, Some(1), "the clash merge stops");
    std::fs::write(work.join("menu.txt"), "dessert: pie and ice cream\n").unwrap();
    ok(work, &["add", "menu.txt"]);
    ok(work, &["commit", "--no-edit"]);
    let clash_result = oid_of(work, "main");

    write_journal(work, &[merge_entry(1, feature_result)]);
    let flags = tour_progress(work).expect("reads");
    assert_eq!(
        flags,
        TourTask::ALL.map(|t| t == TourTask::MergeWithPreview),
        "feature"
    );

    write_journal(
        work,
        &[merge_entry(1, feature_result), merge_entry(2, clash_result)],
    );
    let flags = tour_progress(work).expect("reads");
    assert_eq!(
        flags,
        TourTask::ALL.map(|t| matches!(t, TourTask::MergeWithPreview | TourTask::ResolveConflict)),
        "clash"
    );
}

#[test]
fn a_fast_forward_result_is_not_a_resolved_conflict() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, AT).expect("builds");
    // `main`'s tip has one parent: what a fast-forward would record.
    write_journal(repo.work(), &[merge_entry(1, oid_of(repo.work(), "main"))]);
    let flags = tour_progress(repo.work()).expect("reads");
    assert_eq!(
        flags,
        TourTask::ALL.map(|t| t == TourTask::MergeWithPreview)
    );
}

// ---- progress from the practice repo's own journal ----

#[test]
fn a_fresh_practice_repo_has_nothing_done() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, NOW).expect("builds");
    assert_eq!(tour_progress(repo.work()).expect("reads"), [false; 8]);
}

#[test]
fn progress_reads_the_practice_repos_journal() {
    let (_top, under) = scratch();
    let repo = build_practice_repo(&under, NOW).expect("builds");
    let journal = repo.work().join(format!(".git/{DIR}/undo-journal.jsonl"));
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    let push = committed(1, OperationKind::Push, 1, 2);
    let mut bytes = render_pending(&push).unwrap();
    bytes.extend(render_outcome(&push).unwrap());
    std::fs::write(&journal, bytes).unwrap();

    let flags = tour_progress(repo.work()).expect("reads");

    assert_eq!(flags, TourTask::ALL.map(|t| t == TourTask::Push));
}

#[test]
fn progress_of_a_folder_that_is_not_a_repo_is_an_error() {
    let (_top, under) = scratch();
    let answer = tour_progress(&under);
    assert!(matches!(answer, Err(TourError::Git { .. })), "{answer:?}");
}
