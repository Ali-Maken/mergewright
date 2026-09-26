//! The raw history reads a rewrite plans and runs from: ranges, picks,
//! messages, parents, the base of his branch, and the index as a commit.
//! Reads only — nothing here decides or refuses.

use std::collections::BTreeMap;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::branches::Branches;
use graft_git::history_reads;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::{self, FactsError};
use crate::operation::Tip;
use crate::preview::is_ancestor;

use super::{BranchLine, TodoAction, TodoLine};

/// One commit of the range, as the plan needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RangeCommit {
    pub(crate) oid: Oid,
    pub(crate) parents: Vec<Oid>,
    pub(crate) subject: String,
    /// The whole message, as git stores it.
    pub(crate) message: String,
}

impl RangeCommit {
    pub(super) fn pick(&self) -> TodoLine {
        TodoLine {
            action: TodoAction::Pick,
            commit: self.oid,
            subject: self.subject.clone(),
        }
    }
}

/// Where his branch left its base: the merge-base with its upstream when
/// one is set and fetched, else with the default branch. `None` when there
/// is neither, or they share no history.
pub(super) fn base_of(
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
pub(super) fn default_branch_of(
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
pub(super) fn ancestor(
    repo: &RepoHandle,
    older: Oid,
    newer: Oid,
    stop: &dyn Stop,
) -> Result<bool, FactsError> {
    is_ancestor(repo, older, newer, stop).map_err(FactsError::of_history)
}

/// A commit's parents, first parent first.
pub(super) fn parents_of(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Vec<Oid>, FactsError> {
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
pub(super) fn read_range(
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

/// The index written as a commit object with no ref, parent `head`.
pub(super) fn commit_index(
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

pub(super) fn parse_oid(hex: &str) -> Result<Oid, FactsError> {
    Oid::from_hex(hex).map_err(|_| unparsable(format!("{hex:?} is not a commit id")))
}

pub(super) fn ref_name(name: &str) -> Result<RefName, FactsError> {
    RefName::new(name).map_err(|_| unparsable(format!("{name:?} is not a ref name")))
}

pub(super) fn unparsable(what: String) -> FactsError {
    FactsError::History(RunError::Unparsable(what))
}
