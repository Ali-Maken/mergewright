//! What a rewrite request plans to — the range of his branch, the refusals
//! before anything runs, and the line of branches built on his.

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::branches::Branches;
use graft_git::history_reads;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::{self, FactsError};
use crate::operation::{Refusal, RepoFacts, Tip};
use crate::preview::is_ancestor;

use super::todo::autosquash;
use super::{
    BranchLine, Dependents, RewriteError, RewriteParams, RewriteRequest, TodoAction, TodoLine,
    default_message, squash_groups,
};

/// The oldest git whose `rebase -i` knows `update-ref`: a rewrite with
/// branches following needs it (ADR-0047 §4).
pub(crate) const UPDATE_REF_GIT: GitVersion = GitVersion {
    major: 2,
    minor: 38,
    patch: 0,
};

/// One commit of the range, as the plan needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RangeCommit {
    pub(crate) oid: Oid,
    pub(crate) parents: Vec<Oid>,
    pub(crate) subject: String,
    /// The whole message, as git stores it.
    pub(crate) message: String,
}

impl RangeCommit {
    fn pick(&self) -> TodoLine {
        TodoLine {
            action: TodoAction::Pick,
            commit: self.oid,
            subject: self.subject.clone(),
        }
    }
}

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
/// `default_branch` is the user's `graph.default_branch` when set: it wins
/// over git's guess (ADR-0001 rule 1), for the base of the range and for
/// which branch is never a dependent.
///
/// # Errors
/// [`RewriteError::Refused`], checked in this order: not on a branch;
/// nothing staged (`AmendInto`); the commit not on his branch; the first
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
    let head = Tip {
        name: name.clone(),
        oid,
    };
    match req {
        RewriteRequest::Open { from } => open(repo, git, head, *from, default_branch, stop),
        RewriteRequest::AmendInto { target } => {
            amend_into(repo, git, head, *target, facts, default_branch, stop)
        }
    }
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
        }));
    }
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
    })
}

/// His branch and the branches built on it, or the refusal that stops the
/// rewrite: a fork, one checked out in another worktree, or an old git.
pub(crate) fn line_of(
    repo: &RepoHandle,
    git: &GitVersion,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<BranchLine, RewriteError> {
    let line = match dependents(repo, git, head, default_branch, stop)? {
        Dependents::Line(line) => line,
        Dependents::Fork { a, b } => {
            return Err(RewriteError::Refused(Refusal::ForkedDependents { a, b }));
        }
    };
    if line.branches.len() > 1 {
        if let Some((branch, worktree)) = checked_out_elsewhere(repo, line.dependents(), stop)? {
            return Err(RewriteError::Refused(Refusal::DependentElsewhere {
                branch,
                worktree,
            }));
        }
        if *git < UPDATE_REF_GIT {
            return Err(RewriteError::Refused(Refusal::GitTooOld { need: "2.38" }));
        }
    }
    Ok(line)
}

/// The branches built on `branch`, bottom first after it.
///
/// A local branch B is one when B's first-parent chain reaches `branch`'s
/// tip with no merge between the two, B is not the default branch, and B
/// has at least one commit of its own (a second name for the same commit —
/// a backup made before a rewrite — is not built on it). They must form one
/// line, each built on the one below it; two that are not are a
/// [`Dependents::Fork`], and Graft does not guess which one follows.
///
/// A branch checked out in another worktree is still in the line: it is
/// built on his, and leaving it behind is the duplicated history the chain
/// exists to prevent. [`plan`] refuses it in words instead.
///
/// `default_branch` is the user's `graph.default_branch` when set; it wins
/// over git's guess.
///
/// # Errors
/// [`FactsError`] when a read fails, [`FactsError::Stopped`] when `stop`
/// asks.
pub fn dependents(
    repo: &RepoHandle,
    _git: &GitVersion,
    branch: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<Dependents, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let default = default_branch_of(repo, default_branch)?;
    let mut found: Vec<(Tip, Vec<Oid>)> = Vec::new();
    for head in local_branches(repo, Some(branch.oid), stop)? {
        if head.name == branch.name
            || Some(&head.name) == default.as_ref()
            || head.oid == branch.oid
        {
            continue;
        }
        if let Some(chain) = chain_above(repo, branch.oid, head.oid, stop)? {
            found.push((
                Tip {
                    name: head.name,
                    oid: head.oid,
                },
                chain,
            ));
        }
    }
    found.sort_by(|(a, a_chain), (b, b_chain)| {
        a_chain
            .len()
            .cmp(&b_chain.len())
            .then_with(|| a.name.cmp(&b.name))
    });
    for pair in found.windows(2) {
        let [(lower, _), (upper, upper_chain)] = pair else {
            continue;
        };
        if lower.oid != upper.oid && !upper_chain.contains(&lower.oid) {
            return Ok(Dependents::Fork {
                a: ref_name(&lower.name)?,
                b: ref_name(&upper.name)?,
            });
        }
    }
    let mut branches = vec![branch.clone()];
    branches.extend(found.into_iter().map(|(tip, _)| tip));
    Ok(Dependents::Line(BranchLine { branches }))
}

/// The commits from `tip` (excluded) up `above`'s first-parent chain, newest
/// first — `None` unless that chain reaches `tip` with no merge on the way.
fn chain_above(
    repo: &RepoHandle,
    tip: Oid,
    above: Oid,
    stop: &dyn Stop,
) -> Result<Option<Vec<Oid>>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--first-parent"),
            GitArg::Flag("--parents"),
            GitArg::Oid(above),
            GitArg::Word(format!("^{tip}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut chain = Vec::new();
    let mut last_parent = None;
    for line in text.lines() {
        let ids = line
            .split_whitespace()
            .map(parse_oid)
            .collect::<Result<Vec<Oid>, FactsError>>()?;
        let [commit, parent] = ids.as_slice() else {
            // A merge, or a root: not a straight line down to `tip`.
            return Ok(None);
        };
        chain.push(*commit);
        last_parent = Some(*parent);
    }
    Ok((last_parent == Some(tip)).then_some(chain))
}

/// A local branch as `for-each-ref` lists it.
struct LocalBranch {
    name: String,
    oid: Oid,
    /// The worktree it is checked out in, if any.
    worktree: Option<PathBuf>,
}

/// Every local branch, or only those containing `contains`.
fn local_branches(
    repo: &RepoHandle,
    contains: Option<Oid>,
    stop: &dyn Stop,
) -> Result<Vec<LocalBranch>, FactsError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Word("--format=%(refname)%00%(objectname)%00%(worktreepath)".to_owned()),
    ];
    if let Some(oid) = contains {
        args.push(GitArg::Flag("--contains"));
        args.push(GitArg::Oid(oid));
    }
    args.push(GitArg::Word("refs/heads/".to_owned()));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8(output.stdout).map_err(|error| {
        FactsError::NonUtf8Ref(String::from_utf8_lossy(error.as_bytes()).into_owned())
    })?;
    text.lines()
        .map(|line| {
            let mut fields = line.split('\0');
            let (Some(name), Some(oid), worktree) = (fields.next(), fields.next(), fields.next())
            else {
                return Err(unparsable(format!("for-each-ref line {line:?}")));
            };
            Ok(LocalBranch {
                name: name.to_owned(),
                oid: parse_oid(oid)?,
                worktree: worktree.filter(|path| !path.is_empty()).map(PathBuf::from),
            })
        })
        .collect()
}

/// The first of `branches` checked out in a worktree other than this one,
/// and where.
fn checked_out_elsewhere(
    repo: &RepoHandle,
    branches: &[Tip],
    stop: &dyn Stop,
) -> Result<Option<(RefName, PathBuf)>, FactsError> {
    let here = repo.workdir().map(canonical);
    for local in local_branches(repo, None, stop)? {
        let Some(worktree) = local.worktree else {
            continue;
        };
        if !branches.iter().any(|tip| tip.name == local.name) {
            continue;
        }
        if here.as_ref() != Some(&canonical(&worktree)) {
            return Ok(Some((ref_name(&local.name)?, worktree)));
        }
    }
    Ok(None)
}

/// A path with symlinks resolved, for comparing two spellings of one
/// worktree (`/tmp` and `/private/tmp` on macOS). One that cannot be
/// resolved is compared as written.
fn canonical(path: &std::path::Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Where his branch left its base: the merge-base with its upstream when
/// one is set and fetched, else with the default branch. `None` when there
/// is neither, or they share no history.
fn base_of(
    repo: &RepoHandle,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<Option<Oid>, FactsError> {
    let upstream = graft_git::remote::upstream_of(repo, &head.name)?
        .and_then(|upstream| upstream.tracking_tip);
    let against = match upstream {
        Some(tip) => Some(tip),
        None => match default_branch_of(repo, default_branch)? {
            Some(name) => facts::read_refs(repo, std::slice::from_ref(&name))?
                .get(&name)
                .copied()
                .flatten(),
            None => None,
        },
    };
    let Some(against) = against else {
        return Ok(None);
    };
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    history_reads::merge_base(repo, head.oid, against).map_err(FactsError::of_history)
}

/// The default branch's full name: the user's override when set, else
/// ADR-0001's chain.
fn default_branch_of(
    repo: &RepoHandle,
    user_override: Option<&RefName>,
) -> Result<Option<String>, FactsError> {
    Branches::resolve(repo, user_override)?
        .default_branch()
        .map(|name| {
            String::from_utf8(name.as_bytes().to_vec()).map_err(|error| {
                FactsError::NonUtf8Ref(String::from_utf8_lossy(error.as_bytes()).into_owned())
            })
        })
        .transpose()
}

/// `older` is `newer` or one of its ancestors.
fn ancestor(
    repo: &RepoHandle,
    older: Oid,
    newer: Oid,
    stop: &dyn Stop,
) -> Result<bool, FactsError> {
    is_ancestor(repo, older, newer, stop).map_err(FactsError::of_history)
}

/// A commit's parents, first parent first.
fn parents_of(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Vec<Oid>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            GitArg::Word("--max-count=1".to_owned()),
            GitArg::Oid(commit),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace().skip(1).map(parse_oid).collect()
}

/// The commits in `include` and not in `exclude`, oldest first, whole
/// messages included. `first_parent` walks the first-parent chain only.
pub(crate) fn read_range(
    repo: &RepoHandle,
    exclude: Oid,
    include: Oid,
    first_parent: bool,
    stop: &dyn Stop,
) -> Result<Vec<RangeCommit>, FactsError> {
    let mut args = vec![
        GitArg::Flag("rev-list"),
        GitArg::Flag("--reverse"),
        GitArg::Flag("--topo-order"),
        GitArg::Word("--format=%x1e%H%x00%P%x00%s%x00%B%x1f".to_owned()),
    ];
    if first_parent {
        args.push(GitArg::Flag("--first-parent"));
    }
    args.push(GitArg::Oid(include));
    args.push(GitArg::Word(format!("^{exclude}")));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split('\u{1e}')
        .skip(1)
        .map(|record| {
            let record = record.split('\u{1f}').next().unwrap_or(record);
            let mut fields = record.splitn(4, '\0');
            let (Some(oid), Some(parents), Some(subject), Some(message)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                return Err(unparsable(format!("rev-list record {record:?}")));
            };
            Ok(RangeCommit {
                oid: parse_oid(oid)?,
                parents: parents
                    .split_whitespace()
                    .map(parse_oid)
                    .collect::<Result<_, _>>()?,
                subject: subject.to_owned(),
                message: message.to_owned(),
            })
        })
        .collect()
}

/// Each branch built on his, with its own commits as picks, bottom first:
/// what follows his rows in the todo git runs.
pub(crate) fn line_picks(
    repo: &RepoHandle,
    line: &BranchLine,
    stop: &dyn Stop,
) -> Result<Vec<(String, Vec<TodoLine>)>, FactsError> {
    line.branches
        .windows(2)
        .map(|pair| {
            let [below, branch] = pair else {
                return Ok((String::new(), Vec::new()));
            };
            let commits = read_range(repo, below.oid, branch.oid, true, stop)?;
            Ok((
                branch.name.clone(),
                commits.iter().map(RangeCommit::pick).collect(),
            ))
        })
        .collect()
}

/// The picks of `r` (a rebase of `head` onto `onto`): the commits git's own
/// `rebase -i <onto>` would list, oldest first — on `head` and not on
/// `onto`, merges left out, and a commit whose change `onto` already has
/// left out too (git's `--cherry-mark` rule, which the plain `rebase`
/// before ADR-0047 reported as "patch contents already upstream").
pub(crate) fn rebase_rows(
    repo: &RepoHandle,
    onto: Oid,
    head: Oid,
    stop: &dyn Stop,
) -> Result<Vec<TodoLine>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--reverse"),
            GitArg::Flag("--topo-order"),
            GitArg::Flag("--right-only"),
            GitArg::Flag("--cherry-pick"),
            GitArg::Flag("--no-merges"),
            GitArg::Word("--format=%x1e%H%x00%s%x1f".to_owned()),
            GitArg::Word(format!("{onto}...{head}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split('\u{1e}')
        .skip(1)
        .map(|record| {
            let record = record.split('\u{1f}').next().unwrap_or(record);
            let Some((oid, subject)) = record.split_once('\0') else {
                return Err(unparsable(format!("rev-list record {record:?}")));
            };
            Ok(TodoLine {
                action: TodoAction::Pick,
                commit: parse_oid(oid)?,
                subject: subject.to_owned(),
            })
        })
        .collect()
}

/// Each commit's whole message, as git stores it.
pub(crate) fn messages_of(
    repo: &RepoHandle,
    commits: &[Oid],
    stop: &dyn Stop,
) -> Result<BTreeMap<Oid, String>, FactsError> {
    if commits.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut args = vec![
        GitArg::Flag("rev-list"),
        GitArg::Word("--no-walk=unsorted".to_owned()),
        GitArg::Word("--format=%x1e%H%x00%B%x1f".to_owned()),
    ];
    args.extend(commits.iter().map(|oid| GitArg::Oid(*oid)));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split('\u{1e}')
        .skip(1)
        .map(|record| {
            let record = record.split('\u{1f}').next().unwrap_or(record);
            let Some((oid, message)) = record.split_once('\0') else {
                return Err(unparsable(format!("rev-list record {record:?}")));
            };
            Ok((parse_oid(oid)?, message.to_owned()))
        })
        .collect()
}

/// The first merge in the range refuses the rewrite: `--rebase-merges` is
/// not in M7.
fn refuse_merges(range: &[RangeCommit]) -> Result<(), RewriteError> {
    match range.iter().find(|commit| commit.parents.len() > 1) {
        Some(merge) => Err(RewriteError::Refused(Refusal::MergeInRange {
            merge: merge.oid,
        })),
        None => Ok(()),
    }
}

/// The index written as a commit object with no ref, parent `head`.
fn commit_index(
    repo: &RepoHandle,
    head: Oid,
    subject: &str,
    stop: &dyn Stop,
) -> Result<Oid, FactsError> {
    let tree = GitInvocation::new(repo)
        .arg(GitArg::Flag("write-tree"))
        .run(stop)
        .map_err(FactsError::of_history)?;
    let tree = parse_oid(String::from_utf8_lossy(&tree.stdout).trim())?;
    let commit = GitInvocation::new(repo)
        .args([
            GitArg::Flag("commit-tree"),
            GitArg::Oid(tree),
            GitArg::Flag("-p"),
            GitArg::Oid(head),
            GitArg::Flag("-F"),
            GitArg::Flag("-"),
        ])
        .stdin(format!("{subject}\n").into_bytes())
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_oid(String::from_utf8_lossy(&commit.stdout).trim())
}

fn parse_oid(hex: &str) -> Result<Oid, FactsError> {
    Oid::from_hex(hex).map_err(|_| unparsable(format!("{hex:?} is not a commit id")))
}

fn ref_name(name: &str) -> Result<RefName, FactsError> {
    RefName::new(name).map_err(|_| unparsable(format!("{name:?} is not a ref name")))
}

fn unparsable(what: String) -> FactsError {
    FactsError::History(RunError::Unparsable(what))
}
