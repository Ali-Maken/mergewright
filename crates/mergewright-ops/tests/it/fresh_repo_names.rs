//! A repository with no names marker is on the current names
//! (pre-publish-cleanup/11b2): its journal and backups land under
//! `on_disk::DIR` and `on_disk::REF_PREFIX`, and no other app folder appears.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::oid::Oid;
use mergewright_core::on_disk::{DIR, REF_PREFIX};
use mergewright_ops::operation::{FastForward, MergeParams, Operation, Source, Tip};

use crate::support::{TempRepo, run};

#[test]
fn journal_and_backups_use_current_names() {
    let repo = TempRepo::init();
    // No marker of any generation: the folder `init` made goes.
    std::fs::remove_dir_all(repo.path().join(".git").join(DIR)).expect("marker dir removed");
    repo.write("a.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("b.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("c.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);

    let oid = |name: &str| Oid::from_hex(&repo.git(&["rev-parse", name])).expect("an oid");
    let op = Operation::Merge(MergeParams {
        source: Source::Branch("refs/heads/feature".to_owned()),
        into: Tip {
            name: "refs/heads/main".to_owned(),
            oid: oid("refs/heads/main"),
        },
        source_tip: oid("refs/heads/feature"),
        ff: FastForward::Never,
    });
    let entry = run(&repo, &op, 1_000);

    let backup = entry.backup_ref.expect("the merge took a backup");
    assert!(backup.starts_with(REF_PREFIX), "{backup}");
    assert!(
        repo.exists(&format!(".git/{DIR}/undo-journal.jsonl")),
        "journal under .git/{DIR}/"
    );
    let refs = repo.git(&["for-each-ref", "--format=%(refname)"]);
    for name in refs.lines() {
        assert!(
            name.starts_with("refs/heads/") || name.starts_with(REF_PREFIX),
            "a ref outside the current prefix: {name}"
        );
    }
    let git_dir = repo.path().join(".git");
    let known = ["objects", "refs", "logs", "hooks", "info", "branches", DIR];
    for entry in std::fs::read_dir(&git_dir).expect("reads") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            assert!(
                known.contains(&name.as_str()),
                "an app dir other than {DIR}: {name}"
            );
        }
    }
}
