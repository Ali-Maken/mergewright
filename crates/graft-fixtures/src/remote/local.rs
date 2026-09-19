//! A bare repository and a working clone of it, on the `file` transport.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::run_git;
use crate::{CorpusError, Scratch, Shape, try_borrow};

/// A remote on disk and a clone that tracks it.
///
/// `git clone --bare` of a corpus shape is the remote; `git clone` of that is
/// the working repository, so its `origin` and its default branch's upstream
/// are the ones `git clone` sets — real, not fabricated. A third clone,
/// private to this value, stands in for a teammate: [`commit_on_remote`]
/// commits there and pushes, so the working clone falls behind without being
/// touched.
///
/// Everything lives in one temporary directory, deleted on drop.
///
/// [`commit_on_remote`]: LocalRemote::commit_on_remote
#[derive(Debug)]
pub struct LocalRemote {
    /// Owns the directory everything below lives in; deleting it on drop is
    /// its whole job, so nothing reads it.
    _scratch: Scratch,
    bare: PathBuf,
    work: PathBuf,
    /// The teammate's clone [`LocalRemote::commit_on_remote`] pushes from.
    teammate: PathBuf,
    /// `HOME` for every `git` run here: an empty directory, so no user
    /// config, credential or hook can leak in.
    home: PathBuf,
    default_branch: String,
    /// Numbers the files the commits add, so each commit has new content.
    commits: AtomicUsize,
}

impl LocalRemote {
    /// Clones `shape` into a bare remote and clones that into a working
    /// repository.
    ///
    /// # Errors
    /// [`CorpusError`] if the shape cannot be had on this machine
    /// ([`CorpusError::Unavailable`]), the scratch directory cannot be made,
    /// or a `git clone` fails — including a shape whose `HEAD` is detached,
    /// which has no default branch to track.
    pub fn new(shape: Shape) -> Result<LocalRemote, CorpusError> {
        let source = try_borrow(shape)?;
        let scratch = Scratch::empty("remote").map_err(|source| CorpusError::Io {
            context: "creating the remote's scratch directory".to_owned(),
            source,
        })?;
        let root = scratch.path().to_path_buf();
        let home = root.join("home");
        fs::create_dir(&home).map_err(|source| CorpusError::Io {
            context: format!("creating {}", home.display()),
            source,
        })?;
        let bare = root.join("remote.git");
        let work = root.join("work");
        let teammate = root.join("teammate");

        let source_arg = source.to_string_lossy();
        let bare_arg = bare.to_string_lossy();
        // `--no-hardlinks`: the corpus is shared and read-only; the remote
        // must not share inodes with it.
        run_git(
            &home,
            &root,
            &[
                "clone",
                "-q",
                "--bare",
                "--no-hardlinks",
                &source_arg,
                &bare_arg,
            ],
            &[],
        )?;
        let head = run_git(&home, &bare, &["symbolic-ref", "--short", "HEAD"], &[])?;
        let default_branch = String::from_utf8_lossy(&head).trim().to_owned();
        for clone in [&work, &teammate] {
            run_git(
                &home,
                &root,
                &["clone", "-q", &bare_arg, &clone.to_string_lossy()],
                &[],
            )?;
        }

        Ok(LocalRemote {
            _scratch: scratch,
            bare,
            work,
            teammate,
            home,
            default_branch,
            commits: AtomicUsize::new(0),
        })
    }

    /// The bare repository — the remote.
    pub fn bare(&self) -> &Path {
        &self.bare
    }

    /// The working clone, whose `origin` is [`bare`](LocalRemote::bare).
    pub fn work(&self) -> &Path {
        &self.work
    }

    /// The remote as a `file://` URL.
    pub fn url(&self) -> String {
        format!("file://{}", self.bare.display())
    }

    /// The remote's default branch, short name (`main`): what its `HEAD`
    /// pointed at when it was cloned.
    pub fn default_branch(&self) -> &str {
        &self.default_branch
    }

    /// Commits a new file on the remote's default branch from another clone
    /// and pushes it, so [`work`](LocalRemote::work) is one commit behind
    /// once it fetches. Returns the new tip, 40 hex.
    ///
    /// # Errors
    /// The `git` failure, its stderr in the message.
    pub fn commit_on_remote(&self, subject: &str) -> io::Result<String> {
        let branch = &self.default_branch;
        // Catch up first: the working clone may have pushed since the last
        // call, and a stale teammate would be rejected.
        self.git(&self.teammate, &["fetch", "-q", "origin"])?;
        self.git(
            &self.teammate,
            &["reset", "-q", "--hard", &format!("origin/{branch}")],
        )?;
        let tip = self.commit_in(&self.teammate, "remote", subject)?;
        self.git(
            &self.teammate,
            &["push", "-q", "origin", &format!("HEAD:refs/heads/{branch}")],
        )?;
        Ok(tip)
    }

    /// Commits a new file in [`work`](LocalRemote::work) without pushing, so
    /// it is one commit ahead. Returns the new tip, 40 hex.
    ///
    /// # Errors
    /// The `git` failure, its stderr in the message.
    pub fn commit_locally(&self, subject: &str) -> io::Result<String> {
        self.commit_in(&self.work, "local", subject)
    }

    /// Runs hermetic `git <args>` in `dir` — the same isolation every command
    /// here gets — and returns its stdout, trimmed.
    ///
    /// # Errors
    /// If `git` cannot start, outlives the deadline or exits non-zero; the
    /// message carries its stderr.
    pub fn git(&self, dir: &Path, args: &[&str]) -> io::Result<String> {
        let out = run_git(&self.home, dir, args, &[]).map_err(io::Error::other)?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    /// The empty `HOME` every `git` here runs with.
    #[cfg(test)]
    pub(crate) fn home(&self) -> &Path {
        &self.home
    }

    fn commit_in(&self, dir: &Path, prefix: &str, subject: &str) -> io::Result<String> {
        let n = self.commits.fetch_add(1, Ordering::Relaxed);
        let name = format!("{prefix}-{n}.txt");
        fs::write(dir.join(&name), format!("{subject}\n"))?;
        self.git(dir, &["add", "--", &name])?;
        self.git(dir, &["commit", "-q", "-m", subject])?;
        self.git(dir, &["rev-parse", "HEAD"])
    }
}
