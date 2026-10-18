//! `progress`'s followers (m7-power-tools/07d D2): the branches of the line
//! above the one a rewrite stopped in, each with its tip's author and how
//! many of its commits are still to come — read from a real `git rebase -i`
//! stopped on an `edit` row, with the plan Mergewright saves on disk as a relaunch
//! reads it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::oid::Oid;
use mergewright_ops::rewrite::{Follower, progress};

use crate::support::{Never, TempRepo};
use mergewright_core::on_disk::CURRENT_DIR;

fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

/// One commit changing its own file, authored by `author`.
fn commit_by(repo: &TempRepo, name: &str, author: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.git(&["add", "-A"]);
    repo.git(&[
        "-c",
        &format!("user.name={author}"),
        "commit",
        "-q",
        "-m",
        name,
    ]);
}

/// `main` (m1); his `feature-a` four commits on it; Sara's `feature-b` two
/// on that; Lina's `feature-c` one on that. `HEAD` on `feature-c`.
fn three_branch_line() -> TempRepo {
    let repo = TempRepo::init();
    commit_by(&repo, "m1", "Ali");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    for name in ["a1", "a2", "a3", "a4"] {
        commit_by(&repo, name, "Ali");
    }
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit_by(&repo, "b1", "Sara");
    commit_by(&repo, "b2", "Sara");
    repo.git(&["switch", "-q", "-c", "feature-c"]);
    commit_by(&repo, "c1", "Lina");
    repo
}

/// The plan a Mergewright rewrite of the line saves before its Pending line.
fn save_plan(repo: &TempRepo) {
    let dir = repo
        .path()
        .join(format!(".git/{CURRENT_DIR}/rewrite/1700000000000-0000"));
    std::fs::create_dir_all(&dir).expect("the plan dir is made");
    let plan_json = format!(
        r#"{{"home":"refs/heads/feature-a","line":[["refs/heads/feature-a","{}"],["refs/heads/feature-b","{}"],["refs/heads/feature-c","{}"]],"onto":"{}"}}"#,
        oid(repo, "feature-a"),
        oid(repo, "feature-b"),
        oid(repo, "feature-c"),
        oid(repo, "main")
    );
    std::fs::write(dir.join("plan.json"), plan_json).expect("the plan is written");
}

/// The todo Mergewright writes for the line, with `edit` on the row named.
fn line_todo(repo: &TempRepo, edit: &str) -> String {
    let row = |name: &str, rev: &str| {
        let word = if name == edit { "edit" } else { "pick" };
        format!("{word} {} {name}\n", oid(repo, rev))
    };
    [
        row("a1", "feature-a~3"),
        row("a2", "feature-a~2"),
        row("a3", "feature-a~1"),
        row("a4", "feature-a"),
        "update-ref refs/heads/feature-a\n".to_owned(),
        row("b1", "feature-b~1"),
        row("b2", "feature-b"),
        "update-ref refs/heads/feature-b\n".to_owned(),
        row("c1", "feature-c"),
    ]
    .concat()
}

/// Start `git rebase -i` in the shell with `todo` as the whole todo.
fn rebase_with_todo(repo: &TempRepo, todo: &str, args: &[&str]) {
    let file = repo.path().with_extension("todo");
    std::fs::write(&file, todo).expect("the todo is written");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["rebase", "-i", "--no-autostash"])
        .args(args)
        .env(
            "GIT_SEQUENCE_EDITOR",
            format!("cp '{}'", file.to_str().expect("a UTF-8 temp path")),
        )
        .env("GIT_EDITOR", "true")
        .output()
        .expect("git runs");
    std::fs::remove_file(&file).expect("the todo clears");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn follower(branch: &str, tip_author: &str, commits: usize) -> Follower {
    Follower {
        branch: branch.to_owned(),
        tip_author: tip_author.to_owned(),
        commits,
    }
}

#[test]
fn progress_names_each_follower_with_author_and_count() {
    let repo = three_branch_line();
    save_plan(&repo);
    let todo = line_todo(&repo, "a2");
    rebase_with_todo(&repo, &todo, &["main"]);

    let found = progress(&repo.open(), &Never)
        .expect("it reads")
        .expect("a rebase is stopped");

    assert_eq!(
        found.followers,
        [
            follower("feature-b", "Sara", 2),
            follower("feature-c", "Lina", 1)
        ]
    );
    let here: Vec<&str> = found
        .pending_here()
        .iter()
        .map(|row| row.subject.as_str())
        .collect();
    assert_eq!(here, ["a3", "a4"], "his rows still to apply, oldest first");
    assert_eq!(found.pending.len(), 5, "pending still holds every row");
    assert_eq!(found.step, (2, 7), "the step counts every pick-like row");
}

#[test]
fn no_followers_on_the_top_branch() {
    let repo = three_branch_line();
    save_plan(&repo);
    let todo = line_todo(&repo, "c1");
    rebase_with_todo(&repo, &todo, &["main"]);

    let found = progress(&repo.open(), &Never)
        .expect("it reads")
        .expect("a rebase is stopped");

    let (_, index, of) = found.branch.clone().expect("a line of three");
    assert_eq!((index, of), (3, 3));
    assert_eq!(found.followers, []);
    assert!(found.pending_here().is_empty());
}

#[test]
fn no_followers_for_a_shell_rebase() {
    let repo = three_branch_line();
    // No saved plan: the shell's own `rebase -i --update-refs`.
    let todo = line_todo(&repo, "a2");
    rebase_with_todo(&repo, &todo, &["--update-refs", "main"]);

    let found = progress(&repo.open(), &Never)
        .expect("it reads")
        .expect("a rebase is stopped");

    assert_eq!(found.branch, None);
    assert_eq!(found.followers, []);
    assert_eq!(
        found.pending_here().len(),
        found.pending.len(),
        "with no line known, every pending row is here"
    );
}
