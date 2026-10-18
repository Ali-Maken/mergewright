//! The move's scope: every worktree's folder, files nobody listed, and a
//! paused rewrite's saved plan.

use std::path::Path;

use mergewright_core::oid::Oid;
use mergewright_core::on_disk::Generation;
use mergewright_ops::apply::{ApplyError, Resume, resume};
use mergewright_ops::facts::read_facts;
use mergewright_ops::operation::{FactsScope, Operation};
use mergewright_ops::rename::SettleOutcome;
use mergewright_ops::rewrite::{RewriteRequest, TodoAction, plan};

use super::{as_new, old_repo, refs_under, settle};
use crate::support::{NOW, Never, TempRepo, context, git_version, journal_for, try_run};
use mergewright_core::on_disk::CURRENT_DIR;

/// `old_repo` with a linked worktree `linked` that has staged once (its own
/// journal, its own backup), and a second, `never`, whose folder holds a
/// journal no build of this one has opened.
fn with_worktrees() -> TempRepo {
    let repo = old_repo(0, 1);
    repo.git(&["branch", "side"]);
    repo.git(&["worktree", "add", "-q", "linked", "side"]);
    repo.harden(Path::new("linked"));
    repo.git(&["worktree", "add", "-q", "--detach", "never", "HEAD"]);
    repo.write("linked/a.rs", b"linked edit\n");
    let handle = repo.open_at("linked");
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 300, &Never);
    mergewright_ops::apply::apply(
        &mut ctx,
        &Operation::Stage(mergewright_ops::operation::Target::Paths(vec![
            "a.rs".into(),
        ])),
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    let never = repo.path().join(".git/worktrees/never/graft");
    std::fs::create_dir_all(&never).expect("the folder is made");
    std::fs::copy(
        repo.path()
            .join(".git/worktrees/linked/graft/undo-journal.jsonl"),
        never.join("undo-journal.jsonl"),
    )
    .expect("a journal is put there");
    repo
}

#[test]
fn every_worktree_moves_in_one_pass() {
    let repo = with_worktrees();
    let old = refs_under(&repo, "refs/graft/");
    assert_eq!(old.len(), 2, "main's backup and linked's: {old:?}");
    let journals = ["", "worktrees/linked/", "worktrees/never/"]
        .map(|at| (at, repo.read(&format!(".git/{at}graft/undo-journal.jsonl"))));

    // Opened from the linked worktree: one pass moves the whole repository.
    let handle = repo.open_at("linked");
    let settled = mergewright_ops::rename::settle_names(&handle);
    assert!(
        matches!(settled.outcome, SettleOutcome::Migrated { refs: 2, .. }),
        "{:?}",
        settled.outcome
    );

    assert_eq!(
        refs_under(&repo, &format!("refs/{CURRENT_DIR}/")),
        as_new(&old),
        "refs once"
    );
    for (at, bytes) in journals {
        assert!(
            !repo.exists(&format!(".git/{at}graft")),
            "{at}: old folder gone"
        );
        let moved = repo.read(&format!(
            ".git/{at}{}/undo-journal.jsonl",
            Generation::CURRENT.dir()
        ));
        if at == "worktrees/linked/" {
            assert!(
                moved.starts_with(&bytes),
                "the opener's gains the move's lines"
            );
            assert!(moved.len() > bytes.len());
        } else {
            assert_eq!(moved, bytes, "{at}: byte-equal");
        }
    }
    let linked = repo.open_at("linked");
    assert_eq!(linked.on_disk_names(), Generation::CURRENT);
    assert_eq!(
        journal_for(&linked).entries().len(),
        2,
        "its stage and the move"
    );
}

#[test]
fn git_dir_and_git_path_dirs_both_move() {
    let repo = with_worktrees();
    for at in [".", "linked"] {
        let handle = repo.open_at(at);
        assert_eq!(
            handle.git_path(Path::new("graft")).expect("resolves"),
            handle.git_dir().join("graft"),
            "{at}: one folder under either name"
        );
    }
    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));
    for at in [".", "linked"] {
        let handle = repo.open_at(at);
        let new = handle.git_dir().join(CURRENT_DIR);
        assert_eq!(
            handle.git_path(Path::new(CURRENT_DIR)).expect("resolves"),
            new
        );
        assert!(new.join("undo-journal.jsonl").is_file(), "{at}");
        assert!(!handle.git_dir().join("graft").exists(), "{at}");
    }
}

#[test]
fn unknown_files_travel() {
    let repo = with_worktrees();
    let unknown = [
        (".git/graft/mystery.bin", vec![0u8, 1, 2, 255, b'\n']),
        (".git/graft/deep/er/note", b"kept\n".to_vec()),
        (".git/worktrees/linked/graft/odd", b"odd\n".to_vec()),
        (".git/worktrees/never/graft/x/y", b"y\n".to_vec()),
    ];
    for (path, bytes) in &unknown {
        repo.write(path, bytes);
    }
    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));
    for (path, bytes) in &unknown {
        let moved = path.replace("/graft/", &format!("/{CURRENT_DIR}/"));
        assert_eq!(&repo.read(&moved), bytes, "{moved}");
    }
}

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

#[test]
fn saved_rewrite_plan_survives() {
    let repo = TempRepo::init_on(Generation::Graft);
    repo.write("m1.txt", b"m1\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    for name in ["a1", "a2"] {
        repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
        repo.commit_all(name);
    }
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &FactsScope::default())
        .unwrap_or_else(|why| panic!("the facts read: {}", why.explain()));
    let mut params = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open {
            from: id(&repo, "feature~1"),
        },
        &facts,
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    params.todo[0].action = TodoAction::Edit;
    let entry = match try_run(&repo, &Operation::Rewrite(params), NOW) {
        Err(ApplyError::Conflicted { entry, .. }) => entry.id,
        other => panic!(
            "expected a stop: {:?}",
            other.map(|applied| applied.entry.status)
        ),
    };
    let saved = format!("rewrite/{}/plan.json", entry.as_str());
    let plan_bytes = repo.read(&format!(".git/graft/{saved}"));

    assert!(matches!(
        settle(&repo).outcome,
        SettleOutcome::Migrated { .. }
    ));
    assert_eq!(
        repo.read(&format!(".git/{}/{saved}", Generation::CURRENT.dir())),
        plan_bytes,
        "listed"
    );

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 3, &Never);
    let finished = resume(&mut ctx, Resume::Continue { solved: Vec::new() })
        .unwrap_or_else(|why| panic!("c continues: {}", why.explain(&crate::support::Letters)));
    assert_eq!(finished.entry.id, entry, "the paused rewrite resumed");
    assert!(!repo.exists(".git/rebase-merge"));
    assert!(
        !repo.exists(&format!(
            ".git/{}/rewrite/{}",
            Generation::CURRENT.dir(),
            entry.as_str()
        )),
        "the finished rewrite cleared its plan from the new folder"
    );
    assert_eq!(repo.git(&["log", "--format=%s", "-2"]), "a2\na1");
}
