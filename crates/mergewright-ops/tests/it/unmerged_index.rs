//! The unmerged index as a recorded state (m6-conflicts/08).
//!
//! Every assertion goes through `git` or the filesystem: an index is judged
//! by `git ls-files --stage`, never by a value a function handed back
//! (PROMPT.md §7.2 item 4).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};

use mergewright_core::journal::{EntryId, IndexState, OperationKind, Touches};
use mergewright_core::oid::Oid;
use mergewright_git::status::UnmergedHow;
use mergewright_ops::apply::ApplyError;
use mergewright_ops::operation::{Operation, ResolveWholeParams, Side};
use mergewright_ops::snapshot;
use mergewright_ops::undo::{self, IndexPaths, InversePlan, Step};

use crate::support::{
    NOW, Never, TempRepo, context, journal_for, run, run_redo, run_undo, state_of, try_run,
};
use mergewright_core::on_disk::DIR;

/// A merge that stopped on one conflicted file, built in-test with plain
/// `git` so the shapes are git's own and not a fixture's transcription.
fn stopped_merge() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("both.txt", b"base one\nbase two\n");
    repo.write("plain.txt", b"untouched\n");
    repo.commit_all("base");
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("both.txt", b"theirs one\nbase two\n");
    repo.write("added.txt", b"added on the branch\n");
    repo.commit_all("theirs");
    repo.git(&["checkout", "-q", "main"]);
    repo.write("both.txt", b"ours one\nbase two\n");
    repo.commit_all("ours");
    assert!(
        repo.try_git(&["merge", "feature"]).is_err(),
        "the merge is meant to stop"
    );
    repo
}

/// A repository whose stash pop is about to conflict: one stash entry, and a
/// commit on top that touches the same line.
pub(crate) fn about_to_pop_a_conflict() -> (TempRepo, Oid) {
    let repo = TempRepo::init();
    repo.write("p.txt", b"base one\nbase two\n");
    repo.commit_all("base");
    repo.write("p.txt", b"stashed one\nbase two\n");
    repo.git(&["stash", "push", "-q", "-m", "will conflict"]);
    let stashed =
        Oid::from_hex(&repo.git(&["rev-parse", "stash@{0}"])).expect("the stash commit reads");
    repo.write("p.txt", b"committed one\nbase two\n");
    repo.commit_all("moved on");
    (repo, stashed)
}

fn stage_listing(repo: &TempRepo) -> String {
    repo.git(&["ls-files", "--stage"])
}

fn take_backup(repo: &TempRepo) -> snapshot::Snapshot {
    let handle = repo.open();
    let scratch = repo.path().join(format!(".git/{DIR}"));
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    let id = EntryId::new(NOW, [3; 10]);
    let head = Some(Oid::from_hex(&repo.git(&["rev-parse", "HEAD"])).expect("HEAD is an id"));
    snapshot::take(&handle, &scratch, &id, "main", "probe", &[], head, &Never)
        .expect("the snapshot is taken")
}

/// Restore one recorded index over whatever the repository holds now.
fn restore_the_index(repo: &TempRepo, index: IndexState, at: i64) {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    mergewright_ops::restore::execute(
        &mut ctx,
        &InversePlan {
            reverses: EntryId::new(NOW, [0; 10]),
            kind: OperationKind::Undo,
            description: "probe".to_owned(),
            touches: Touches {
                refs: Vec::new(),
                index: true,
                worktree: false,
            },
            steps: vec![Step::RestoreIndex {
                index,
                paths: IndexPaths::Everything,
            }],
            pins: Vec::new(),
        },
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
}

/// `take` over an unmerged index gives the stage tree, that tree holds every
/// stage of every path, and reading it back reproduces `git ls-files --stage`
/// byte for byte.
#[test]
fn unmerged_index_round_trips() {
    let repo = stopped_merge();
    let before = stage_listing(&repo);
    assert_eq!(
        before.lines().count(),
        5,
        "two clean paths and one path's three stages:\n{before}"
    );

    let taken = take_backup(&repo);
    let IndexState::Unmerged { unmerged } = taken.index else {
        panic!("a stopped merge is an unmerged index: {:?}", taken.index);
    };

    // The tree's shape: `<stage>/<path>` for every index entry, and nothing
    // else.
    let listing = repo.git(&["ls-tree", "-r", "--name-only", &unmerged.to_string()]);
    let mut names: Vec<&str> = listing.lines().collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "0/added.txt",
            "0/plain.txt",
            "1/both.txt",
            "2/both.txt",
            "3/both.txt",
        ],
        "the conflicted path has stages 1-3, every other path stage 0"
    );

    // The backup commit carries it under `unmerged/`, never `index/`.
    let commit = taken.commit.to_string();
    assert_eq!(
        repo.git(&["ls-tree", "--name-only", &commit]),
        "unmerged\nworktree"
    );

    // And a restore reproduces the index exactly. The index is wrecked first
    // — every conflict resolved — so the round trip has real work to undo.
    repo.git(&["add", "-A"]);
    assert_ne!(stage_listing(&repo), before, "the index really moved");

    restore_the_index(&repo, taken.index, NOW + 1);

    assert_eq!(
        stage_listing(&repo),
        before,
        "`git ls-files --stage` is byte-for-byte what it was"
    );
}

/// A merged index still gives the old layout: the same `index/` subtree, the
/// same tree id, no `unmerged/`. Every backup ref written since M3 reads as
/// before.
#[test]
fn merged_layout_unchanged() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"fn a() {}\n");
    repo.commit_all("one");
    repo.write("a.rs", b"fn a() { changed }\n");
    repo.git(&["add", "a.rs"]);

    let expected = repo.git(&["write-tree"]);
    let taken = take_backup(&repo);
    assert_eq!(
        taken.index,
        IndexState::Merged(Oid::from_hex(&expected).expect("write-tree printed an oid")),
        "a clean index is the plain `write-tree` tree"
    );

    let commit = taken.commit.to_string();
    assert_eq!(
        repo.git(&["ls-tree", "--name-only", &commit]),
        "index\nworktree",
        "no `unmerged/` in sight"
    );
    assert_eq!(
        repo.git(&["rev-parse", &format!("{commit}^{{tree}}:index")]),
        expected,
        "and the `index/` subtree is the pre-change layout's tree"
    );
}

/// Undoing a conflicted pop with nothing touched since shows **no** drift and
/// needs no `force`.
///
/// Before m6-conflicts/08 an unmerged index read as "no tree", so the
/// after-state was an absence, §6.7 could not compare it to anything, and `u`
/// demanded a force for drift that was not there
/// (`ops_execution_hard.rs`'s older pop test passes `true` for that reason).
#[test]
fn conflicted_pop_undo_is_clean_without_force() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let before = state_of(&repo);

    let failed = try_run(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: stashed,
        },
        NOW,
    );
    let entry = match failed {
        Err(ApplyError::Clashed { entry, .. }) => *entry,
        other => panic!("the pop should have conflicted: {other:?}"),
    };

    let index_after = entry.index_after.expect("the unmerged index is recorded");
    assert!(
        index_after.is_unmerged(),
        "a clashed pop leaves an unmerged index: {index_after:?}"
    );

    // Nothing has been touched since, so §6.7's comparison must come up
    // clean — the whole point of recording the shape rather than a gap.
    let handle = repo.open();
    let scratch = repo.path().join(format!(".git/{DIR}"));
    let refs: Vec<String> = entry.refs_after.keys().cloned().collect();
    let now = mergewright_ops::facts::read_state_now(&handle, &scratch, &refs, &Never)
        .expect("the state reads");
    let drift = undo::drift(&entry, &now);
    assert!(
        drift.is_clean(),
        "no drift after a conflicted pop: {}",
        drift.explain()
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        state_of(&repo),
        before,
        "and no force was needed to put it back"
    );
}

/// `Stage p` → `u` → `U` → `u`, `u` → `U`: the walk the defect blocked, both
/// halves. Truth from `git`, never from a returned value.
#[test]
fn pop_conflict_undo_redo_walks_both_ways() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let clean = state_of(&repo);

    let failed = try_run(
        &repo,
        &Operation::StashPop {
            index: 0,
            commit: stashed,
        },
        NOW,
    );
    assert!(failed.is_err(), "the pop conflicts");

    let conflicted_stages = repo.git(&["ls-files", "-u"]);
    let conflicted_bytes = repo.read("p.txt");
    assert_eq!(
        conflicted_stages.lines().count(),
        3,
        "three stages for p.txt"
    );

    // Resolve it: the resolution is its own entry, nothing being live.
    run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("p.txt"),
            side: Side::Ours,
            how: UnmergedHow::BothModified,
        }),
        NOW + 1,
    );
    assert_eq!(repo.git(&["ls-files", "-u"]), String::new());

    // `u` puts the conflict back, stages and bytes.
    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.git(&["ls-files", "-u"]), conflicted_stages);
    assert_eq!(repo.read("p.txt"), conflicted_bytes);
    assert!(
        repo.git(&["status", "--porcelain"]).starts_with("UU"),
        "git reads it as unmerged, not modified: {}",
        repo.git(&["status", "--porcelain"])
    );

    // `U` resolves it again.
    run_redo(&repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.git(&["ls-files", "-u"]), String::new());

    // `u`, `u`: the resolution, then the pop itself. The second `u` is the
    // one the defect blocked outright — undo being linear, the resolution's
    // missing before-index made every older entry unreachable.
    run_undo(&repo, NOW + 4, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    run_undo(&repo, NOW + 5, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), clean, "the pop is undone");
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "and the stash is back on the list"
    );

    // `U`: the pop again, markers on disk and an unmerged index.
    run_redo(&repo, NOW + 6, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.git(&["ls-files", "-u"]),
        conflicted_stages,
        "the same three stages"
    );
    assert_eq!(repo.read("p.txt"), conflicted_bytes, "markers and all");
}

/// Three hundred conflicted paths: the stage tree and the restore are linear
/// in the index and there is no size refusal. The wall clock is printed,
/// never asserted — a laptop is not a benchmark rig (PROMPT.md 7.1).
#[test]
fn three_hundred_conflicted_paths() {
    let repo = TempRepo::init();
    for n in 0..300 {
        repo.write(&format!("f{n:03}.txt"), b"base\n");
    }
    repo.commit_all("base");
    repo.git(&["checkout", "-q", "-b", "feature"]);
    for n in 0..300 {
        repo.write(&format!("f{n:03}.txt"), b"theirs\n");
    }
    repo.commit_all("theirs");
    repo.git(&["checkout", "-q", "main"]);
    for n in 0..300 {
        repo.write(&format!("f{n:03}.txt"), b"ours\n");
    }
    repo.commit_all("ours");
    assert!(repo.try_git(&["merge", "feature"]).is_err());

    let before = stage_listing(&repo);
    assert_eq!(before.lines().count(), 900, "300 paths, three stages each");

    let started = std::time::Instant::now();
    let taken = take_backup(&repo);
    let tree_ms = started.elapsed().as_millis();
    assert!(taken.index.is_unmerged());

    repo.git(&["add", "-A"]);
    let started = std::time::Instant::now();
    restore_the_index(&repo, taken.index, NOW + 1);
    let restore_ms = started.elapsed().as_millis();

    assert_eq!(
        stage_listing(&repo),
        before,
        "900 index lines, byte for byte"
    );
    println!("stage tree: {tree_ms} ms, restore: {restore_ms} ms");
}

/// Every journal line M3–M5 wrote still parses, and still renders back
/// **byte-identical**.
///
/// The golden is real: six lines captured from the renderer as it stood
/// before this group touched it (git 2.54.0), not a hand-written sample. A
/// bare-oid `index_before` has to read as `Merged` and write back bare, or
/// every journal on disk is one `u` cannot use.
#[test]
fn merged_lines_render_unchanged() {
    let golden = include_bytes!("goldens/m5-journal.jsonl");
    let parsed = mergewright_ops::journal::parse(golden).expect("the m5 golden parses");
    assert_eq!(parsed.entries.len(), 3, "three entries, six lines");
    assert!(parsed.torn_tail.is_none());

    for entry in &parsed.entries {
        for state in [entry.index_before, entry.index_after]
            .into_iter()
            .flatten()
        {
            assert!(
                !state.is_unmerged(),
                "an M5 line is always a merged index: {state:?}"
            );
        }
    }

    let mut rendered = Vec::new();
    for entry in &parsed.entries {
        rendered.extend(mergewright_ops::journal::render_pending(entry).expect("the pending line"));
        rendered.extend(mergewright_ops::journal::render_outcome(entry).expect("the outcome line"));
    }
    assert_eq!(
        String::from_utf8_lossy(&rendered),
        String::from_utf8_lossy(golden),
        "an M5 journal renders back exactly as it was written"
    );
}

/// One path's stages, read back out of a recorded state.
#[test]
fn stage_entries_reads_one_path_out_of_either_shape() {
    let repo = stopped_merge();
    let taken = take_backup(&repo);
    let handle = repo.open();

    let held = snapshot::stage_entries(&handle, taken.index, Path::new("both.txt"), &Never)
        .expect("the stages read");
    assert_eq!(
        held.iter().map(|entry| entry.stage).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let expected = repo.git(&["ls-files", "-u", "--", "both.txt"]);
    for entry in &held {
        assert!(
            expected.contains(&entry.oid.to_string()),
            "stage {} names a blob `ls-files -u` does not: {expected}",
            entry.stage
        );
    }

    let plain = snapshot::stage_entries(&handle, taken.index, Path::new("plain.txt"), &Never)
        .expect("the stages read");
    assert_eq!(plain.len(), 1);
    assert_eq!(plain[0].stage, 0);

    let absent = snapshot::stage_entries(&handle, taken.index, Path::new("nope.txt"), &Never)
        .expect("the stages read");
    assert!(absent.is_empty(), "a path the state does not hold has none");
}
