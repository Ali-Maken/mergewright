//! The squash-merge suggestion (m7/05, note R3; research
//! `stacked-branches.md` §3): a bottom branch whose whole change already
//! landed on `onto` as one commit — a squash-merged pull request — is
//! suggested dropped, so its commits are not replayed onto their own copy.
//!
//! Local only: the combined `git patch-id --stable` of the branch's range
//! (`diff <split> <tip>`) against each commit on `onto` since the split,
//! capped at [`DROP_CHECK_CAP`]; past the cap, `not checked`. Both diffs are
//! porcelain with the same flags (`--no-renames --no-ext-diff --no-textconv
//! --no-color`), so the user's diff config shapes both alike.

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::history_reads;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::Tip;
use crate::rewrite::BranchLine;

/// How many commits on `onto` since the split are compared. Past it the
/// check is [`DropSuggestion::NotChecked`].
pub const DROP_CHECK_CAP: usize = 200;

/// The bottom branch's change is already on `onto`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlreadyIn {
    /// The bottom branch, full name.
    pub branch: RefName,
    /// Its commits since the split, oldest first: the rows dropped.
    pub commits: Vec<Oid>,
    /// The commit on `onto` whose patch-id matches the whole range.
    pub matches: Oid,
    /// That commit's subject.
    pub subject: String,
}

/// What the check found worth saying. Nothing matched is `None`, not a
/// variant: there is nothing to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropSuggestion {
    /// A match: drop these commits.
    Drop(AlreadyIn),
    /// The check was not made, and why (past [`DROP_CHECK_CAP`]). Never
    /// silent: the box says so.
    NotChecked(String),
}

/// Is the bottom branch of `line` already on `onto`, squashed into one
/// commit? `None` when nothing matched, the branch has no commits of its
/// own, or `onto` has nothing since the split.
///
/// Deviation from the ticket's `Option<AlreadyIn>`: past the cap the box
/// must say `not checked`, which `None` cannot carry.
///
/// # Errors
/// [`FactsError::History`] when a read fails; [`FactsError::Stopped`].
pub fn suggest_drops(
    repo: &RepoHandle,
    _git: &GitVersion,
    line: &BranchLine,
    onto: Oid,
    stop: &dyn Stop,
) -> Result<Option<DropSuggestion>, FactsError> {
    let Some(bottom) = line.branches.first() else {
        return Ok(None);
    };
    polled(stop)?;
    let split =
        history_reads::merge_base(repo, onto, bottom.oid).map_err(FactsError::of_history)?;
    let Some(split) = split.filter(|&split| split != bottom.oid && split != onto) else {
        return Ok(None);
    };
    let since = format!("{split}..{onto}");
    let count = text(
        repo,
        vec![
            GitArg::Flag("rev-list"),
            GitArg::Flag("--count"),
            GitArg::Word(since.clone()),
        ],
        stop,
    )?;
    let count: usize = count
        .trim()
        .parse()
        .map_err(|_| unparsable(format!("rev-list --count printed {count:?}")))?;
    if count > DROP_CHECK_CAP {
        return Ok(Some(DropSuggestion::NotChecked(format!(
            "more than {DROP_CHECK_CAP} commits since it left"
        ))));
    }
    match matching_commit(repo, split, bottom.oid, &since, stop)? {
        Some(matches) => already_in(repo, bottom, split, matches, stop).map(Some),
        None => Ok(None),
    }
}

/// The commit in `since` whose patch-id is the range `split..tip`'s.
fn matching_commit(
    repo: &RepoHandle,
    split: Oid,
    tip: Oid,
    since: &str,
    stop: &dyn Stop,
) -> Result<Option<Oid>, FactsError> {
    let range = patch(
        repo,
        "diff",
        vec![GitArg::Oid(split), GitArg::Oid(tip)],
        stop,
    )?;
    // An empty range diff (the branch's commits cancel out) has no id, and
    // matches nothing worth dropping.
    let Some((wanted, _)) = patch_ids(repo, range, stop)?.into_iter().next() else {
        return Ok(None);
    };
    let log = patch(
        repo,
        "log",
        vec![
            GitArg::Flag("-p"),
            GitArg::Flag("--no-merges"),
            GitArg::Flag("--format=commit %H"),
            GitArg::Word(since.to_owned()),
        ],
        stop,
    )?;
    let found = patch_ids(repo, log, stop)?
        .into_iter()
        .find_map(|(id, commit)| (id == wanted).then_some(commit));
    found
        .map(|commit| {
            Oid::from_hex(&commit).map_err(|_| unparsable(format!("patch-id printed {commit:?}")))
        })
        .transpose()
}

/// The suggestion for `bottom`, whose range matched `matches`.
fn already_in(
    repo: &RepoHandle,
    bottom: &Tip,
    split: Oid,
    matches: Oid,
    stop: &dyn Stop,
) -> Result<DropSuggestion, FactsError> {
    let subject = text(
        repo,
        vec![
            GitArg::Flag("log"),
            GitArg::Flag("-1"),
            GitArg::Flag("--format=%s"),
            GitArg::Oid(matches),
        ],
        stop,
    )?;
    let commits = text(
        repo,
        vec![
            GitArg::Flag("rev-list"),
            GitArg::Flag("--reverse"),
            GitArg::Flag("--no-merges"),
            GitArg::Word(format!("{split}..{}", bottom.oid)),
        ],
        stop,
    )?
    .lines()
    .map(|line| {
        Oid::from_hex(line.trim()).map_err(|_| unparsable(format!("rev-list printed {line:?}")))
    })
    .collect::<Result<Vec<_>, _>>()?;
    let branch = RefName::new(bottom.name.as_bytes())
        .map_err(|_| unparsable(format!("the branch name {:?}", bottom.name)))?;
    Ok(DropSuggestion::Drop(AlreadyIn {
        branch,
        commits,
        matches,
        subject: subject.trim_end().to_owned(),
    }))
}

/// `git <command>` with the diff flags both sides share, as bytes for
/// `patch-id`.
fn patch(
    repo: &RepoHandle,
    command: &'static str,
    rest: Vec<GitArg>,
    stop: &dyn Stop,
) -> Result<Vec<u8>, FactsError> {
    polled(stop)?;
    let mut args = vec![GitArg::Flag(command)];
    for flag in [
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--no-renames",
    ] {
        args.push(GitArg::Flag(flag));
    }
    args.extend(rest);
    Ok(GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?
        .stdout)
}

/// `git patch-id --stable` over `input`: `(patch id, commit)` per patch, in
/// order. A plain diff's commit is all zeros.
fn patch_ids(
    repo: &RepoHandle,
    input: Vec<u8>,
    stop: &dyn Stop,
) -> Result<Vec<(String, String)>, FactsError> {
    polled(stop)?;
    let output = GitInvocation::new(repo)
        .args([GitArg::Flag("patch-id"), GitArg::Flag("--stable")])
        .stdin(input)
        .run(stop)
        .map_err(FactsError::of_history)?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(id, commit)| (id.to_owned(), commit.trim().to_owned()))
        .collect())
}

fn text(repo: &RepoHandle, args: Vec<GitArg>, stop: &dyn Stop) -> Result<String, FactsError> {
    polled(stop)?;
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn unparsable(what: String) -> FactsError {
    FactsError::History(RunError::Unparsable(what))
}

fn polled(stop: &dyn Stop) -> Result<(), FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    Ok(())
}
