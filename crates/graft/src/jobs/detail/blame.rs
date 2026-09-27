//! The blame job (m7-power-tools/20): the blame read, and for `^` the line
//! the cursor goes to.

use std::sync::Arc;

use graft_git::blame::{BlameError, blame};
use graft_git::diff::{DiffOptions, DiffSpec, FileEntry, FileStatus, file_diff};

use super::super::Cancel;
use crate::repos::RepoStore;
use crate::update::place::{BlameAnswer, BlameFailure, BlameRequest, came_from};

/// A blame place's read (m7-power-tools/20): the blame, and for `^` the
/// line the cursor goes to, from the commit's own diff of the file with no
/// context (`came_from`).
pub(in crate::jobs) fn read_blame(
    repos: &RepoStore,
    request: &BlameRequest,
    stop: &Cancel,
) -> Result<Arc<BlameAnswer>, BlameFailure> {
    let repo = repos
        .shared()
        .map_err(|error| BlameFailure::Failed(error.to_string()))?;
    let handle = repo.handle();
    let blame = blame(&handle, &request.rev, &request.path, &request.options, stop)
        .map_err(|error| blame_failure(&error))?;
    let line = match &request.came_from {
        None => None,
        Some(asked) => {
            let entry = FileEntry {
                path: path_bytes(&asked.path),
                old_path: (asked.old_path != asked.path).then(|| path_bytes(&asked.old_path)),
                status: FileStatus::Modified,
                added: None,
                removed: None,
            };
            let spec = DiffSpec::Commit {
                commit: asked.commit,
                against: None,
            };
            let options = DiffOptions {
                context: 0,
                ignore_whitespace: false,
            };
            let diff = file_diff(&handle, &spec, &entry, &options, usize::MAX, stop)
                .map_err(|error| BlameFailure::Failed(error.to_string()))?;
            Some(came_from(&diff.hunks, asked.line))
        }
    };
    Ok(Arc::new(BlameAnswer { blame, line }))
}

/// What the place makes of git's refusal: the two it words itself (B1's
/// `… isn't committed yet · no blame`, `… is binary · no blame`), and git's
/// or the system's own words for the rest. A shallow file that could not be
/// read says so on one line: the place draws its body on one row.
pub(super) fn blame_failure(error: &BlameError) -> BlameFailure {
    match error {
        BlameError::Binary => BlameFailure::Binary,
        BlameError::Untracked => BlameFailure::Untracked,
        BlameError::Shallow { .. } => {
            let words = error.to_string();
            // `split` yields at least one piece, even of an empty string.
            let first = words.split('\n').next().unwrap_or(&words);
            BlameFailure::Failed(first.to_owned())
        }
        BlameError::Run(_)
        | BlameError::Parse(_)
        | BlameError::Stopped
        | BlameError::TooLarge { .. }
        | BlameError::Changed { .. } => BlameFailure::Failed(error.to_string()),
    }
}

/// `path` as git spells it.
#[cfg(unix)]
fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

/// `path` as git spells it.
#[cfg(not(unix))]
fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    path.to_string_lossy().replace('\\', "/").into_bytes()
}
