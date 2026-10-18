//! A temporary copy of the index, removed on every path: what snapshots and
//! a commit of chosen files run their index-writing steps on, so nothing
//! here writes the real `.git/index`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, Stop};

use super::SnapshotError;
use super::plumbing::stopped;

/// A copy of the real index, removed when it goes out of scope on **every**
/// path — a leaked temp index would be re-copied forever and is invisible to
/// the user.
pub(crate) struct TempIndex {
    path: PathBuf,
    /// [`TempIndex::remove`] already tried, and said how it went: `Drop`
    /// leaves the file alone.
    removed: bool,
}

impl TempIndex {
    /// Copy `<git-dir>/index` into `scratch` under a name no other snapshot
    /// in this process or any other can take.
    ///
    /// A missing index is not an error: a repository with no `.git/index`
    /// yet (a fresh `git init`) has an empty index, and `write-tree` over a
    /// missing `GIT_INDEX_FILE` prints the empty tree — verified on git 2.54.
    ///
    /// The copy keeps the index's **modification time**. git reads an entry
    /// as unchanged when its stat matches the file, unless the entry is as
    /// new as the index file — "racily clean" — when it reads the content
    /// instead. A copy dated now made every entry look older than its index,
    /// so an edit made in the same clock tick as git's last write, at the
    /// same size, was trusted away: the snapshot and the put-back forecast
    /// missed it. Linux's coarse file clock hit it 8 runs in 200; macOS's
    /// `fs::copy` happens to keep the time (m8-polish-and-ship/30).
    pub(super) fn copied_from(
        repo: &RepoHandle,
        scratch: &Path,
    ) -> Result<TempIndex, SnapshotError> {
        Self::copied_to(repo, scratch.join(Self::unique_name("tmp-index")))
    }

    /// The same copy at `path`, which the caller names for its run alone
    /// (`apply::run`'s scratch index, ease-pass/04r), and **always a file**:
    /// with no real index yet, an empty index is written there
    /// (`git read-tree --empty`), so a scratch index that later goes missing
    /// reads as gone, never as the empty index. [`TempIndex::remove`] says
    /// when it could not go.
    pub(crate) fn copied_as(
        repo: &RepoHandle,
        path: PathBuf,
        stop: &dyn Stop,
    ) -> Result<TempIndex, SnapshotError> {
        let copy = Self::copied_to(repo, path)?;
        if !copy.path.try_exists()? {
            GitInvocation::new(repo)
                .env("GIT_INDEX_FILE", &copy.path)
                .args([GitArg::Flag("read-tree"), GitArg::Flag("--empty")])
                .run(stop)
                .map_err(stopped)?;
        }
        Ok(copy)
    }

    fn copied_to(repo: &RepoHandle, path: PathBuf) -> Result<TempIndex, SnapshotError> {
        let mut source = match std::fs::File::open(repo.git_dir().join("index")) {
            Ok(source) => source,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                // No index file: the empty index. `write-tree` and `add -A`
                // both accept a `GIT_INDEX_FILE` that does not exist yet.
                if path.exists() {
                    std::fs::remove_file(&path)?;
                }
                return Ok(TempIndex::at(path));
            }
            Err(why) => return Err(why.into()),
        };
        // From the open file, so the time and the bytes are one index's even
        // if git replaces `.git/index` meanwhile (it renames a new one in).
        let modified = source.metadata()?.modified()?;
        let temp = TempIndex::at(path);
        let mut copy = std::fs::File::create(&temp.path)?;
        std::io::copy(&mut source, &mut copy)?;
        copy.set_modified(modified)?;
        Ok(temp)
    }

    /// A name no index file exists at yet: `update-index` creates it. The
    /// stage tree is built on one of these rather than on a copy, so nothing
    /// the real index holds can survive into it.
    pub(crate) fn fresh(scratch: &Path) -> Result<TempIndex, SnapshotError> {
        let path = scratch.join(Self::unique_name("tmp-stage-index"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(TempIndex::at(path))
    }

    fn at(path: PathBuf) -> TempIndex {
        TempIndex {
            path,
            removed: false,
        }
    }

    /// `<prefix>-<pid>-<n>`: a name no other run, in this process or any
    /// other, takes while this one is alive.
    pub(crate) fn unique_name(prefix: &str) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}-{}-{unique}", std::process::id())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Remove the copy now and say whether it went, where `Drop` can only
    /// print. Already gone is gone.
    pub(crate) fn remove(mut self) -> std::io::Result<()> {
        self.removed = true;
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            // Nothing to remove: what was asked for is true.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(why) => Err(why),
        }
    }
}

impl Drop for TempIndex {
    fn drop(&mut self) {
        if self.removed {
            return;
        }
        // Nothing can be done about a failed removal here and it must not
        // mask the operation's own result; the next snapshot uses a fresh
        // name, so a survivor is inert scratch, never read again.
        // Said out loud rather than asserted: a panic in `Drop` while a test
        // is already unwinding turns the real failure into an abort with no
        // message (m3/03 review, Minor 4).
        if let Err(why) = std::fs::remove_file(&self.path)
            && why.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "{}: temp index {} survived: {why}",
                mergewright_core::on_disk::Generation::CURRENT.command(),
                self.path.display()
            );
        }
    }
}
