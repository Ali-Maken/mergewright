//! His own bytes back over what git wrote (R4, ADR-0052): the `raw/` tree a
//! before-snapshot keeps, written file by file.

use std::collections::BTreeSet;

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

/// The paths `raw` holds whose file on disk, cleaned as git would clean it,
/// is the blob `tree` holds there: his content, which a set-aside's put-back
/// (`stash apply`) wrote back in git's encoding, so [`write_raw`] may put his
/// own bytes over it (m7-power-tools/30s). A path whose content is not his
/// (the put-back merged into a file the operation changed) is left out: his
/// bytes are for his content, never for a merge of it. So is anything that is
/// not a regular file on disk.
///
/// Three spawns at most, whatever the count: `ls-tree` of `raw` (usually a
/// handful of paths), `ls-tree` of `tree` for those paths, and one
/// `hash-object --stdin-paths`, which cleans each file by its own path.
pub(crate) fn same_content_paths(
    ctx: &mut Context<'_>,
    raw: Oid,
    tree: Oid,
    commands: &mut Vec<String>,
) -> Result<Vec<Vec<u8>>, RestoreError> {
    let kept: Vec<Vec<u8>> = tree_entries(ctx, raw, &[], commands)?.into_keys().collect();
    if kept.is_empty() {
        return Ok(Vec::new());
    }
    let Some(root) = ctx.repo.workdir().map(std::path::Path::to_path_buf) else {
        // A bare repository has no working tree for a put-back to write.
        return Ok(Vec::new());
    };
    let on_disk: Vec<(Vec<u8>, Oid)> = tree_entries(ctx, tree, &kept, commands)?
        .into_iter()
        .filter(|(path, (mode, _))| {
            mode.starts_with("100")
                && std::fs::symlink_metadata(root.join(snapshot::path_of(path.clone())))
                    .is_ok_and(|metadata| metadata.is_file())
        })
        .map(|(path, (_, blob))| (path, blob))
        .collect();
    if on_disk.is_empty() {
        return Ok(Vec::new());
    }
    let mut lines = Vec::new();
    for (path, _) in &on_disk {
        lines.extend(snapshot::c_quoted(path));
        lines.push(b'\n');
    }
    let args = vec![GitArg::Flag("hash-object"), GitArg::Flag("--stdin-paths")];
    let output = run(ctx, args, Some(lines), commands)?;
    let cleaned = String::from_utf8_lossy(&output)
        .lines()
        .map(|line| {
            Oid::from_hex(line)
                .map_err(|_| unparsable(&format!("hash-object --stdin-paths printed {line:?}")))
        })
        .collect::<Result<Vec<Oid>, RestoreError>>()?;
    if cleaned.len() != on_disk.len() {
        return Err(unparsable(&format!(
            "hash-object --stdin-paths printed {} ids for {} paths",
            cleaned.len(),
            on_disk.len()
        )));
    }
    Ok(on_disk
        .into_iter()
        .zip(cleaned)
        .filter(|((_, blob), hash)| blob == hash)
        .map(|((path, _), _)| path)
        .collect())
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

/// How many temporary names [`write_his_bytes`] tries before giving up: a
/// clash means a file is already there (a crashed run's leftover).
const TEMPORARY_TRIES: u32 = 16;

/// Write `bytes` at `path` as git's checkout would create it — mode `0o777`
/// or `0o666`, less the umask — through a temporary file beside it and a
/// rename, so a crash of this process leaves git's version or his, never
/// half of either. Not synced to disk first, like git's own checkout.
///
/// The temporary is a short name of its own in the same directory
/// (`.~<6 hex>`), not `path`'s name plus a suffix, so a file name near the
/// filesystem's limit (255 bytes) still gets one.
fn write_his_bytes(path: &std::path::Path, bytes: &[u8], executable: bool) -> std::io::Result<()> {
    write_his_bytes_via(
        path,
        bytes,
        executable,
        (0..TEMPORARY_TRIES).map(|attempt| temporary_name(std::process::id(), attempt)),
    )
}

/// Attempt `attempt`'s `.~<6 hex>` name for process `pid`:
/// `(pid × 16 + attempt) mod 2²⁴`. Fixed per process, so the 16 names a
/// process tries are its own (two Graft processes differ unless their pids
/// match mod 2²⁰) and a test can name them; one write's temporary is renamed
/// away before the next write starts, so a process never clashes with
/// itself, only with a leftover.
fn temporary_name(pid: u32, attempt: u32) -> String {
    let n = pid.wrapping_mul(TEMPORARY_TRIES).wrapping_add(attempt) & 0x00ff_ffff;
    format!(".~{n:06x}")
}

/// [`write_his_bytes`] with the temporary's candidate names given: the first
/// one free in `path`'s directory is used. A name already taken is skipped,
/// never written over (`create_new`); all of them taken is `AlreadyExists`.
fn write_his_bytes_via(
    path: &std::path::Path,
    bytes: &[u8],
    executable: bool,
    names: impl IntoIterator<Item = String>,
) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let mut created = None;
    for name in names {
        let temporary = dir.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(if executable { 0o777 } else { 0o666 })
            .open(&temporary)
        {
            Ok(file) => {
                created = Some((file, temporary));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    let Some((mut file, temporary)) = created else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "every temporary name tried beside it is taken",
        ));
    };
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

    /// `--stdin-paths` lines are C-quoted as git prints them: a plain path
    /// in quotes; `"` and `\` escaped; a newline, a tab and a non-ASCII byte
    /// as octal, so no path can end its line early or be re-read otherwise.
    #[test]
    fn a_path_is_c_quoted_for_stdin_paths() {
        use snapshot::c_quoted;
        assert_eq!(c_quoted(b"src/a.rs"), b"\"src/a.rs\"");
        assert_eq!(c_quoted(b"a\"b\\c"), b"\"a\\\"b\\\\c\"");
        assert_eq!(c_quoted(b"new\nline\ttab"), b"\"new\\012line\\011tab\"");
        assert_eq!(c_quoted("é".as_bytes()), b"\"\\303\\251\"");
    }

    /// A 250-byte file name: the old `<name>.graft-raw-<pid>` temporary
    /// passed `NAME_MAX` (255) and failed every time; the short `.~<hex>` name
    /// beside it does not. His bytes and the exec bit land, no temporary is
    /// left behind.
    #[test]
    fn a_250_byte_name_is_written_through_a_short_temporary() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("n".repeat(250));
        std::fs::write(&path, b"git's\r\n").unwrap();

        write_his_bytes(&path, b"his\n", true).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"his\n");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o100, 0o100, "executable: {mode:o}");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1, "no temporary left: {names:?}");
    }

    /// A temporary name already taken is skipped and the file there is left
    /// as it was; the next name is used.
    #[test]
    fn a_taken_temporary_name_is_skipped_never_written_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.rs");
        std::fs::write(&path, b"git's\r\n").unwrap();
        std::fs::write(dir.path().join(".~000001"), b"someone's").unwrap();

        write_his_bytes_via(
            &path,
            b"his\n",
            false,
            [".~000001".to_owned(), ".~000002".to_owned()],
        )
        .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"his\n");
        assert_eq!(
            std::fs::read(dir.path().join(".~000001")).unwrap(),
            b"someone's"
        );
        assert!(!dir.path().join(".~000002").exists(), "renamed away");
    }

    /// Every name taken: `AlreadyExists`, `path` untouched, nothing written
    /// over.
    #[test]
    fn every_temporary_name_taken_fails_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.rs");
        std::fs::write(&path, b"git's\r\n").unwrap();
        std::fs::write(dir.path().join(".~000001"), b"someone's").unwrap();

        let failed = write_his_bytes_via(&path, b"his\n", false, [".~000001".to_owned()]);

        assert_eq!(
            failed.map_err(|error| error.kind()),
            Err(std::io::ErrorKind::AlreadyExists)
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"git's\r\n");
        assert_eq!(
            std::fs::read(dir.path().join(".~000001")).unwrap(),
            b"someone's"
        );
    }

    /// A process's 16 names are `.~<6 hex>` of `pid × 16 + attempt`, all
    /// different; the integration tests' "file in the way" helper names them
    /// the same way.
    #[test]
    fn temporary_names_are_short_and_fixed_per_process() {
        assert_eq!(temporary_name(1, 0), ".~000010");
        assert_eq!(temporary_name(1, 15), ".~00001f");
        assert_eq!(temporary_name(0x0123_4567, 3), ".~345673");
        let names: std::collections::BTreeSet<String> = (0..TEMPORARY_TRIES)
            .map(|attempt| temporary_name(99_999, attempt))
            .collect();
        assert_eq!(names.len(), 16);
    }

    /// A raw write that fails names the path and the filesystem's reason in
    /// its own words, not the snapshot's ("snapshot scratch: …").
    #[test]
    fn a_failed_raw_write_names_the_path() {
        let error = RestoreError::WriteRaw {
            path: std::path::PathBuf::from("src/a.rs"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert_eq!(
            error.explain(&crate::keys::Letters),
            "could not write src/a.rs back as it was: denied"
        );
    }
}
