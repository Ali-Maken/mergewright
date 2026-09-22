//! Table tests for the four properties: every kind's exact command lines,
//! every `Refusal` one fact away from a passing baseline.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::commit::Timestamp;
use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::run::GitArg;
use graft_git::stash::StashEntry;
use graft_git::state::InProgress;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_git::version::GitVersion;

use super::{
    CommitParams, Operation, PatchSide, Refusal, RepoFacts, StashParams, StashScope, Target,
};

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
        unmerged_how: None,
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
        remotes: vec![],
        upstream: None,
        refs: BTreeMap::new(),
        markers: vec![],
    }
}

fn paths(list: &[&str]) -> Target {
    Target::Paths(list.iter().map(PathBuf::from).collect())
}

fn patch_for(path: &str, side: PatchSide) -> Target {
    Target::Patch {
        path: PathBuf::from(path),
        patch: format!(
            "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-a\n+b\n"
        )
        .into_bytes(),
        side,
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
        ["git --literal-pathspecs add -- u.rs n.rs"]
    );
    assert_eq!(
        lines(&Operation::Unstage(paths(&["s.rs"])), &f),
        ["git --literal-pathspecs restore --staged -- s.rs"]
    );
    let stage_patch = Operation::Stage(patch_for("u.rs", PatchSide::Worktree));
    assert_eq!(lines(&stage_patch, &f), ["git apply --cached -"]);
    assert!(stdin(&stage_patch, &f).starts_with(b"diff --git a/u.rs"));
    assert_eq!(
        lines(&Operation::Unstage(patch_for("s.rs", PatchSide::Index)), &f),
        ["git apply --cached -R -"]
    );
    assert_eq!(
        lines(
            &Operation::Discard(patch_for("u.rs", PatchSide::Worktree)),
            &f
        ),
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
        ["git --literal-pathspecs rm --cached --quiet -- s.rs"]
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
        ["git --literal-pathspecs restore --staged -- new.rs old.rs"]
    );
    assert_eq!(
        lines(&Operation::Discard(paths(&["new.rs"])), &f),
        ["git --literal-pathspecs restore --staged --worktree --source=HEAD -- new.rs old.rs"]
    );
}

#[test]
fn plan_discard_is_one_invocation_per_class() {
    let f = facts();
    assert_eq!(
        lines(&Operation::Discard(paths(&["s.rs", "u.rs", "n.rs"])), &f),
        [
            "git --literal-pathspecs restore --staged --worktree --source=HEAD -- s.rs",
            "git --literal-pathspecs restore --worktree -- u.rs",
            "git --literal-pathspecs clean -f -- n.rs",
        ]
    );
    assert_eq!(
        lines(&Operation::Discard(paths(&["u.rs"])), &f),
        ["git --literal-pathspecs restore --worktree -- u.rs"]
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
            "git --literal-pathspecs rm --cached --quiet -- s.rs",
            "git --literal-pathspecs clean -f -- s.rs n.rs"
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
    let push = |message: Option<&str>, scope| {
        Operation::StashPush(StashParams {
            message: message.map(str::to_owned),
            scope,
        })
    };
    let untracked_too = StashScope::All {
        include_untracked: true,
    };
    assert_eq!(
        lines(&push(None, StashScope::default()), &f),
        ["git stash push"]
    );
    assert_eq!(
        lines(&push(Some("wip"), untracked_too), &f),
        ["git stash push -u -m wip"]
    );
    assert_eq!(
        lines(&push(Some("two words"), StashScope::StagedOnly), &f),
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
        Operation::Stage(patch_for("u.rs", PatchSide::Worktree)),
        Operation::Unstage(paths(&["s.rs"])),
        Operation::Unstage(patch_for("s.rs", PatchSide::Index)),
        Operation::Discard(paths(&["s.rs", "u.rs", "n.rs"])),
        Operation::Discard(patch_for("u.rs", PatchSide::Worktree)),
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
    assert_eq!(
        Operation::Stage(patch_for("n.rs", PatchSide::Worktree)).check(&f),
        wrong("n.rs")
    );
    assert_eq!(
        Operation::Unstage(patch_for("u.rs", PatchSide::Worktree)).check(&f),
        wrong("u.rs")
    );
    assert_eq!(
        Operation::Discard(patch_for("s.rs", PatchSide::Worktree)).check(&f),
        wrong("s.rs")
    );
    // A hunk from the staged diff of a staged file is discardable.
    assert_eq!(
        Operation::Discard(patch_for("s.rs", PatchSide::Index)).check(&f),
        Ok(())
    );
    assert_eq!(
        Operation::Discard(patch_for("u.rs", PatchSide::Index)).check(&f),
        wrong("u.rs")
    );
}

#[test]
fn a_hunk_from_the_other_diff_is_refused_for_stage_and_unstage() {
    let f = facts();
    // `u.rs` has an unstaged change, so the path is in state; the side is not.
    let mut both = facts();
    both.status.entries.push(StatusEntry {
        staged: Some(Change::Modified),
        ..unstaged("both.rs")
    });
    assert_eq!(
        Operation::Stage(patch_for("both.rs", PatchSide::Index)).check(&both),
        Err(Refusal::PatchSideMismatch)
    );
    assert_eq!(
        Operation::Unstage(patch_for("both.rs", PatchSide::Worktree)).check(&both),
        Err(Refusal::PatchSideMismatch)
    );
    assert_eq!(
        Operation::Stage(patch_for("u.rs", PatchSide::Worktree)).check(&f),
        Ok(())
    );
}

#[test]
fn discarding_an_index_hunk_reverses_it_in_the_working_tree_then_the_index() {
    let f = facts();
    assert_eq!(
        lines(&Operation::Discard(patch_for("s.rs", PatchSide::Index)), &f),
        ["git apply -R -", "git apply --cached -R -"]
    );
}

#[test]
fn paths_are_literal_pathspecs_so_a_star_names_one_file() {
    let mut f = facts();
    f.status.entries.push(unstaged("a*b"));
    let plan = Operation::Stage(paths(&["a*b"])).plan(&f);
    assert_eq!(plan[0].args[0], GitArg::Flag("--literal-pathspecs"));
    assert_eq!(
        lines(&Operation::Stage(paths(&["a*b"])), &f),
        ["git --literal-pathspecs add -- 'a*b'"]
    );
}

#[test]
fn a_patch_for_another_file_is_refused() {
    let f = facts();
    let Target::Patch { patch, .. } = patch_for("other.rs", PatchSide::Worktree) else {
        unreachable!()
    };
    let target = Target::Patch {
        path: PathBuf::from("u.rs"),
        patch,
        side: PatchSide::Worktree,
    };
    assert_eq!(
        Operation::Stage(target).check(&f),
        Err(Refusal::PatchPathMismatch)
    );
    let new_file = Target::Patch {
        path: PathBuf::from("u.rs"),
        patch: b"--- /dev/null\n+++ b/u.rs\n@@ -0,0 +1 @@\n+a\n".to_vec(),
        side: PatchSide::Worktree,
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
    let push = |scope| {
        Operation::StashPush(StashParams {
            message: None,
            scope,
        })
    };
    let untracked_too = StashScope::All {
        include_untracked: true,
    };
    assert_eq!(
        push(StashScope::default()).check(&only_untracked),
        Err(Refusal::NothingToStash)
    );
    assert_eq!(push(untracked_too).check(&only_untracked), Ok(()));
    // `--staged` with nothing staged: git says `No staged changes`, exit 1.
    let mut nothing_staged = facts();
    nothing_staged.status.entries.retain(|e| !e.is_staged());
    assert_eq!(
        push(StashScope::StagedOnly).check(&nothing_staged),
        Err(Refusal::NothingToStash)
    );
    assert_eq!(push(StashScope::default()).check(&nothing_staged), Ok(()));
    // No commit yet: `You do not have the initial commit yet`, exit 1.
    let unborn = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(
        push(StashScope::default()).check(&unborn),
        Err(Refusal::UnbornHead)
    );
    let old_git = RepoFacts {
        git: GitVersion {
            major: 2,
            minor: 34,
            patch: 9,
        },
        ..facts()
    };
    assert_eq!(
        push(StashScope::StagedOnly).check(&old_git),
        Err(Refusal::NeedsGit {
            feature: "stash --staged",
            minimum: GitVersion {
                major: 2,
                minor: 35,
                patch: 0
            }
        })
    );
    assert_eq!(push(StashScope::StagedOnly).check(&facts()), Ok(()));
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
            Operation::Unstage(patch_for("src/a.rs", PatchSide::Index)),
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
        Refusal::PatchSideMismatch,
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

// ---- the conflict operations (m6-conflicts/03) -----------------------------

use graft_git::status::UnmergedHow;

use super::{ResolveWholeParams, RestartParams, Side};

/// Facts with one unmerged path of the given shape, stopped on a merge.
fn conflicted(path: &str, how: UnmergedHow) -> RepoFacts {
    RepoFacts {
        status: WorkingTreeStatus {
            entries: vec![StatusEntry {
                unmerged: true,
                unmerged_how: Some(how),
                ..entry(path)
            }],
        },
        in_progress: Some(InProgress::Merge),
        ..facts()
    }
}

fn resolve(path: &str, side: Side, how: UnmergedHow) -> Operation {
    Operation::ResolveWholeFile(ResolveWholeParams {
        path: PathBuf::from(path),
        side,
        how,
    })
}

fn restart(path: &str) -> Operation {
    Operation::RestartConflict(RestartParams {
        path: PathBuf::from(path),
    })
}

/// The table in `operation/conflict.rs`, as command lines. Every row was run
/// against git 2.54 in a throwaway stopped merge before it was written down;
/// `tests/it/conflict_ops.rs` re-runs each one against real git and compares
/// the result with git's own commands.
#[test]
fn plan_whole_file_resolution_per_shape() {
    let checkout_ours =
        "git --literal-pathspecs checkout --ours -- p.rs|git --literal-pathspecs add -- p.rs";
    let checkout_theirs =
        "git --literal-pathspecs checkout --theirs -- p.rs|git --literal-pathspecs add -- p.rs";
    let add = "git --literal-pathspecs add -- p.rs";
    let remove = "git --literal-pathspecs rm --quiet -- p.rs";
    let rows: [(UnmergedHow, Side, &str); 14] = [
        (UnmergedHow::BothModified, Side::Ours, checkout_ours),
        (UnmergedHow::BothModified, Side::Theirs, checkout_theirs),
        (UnmergedHow::BothAdded, Side::Ours, checkout_ours),
        (UnmergedHow::BothAdded, Side::Theirs, checkout_theirs),
        (UnmergedHow::DeletedByThem, Side::Ours, add),
        (UnmergedHow::DeletedByThem, Side::Theirs, remove),
        (UnmergedHow::DeletedByUs, Side::Theirs, add),
        (UnmergedHow::DeletedByUs, Side::Ours, remove),
        (UnmergedHow::AddedByUs, Side::Ours, add),
        (UnmergedHow::AddedByUs, Side::Theirs, remove),
        (UnmergedHow::AddedByThem, Side::Theirs, add),
        (UnmergedHow::AddedByThem, Side::Ours, remove),
        (UnmergedHow::BothDeleted, Side::Ours, remove),
        (UnmergedHow::BothDeleted, Side::Theirs, remove),
    ];
    for (how, side, expected) in rows {
        let f = conflicted("p.rs", how);
        assert_eq!(
            lines(&resolve("p.rs", side, how), &f).join("|"),
            expected,
            "{how:?} {side:?}"
        );
    }
}

#[test]
fn plan_restart_is_checkout_merge() {
    let f = conflicted("p.rs", UnmergedHow::BothModified);
    assert_eq!(
        lines(&restart("p.rs"), &f),
        ["git --literal-pathspecs checkout --merge -- p.rs"]
    );
}

/// Both run inside a live merge — that is where a conflict lives — and both
/// refuse a path the status does not call unmerged.
#[test]
fn conflict_operations_check_against_the_status() {
    let f = conflicted("p.rs", UnmergedHow::DeletedByThem);
    assert_eq!(
        resolve("p.rs", Side::Ours, UnmergedHow::DeletedByThem).check(&f),
        Ok(())
    );
    // A restart is refused on a one-sided shape, not admitted: `checkout
    // --merge` has nothing to merge and exits 1 with "does not have all
    // necessary versions" (measured, git 2.54.0 — the table on
    // `conflict::check_restart`). This test used to pin `Ok(())` and so
    // pinned the raw git error (m6 g03 review, M1).
    assert_eq!(
        restart("p.rs").check(&f),
        Err(Refusal::NotMarkerShape {
            path: PathBuf::from("p.rs"),
            how: UnmergedHow::DeletedByThem,
        })
    );
    // The two shapes it does work on.
    for how in [UnmergedHow::BothModified, UnmergedHow::BothAdded] {
        assert_eq!(restart("p.rs").check(&conflicted("p.rs", how)), Ok(()));
    }
    // The shape the caller held is not the shape the status reads now: the
    // side they picked may not exist in the shape it became.
    assert_eq!(
        resolve("p.rs", Side::Ours, UnmergedHow::BothModified).check(&f),
        Err(Refusal::NotUnmerged {
            path: PathBuf::from("p.rs"),
        })
    );
    // A path with no unmerged entry at all.
    assert_eq!(
        resolve("other.rs", Side::Ours, UnmergedHow::BothModified).check(&f),
        Err(Refusal::NotUnmerged {
            path: PathBuf::from("other.rs"),
        })
    );
    assert_eq!(
        restart("other.rs").check(&f),
        Err(Refusal::NotUnmerged {
            path: PathBuf::from("other.rs"),
        })
    );
}

#[test]
fn conflict_operations_describe_and_record_their_kind() {
    let resolved = resolve("src/a.rs", Side::Ours, UnmergedHow::BothModified);
    assert_eq!(resolved.describe(), "resolved src/a.rs (ours)");
    assert_eq!(resolved.kind(), OperationKind::ResolveConflict);
    assert_eq!(
        resolve("src/a.rs", Side::Theirs, UnmergedHow::AddedByThem).describe(),
        "resolved src/a.rs (theirs)"
    );
    // `BothDeleted` has no side to name: both sides deleted it.
    assert_eq!(
        resolve("src/a.rs", Side::Ours, UnmergedHow::BothDeleted).describe(),
        "resolved src/a.rs (deleted)"
    );
    assert_eq!(restart("src/a.rs").describe(), "restarted src/a.rs");
    assert_eq!(restart("src/a.rs").kind(), OperationKind::RestartConflict);
}

#[test]
fn conflict_operations_touch_the_index_and_the_working_tree() {
    let f = conflicted("p.rs", UnmergedHow::BothModified);
    let expected = Touches {
        refs: vec![],
        index: true,
        worktree: true,
    };
    assert_eq!(
        resolve("p.rs", Side::Ours, UnmergedHow::BothModified).touches(&f),
        expected
    );
    assert_eq!(restart("p.rs").touches(&f), expected);
}

#[test]
fn conflict_refusals_in_words() {
    assert_eq!(
        Refusal::NotUnmerged {
            path: PathBuf::from("src/a.rs"),
        }
        .explain(),
        "src/a.rs is not conflicted"
    );
    assert_eq!(
        Refusal::ConflictsOpen {
            path: PathBuf::from("src/a.rs"),
            open: 2,
        }
        .explain(),
        "2 conflicts still open in src/a.rs"
    );
    assert_eq!(
        Refusal::NotMarkerShape {
            path: PathBuf::from("src/a.rs"),
            how: UnmergedHow::DeletedByThem,
        }
        .explain(),
        "src/a.rs: deleted by them — no markers to take from"
    );
}
