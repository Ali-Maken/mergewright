//! A replay's rows of ADR-0049 §4: a rebase or rewrite moves `HEAD` to
//! `onto` (git's `reset --hard`), then each replayed commit writes the
//! paths of its own change; a pick or revert only the latter. The engine's
//! `switch <home>` afterwards writes only paths in this union.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, Stop};

use super::reads::{diff_tree, head, oids, parents_of, paths_in, run_error};
use crate::facts::FactsError;
use crate::operation::{Operation, RevertParams};
use crate::rewrite::{RewriteParams, TodoAction, git_words};

/// One commit a replay applies, and the commit its change is taken against.
struct Replayed {
    commit: Oid,
    /// `None`: its first parent (a root commit: its whole tree).
    against: Option<Oid>,
}

impl Replayed {
    fn first_parent(commit: Oid) -> Self {
        Self {
            commit,
            against: None,
        }
    }
}

/// Every path the replay writes. Empty for a kind that isn't a replay.
pub(super) fn replayed_paths(
    repo: &RepoHandle,
    op: &Operation,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let (onto, replayed) = match op {
        Operation::Rebase(params) => (
            Some(params.onto_tip),
            commits_between(repo, params.onto_tip, head(), stop)?,
        ),
        Operation::Rewrite(params) => {
            let (onto, passed) = skip_unchanged(repo, params, stop)?;
            (Some(onto), rewritten(repo, params, passed, stop)?)
        }
        Operation::CherryPick(params) => (
            None,
            params
                .commits
                .iter()
                .copied()
                .map(Replayed::first_parent)
                .collect(),
        ),
        Operation::Revert(params) => (None, vec![reverted(repo, params, stop)?]),
        _ => return Ok(BTreeSet::new()),
    };
    let mut paths = match onto {
        Some(onto) => diff_tree(repo, &[], [head(), GitArg::Oid(onto)], stop)?,
        None => BTreeSet::new(),
    };
    paths.extend(own_changes(repo, &replayed, stop)?);
    Ok(paths)
}

/// git's `skip_unnecessary_picks` (`sequencer.c`), which runs because
/// Mergewright's rewrite never passes `--no-ff`: while the next row git is
/// handed is a `pick` whose only parent is the running `onto`, git moves
/// `onto` to that commit and never writes it; a `drop` row is passed over;
/// any other row, a root or a merge ends it. Returns the `onto` git starts
/// from and how many leading rows it passed (review 28 m1: `R` leaving the
/// untracking commit alone, or `a` into it, never writes `onto`'s tree).
///
/// One spawn, and only when a leading row is a `pick`: the parents of the
/// leading rows, in one `rev-list --no-walk --parents`.
fn skip_unchanged(
    repo: &RepoHandle,
    params: &RewriteParams,
    stop: &dyn Stop,
) -> Result<(Oid, usize), FactsError> {
    let words = git_words(&params.todo, &params.messages);
    let leading = words
        .iter()
        .take_while(|word| matches!(word, TodoAction::Pick | TodoAction::Drop))
        .count();
    if !words[..leading].contains(&TodoAction::Pick) {
        return Ok((params.onto, 0));
    }
    let commits: Vec<Oid> = params.todo[..leading]
        .iter()
        .map(|line| line.commit)
        .collect();
    let parents = parents_of_each(repo, &commits, stop)?;
    let mut onto = params.onto;
    let mut passed = 0;
    for (line, word) in params.todo[..leading].iter().zip(&words) {
        if *word == TodoAction::Pick {
            match parents.get(&line.commit).map(Vec::as_slice) {
                Some([only]) if *only == onto => onto = line.commit,
                _ => break,
            }
        }
        passed += 1;
    }
    Ok((onto, passed))
}

/// Each commit's parents: `rev-list --no-walk --parents <commits>`, one
/// line `<commit> <parent>…` a commit.
fn parents_of_each(
    repo: &RepoHandle,
    commits: &[Oid],
    stop: &dyn Stop,
) -> Result<BTreeMap<Oid, Vec<Oid>>, FactsError> {
    let mut args = vec![
        GitArg::Flag("rev-list"),
        GitArg::Flag("--no-walk"),
        GitArg::Flag("--parents"),
    ];
    args.extend(commits.iter().copied().map(GitArg::Oid));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(run_error)?;
    let mut parents = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some((commit, rest)) = oids(line.as_bytes(), "rev-list")?.split_first() {
            parents.insert(*commit, rest.to_vec());
        }
    }
    Ok(parents)
}

/// A rewrite's replayed commits: every row from the first git doesn't skip
/// that isn't dropped (the `made` fixups are rows), then the dependents'
/// commits, which follow his rows as picks when git is started on the top
/// of the line.
fn rewritten(
    repo: &RepoHandle,
    params: &RewriteParams,
    passed: usize,
    stop: &dyn Stop,
) -> Result<Vec<Replayed>, FactsError> {
    let mut replayed: Vec<Replayed> = params.todo[passed..]
        .iter()
        .filter(|line| line.action != TodoAction::Drop)
        .map(|line| Replayed::first_parent(line.commit))
        .collect();
    if let Some(top) = params.line.top()
        && top.name != params.head.name
    {
        replayed.extend(commits_between(
            repo,
            params.head.oid,
            GitArg::Oid(top.oid),
            stop,
        )?);
    }
    Ok(replayed)
}

/// A revert's change: against `parent`, else parent `-m n` (no `-m`: the
/// first).
fn reverted(
    repo: &RepoHandle,
    params: &RevertParams,
    stop: &dyn Stop,
) -> Result<Replayed, FactsError> {
    let against = if let Some(parent) = params.parent {
        Some(parent)
    } else {
        let nth = usize::from(params.mainline.unwrap_or(1)).saturating_sub(1);
        parents_of(repo, params.commit, stop)?.get(nth).copied()
    };
    Ok(Replayed {
        commit: params.commit,
        against,
    })
}

/// The commits a rebase of `tip` onto `base` replays: `rev-list
/// --no-merges <base>..<tip>` (git drops merges from a rebase that doesn't
/// keep them).
fn commits_between(
    repo: &RepoHandle,
    base: Oid,
    tip: GitArg,
    stop: &dyn Stop,
) -> Result<Vec<Replayed>, FactsError> {
    let output = GitInvocation::new(repo)
        .args(vec![
            GitArg::Flag("rev-list"),
            GitArg::Flag("--no-merges"),
            tip,
            GitArg::Ref(format!("^{base}").into_bytes()),
        ])
        .run(stop)
        .map_err(run_error)?;
    Ok(oids(&output.stdout, "rev-list")?
        .into_iter()
        .map(Replayed::first_parent)
        .collect())
}

/// Every path the replayed commits change, each against the commit it is
/// taken against: one `diff-tree --stdin` for all of them.
fn own_changes(
    repo: &RepoHandle,
    replayed: &[Replayed],
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    if replayed.is_empty() {
        return Ok(BTreeSet::new());
    }
    // `<commit>` alone is diffed against its parent; `<commit> <parent>`
    // against the parent named.
    let mut input = String::new();
    for one in replayed {
        input.push_str(&one.commit.to_string());
        if let Some(against) = one.against {
            input.push(' ');
            input.push_str(&against.to_string());
        }
        input.push('\n');
    }
    let output = GitInvocation::new(repo)
        .args(vec![
            GitArg::Flag("diff-tree"),
            GitArg::Flag("--stdin"),
            GitArg::Flag("--no-commit-id"),
            GitArg::Flag("--root"),
            GitArg::Flag("-r"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("-z"),
        ])
        .stdin(input.into_bytes())
        .run(stop)
        .map_err(run_error)?;
    Ok(paths_in(&output.stdout))
}
