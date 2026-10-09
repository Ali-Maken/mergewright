//! Every tracked path, for the palette's FILES group (m8 12b).
//!
//! One `git ls-files -z` per palette open. Held as one byte buffer and the
//! end offset of each path, not a `Vec<PathBuf>`: a 100 000-path index is one
//! allocation and 400 KB of offsets, and the palette's matcher walks bytes.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The paths `git ls-files` lists, in index order, each once.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TrackedPaths {
    bytes: Vec<u8>,
    /// `ends[i]` is one past path `i`'s last byte in `bytes`; path `i` starts
    /// at `ends[i - 1]` (0 for the first).
    ends: Vec<u32>,
}

impl TrackedPaths {
    /// How many paths.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// No paths: a bare repository, or an empty index.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    /// Path `ix`, as git printed it (raw bytes, `/`-separated, repo-relative).
    #[must_use]
    pub fn get(&self, ix: usize) -> Option<&[u8]> {
        let end = *self.ends.get(ix)? as usize;
        let start = match ix {
            0 => 0,
            _ => self.ends[ix - 1] as usize,
        };
        Some(&self.bytes[start..end])
    }

    /// Every path, in index order.
    pub fn iter(&self) -> impl Iterator<Item = &[u8]> + '_ {
        (0..self.len()).filter_map(|ix| self.get(ix))
    }

    /// Parses `ls-files -z` output: split on NUL, empty records
    /// skipped, a path repeated on adjacent records (an unmerged path, once
    /// per stage) kept once.
    #[must_use]
    pub fn from_nul_separated(stdout: &[u8]) -> Self {
        let mut paths = TrackedPaths::default();
        for record in stdout.split(|&b| b == 0) {
            if record.is_empty() || paths.get(paths.len().wrapping_sub(1)) == Some(record) {
                continue;
            }
            paths.bytes.extend_from_slice(record);
            // `TRACKED_CAP` < 4 GiB, so every offset fits a `u32`.
            let end = u32::try_from(paths.bytes.len()).unwrap_or(u32::MAX);
            paths.ends.push(end);
        }
        paths
    }
}

/// The most bytes `ls-files -z` may print before the list is refused as
/// [`StatusError::TooLarge`]. Must stay under 4 GiB (offsets are `u32`).
pub const TRACKED_CAP: usize = 64 << 20;

/// Every tracked path, from one `git ls-files -z`. A bare repository has no
/// index to list: empty, no spawn.
///
/// # Errors
/// [`StatusError::Run`] if git could not be run, was stopped, or exited
/// non-zero; [`StatusError::TooLarge`] past [`TRACKED_CAP`].
pub fn tracked_paths(repo: &RepoHandle, stop: &dyn Stop) -> Result<TrackedPaths, StatusError> {
    tracked_paths_capped(repo, stop, TRACKED_CAP)
}

fn tracked_paths_capped(
    repo: &RepoHandle,
    stop: &dyn Stop,
    cap: usize,
) -> Result<TrackedPaths, StatusError> {
    if repo.workdir().is_none() {
        return Ok(TrackedPaths::default());
    }
    let output = GitInvocation::new(repo)
        .args([GitArg::Flag("ls-files"), GitArg::Flag("-z")])
        .output_cap(cap)
        .run(stop)?;
    if output.truncated {
        return Err(StatusError::TooLarge { bytes: cap });
    }
    Ok(TrackedPaths::from_nul_separated(&output.stdout))
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    use super::{StatusError, TrackedPaths, tracked_paths, tracked_paths_capped};
    use crate::fixture::Fixture;
    use crate::repo::RepoHandle;

    fn open(fixture: &Fixture) -> RepoHandle {
        RepoHandle::open(fixture.root()).expect("a fixture repository opens")
    }

    /// `git update-index --index-info` with raw bytes on stdin: paths that
    /// cannot be files on this disk (non-UTF-8 on APFS) or that are unmerged.
    fn index_info(fixture: &Fixture, input: &[u8]) {
        let mut child = Command::new("git")
            .args(["update-index", "--index-info"])
            .current_dir(fixture.root())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdin(Stdio::piped())
            .spawn()
            .expect("git starts");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(input)
            .expect("stdin takes the records");
        assert!(child.wait().expect("git ends").success());
    }

    fn blob(fixture: &Fixture) -> String {
        let file = fixture.root().join("blob.txt");
        std::fs::write(&file, "x\n").expect("a file");
        let id = fixture.git(&["hash-object", "-w", "blob.txt"]);
        std::fs::remove_file(&file).expect("removed");
        id
    }

    fn all(paths: &TrackedPaths) -> Vec<Vec<u8>> {
        paths.iter().map(<[u8]>::to_vec).collect()
    }

    #[test]
    fn tracked_paths_splits_on_nul_in_index_order() {
        let fixture = Fixture::empty_repo();
        let id = blob(&fixture);
        index_info(
            &fixture,
            format!("100644 {id}\tsrc/main.rs\n100644 {id}\tREADME.md\n100644 {id}\ta b.txt\n")
                .as_bytes(),
        );
        let paths = tracked_paths(&open(&fixture), &()).expect("reads");
        assert_eq!(
            all(&paths),
            vec![
                b"README.md".to_vec(),
                b"a b.txt".to_vec(),
                b"src/main.rs".to_vec()
            ]
        );
        assert_eq!(paths.len(), 3);
        assert_eq!(paths.get(2), Some(&b"src/main.rs"[..]));
        assert_eq!(paths.get(3), None);
    }

    #[test]
    fn tracked_paths_keeps_a_non_utf8_path() {
        let fixture = Fixture::empty_repo();
        let id = blob(&fixture);
        let mut input = format!("100644 {id}\t").into_bytes();
        input.extend_from_slice(b"caf\xe9.txt\n");
        // `core.quotepath` does not apply to `-z` output: the bytes come raw.
        index_info(&fixture, &input);
        let paths = tracked_paths(&open(&fixture), &()).expect("reads");
        assert_eq!(all(&paths), vec![b"caf\xe9.txt".to_vec()]);
    }

    #[test]
    fn tracked_paths_lists_an_unmerged_path_once() {
        let fixture = Fixture::empty_repo();
        let id = blob(&fixture);
        index_info(
            &fixture,
            format!("100644 {id} 1\tboth.txt\n100644 {id} 2\tboth.txt\n100644 {id} 3\tboth.txt\n100644 {id} 0\tz.txt\n")
                .as_bytes(),
        );
        let paths = tracked_paths(&open(&fixture), &()).expect("reads");
        assert_eq!(all(&paths), vec![b"both.txt".to_vec(), b"z.txt".to_vec()]);
    }

    #[test]
    fn tracked_paths_of_a_bare_repository_is_empty() {
        let fixture = Fixture::bare_repo();
        let paths = tracked_paths(&open(&fixture), &()).expect("no spawn, no error");
        assert!(paths.is_empty());
    }

    #[test]
    fn tracked_paths_over_the_cap_is_too_large() {
        let fixture = Fixture::empty_repo();
        let id = blob(&fixture);
        index_info(
            &fixture,
            format!("100644 {id}\tone.txt\n100644 {id}\ttwo.txt\n").as_bytes(),
        );
        let capped = tracked_paths_capped(&open(&fixture), &(), 5);
        assert!(
            matches!(capped, Err(StatusError::TooLarge { bytes: 5 })),
            "{capped:?}"
        );
    }
}
