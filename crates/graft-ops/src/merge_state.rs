//! The merge git has in progress, as files: reading `MERGE_HEAD`,
//! `MERGE_MSG` and `MERGE_MODE` into a [`MergeState`] for every journal
//! entry, and writing them back for `u` of a finished merge
//! (m6-conflicts/20).
//!
//! Reading happens on every entry (`apply::begin` and `apply::finish`), so it
//! is three file reads under [`RepoHandle::git_dir`] and no spawn — the same
//! place `graft_git::state::in_progress` looks for `MERGE_HEAD`, which is
//! what makes "merging" and "what the merge is" one answer. Writing happens
//! only in [`crate::undo::Step::SetMergeState`], so it asks git where the
//! files go (`rev-parse --git-path`) and asks git afterwards whether it sees
//! the merge (`rev-parse -q --verify MERGE_HEAD`) — the file `git status`
//! checks to say "you are still merging".
//!
//! This is the one place Graft writes git's merge files, and only to put
//! back bytes git itself wrote: the forward path never writes `MERGE_MSG`
//! (an edited message goes to `git commit` on stdin).

use std::io;
use std::path::{Path, PathBuf};

use graft_core::journal::MergeState;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

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
    /// `MERGE_HEAD` names more than one commit: an octopus merge, which
    /// Graft does not record (its `MergeState` holds one commit).
    #[error("an octopus merge is in progress ({heads} heads); Graft records one MERGE_HEAD")]
    Octopus {
        /// How many commits it names.
        heads: usize,
    },
    /// `MERGE_HEAD` holds something that is not an object id.
    #[error("MERGE_HEAD holds {printed:?}, which is not an object id")]
    NotAnOid {
        /// What it holds, first line.
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
/// # Errors
/// [`MergeStateError::Io`] for a file that exists and will not read;
/// [`MergeStateError::Octopus`] / [`MergeStateError::NotAnOid`] for a
/// `MERGE_HEAD` that is not one commit.
pub(crate) fn read_merge_state(repo: &RepoHandle) -> Result<Option<MergeState>, MergeStateError> {
    let dir = repo.git_dir();
    let Some(head) = read_file(dir, "MERGE_HEAD")? else {
        return Ok(None);
    };
    let merge_head = one_commit(&head)?;
    let merge_msg = read_file(dir, "MERGE_MSG")?.unwrap_or_default();
    let merge_mode = read_file(dir, "MERGE_MODE")?;
    Ok(Some(MergeState {
        merge_head,
        merge_msg,
        merge_mode,
    }))
}

/// Whether a `git merge --autostash` left its stash waiting in
/// `MERGE_AUTOSTASH`, for the merge's finish to apply.
///
/// Verified against git (`finish_merge::git_applies_merge_autostash_when_the_merge_is_finished`):
/// both `git merge --continue` and `git commit` apply it. A finish Graft
/// journals would then write the stash's changes into the working tree and
/// use the stash up, and `u` would put the tree back without them — so
/// `apply` refuses such a finish, and it is finished in the shell.
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
            let head = format!("{}\n", state.merge_head).into_bytes();
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

/// `MERGE_HEAD`'s bytes as the one commit they must name.
fn one_commit(bytes: &[u8]) -> Result<Oid, MergeStateError> {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    match lines.as_slice() {
        [one] => Oid::from_hex(one.trim()).map_err(|_why| MergeStateError::NotAnOid {
            printed: (*one).to_owned(),
        }),
        [] => Err(MergeStateError::NotAnOid {
            printed: String::new(),
        }),
        many => Err(MergeStateError::Octopus { heads: many.len() }),
    }
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
    temporary.push(".graft-tmp");
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
    fn merge_head_is_one_commit_or_refused_in_words() {
        let id = "acf15d58a3b48e5878fe281a624fb5c688dcaee1";
        assert_eq!(
            one_commit(format!("{id}\n").as_bytes()).expect("one line"),
            Oid::from_hex(id).expect("hex")
        );
        assert!(matches!(
            one_commit(format!("{id}\n{id}\n").as_bytes()),
            Err(MergeStateError::Octopus { heads: 2 })
        ));
        assert!(matches!(
            one_commit(b"not an id\n"),
            Err(MergeStateError::NotAnOid { .. })
        ));
        assert!(matches!(
            one_commit(b""),
            Err(MergeStateError::NotAnOid { .. })
        ));
    }
}
