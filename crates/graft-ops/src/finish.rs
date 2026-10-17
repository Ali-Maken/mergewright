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
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::state::{self, InProgress};

use crate::conflict::{ConflictSetError, conflicts_of, merge_msg};
use crate::facts::{self, FactsError};
use crate::snapshot::{SnapshotError, TempIndex};

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
    /// The review's copy of the index could not be made or removed.
    #[error("the review's copy of the index: {0}")]
    Scratch(#[from] SnapshotError),
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
    summary_of(repo, &[], &())
}

/// The review's facts as if `solved` were staged: a scratch index, never the
/// real one (ease-pass/05b, hard choice 3).
///
/// The real index is copied to a temporary file beside the journal,
/// `git add -- <solved>` runs on the copy (`GIT_INDEX_FILE`, the snapshot
/// plumbing's route), and each resolved file's result is read from it. The
/// copy is removed on every path out. With nothing in `solved` this is
/// [`finish_summary`], with no copy made.
///
/// `by_git` is read from the real index: it leaves out every path the merge
/// conflicted on, and `solved` are exactly such paths, so staging them on
/// the copy changes nothing it lists.
///
/// # Errors
/// [`SummaryError::NotMerging`] unless git has a merge in progress;
/// [`SummaryError::StillUnmerged`] counting the unmerged paths not in
/// `solved`; [`SummaryError::Scratch`] when the copy could not be made or
/// removed; the read's own failure otherwise.
pub fn finish_preview(
    repo: &RepoHandle,
    solved: &[PathBuf],
    stop: &dyn Stop,
) -> Result<FinishSummary, SummaryError> {
    summary_of(repo, solved, stop)
}

/// [`finish_summary`] and [`finish_preview`]: one body.
fn summary_of(
    repo: &RepoHandle,
    solved: &[PathBuf],
    stop: &dyn Stop,
) -> Result<FinishSummary, SummaryError> {
    let in_progress = state::in_progress(repo).map_err(FactsError::State)?;
    if in_progress != Some(InProgress::Merge) {
        return Err(SummaryError::NotMerging);
    }
    let status = graft_git::status::read(repo, stop).map_err(FactsError::from)?;
    let unmerged: Vec<PathBuf> = status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path.clone())
        .collect();
    let count = unmerged
        .iter()
        .filter(|path| !solved.contains(path))
        .count();
    if count > 0 {
        return Err(SummaryError::StillUnmerged { count });
    }
    // Only the solved paths git still calls unmerged need the copy: one
    // staged already reads the same from the real index.
    let to_stage: Vec<PathBuf> = unmerged;
    let copy = if to_stage.is_empty() {
        None
    } else {
        Some(review_index(repo, &to_stage, stop)?)
    };
    let index = copy.as_ref().map(TempIndex::path);
    let summary = summary_on(repo, in_progress, &status, index, stop);
    match copy {
        // Removed before the summary is returned, so a copy that would not
        // go is said rather than left behind in silence.
        Some(copy) => {
            let removed = copy.remove();
            let summary = summary?;
            removed.map_err(|why| SummaryError::Scratch(SnapshotError::Io(why)))?;
            Ok(summary)
        }
        None => summary,
    }
}

/// A copy of the real index with `paths` staged on it: `GIT_INDEX_FILE`
/// set, nothing written to `.git/index`. Beside the journal, under a name
/// no other run takes; [`TempIndex`] removes it on every path.
fn review_index(
    repo: &RepoHandle,
    paths: &[PathBuf],
    stop: &dyn Stop,
) -> Result<TempIndex, SummaryError> {
    let journal = crate::journal::journal_path(repo)?;
    let scratch = journal.parent().ok_or_else(|| {
        SummaryError::Scratch(SnapshotError::Io(io::Error::other(
            "the journal has no directory to copy the index into",
        )))
    })?;
    std::fs::create_dir_all(scratch)?;
    let copy = TempIndex::copied_as(
        repo,
        scratch.join(TempIndex::unique_name("tmp-review-index")),
        stop,
    )?;
    let Some(add) = crate::operation::stage_solved_args(paths) else {
        return Ok(copy);
    };
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", copy.path())
        .args(add)
        .run(stop)?;
    Ok(copy)
}

/// The summary's reads, with each resolved file's result read from `index`
/// when given (the review's copy), else from the real index.
fn summary_on(
    repo: &RepoHandle,
    in_progress: Option<InProgress>,
    status: &graft_git::status::WorkingTreeStatus,
    index: Option<&Path>,
    stop: &dyn Stop,
) -> Result<FinishSummary, SummaryError> {
    let set = conflicts_of(repo, in_progress, status)?;
    let prepared = merge_msg::read(repo)?;
    let into = match facts::head_ref(repo)? {
        HeadRef::Branch(name) | HeadRef::Unborn(name) => short_branch(&name),
        HeadRef::Detached(oid) => short_id(repo, &oid.to_string())?,
    };
    let from = merged_from(repo, prepared.as_deref())?;
    let message = match &prepared {
        Some(bytes) => merge_msg::without_comments(bytes, &merge_msg::comment_prefix(repo, stop)?),
        // git writes MERGE_MSG for every stopped merge; one removed by hand
        // leaves no message to show, which the summary shows as empty.
        None => Vec::new(),
    };
    Ok(FinishSummary {
        into,
        from,
        by_you: results(repo, &set.paths, index)?,
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
fn results(
    repo: &RepoHandle,
    paths: &[PathBuf],
    index: Option<&Path>,
) -> Result<Vec<ByYou>, SummaryError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let staged = staged_blobs(repo, paths, index)?;
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

/// `ls-files -s`: each path's stage-0 blob in the index — `index`, the
/// review's copy, when given.
fn staged_blobs(
    repo: &RepoHandle,
    paths: &[PathBuf],
    index: Option<&Path>,
) -> Result<HashMap<PathBuf, Oid>, RunError> {
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-files"),
        GitArg::Flag("-s"),
        GitArg::Flag("-z"),
        GitArg::Separator,
    ];
    args.extend(paths.iter().map(|path| GitArg::Path(bytes_of(path))));
    let mut invocation = GitInvocation::new(repo).args(args);
    if let Some(index) = index {
        invocation = invocation.env("GIT_INDEX_FILE", index);
    }
    let output = invocation.run(&())?;
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

/// What a merge in progress is merging in, in [`FinishSummary::from`]'s
/// words: the name `MERGE_MSG`'s first line gives, else `MERGE_HEAD`'s short
/// id. One rule for the summary's title and for the journal's `finish merge
/// of <from>` (m6-conflicts/20), so the two never name it differently.
pub(crate) fn merged_from(
    repo: &RepoHandle,
    merge_msg: Option<&[u8]>,
) -> Result<Vec<u8>, RunError> {
    match merge_msg.and_then(merge_msg::merged_from) {
        Some(name) => Ok(name),
        None => short_id(repo, "MERGE_HEAD"),
    }
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
