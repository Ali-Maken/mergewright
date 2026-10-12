//! Which reflog candidates nothing holds any more: the lost commits `J`
//! shows (m7-power-tools/19, ADR-0046 "reachability is measured, not
//! assumed").
//!
//! One `git rev-list`, measured against the other two methods the ticket
//! named on a 50 000-ref repository (`docs/perf/lost-commit-cost.txt`,
//! `scripts/m7-perf-lost.sh`):
//!
//! ```text
//! git rev-list --parents --ignore-missing --stdin --not --branches --tags --remotes
//!   stdin: <candidate>…  ^HEAD  ^<every stash entry>…
//! ```
//!
//! Every commit it prints is reachable from a candidate and from nothing
//! held, so a candidate printed is lost, and the printed parent lines give
//! each lost candidate's lost first-parent ancestors. The stash entries come
//! from a first, cheap `git rev-list -g --ignore-missing refs/stash`:
//! `stash@{1}` and older live only in `refs/stash`'s reflog, and each one
//! holds its commits. The command line's `--not` does not reach stdin, so
//! the candidates stay positive, and the negatives on stdin are `^<rev>`
//! lines, not a `--not` line: git before 2.42 refuses any option on stdin
//! (checked on 2.38.5 and 2.54, m7-power-tools/18b).
//!
//! **Held** is exactly: `refs/heads/*`, `refs/tags/*` (git peels an
//! annotated tag itself), `refs/remotes/*`, every stash entry, and `HEAD`
//! (which only adds anything when detached). Never the app's own refs
//! (under any generation's `Generation::ref_prefix`): a commit only Graft's own backup
//! keeps is lost to the person using it.

use std::collections::{HashMap, HashSet};

use graft_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// The most first-parent ancestors a run counts under its tip.
const UNDER_CAP: usize = 999;

/// A run of lost commits: `tip`, the newest, and how many of its
/// first-parent ancestors are lost with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LostRun {
    /// The newest lost commit of the run: a lost candidate no other lost
    /// candidate's first-parent chain passes through.
    pub tip: Oid,
    /// Its lost first-parent ancestors, capped at 999. The candidates among
    /// them fold into this run rather than making runs of their own.
    pub under: usize,
}

/// [`unreached`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreached {
    /// The runs, in the order their tips were given as candidates.
    pub lost: Vec<LostRun>,
    /// How many candidates were checked: all of them, or `cap`.
    pub checked: usize,
    /// Candidates past `cap` were left unchecked.
    pub capped: bool,
}

/// The candidates nothing holds any more, grouped into [`LostRun`]s.
///
/// `candidates` is newest first (the reflogs' old values); only the first
/// `cap` are checked. A candidate git no longer has at all (pruned) is
/// passed over, not reported: there is nothing left to recover. Two
/// spawns, both stoppable.
///
/// # Errors
/// [`RunError`] from the runner, `Stopped` included; `Unparsable` when
/// `rev-list`'s answer is not the shape read here or was cut at the
/// runner's output cap (more lost history than 64 MiB of ids).
pub fn unreached(
    repo: &RepoHandle,
    candidates: &[Oid],
    cap: usize,
    stop: &dyn Stop,
) -> Result<Unreached, RunError> {
    let checked = candidates.len().min(cap);
    let capped = candidates.len() > cap;
    let candidates = &candidates[..checked];
    if stop.should_stop() {
        return Err(RunError::Stopped);
    }
    if candidates.is_empty() {
        return Ok(Unreached {
            lost: Vec::new(),
            checked,
            capped,
        });
    }

    let stash = stash_entries(repo, stop)?;
    let mut input = String::new();
    for candidate in candidates {
        input.push_str(&format!("{candidate}\n"));
    }
    // Negatives as `^<rev>` lines: `--not` on stdin is refused before git
    // 2.42 ("options not supported in --stdin mode", seen on 2.38.5), and
    // `^<rev>` is read the same by every version.
    input.push_str("^HEAD\n");
    for entry in &stash {
        input.push_str(&format!("^{entry}\n"));
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
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
            "rev-list's lost history outgrew the output cap".to_owned(),
        ));
    }
    let first_parent = parse_parents(&output.stdout)?;
    Ok(Unreached {
        lost: fold(candidates, &first_parent),
        checked,
        capped,
    })
}

/// Every stash entry, `stash@{0}` first; none without a `refs/stash`.
fn stash_entries(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<Oid>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--walk-reflogs"),
            GitArg::Flag("--ignore-missing"),
            GitArg::Ref(b"refs/stash".to_vec()),
        ])
        .run(stop)?;
    output
        .stdout
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(parse_id)
        .collect()
}

/// `rev-list --parents` lines, `<id> [<parent>…]`, as each printed
/// commit's first parent (`None` for a root).
fn parse_parents(stdout: &[u8]) -> Result<HashMap<Oid, Option<Oid>>, RunError> {
    let mut first_parent = HashMap::new();
    for line in stdout.split(|&byte| byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split(|&byte| byte == b' ');
        // `split` yields at least one field, even for a line with no space.
        let commit = parse_id(fields.next().unwrap_or(line))?;
        let parent = fields.next().map(parse_id).transpose()?;
        first_parent.insert(commit, parent);
    }
    Ok(first_parent)
}

fn parse_id(hex: &[u8]) -> Result<Oid, RunError> {
    let text = String::from_utf8_lossy(hex);
    Oid::from_hex(&text)
        .map_err(|why| RunError::Unparsable(format!("rev-list printed {text:?}, not an id: {why}")))
}

/// Folds the lost candidates (those `lost` holds) into runs. A candidate on
/// another lost candidate's first-parent chain belongs to that one's run,
/// whichever of the two came first in `candidates`: every chain is walked
/// once, stopping where an earlier walk already went.
fn fold(candidates: &[Oid], lost: &HashMap<Oid, Option<Oid>>) -> Vec<LostRun> {
    let is_candidate: HashSet<Oid> = candidates.iter().copied().collect();
    let mut visited = HashSet::new();
    let mut absorbed = HashSet::new();
    for &candidate in candidates {
        if !lost.contains_key(&candidate) || !visited.insert(candidate) {
            continue;
        }
        let mut at = lost.get(&candidate).copied().flatten();
        while let Some(commit) = at.filter(|commit| lost.contains_key(commit)) {
            if is_candidate.contains(&commit) {
                absorbed.insert(commit);
            }
            if !visited.insert(commit) {
                break;
            }
            at = lost.get(&commit).copied().flatten();
        }
    }

    let mut runs = Vec::new();
    let mut emitted = HashSet::new();
    for &candidate in candidates {
        if lost.contains_key(&candidate)
            && !absorbed.contains(&candidate)
            && emitted.insert(candidate)
        {
            runs.push(LostRun {
                tip: candidate,
                under: under(candidate, lost),
            });
        }
    }
    runs
}

/// How many of `tip`'s first-parent ancestors are lost, up to
/// [`UNDER_CAP`].
fn under(tip: Oid, lost: &HashMap<Oid, Option<Oid>>) -> usize {
    let mut count = 0;
    let mut at = lost.get(&tip).copied().flatten();
    while let Some(commit) = at.filter(|commit| lost.contains_key(commit)) {
        if count == UNDER_CAP {
            break;
        }
        count += 1;
        at = lost.get(&commit).copied().flatten();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Oid {
        Oid::from_hex(&format!("{n:02x}").repeat(20)).expect("a valid id")
    }

    #[test]
    fn under_stops_at_the_cap() {
        let mut lost = HashMap::new();
        // A lost chain of 1 200 commits, ids from a counter.
        let chain: Vec<Oid> = (0u32..1200)
            .map(|n| Oid::from_hex(&format!("{n:040x}")).expect("a valid id"))
            .collect();
        for pair in chain.windows(2) {
            lost.insert(pair[0], Some(pair[1]));
        }
        lost.insert(chain[1199], None);
        assert_eq!(under(chain[0], &lost), UNDER_CAP);
        assert_eq!(under(chain[1000], &lost), 199);
    }

    #[test]
    fn parse_parents_reads_roots_and_merges() {
        let text = format!("{} {} {}\n{}\n", id(1), id(2), id(3), id(2));
        let parsed = parse_parents(text.as_bytes()).expect("parses");
        assert_eq!(parsed.get(&id(1)), Some(&Some(id(2))));
        assert_eq!(parsed.get(&id(2)), Some(&None));
    }

    #[test]
    fn parse_parents_refuses_a_line_that_is_not_an_id() {
        assert!(matches!(
            parse_parents(b"not-an-id\n"),
            Err(RunError::Unparsable(_))
        ));
    }
}
