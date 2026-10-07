//! The stash list previews before it writes (m6-conflicts/86): what
//! `preview::plan` draws for `StashApply` / `StashPop`.
//!
//! Every repository here is real. The forecast is judged against git itself:
//! `the_stash_forecast_agrees_with_git` runs the put-back on a copy and
//! compares what git did with what the preview said.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_git::version::GitVersion;
use graft_ops::facts::read_facts;
use graft_ops::operation::{Operation, Refusal};
use graft_ops::preview::{self, ConflictForecast, PreviewError, PreviewPlan, WorktreeEffect};

use crate::support::{Never, TempRepo, git_version};

// ── fixtures ────────────────────────────────────────────────────────────

/// `notes.md` and `other.txt` committed; `notes.md` edited and stashed with
/// `-u` (Graft's own default scope). The tree is clean afterwards.
fn stashed() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("notes.md", b"base\n");
    repo.write("other.txt", b"other\n");
    repo.commit_all("one");
    repo.write("notes.md", b"stashed line\n");
    repo.git(&["stash", "push", "-q", "-u"]);
    repo
}

/// As [`stashed`], then `notes.md` changed again and committed: putting the
/// stash back clashes on it.
fn clashing() -> TempRepo {
    let repo = stashed();
    repo.write("notes.md", b"committed line\n");
    repo.commit_all("two");
    repo
}

/// As [`stashed`], with an unsaved (worktree ≠ index) edit to `notes.md`.
fn tracked_in_the_way() -> TempRepo {
    let repo = stashed();
    repo.write("notes.md", b"unsaved\n");
    repo
}

/// A stash that carries a new, untracked `new.txt`; a different `new.txt`
/// has been made since.
fn untracked_in_the_way() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("notes.md", b"base\n");
    repo.commit_all("one");
    repo.write("notes.md", b"stashed line\n");
    repo.write("new.txt", b"stashed new\n");
    repo.git(&["stash", "push", "-q", "-u"]);
    repo.write("new.txt", b"mine\n");
    repo
}

/// As [`stashed`], with a staged edit to another file.
fn staged_elsewhere() -> TempRepo {
    let repo = stashed();
    repo.write("other.txt", b"staged\n");
    repo.git(&["add", "other.txt"]);
    repo
}

/// A stash of one untracked file and nothing else.
fn untracked_only() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("notes.md", b"base\n");
    repo.commit_all("one");
    repo.write("new.txt", b"stashed new\n");
    repo.git(&["stash", "push", "-q", "-u"]);
    repo
}

/// A staged change to line 5 of `f`, stashed; a change to line 3 committed
/// since. The three-way merge is clean, but git's `--index` put-back first
/// applies the staged part to the index, whose context no longer matches:
/// `conflicts in index. Try without --index.` (review 86, M1).
fn staged_no_longer_applies() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("f", b"1\n2\n3\n4\n5\n6\n");
    repo.commit_all("one");
    repo.write("f", b"1\n2\n3\n4\nfive\n6\n");
    repo.git(&["add", "f"]);
    repo.git(&["stash", "push", "-q"]);
    repo.write("f", b"1\n2\nthree\n4\n5\n6\n");
    repo.commit_all("two");
    repo
}

/// A stash whose only change is staged: `notes.md` staged, then its file put
/// back to the committed bytes. `stash^1`→`stash` is empty; `stash^2` holds
/// the change (review 86, m1).
fn index_only() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("notes.md", b"base\n");
    repo.commit_all("one");
    repo.write("notes.md", b"staged\n");
    repo.git(&["add", "notes.md"]);
    repo.write("notes.md", b"base\n");
    repo.git(&["stash", "push", "-q"]);
    repo
}

/// A stash of `a.txt` and `b.txt`; since, an unsaved edit to `a.txt` and a
/// staged-only edit to `b.txt` (review 86, m5).
fn mixed_in_the_way() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.write("b.txt", b"b\n");
    repo.commit_all("one");
    repo.write("a.txt", b"stashed a\n");
    repo.write("b.txt", b"stashed b\n");
    repo.git(&["stash", "push", "-q", "-u"]);
    repo.write("a.txt", b"unsaved a\n");
    repo.write("b.txt", b"staged b\n");
    repo.git(&["add", "b.txt"]);
    repo
}

fn top_stash(repo: &TempRepo) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", "stash@{0}"])).expect("a full id")
}

fn apply_top(repo: &TempRepo) -> Operation {
    Operation::StashApply {
        index: 0,
        commit: top_stash(repo),
    }
}

fn pop_top(repo: &TempRepo) -> Operation {
    Operation::StashPop {
        index: 0,
        commit: top_stash(repo),
    }
}

fn plan_with(
    repo: &TempRepo,
    op: &Operation,
    git: &GitVersion,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    preview::plan(&handle, git, op, &facts, stop)
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    plan_with(repo, op, &git_version(), &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

// ── the plan ────────────────────────────────────────────────────────────

#[test]
fn a_stash_that_applies_cleanly_forecasts_clean() {
    let repo = stashed();
    for op in [apply_top(&repo), pop_top(&repo)] {
        let plan = plan_of(&repo, &op);
        assert_eq!(plan.conflicts, ConflictForecast::Clean, "{op:?}");
        assert_eq!(
            plan.worktree,
            WorktreeEffect::Updated {
                paths: paths(&["notes.md"]),
                more: false
            }
        );
        assert!(plan.ghosts.is_empty() && plan.ref_moves.is_empty() && plan.head.is_none());
        assert_eq!(plan.description, op.describe());
        assert_eq!(plan.commands.len(), 1, "the plan's own line");
    }
}

#[test]
fn a_stash_that_clashes_forecasts_its_paths() {
    let repo = clashing();
    let plan = plan_of(&repo, &pop_top(&repo));
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Likely {
            paths: paths(&["notes.md"])
        }
    );
}

#[test]
fn unsaved_edits_in_the_way_refuse_the_preview() {
    let repo = tracked_in_the_way();
    let answer = plan_with(&repo, &apply_top(&repo), &git_version(), &Never);
    match answer {
        Err(PreviewError::Refused(refusal)) => {
            assert_eq!(
                refusal,
                Refusal::StashInTheWay {
                    unsaved: paths(&["notes.md"]),
                    staged: Vec::new(),
                    untracked: Vec::new(),
                    ignored: Vec::new(),
                }
            );
            assert_eq!(
                refusal.explain(&crate::support::Letters),
                "your unsaved edits to notes.md are in the way"
            );
        }
        other => panic!("refused, in the way: {other:?}"),
    }
}

/// An untracked or ignored file in the way is not his edit: it reads
/// `<path> is in the way` (review 86, m4 — the conductor's ruling); an
/// ignored one adds `(ignored)` and `move it aside first`, since `S` can't
/// take it (review 28, m4).
#[test]
fn an_untracked_file_in_the_way_refuses_the_preview() {
    let repo = untracked_in_the_way();
    let expected = Refusal::StashInTheWay {
        unsaved: Vec::new(),
        staged: Vec::new(),
        untracked: paths(&["new.txt"]),
        ignored: Vec::new(),
    };
    let answer = plan_with(&repo, &pop_top(&repo), &git_version(), &Never);
    match answer {
        Err(PreviewError::Refused(refusal)) => {
            assert_eq!(refusal, expected);
            assert_eq!(
                refusal.explain(&crate::support::Letters),
                "new.txt is in the way"
            );
        }
        other => panic!("refused, in the way: {other:?}"),
    }

    // An ignored file is not in the status, so `check` does not see it; git
    // still refuses to write over it. The preview reads the disk.
    repo.write(".git/info/exclude", b"new.txt\n");
    let answer = plan_with(&repo, &pop_top(&repo), &git_version(), &Never);
    match answer {
        Err(PreviewError::Refused(refusal)) => {
            let ignored = Refusal::StashInTheWay {
                unsaved: Vec::new(),
                staged: Vec::new(),
                untracked: Vec::new(),
                ignored: paths(&["new.txt"]),
            };
            assert_eq!(refusal, ignored, "an ignored file in the way");
            assert_eq!(
                refusal.explain(&crate::support::Letters),
                "new.txt (ignored) is in the way · move it aside first"
            );
        }
        other => panic!("an ignored file in the way: {other:?}"),
    }
}

/// Unsaved edits to one stashed path and staged-only edits to another: the
/// refusal names both, so fixing the first set does not meet a second
/// refusal it never mentioned (review 86, m5).
#[test]
fn a_mixed_refusal_names_unsaved_and_staged() {
    let repo = mixed_in_the_way();
    let answer = plan_with(&repo, &apply_top(&repo), &git_version(), &Never);
    match answer {
        Err(PreviewError::Refused(refusal)) => {
            assert_eq!(
                refusal,
                Refusal::StashInTheWay {
                    unsaved: paths(&["a.txt"]),
                    staged: paths(&["b.txt"]),
                    untracked: Vec::new(),
                    ignored: Vec::new(),
                }
            );
            assert_eq!(
                refusal.explain(&crate::support::Letters),
                "your unsaved edits to a.txt and your staged edits to b.txt are in the way"
            );
        }
        other => panic!("refused, both named: {other:?}"),
    }
}

/// git's `--index` rule: when the staged part no longer applies to the index,
/// `stash apply --index` refuses before writing. The preview refuses too,
/// never `no conflicts expected` (review 86, M1).
#[test]
fn a_staged_part_that_no_longer_applies_refuses_the_preview() {
    let repo = staged_no_longer_applies();
    let before = written(&repo);
    for op in [apply_top(&repo), pop_top(&repo)] {
        let answer = plan_with(&repo, &op, &git_version(), &Never);
        // `apply --check` ran on the temporary copy: nothing written.
        assert_eq!(written(&repo), before, "{op:?}");
        match answer {
            Err(PreviewError::Refused(refusal)) => {
                assert_eq!(refusal, Refusal::StashIndexNoLongerApplies { index: 0 });
                assert_eq!(
                    refusal.explain(&crate::support::Letters),
                    "the staged part of stash@{0} no longer applies"
                );
            }
            other => panic!("{op:?}: refused, the staged part: {other:?}"),
        }
    }
}

/// A stash whose change is only staged names the staged paths, never an
/// empty `working tree:` list (review 86, m1).
#[test]
fn an_index_only_stash_names_its_staged_paths() {
    let repo = index_only();
    let plan = plan_of(&repo, &apply_top(&repo));
    assert_eq!(
        plan.worktree,
        WorktreeEffect::IndexOnly {
            paths: paths(&["notes.md"]),
            more: false
        }
    );
    assert_eq!(plan.conflicts, ConflictForecast::Clean);
}

#[test]
fn an_untracked_only_stash_lists_its_files() {
    let repo = untracked_only();
    let plan = plan_of(&repo, &apply_top(&repo));
    assert_eq!(
        plan.worktree,
        WorktreeEffect::Updated {
            paths: paths(&["new.txt"]),
            more: false
        }
    );
    assert_eq!(plan.conflicts, ConflictForecast::Clean);
}

/// Everything `plan` could have written, read from git: the index file's
/// bytes, every ref, the stash list, the status, and the git dir's top-level
/// names (no temporary index left behind). As 84's writes test.
fn written(repo: &TempRepo) -> (Vec<u8>, String, String, String, Vec<String>) {
    let mut names: Vec<String> = std::fs::read_dir(repo.path().join(".git"))
        .expect("the git dir reads")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    (
        std::fs::read(repo.path().join(".git/index")).expect("the index reads"),
        repo.git(&["for-each-ref"]),
        repo.git(&["stash", "list"]),
        // `--no-optional-locks`: a plain status refreshes `.git/index`.
        repo.git(&["--no-optional-locks", "status", "--porcelain=v2"]),
        names,
    )
}

#[test]
fn the_stash_preview_writes_no_ref_index_or_file() {
    for repo in [clashing(), staged_elsewhere()] {
        // A clean file whose stat no longer matches the index: anything that
        // refreshed the real index would rewrite `.git/index`.
        std::process::Command::new("touch")
            .args(["-t", "200001010000"])
            .arg(repo.path().join("other.txt"))
            .status()
            .expect("touch runs");
        let before = written(&repo);
        for op in [apply_top(&repo), pop_top(&repo)] {
            let _plan: PreviewPlan = plan_of(&repo, &op);
            assert_eq!(written(&repo), before, "{op:?}");
        }
    }
}

/// A `Stop` that lets the first `n` polls through, then stops.
struct StopAfter {
    left: AtomicUsize,
}

impl Stop for StopAfter {
    fn should_stop(&self) -> bool {
        self.left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_err()
    }
}

/// `plan`'s own first poll passes; every later poll is tried, until the
/// plan completes: a stop between any two spawns is `Stopped`, never a
/// forecast with a hole in it, and the plan that finally completes is a real
/// forecast (review 86, m6: the loop used to end before the last spawn).
#[test]
fn the_stash_preview_honours_stop() {
    let repo = clashing();
    let op = pop_top(&repo);
    let mut polls = 1;
    let plan = loop {
        let stop = StopAfter {
            left: AtomicUsize::new(polls),
        };
        match plan_with(&repo, &op, &git_version(), &stop) {
            Err(PreviewError::Stopped) => polls += 1,
            Ok(plan) => break plan,
            Err(other) => panic!("after {polls} polls: {other:?}"),
        }
        assert!(polls < 200, "the plan never completed");
    };
    assert!(polls > 2, "stopped at the first poll and between spawns");
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Likely {
            paths: paths(&["notes.md"])
        },
        "completed after {polls} polls"
    );
}

/// On a git older than `merge-tree --merge-base` (2.40) the forecast is not
/// checked, and says why; the files are still listed.
#[test]
fn an_old_git_leaves_the_stash_forecast_not_checked() {
    let repo = clashing();
    let old = GitVersion {
        major: 2,
        minor: 39,
        patch: 0,
    };
    let plan = plan_with(&repo, &pop_top(&repo), &old, &Never).expect("the preview still draws");
    match plan.conflicts {
        ConflictForecast::NotChecked(why) => assert!(why.contains("2.40"), "{why}"),
        other => panic!("not checked on git 2.39: {other:?}"),
    }
    assert_eq!(
        plan.worktree,
        WorktreeEffect::Updated {
            paths: paths(&["notes.md"]),
            more: false
        }
    );
}

// ── the oracle ──────────────────────────────────────────────────────────

/// What git did when it ran the put-back on a copy.
struct GitDid {
    ok: bool,
    unmerged: Vec<PathBuf>,
    /// `status --porcelain=v2` and every file's bytes, before and after.
    tree_before: String,
    tree_after: String,
}

/// A throwaway copy of `repo`, the put-back run there with the plan's own
/// command (`stash apply --index stash@{0}`).
fn run_on_copy(repo: &TempRepo) -> GitDid {
    let copy = TempRepo::init();
    std::fs::remove_dir_all(copy.path()).expect("the empty copy clears");
    let copied = std::process::Command::new("cp")
        .arg("-R")
        .arg(repo.path())
        .arg(copy.path())
        .status()
        .expect("cp runs");
    assert!(copied.success());
    let tree = |repo: &TempRepo| {
        let status = repo.git(&["--no-optional-locks", "status", "--porcelain=v2", "-uall"]);
        let mut files: Vec<String> = repo
            .git(&["ls-files", "-co", "--exclude-standard"])
            .lines()
            .map(|name| {
                format!(
                    "{name}={}",
                    String::from_utf8_lossy(&repo.read(name)).trim_end()
                )
            })
            .collect();
        files.sort();
        format!("{status}\n{}", files.join("\n"))
    };
    let tree_before = tree(&copy);
    let ok = copy
        .try_git(&["stash", "apply", "--index", "stash@{0}"])
        .is_ok();
    let unmerged: Vec<PathBuf> = copy
        .git(&["diff", "--name-only", "--diff-filter=U"])
        .lines()
        .map(PathBuf::from)
        .collect();
    let tree_after = tree(&copy);
    GitDid {
        ok,
        unmerged,
        tree_before,
        tree_after,
    }
}

/// The forecast agrees with `git stash apply` on every row: `Clean` ⇒ git
/// exits 0 with nothing unmerged; `Likely { paths }` ⇒ the unmerged paths
/// are `paths`; `StashInTheWay` ⇒ git exits non-zero (tracked row: the tree
/// is unchanged, `docs/research/commit-stash.md` A1.4);
/// `StashIndexNoLongerApplies` ⇒ git exits non-zero and writes nothing.
#[test]
fn the_stash_forecast_agrees_with_git() {
    let rows: [(&str, TempRepo); 8] = [
        ("clean", stashed()),
        ("clash", clashing()),
        ("tracked in the way", tracked_in_the_way()),
        ("untracked in the way", untracked_in_the_way()),
        ("staged edits elsewhere", staged_elsewhere()),
        ("untracked-only", untracked_only()),
        (
            "staged line 5, committed line 3",
            staged_no_longer_applies(),
        ),
        ("index only", index_only()),
    ];
    let mut seen = Vec::new();
    for (row, repo) in &rows {
        let answer = plan_with(repo, &apply_top(repo), &git_version(), &Never);
        let did = run_on_copy(repo);
        match answer {
            Ok(plan) => match plan.conflicts {
                ConflictForecast::Clean => {
                    assert!(did.ok, "{row}: forecast clean, git failed");
                    assert!(did.unmerged.is_empty(), "{row}: {:?}", did.unmerged);
                    seen.push("clean");
                }
                ConflictForecast::Likely { paths } => {
                    assert!(!did.ok, "{row}: forecast a clash, git exited 0");
                    assert_eq!(did.unmerged, paths, "{row}");
                    seen.push("likely");
                }
                ConflictForecast::NotChecked(why) => panic!("{row}: not checked ({why})"),
                // Only a rebase or rewrite's chained replay stops at a step.
                stop @ ConflictForecast::StopsAt { .. } => panic!("{row}: {stop:?}"),
            },
            Err(PreviewError::Refused(Refusal::StashInTheWay { .. })) => {
                assert!(!did.ok, "{row}: refused as in the way, git exited 0");
                if *row == "tracked in the way" {
                    assert_eq!(did.tree_after, did.tree_before, "{row}: A1.4");
                }
                seen.push("in the way");
            }
            Err(PreviewError::Refused(Refusal::StashIndexNoLongerApplies { .. })) => {
                assert!(!did.ok, "{row}: refused, git exited 0");
                assert_eq!(did.tree_after, did.tree_before, "{row}: nothing written");
                seen.push("staged part refused");
            }
            Err(other) => panic!("{row}: {other:?}"),
        }
    }
    assert_eq!(
        seen,
        [
            "clean",
            "likely",
            "in the way",
            "in the way",
            "clean",
            "clean",
            "staged part refused",
            "clean"
        ]
    );
}
