//! Submodules in the status: each one's own state, and those never set up.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The `sub` field: `N...` (not a submodule) or `S<c><m><u>`, where each
/// letter is its flag or `.`. Anything else is not what git prints.
pub(super) fn submodule_status(field: &[u8]) -> Result<Option<SubmoduleStatus>, StatusError> {
    let flag = |letter: u8, set: u8| match letter {
        b'.' => Some(false),
        l if l == set => Some(true),
        _ => None,
    };
    let parsed = match field {
        b"N..." => return Ok(None),
        [b'S', c, m, u] => (|| {
            Some(SubmoduleStatus {
                pointer_moved: flag(*c, b'C')?,
                changed_inside: flag(*m, b'M')?,
                untracked_inside: flag(*u, b'U')?,
            })
        })(),
        _ => None,
    };
    parsed.map(Some).ok_or_else(|| {
        StatusError::Malformed(format!(
            "a sub field must be N... or S<C|.><M|.><U|.>, not {:?}",
            String::from_utf8_lossy(field)
        ))
    })
}

/// Every submodule that is **not set up**: a gitlink (mode `160000`) in the
/// index whose directory has no `.git` — a clone that never ran `git
/// submodule update --init`, or one `deinit`ed since (m7-power-tools/13).
/// `git status` says nothing about these at all, so [`read`] cannot.
///
/// No `.gitmodules` in the worktree → no spawn and an empty list: the common
/// repository pays nothing. Otherwise one `git ls-files --stage -z`.
///
/// # Errors
/// [`StatusError::Run`] if git could not be run or exited non-zero;
/// [`StatusError::TooLarge`] past `OUTPUT_CAP`; [`StatusError::Malformed`]
/// for a line that is not `<mode> <oid> <stage>\t<path>`.
pub fn submodules_not_set_up(
    repo: &RepoHandle,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, StatusError> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };
    if !workdir.join(".gitmodules").exists() {
        return Ok(Vec::new());
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("ls-files"),
            GitArg::Flag("--stage"),
            GitArg::Flag("-z"),
        ])
        .output_cap(OUTPUT_CAP)
        .run(stop)?;
    if output.truncated {
        return Err(StatusError::TooLarge { bytes: OUTPUT_CAP });
    }
    let mut paths = Vec::new();
    for record in output.stdout.split(|&b| b == 0) {
        if record.is_empty() {
            continue;
        }
        let tab = record.iter().position(|&b| b == b'\t').ok_or_else(|| {
            StatusError::Malformed("an ls-files --stage record has no tab".to_owned())
        })?;
        let (info, path) = (&record[..tab], &record[tab + 1..]);
        if !info.starts_with(b"160000 ") {
            continue;
        }
        let path = bytes_to_pathbuf(path);
        // A set-up submodule has a `.git` (a gitfile into the parent's
        // `.git/modules`, or an old-style directory); either counts.
        // An unmerged gitlink is listed once per stage, consecutively.
        if paths.last() != Some(&path) && !workdir.join(&path).join(".git").exists() {
            paths.push(path);
        }
    }
    Ok(paths)
}
