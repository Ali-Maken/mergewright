//! The before-snapshot's `raw/` (ADR-0052): his bytes where git would not
//! give them back.
//!
//! `worktree/` is git's **cleaned** tree (`add -A`), and a restore writes it
//! through git's **smudge**. For most files the two are inverses. They are
//! not for an LF file under `core.autocrlf=true` (it comes back CRLF), a CRLF
//! file under `input` (it comes back LF), or a one-way clean filter such as
//! nbstripout (the outputs never come back). His bytes are his work, so those
//! paths are kept as they are on disk.
//!
//! Three questions, each asked only of the paths the one before let through,
//! so a clean worktree spawns no `git` and a repo that converts nothing one
//! (m7-power-tools/30r; every spawn is pushed to `commands`):
//!
//! 1. **Uncommitted?** `HEAD`'s tree against the worktree tree, read in
//!    process (`snapshot_reads::changed_files`, `diff-tree`'s answer): added,
//!    modified or type-changed regular files. A path that matches `HEAD` is
//!    in history and comes back as git's own checkout writes it.
//! 2. **Would git convert it?** `core.autocrlf`, re-read from disk in process
//!    (`snapshot_reads::autocrlf_converts`), and `check-attr` on the
//!    conversion attributes, which stays git's: its sources are git's to
//!    know. Saying yes wrongly costs one comparison; saying no wrongly loses
//!    his bytes, so every doubt is a yes.
//! 3. **Would git's checkout give it back?** The cleaned blobs checked out by
//!    git's own `checkout-index` into scratch, then compared with the disk.
//!    Not `cat-file --batch --filters`: its header prints the size *before*
//!    the filter (git 2.54), so its output cannot be split.
//!
//! What fails the third question is hashed with `--no-filters` and written as
//! a tree at its own path and mode.

use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::snapshot_reads::{self, ReadError};

use super::SnapshotError;
use super::TempIndex;
use super::plumbing::{c_quoted, oid_from, path_of, stopped};

mod converted;

use converted::converted;

#[derive(Clone, Debug, PartialEq, Eq)]
struct File {
    /// `100644` or `100755`, as the worktree tree records it.
    mode: Vec<u8>,
    /// The cleaned blob (`worktree/`'s).
    oid: Oid,
    /// Relative to the worktree root, raw bytes.
    path: Vec<u8>,
}

/// R2: the paths of `worktree_tree` that are uncommitted, that git would
/// convert, and whose bytes on disk git's smudge would not give back, as a
/// tree of their raw bytes (`hash-object -w --no-filters`). `None` when there
/// are none.
///
/// `head` is `HEAD`'s commit, `None` when unborn: then every path is
/// uncommitted. `worktree_tree` is the tree `worktree_tree` wrote a moment
/// ago, over the same disk.
///
/// # Errors
/// [`SnapshotError`] when a `git` step fails or prints what it should not, or
/// a file cannot be read; `Read` when an in-process read (a tree, the config)
/// fails; `Stopped` when `stop` fires.
pub fn raw_record(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
    head: Option<Oid>,
    worktree_tree: Oid,
    commands: &mut Vec<String>,
) -> Result<Option<Oid>, SnapshotError> {
    // A bare repository has no working tree, so no bytes of his to keep.
    let Some(root) = repo.workdir() else {
        return Ok(None);
    };
    let changed = uncommitted(repo, head, worktree_tree, stop)?;
    if changed.is_empty() {
        return Ok(None);
    }
    let autocrlf = snapshot_reads::autocrlf_converts(repo).map_err(read_failed)?;
    let converted = converted(repo, changed, autocrlf, stop, commands)?;
    if converted.is_empty() {
        return Ok(None);
    }
    let differ = not_given_back(repo, scratch, root, converted, stop, commands)?;
    if differ.is_empty() {
        return Ok(None);
    }
    raw_tree(repo, scratch, &differ, stop, commands).map(Some)
}

/// `invocation`, its command line pushed to `commands` first.
fn run_logged(
    invocation: &GitInvocation,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<mergewright_git::run::GitOutput, RunError> {
    commands.push(invocation.command_line());
    invocation.run(stop)
}

/// Question 1: the regular files `worktree_tree` holds apart from `HEAD`,
/// read in process: `diff-tree -r --no-renames`'s answer (unborn: every
/// file) kept to regular files on the new side. A deletion, a symlink and a
/// gitlink are left out: git filters neither a symlink nor a gitlink.
fn uncommitted(
    repo: &RepoHandle,
    head: Option<Oid>,
    worktree_tree: Oid,
    stop: &dyn Stop,
) -> Result<Vec<File>, SnapshotError> {
    let changed =
        snapshot_reads::changed_files(repo, head, worktree_tree, stop).map_err(read_failed)?;
    Ok(changed
        .into_iter()
        .map(|file| File {
            mode: if file.executable {
                b"100755".to_vec()
            } else {
                b"100644".to_vec()
            },
            oid: file.blob,
            path: file.path,
        })
        .collect())
}

/// A read's failure as the snapshot's, the **stop kept as a stop**, so a
/// caller can tell "you cancelled" from "the repository would not read".
fn read_failed(why: ReadError) -> SnapshotError {
    match why {
        ReadError::Stopped => SnapshotError::Stopped,
        other => SnapshotError::Read(other),
    }
}

/// Question 3: of `files`, the ones whose bytes on disk differ from what
/// git's own checkout of the cleaned blob writes.
///
/// The blobs go on a fresh temporary index and `checkout-index` writes them
/// under a scratch directory, through every smudge, `eol` and filter the
/// path's attributes name — the same machinery a restore uses. The directory
/// is removed on every path out.
fn not_given_back(
    repo: &RepoHandle,
    scratch: &Path,
    root: &Path,
    files: Vec<File>,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<Vec<File>, SnapshotError> {
    let index = TempIndex::fresh(scratch)?;
    fill_index(
        repo,
        &index,
        files.iter().map(|file| (file, file.oid)),
        stop,
        commands,
    )?;
    let out = ScratchDir::new(scratch)?;
    let mut prefix = out.path().as_os_str().as_bytes().to_vec();
    prefix.push(b'/');
    let checkout = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index.path())
        .arg(GitArg::Flag("checkout-index"))
        .arg(GitArg::Flag("-a"))
        .arg(GitArg::Flag("-f"))
        .arg(GitArg::Glued("--prefix=", prefix.clone()));
    if let Err(error) = run_logged(&checkout, stop, commands) {
        return Err(name_the_failing_file(
            repo, &index, &prefix, &files, error, stop, commands,
        ));
    }
    let mut differ = Vec::new();
    for file in files {
        let relative = path_of(file.path.clone());
        if !same_bytes(&root.join(&relative), &out.path().join(&relative))? {
            differ.push(file);
        }
    }
    Ok(differ)
}

/// `checkout-index -a` failed, and git's words — for a failing smudge filter,
/// say — may name the filter, not the file (m8-polish-and-ship/20 item 6). Only on
/// this failure path, each file is checked out alone until one fails: that
/// file is named. The first error stands when none fails alone (or on a
/// stop), so nothing is reported that was not seen.
fn name_the_failing_file(
    repo: &RepoHandle,
    index: &TempIndex,
    prefix: &[u8],
    files: &[File],
    first: RunError,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> SnapshotError {
    if matches!(first, RunError::Stopped) {
        return stopped(first);
    }
    for file in files {
        let one = GitInvocation::new(repo)
            .env("GIT_INDEX_FILE", index.path())
            .arg(GitArg::Flag("checkout-index"))
            .arg(GitArg::Flag("-f"))
            .arg(GitArg::Glued("--prefix=", prefix.to_vec()))
            .arg(GitArg::Path(file.path.clone()));
        match run_logged(&one, stop, commands) {
            Ok(_) => {}
            Err(RunError::Stopped) => return SnapshotError::Stopped,
            Err(error) => {
                return SnapshotError::CheckOut {
                    path: String::from_utf8_lossy(&file.path).into_owned(),
                    error,
                };
            }
        }
    }
    stopped(first)
}

/// The raw blobs (`hash-object -w --no-filters`, read from disk as they are)
/// on a fresh temporary index at each file's own path and mode, written as
/// a tree. The paths go on stdin, one C-quoted line each, as
/// `restore::raw::same_content_paths` sends them: one argument per path
/// would pass `ARG_MAX` on a large kept set and refuse every operation.
fn raw_tree(
    repo: &RepoHandle,
    scratch: &Path,
    files: &[File],
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<Oid, SnapshotError> {
    let mut lines = Vec::new();
    for file in files {
        lines.extend(c_quoted(&file.path));
        lines.push(b'\n');
    }
    let invocation = GitInvocation::new(repo)
        .arg(GitArg::Flag("hash-object"))
        .arg(GitArg::Flag("-w"))
        .arg(GitArg::Flag("--no-filters"))
        .arg(GitArg::Flag("--stdin-paths"))
        .stdin(lines);
    let output = run_logged(&invocation, stop, commands).map_err(stopped)?;
    let printed: Vec<&[u8]> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    if printed.len() != files.len() {
        return Err(unreadable("hash-object", &output.stdout));
    }
    let raw: Vec<Oid> = printed
        .into_iter()
        .map(|line| oid_from(line, "hash-object"))
        .collect::<Result<_, _>>()?;
    let index = TempIndex::fresh(scratch)?;
    fill_index(repo, &index, files.iter().zip(raw), stop, commands)?;
    // `plumbing::write_tree`'s invocation, logged.
    let write = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index.path())
        .arg(GitArg::Flag("write-tree"));
    let output = run_logged(&write, stop, commands).map_err(stopped)?;
    oid_from(&output.stdout, "write-tree")
}

/// `update-index -z --index-info` onto `index`: each file at its own path and
/// mode, with the blob paired with it.
fn fill_index<'a>(
    repo: &RepoHandle,
    index: &TempIndex,
    entries: impl IntoIterator<Item = (&'a File, Oid)>,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<(), SnapshotError> {
    let mut stdin = Vec::new();
    for (file, blob) in entries {
        stdin.extend_from_slice(&file.mode);
        stdin.push(b' ');
        stdin.extend_from_slice(blob.to_string().as_bytes());
        stdin.push(b'\t');
        stdin.extend_from_slice(&file.path);
        stdin.push(0);
    }
    let invocation = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index.path())
        .arg(GitArg::Flag("update-index"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Flag("--index-info"))
        .stdin(stdin);
    run_logged(&invocation, stop, commands).map_err(stopped)?;
    Ok(())
}

/// Whether two files hold the same bytes: lengths first, then 64 KiB at a
/// time, so a large file is never read whole.
fn same_bytes(one: &Path, other: &Path) -> Result<bool, SnapshotError> {
    if std::fs::metadata(one)?.len() != std::fs::metadata(other)?.len() {
        return Ok(false);
    }
    let mut one = std::fs::File::open(one)?;
    let mut other = std::fs::File::open(other)?;
    let mut left = vec![0_u8; 64 * 1024];
    let mut right = vec![0_u8; 64 * 1024];
    loop {
        let read = one.read(&mut left)?;
        if read == 0 {
            // Equal lengths and every chunk equal: the other is at its end too.
            return Ok(true);
        }
        other.read_exact(&mut right[..read])?;
        if left[..read] != right[..read] {
            return Ok(false);
        }
    }
}

fn unreadable(what: &'static str, printed: &[u8]) -> SnapshotError {
    SnapshotError::NotAnOid {
        what,
        printed: String::from_utf8_lossy(printed)
            .lines()
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

/// A directory under the scratch that `checkout-index` writes into, removed
/// with everything in it when it goes out of scope, on every path.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(scratch: &Path) -> Result<ScratchDir, SnapshotError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = scratch.join(format!("tmp-raw-{}-{unique}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        std::fs::create_dir_all(&path)?;
        Ok(ScratchDir { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // As `TempIndex`'s drop: nothing can be done about a failed removal
        // here, and it must not mask the snapshot's own result. A survivor
        // is inert (the next one takes a fresh name), and it is said out
        // loud rather than asserted.
        if let Err(why) = std::fs::remove_dir_all(&self.path)
            && why.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "{}: scratch {} survived: {why}",
                mergewright_core::on_disk::COMMAND,
                self.path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests;
