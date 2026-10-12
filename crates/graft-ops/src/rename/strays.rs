//! The stray pass: what is under the old names after the switch, carried
//! to the new or set aside, never deleted unread (review m8-26 M2).
//!
//! It is step 8 of the move, all of a `switched` clean-up, and what runs
//! when a repository already `clean` has old names on disk again — an older
//! build, still open on it, wrote them. Under the lock held alone.
//!
//! Old copies are deleted once they read back the same as their new ones.
//! What an older build wrote **after** its copy was taken never had a copy:
//! it goes to `older-version/<n>/`, and the status row says so.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use graft_core::on_disk::Generation;
use graft_git::repo::RepoHandle;

use super::{KEPT_ASIDE_DIR, MigrateError, refs, trees};

/// What one stray pass did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Strays {
    /// Old refs carried, deleted or set aside.
    pub refs: usize,
    /// Old files carried, deleted or set aside.
    pub files: usize,
    /// Where what differed went: the pass's folder when a file stayed
    /// there, else the pass's ref prefix when only refs did; `None` when
    /// nothing differed.
    pub kept_aside: Option<PathBuf>,
}

/// Carry or set aside everything under `from`'s names. The refs are listed
/// first, in one `for-each-ref` naming both prefixes (the pass number must
/// be unused by refs and folders alike); then every old folder's files;
/// then the refs, in one transaction.
pub(crate) fn settle_strays(repo: &RepoHandle, from: Generation) -> Result<Strays, MigrateError> {
    let common = repo.common_dir();
    let (old_refs, new_refs) = refs::list_both(repo, from)?;
    let kept_dir = common.join(Generation::CURRENT.dir()).join(KEPT_ASIDE_DIR);
    let mut pass = 1;
    let pass_dir = loop {
        let dir = kept_dir.join(pass.to_string());
        let taken = dir
            .try_exists()
            .map_err(|error| MigrateError::files(&dir, &error))?
            || new_refs
                .keys()
                .any(|name| name.starts_with(&refs::kept_aside_prefix(pass)));
        if !taken {
            break dir;
        }
        pass += 1;
    };
    let mut files = 0;
    for tree in trees::old_trees(common, from)? {
        files += trees::set_aside(common, &tree, &pass_dir)?.0;
    }
    let files_kept = trees::finish_pass(&pass_dir)?;
    let refs_kept = refs::carry_or_set_aside(repo, &old_refs, &new_refs, pass, from)?;
    let kept_aside = if files_kept {
        Some(pass_dir)
    } else if refs_kept > 0 {
        Some(PathBuf::from(refs::kept_aside_prefix(pass)))
    } else {
        None
    };
    Ok(Strays {
        refs: old_refs.len(),
        files,
        kept_aside,
    })
}

/// [`settle_strays`] over every generation older than `CURRENT`, newest
/// first: what a `switched` clean-up or a `clean` open finds. Each
/// generation takes its own pass number, and `kept_aside` names the first
/// that kept anything. Not yet ADR-0057 2026-10-11 §5's one pass across
/// generations; with one generation older than `CURRENT` the two agree.
pub(crate) fn settle_older_strays(repo: &RepoHandle) -> Result<Strays, MigrateError> {
    let mut all = Strays::default();
    for from in Generation::CURRENT.older() {
        let one = settle_strays(repo, from)?;
        all.refs += one.refs;
        all.files += one.files;
        all.kept_aside = all.kept_aside.or(one.kept_aside);
    }
    Ok(all)
}

/// [`old_names_present`] for any generation older than `CURRENT`.
pub(crate) fn older_names_present(repo: &RepoHandle) -> Result<bool, MigrateError> {
    for from in Generation::CURRENT.older() {
        if old_names_present(repo, from)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether anything is under `from`'s names again: a file anywhere below
/// `<common>/<from>/`, `<common>/refs/<from>/` or the opening worktree's
/// `<git_dir>/<from>/`. `stat` and `read_dir` only — no spawn, so a `clean`
/// open stays spawn-free. Refs only in `packed-refs` are not seen; an
/// older build writes loose ones.
pub(crate) fn old_names_present(repo: &RepoHandle, from: Generation) -> Result<bool, MigrateError> {
    let common = repo.common_dir();
    let old = from.dir();
    for folder in [
        common.join(old),
        common.join("refs").join(old),
        repo.git_dir().join(old),
    ] {
        if holds_a_file(&folder)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Anything but a folder anywhere below `folder`; `false` when it is
/// missing.
fn holds_a_file(folder: &Path) -> Result<bool, MigrateError> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        // Not a folder at all: something is there.
        Err(error) if error.kind() == io::ErrorKind::NotADirectory => return Ok(true),
        Err(error) => return Err(MigrateError::files(folder, &error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| MigrateError::files(folder, &error))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| MigrateError::files(&path, &error))?;
        if !kind.is_dir() || holds_a_file(&path)? {
            return Ok(true);
        }
    }
    Ok(false)
}
