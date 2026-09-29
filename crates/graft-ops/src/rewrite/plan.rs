//! What a rewrite request plans to — the range of his branch and the
//! refusals before anything runs. The line of branches built on his is
//! `dependents`'s; the history reads are `reads`'s.

use std::collections::BTreeMap;

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::version::GitVersion;

use crate::facts;
use crate::operation::{RebaseParams, Refusal, RepoFacts, RewriteKey, Tip};

use super::dependents::line_of;
use super::reads::{RangeCommit, ancestor, base_of, commit_index, parents_of, read_range};
use super::todo::autosquash;
use super::unpushed::{RangeStop, your_commits};
use super::{
    RewriteError, RewriteParams, RewriteRequest, TodoAction, TodoLine, default_message,
    squash_groups,
};

/// Plan a rewrite of the branch `HEAD` is on.
///
/// `Open { from }`: his rows run from `HEAD` down to where the branch left
/// its base — the merge-base with its upstream if one is set and fetched,
/// else with the default branch — and further down to `from` when `from`
/// is that deep or deeper. `fixup!`/`squash!` rows come back pre-marked
/// and moved above their target, never applied unseen; a squash group gets
/// its default message ([`default_message`]). The first commit of the
/// repository is never a row: it has no parent to replay onto, so a range
/// reaching it stops above it and it is the base.
///
/// `AmendInto { target }`: the index is written as a commit object with no
/// ref (parent `HEAD`) and planned as a fixup right after `target`; `made`
/// records it. On `HEAD` with nothing built on it this is no rewrite — the
/// app keeps its plain amend — and is refused.
///
/// `Absorb`: each staged hunk goes to the commit that last changed its lines
/// ([`crate::absorb`]), one commit object per target, each a fixup right
/// after its target; none placed is [`Refusal::NothingToAbsorb`].
///
/// `FromRebase(params)`: the rebase a preview drew, as its `y` would run it
/// — the rows it replays onto `onto_tip`, its drops marked `drop`, its line
/// as given. `HEAD` not on `params.branch` at its tip (moved or switched
/// since the preview), or a drop with no row, is [`Refusal::Moved`].
///
/// Absorb and `AmendInto` reach one range, on every base (06e ruling): his
/// first-parent commits down to the first a remote-tracking ref holds, the
/// base, a merge or the first commit — and, on the default or only branch
/// with no base below `HEAD`, at most the last
/// [`super::ABSORB_FALLBACK_DEPTH`]. `AmendInto` on `HEAD` skips it.
///
/// `default_branch` is the user's `graph.default_branch` when set: it wins
/// over git's guess (ADR-0001 rule 1), for the base of the range and for
/// which branch is never a dependent.
///
/// # Errors
/// [`RewriteError::Refused`], checked in this order: not on a branch;
/// nothing staged (`AmendInto`); the commit not on his branch, or past his
/// commits (already pushed, past the cap, below a merge); every commit
/// pushed, or none of its own (`Absorb`); the first
/// commit (`AmendInto`); a merge in the range; the branches built on his
/// forking, or one checked out in another worktree; git older than 2.38 with
/// branches to follow; `AmendInto` on `HEAD` with none. [`RewriteError::Facts`]
/// when a read fails; [`RewriteError::Stopped`].
pub fn plan(
    repo: &RepoHandle,
    git: &GitVersion,
    req: &RewriteRequest,
    facts: &RepoFacts,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    let head = head_of(repo, facts, stop)?;
    match req {
        RewriteRequest::Open { from } => open(repo, git, head, *from, default_branch, stop),
        RewriteRequest::AmendInto { target } => {
            amend_into(repo, git, head, *target, facts, default_branch, stop)
        }
        RewriteRequest::Absorb => super::absorb::absorb(repo, git, head, default_branch, stop),
        RewriteRequest::FromRebase(params) => from_rebase(repo, head, params, stop),
    }
}

/// The rebase `params` as an editor todo: exactly the rows `r`'s run
/// replays ([`super::rebase_rows`], as `apply::engine` reads them), every
/// one a pick but the suggested drops. No autosquash re-marking and no
/// range walk: unedited, the editor runs the rebase `y` would. The line is
/// the preview's, as-is — `plan_as_run` checked it, and `y`'s pin check
/// catches a move since.
fn from_rebase(
    repo: &RepoHandle,
    head: Tip,
    params: &RebaseParams,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    if head != params.branch {
        // HEAD moved or switched since the preview was drawn.
        return Err(RewriteError::Refused(Refusal::Moved {
            name: params.branch.name.clone(),
        }));
    }
    let mut todo = super::rebase_rows(repo, params.onto_tip, head.oid, stop)?;
    for dropped in &params.drops {
        let Some(row) = todo.iter_mut().find(|row| row.commit == *dropped) else {
            // A drop with no row: the branch is not what the preview
            // measured — as `y` refuses it (`apply::engine::prepare`).
            return Err(RewriteError::Refused(Refusal::Moved {
                name: params.branch.name.clone(),
            }));
        };
        row.action = TodoAction::Drop;
    }
    Ok(RewriteParams {
        head,
        onto: params.onto_tip,
        todo,
        line: params.line.clone(),
        messages: BTreeMap::new(),
        made: Vec::new(),
        index_base: None,
    })
}

/// The branch `HEAD` is on and its tip.
pub(super) fn head_of(
    repo: &RepoHandle,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Tip, RewriteError> {
    if stop.should_stop() {
        return Err(RewriteError::Stopped);
    }
    let HeadRef::Branch(name) = &facts.head else {
        return Err(RewriteError::Refused(Refusal::NotOnABranch));
    };
    let Some(oid) = facts::read_refs(repo, std::slice::from_ref(name))?
        .get(name)
        .copied()
        .flatten()
    else {
        // A branch with no commit yet: there is nothing to rewrite, and
        // nothing a rewrite could stand on.
        return Err(RewriteError::Refused(Refusal::NotOnABranch));
    };
    Ok(Tip {
        name: name.clone(),
        oid,
    })
}

fn open(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    from: Oid,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    if !ancestor(repo, from, head.oid, stop)? {
        return Err(RewriteError::Refused(Refusal::NotOnYourBranch {
            commit: from,
            by: RewriteKey::Open,
        }));
    }
    let onto = match base_of(repo, &head, default_branch, stop)? {
        Some(base) if !ancestor(repo, from, base, stop)? => base,
        // `from` is at the base or deeper, or there is no base: the rows run
        // down to `from` itself.
        _ => match parents_of(repo, from, stop)?.first() {
            Some(parent) => *parent,
            // The first commit: nothing to replay it onto. It stays as the
            // base, and the rows stop above it.
            None => from,
        },
    };
    let range = read_range(repo, onto, head.oid, false, stop)?;
    refuse_merges(&range)?;
    let line = line_of(repo, git, &head, default_branch, stop)?;
    let todo = autosquash(range.iter().map(RangeCommit::pick).collect());
    let by_oid: BTreeMap<Oid, &str> = range
        .iter()
        .map(|commit| (commit.oid, commit.message.as_str()))
        .collect();
    let message_of = |row: usize| {
        todo.get(row)
            .and_then(|line| by_oid.get(&line.commit))
            .map_or_else(String::new, |message| (*message).to_owned())
    };
    let messages = squash_groups(&todo)
        .into_iter()
        .filter(|group| {
            group
                .melded
                .iter()
                .any(|&row| todo[row].action == TodoAction::Squash)
        })
        .map(|group| (group.kept, default_message(&group, &message_of, &todo)))
        .collect();
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages,
        made: Vec::new(),
        index_base: None,
    })
}

fn amend_into(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    target: Oid,
    facts: &RepoFacts,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    if !facts
        .status
        .entries
        .iter()
        .any(graft_git::status::StatusEntry::is_staged)
    {
        return Err(RewriteError::Refused(Refusal::NothingStagedFor { target }));
    }
    if !ancestor(repo, target, head.oid, stop)? {
        return Err(RewriteError::Refused(Refusal::NotOnYourBranch {
            commit: target,
            by: RewriteKey::AmendInto,
        }));
    }
    refuse_out_of_reach(repo, &head, target, default_branch, stop)?;
    let Some(&onto) = parents_of(repo, target, stop)?.first() else {
        return Err(RewriteError::Refused(Refusal::RootCommit {
            commit: target,
        }));
    };
    let range = read_range(repo, onto, head.oid, false, stop)?;
    refuse_merges(&range)?;
    let line = line_of(repo, git, &head, default_branch, stop)?;
    if target == head.oid && line.branches.len() == 1 {
        return Err(RewriteError::Refused(Refusal::PlainAmend));
    }
    let subject = range
        .iter()
        .find(|commit| commit.oid == target)
        .map_or_else(String::new, |commit| commit.subject.clone());
    let fixup_subject = format!("fixup! {subject}");
    let made = commit_index(repo, head.oid, &fixup_subject, stop)?;
    let mut todo: Vec<TodoLine> = range.iter().map(RangeCommit::pick).collect();
    let at = todo
        .iter()
        .position(|line| line.commit == target)
        .map_or(0, |at| at + 1);
    todo.insert(
        at,
        TodoLine {
            action: TodoAction::Fixup,
            commit: made,
            subject: fixup_subject,
        },
    );
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages: BTreeMap::new(),
        made: vec![made],
        index_base: None,
    })
}

/// `a` reaches exactly the commits absorb could put a hunk into
/// ([`your_commits`]); a target past them — an ancestor of `HEAD`, checked
/// before — is refused by what ended the walk (06e ruling table). `HEAD`
/// itself is amend, whatever a remote holds or what is built on it: no
/// range check (06e ruling 5).
fn refuse_out_of_reach(
    repo: &RepoHandle,
    head: &Tip,
    target: Oid,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<(), RewriteError> {
    if target == head.oid {
        return Ok(());
    }
    let range = your_commits(repo, head, default_branch, stop)?;
    if range.commits.contains(&target) {
        return Ok(());
    }
    let refusal = match range.stop {
        // Every commit not taken is an ancestor of the one the walk stopped
        // at, so a pushed stop means the target is pushed.
        RangeStop::Pushed => Refusal::AlreadyPushed { commit: target },
        RangeStop::Base | RangeStop::NotItsOwn => Refusal::NotOnYourBranch {
            commit: target,
            by: RewriteKey::AmendInto,
        },
        RangeStop::Merge(merge) => Refusal::MergeInRange { merge },
        RangeStop::Root => Refusal::RootCommit { commit: target },
        RangeStop::Cap => Refusal::BeyondReach {
            commit: target,
            branch: short_name(head),
        },
    };
    Err(RewriteError::Refused(refusal))
}

/// The branch's short name (`main`), for the words.
pub(super) fn short_name(head: &Tip) -> String {
    head.name
        .strip_prefix("refs/heads/")
        .unwrap_or(&head.name)
        .to_owned()
}

/// The first merge in the range refuses the rewrite: `--rebase-merges` is
/// not in M7.
pub(super) fn refuse_merges(range: &[RangeCommit]) -> Result<(), RewriteError> {
    match range.iter().find(|commit| commit.parents.len() > 1) {
        Some(merge) => Err(RewriteError::Refused(Refusal::MergeInRange {
            merge: merge.oid,
        })),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    //! `FromRebase` against a real repository: the editor `R` opens from a
    //! rebase preview runs, unedited, exactly the rebase's own todo.

    use graft_core::journal::HeadRef;
    use graft_core::oid::Oid;
    use graft_git::repo::RepoHandle;
    use graft_git::run::Stop;

    use crate::facts::read_facts;
    use crate::operation::{FactsScope, RebaseParams, Refusal, RepoFacts, Source, Tip};
    use crate::rewrite::{
        BranchLine, RewriteError, RewriteParams, RewriteRequest, TodoAction, TodoLine,
    };

    use super::plan;

    struct Never;

    impl Stop for Never {
        fn should_stop(&self) -> bool {
            false
        }
    }

    /// A scratch repository, removed on drop.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            // Best effort: a left-over scratch directory is noise, not a
            // failure, and a panic in `Drop` would hide the real one.
            let _removed = std::fs::remove_dir_all(&self.0);
        }
    }

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("graft-from-rebase-{name}-{}", std::process::id()));
            // A directory left by an earlier run of this very test: start
            // clean; absent is the usual case.
            let _stale = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("the scratch directory");
            let scratch = Scratch(dir);
            scratch.git(&["init", "-q", "-b", "main"]);
            scratch.git(&["config", "user.name", "T"]);
            scratch.git(&["config", "user.email", "t@example.com"]);
            scratch.git(&["config", "commit.gpgsign", "false"]);
            scratch
        }

        fn git(&self, args: &[&str]) -> String {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(&self.0)
                .output()
                .expect("git runs");
            assert!(out.status.success(), "git {args:?}: {out:?}");
            String::from_utf8(out.stdout)
                .expect("utf-8")
                .trim()
                .to_owned()
        }

        fn commit(&self, name: &str) {
            std::fs::write(self.0.join(format!("{name}.txt")), name).expect("write");
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "-m", name]);
        }

        fn oid(&self, rev: &str) -> Oid {
            Oid::from_hex(&self.git(&["rev-parse", rev])).expect("an id")
        }

        fn tip(&self, branch: &str) -> Tip {
            Tip {
                name: format!("refs/heads/{branch}"),
                oid: self.oid(branch),
            }
        }

        fn handle(&self) -> RepoHandle {
            RepoHandle::open(&self.0).expect("it opens")
        }
    }

    fn git_version() -> graft_git::version::GitVersion {
        graft_git::version::system_git()
            .expect("git is on PATH")
            .version()
    }

    fn facts(repo: &RepoHandle) -> RepoFacts {
        read_facts(repo, git_version(), &Never, &FactsScope::default()).expect("the facts read")
    }

    /// `feature` (a1 a2 a3) off m1, `main` moved on by m2; `HEAD` on
    /// `feature`. The rebase of `feature` onto `main`, a1 and a3 dropped.
    fn rebase_of_feature(name: &str) -> (Scratch, RebaseParams) {
        let repo = Scratch::new(name);
        repo.commit("m1");
        repo.git(&["switch", "-q", "-c", "feature"]);
        repo.commit("a1");
        repo.commit("a2");
        repo.commit("a3");
        repo.git(&["switch", "-q", "main"]);
        repo.commit("m2");
        repo.git(&["switch", "-q", "feature"]);
        let mut params = RebaseParams::new(
            repo.tip("feature"),
            Source::Branch("refs/heads/main".to_owned()),
            repo.oid("main"),
        );
        params.drops = vec![repo.oid("feature~2"), repo.oid("feature")];
        (repo, params)
    }

    fn plan_from(repo: &Scratch, params: &RebaseParams) -> Result<RewriteParams, RewriteError> {
        let handle = repo.handle();
        plan(
            &handle,
            &git_version(),
            &RewriteRequest::FromRebase(params.clone()),
            &facts(&handle),
            None,
            &Never,
        )
    }

    #[test]
    fn from_rebase_unedited_matches_the_rebase_todo() {
        let (repo, params) = rebase_of_feature("matches");

        let planned = plan_from(&repo, &params).expect("it plans");

        // What `rebase -i main` lists, read here by git itself, oldest
        // first, each drop marked `drop`.
        let listed = repo.git(&["rev-list", "--reverse", "main..feature"]);
        let expected: Vec<TodoLine> = listed
            .lines()
            .zip(["a1", "a2", "a3"])
            .map(|(oid, subject)| {
                let commit = Oid::from_hex(oid).expect("an id");
                TodoLine {
                    action: if params.drops.contains(&commit) {
                        TodoAction::Drop
                    } else {
                        TodoAction::Pick
                    },
                    commit,
                    subject: subject.to_owned(),
                }
            })
            .collect();
        assert_eq!(planned.todo, expected);
        assert_eq!(
            planned
                .todo
                .iter()
                .map(|row| row.action)
                .collect::<Vec<_>>(),
            [TodoAction::Drop, TodoAction::Pick, TodoAction::Drop]
        );
        assert_eq!(planned.onto, params.onto_tip);
        assert_eq!(planned.line, params.line);
        assert_eq!(planned.head, params.branch);
        assert!(planned.messages.is_empty());
        assert!(planned.made.is_empty());
        assert_eq!(planned.index_base, None);
    }

    #[test]
    fn from_rebase_keeps_the_line_as_given() {
        let (repo, mut params) = rebase_of_feature("line");
        repo.git(&["branch", "on-top", "feature"]);
        params.line = BranchLine {
            branches: vec![repo.tip("feature"), repo.tip("on-top")],
        };

        let planned = plan_from(&repo, &params).expect("it plans");

        assert_eq!(planned.line, params.line);
    }

    #[test]
    fn from_rebase_refuses_when_head_moved() {
        let (repo, params) = rebase_of_feature("moved");
        repo.commit("a4");

        let refused = plan_from(&repo, &params);

        assert!(
            matches!(
                &refused,
                Err(RewriteError::Refused(Refusal::Moved { name })) if name == "refs/heads/feature"
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn from_rebase_refuses_when_head_switched() {
        let (repo, params) = rebase_of_feature("switched");
        repo.git(&["switch", "-q", "-c", "other"]);

        let refused = plan_from(&repo, &params);

        assert!(
            matches!(
                &refused,
                Err(RewriteError::Refused(Refusal::Moved { name })) if name == "refs/heads/feature"
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn from_rebase_refuses_detached() {
        let (repo, params) = rebase_of_feature("detached");
        repo.git(&["switch", "-q", "--detach"]);
        let handle = repo.handle();
        assert!(
            matches!(facts(&handle).head, HeadRef::Detached(_)),
            "HEAD is detached"
        );

        let refused = plan_from(&repo, &params);

        assert!(
            matches!(&refused, Err(RewriteError::Refused(Refusal::NotOnABranch))),
            "{refused:?}"
        );
    }
}
