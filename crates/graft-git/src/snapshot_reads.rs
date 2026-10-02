//! In-process reads for the before-snapshot's `raw/` (ADR-0052,
//! m7-power-tools/30r): which regular files differ from `HEAD`, and whether
//! `core.autocrlf` converts. No `git` spawn: `raw_record` asks both on every
//! before-snapshot, and each spawn cost about 6 ms on the perf machine
//! (`docs/perf/30q-snapshot.txt`).
//!
//! Both answer the question git would, because a wrong "no" loses his bytes:
//! every doubt is a yes (ADR-0052). The attribute question stays with
//! `git check-attr`: its sources are git's to know.

use std::collections::HashMap;

use gix::objs::tree::EntryKind;
use graft_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::run::Stop;

/// A regular file of `to_tree` that differs from `HEAD`'s tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedFile {
    /// Repo-relative, `/`-separated, raw bytes.
    pub path: Vec<u8>,
    /// `100755` in the tree (else `100644`).
    pub executable: bool,
    /// The blob `to_tree` holds at `path`.
    pub blob: Oid,
}

/// Why a snapshot read failed. Never an empty answer standing in for one.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// An object could not be found, read or decoded as the tree it should
    /// be.
    #[error("cannot read object {oid}: {message}")]
    Object {
        /// The object.
        oid: Oid,
        /// gix's reason, one line.
        message: String,
    },
    /// The configuration could not be read from disk.
    #[error("cannot read the git config: {0}")]
    Config(String),
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

/// The regular files (100644/100755) in `to_tree` that differ from `head`'s
/// tree: added, modified, mode-only or type-changed *into* a regular file.
/// The same set `git diff-tree -r -z --no-renames <head> <to_tree>` gives,
/// filtered to regular files on the new side. Symlinks, gitlinks and
/// deletions are left out. `head: None` (unborn) means every regular file in
/// `to_tree`. Sorted by path bytes.
///
/// A subtree with the same id on both sides is skipped unread, so a clean
/// worktree costs two tree reads. Modes compare as git canonicalises them
/// (`100664` is `100644`), so a legacy mode alone is no change.
///
/// # Errors
/// [`ReadError::Object`] for an object that cannot be read — never an empty
/// list; [`ReadError::Stopped`] when `stop` fires.
pub fn changed_files(
    repo: &RepoHandle,
    head: Option<Oid>,
    to_tree: Oid,
    stop: &dyn Stop,
) -> Result<Vec<ChangedFile>, ReadError> {
    let gix = repo.gix();
    let old = head.map(|commit| tree_of(gix, commit)).transpose()?;
    let new = tree_of(gix, to_tree)?;
    let mut changed = Vec::new();
    // Depth first over the pairs still to compare: `HEAD`'s side (if it has
    // a directory there), the new side, and their directory's path with its
    // trailing `/`.
    let mut pending: Vec<(Option<Oid>, Oid, Vec<u8>)> = vec![(old, new, Vec::new())];
    while let Some((old, new, prefix)) = pending.pop() {
        if stop.should_stop() {
            return Err(ReadError::Stopped);
        }
        if old == Some(new) {
            continue;
        }
        let before: HashMap<Vec<u8>, Entry> = match old {
            Some(old) => entries(gix, old)?
                .into_iter()
                .map(|entry| (entry.name.clone(), entry))
                .collect(),
            None => HashMap::new(),
        };
        for entry in entries(gix, new)? {
            let at = before.get(&entry.name);
            let mut path = prefix.clone();
            path.extend_from_slice(&entry.name);
            match entry.kind {
                EntryKind::Blob | EntryKind::BlobExecutable => {
                    let same = at.is_some_and(|at| at.kind == entry.kind && at.id == entry.id);
                    if !same {
                        changed.push(ChangedFile {
                            path,
                            executable: entry.kind == EntryKind::BlobExecutable,
                            blob: entry.id,
                        });
                    }
                }
                EntryKind::Tree => {
                    // A directory where `HEAD` had a file, a link or nothing:
                    // every file under it is new.
                    let old = at.filter(|at| at.kind == EntryKind::Tree).map(|at| at.id);
                    path.push(b'/');
                    pending.push((old, entry.id, path));
                }
                // git filters neither a symlink nor a gitlink.
                EntryKind::Link | EntryKind::Commit => {}
            }
        }
    }
    changed.sort_by(|one, other| one.path.cmp(&other.path));
    Ok(changed)
}

/// `core.autocrlf` as git would read it now: the config stack re-read from
/// disk on this call (same sources, includes and worktree config as git),
/// never the handle's cached snapshot. `false` only for a git-false value or
/// unset; `true`, `input` and any other value are `true` (ADR-0052: doubt is
/// a yes).
///
/// The handle's repository is opened once for the whole run, so its config
/// is from launch, and he may set `core.autocrlf` in his shell after that.
/// So the repository is opened again, with the handle's own options, for
/// this one read. One option differs: the git installation's own config file
/// is read too (`git_binary`), as git reads it; Apple's and Homebrew's git
/// keep theirs outside `/etc/gitconfig`. gix finds that file by running
/// `git config` once per process; no later call spawns.
///
/// Config given in the environment counts too, as it does for every git
/// this process spawns: gix reads `GIT_CONFIG_COUNT`/`_KEY_n`/`_VALUE_n`
/// itself; `GIT_CONFIG_PARAMETERS` (what `git -c` hands its children, set
/// when Graft runs under git) gix does not read, so its `core.autocrlf`, if
/// any, is passed in as a `-c` override (m8-polish-and-ship/21).
///
/// # Errors
/// [`ReadError::Config`] when the repository cannot be opened again, or
/// `GIT_CONFIG_PARAMETERS` is malformed (git itself refuses to run then).
pub fn autocrlf_converts(repo: &RepoHandle) -> Result<bool, ReadError> {
    let gix = repo.gix();
    let mut options = gix.open_options().clone();
    options.permissions.config.git_binary = true;
    if let Some(parameters) = std::env::var_os("GIT_CONFIG_PARAMETERS") {
        use std::os::unix::ffi::OsStrExt;
        let pairs = config_parameters(parameters.as_bytes()).map_err(ReadError::Config)?;
        let overrides: Vec<Vec<u8>> = pairs
            .into_iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(b"core.autocrlf"))
            .map(|(_, value)| match value {
                // A bare key: an implicit `true`, as `-c core.autocrlf` is.
                None => b"core.autocrlf".to_vec(),
                Some(value) => [&b"core.autocrlf="[..], &value].concat(),
            })
            .collect();
        options = options.cli_overrides(overrides);
    }
    let fresh =
        gix::open_opts(gix.git_dir(), options).map_err(|why| ReadError::Config(one_line(&why)))?;
    match fresh.config_snapshot().try_boolean("core.autocrlf") {
        // Unset: git converts nothing.
        Ok(None) => Ok(false),
        // git's own boolean: `false`, `no`, `off`, `0` and empty are off; a
        // bare key, `true`, `yes`, `on` and any other number are on.
        Ok(Some(converts)) => Ok(converts),
        // Not a boolean: `input` converts, and anything else is a value git
        // would stop on. Either way, a yes.
        Err(_not_a_boolean) => Ok(true),
    }
}

/// One entry of a tree, owned, its mode canonicalised as git does.
struct Entry {
    name: Vec<u8>,
    kind: EntryKind,
    id: Oid,
}

/// The tree `oid` names, peeled from a commit (or tag) as `diff-tree` peels
/// its tree-ish arguments.
fn tree_of(repo: &gix::Repository, oid: Oid) -> Result<Oid, ReadError> {
    let object = repo
        .find_object(gix_id(oid)?)
        .map_err(|why| unreadable(oid, &why))?;
    let tree = object.peel_to_tree().map_err(|why| unreadable(oid, &why))?;
    Oid::from_bytes(tree.id.as_bytes()).map_err(|why| unreadable(oid, &why))
}

/// A tree's entries, in the tree's order.
fn entries(repo: &gix::Repository, tree: Oid) -> Result<Vec<Entry>, ReadError> {
    let object = repo
        .find_object(gix_id(tree)?)
        .map_err(|why| unreadable(tree, &why))?
        .try_into_tree()
        .map_err(|why| unreadable(tree, &why))?;
    let decoded = object.decode().map_err(|why| unreadable(tree, &why))?;
    decoded
        .entries
        .iter()
        .map(|entry| {
            let id = Oid::from_bytes(entry.oid.as_bytes()).map_err(|why| {
                let message = format!("entry {}: {why}", entry.filename);
                unreadable(tree, &message)
            })?;
            Ok(Entry {
                name: entry.filename.to_vec(),
                kind: entry.mode.kind(),
                id,
            })
        })
        .collect()
}

fn gix_id(oid: Oid) -> Result<gix::ObjectId, ReadError> {
    gix::hash::oid::try_from_bytes(oid.as_bytes())
        .map(ToOwned::to_owned)
        .map_err(|why| unreadable(oid, &why))
}

fn unreadable(oid: Oid, why: &dyn std::fmt::Display) -> ReadError {
    ReadError::Object {
        oid,
        message: one_line(why),
    }
}

/// One `GIT_CONFIG_PARAMETERS` entry: the key, and its value (`None` for
/// a bare key, an implicit `true`).
type ConfigPair = (Vec<u8>, Option<Vec<u8>>);

/// `GIT_CONFIG_PARAMETERS` as git parses it (`config.c`
/// `parse_config_env_list`): space-separated entries, each `'key'='value'`,
/// `'key'=` (implicit `true`), or the older `'key=value'` / `'key'`; every
/// quoted part shell single-quoted, `'\''` and `'\!'` standing for `'`
/// and `!`. Anything else is the error git gives ("bogus format").
fn config_parameters(raw: &[u8]) -> Result<Vec<ConfigPair>, String> {
    let bogus = || "bogus format in GIT_CONFIG_PARAMETERS".to_owned();
    let mut pairs = Vec::new();
    let mut rest = raw;
    while !rest.is_empty() {
        let (key, after) = single_quoted(rest).ok_or_else(bogus)?;
        rest = after;
        match rest.first() {
            None | Some(b' ' | b'\t' | b'\n') => {
                // Old style: `'key=value'`, or `'key'` bare.
                match key.iter().position(|byte| *byte == b'=') {
                    Some(at) => pairs.push((key[..at].to_vec(), Some(key[at + 1..].to_vec()))),
                    None => pairs.push((key, None)),
                }
            }
            Some(b'=') => {
                rest = &rest[1..];
                match rest.first() {
                    Some(b'\'') => {
                        let (value, after) = single_quoted(rest).ok_or_else(bogus)?;
                        if !matches!(after.first(), None | Some(b' ' | b'\t' | b'\n')) {
                            return Err(bogus());
                        }
                        rest = after;
                        pairs.push((key, Some(value)));
                    }
                    None | Some(b' ' | b'\t' | b'\n') => pairs.push((key, None)),
                    Some(_) => return Err(bogus()),
                }
            }
            Some(_) => return Err(bogus()),
        }
        while let Some(b' ' | b'\t' | b'\n') = rest.first() {
            rest = &rest[1..];
        }
    }
    Ok(pairs)
}

/// One shell single-quoted word at the start of `text` (git's
/// `sq_dequote_step`) and what follows it; `None` when `text` does not start
/// with a quote or the word is malformed.
fn single_quoted(text: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let mut rest = text.strip_prefix(b"'")?;
    let mut word = Vec::new();
    loop {
        let (&byte, after) = rest.split_first()?;
        rest = after;
        if byte != b'\'' {
            word.push(byte);
            continue;
        }
        // Out of the quotes: `\'` or `\!` then a quote resumes the word.
        match rest {
            [b'\\', escaped @ (b'\'' | b'!'), b'\'', after @ ..] => {
                word.push(*escaped);
                rest = after;
            }
            [b'\\', ..] => return None,
            _ => return Some((word, rest)),
        }
    }
}

#[cfg(test)]
mod tests;

/// `why` on one line: the status line has one.
fn one_line(why: &dyn std::fmt::Display) -> String {
    why.to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
