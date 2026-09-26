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
        // `Unmerged` left this list in m6-conflicts/72: its words are the
        // M6 line (`<path> +N still has conflicts · resolve it first`),
        // pinned by `the_unmerged_refusal_names_no_shell_and_no_milestone`.
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

use super::{ResolveWholeParams, RestartKind, RestartParams, Side};

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
        kind: RestartKind::Markers,
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
    // A restart is the one conflict operation that does **not** refuse here
    // for a path the status leaves out. After `⏎` the path is at stage 0 and
    // has no `UnmergedHow` to read at all, and whether its sides were kept is
    // a question only a spawn can answer — so `check` admits it and
    // `apply::prepare_restart` decides, with the same marker gate, against
    // the stages it finds (m6-conflicts/08 scope 5(c)).
    assert_eq!(restart("other.rs").check(&f), Ok(()));
}

/// The stage set, read back as the shape git's `XY` pair would have printed —
/// the only way the marker gate can be asked of stages that come out of a
/// recorded tree rather than out of the status.
#[test]
fn a_stage_set_spells_the_shape_it_came_from() {
    use super::conflict::shape_of_stages;
    use crate::snapshot::StageEntry;

    let at = |stages: &[u8]| -> Vec<StageEntry> {
        stages
            .iter()
            .map(|stage| StageEntry {
                mode: 0o100_644,
                oid: graft_core::oid::Oid::from_hex(&"a".repeat(40)).expect("an oid"),
                stage: *stage,
            })
            .collect()
    };
    assert_eq!(
        shape_of_stages(&at(&[1, 2, 3])),
        Some(UnmergedHow::BothModified)
    );
    assert_eq!(shape_of_stages(&at(&[2, 3])), Some(UnmergedHow::BothAdded));
    assert_eq!(
        shape_of_stages(&at(&[1, 2])),
        Some(UnmergedHow::DeletedByThem)
    );
    assert_eq!(
        shape_of_stages(&at(&[1, 3])),
        Some(UnmergedHow::DeletedByUs)
    );
    assert_eq!(shape_of_stages(&at(&[2])), Some(UnmergedHow::AddedByUs));
    assert_eq!(shape_of_stages(&at(&[3])), Some(UnmergedHow::AddedByThem));
    assert_eq!(shape_of_stages(&at(&[1])), Some(UnmergedHow::BothDeleted));
    assert_eq!(shape_of_stages(&at(&[])), None);
    // Exactly the two `checkout --merge` can merge are the two the gate lets
    // through, wherever the stages came from.
    for stages in [vec![1, 2, 3], vec![2, 3]] {
        assert!(
            shape_of_stages(&at(&stages))
                .expect("a shape")
                .has_markers(),
            "{stages:?} should be a marker shape"
        );
    }
    for stages in [vec![1, 2], vec![1, 3], vec![2], vec![3], vec![1]] {
        assert!(
            !shape_of_stages(&at(&stages))
                .expect("a shape")
                .has_markers(),
            "{stages:?} should not be a marker shape"
        );
    }
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
        Refusal::NoStagesKept {
            path: PathBuf::from("src/a.rs"),
        }
        .explain(),
        "src/a.rs: the conflict's sides were not kept"
    );
    assert_eq!(
        Refusal::ConflictsOpen {
            path: PathBuf::from("src/a.rs"),
            open: 2,
        }
        .explain(),
        "2 conflicts still open in src/a.rs"
    );
    // m6-conflicts/46: one open block reads as one, not "1 conflicts".
    assert_eq!(
        Refusal::ConflictsOpen {
            path: PathBuf::from("both.txt"),
            open: 1,
        }
        .explain(),
        "1 conflict still open in both.txt"
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

/// ADR-0045's words for `Unmerged`: what to do, in Graft. Before
/// m6-conflicts/72 they sent the user to the shell "until a later
/// milestone" — the milestone that resolves conflicts in Graft.
#[test]
fn the_unmerged_refusal_names_no_shell_and_no_milestone() {
    let one = Refusal::Unmerged {
        paths: vec![PathBuf::from("src/a.rs")],
    };
    assert_eq!(
        one.explain(),
        "src/a.rs still has conflicts · resolve it first"
    );
    let three = Refusal::Unmerged {
        paths: vec![
            PathBuf::from("src/a.rs"),
            PathBuf::from("b.rs"),
            PathBuf::from("c.rs"),
        ],
    };
    assert_eq!(
        three.explain(),
        "src/a.rs +2 still has conflicts · resolve it first"
    );
    for words in [one.explain(), three.explain()] {
        assert!(!words.contains("shell"), "{words}");
        assert!(!words.contains("milestone"), "{words}");
    }
}

// ---- moves_head -------------------------------------------------------------

fn tip(name: &str, byte: u8) -> super::Tip {
    super::Tip {
        name: name.to_owned(),
        oid: oid(byte),
    }
}

/// Every write that is not a history or remote kind: none moves `HEAD` but
/// the two commits.
#[expect(
    clippy::too_many_lines,
    reason = "one row per variant, as a table: split, the rows no longer read as one list"
)]
fn working_tree_cases() -> Vec<(Operation, bool)> {
    let commit = CommitParams {
        message: "m".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
    };
    let paths = || Target::Paths(vec![PathBuf::from("a")]);
    vec![
        (Operation::Stage(paths()), false),
        (Operation::Unstage(paths()), false),
        (Operation::Discard(paths()), false),
        (Operation::Commit(commit.clone()), true),
        (Operation::Amend(commit), true),
        (
            Operation::StashPush(StashParams {
                message: None,
                scope: StashScope::All {
                    include_untracked: false,
                },
            }),
            false,
        ),
        (
            Operation::StashApply {
                index: 0,
                commit: oid(1),
            },
            false,
        ),
        (
            Operation::StashPop {
                index: 0,
                commit: oid(1),
            },
            false,
        ),
        (
            Operation::StashDrop {
                index: 0,
                commit: oid(1),
            },
            false,
        ),
        (Operation::DropPutBack { stash: oid(1) }, false),
        (
            resolve("p.rs", Side::Ours, UnmergedHow::DeletedByThem),
            false,
        ),
        (restart("p.rs"), false),
        (
            Operation::FinishMerge(super::FinishMergeParams {
                merge_head: oid(0x0b),
                message: None,
            }),
            true,
        ),
        // `merge --abort` puts the tree back to `HEAD`; `HEAD` stays.
        (
            Operation::AbortMerge(super::AbortMergeParams {
                merge_head: oid(0x0b),
            }),
            false,
        ),
        (
            Operation::Recover(super::RecoverParams {
                target: super::RecoverTarget::MoveRef {
                    name: "main".to_owned(),
                    to: oid(0x0a),
                    expected: oid(0x0b),
                },
                because: "the shell's reset".to_owned(),
            }),
            false,
        ),
    ]
}

fn remote_cases() -> Vec<(Operation, bool)> {
    vec![
        (
            Operation::Fetch(super::FetchParams {
                remote: None,
                prune: false,
            }),
            false,
        ),
        (
            Operation::Pull(super::PullParams {
                branch: "refs/heads/main".to_owned(),
                upstream: graft_git::remote::TrackedUpstream {
                    remote: "origin".to_owned(),
                    remote_branch: "refs/heads/main".to_owned(),
                    tracking_ref: "refs/remotes/origin/main".to_owned(),
                    tracking_tip: Some(oid(7)),
                },
            }),
            true,
        ),
        (
            Operation::Push(super::PushParams {
                branch: "refs/heads/main".to_owned(),
                remote: "origin".to_owned(),
                remote_branch: "refs/heads/main".to_owned(),
                set_upstream: false,
                lease: None,
                source_commit: None,
            }),
            false,
        ),
    ]
}

/// The history kinds, both reset targets and a new branch both ways.
#[expect(
    clippy::too_many_lines,
    reason = "one row per variant, as a table: split, the rows no longer read as one list"
)]
fn history_cases() -> Vec<(Operation, bool)> {
    use super::{
        BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
        MergeParams, PickParams, RebaseParams, RefDeleteParams, ResetMode, ResetParams,
        ResetTarget, RevertParams, Source, TagCreateParams,
    };
    let main = || tip("refs/heads/main", 0x0a);
    let create = |checkout| {
        Operation::BranchCreate(BranchCreateParams {
            name: "nb".to_owned(),
            at: oid(0x0a),
            checkout,
        })
    };
    let reset = |target| {
        Operation::Reset(ResetParams {
            target,
            to: oid(0x0d),
        })
    };
    vec![
        (
            Operation::Merge(MergeParams {
                source: Source::Branch("refs/heads/feat".to_owned()),
                into: main(),
                source_tip: oid(0x0b),
                ff: FastForward::Default,
            }),
            true,
        ),
        (
            Operation::Rebase(RebaseParams {
                branch: main(),
                onto: Source::Branch("refs/heads/feat".to_owned()),
                onto_tip: oid(0x0b),
            }),
            true,
        ),
        (
            Operation::Rewrite(crate::rewrite::RewriteParams {
                head: main(),
                onto: oid(0x0b),
                todo: Vec::new(),
                line: crate::rewrite::BranchLine {
                    branches: vec![main()],
                },
                messages: BTreeMap::new(),
                made: Vec::new(),
                index_base: None,
            }),
            true,
        ),
        (
            Operation::CherryPick(PickParams {
                commits: vec![oid(0x0c)],
                subject: "s".to_owned(),
                head: main(),
            }),
            true,
        ),
        (
            Operation::Revert(RevertParams {
                commit: oid(0x0c),
                subject: "s".to_owned(),
                mainline: None,
                parents: 1,
                parent: Some(oid(0x0b)),
                head: main(),
            }),
            true,
        ),
        (
            reset(ResetTarget::Head {
                branch: main(),
                mode: ResetMode::Hard,
            }),
            true,
        ),
        // `branch -f` of a branch `HEAD` is not on: `HEAD` stays.
        (
            reset(ResetTarget::Branch(tip("refs/heads/feat", 0x0b))),
            false,
        ),
        (
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Branch("refs/heads/feat".to_owned()),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            }),
            true,
        ),
        (create(true), true),
        (create(false), false),
        (
            Operation::BranchDelete(RefDeleteParams {
                name: "refs/heads/feat".to_owned(),
                expected: oid(0x0b),
            }),
            false,
        ),
        (
            Operation::TagCreate(TagCreateParams {
                name: "v1".to_owned(),
                at: oid(0x0a),
                message: None,
            }),
            false,
        ),
        (
            Operation::TagDelete(RefDeleteParams {
                name: "refs/tags/v1".to_owned(),
                expected: oid(0x0a),
            }),
            false,
        ),
        (
            Operation::BranchFromStash(BranchFromStashParams {
                name: "nb".to_owned(),
                index: 0,
                commit: oid(0x50),
            }),
            true,
        ),
    ]
}

/// Every variant, both reset targets, a new branch both ways: which finished
/// writes land the graph cursor on the new `HEAD` (m7-power-tools/03, UX
/// §3.1). `moves_head` has no `_` arm, so a new variant fails to compile
/// until it is added there; the count below makes it fail here until it is
/// added to this table too.
#[test]
fn moves_head_covers_every_operation() {
    let mut cases = working_tree_cases();
    cases.extend(remote_cases());
    cases.extend(history_cases());
    let kinds: std::collections::BTreeSet<_> = cases
        .iter()
        .map(|(op, _)| std::mem::discriminant(op))
        .map(|kind| format!("{kind:?}"))
        .collect();
    assert_eq!(kinds.len(), 30, "one row per variant");
    for (op, lands) in cases {
        assert_eq!(op.moves_head(), lands, "{}", op.describe());
    }
}

/// m7-power-tools/28a (ADR-0049): the four word rows, a mixed case, and one
/// `<doing>` per kind in the table.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one row per kind in ADR-0049's `<doing>` table, read top to bottom against it"
)]
fn in_the_way_words() {
    use super::{
        BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
        MergeParams, PickParams, RebaseParams, RecoverParams, RecoverTarget, ResetMode,
        ResetParams, ResetTarget, RevertParams, Source, Tip,
    };
    use crate::in_the_way::doing;

    let words = |ignored: &[&str], untracked: &[&str]| {
        Refusal::InTheWay {
            doing: "switching to other".to_owned(),
            ignored: ignored.iter().map(PathBuf::from).collect(),
            untracked: untracked.iter().map(PathBuf::from).collect(),
        }
        .explain()
    };
    assert_eq!(
        words(&["c.txt"], &[]),
        "c.txt (ignored) is in the way of switching to other · move it aside first"
    );
    assert_eq!(
        words(&["c.txt", "d.txt", "e.txt"], &[]),
        "c.txt (ignored) +2 more are in the way of switching to other · move them aside first"
    );
    assert_eq!(
        words(&[], &["c.txt"]),
        "c.txt is in the way of switching to other · S stashes it first"
    );
    assert_eq!(
        words(&[], &["c.txt", "d.txt"]),
        "c.txt +1 more are in the way of switching to other · S stashes them first"
    );
    // Mixed: the ignored path names the line and decides the next step;
    // `+N` counts the untracked ones too.
    assert_eq!(
        words(&["z.env"], &["a.txt", "b.txt"]),
        "z.env (ignored) +2 more are in the way of switching to other · move them aside first"
    );

    let tip = |name: &str| Tip {
        name: name.to_owned(),
        oid: oid(1),
    };
    let sha = |byte: u8| oid(byte).to_string()[..7].to_owned();
    let cases: Vec<(Operation, Option<String>)> = vec![
        (
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Branch("refs/heads/other".to_owned()),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            }),
            Some("switching to other".to_owned()),
        ),
        (
            Operation::Checkout(CheckoutParams {
                to: CheckoutTarget::Detached(oid(0x0a)),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            }),
            Some(format!("switching to {}", sha(0x0a))),
        ),
        (
            Operation::BranchCreate(BranchCreateParams {
                name: "fresh".to_owned(),
                at: oid(2),
                checkout: true,
            }),
            Some("switching to fresh".to_owned()),
        ),
        (
            Operation::BranchCreate(BranchCreateParams {
                name: "fresh".to_owned(),
                at: oid(2),
                checkout: false,
            }),
            None,
        ),
        (
            Operation::BranchFromStash(BranchFromStashParams {
                name: "from-stash".to_owned(),
                index: 0,
                commit: oid(3),
            }),
            Some("switching to from-stash".to_owned()),
        ),
        (
            Operation::Merge(MergeParams {
                source: Source::Branch("refs/heads/other".to_owned()),
                into: tip("refs/heads/main"),
                source_tip: oid(4),
                ff: FastForward::Default,
            }),
            Some("merging other".to_owned()),
        ),
        (
            Operation::Merge(MergeParams {
                source: Source::Commit(oid(4)),
                into: tip("refs/heads/main"),
                source_tip: oid(4),
                ff: FastForward::Never,
            }),
            Some(format!("merging {}", sha(4))),
        ),
        (
            Operation::Pull(super::PullParams {
                branch: "refs/heads/main".to_owned(),
                upstream: graft_git::remote::TrackedUpstream {
                    remote: "origin".to_owned(),
                    remote_branch: "refs/heads/main".to_owned(),
                    tracking_ref: "refs/remotes/origin/main".to_owned(),
                    tracking_tip: Some(oid(7)),
                },
            }),
            Some("pulling origin/main".to_owned()),
        ),
        (
            Operation::Reset(ResetParams {
                target: ResetTarget::Head {
                    branch: tip("refs/heads/main"),
                    mode: ResetMode::Hard,
                },
                to: oid(5),
            }),
            Some(format!("resetting main to {}", sha(5))),
        ),
        (
            Operation::Reset(ResetParams {
                target: ResetTarget::Head {
                    branch: tip("refs/heads/main"),
                    mode: ResetMode::Mixed,
                },
                to: oid(5),
            }),
            None,
        ),
        (
            Operation::Recover(RecoverParams {
                target: RecoverTarget::MoveRef {
                    name: "main".to_owned(),
                    to: oid(6),
                    expected: oid(1),
                },
                because: "the shell's reset".to_owned(),
            }),
            Some(format!("putting main back to {}", sha(6))),
        ),
        (
            Operation::Rebase(RebaseParams {
                branch: tip("refs/heads/main"),
                onto: Source::Branch("refs/heads/feat".to_owned()),
                onto_tip: oid(0x0b),
            }),
            Some("rebasing onto feat".to_owned()),
        ),
        (
            Operation::Rebase(RebaseParams {
                branch: tip("refs/heads/main"),
                onto: Source::Commit(oid(0x0b)),
                onto_tip: oid(0x0b),
            }),
            Some(format!("rebasing onto {}", sha(0x0b))),
        ),
        (
            Operation::Rewrite(crate::rewrite::RewriteParams {
                head: tip("refs/heads/main"),
                onto: oid(0x0b),
                todo: Vec::new(),
                line: crate::rewrite::BranchLine {
                    branches: vec![tip("refs/heads/main")],
                },
                messages: BTreeMap::new(),
                made: Vec::new(),
                index_base: None,
            }),
            Some("rewriting main".to_owned()),
        ),
        (
            Operation::CherryPick(PickParams {
                commits: vec![oid(0x0c)],
                subject: "s".to_owned(),
                head: tip("refs/heads/main"),
            }),
            Some(format!("picking {}", sha(0x0c))),
        ),
        (
            Operation::CherryPick(PickParams {
                commits: vec![oid(0x0c), oid(0x0d)],
                subject: "s".to_owned(),
                head: tip("refs/heads/main"),
            }),
            Some("picking 2 commits".to_owned()),
        ),
        (
            Operation::Revert(RevertParams {
                commit: oid(0x0e),
                subject: "s".to_owned(),
                mainline: None,
                parents: 1,
                parent: None,
                head: tip("refs/heads/main"),
            }),
            Some(format!("reverting {}", sha(0x0e))),
        ),
        (
            Operation::StashApply {
                index: 0,
                commit: oid(3),
            },
            None,
        ),
    ];
    for (op, expected) in cases {
        assert_eq!(doing(&op), expected, "{}", op.describe());
    }
}

/// Review 28 m4: a stash put-back's refusal with an ignored file in it
/// reads `(ignored)` and `move it aside first`, as every other kind does;
/// without one, the stash preview's words are unchanged.
#[test]
fn stash_in_the_way_words_name_an_ignored_file() {
    let paths = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
    let words = |unsaved: &[&str], untracked: &[&str], ignored: &[&str]| {
        Refusal::StashInTheWay {
            unsaved: paths(unsaved),
            staged: Vec::new(),
            untracked: paths(untracked),
            ignored: paths(ignored),
        }
        .explain()
    };
    let cases = [
        (words(&[], &["n.txt"], &[]), "n.txt is in the way"),
        (
            words(&[], &[], &["c.txt"]),
            "c.txt (ignored) is in the way · move it aside first",
        ),
        (
            words(&[], &["n.txt"], &["c.txt", "d.txt"]),
            "c.txt (ignored) +2 more are in the way · move them aside first",
        ),
        (
            words(&["a.txt"], &[], &["c.txt"]),
            "your unsaved edits to a.txt and c.txt (ignored) are in the way · move it aside first",
        ),
        (
            words(&["a.txt"], &["n.txt"], &[]),
            "your unsaved edits to a.txt and n.txt are in the way",
        ),
    ];
    for (seen, wanted) in cases {
        assert_eq!(seen, wanted);
    }
}
