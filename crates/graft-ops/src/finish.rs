//! What a merge will commit, read before the user presses `⏎` on the finish
//! summary (m6-conflicts/09, Amendment 2 "Finish summary"): the files he
//! resolved and which side each came out as, the files git combined on its
//! own (which he has never seen otherwise), and git's prepared message.
//!
//! Reads only. The commit itself is `apply::resume` with
//! [`Resume::ContinueWith`](crate::apply::Resume::ContinueWith).

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::diff::{DiffError, DiffOptions, DiffSpec, FileStatus, list_files};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError};
use graft_git::state::{self, InProgress};

use crate::conflict::{ConflictSetError, conflicts_of, merge_msg};
use crate::facts::{self, FactsError};

/// One screen of what a merge will commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinishSummary {
    /// The branch merged into (short name), or `HEAD`'s short id when
    /// detached.
    pub into: Vec<u8>,
    /// What was merged in: the name `MERGE_MSG`'s first line gives
    /// (`feature`, or `main of <url>` for a pull), else `MERGE_HEAD`'s short
    /// id.
    pub from: Vec<u8>,
    /// The files the merge conflicted on, in the conflict set's order.
    pub by_you: Vec<ByYou>,
    /// Every other staged change, in git's diff order.
    pub by_git: Vec<ByGit>,
    /// `MERGE_MSG` with its comment lines removed.
    pub message: Vec<u8>,
    /// Whether it is known which files were conflicts. When false,
    /// `by_you` is empty and every staged change is in `by_git`.
    pub recorded: bool,
}

/// A file the user resolved, and what it came out as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByYou {
    /// The path.
    pub path: PathBuf,
    /// The staged result against both sides.
    pub result: ResultIs,
}

/// The staged blob compared with `HEAD:path` and `MERGE_HEAD:path`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultIs {
    /// Byte for byte `HEAD`'s (checked first, so a path both sides agree
    /// on reads as yours).
    Yours,
    /// Byte for byte `MERGE_HEAD`'s.
    Theirs,
    /// Not in the index: the merge deletes it.
    Deleted,
    /// Neither side's: a combination, or an edit.
    Combined,
}

/// A change git made on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByGit {
    /// The path on the new side.
    pub path: PathBuf,
    /// What kind of change.
    pub change: Change,
    /// Lines added; 0 for a binary file.
    pub added: usize,
    /// Lines removed; 0 for a binary file.
    pub removed: usize,
}

/// The kind of a staged change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// A new file. A copy git detected is one too: the copy is new.
    Added,
    /// Content or mode changed.
    Modified,
    /// The file goes.
    Deleted,
    /// Moved from another path.
    Renamed {
        /// The old path.
        from: PathBuf,
    },
    /// A file became a symlink, or the other way round.
    TypeChanged,
}

/// Why there is no summary.
#[derive(Debug, thiserror::Error)]
pub enum SummaryError {
    /// git has no merge in progress: a summary is merge only this milestone.
    #[error("no merge is in progress")]
    NotMerging,
    /// Something is still unmerged: git cannot commit yet.
    #[error("{count} still conflicted")]
    StillUnmerged {
        /// How many paths.
        count: usize,
    },
    /// The repository's state could not be read.
    #[error("{}", .0.explain())]
    Facts(FactsError),
    /// A `git` read would not run.
    #[error("{0}")]
    Run(#[from] RunError),
    /// `MERGE_MSG` exists but could not be read.
    #[error("could not read git's merge message: {0}")]
    Io(#[from] io::Error),
    /// The conflict set could not be read.
    #[error("{0}")]
    Conflicts(#[from] ConflictSetError),
    /// The staged diff could not be read.
    #[error("{0}")]
    Diff(#[from] DiffError),
}

impl From<FactsError> for SummaryError {
    fn from(error: FactsError) -> Self {
        SummaryError::Facts(error)
    }
}

/// The summary of the merge in progress.
///
/// # Errors
/// [`SummaryError::NotMerging`] unless git has a merge in progress;
/// [`SummaryError::StillUnmerged`] while any path is unmerged; the read's
/// own failure otherwise.
pub fn finish_summary(repo: &RepoHandle) -> Result<FinishSummary, SummaryError> {
    // No caller-supplied stop: every read here is one short `git` call.
    let stop = &();
    let in_progress = state::in_progress(repo).map_err(FactsError::State)?;
    if in_progress != Some(InProgress::Merge) {
        return Err(SummaryError::NotMerging);
    }
    let status = graft_git::status::read(repo, stop).map_err(FactsError::from)?;
    let count = status.entries.iter().filter(|entry| entry.unmerged).count();
    if count > 0 {
        return Err(SummaryError::StillUnmerged { count });
    }
    let set = conflicts_of(repo, in_progress, &status)?;
    let prepared = merge_msg::read(repo)?;
    let into = match facts::head_ref(repo)? {
        HeadRef::Branch(name) | HeadRef::Unborn(name) => short_branch(&name),
        HeadRef::Detached(oid) => short_id(repo, &oid.to_string())?,
    };
    let from = match prepared.as_deref().and_then(merge_msg::merged_from) {
        Some(name) => name,
        None => short_id(repo, "MERGE_HEAD")?,
    };
    let message = match &prepared {
        Some(bytes) => merge_msg::without_comments(bytes, &merge_msg::comment_prefix(repo, stop)?),
        // git writes MERGE_MSG for every stopped merge; one removed by hand
        // leaves no message to show, which the summary shows as empty.
        None => Vec::new(),
    };
    Ok(FinishSummary {
        into,
        from,
        by_you: results(repo, &set.paths)?,
        by_git: changed_by_git(repo, &set.paths)?,
        message,
        recorded: set.recorded,
    })
}

/// Every staged change but the conflicted paths, in git's diff order.
fn changed_by_git(repo: &RepoHandle, conflicted: &[PathBuf]) -> Result<Vec<ByGit>, SummaryError> {
    let yours: HashSet<&Path> = conflicted.iter().map(PathBuf::as_path).collect();
    let staged = list_files(repo, &DiffSpec::Staged, &DiffOptions::default(), &())?;
    let mut by_git = Vec::new();
    for file in staged.files {
        let path = path_of(file.path);
        if yours.contains(path.as_path()) {
            continue;
        }
        let change = match file.status {
            FileStatus::Added | FileStatus::Copied { .. } => Change::Added,
            FileStatus::Deleted => Change::Deleted,
            FileStatus::Renamed { .. } => Change::Renamed {
                from: path_of(file.old_path.ok_or_else(|| {
                    RunError::Unparsable(format!("a rename with no old path: {}", path.display()))
                })?),
            },
            FileStatus::TypeChanged => Change::TypeChanged,
            // `Staged` is `diff-index --cached`: it reports neither an
            // untracked file nor, with nothing unmerged, an unmerged one.
            FileStatus::Modified | FileStatus::Unmerged | FileStatus::Untracked => Change::Modified,
        };
        by_git.push(ByGit {
            path,
            change,
            // `None` is a binary file: no line counts to show.
            added: file.added.map_or(0, |n| n as usize),
            removed: file.removed.map_or(0, |n| n as usize),
        });
    }
    Ok(by_git)
}

/// Each conflicted path's staged blob against `HEAD`'s and `MERGE_HEAD`'s.
fn results(repo: &RepoHandle, paths: &[PathBuf]) -> Result<Vec<ByYou>, SummaryError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let staged = staged_blobs(repo, paths)?;
    let ours = tree_blobs(repo, "HEAD", paths)?;
    let theirs = tree_blobs(repo, "MERGE_HEAD", paths)?;
    Ok(paths
        .iter()
        .map(|path| ByYou {
            path: path.clone(),
            result: match staged.get(path) {
                None => ResultIs::Deleted,
                Some(blob) if ours.get(path) == Some(blob) => ResultIs::Yours,
                Some(blob) if theirs.get(path) == Some(blob) => ResultIs::Theirs,
                Some(_) => ResultIs::Combined,
            },
        })
        .collect())
}

/// `ls-files -s`: each path's stage-0 blob in the index.
fn staged_blobs(repo: &RepoHandle, paths: &[PathBuf]) -> Result<HashMap<PathBuf, Oid>, RunError> {
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-files"),
        GitArg::Flag("-s"),
        GitArg::Flag("-z"),
        GitArg::Separator,
    ];
    args.extend(paths.iter().map(|path| GitArg::Path(bytes_of(path))));
    let output = GitInvocation::new(repo).args(args).run(&())?;
    // `<mode> <oid> <stage>\t<path>`
    records(&output.stdout, |fields| match fields {
        [_, _, b"0"] => Some(1),
        _ => None,
    })
}

/// `ls-tree -r`: each path's blob in a commit's tree.
fn tree_blobs(
    repo: &RepoHandle,
    commit: &'static str,
    paths: &[PathBuf],
) -> Result<HashMap<PathBuf, Oid>, RunError> {
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Flag(commit),
        GitArg::Separator,
    ];
    args.extend(paths.iter().map(|path| GitArg::Path(bytes_of(path))));
    let output = GitInvocation::new(repo).args(args).run(&())?;
    // `<mode> <type> <oid>\t<path>`
    records(&output.stdout, |fields| match fields {
        [_, _, _] => Some(2),
        _ => None,
    })
}

/// NUL-terminated `<fields>\t<path>` records; `pick` says which field is the object id
/// among the space-separated fields, `None` to skip the record.
fn records(
    stdout: &[u8],
    pick: impl Fn(&[&[u8]]) -> Option<usize>,
) -> Result<HashMap<PathBuf, Oid>, RunError> {
    let mut found = HashMap::new();
    for record in stdout.split(|byte| *byte == 0).filter(|r| !r.is_empty()) {
        let unparsable = || RunError::Unparsable(String::from_utf8_lossy(record).into_owned());
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(unparsable)?;
        let fields: Vec<&[u8]> = record[..tab].split(|byte| *byte == b' ').collect();
        let Some(oid) = pick(&fields).and_then(|at| fields.get(at)) else {
            continue;
        };
        let oid = std::str::from_utf8(oid)
            .ok()
            .and_then(|hex| Oid::from_hex(hex).ok())
            .ok_or_else(unparsable)?;
        found.insert(path_of(record[tab + 1..].to_vec()), oid);
    }
    Ok(found)
}

/// `git rev-parse --short <rev>`.
fn short_id(repo: &RepoHandle, rev: &str) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-parse"),
            GitArg::Flag("--short"),
            GitArg::Word(rev.to_owned()),
        ])
        .run(&())?;
    Ok(output.stdout.trim_ascii_end().to_vec())
}

fn short_branch(full: &str) -> Vec<u8> {
    full.strip_prefix("refs/heads/")
        .unwrap_or(full)
        .as_bytes()
        .to_vec()
}

fn bytes_of(path: &Path) -> Vec<u8> {
    path.as_os_str().as_encoded_bytes().to_vec()
}

fn path_of(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(bytes))
}
