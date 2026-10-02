//! A conflict file on disk and in the index: [`read`] and [`write_back`] for
//! the working-tree file, [`stage_bytes`] for the three index stages, and
//! [`diff3_file`], git's own `diff3` merge of them.

use std::io::{self, Read, Write};
use std::path::Path;

use graft_core::oid::Oid;

use super::{ConflictFile, ParseError, blob_id, parse};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError};

/// git's own rule for a binary file: a NUL in the first 8 KiB.
const BINARY_PROBE: usize = 8 * 1024;

/// The largest file `read` will parse.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Why a file could not be read as a conflict file.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The file could not be opened or read.
    #[error("could not read the file: {0}")]
    Io(#[from] io::Error),
    /// A NUL in the first 8 KiB: git calls it binary and writes no markers.
    #[error("the file is binary")]
    Binary,
    /// Larger than the resolver handles.
    #[error("the file is {bytes} bytes; the resolver handles up to {MAX_BYTES}")]
    TooLarge {
        /// The file's size.
        bytes: u64,
    },
    /// The bytes are not a well-formed marker file.
    #[error("{0}")]
    Parse(#[from] ParseError),
}

/// Reads and parses a working-tree file.
pub fn read(path: &Path) -> Result<ConflictFile, ReadError> {
    let mut file = std::fs::File::open(path)?;
    let bytes = file.metadata()?.len();
    if bytes > MAX_BYTES {
        return Err(ReadError::TooLarge { bytes });
    }
    // A capacity hint only: `bytes` is at most 8 MiB here, so the conversion
    // cannot fail on any target this builds for, and a 0 would merely grow.
    let mut content = Vec::with_capacity(usize::try_from(bytes).unwrap_or(0));
    file.read_to_end(&mut content)?;
    let probe = &content[..content.len().min(BINARY_PROBE)];
    if probe.contains(&0) {
        return Err(ReadError::Binary);
    }
    Ok(parse(&content)?)
}

/// Why a file could not be written back.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The file or its temporary could not be written or renamed.
    #[error("could not write the file: {0}")]
    Io(#[from] io::Error),
    /// The file's bytes are no longer the ones the caller parsed.
    #[error("the file changed on disk since it was read")]
    Changed {
        /// The blob id of what is there now.
        now: Oid,
    },
}

/// Writes `file`'s rendering over `path`, refusing when the file's current
/// blob id is not `expected`. The bytes go to a temporary in the same
/// directory with the file's mode bits, then `rename` over the path, so a
/// crash mid-write leaves the old file whole. Returns the new fingerprint.
pub fn write_back(path: &Path, file: &ConflictFile, expected: Oid) -> Result<Oid, WriteError> {
    let current = std::fs::read(path)?;
    let now = blob_id(&current);
    if now != expected {
        return Err(WriteError::Changed { now });
    }
    let mode = std::fs::metadata(path)?.permissions();
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path has no parent directory",
        )
    })?;
    let rendered = file.render();
    let mut temporary = tempfile::Builder::new()
        .prefix(".graft-")
        .tempfile_in(directory)?;
    temporary.write_all(&rendered)?;
    temporary.as_file().sync_all()?;
    temporary.as_file().set_permissions(mode)?;
    temporary.persist(path).map_err(|failed| failed.error)?;
    Ok(blob_id(&rendered))
}

/// One index stage of a path: its mode and blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stage {
    mode: u32,
    blob: Oid,
}

/// The three stages of an unmerged path, `git ls-files -u`; `None` where a
/// stage is missing (a whole-file shape: added or deleted on one side).
fn stages(repo: &RepoHandle, path: &Path) -> Result<[Option<Stage>; 3], RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("--literal-pathspecs"),
            GitArg::Flag("ls-files"),
            GitArg::Flag("-u"),
            GitArg::Flag("-z"),
            GitArg::Separator,
            GitArg::Path(path_bytes(path)),
        ])
        .run(&())?;
    let mut found = [None; 3];
    for record in output.stdout.split(|&b| b == 0) {
        if record.is_empty() {
            continue;
        }
        let malformed = || RunError::Unparsable(format!("ls-files -u record {record:?}"));
        let tab = record
            .iter()
            .position(|&b| b == b'\t')
            .ok_or_else(malformed)?;
        let header = std::str::from_utf8(&record[..tab]).map_err(|_| malformed())?;
        let mut fields = header.split(' ');
        let (Some(mode), Some(blob), Some(stage), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(malformed());
        };
        let mode = u32::from_str_radix(mode, 8).map_err(|_| malformed())?;
        let blob = Oid::from_hex(blob).map_err(|_| malformed())?;
        let slot = match stage {
            "1" => 0,
            "2" => 1,
            "3" => 2,
            _ => return Err(malformed()),
        };
        found[slot] = Some(Stage { mode, blob });
    }
    Ok(found)
}

/// One stage's content **raw**, as the object database holds it: `git show
/// :N:<path>` in effect, the blob `ls-files -u` names for that stage, with no
/// `.gitattributes` conversion applied. That is what a whole-file view wants
/// — the committed bytes — and it is *not* what the working tree holds under
/// `eol=crlf`, `core.autocrlf` or a smudge filter; [`diff3_file`] uses the
/// converted form (`filtered_bytes`) for that reason.
///
/// `Ok(None)` when the stage is missing — the path was added or deleted on
/// that side — rather than a git error to parse.
pub fn stage_bytes(repo: &RepoHandle, path: &Path, stage: u8) -> Result<Option<Vec<u8>>, RunError> {
    let slot = match stage {
        1..=3 => usize::from(stage - 1),
        other => {
            return Err(RunError::Unparsable(format!(
                "stage {other} is not one of 1, 2, 3"
            )));
        }
    };
    let Some(found) = stages(repo, path)?[slot] else {
        return Ok(None);
    };
    blob_bytes(repo, found.blob).map(Some)
}

/// A blob's content **raw**, `git cat-file blob <id>`: the object's own
/// bytes, no `.gitattributes` conversion.
fn blob_bytes(repo: &RepoHandle, blob: Oid) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("blob"),
            GitArg::Oid(blob),
        ])
        .run(&())?;
    Ok(output.stdout)
}

/// A blob's content **as it would sit in the working tree at `path`**: `git
/// cat-file --filters --path=<path> <id>`, so the path's `.gitattributes`
/// apply — `text`/`eol=crlf`/`core.autocrlf` line endings, `ident`, a smudge
/// filter.
///
/// Why [`diff3_file`] needs this and not [`blob_bytes`]: a repository with
/// `eol=crlf` stores LF blobs and checks out CRLF, so merging the raw stages
/// reproduces LF sides that match no line of the CRLF file on disk, and
/// [`attach_base`] would call a pristine file edited. Measured on git 2.54,
/// 2026-09-22: with `f.txt text eol=crlf`, the stop writes CRLF throughout
/// (markers included) while `git show :2:f.txt` is LF; `merge-file --diff3`
/// over the filtered copies prints CRLF sides and CRLF markers, matching the
/// working tree line for line.
fn filtered_bytes(repo: &RepoHandle, path: &Path, blob: Oid) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("--filters"),
            GitArg::Glued("--path=", path_bytes(path)),
            GitArg::Oid(blob),
        ])
        .run(&())?;
    Ok(output.stdout)
}

/// Why git's `diff3` blocks could not be reproduced.
#[derive(Debug, thiserror::Error)]
pub enum Diff3Error {
    /// A stage could not be read.
    #[error(transparent)]
    Run(#[from] RunError),
    /// The scratch copies of the stages could not be written.
    #[error("could not write the stages to a scratch directory: {0}")]
    Io(#[from] io::Error),
    /// `merge-file` itself failed (its exit code past the conflict count).
    #[error("git merge-file failed: {stderr_first_line}")]
    MergeFile {
        /// The exit code; `None` when git was killed.
        status: Option<i32>,
        /// What it said.
        stderr_first_line: String,
    },
    /// `merge-file`'s output did not parse — a marker rule this module has
    /// wrong, not a repository state.
    #[error("git merge-file printed something this parser cannot read: {0}")]
    Parse(#[from] ParseError),
}

/// `merge-file` exits with the conflict count, capped here; above it, or
/// killed, something went wrong.
const MERGE_FILE_MAX_CONFLICTS: i32 = 127;

/// git's own conflict file for the path, base included: the three stages
/// written to a scratch directory **in their working-tree form** (see
/// `filtered_bytes`) and merged with `git merge-file -p --diff3`, whose
/// output is what git would have written to the working tree under the
/// `diff3` conflict style — same engine, same hunks. `None` when a stage is
/// missing, since a whole-file shape has no blocks.
///
/// The scratch directory is a `tempfile` directory, not one under `.git/`:
/// nothing is left in the user's repository if this process dies mid-way,
/// and a read-only checkout (the sealed test corpus among them) works too.
/// The labels on the returned blocks are `ours`, `base`, `theirs`;
/// [`attach_base`](super::attach_base) compares content, never labels.
pub fn diff3_file(repo: &RepoHandle, path: &Path) -> Result<Option<ConflictFile>, Diff3Error> {
    let [Some(base), Some(ours), Some(theirs)] = stages(repo, path)? else {
        return Ok(None);
    };
    let scratch = tempfile::Builder::new().prefix("graft-diff3-").tempdir()?;
    for (name, stage) in [("ours", ours), ("base", base), ("theirs", theirs)] {
        // Filtered, not raw: the sides have to equal the working tree's
        // lines byte for byte for `attach_base` to match them.
        std::fs::write(
            scratch.path().join(name),
            filtered_bytes(repo, path, stage.blob)?,
        )?;
    }
    let raw = GitInvocation::in_directory(scratch.path())
        .args([
            GitArg::Flag("merge-file"),
            GitArg::Flag("-p"),
            GitArg::Flag("--diff3"),
            GitArg::Flag("-L"),
            GitArg::Flag("ours"),
            GitArg::Flag("-L"),
            GitArg::Flag("base"),
            GitArg::Flag("-L"),
            GitArg::Flag("theirs"),
            GitArg::Flag("ours"),
            GitArg::Flag("base"),
            GitArg::Flag("theirs"),
        ])
        .run_raw(&())?;
    match raw.code {
        Some(conflicts) if (0..=MERGE_FILE_MAX_CONFLICTS).contains(&conflicts) => {}
        status => {
            return Err(Diff3Error::MergeFile {
                status,
                stderr_first_line: first_line(&raw.output.stderr),
            });
        }
    }
    Ok(Some(parse(&raw.output.stdout)?))
}

/// The first non-empty line of some git stderr, lossily.
fn first_line(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    match text.lines().map(str::trim).find(|line| !line.is_empty()) {
        Some(line) => line.to_owned(),
        // git said nothing: not a failure to read, just nothing to show.
        None => String::new(),
    }
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}
