//! A generated history shaped like a team monorepo (m7-power-tools/41).
//!
//! The 100k [`synthetic`](crate) history is the wrong shape for scroll
//! performance: what costs a graph row is lane width and how often the
//! selection's thread changes from row to row, not the commit count. A team
//! repository has many short feature branches, many open at once, each merged
//! into `main` by a merge commit. This module generates that shape.
//!
//! - `main` advances only by two-parent merges (after its root commit).
//! - Each feature branch has 1–`max_branch_len` commits, forks from a `main`
//!   commit up to [`FORK_WINDOW`] rows back, and is merged into `main`.
//! - About `open` branches are open (forked, not yet merged) at any row.
//! - One branch in five merges `main` into itself once before its own merge.
//! - Refs ([`RefMix`]) are packed, like a fetched repository; optionally a
//!   split commit-graph chain.
//!
//! One `git fast-import` process writes every object, timed off the stream's
//! fixed clock, so object ids are byte-identical run to run. Not behind
//! `heavy-tests`: the fast tier borrows [`TeamSpec::SMALL`].

use std::io;
use std::path::{Path, PathBuf};

use crate::cache::{self, CacheError};
use crate::digest;
use crate::git::{self, GitError};
use crate::permissions;
use crate::stream::{Mark, Stream};

/// How far back, in `main` commits, a feature branch may fork.
const FORK_WINDOW: u16 = 200;

/// The branch every feature lands on.
const MAIN: &[u8] = b"refs/heads/main";

/// Every branch whose number is a multiple of this merges `main` into itself
/// once before its own merge.
const MAIN_INTO_FEATURE_EVERY: usize = 5;

/// Where every feature commit is written before the final refs are pointed:
/// fast-import needs a ref per commit, and most branches keep none. Deleted
/// before the repository is published.
const SCRATCH_REF: &str = "refs/team-scratch";

/// The generator's one tally that cannot be read back from git, written
/// beside the repository's own files so [`stats`] can report it.
const MOST_OPEN_FILE: &str = "graft-team-most-open";

/// Directory prefix in the fixture cache; distinct from `synthetic-`.
const PREFIX: &str = "team-";

/// How many refs of each kind the repository carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RefMix {
    /// Branches under `refs/heads/feature/*`: the most recent branch tips.
    pub local: u32,
    /// Branches under `refs/remotes/origin/feature/*`: the most recent tips.
    pub remote: u32,
    /// Tags under `refs/tags/v*`, on `main` merges spread through history.
    pub tags: u32,
    /// How many of `tags` are annotated (the first ones).
    pub annotated: u32,
}

/// What to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TeamSpec {
    /// Total commits, merges included.
    pub commits: u32,
    /// Branches open at any row, steady state.
    pub open: u16,
    /// Longest feature branch, in its own commits (merges of `main` excluded).
    pub max_branch_len: u16,
    /// Refs besides `main`.
    pub refs: RefMix,
    /// Write a split commit-graph chain.
    pub split_commit_graph: bool,
}

impl TeamSpec {
    /// His scale: 17 700 commits, open 20, `max_branch_len` 6, refs 330 local /
    /// 900 remote / 100 tags (50 annotated) = 1 330 refs, split chain.
    pub const MONOREPO: TeamSpec = TeamSpec {
        commits: 17_700,
        open: 20,
        max_branch_len: 6,
        refs: RefMix {
            local: 330,
            remote: 900,
            tags: 100,
            annotated: 50,
        },
        split_commit_graph: true,
    };

    /// Fast tier: 400 commits, open 10, `max_branch_len` 4, refs 30/30/5
    /// (2 annotated), no commit-graph.
    pub const SMALL: TeamSpec = TeamSpec {
        commits: 400,
        open: 10,
        max_branch_len: 4,
        refs: RefMix {
            local: 30,
            remote: 30,
            tags: 5,
            annotated: 2,
        },
        split_commit_graph: false,
    };
}

/// What a built repository holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeamStats {
    /// `git rev-list --count --all`.
    pub commits: u32,
    /// `git rev-list --merges --count --all`.
    pub merges: u32,
    /// Every ref except `refs/heads/main`.
    pub refs: u32,
    /// The most feature branches forked and not yet merged at any `main`
    /// commit. From the generator's plan, **not** re-read from git (a merged
    /// branch leaves no trace of when it forked that git can count).
    pub most_open: u16,
}

/// Why the team repository could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum TeamError {
    /// A file could not be written or read.
    #[error("team fixture: {0}")]
    Io(#[from] io::Error),
    /// A `git` command failed.
    #[error("team fixture: {0}")]
    Git(#[from] GitError),
    /// It could not be cached, published or measured.
    #[error("team fixture: {0}")]
    Cache(#[from] CacheError),
    /// The spec cannot be built as asked, or git's output is not what was
    /// expected.
    #[error("team fixture: {0}")]
    Spec(&'static str),
}

/// Builds into `into`, an existing empty directory. Deterministic: the same
/// spec gives the same object ids.
pub fn build(spec: &TeamSpec, into: &Path) -> Result<TeamStats, TeamError> {
    let plan = Plan::generate(spec)?;
    git::init_repo(into)?;

    let stream_path = into.join("team.fi");
    std::fs::write(&stream_path, plan.stream)?;
    git::run_with_stdin_file(into, &["fast-import", "--quiet", "--done"], &stream_path)?;
    // The stream must not survive into the published fixture.
    std::fs::remove_file(&stream_path)?;

    git::run(into, &["update-ref", "-d", SCRATCH_REF])?;
    git::run(into, &["pack-refs", "--all", "--prune"])?;
    git::run(into, &["reset", "-q", "--hard", "refs/heads/main"])?;
    if spec.split_commit_graph {
        git::run(into, &["commit-graph", "write", "--reachable", "--split"])?;
    }
    std::fs::write(
        into.join(".git").join(MOST_OPEN_FILE),
        plan.most_open.to_string(),
    )?;
    stats(into)
}

/// A read-only cached copy (`team-`, key `digest::team_key`), built if missing.
pub fn borrow(spec: &TeamSpec) -> Result<PathBuf, TeamError> {
    let base = cache::base_dir();
    let key = digest::team_key(spec)?;
    let root = base.join(format!("{PREFIX}{}-{key}", spec.commits));
    if cache::is_built(&root) {
        return Ok(root);
    }
    std::fs::create_dir_all(&base)?;
    let staging = cache::staging_in(&base, PREFIX)?;
    build(spec, staging.path())?;
    std::fs::write(
        staging.path().join(cache::MARKER),
        format!("{spec:?}\n{}\n", git::version()?),
    )?;
    permissions::seal(staging.path())?;
    cache::claim(&staging.keep(), &root)?;
    Ok(root)
}

/// Counted back from the repository with git (`rev-list`, `for-each-ref`),
/// never from the generator's tallies — except `most_open`, which git
/// cannot know (see [`TeamStats::most_open`]).
pub fn stats(repo: &Path) -> Result<TeamStats, TeamError> {
    let commits = count(&git::run(repo, &["rev-list", "--count", "--all"])?)?;
    let merges = count(&git::run(
        repo,
        &["rev-list", "--merges", "--count", "--all"],
    )?)?;
    let listed = git::run(repo, &["for-each-ref", "--format=%(refname)"])?;
    let refs = String::from_utf8_lossy(&listed)
        .lines()
        .filter(|name| *name != "refs/heads/main")
        .count();
    let most_open = std::fs::read_to_string(repo.join(".git").join(MOST_OPEN_FILE))?;
    Ok(TeamStats {
        commits,
        merges,
        refs: u32::try_from(refs).map_err(|_| TeamError::Spec("more than u32::MAX refs"))?,
        most_open: most_open
            .trim()
            .parse()
            .map_err(|_| TeamError::Spec("most-open file is not a number"))?,
    })
}

fn count(out: &[u8]) -> Result<u32, TeamError> {
    String::from_utf8_lossy(out)
        .trim()
        .parse()
        .map_err(|_| TeamError::Spec("git printed something that is not a count"))
}

/// A feature branch in flight.
struct Branch {
    number: usize,
    /// The `main` commit it forks from.
    fork: Mark,
    /// Its own commits, planned.
    len: u16,
    /// Its own commits so far.
    done: u16,
    /// Its tip, once it has a commit.
    tip: Option<Mark>,
    /// Still to merge `main` into itself.
    wants_main: bool,
}

/// The stream, and the one tally git cannot give back.
struct Plan {
    stream: Vec<u8>,
    most_open: u16,
}

/// A fixed-seed xorshift: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    /// A number in `0..n`; `n` must be at least 1.
    fn below(&mut self, n: u16) -> u16 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        // The remainder is below `n`, a `u16`, so the conversion cannot
        // fail; the fallback is never taken.
        u16::try_from(self.0 % u64::from(n)).unwrap_or(0)
    }
}

impl Branch {
    /// A new branch forking from one of the last [`FORK_WINDOW`] `main` commits.
    fn fork(number: usize, spec: &TeamSpec, main: &[Mark], rng: &mut Rng) -> Branch {
        let window = u16::try_from(main.len()).map_or(FORK_WINDOW, |len| len.min(FORK_WINDOW));
        Branch {
            number,
            fork: main[main.len() - 1 - usize::from(rng.below(window))],
            len: 1 + rng.below(spec.max_branch_len),
            done: 0,
            tip: None,
            wants_main: number.is_multiple_of(MAIN_INTO_FEATURE_EVERY),
        }
    }

    /// Writes this branch's next commit: one of its own, then (one branch in
    /// five) a merge of `main` into it, then its merge into `main` — whose
    /// mark is returned, the branch being done.
    fn advance(&mut self, out: &mut Stream, main_tip: Mark) -> Option<Mark> {
        let name = format!("feature/{:05}", self.number);
        let path = format!("{name}.txt");
        let tip = self.tip.unwrap_or(self.fork);
        let scratch = SCRATCH_REF.as_bytes();
        if self.done < self.len {
            self.done += 1;
            let content = format!("{name} change {}\n", self.done);
            let message = format!("{name}: change {}\n", self.done);
            let file = (path.as_bytes(), content.as_bytes());
            self.tip = Some(out.commit(scratch, message.as_bytes(), &[tip], &[file]));
            None
        } else if self.wants_main {
            let message = format!("Merge branch 'main' into {name}\n");
            let parents = [tip, main_tip];
            self.tip = Some(out.commit(scratch, message.as_bytes(), &parents, &[]));
            self.wants_main = false;
            None
        } else {
            // fast-import starts a merge from its first parent's tree, so the
            // branch's file is spelled out to land on `main`.
            let content = format!("{name} change {}\n", self.done);
            let message = format!("Merge branch '{name}' into main\n");
            let file = (path.as_bytes(), content.as_bytes());
            Some(out.commit(MAIN, message.as_bytes(), &[main_tip, tip], &[file]))
        }
    }
}

impl Plan {
    fn generate(spec: &TeamSpec) -> Result<Plan, TeamError> {
        if spec.commits < 2 || spec.open == 0 || spec.max_branch_len == 0 {
            return Err(TeamError::Spec(
                "need commits ≥ 2, open ≥ 1, max_branch_len ≥ 1",
            ));
        }
        if spec.refs.annotated > spec.refs.tags {
            return Err(TeamError::Spec("more annotated tags than tags"));
        }
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut out = Stream::new();
        let mut main = vec![out.commit(MAIN, b"Initial commit\n", &[], &[])];
        let mut written: u32 = 1;
        let mut open: Vec<Branch> = Vec::new();
        // Every branch's tip, in the order branches were opened.
        let mut tips: Vec<(usize, Mark)> = Vec::new();
        let mut next_number = 0usize;
        let mut most_open = 0usize;

        while written < spec.commits {
            while open.len() < usize::from(spec.open) {
                open.push(Branch::fork(next_number, spec, &main, &mut rng));
                next_number += 1;
            }
            // `open.len()` equals `spec.open`, a `u16`, here.
            let pick = usize::from(rng.below(spec.open));
            let main_tip = main[main.len() - 1];
            written += 1;
            if let Some(merged) = open[pick].advance(&mut out, main_tip) {
                main.push(merged);
                let branch = open.swap_remove(pick);
                tips.push((branch.number, branch.tip.unwrap_or(branch.fork)));
                most_open = most_open.max(open.len());
            }
        }
        // Open branches keep their tips too; ordered by number so the
        // newest-branch refs include them.
        for branch in &open {
            if let Some(tip) = branch.tip {
                tips.push((branch.number, tip));
            }
        }
        tips.sort_by_key(|(number, _)| *number);
        point_refs(spec, &mut out, &tips, &main)?;
        Ok(Plan {
            stream: out.finish(),
            most_open: u16::try_from(most_open).map_err(|_| TeamError::Spec("open > u16"))?,
        })
    }
}

/// Points the local, remote and tag refs at the newest tips and at `main`
/// merges spread evenly through history.
fn point_refs(
    spec: &TeamSpec,
    out: &mut Stream,
    tips: &[(usize, Mark)],
    main: &[Mark],
) -> Result<(), TeamError> {
    let mix = spec.refs;
    let newest = |n: u32| -> Result<&[(usize, Mark)], TeamError> {
        let n = n as usize;
        tips.len()
            .checked_sub(n)
            .map(|from| &tips[from..])
            .ok_or(TeamError::Spec("fewer branches than refs asked for"))
    };
    for (number, tip) in newest(mix.local)? {
        out.point(format!("refs/heads/feature/{number:05}").as_bytes(), *tip);
    }
    for (number, tip) in newest(mix.remote)? {
        out.point(
            format!("refs/remotes/origin/feature/{number:05}").as_bytes(),
            *tip,
        );
    }
    let tags = mix.tags as usize;
    if tags > main.len() {
        return Err(TeamError::Spec("more tags than main commits"));
    }
    for n in 0..mix.tags {
        let at = main[(n as usize + 1) * main.len() / (tags + 1)];
        let name = format!("v0.{n}");
        if n < mix.annotated {
            out.annotated_tag(name.as_bytes(), at, format!("Release {name}\n").as_bytes());
        } else {
            out.point(format!("refs/tags/{name}").as_bytes(), at);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn built() -> (tempfile::TempDir, TeamStats) {
        let dir = tempfile::tempdir().expect("tempdir");
        let stats = build(&TeamSpec::SMALL, dir.path()).expect("build SMALL");
        (dir, stats)
    }

    fn text(repo: &Path, args: &[&str]) -> String {
        String::from_utf8(git::run(repo, args).expect("git")).expect("utf-8")
    }

    #[test]
    fn small_shape() {
        let (dir, built) = built();
        let read = stats(dir.path()).expect("stats");
        assert_eq!(built, read);
        assert_eq!(read.commits, 400);
        assert_eq!(read.refs, 65);
        assert!(read.merges >= 40, "{read:?}");
        assert!(read.most_open >= 8, "{read:?}");
        let refs = text(dir.path(), &["for-each-ref", "--format=%(refname)"]);
        assert_eq!(
            refs.lines()
                .filter(|r| r.starts_with("refs/heads/feature/"))
                .count(),
            30
        );
        assert_eq!(
            refs.lines()
                .filter(|r| r.starts_with("refs/remotes/origin/"))
                .count(),
            30
        );
        assert_eq!(
            refs.lines()
                .filter(|r| r.starts_with("refs/tags/v"))
                .count(),
            5
        );
        let tag_kinds = text(
            dir.path(),
            &["for-each-ref", "--format=%(objecttype)", "refs/tags"],
        );
        assert_eq!(tag_kinds.lines().filter(|k| *k == "tag").count(), 2);
        // Packed, like a fetched repository: no loose ref files.
        assert!(dir.path().join(".git/packed-refs").is_file());
        assert!(!dir.path().join(".git/refs/heads/main").exists());
    }

    #[test]
    fn build_is_deterministic() {
        let (a, _) = built();
        let (b, _) = built();
        let refs = ["for-each-ref", "--format=%(objectname) %(refname)"];
        assert_eq!(
            text(a.path(), &["rev-parse", "main"]),
            text(b.path(), &["rev-parse", "main"])
        );
        assert_eq!(text(a.path(), &refs), text(b.path(), &refs));
    }

    #[test]
    fn main_advances_by_merges() {
        let (dir, _) = built();
        let first_parent = text(
            dir.path(),
            &[
                "rev-list",
                "--first-parent",
                "--format=%P%x09%s",
                "--no-commit-header",
                "main",
            ],
        );
        let lines: Vec<&str> = first_parent.lines().collect();
        assert!(lines.len() > 40);
        // Every main commit but the root is a two-parent merge.
        for line in &lines[..lines.len() - 1] {
            let (parents, subject) = line.split_once('\t').expect("tab");
            let parents: Vec<&str> = parents.split(' ').collect();
            assert_eq!(parents.len(), 2, "{line}");
            assert!(subject.starts_with("Merge branch 'feature/"), "{line}");
            let second = text(dir.path(), &["log", "-1", "--format=%s", parents[1]]);
            assert!(
                second.starts_with("feature/")
                    || second.starts_with("Merge branch 'main' into feature/"),
                "second parent of {line} is {second}"
            );
        }
        let feature_merges = text(dir.path(), &["log", "--all", "--format=%s", "--merges"]);
        assert!(
            feature_merges
                .lines()
                .any(|s| s.starts_with("Merge branch 'main' into feature/"))
        );
    }

    #[test]
    fn small_is_fsck_clean() {
        let (dir, _) = built();
        git::run(dir.path(), &["fsck", "--strict"]).expect("fsck --strict clean");
    }

    #[test]
    fn borrow_is_cached_apart_from_synthetic() {
        let first = borrow(&TeamSpec::SMALL).expect("borrow");
        let marker = first.join(cache::MARKER);
        let built_at = std::fs::metadata(&marker)
            .and_then(|m| m.modified())
            .expect("marker");
        let second = borrow(&TeamSpec::SMALL).expect("borrow again");
        assert_eq!(first, second);
        let again = std::fs::metadata(&marker)
            .and_then(|m| m.modified())
            .expect("marker");
        assert_eq!(built_at, again, "second borrow rebuilt");
        let name = first.file_name().and_then(|n| n.to_str()).expect("name");
        assert!(
            name.starts_with(PREFIX) && !name.starts_with("synthetic-"),
            "{name}"
        );
        assert_eq!(stats(&first).expect("stats").commits, 400);
    }
}
