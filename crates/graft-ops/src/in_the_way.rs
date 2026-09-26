//! The one read of "his files in a write's way" (ADR-0049): paths on disk
//! that git doesn't keep — untracked or ignored — where an operation would
//! write one of its own.
//!
//! Decided from the disk, never from `git status` alone: status never lists
//! an **ignored** file, yet `switch`, `merge`, `reset --hard` and the rest
//! write over one with rc 0, and the backup (`add -A`) can't hold it, so
//! `u` could not bring it back (review m7-22, C1; scout 28's probe). The
//! answer is to refuse: [`crate::preview::plan`] and `apply` both ask
//! [`blocking`] before anything is written, so a refusal leaves no journal
//! line.
//!
//! Spawns, whatever the size of the change: one for the paths the
//! operation writes (a few more for a stash: its parents and each of its
//! trees; for a replay, a `rev-list` of the replayed commits and one
//! `diff-tree --stdin` over all of them), one `ls-files` for the index.
//! Never one per path or per commit.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;
use crate::operation::{
    CheckoutTarget, Operation, RecoverTarget, Refusal, RepoFacts, ResetMode, ResetTarget, Source,
    branch_ref, short, short_ref,
};
use crate::rewrite::TodoAction;
use crate::snapshot::{SnapshotError, tracked_paths_now};

/// Paths on disk that git doesn't keep and that the operation would write
/// over. Each list is sorted and unique.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blocking {
    /// Not listed by `git status`: a `.gitignore` or `info/exclude` rule
    /// hides it. Neither `S` nor `c` takes one; he moves it aside.
    pub ignored: Vec<PathBuf>,
    /// Listed by `git status` as untracked: `S` (`stash push -u`) takes it.
    pub untracked: Vec<PathBuf>,
}

impl Blocking {
    /// Nothing in the way.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ignored.is_empty() && self.untracked.is_empty()
    }

    /// Both lists as one, sorted: what a stash's refusal names.
    fn all(self) -> Vec<PathBuf> {
        let mut all: BTreeSet<PathBuf> = self.ignored.into_iter().collect();
        all.extend(self.untracked);
        all.into_iter().collect()
    }
}

/// The one read. Empty for a kind ADR-0049 §2 doesn't cover, and in a
/// repository with no working tree.
///
/// # Errors
/// [`FactsError`]: a `git` read failed, a path could not be looked for on
/// disk ([`FactsError::OnDisk`]), or the caller stopped
/// ([`FactsError::Stopped`]).
pub fn blocking(
    repo: &RepoHandle,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Blocking, FactsError> {
    if repo.workdir().is_none() {
        // A bare repository has no files to write over.
        return Ok(Blocking::default());
    }
    let Some(written) = written_paths(repo, op, facts, stop)? else {
        return Ok(Blocking::default());
    };
    if written.is_empty() {
        return Ok(Blocking::default());
    }
    let tracked = tracked_paths_now(repo, stop, &mut Vec::new()).map_err(snapshot_error)?;
    // A tracked path is the set-aside's or git's own check's (ADR-0045 §4),
    // never his file in the way.
    let candidates: BTreeSet<PathBuf> = written
        .into_iter()
        .filter(|path| !tracked.contains(path))
        .collect();
    let found = on_disk(repo, &candidates, &tracked)?;
    Ok(sort_out(found, facts))
}

/// `Some(InTheWay { .. })` when not empty, `None` when empty. Stash kinds
/// never call this; they build [`Refusal::StashInTheWay`].
#[must_use]
pub fn refusal(op: &Operation, blocking: Blocking) -> Option<Refusal> {
    if blocking.is_empty() {
        return None;
    }
    Some(Refusal::InTheWay {
        // Every kind `written_paths` answers for has its words; a kind
        // without them never gets here with anything in the way.
        doing: doing(op).unwrap_or_else(|| op.describe()),
        ignored: blocking.ignored,
        untracked: blocking.untracked,
    })
}

/// The refusal `preview::plan` and `apply` both give: a stash put-back's in
/// the stash preview's own words (m6-conflicts/86, unchanged), every other
/// kind's through [`refusal`].
pub(crate) fn refused(op: &Operation, blocking: Blocking) -> Option<Refusal> {
    match op {
        Operation::StashApply { .. } | Operation::StashPop { .. } => {
            (!blocking.is_empty()).then(|| Refusal::StashInTheWay {
                unsaved: Vec::new(),
                staged: Vec::new(),
                untracked: blocking.all(),
            })
        }
        _ => refusal(op, blocking),
    }
}

/// `<doing>` in `<path> is in the way of <doing>`. `None` for a kind that
/// isn't covered.
#[must_use]
pub fn doing(op: &Operation) -> Option<String> {
    let words = match op {
        Operation::Checkout(params) => match &params.to {
            CheckoutTarget::Branch(name) => format!("switching to {}", short_ref(name)),
            CheckoutTarget::Detached(oid) => format!("switching to {}", short(*oid)),
        },
        Operation::BranchCreate(params) if params.checkout => {
            format!("switching to {}", params.name)
        }
        Operation::BranchFromStash(params) => format!("switching to {}", params.name),
        Operation::Merge(params) => format!("merging {}", source_words(&params.source)),
        Operation::Pull(_) => format!("pulling {}", op.remote_names()?.remote_side),
        Operation::Reset(params) => match &params.target {
            ResetTarget::Head {
                branch,
                mode: ResetMode::Hard,
            } => format!(
                "resetting {} to {}",
                short_ref(&branch.name),
                short(params.to)
            ),
            _ => return None,
        },
        Operation::Recover(params) => match &params.target {
            RecoverTarget::MoveRef { name, to, .. } => {
                format!("putting {name} back to {}", short(*to))
            }
            _ => return None,
        },
        Operation::Rebase(params) => format!("rebasing onto {}", source_words(&params.onto)),
        Operation::Rewrite(params) => format!("rewriting {}", short_ref(&params.head.name)),
        Operation::CherryPick(params) => match params.commits.as_slice() {
            [one] => format!("picking {}", short(*one)),
            several => format!("picking {} commits", several.len()),
        },
        Operation::Revert(params) => format!("reverting {}", short(params.commit)),
        _ => return None,
    };
    Some(words)
}

fn source_words(source: &Source) -> String {
    match source {
        Source::Branch(name) => short_ref(name).to_owned(),
        Source::Commit(oid) => short(*oid),
    }
}

/// ADR-0049 §4's table: every path the operation writes, uncapped.
/// `Ok(None)` = the kind writes no path the index doesn't track.
///
/// One deviation from the table, a superset: a hard reset (and a put-back
/// of the checked-out branch, which is one) writes every path where the
/// index differs from the target — `diff-index --cached <to>` — not only
/// `HEAD`'s diff to it. A file he `git rm --cached` is in no diff of
/// `HEAD`'s, yet `reset --hard` writes it back over his bytes.
pub(crate) fn written_paths(
    repo: &RepoHandle,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Option<BTreeSet<PathBuf>>, FactsError> {
    let paths = match op {
        Operation::Checkout(params) => from_head(repo, facts, landed(&params.to, facts), stop)?,
        Operation::BranchCreate(params) if params.checkout => {
            from_head(repo, facts, GitArg::Oid(params.at), stop)?
        }
        Operation::Merge(params) if unborn(facts) => {
            listed(repo, GitArg::Oid(params.source_tip), stop)?
        }
        // The incoming side: what the source changed since the merge base.
        // A fast-forward's base is `HEAD`, so then this is `HEAD`'s diff.
        Operation::Merge(params) => diff_tree(
            repo,
            &["--merge-base"],
            [head(), GitArg::Oid(params.source_tip)],
            stop,
        )?,
        // Before the fetch: its tip as last fetched. What the fetch brings
        // is guarded by git's own `--no-overwrite-ignore` (ADR-0049 §4).
        Operation::Pull(params) => match params.upstream.tracking_tip {
            Some(tip) => from_head(repo, facts, GitArg::Oid(tip), stop)?,
            None => BTreeSet::new(),
        },
        Operation::Reset(params) => match &params.target {
            ResetTarget::Head {
                mode: ResetMode::Hard,
                ..
            } => hard_reset(repo, params.to, stop)?,
            _ => return Ok(None),
        },
        Operation::Recover(params) => match &params.target {
            RecoverTarget::MoveRef { name, to, .. } if matches!(&facts.head, HeadRef::Branch(head) if *head == branch_ref(name)) => {
                hard_reset(repo, *to, stop)?
            }
            // Not checked out: an `update-ref` writes no file. A file
            // put-back writes only paths the backup holds, over his edits
            // `check` already refuses.
            _ => return Ok(None),
        },
        Operation::BranchFromStash(params) => {
            // `switch -c <name> stash^1`, then `stash pop --index`.
            let parents = parents_of(repo, params.commit, stop)?;
            let mut paths = match parents.first() {
                Some(base) => from_head(repo, facts, GitArg::Oid(*base), stop)?,
                None => BTreeSet::new(),
            };
            paths.extend(stash_paths(repo, params.commit, &parents, stop)?);
            paths
        }
        Operation::StashApply { commit, .. } | Operation::StashPop { commit, .. } => {
            let parents = parents_of(repo, *commit, stop)?;
            stash_paths(repo, *commit, &parents, stop)?
        }
        Operation::Rebase(_)
        | Operation::Rewrite(_)
        | Operation::CherryPick(_)
        | Operation::Revert(_) => replayed_paths(repo, op, stop)?,
        // Stage, unstage, discard, the conflict kinds, commits and every
        // ref-only kind write only paths the index tracks, or none.
        _ => return Ok(None),
    };
    Ok(Some(paths))
}

/// A replay's rows of ADR-0049 §4: a rebase or rewrite moves `HEAD` to
/// `onto` (git's `reset --hard`), then each replayed commit writes the
/// paths of its own change; a pick or revert only the latter. The engine's
/// `switch <home>` afterwards writes only paths in this union. Empty for a
/// kind that isn't a replay.
fn replayed_paths(
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
            let mut replayed: Vec<Replayed> = params
                .todo
                .iter()
                .filter(|line| line.action != TodoAction::Drop)
                .map(|line| Replayed::first_parent(line.commit))
                .collect();
            replayed.extend(params.made.iter().copied().map(Replayed::first_parent));
            // The dependents' commits follow his rows as picks, and git is
            // started on the top of the line.
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
            (Some(params.onto), replayed)
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
        Operation::Revert(params) => {
            let against = if let Some(parent) = params.parent {
                Some(parent)
            } else {
                // `-m n` reverts against parent `n`; no `-m`, the first.
                let nth = usize::from(params.mainline.unwrap_or(1)).saturating_sub(1);
                parents_of(repo, params.commit, stop)?.get(nth).copied()
            };
            let replayed = Replayed {
                commit: params.commit,
                against,
            };
            (None, vec![replayed])
        }
        _ => return Ok(BTreeSet::new()),
    };
    let mut paths = match onto {
        Some(onto) => diff_tree(repo, &[], [head(), GitArg::Oid(onto)], stop)?,
        None => BTreeSet::new(),
    };
    paths.extend(own_changes(repo, &replayed, stop)?);
    Ok(paths)
}

/// Where a checkout lands: the branch's tip as the facts read it.
fn landed(to: &CheckoutTarget, facts: &RepoFacts) -> GitArg {
    match to {
        CheckoutTarget::Branch(name) => facts
            .refs
            .get(name)
            .copied()
            .flatten()
            .map_or_else(|| GitArg::Ref(name.clone().into_bytes()), GitArg::Oid),
        CheckoutTarget::Detached(oid) => GitArg::Oid(*oid),
    }
}

fn head() -> GitArg {
    GitArg::Ref(b"HEAD".to_vec())
}

/// `HEAD`'s diff to where it lands; every path of the landed tree when
/// `HEAD` has no commit yet.
fn from_head(
    repo: &RepoHandle,
    facts: &RepoFacts,
    landed: GitArg,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    if unborn(facts) {
        return listed(repo, landed, stop);
    }
    diff_tree(repo, &[], [head(), landed], stop)
}

/// `diff-tree -r --name-only --no-renames -z [flags] <from> <to>`.
fn diff_tree(
    repo: &RepoHandle,
    flags: &[&'static str],
    [from, to]: [GitArg; 2],
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let mut args = vec![
        GitArg::Flag("diff-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("--name-only"),
        GitArg::Flag("--no-renames"),
        GitArg::Flag("-z"),
    ];
    args.extend(flags.iter().map(|flag| GitArg::Flag(flag)));
    args.extend([from, to]);
    names(repo, args, stop)
}

fn unborn(facts: &RepoFacts) -> bool {
    matches!(facts.head, HeadRef::Unborn(_))
}

/// Every path where the index differs from `to`: what `reset --hard <to>`
/// writes.
fn hard_reset(
    repo: &RepoHandle,
    to: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    names(
        repo,
        vec![
            GitArg::Flag("diff-index"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("-z"),
            GitArg::Oid(to),
        ],
        stop,
    )
}

/// A stash put back: its worktree change (`stash^1`→`stash`), its staged
/// change (`stash^1`→`stash^2`) and its untracked files (`stash^3`).
fn stash_paths(
    repo: &RepoHandle,
    stash: Oid,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let Some(&base) = parents.first() else {
        return Ok(BTreeSet::new());
    };
    let mut paths = diff_tree(repo, &[], [GitArg::Oid(base), GitArg::Oid(stash)], stop)?;
    if let Some(&staged) = parents.get(1) {
        paths.extend(diff_tree(
            repo,
            &[],
            [GitArg::Oid(base), GitArg::Oid(staged)],
            stop,
        )?);
    }
    if let Some(&untracked) = parents.get(2) {
        paths.extend(listed(repo, GitArg::Oid(untracked), stop)?);
    }
    Ok(paths)
}

/// Every path a tree holds: `ls-tree -r --name-only -z <tree>`.
fn listed(
    repo: &RepoHandle,
    tree: GitArg,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    names(
        repo,
        vec![
            GitArg::Flag("ls-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-z"),
            tree,
        ],
        stop,
    )
}

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
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(|line| {
            Oid::from_hex(line.trim())
                .map(Replayed::first_parent)
                .map_err(|_| {
                    FactsError::Snapshot(SnapshotError::NotAnOid {
                        what: "rev-list",
                        printed: line.to_owned(),
                    })
                })
        })
        .collect()
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
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(path_of)
        .collect())
}

/// A commit's parents, in order: `rev-parse <commit>^@`.
fn parents_of(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Vec<Oid>, FactsError> {
    let output = GitInvocation::new(repo)
        .args(vec![
            GitArg::Flag("rev-parse"),
            GitArg::Ref(format!("{commit}^@").into_bytes()),
        ])
        .run(stop)
        .map_err(run_error)?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(|line| {
            Oid::from_hex(line.trim()).map_err(|_| {
                FactsError::Snapshot(SnapshotError::NotAnOid {
                    what: "rev-parse",
                    printed: line.to_owned(),
                })
            })
        })
        .collect()
}

/// Run a `-z --name-only` read and collect its paths, never decoded.
fn names(
    repo: &RepoHandle,
    args: Vec<GitArg>,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(run_error)?;
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(path_of)
        .collect())
}

/// Raw path bytes as a `PathBuf`, never decoded.
fn path_of(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

/// The one `symlink_metadata` loop: which of `paths` his bytes stand in the
/// way of, as the path on disk that holds them.
///
/// - Something at the path — a file, a link, or a directory holding
///   anything the index doesn't track — counts as the path itself. A
///   directory of only tracked files is git's to empty (a tracked `dist/`
///   turning into a file `dist`), and an empty one holds no bytes.
/// - A file where the path needs a directory (`a` for `a/b`, `ENOTDIR`)
///   counts as that file, the first ancestor that isn't a directory —
///   unless the index tracks it: then git removes it as part of the change.
/// - Nothing there (`NotFound`) is nothing in the way.
///
/// # Errors
/// [`FactsError::OnDisk`] for any other failure to look.
pub(crate) fn on_disk(
    repo: &RepoHandle,
    paths: &BTreeSet<PathBuf>,
    tracked: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, FactsError> {
    let Some(root) = repo.workdir() else {
        return Ok(Vec::new());
    };
    let mut found = BTreeSet::new();
    for path in paths {
        match std::fs::symlink_metadata(root.join(path)) {
            Ok(meta) if meta.is_dir() => {
                if holds_untracked(root, path, tracked)? {
                    found.insert(path.clone());
                }
            }
            Ok(_) => {
                found.insert(path.clone());
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotADirectory => {
                if let Some(above) = file_above(root, path)?
                    && !tracked.contains(&above)
                {
                    found.insert(above);
                }
            }
            Err(source) => {
                return Err(FactsError::OnDisk {
                    path: path.clone(),
                    source,
                });
            }
        }
    }
    Ok(found.into_iter().collect())
}

/// The first ancestor of `path` that stands on disk and isn't a directory.
fn file_above(root: &Path, path: &Path) -> Result<Option<PathBuf>, FactsError> {
    let mut ancestors: Vec<&Path> = path
        .ancestors()
        .skip(1)
        .filter(|above| !above.as_os_str().is_empty())
        .collect();
    ancestors.reverse();
    for above in ancestors {
        match std::fs::symlink_metadata(root.join(above)) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Ok(Some(above.to_path_buf())),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(FactsError::OnDisk {
                    path: above.to_path_buf(),
                    source,
                });
            }
        }
    }
    Ok(None)
}

/// Whether the directory at `dir` holds anything the index doesn't track,
/// at any depth. Stops at the first such entry.
fn holds_untracked(
    root: &Path,
    dir: &Path,
    tracked: &BTreeSet<PathBuf>,
) -> Result<bool, FactsError> {
    let on_disk = |source| FactsError::OnDisk {
        path: dir.to_path_buf(),
        source,
    };
    for entry in std::fs::read_dir(root.join(dir)).map_err(on_disk)? {
        let entry = entry.map_err(on_disk)?;
        let path = dir.join(entry.file_name());
        let is_dir = entry.file_type().map_err(on_disk)?.is_dir();
        if is_dir {
            if holds_untracked(root, &path, tracked)? {
                return Ok(true);
            }
        } else if !tracked.contains(&path) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Ignored or untracked: `git status` lists an untracked file (or, for a
/// directory, something under it); an ignored one it never lists.
fn sort_out(found: Vec<PathBuf>, facts: &RepoFacts) -> Blocking {
    let listed: Vec<&Path> = facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.untracked)
        .map(|entry| entry.path.as_path())
        .collect();
    let mut blocking = Blocking::default();
    for path in found {
        if listed.iter().any(|untracked| untracked.starts_with(&path)) {
            blocking.untracked.push(path);
        } else {
            blocking.ignored.push(path);
        }
    }
    blocking
}

fn run_error(why: RunError) -> FactsError {
    match why {
        RunError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(SnapshotError::Run(other)),
    }
}

fn snapshot_error(why: SnapshotError) -> FactsError {
    match why {
        SnapshotError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(other),
    }
}
