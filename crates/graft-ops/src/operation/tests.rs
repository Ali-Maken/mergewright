//! Table tests for the four properties: every kind's exact command lines,
//! every `Refusal` one fact away from a passing baseline.

use std::path::{Path, PathBuf};

use graft_core::commit::Timestamp;
use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::stash::StashEntry;
use graft_git::state::InProgress;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_git::version::GitVersion;

use super::{CommitParams, Operation, Refusal, RepoFacts, StashParams, Target};

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).unwrap()
}

fn entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: PathBuf::from(path),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: false,
        submodule: false,
    }
}

fn staged(path: &str) -> StatusEntry {
    StatusEntry {
        staged: Some(Change::Modified),
        ..entry(path)
    }
}

fn unstaged(path: &str) -> StatusEntry {
    StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry(path)
    }
}

fn untracked(path: &str) -> StatusEntry {
    StatusEntry {
        untracked: true,
        ..entry(path)
    }
}

fn stash(index: usize, paths: &[&str]) -> StashEntry {
    StashEntry {
        index,
        commit: oid(0x10 + u8::try_from(index).unwrap()),
        message: format!("On main: wip {index}"),
        at: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
        paths: paths.iter().map(PathBuf::from).collect(),
    }
}

/// A born repository on `main` with one of each change and one stash.
fn facts() -> RepoFacts {
    RepoFacts {
        bare: false,
        head: HeadRef::Branch("refs/heads/main".to_owned()),
        in_progress: None,
        status: WorkingTreeStatus {
            entries: vec![staged("s.rs"), unstaged("u.rs"), untracked("n.rs")],
        },
        stashes: vec![stash(0, &["stashed.rs"])],
        git: GitVersion {
            major: 2,
            minor: 40,
            patch: 0,
        },
    }
}

fn paths(list: &[&str]) -> Target {
    Target::Paths(list.iter().map(PathBuf::from).collect())
}

fn patch_for(path: &str) -> Target {
    Target::Patch {
        path: PathBuf::from(path),
        patch: format!(
            "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-a\n+b\n"
        )
        .into_bytes(),
    }
}

fn commit(message: &str) -> CommitParams {
    CommitParams {
        message: message.to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
    }
}

fn lines(op: &Operation, facts: &RepoFacts) -> Vec<String> {
    op.plan(facts)
        .iter()
        .map(super::Invocation::command_line)
        .collect()
}

fn stdin(op: &Operation, facts: &RepoFacts) -> Vec<u8> {
    op.plan(facts).remove(0).stdin.unwrap()
}

// ---- plan -----------------------------------------------------------------

#[test]
fn plan_stage_and_unstage_paths_and_patches() {
    let f = facts();
    assert_eq!(
        lines(&Operation::Stage(paths(&["u.rs", "n.rs"])), &f),
        ["git add -- u.rs n.rs"]
    );
    assert_eq!(
        lines(&Operation::Unstage(paths(&["s.rs"])), &f),
        ["git restore --staged -- s.rs"]
    );
    let stage_patch = Operation::Stage(patch_for("u.rs"));
    assert_eq!(lines(&stage_patch, &f), ["git apply --cached -"]);
    assert!(stdin(&stage_patch, &f).starts_with(b"diff --git a/u.rs"));
    assert_eq!(
        lines(&Operation::Unstage(patch_for("s.rs")), &f),
        ["git apply --cached -R -"]
    );
    assert_eq!(
        lines(&Operation::Discard(patch_for("u.rs")), &f),
        ["git apply -R -"]
    );
}

#[test]
fn plan_unstage_on_an_unborn_head_removes_from_the_index() {
    let f = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(
        lines(&Operation::Unstage(paths(&["s.rs"])), &f),
        ["git rm --cached --quiet -- s.rs"]
    );
}

#[test]
fn plan_stage_and_unstage_of_a_rename_name_both_paths() {
    let mut f = facts();
    f.status.entries.push(StatusEntry {
        original_path: Some(PathBuf::from("old.rs")),
        staged: Some(Change::Renamed),
        ..entry("new.rs")
    });
    assert_eq!(
        lines(&Operation::Unstage(paths(&["new.rs"])), &f),
        ["git restore --staged -- new.rs old.rs"]
    );
    assert_eq!(
        lines(&Operation::Discard(paths(&["new.rs"])), &f),
        ["git restore --staged --worktree --source=HEAD -- new.rs old.rs"]
    );
}

#[test]
fn plan_discard_is_one_invocation_per_class() {
    let f = facts();
    assert_eq!(
        lines(&Operation::Discard(paths(&["s.rs", "u.rs", "n.rs"])), &f),
        [
            "git restore --staged --worktree --source=HEAD -- s.rs",
            "git restore --worktree -- u.rs",
            "git clean -f -- n.rs",
        ]
    );
    assert_eq!(
        lines(&Operation::Discard(paths(&["u.rs"])), &f),
        ["git restore --worktree -- u.rs"]
    );
}

#[test]
fn plan_discard_on_an_unborn_head_has_no_head_to_restore_from() {
    let f = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(
        lines(&Operation::Discard(paths(&["s.rs", "n.rs"])), &f),
        [
            "git rm --cached --quiet -- s.rs",
            "git clean -f -- s.rs n.rs"
        ]
    );
}

#[test]
fn plan_commit_and_amend_feed_the_message_on_stdin_never_dash_m() {
    let f = facts();
    let plain = Operation::Commit(commit("subject\r\n\r\nbody\n\n"));
    assert_eq!(lines(&plain, &f), ["git commit -F -"]);
    assert_eq!(stdin(&plain, &f), b"subject\n\nbody\n");
    let flagged = Operation::Commit(CommitParams {
        sign_off: true,
        no_verify: true,
        allow_empty: true,
        ..commit("s")
    });
    assert_eq!(
        lines(&flagged, &f),
        ["git commit -F - --signoff --no-verify --allow-empty"]
    );
    let amend = Operation::Amend(CommitParams {
        sign_off: true,
        ..commit("s")
    });
    assert_eq!(lines(&amend, &f), ["git commit -F - --amend --signoff"]);
    assert!(lines(&flagged, &f).iter().all(|line| !line.contains("-m")));
}

#[test]
fn plan_stash_kinds() {
    let f = facts();
    let push = |message: Option<&str>, include_untracked, staged_only| {
        Operation::StashPush(StashParams {
            message: message.map(str::to_owned),
            include_untracked,
            staged_only,
        })
    };
    assert_eq!(lines(&push(None, false, false), &f), ["git stash push"]);
    assert_eq!(
        lines(&push(Some("wip"), true, false), &f),
        ["git stash push -u -m wip"]
    );
    assert_eq!(
        lines(&push(Some("two words"), false, true), &f),
        ["git stash push --staged -m 'two words'"]
    );
    let at = |index| (index, oid(0x10 + u8::try_from(index).unwrap()));
    let (index, commit) = at(0);
    assert_eq!(
        lines(&Operation::StashApply { index, commit }, &f),
        ["git stash apply --index 'stash@{0}'"]
    );
    assert_eq!(
        lines(&Operation::StashPop { index, commit }, &f),
        ["git stash pop --index 'stash@{0}'"]
    );
    assert_eq!(
        lines(&Operation::StashDrop { index, commit }, &f),
        ["git stash drop 'stash@{0}'"]
    );
}

// ---- check ----------------------------------------------------------------

fn baseline_ops() -> Vec<Operation> {
    vec![
        Operation::Stage(paths(&["u.rs"])),
        Operation::Stage(paths(&["n.rs"])),
        Operation::Stage(patch_for("u.rs")),
        Operation::Unstage(paths(&["s.rs"])),
        Operation::Unstage(patch_for("s.rs")),
        Operation::Discard(paths(&["s.rs", "u.rs", "n.rs"])),
        Operation::Discard(patch_for("u.rs")),
        Operation::Commit(commit("fix")),
        Operation::Amend(commit("fix")),
        Operation::StashPush(StashParams::default()),
        Operation::StashApply {
            index: 0,
            commit: oid(0x10),
        },
        Operation::StashPop {
            index: 0,
            commit: oid(0x10),
        },
        Operation::StashDrop {
            index: 0,
            commit: oid(0x10),
        },
    ]
}

#[test]
fn every_kind_passes_the_baseline() {
    let f = facts();
    for op in baseline_ops() {
        assert_eq!(op.check(&f), Ok(()), "{}", op.describe());
    }
}

#[test]
fn bare_and_unmerged_refuse_everything() {
    let bare = RepoFacts {
        bare: true,
        ..facts()
    };
    let mut unmerged = facts();
    unmerged.status.entries.push(StatusEntry {
        unmerged: true,
        ..entry("c.rs")
    });
    for op in baseline_ops() {
        assert_eq!(op.check(&bare), Err(Refusal::Bare), "{}", op.describe());
        assert_eq!(
            op.check(&unmerged),
            Err(Refusal::Unmerged {
                paths: vec![PathBuf::from("c.rs")]
            }),
            "{}",
            op.describe()
        );
    }
}

#[test]
fn mid_rebase_stage_and_unstage_are_allowed_and_the_rest_wait() {
    let f = RepoFacts {
        in_progress: Some(InProgress::Rebase),
        ..facts()
    };
    for op in baseline_ops() {
        let allowed = matches!(op, Operation::Stage(_) | Operation::Unstage(_));
        let verdict = op.check(&f);
        if allowed {
            assert_eq!(verdict, Ok(()), "{}", op.describe());
        } else {
            assert_eq!(
                verdict,
                Err(Refusal::InProgress(InProgress::Rebase)),
                "{}",
                op.describe()
            );
        }
    }
    let explained = Refusal::InProgress(InProgress::Rebase).explain();
    assert!(
        explained.contains("git rebase --continue") && explained.contains("git rebase --abort")
    );
}

#[test]
fn a_path_in_the_wrong_state_is_refused_by_name() {
    let f = facts();
    let wrong = |path: &str| {
        Err(Refusal::PathNotInState {
            path: PathBuf::from(path),
        })
    };
    assert_eq!(Operation::Stage(paths(&["s.rs"])).check(&f), wrong("s.rs"));
    assert_eq!(
        Operation::Stage(paths(&["missing.rs"])).check(&f),
        wrong("missing.rs")
    );
    assert_eq!(
        Operation::Unstage(paths(&["u.rs"])).check(&f),
        wrong("u.rs")
    );
    assert_eq!(
        Operation::Discard(paths(&["clean.rs"])).check(&f),
        wrong("clean.rs")
    );
    // A patch needs a tracked file on the right side (ADR-0034: no hunk
    // staging inside untracked files).
    assert_eq!(Operation::Stage(patch_for("n.rs")).check(&f), wrong("n.rs"));
    assert_eq!(
        Operation::Unstage(patch_for("u.rs")).check(&f),
        wrong("u.rs")
    );
    assert_eq!(
        Operation::Discard(patch_for("s.rs")).check(&f),
        wrong("s.rs")
    );
}

#[test]
fn a_patch_for_another_file_is_refused() {
    let f = facts();
    let Target::Patch { patch, .. } = patch_for("other.rs") else {
        unreachable!()
    };
    let target = Target::Patch {
        path: PathBuf::from("u.rs"),
        patch,
    };
    assert_eq!(
        Operation::Stage(target).check(&f),
        Err(Refusal::PatchPathMismatch)
    );
    let new_file = Target::Patch {
        path: PathBuf::from("u.rs"),
        patch: b"--- /dev/null\n+++ b/u.rs\n@@ -0,0 +1 @@\n+a\n".to_vec(),
    };
    assert_eq!(Operation::Stage(new_file).check(&f), Ok(()));
}

#[test]
fn commit_refusals() {
    let f = facts();
    let mut nothing_staged = facts();
    nothing_staged.status.entries.retain(|e| !e.is_staged());
    assert_eq!(
        Operation::Commit(commit("fix")).check(&nothing_staged),
        Err(Refusal::NothingStaged)
    );
    let allow_empty = CommitParams {
        allow_empty: true,
        ..commit("fix")
    };
    assert_eq!(
        Operation::Commit(allow_empty).check(&nothing_staged),
        Ok(())
    );
    assert_eq!(
        Operation::Commit(commit("\n  \n")).check(&f),
        Err(Refusal::EmptySubject)
    );
    assert_eq!(
        Operation::Amend(commit("")).check(&f),
        Err(Refusal::EmptySubject)
    );
    // An amend with nothing staged is a reword.
    assert_eq!(
        Operation::Amend(commit("reword")).check(&nothing_staged),
        Ok(())
    );
    let unborn = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(
        Operation::Amend(commit("fix")).check(&unborn),
        Err(Refusal::UnbornHead)
    );
    assert_eq!(Operation::Commit(commit("first")).check(&unborn), Ok(()));
}

#[test]
fn stash_push_refusals() {
    let mut only_untracked = facts();
    only_untracked.status.entries.retain(|e| e.untracked);
    let push = |include_untracked, staged_only| {
        Operation::StashPush(StashParams {
            message: None,
            include_untracked,
            staged_only,
        })
    };
    assert_eq!(
        push(false, false).check(&only_untracked),
        Err(Refusal::NothingToStash)
    );
    assert_eq!(push(true, false).check(&only_untracked), Ok(()));
    let old_git = RepoFacts {
        git: GitVersion {
            major: 2,
            minor: 34,
            patch: 9,
        },
        ..facts()
    };
    assert_eq!(
        push(false, true).check(&old_git),
        Err(Refusal::NeedsGit {
            feature: "stash --staged",
            minimum: GitVersion {
                major: 2,
                minor: 35,
                patch: 0
            }
        })
    );
    assert_eq!(push(false, true).check(&facts()), Ok(()));
}

#[test]
fn stash_apply_pop_drop_refusals() {
    let f = facts();
    let apply = |index, commit| Operation::StashApply { index, commit };
    assert_eq!(apply(3, oid(0x13)).check(&f), Err(Refusal::NoSuchStash));
    assert_eq!(
        apply(0, oid(0x99)).check(&f),
        Err(Refusal::StashListChanged)
    );
    assert_eq!(
        Operation::StashDrop {
            index: 0,
            commit: oid(0x99)
        }
        .check(&f),
        Err(Refusal::StashListChanged)
    );
    // The stash touches a file that is dirty now: refused. Dirty elsewhere: fine.
    let mut dirty_there = facts();
    dirty_there.status.entries.push(unstaged("stashed.rs"));
    assert_eq!(
        apply(0, oid(0x10)).check(&dirty_there),
        Err(Refusal::TouchesChangedFiles {
            paths: vec![PathBuf::from("stashed.rs")]
        })
    );
    assert_eq!(
        Operation::StashPop {
            index: 0,
            commit: oid(0x10)
        }
        .check(&dirty_there),
        Err(Refusal::TouchesChangedFiles {
            paths: vec![PathBuf::from("stashed.rs")]
        })
    );
    assert_eq!(apply(0, oid(0x10)).check(&f), Ok(()));
    // Drop never looks at the working tree.
    assert_eq!(
        Operation::StashDrop {
            index: 0,
            commit: oid(0x10)
        }
        .check(&dirty_there),
        Ok(())
    );
}

// ---- touches, describe, kind, explain ------------------------------------

#[test]
fn touches_per_kind() {
    let f = facts();
    let t = |refs: &[&str], index, worktree| Touches {
        refs: refs.iter().map(|&r| r.to_owned()).collect(),
        index,
        worktree,
    };
    let ops = baseline_ops();
    let by_kind = |kind| ops.iter().find(|op| op.kind() == kind).unwrap().touches(&f);
    assert_eq!(by_kind(OperationKind::Stage), t(&[], true, false));
    assert_eq!(by_kind(OperationKind::Unstage), t(&[], true, false));
    assert_eq!(by_kind(OperationKind::Discard), t(&[], true, true));
    assert_eq!(
        by_kind(OperationKind::Commit),
        t(&["refs/heads/main"], true, false)
    );
    assert_eq!(
        by_kind(OperationKind::Amend),
        t(&["refs/heads/main"], true, false)
    );
    assert_eq!(
        by_kind(OperationKind::StashPush),
        t(&["refs/stash"], true, true)
    );
    assert_eq!(
        by_kind(OperationKind::StashApply),
        t(&["refs/stash"], true, true)
    );
    assert_eq!(
        by_kind(OperationKind::StashPop),
        t(&["refs/stash"], true, true)
    );
    assert_eq!(
        by_kind(OperationKind::StashDrop),
        t(&["refs/stash"], false, false)
    );
    let detached = RepoFacts {
        head: HeadRef::Detached(oid(1)),
        ..facts()
    };
    assert_eq!(
        Operation::Commit(commit("x")).touches(&detached).refs,
        ["HEAD"]
    );
    let unborn = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(
        Operation::Commit(commit("x")).touches(&unborn).refs,
        ["refs/heads/main"]
    );
}

#[test]
fn describe_reads_like_the_browser_line() {
    let cases = [
        (Operation::Stage(paths(&["src/a.rs"])), "stage src/a.rs"),
        (Operation::Stage(paths(&["a", "b", "c"])), "stage 3 files"),
        (
            Operation::Unstage(patch_for("src/a.rs")),
            "unstage a hunk of src/a.rs",
        ),
        (Operation::Discard(paths(&["src/a.rs"])), "discard src/a.rs"),
        (
            Operation::Commit(commit("\n\nfix it  \nbody")),
            "commit \"fix it\"",
        ),
        (Operation::Amend(commit("fix it")), "amend \"fix it\""),
        (
            Operation::StashPush(StashParams {
                message: Some("wip".to_owned()),
                ..StashParams::default()
            }),
            "stash \"wip\"",
        ),
        (
            Operation::StashPush(StashParams::default()),
            "stash the working tree",
        ),
        (
            Operation::StashApply {
                index: 0,
                commit: oid(1),
            },
            "apply stash@{0}",
        ),
        (
            Operation::StashPop {
                index: 2,
                commit: oid(1),
            },
            "pop stash@{2}",
        ),
        (
            Operation::StashDrop {
                index: 1,
                commit: oid(1),
            },
            "drop stash@{1}",
        ),
    ];
    for (op, text) in cases {
        assert_eq!(op.describe(), text);
    }
}

#[test]
fn every_refusal_explains_itself_in_one_actionable_sentence() {
    let all = [
        Refusal::Bare,
        Refusal::InProgress(InProgress::Merge),
        Refusal::Unmerged {
            paths: vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")],
        },
        Refusal::PathNotInState {
            path: PathBuf::from("a"),
        },
        Refusal::PatchPathMismatch,
        Refusal::NothingStaged,
        Refusal::EmptySubject,
        Refusal::UnbornHead,
        Refusal::NothingToStash,
        Refusal::NeedsGit {
            feature: "stash --staged",
            minimum: GitVersion {
                major: 2,
                minor: 35,
                patch: 0,
            },
        },
        Refusal::NoSuchStash,
        Refusal::StashListChanged,
        Refusal::TouchesChangedFiles {
            paths: vec![PathBuf::from("a")],
        },
    ];
    for refusal in all {
        let text = refusal.explain();
        assert!(text.ends_with('.'), "{text}");
        assert!(text.len() > 20 && !text.contains('\n'), "{text}");
    }
    assert!(
        Refusal::InProgress(InProgress::Merge)
            .explain()
            .contains("`git merge --abort`")
    );
    assert!(
        Refusal::Unmerged {
            paths: vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]
        }
        .explain()
        .contains("a and 2 more files")
    );
    assert!(
        Refusal::NeedsGit {
            feature: "stash --staged",
            minimum: GitVersion {
                major: 2,
                minor: 35,
                patch: 0
            }
        }
        .explain()
        .contains("2.35.0")
    );
}

#[test]
fn kind_matches_the_variant() {
    let kinds: Vec<OperationKind> = baseline_ops().iter().map(Operation::kind).collect();
    assert_eq!(
        kinds,
        [
            OperationKind::Stage,
            OperationKind::Stage,
            OperationKind::Stage,
            OperationKind::Unstage,
            OperationKind::Unstage,
            OperationKind::Discard,
            OperationKind::Discard,
            OperationKind::Commit,
            OperationKind::Amend,
            OperationKind::StashPush,
            OperationKind::StashApply,
            OperationKind::StashPop,
            OperationKind::StashDrop,
        ]
    );
    assert!(Path::new("u.rs").is_relative());
}
