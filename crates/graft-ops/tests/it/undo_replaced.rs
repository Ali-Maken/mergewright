//! m6-conflicts/77: an undo forced past drift names exactly what it
//! replaces, and the way back puts it back.
//!
//! Every expectation is read out of the repository with `git` and the files
//! on disk — before `y` (the forced run), after it, and in the reversed
//! entry's recorded after-state — never from the value the run returned.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use graft_core::journal::{HeadRef, JournalEntry};
use graft_ops::operation::{CheckoutParams, CheckoutTarget, Operation};
use graft_ops::undo::{Replaced, UndoError};

use crate::support::{NOW, RepoState, TempRepo, journal_for, run, run_redo, run_undo, state_of};

/// `main` holds `base.txt` and `main.txt`; `feature` (off the first commit)
/// holds `base.txt` and `a.txt`, `b.txt`, `c.txt`. `main` is checked out.
/// Checking out `feature` writes exactly `main.txt`, `a.txt`, `b.txt`,
/// `c.txt`; `base.txt` is outside every restore's scope.
fn main_and_feature() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"a\n");
    repo.write("b.txt", b"b\n");
    repo.write("c.txt", b"c\n");
    repo.commit_all("three on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

fn checkout_feature() -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    })
}

fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    journal_for(&repo.open()).entries().to_vec()
}

/// What each path holds, as `git` hashes it: the working tree's files, the
/// index's entries, and the reversed entry's recorded after-state of both.
#[derive(Debug)]
struct Holds {
    worktree: BTreeMap<PathBuf, String>,
    index: BTreeMap<PathBuf, String>,
}

fn holds_now(repo: &TempRepo) -> Holds {
    let worktree = state_of(repo)
        .files
        .into_iter()
        .map(|(path, _bytes)| {
            let blob = repo.git(&["hash-object", "--no-filters", &path]);
            (PathBuf::from(path), blob)
        })
        .collect();
    let index = repo
        .git(&["ls-files", "-s"])
        .lines()
        .map(|line| {
            let (entry, path) = line
                .split_once('\t')
                .expect("ls-files -s: <entry>\\t<path>");
            (PathBuf::from(path), entry.to_owned())
        })
        .collect();
    Holds { worktree, index }
}

/// A recorded tree, read the same two ways: its blobs by path, and its
/// entries in `ls-files -s`'s own shape (`<mode> <oid> 0`).
fn holds_in(repo: &TempRepo, worktree: &str, index: &str) -> Holds {
    let listed = |tree: &str| -> Vec<(PathBuf, String, String)> {
        repo.git(&["ls-tree", "-r", tree])
            .lines()
            .map(|line| {
                let (head, path) = line.split_once('\t').expect("ls-tree: <head>\\t<path>");
                let mut fields = head.split(' ');
                let mode = fields.next().expect("mode").to_owned();
                let _kind = fields.next();
                let blob = fields.next().expect("oid").to_owned();
                (PathBuf::from(path), mode, blob)
            })
            .collect()
    };
    Holds {
        worktree: listed(worktree)
            .into_iter()
            .map(|(path, _mode, blob)| (path, blob))
            .collect(),
        index: listed(index)
            .into_iter()
            .map(|(path, mode, blob)| (path, format!("{mode} {blob} 0")))
            .collect(),
    }
}

/// The invariant's set, from `git` alone: a path whose bytes the forced
/// run changed, and which the user had changed from the recorded
/// after-state — in the working tree or in the index.
fn taken(pre: &Holds, post: &Holds, recorded: &Holds) -> Vec<PathBuf> {
    let side = |pre: &BTreeMap<PathBuf, String>,
                post: &BTreeMap<PathBuf, String>,
                recorded: &BTreeMap<PathBuf, String>|
     -> BTreeSet<PathBuf> {
        pre.keys()
            .chain(post.keys())
            .filter(|path| {
                pre.get(*path) != post.get(*path) && pre.get(*path) != recorded.get(*path)
            })
            .cloned()
            .collect()
    };
    let mut found = side(&pre.worktree, &post.worktree, &recorded.worktree);
    found.extend(side(&pre.index, &post.index, &recorded.index));
    found.into_iter().collect()
}

/// One scenario: what the user does between the operation and `u`, and the
/// paths the box must name.
struct Scenario {
    name: &'static str,
    /// Run after the checkout (and, for a redo, after the clean undo).
    edit: fn(&TempRepo),
    /// `U` over drift rather than `u`.
    redo: bool,
    names: &'static [&'static str],
}

#[allow(
    clippy::too_many_lines,
    reason = "one table of scenarios, one row each: data, not logic"
)]
fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "an unstaged edit inside the checkout's paths",
            edit: |repo| repo.write("a.txt", b"mine\n"),
            redo: false,
            names: &["a.txt"],
        },
        Scenario {
            name: "a staged edit",
            edit: |repo| {
                repo.write("a.txt", b"staged\n");
                repo.git(&["add", "a.txt"]);
            },
            redo: false,
            names: &["a.txt"],
        },
        Scenario {
            name: "a staged edit and a different one on disk",
            edit: |repo| {
                repo.write("a.txt", b"staged\n");
                repo.git(&["add", "a.txt"]);
                repo.write("a.txt", b"on disk\n");
            },
            redo: false,
            names: &["a.txt"],
        },
        Scenario {
            name: "an edit inside and one outside (base.txt)",
            edit: |repo| {
                repo.write("a.txt", b"mine\n");
                repo.write("base.txt", b"base, edited\n");
            },
            redo: false,
            names: &["a.txt"],
        },
        Scenario {
            name: "three files",
            edit: |repo| {
                repo.write("a.txt", b"a, mine\n");
                repo.write("b.txt", b"b, mine\n");
                repo.write("c.txt", b"c, mine\n");
            },
            redo: false,
            names: &["a.txt", "b.txt", "c.txt"],
        },
        Scenario {
            name: "an untracked file where main.txt goes back",
            edit: |repo| repo.write("main.txt", b"not main's\n"),
            redo: false,
            names: &["main.txt"],
        },
        Scenario {
            name: "an untracked file holding exactly what goes back",
            edit: |repo| repo.write("main.txt", b"main\n"),
            redo: false,
            names: &[],
        },
        Scenario {
            name: "a tracked file deleted by hand, which the undo deletes too",
            edit: |repo| std::fs::remove_file(repo.path().join("a.txt")).expect("a.txt is there"),
            redo: false,
            names: &[],
        },
        Scenario {
            name: "an untracked file outside the checkout's paths",
            edit: |repo| repo.write("new.txt", b"new\n"),
            redo: false,
            names: &[],
        },
        Scenario {
            name: "a staged edit whose disk copy went back to the checkout's",
            edit: |repo| {
                repo.write("a.txt", b"staged\n");
                repo.git(&["add", "a.txt"]);
                repo.write("a.txt", b"a\n");
            },
            redo: false,
            names: &["a.txt"],
        },
        Scenario {
            name: "a redo over an edit to main.txt",
            edit: |repo| repo.write("main.txt", b"main, edited\n"),
            redo: true,
            names: &["main.txt"],
        },
    ]
}

/// Each replaced path is in the forced run's own backup, as it stood before
/// `y`: some commit on the backup ref holds it at `worktree/<p>` (or at
/// `index/<p>`, where only the staged copy was the user's).
fn backup_keeps(
    repo: &TempRepo,
    name: &str,
    forced: &graft_ops::apply::Applied,
    paths: &[PathBuf],
    pre: &Holds,
    recorded: &Holds,
) {
    let backup = forced
        .entry
        .backup_ref
        .clone()
        .expect("a forced undo takes a backup");
    let commits = repo.git(&["rev-list", &backup]);
    for path in paths {
        let (side, blob) = match (pre.worktree.get(path), recorded.worktree.get(path)) {
            (Some(blob), recorded) if Some(blob) != recorded => ("worktree", blob.clone()),
            _ => {
                let staged = pre.index.get(path).expect("replaced in the index, then");
                let blob = staged.split(' ').nth(1).expect("<mode> <oid> <stage>");
                ("index", blob.to_owned())
            }
        };
        let spec = |commit: &str| format!("{commit}:{side}/{}", path.display());
        assert!(
            commits.lines().any(|commit| {
                repo.try_git(&["rev-parse", "--verify", "-q", &spec(commit)])
                    .is_ok_and(|held| held == blob)
            }),
            "{name}: {backup} keeps {blob} at {side}/{}",
            path.display()
        );
    }
}

/// The time machine is the same `run_plan` per step; `u` and `U` are the
/// two directions of it.
fn reverse(
    repo: &TempRepo,
    is_redo: bool,
    at: i64,
    force: bool,
) -> Result<graft_ops::apply::Applied, UndoError> {
    if is_redo {
        run_redo(repo, at, force)
    } else {
        run_undo(repo, at, force)
    }
}

#[test]
fn replaced_names_exactly_what_the_run_took() {
    for scenario in scenarios() {
        let name = scenario.name;
        let repo = main_and_feature();
        run(&repo, &checkout_feature(), NOW);
        if scenario.redo {
            run_undo(&repo, NOW + 1, false)
                .unwrap_or_else(|why| panic!("{name}: {}", why.explain()));
        }
        (scenario.edit)(&repo);
        let reversed = entries(&repo)
            .last()
            .cloned()
            .expect("the entry to reverse");
        let pre = holds_now(&repo);
        let pre_state: RepoState = state_of(&repo);

        // The box: the unforced run stops at the rail and names them.
        let boxed = match reverse(&repo, scenario.redo, NOW + 2, false) {
            Err(UndoError::Drift(drift)) => drift.replaced,
            Err(other) => panic!("{name}: the rail answered {}", other.explain()),
            Ok(_) => panic!("{name}: no drift box over an edit"),
        };
        assert_eq!(state_of(&repo), pre_state, "{name}: the box wrote nothing");

        // `y`: the forced run.
        let forced = reverse(&repo, scenario.redo, NOW + 3, true)
            .unwrap_or_else(|why| panic!("{name}: {}", why.explain()));
        let post = holds_now(&repo);
        let recorded = holds_in(
            &repo,
            &reversed.worktree_after.expect("recorded").to_string(),
            &reversed.index_after.expect("recorded").tree().to_string(),
        );
        let truth = taken(&pre, &post, &recorded);
        let named: Vec<PathBuf> = scenario.names.iter().map(PathBuf::from).collect();
        assert_eq!(truth, named, "{name}: git says these were taken");
        let expected = Replaced {
            paths: named,
            whole_tree: false,
        };
        assert_eq!(boxed, expected, "{name}: the box names what the run takes");
        assert_eq!(
            forced.replaced, expected,
            "{name}: the landing names what it took"
        );

        backup_keeps(&repo, name, &forced, &expected.paths, &pre, &recorded);

        // And after an undo, the way back is one `U`, byte-exact: the redo
        // reverses this very `Undo` entry, from its own before-state.
        //
        // Not after a redo. `u` then reverses the root operation from *its*
        // record (`journal::next_undo` passes over `Redo` entries, and
        // `plan_undo` inverts the operation), which never held the edit — it
        // is in the redo's backup only. `crates/graft/tests/it/
        // undo_over_drift.rs` pins that, and the box does not promise `u`.
        if !scenario.redo {
            run_redo(&repo, NOW + 4, false)
                .unwrap_or_else(|why| panic!("{name}, the way back: {}", why.explain()));
            assert_eq!(state_of(&repo), pre_state, "{name}: U put it back");
        }
    }
}

/// Decided 4: the read runs only on drift. A clean undo's Command Log holds
/// no `diff-tree` but the restore's own scope read (`<after> <before>`); the
/// same undo forced over an edit holds the read against the tree as it is
/// now as well — the control that the log would show it.
#[test]
fn a_clean_undo_spawns_no_diff_tree() {
    let diff_trees = |entry: &JournalEntry| -> Vec<String> {
        entry
            .commands
            .iter()
            .filter(|line| line.contains("diff-tree"))
            .cloned()
            .collect()
    };

    let clean = main_and_feature();
    let checkout = run(&clean, &checkout_feature(), NOW);
    let undone = run_undo(&clean, NOW + 1, false)
        .expect("a clean undo")
        .entry;
    let scope = format!(
        "{} {}",
        checkout.worktree_after.expect("recorded"),
        checkout.worktree_before.expect("recorded")
    );
    let lines = diff_trees(&undone);
    assert!(
        !lines.is_empty(),
        "the restore's scope read is logged: {:?}",
        undone.commands
    );
    for line in &lines {
        assert!(
            line.ends_with(&scope),
            "a clean undo ran a diff-tree of its own: {line}"
        );
    }

    let drifted = main_and_feature();
    let checkout = run(&drifted, &checkout_feature(), NOW);
    drifted.write("a.txt", b"mine\n");
    let forced = run_undo(&drifted, NOW + 1, true)
        .expect("a forced undo")
        .entry;
    let now = forced
        .worktree_before
        .expect("the forced undo's own snapshot");
    let read = format!("{} {now}", checkout.worktree_after.expect("recorded"));
    assert!(
        diff_trees(&forced).iter().any(|line| line.ends_with(&read)),
        "the forced run's read is in its log: {:?}",
        forced.commands
    );
}
