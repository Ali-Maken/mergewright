//! How far apart two commits are, and which commits only a stash reaches
//! (m7-power-tools/14a). Plain reads: every spawn is a `rev-list`,
//! `merge-base` or `cat-file --batch-check`, each stoppable.

use std::collections::HashMap;

use graft_core::oid::Oid;

use super::Range;
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// Each commit `rev-list --parents` printed, to its parents.
type Parents = HashMap<Oid, Vec<Oid>>;

/// [`compare_span`]'s answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareSpan {
    /// `split` is the merge base; `ahead` = commits in `tip` not in `split`.
    Since {
        /// Where the two histories split.
        split: Oid,
        /// Commits reachable from `tip` and not from `split`.
        ahead: usize,
    },
    /// `git merge-base` exit 1: no common ancestor.
    NoSharedHistory,
}

/// Where `base` and `tip` split and how many commits `tip` has since.
///
/// `tip` an ancestor of `base` (or equal to it) answers `split == tip`,
/// `ahead == 0`; the caller words that case.
///
/// # Errors
/// [`RunError`] from the runner, `Stopped` included; never a partial answer.
pub fn compare_span(
    repo: &RepoHandle,
    base: Oid,
    tip: Oid,
    stop: &dyn Stop,
) -> Result<CompareSpan, RunError> {
    // `merge_base` itself takes no stop (its callers pass none); this is
    // the same spawn with the caller's stop, so a raised stop spawns nothing.
    let invocation = GitInvocation::new(repo).args([
        GitArg::Flag("merge-base"),
        GitArg::Oid(base),
        GitArg::Oid(tip),
    ]);
    let split = match invocation.run(stop) {
        Ok(output) => parse_id(output.stdout.trim_ascii(), "merge-base")?,
        // No common ancestor: exit 1 with empty stdout, an answer here.
        Err(RunError::Exited { status: 1, .. }) => return Ok(CompareSpan::NoSharedHistory),
        Err(other) => return Err(other),
    };
    let ahead = count_between(repo, split, tip, stop)?;
    Ok(CompareSpan::Since { split, ahead })
}

/// How many commits `include` reaches that `exclude` does not:
/// `git rev-list --count <include> ^<exclude>`. Never capped: `--count`
/// lists nothing.
///
/// # Errors
/// [`RunError`] from the runner, `Stopped` included; `Unparsable` when git
/// prints anything but one integer.
pub fn count_between(
    repo: &RepoHandle,
    exclude: Oid,
    include: Oid,
    stop: &dyn Stop,
) -> Result<usize, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--count"),
            GitArg::Oid(include),
            GitArg::Word(format!("^{exclude}")),
        ])
        .run(stop)?;
    let text = String::from_utf8_lossy(output.stdout.trim_ascii());
    text.parse().map_err(|why| {
        RunError::Unparsable(format!(
            "rev-list --count printed {text:?}, not a count: {why}"
        ))
    })
}

/// The newest `cap` commits reachable from `include`, with no lower bound
/// (`rev-list <include>`), oldest first: [`commits_between`](super::commits_between)
/// without an `exclude`. For a tip's own commit, or the recent history of
/// a branch nothing else holds apart (ease-pass 01).
///
/// # Errors
/// [`RunError`] from the runner: git missing, stopped, or `include` naming
/// no commit in this repository.
pub fn commits_from(
    repo: &RepoHandle,
    include: Oid,
    cap: usize,
    stop: &dyn Stop,
) -> Result<Range, RunError> {
    super::rev_list(repo, cap, vec![GitArg::Oid(include)], stop)
}

/// [`only_in_stash`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashOnly {
    /// (commit, n): n is the newest stash entry (`stash@{n}`) whose first
    /// parent reaches it.
    pub commits: Vec<(Oid, usize)>,
    /// More such commits exist past `cap`.
    pub capped: bool,
    /// Entries passed over because their first parent is not readable
    /// (pruned, or the entry itself gone).
    pub skipped: usize,
}

/// The commits a stash entry's **first parent** reaches that no branch,
/// tag, remote or `HEAD` does: the base a stash was made on after that base
/// was rebased away. Never an entry's index or untracked commits (its 2nd
/// and 3rd parents). The app's own refs hold nothing, as in `unreached`.
///
/// `stashes` is newest first (`stash@{0}` first). Each commit is paired
/// with the newest entry reaching it; the answer is ordered newest entry
/// first. Method: one `cat-file --batch-check` for every entry's `^1`, then
/// **one** `rev-list --parents --stdin --not --branches --tags --remotes`
/// fed every base and `^HEAD`, each commit then claimed in-process by the
/// newest entry whose base reaches it. One `rev-list` per entry was
/// measured first and missed the ticket's bound: 356 ms p95 for one entry
/// on 100k commits / 50 652 refs, against `unreached`'s 190 ms
/// (`docs/perf/stash-only-cost.txt`). Not capped inside git: `cap` cuts the
/// answer, and a walk past the runner's output cap is `Unparsable`.
///
/// # Errors
/// [`RunError`] from the runner, `Stopped` included; never a partial answer.
pub fn only_in_stash(
    repo: &RepoHandle,
    stashes: &[Oid],
    cap: usize,
    stop: &dyn Stop,
) -> Result<StashOnly, RunError> {
    if stop.should_stop() {
        return Err(RunError::Stopped);
    }
    let mut answer = StashOnly {
        commits: Vec::new(),
        capped: false,
        skipped: 0,
    };
    if stashes.is_empty() {
        return Ok(answer);
    }
    let bases = first_parents(repo, stashes, stop)?;
    answer.skipped = bases.iter().filter(|base| base.is_none()).count();
    let live: Vec<(usize, Oid)> = bases
        .into_iter()
        .enumerate()
        .filter_map(|(n, base)| base.map(|base| (n, base)))
        .collect();
    if live.is_empty() {
        return Ok(answer);
    }

    let (order, parents) = walk_unheld(repo, &live, stop)?;
    let owner = claim(&live, &parents);
    let mut commits: Vec<(Oid, usize)> = order
        .into_iter()
        .filter_map(|id| owner.get(&id).map(|&n| (id, n)))
        .collect();
    // Newest entry first, git's order within each (stable sort).
    commits.sort_by_key(|&(_, n)| n);
    answer.capped = commits.len() > cap;
    commits.truncate(cap);
    answer.commits = commits;
    Ok(answer)
}

/// What the live bases reach and nothing holds, in git's order, with each
/// commit's parents: one `rev-list --parents --stdin` for every entry.
fn walk_unheld(
    repo: &RepoHandle,
    live: &[(usize, Oid)],
    stop: &dyn Stop,
) -> Result<(Vec<Oid>, Parents), RunError> {
    // Positives on stdin, `^HEAD` there too (a `--not` line on stdin is
    // refused before git 2.42; see `unreached`).
    let mut input = String::new();
    for (_, base) in live {
        input.push_str(&format!("{base}\n"));
    }
    input.push_str("^HEAD\n");
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            // An unborn `HEAD` holds nothing: `^HEAD` is dropped, not fatal.
            // Positive lines are safe: `first_parents` already read each.
            GitArg::Flag("--ignore-missing"),
            GitArg::Flag("--stdin"),
            GitArg::Flag("--not"),
            GitArg::Flag("--branches"),
            GitArg::Flag("--tags"),
            GitArg::Flag("--remotes"),
        ])
        .stdin(input.into_bytes())
        .run(stop)?;
    if output.truncated {
        return Err(RunError::Unparsable(
            "rev-list's stash-only history outgrew the output cap".to_owned(),
        ));
    }
    // Every printed commit, in git's order, with its parents.
    let mut order = Vec::new();
    let mut parents: HashMap<Oid, Vec<Oid>> = HashMap::new();
    for line in output.stdout.split(|&byte| byte == b'\n') {
        let mut ids = line
            .split(|&byte| byte == b' ')
            .filter(|id| !id.is_empty())
            .map(|id| parse_id(id, "rev-list"));
        let Some(id) = ids.next().transpose()? else {
            continue;
        };
        order.push(id);
        parents.insert(id, ids.collect::<Result<_, _>>()?);
    }
    Ok((order, parents))
}

/// Each printed commit to the newest entry that reaches it: walk the
/// printed commits from each base, newest entry first, claiming what is
/// unclaimed.
fn claim(live: &[(usize, Oid)], parents: &HashMap<Oid, Vec<Oid>>) -> HashMap<Oid, usize> {
    let mut owner: HashMap<Oid, usize> = HashMap::new();
    for &(n, base) in live {
        let mut todo = vec![base];
        while let Some(id) = todo.pop() {
            let Some(up) = parents.get(&id) else {
                continue; // held: git did not print it
            };
            if owner.contains_key(&id) {
                continue;
            }
            owner.insert(id, n);
            todo.extend(up.iter().copied());
        }
    }
    owner
}

/// Each entry's first parent, `None` where git cannot read one: one
/// `cat-file --batch-check`, fed `<entry>^1` per line.
fn first_parents(
    repo: &RepoHandle,
    stashes: &[Oid],
    stop: &dyn Stop,
) -> Result<Vec<Option<Oid>>, RunError> {
    let mut input = String::new();
    for entry in stashes {
        input.push_str(&format!("{entry}^1\n"));
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Word("--batch-check=%(objectname) %(objecttype)".to_owned()),
        ])
        .stdin(input.into_bytes())
        .run(stop)?;
    let lines: Vec<&[u8]> = output
        .stdout
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != stashes.len() {
        return Err(RunError::Unparsable(format!(
            "cat-file answered {} lines for {} stash entries",
            lines.len(),
            stashes.len()
        )));
    }
    lines
        .into_iter()
        .map(|line| match line.strip_suffix(b" commit") {
            Some(hex) => parse_id(hex, "cat-file").map(Some),
            // `<input> missing` (or a non-commit): nothing readable to walk.
            None => Ok(None),
        })
        .collect()
}

fn parse_id(hex: &[u8], who: &str) -> Result<Oid, RunError> {
    let text = String::from_utf8_lossy(hex);
    Oid::from_hex(&text)
        .map_err(|why| RunError::Unparsable(format!("{who} printed {text:?}, not an id: {why}")))
}
