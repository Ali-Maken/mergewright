//! The merge git has in progress, as files: reading `MERGE_HEAD`,
//! `MERGE_MSG` and `MERGE_MODE` into a [`MergeState`] for every journal
//! entry, and writing them back for `u` of a finished merge
//! (m6-conflicts/20).
//!
//! Reading happens on every entry (`apply::begin` and `apply::finish`), so it
//! is three file reads under [`RepoHandle::git_dir`] and no spawn — the same
//! place `mergewright_git::state::in_progress` looks for `MERGE_HEAD`, which is
//! what makes "merging" and "what the merge is" one answer. Writing happens
//! only in [`crate::undo::Step::SetMergeState`], so it asks git where the
//! files go (`rev-parse --git-path`) and asks git afterwards whether it sees
//! the merge (`rev-parse -q --verify MERGE_HEAD`) — the file `git status`
//! checks to say "you are still merging".
//!
//! This is the one place Mergewright writes git's merge files, and only to put
//! back bytes git itself wrote: the forward path never writes `MERGE_MSG`
//! (an edited message goes to `git commit` on stdin).

use std::io;
use std::path::{Path, PathBuf};

use mergewright_core::journal::MergeState;
use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};

/// The three files, in the order a restore writes them: the message and the
/// mode first, `MERGE_HEAD` last — its presence is what makes git call the
/// repository "merging", so a restore cut short before it leaves git's
/// state unchanged rather than half a merge.
const FILES: [&str; 3] = ["MERGE_MSG", "MERGE_MODE", "MERGE_HEAD"];

/// Why git's merge state could not be read or put back.
#[derive(Debug, thiserror::Error)]
pub enum MergeStateError {
    /// A merge file exists but could not be read, written or removed.
    #[error("{file}: {source}")]
    Io {
        /// Which file (`MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE`).
        file: &'static str,
        /// What the filesystem said.
        source: io::Error,
    },
    /// `MERGE_HEAD` holds a line that is not an object id, or no line at
    /// all. (Several ids is an octopus merge, and is read — m6-conflicts/38.)
    #[error("MERGE_HEAD holds {printed:?}, which is not an object id")]
    NotAnOid {
        /// The offending line (empty when there was none).
        printed: String,
    },
    /// A `git` read of the merge failed (or was stopped): where its files
    /// live, whether git sees it, or its short name.
    #[error("{0}")]
    Run(RunError),
    /// The files were written but git does not see the merge they describe.
    #[error("git reads MERGE_HEAD as {git_says:?} after it was set to {expected:?}")]
    NotWritten {
        /// What was written; `None` = the merge was removed.
        expected: Option<Oid>,
        /// What `rev-parse -q --verify MERGE_HEAD` says now.
        git_says: Option<Oid>,
    },
}

/// The merge git has in progress, or `None` when there is none.
///
/// `MERGE_HEAD` decides: no file, no merge — a `MERGE_MSG` on its own is a
/// cherry-pick's or a revert's, not a merge. A missing `MERGE_MSG` beside a
/// `MERGE_HEAD` (removed by hand) reads as an empty message.
///
/// An octopus merge (`git merge a b`: one `MERGE_HEAD` line per commit) is
/// read like any other, every head in order — this runs as every journaled
/// operation begins, so refusing a merge shape here would refuse every `u`
/// while that merge is open (m6-conflicts/38). Whether an octopus merge can
/// be *finished* is `resume`'s question, not this one's.
///
/// # Errors
/// [`MergeStateError::Io`] for a file that exists and will not read;
/// [`MergeStateError::NotAnOid`] for a `MERGE_HEAD` line that is not a
/// commit id, or no line at all.
pub(crate) fn read_merge_state(repo: &RepoHandle) -> Result<Option<MergeState>, MergeStateError> {
    let dir = repo.git_dir();
    let Some(head) = read_file(dir, "MERGE_HEAD")? else {
        return Ok(None);
    };
    let (merge_head, other_heads) = commits(&head)?;
    let merge_msg = read_file(dir, "MERGE_MSG")?.unwrap_or_default();
    let merge_mode = read_file(dir, "MERGE_MODE")?;
    Ok(Some(MergeState {
        merge_head,
        other_heads,
        merge_msg,
        merge_mode,
    }))
}

/// Whether a `git merge --autostash` left its stash waiting in
/// `MERGE_AUTOSTASH`, for the merge's finish to apply.
///
/// Verified against git (`finish_merge::git_applies_merge_autostash_when_the_merge_is_finished`,
/// `…_when_the_merge_is_aborted`): `git merge --continue`, `git commit` and
/// `git merge --abort` all apply it. A finish or abort Mergewright journals would
/// then write the stash's changes into the working tree and use the stash
/// up, and `u` would put the tree back without them — so `apply` refuses
/// either end (`Refusal::AutostashWaits`), and it is ended in the shell.
///
/// # Errors
/// [`MergeStateError::Io`] when the file's presence cannot be read.
pub(crate) fn autostash_waits(repo: &RepoHandle) -> Result<bool, MergeStateError> {
    match std::fs::symlink_metadata(repo.git_dir().join("MERGE_AUTOSTASH")) {
        Ok(_) => Ok(true),
        // Absent is the answer "no autostash", not a failure.
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(MergeStateError::Io {
            file: "MERGE_AUTOSTASH",
            source,
        }),
    }
}

/// Make git's merge state `state`: write the three files back (`Some`), or
/// remove them (`None`), then ask git whether it sees what was meant.
///
/// Each file goes through a temporary file and a rename, so a reader never
/// sees half a message. `MERGE_HEAD` is written last and removed first (see
/// [`FILES`]). The `git` lines run go into `commands`, for the Command Log.
///
/// Idempotent: writing the same state twice, or removing an absent one,
/// leaves the same files — which is what a roll-back after a stop inside
/// this step relies on.
///
/// # Errors
/// The first read, write or check that failed; [`MergeStateError::Run`]
/// with [`RunError::Stopped`] when `stop` asked.
pub(crate) fn write_merge_state(
    repo: &RepoHandle,
    state: Option<&MergeState>,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<(), MergeStateError> {
    let paths = git_paths(repo, stop, commands)?;
    match state {
        Some(state) => {
            let head = state.merge_head_bytes();
            let contents = [
                Some(state.merge_msg.as_slice()),
                state.merge_mode.as_deref(),
                Some(head.as_slice()),
            ];
            for ((&file, path), bytes) in FILES.iter().zip(&paths).zip(contents) {
                match bytes {
                    Some(bytes) => replace(path, bytes)
                        .map_err(|source| MergeStateError::Io { file, source })?,
                    None => remove(path).map_err(|source| MergeStateError::Io { file, source })?,
                }
            }
        }
        None => {
            for (&file, path) in FILES.iter().zip(&paths).rev() {
                remove(path).map_err(|source| MergeStateError::Io { file, source })?;
            }
        }
    }
    let expected = state.map(|state| state.merge_head);
    let git_says = merge_head_as_git_sees_it(repo, stop, commands)?;
    if git_says == expected {
        Ok(())
    } else {
        Err(MergeStateError::NotWritten { expected, git_says })
    }
}

/// A file's bytes, or `None` when it does not exist.
fn read_file(dir: &Path, file: &'static str) -> Result<Option<Vec<u8>>, MergeStateError> {
    match std::fs::read(dir.join(file)) {
        Ok(bytes) => Ok(Some(bytes)),
        // Absent is an answer — no merge, or a merge without that file —
        // not a failure.
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(MergeStateError::Io { file, source }),
    }
}

/// `MERGE_HEAD`'s bytes as the commits they name, in order: the first, and
/// the rest (an octopus merge's; empty for an ordinary merge).
fn commits(bytes: &[u8]) -> Result<(Oid, Vec<Oid>), MergeStateError> {
    let text = String::from_utf8_lossy(bytes);
    let mut heads = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            Oid::from_hex(line.trim()).map_err(|_why| MergeStateError::NotAnOid {
                printed: line.to_owned(),
            })
        });
    let Some(first) = heads.next() else {
        return Err(MergeStateError::NotAnOid {
            printed: String::new(),
        });
    };
    Ok((first?, heads.collect::<Result<_, _>>()?))
}

/// Where git keeps the three files, in [`FILES`] order: one `rev-parse` with
/// three `--git-path`s. A relative answer is relative to the directory git
/// ran in, as [`RepoHandle::git_path`] explains.
fn git_paths(
    repo: &RepoHandle,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<Vec<PathBuf>, MergeStateError> {
    let mut args = vec![GitArg::Flag("rev-parse")];
    for file in FILES {
        args.push(GitArg::Word("--git-path".to_owned()));
        args.push(GitArg::Word(file.to_owned()));
    }
    let invocation = GitInvocation::new(repo).args(args);
    commands.push(invocation.command_line());
    let output = invocation.run(stop).map_err(MergeStateError::Run)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let base = repo.workdir().unwrap_or_else(|| repo.git_dir());
    let paths: Vec<PathBuf> = text
        .lines()
        .map(|line| {
            let printed = Path::new(line.trim());
            if printed.is_absolute() {
                printed.to_path_buf()
            } else {
                base.join(printed)
            }
        })
        .collect();
    if paths.len() == FILES.len() {
        Ok(paths)
    } else {
        Err(MergeStateError::Run(RunError::Unparsable(format!(
            "rev-parse --git-path printed {} lines for {} files",
            paths.len(),
            FILES.len()
        ))))
    }
}

/// What `git rev-parse -q --verify MERGE_HEAD` says: the commit, or `None`
/// when git sees no merge (exit 1, nothing printed).
fn merge_head_as_git_sees_it(
    repo: &RepoHandle,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<Option<Oid>, MergeStateError> {
    let invocation = GitInvocation::new(repo).args([
        GitArg::Flag("rev-parse"),
        GitArg::Flag("-q"),
        GitArg::Flag("--verify"),
        GitArg::Word("MERGE_HEAD".to_owned()),
    ]);
    commands.push(invocation.command_line());
    match invocation.run(stop) {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            Oid::from_hex(text.trim())
                .map(Some)
                .map_err(|_why| MergeStateError::NotAnOid {
                    printed: text.trim().to_owned(),
                })
        }
        // `-q --verify` exits 1 and prints nothing for a name that does not
        // resolve: that is the answer "not merging", not a failure.
        Err(RunError::Exited { status: 1, .. }) => Ok(None),
        Err(other) => Err(MergeStateError::Run(other)),
    }
}

/// Write `bytes` to `path` through a temporary file beside it and a rename.
fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".mwr-tmp");
    let temporary = PathBuf::from(temporary);
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

/// Remove `path`; already absent is done.
fn remove(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        // The state asked for is "no such file", and there is none.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_head_is_its_commits_in_order_or_refused_in_words() {
        let id = "acf15d58a3b48e5878fe281a624fb5c688dcaee1";
        let other = "0123456789abcdef0123456789abcdef01234567";
        let oid = |hex: &str| Oid::from_hex(hex).expect("hex");
        assert_eq!(
            commits(format!("{id}\n").as_bytes()).expect("one line"),
            (oid(id), Vec::new())
        );
        assert_eq!(
            commits(format!("{id}\n{other}\n").as_bytes()).expect("an octopus"),
            (oid(id), vec![oid(other)])
        );
        assert!(matches!(
            commits(format!("{id}\nnot an id\n").as_bytes()),
            Err(MergeStateError::NotAnOid { printed }) if printed == "not an id"
        ));
        assert!(matches!(
            commits(b"not an id\n"),
            Err(MergeStateError::NotAnOid { .. })
        ));
        assert!(matches!(
            commits(b""),
            Err(MergeStateError::NotAnOid { .. })
        ));
    }
}
