//! His own bytes back over what git wrote (R4, ADR-0052): the `raw/` tree a
//! before-snapshot keeps, written file by file.

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::run::{GitArg, RunError};

use crate::apply::Context;
use crate::snapshot;

use super::RestoreError;
use super::steps::{run, tree_entries};

/// R4 (ADR-0052): after git wrote `paths`, his own bytes for each of them
/// the step's `raw/` tree holds: the paths git's checkout of the cleaned tree
/// would not give back (an LF file under `core.autocrlf=true`, a one-way
/// clean filter). The index is not touched: the path reads stat-dirty until
/// git refreshes it, and clean after, because clean(raw) is the tree's blob.
///
/// Two spawns, whatever the count: one `ls-tree` of `raw/` (only the kept
/// paths, usually a handful) and one `cat-file --batch` feeding every blob.
pub(crate) fn write_raw(
    ctx: &mut Context<'_>,
    raw: Oid,
    paths: &[Vec<u8>],
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let scoped: BTreeSet<&[u8]> = paths.iter().map(Vec::as_slice).collect();
    let mut kept = tree_entries(ctx, raw, &[], commands)?;
    kept.retain(|path, _| scoped.contains(path.as_slice()));
    if kept.is_empty() {
        return Ok(());
    }
    let repo = ctx.repo;
    let mut wanted = Vec::new();
    for (_, id) in kept.values() {
        wanted.extend_from_slice(id.to_string().as_bytes());
        wanted.push(b'\n');
    }
    let args = vec![GitArg::Flag("cat-file"), GitArg::Flag("--batch")];
    let output = run(ctx, args, Some(wanted), commands)?;
    let blobs = batch_blobs(&output, kept.len())?;
    for ((path, (mode, _)), bytes) in kept.into_iter().zip(blobs) {
        let relative = snapshot::path_of(path);
        let written = match repo.workdir() {
            Some(root) => write_his_bytes(&root.join(&relative), bytes, mode == "100755"),
            // Unreachable in practice: git's own write above needs a working
            // tree, and a bare repository has none.
            None => Err(std::io::Error::other("the repository has no working tree")),
        };
        written.map_err(|source| RestoreError::WriteRaw {
            path: relative,
            source,
        })?;
    }
    Ok(())
}

/// `cat-file --batch`'s output cut into its `count` blobs, in order: each is
/// `<oid> blob <size>\n`, `size` bytes, then `\n`.
fn batch_blobs(output: &[u8], count: usize) -> Result<Vec<&[u8]>, RestoreError> {
    let mut rest = output;
    let mut blobs = Vec::with_capacity(count);
    for _ in 0..count {
        let header_end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| unparsable("cat-file --batch: a header with no newline"))?;
        let header = String::from_utf8_lossy(&rest[..header_end]);
        let size = match header.split(' ').collect::<Vec<_>>()[..] {
            [_, "blob", size] => size.parse::<usize>().ok(),
            _ => None,
        }
        .ok_or_else(|| unparsable(&format!("cat-file --batch: header {header:?}")))?;
        let start = header_end + 1;
        let end = start
            .checked_add(size)
            .filter(|end| rest.get(*end) == Some(&b'\n'))
            .ok_or_else(|| unparsable("cat-file --batch: a blob shorter than its size"))?;
        blobs.push(&rest[start..end]);
        rest = &rest[end + 1..];
    }
    Ok(blobs)
}

/// Write `bytes` at `path` as git's checkout would create it — mode `0o777`
/// or `0o666`, less the umask — through a temporary file beside it and a
/// rename, so a crash of this process leaves git's version or his, never
/// half of either. Not synced to disk first, like git's own checkout.
fn write_his_bytes(path: &std::path::Path, bytes: &[u8], executable: bool) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".graft-raw-{}", std::process::id()));
    let temporary = PathBuf::from(temporary);
    // `create_new`: a file already at that name is never written over.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(if executable { 0o777 } else { 0o666 })
        .open(&temporary)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| std::fs::rename(&temporary, path));
    if written.is_err() {
        // The failure above is the error reported. The temporary is ours
        // (`create_new`), and one left over is noise beside his file, never
        // his work, so failing to remove it changes nothing he has.
        let _removed = std::fs::remove_file(&temporary);
    }
    written
}

/// git printed something this cannot read, as a restore failure.
fn unparsable(what: &str) -> RestoreError {
    RestoreError::Run(RunError::Unparsable(what.to_owned()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    /// `cat-file --batch` cut by each header's size, not by newlines: a blob
    /// holding CRLF, LF and an empty one come out whole and in order; a
    /// `missing` line or a short blob is an error, never a guess.
    #[test]
    fn batch_output_is_cut_by_size() {
        let id = "a".repeat(40);
        let output = format!("{id} blob 5\na\r\nb\n\n{id} blob 0\n\n{id} blob 2\nc\n\n");
        let blobs = batch_blobs(output.as_bytes(), 3).expect("three blobs");
        assert_eq!(blobs, [&b"a\r\nb\n"[..], b"", b"c\n"]);

        assert!(batch_blobs(format!("{id} missing\n").as_bytes(), 1).is_err());
        assert!(batch_blobs(format!("{id} blob 9\nshort\n").as_bytes(), 1).is_err());
        assert!(batch_blobs(b"", 1).is_err(), "fewer blobs than asked for");
    }

    /// A raw write that fails names the path and the filesystem's reason in
    /// its own words, not the snapshot's ("snapshot scratch: …").
    #[test]
    fn a_failed_raw_write_names_the_path() {
        let error = RestoreError::WriteRaw {
            path: PathBuf::from("src/a.rs"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert_eq!(
            error.explain(),
            "could not write src/a.rs back as it was: denied"
        );
    }
}
