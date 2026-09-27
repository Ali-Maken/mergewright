//! A file's size before and after one commit: what a binary change shows
//! in place of lines (m7-power-tools/21c).

use std::path::Path;

use graft_core::oid::Oid;

use crate::blame::path_bytes;
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// A blob's size, in bytes, on each side of a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobSizes {
    /// At `commit^1` (under the old path for a rename). `None`: absent —
    /// the commit added the file, or is a root commit.
    pub old: Option<u64>,
    /// At `commit`. `None`: the commit deleted the file.
    pub new: Option<u64>,
}

/// Sizes of `path` at `commit` and of `old_path.unwrap_or(path)` at
/// `commit^1`. `None`: absent that side (added, deleted, root commit). One
/// `git cat-file --batch-check` for both; `missing` → `None`, anything else
/// unparseable → `Err`.
///
/// A path holding a newline cannot be asked this way (batch input is one
/// object per line); git then answers for a name that does not exist, and
/// the reply fails to parse or reads `missing`.
///
/// # Errors
/// [`RunError`]: git failed, or answered in a shape this does not read.
pub fn blob_sizes(
    repo: &RepoHandle,
    commit: Oid,
    path: &Path,
    old_path: Option<&Path>,
    stop: &dyn Stop,
) -> Result<BlobSizes, RunError> {
    let mut input = format!("{commit}^1:").into_bytes();
    input.extend(path_bytes(old_path.unwrap_or(path)));
    input.push(b'\n');
    input.extend(format!("{commit}:").into_bytes());
    input.extend(path_bytes(path));
    input.push(b'\n');
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Word("--batch-check=%(objecttype) %(objectsize)".to_owned()),
        ])
        .stdin(input)
        .run(stop)?;
    let lines: Vec<&[u8]> = output
        .stdout
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    let [old, new] = lines.as_slice() else {
        return Err(RunError::Unparsable(format!(
            "cat-file answered {} lines for 2 sizes",
            lines.len()
        )));
    };
    Ok(BlobSizes {
        old: size(old)?,
        new: size(new)?,
    })
}

/// One `--batch-check` line: `blob <n>` → `Some(n)`, `<input> missing` →
/// `None`.
fn size(line: &[u8]) -> Result<Option<u64>, RunError> {
    if line.ends_with(b" missing") {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(line);
    text.strip_prefix("blob ")
        .and_then(|n| n.parse().ok())
        .map(Some)
        .ok_or_else(|| RunError::Unparsable(format!("cat-file printed {text:?}, not a blob size")))
}
