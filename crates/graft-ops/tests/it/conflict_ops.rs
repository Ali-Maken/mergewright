//! The two conflict operations against real git (m6-conflicts/03).
//!
//! Every assertion is read back out of the repository with `git` itself: the
//! table in `operation/conflict.rs` claims exactly what `git checkout
//! --ours|--theirs` + `add`/`rm` would leave, so the test runs both and
//! compares.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryStatus, OperationKind};
use graft_git::status::UnmergedHow;
use graft_ops::apply::ApplyError;
use graft_ops::operation::{Operation, Refusal, ResolveWholeParams, RestartParams, Side};

use crate::support::{NOW, TempRepo, run, try_run};

/// A repository stopped on a merge with one path per `UnmergedHow`.
///
/// Rename/rename is what produces `DD` (and the `AU`/`UA` pair that comes
/// with it); every file's content differs from every other's so git's rename
/// detection pairs the ones it is meant to.
fn stopped_merge() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("both.txt", b"both base one\nboth base two\n");
    repo.write("theirs-deleted.txt", b"td base alpha\ntd base beta\n");
    repo.write("ours-deleted.txt", b"od base gamma\nod base delta\n");
    repo.write("renamed.txt", b"renamed base epsilon\nrenamed base zeta\n");
    repo.commit_all("base");

    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("both.txt", b"both theirs one\nboth theirs two\n");
    repo.git(&["rm", "-q", "theirs-deleted.txt"]);
    repo.write("ours-deleted.txt", b"od THEIRS gamma\nod base delta\n");
    repo.write("both-added.txt", b"both-added theirs body\n");
    repo.git(&["mv", "renamed.txt", "renamed-theirs.txt"]);
    repo.commit_all("theirs");

    repo.git(&["checkout", "-q", "main"]);
    repo.write("both.txt", b"both ours one\nboth ours two\n");
    repo.write("theirs-deleted.txt", b"td OURS alpha\ntd base beta\n");
    repo.git(&["rm", "-q", "ours-deleted.txt"]);
    repo.write("both-added.txt", b"both-added ours body\n");
    repo.git(&["mv", "renamed.txt", "renamed-ours.txt"]);
    repo.commit_all("ours");

    let merged = repo.try_git(&["merge", "feature"]);
    assert!(merged.is_err(), "the merge is meant to stop on conflicts");
    repo
}

/// `git ls-files -u -- <path>`, as lines.
fn unmerged_lines(repo: &TempRepo, path: &str) -> String {
    repo.git(&["--literal-pathspecs", "ls-files", "-u", "--", path])
}

/// `git ls-files -s -- <path>`, as lines.
fn index_line(repo: &TempRepo, path: &str) -> String {
    repo.git(&["--literal-pathspecs", "ls-files", "-s", "--", path])
}

/// The blob id of the working-tree file, or `None` when it is gone.
fn worktree_blob(repo: &TempRepo, path: &str) -> Option<String> {
    repo.exists(path)
        .then(|| repo.git(&["hash-object", "--", path]))
}

/// What the table says resolves a row, as the `git` a person would type.
#[derive(Clone, Copy)]
enum ByHand {
    Add,
    Remove,
    CheckoutOurs,
    CheckoutTheirs,
}

impl ByHand {
    /// The argument lists, in order, for one path.
    fn commands(self, path: &str) -> Vec<Vec<String>> {
        let one = |verb: Vec<&str>| {
            let mut args: Vec<String> = vec!["--literal-pathspecs".to_owned()];
            args.extend(verb.into_iter().map(str::to_owned));
            args.push("--".to_owned());
            args.push(path.to_owned());
            args
        };
        match self {
            Self::Add => vec![one(vec!["add"])],
            Self::Remove => vec![one(vec!["rm", "--quiet"])],
            Self::CheckoutOurs => vec![one(vec!["checkout", "--ours"]), one(vec!["add"])],
            Self::CheckoutTheirs => vec![one(vec!["checkout", "--theirs"]), one(vec!["add"])],
        }
    }
}

/// Every shape, and every side that shape has, with the raw commands the
/// table in `operation/conflict.rs` says are equivalent.
const ROWS: [(&str, UnmergedHow, Side, ByHand); 14] = [
    (
        "both.txt",
        UnmergedHow::BothModified,
        Side::Ours,
        ByHand::CheckoutOurs,
    ),
    (
        "both.txt",
        UnmergedHow::BothModified,
        Side::Theirs,
        ByHand::CheckoutTheirs,
    ),
    (
        "both-added.txt",
        UnmergedHow::BothAdded,
        Side::Ours,
        ByHand::CheckoutOurs,
    ),
    (
        "both-added.txt",
        UnmergedHow::BothAdded,
        Side::Theirs,
        ByHand::CheckoutTheirs,
    ),
    (
        "theirs-deleted.txt",
        UnmergedHow::DeletedByThem,
        Side::Ours,
        ByHand::Add,
    ),
    (
        "theirs-deleted.txt",
        UnmergedHow::DeletedByThem,
        Side::Theirs,
        ByHand::Remove,
    ),
    (
        "ours-deleted.txt",
        UnmergedHow::DeletedByUs,
        Side::Theirs,
        ByHand::Add,
    ),
    (
        "ours-deleted.txt",
        UnmergedHow::DeletedByUs,
        Side::Ours,
        ByHand::Remove,
    ),
    (
        "renamed-ours.txt",
        UnmergedHow::AddedByUs,
        Side::Ours,
        ByHand::Add,
    ),
    (
        "renamed-ours.txt",
        UnmergedHow::AddedByUs,
        Side::Theirs,
        ByHand::Remove,
    ),
    (
        "renamed-theirs.txt",
        UnmergedHow::AddedByThem,
        Side::Theirs,
        ByHand::Add,
    ),
    (
        "renamed-theirs.txt",
        UnmergedHow::AddedByThem,
        Side::Ours,
        ByHand::Remove,
    ),
    (
        "renamed.txt",
        UnmergedHow::BothDeleted,
        Side::Ours,
        ByHand::Remove,
    ),
    (
        "renamed.txt",
        UnmergedHow::BothDeleted,
        Side::Theirs,
        ByHand::Remove,
    ),
];

/// The corpus really does hold every shape the table has a row for — a
/// mis-built fixture would otherwise make every row pass vacuously.
#[test]
fn the_stopped_merge_has_one_path_per_shape() {
    let repo = stopped_merge();
    let handle = repo.open();
    let status = graft_git::status::read(&handle, &()).expect("the status reads");
    let mut shapes: Vec<(String, UnmergedHow)> = status
        .entries
        .iter()
        .filter_map(|entry| {
            entry
                .unmerged_how
                .map(|how| (entry.path.display().to_string(), how))
        })
        .collect();
    shapes.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        shapes,
        vec![
            ("both-added.txt".to_owned(), UnmergedHow::BothAdded),
            ("both.txt".to_owned(), UnmergedHow::BothModified),
            ("ours-deleted.txt".to_owned(), UnmergedHow::DeletedByUs),
            ("renamed-ours.txt".to_owned(), UnmergedHow::AddedByUs),
            ("renamed-theirs.txt".to_owned(), UnmergedHow::AddedByThem),
            ("renamed.txt".to_owned(), UnmergedHow::BothDeleted),
            ("theirs-deleted.txt".to_owned(), UnmergedHow::DeletedByThem),
        ]
    );
}

/// Every `(how, side)`, run twice: once through `ResolveWholeFile` and once
/// through git's own commands in a second repository. The index line, the
/// working-tree bytes and `ls-files -u` have to agree, and the path has to
/// leave `ls-files -u` either way.
#[test]
fn whole_file_shapes_resolve_like_git() {
    for (path, how, side, by_hand) in ROWS {
        let ours = stopped_merge();
        let applied = run(
            &ours,
            &Operation::ResolveWholeFile(ResolveWholeParams {
                path: PathBuf::from(path),
                side,
                how,
            }),
            NOW,
        );
        assert_eq!(applied.status, EntryStatus::Committed, "{path} {side:?}");

        let theirs = stopped_merge();
        for command in by_hand.commands(path) {
            let args: Vec<&str> = command.iter().map(String::as_str).collect();
            theirs.git(&args);
        }

        assert_eq!(
            unmerged_lines(&ours, path),
            String::new(),
            "{path} {side:?} is still unmerged"
        );
        assert_eq!(
            index_line(&ours, path),
            index_line(&theirs, path),
            "{path} {side:?} index"
        );
        assert_eq!(
            worktree_blob(&ours, path),
            worktree_blob(&theirs, path),
            "{path} {side:?} working tree"
        );
    }
}

/// The `DeletedByUs` + `Theirs` row the spec marked "confirm": git leaves
/// theirs' bytes in the working tree, so `add` alone is the resolution and
/// the staged blob is stage 3's.
#[test]
fn deleted_by_us_taking_theirs_stages_the_bytes_git_left() {
    let repo = stopped_merge();
    let stage_three = repo
        .git(&[
            "--literal-pathspecs",
            "ls-files",
            "-u",
            "--",
            "ours-deleted.txt",
        ])
        .lines()
        .find(|line| line.split('\t').next().is_some_and(|f| f.ends_with(" 3")))
        .map(|line| {
            line.split_whitespace()
                .nth(1)
                .expect("the blob id is there")
                .to_owned()
        })
        .expect("stage 3 exists for a DU path");
    assert_eq!(
        worktree_blob(&repo, "ours-deleted.txt").as_deref(),
        Some(stage_three.as_str()),
        "git already left theirs' bytes in the working tree"
    );

    run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("ours-deleted.txt"),
            side: Side::Theirs,
            how: UnmergedHow::DeletedByUs,
        }),
        NOW,
    );
    assert!(
        index_line(&repo, "ours-deleted.txt").contains(&stage_three),
        "the staged blob is stage 3's"
    );
    assert_eq!(unmerged_lines(&repo, "ours-deleted.txt"), String::new());
}

/// Inside a live merge the resolution is a step in that operation's span, so
/// it writes no journal line of its own (m5 hard choice 2).
#[test]
fn whole_file_folds_into_live_entry() {
    let repo = stopped_merge();
    let before = journal_ids(&repo);
    run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("theirs-deleted.txt"),
            side: Side::Ours,
            how: UnmergedHow::DeletedByThem,
        }),
        NOW,
    );
    assert_eq!(
        journal_ids(&repo),
        before,
        "a step inside a live merge writes no line"
    );
    assert_eq!(unmerged_lines(&repo, "theirs-deleted.txt"), String::new());
}

/// With nothing live — a conflicted `stash pop` leaves an unmerged index and
/// no `MERGE_HEAD` — the resolution is its own entry, kind `ResolveConflict`.
#[test]
fn whole_file_is_its_own_entry_when_nothing_is_live() {
    let repo = conflicted_stash_pop();
    let applied = run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("both.txt"),
            side: Side::Ours,
            how: UnmergedHow::BothModified,
        }),
        NOW,
    );
    assert_eq!(applied.kind, OperationKind::ResolveConflict);
    assert_eq!(applied.status, EntryStatus::Committed);
    assert_eq!(applied.description, "resolved both.txt (ours)");
    assert!(
        journal_ids(&repo).contains(&applied.id.to_string()),
        "the entry is in the journal file"
    );
    assert_eq!(unmerged_lines(&repo, "both.txt"), String::new());
}

/// What `u` actually does after that entry, recorded rather than assumed:
/// the index was unmerged when the backup was taken and an unmerged index
/// has no tree, so the entry's `index_before` is `None` and the generic
/// inverse has nothing to restore the index from.
#[test]
fn undo_after_a_whole_file_resolution_with_nothing_live() {
    let repo = conflicted_stash_pop();
    let applied = run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("both.txt"),
            side: Side::Ours,
            how: UnmergedHow::BothModified,
        }),
        NOW,
    );
    assert_eq!(applied.index_before, None, "an unmerged index has no tree");
    let undone = crate::support::run_undo(&repo, NOW + 1, false);
    let why = undone.err().map(|error| error.explain());
    assert_eq!(
        why.as_deref(),
        Some("no backup was taken for that operation, so it cannot be undone"),
        "the unmerged index cannot be put back from a tree (`KNOWN_GAPS` 229)"
    );
}

/// `checkout --merge` brings the markers and the three stages back.
#[test]
fn restart_brings_the_conflict_back() {
    let repo = stopped_merge();
    // A take, written to disk the way `conflict::apply_take` writes one.
    repo.write("both.txt", b"both ours one\nboth ours two\n");
    assert!(
        !String::from_utf8_lossy(&repo.read("both.txt")).contains("<<<<<<<"),
        "the markers are gone before the restart"
    );

    run(
        &repo,
        &Operation::RestartConflict(RestartParams {
            path: PathBuf::from("both.txt"),
        }),
        NOW,
    );

    let back = String::from_utf8_lossy(&repo.read("both.txt")).into_owned();
    assert!(back.contains("<<<<<<<"), "the markers are back: {back}");
    assert!(back.contains("both theirs one"), "theirs is back: {back}");
    assert_eq!(
        unmerged_lines(&repo, "both.txt").lines().count(),
        3,
        "all three stages are listed again"
    );
}

/// Both operations refuse a path the status does not call unmerged, before
/// any `git` process runs.
#[test]
fn conflict_operations_refuse_a_path_that_is_not_conflicted() {
    let repo = stopped_merge();
    let clean = PathBuf::from("renamed-ours.txt");
    // Resolve it first, so it is a real path that is no longer unmerged.
    run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: clean.clone(),
            side: Side::Ours,
            how: UnmergedHow::AddedByUs,
        }),
        NOW,
    );

    for op in [
        Operation::ResolveWholeFile(ResolveWholeParams {
            path: clean.clone(),
            side: Side::Ours,
            how: UnmergedHow::AddedByUs,
        }),
        Operation::RestartConflict(RestartParams {
            path: clean.clone(),
        }),
    ] {
        match try_run(&repo, &op, NOW + 1) {
            Err(ApplyError::Refused(Refusal::NotUnmerged { path })) => {
                assert_eq!(path, clean);
            }
            other => panic!("expected NotUnmerged, got {other:?}"),
        }
    }
}

/// A `ResolveWholeFile` whose `how` is not the shape the status reads now is
/// refused: the side the user chose may not exist in the shape it became.
#[test]
fn a_stale_shape_is_refused() {
    let repo = stopped_merge();
    let op = Operation::ResolveWholeFile(ResolveWholeParams {
        path: PathBuf::from("both.txt"),
        side: Side::Ours,
        how: UnmergedHow::DeletedByThem,
    });
    match try_run(&repo, &op, NOW) {
        Err(ApplyError::Refused(Refusal::NotUnmerged { .. })) => {}
        other => panic!("expected NotUnmerged, got {other:?}"),
    }
}

/// A binary `UU` from the real corpus: the one shape whose `how` says it has
/// markers but which still goes to `ResolveWholeFile`, because git wrote no
/// markers into a binary file for a take to reach.
#[test]
fn a_binary_conflict_resolves_whole() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::StoppedMerge);
    let handle = graft_git::repo::RepoHandle::open(scratch.path()).expect("the fixture opens");
    let status = graft_git::status::read(&handle, &()).expect("the status reads");
    assert_eq!(
        status
            .entry(Path::new("pic.bin"))
            .and_then(|entry| entry.unmerged_how),
        Some(UnmergedHow::BothModified)
    );

    let mut journal = crate::support::journal_for(&handle);
    let never = crate::support::Never;
    let mut ctx = crate::support::context(&handle, &mut journal, NOW, &never);
    let applied = graft_ops::apply::apply(
        &mut ctx,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("pic.bin"),
            side: Side::Theirs,
            how: UnmergedHow::BothModified,
        }),
    )
    .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.description, "resolved pic.bin (theirs)");

    let after = graft_git::status::read(&handle, &()).expect("the status reads again");
    assert_eq!(
        after
            .entry(Path::new("pic.bin"))
            .and_then(|entry| entry.unmerged_how),
        None,
        "pic.bin is resolved"
    );
}

/// A repository whose index is unmerged with no operation in progress: what
/// a `stash pop` over a changed file leaves (`KNOWN_GAPS` 229).
fn conflicted_stash_pop() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("both.txt", b"base one\nbase two\n");
    repo.commit_all("base");
    repo.write("both.txt", b"stashed one\nbase two\n");
    repo.git(&["stash", "push", "-q"]);
    repo.write("both.txt", b"worktree one\nbase two\n");
    repo.commit_all("moved on");
    let popped = repo.try_git(&["stash", "pop"]);
    assert!(popped.is_err(), "the pop is meant to conflict");
    assert!(
        !unmerged_lines(&repo, "both.txt").is_empty(),
        "the index is unmerged"
    );
    let handle = repo.open();
    assert_eq!(
        graft_git::state::in_progress(&handle).expect("the state reads"),
        None,
        "a conflicted stash pop leaves no operation in progress"
    );
    repo
}

/// The ids in the journal file, in order.
fn journal_ids(repo: &TempRepo) -> Vec<String> {
    let handle = repo.open();
    let journal = crate::support::journal_for(&handle);
    journal
        .entries()
        .iter()
        .map(|entry| entry.id.to_string())
        .collect()
}
