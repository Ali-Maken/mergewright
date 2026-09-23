//! Reading `$GIT_DIR/MERGE_MSG`, the message git prepares for the commit a
//! stopped operation will write — and, in its comment lines, the list of
//! files the operation conflicted on (m6-conflicts/09).
//!
//! **Read only.** Graft never writes this file: git's own state files are
//! written by git alone (Amendment 2 hard choice 3).
//!
//! What git writes, measured on git 2.54.0 (Apple Git-157): a stopped merge,
//! pull, rebase (merge backend), cherry-pick and revert each end the message
//! with a `# Conflicts:` line — the comment prefix, a space, `Conflicts:` —
//! then one line per file: the prefix, a tab, the path **unquoted** (a space
//! stays a space), e.g. `#\ta.txt`. A conflicted `stash pop` writes no
//! `MERGE_MSG` at all.

use std::io;
use std::path::PathBuf;

use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

/// `MERGE_MSG`'s bytes, or `None` when git has not written one.
///
/// # Errors
/// The file exists but could not be read.
pub(crate) fn read(repo: &RepoHandle) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(repo.git_dir().join("MERGE_MSG")) {
        Ok(bytes) => Ok(Some(bytes)),
        // No message is an answer, not a failure: a stash pop never writes
        // one, and git removes it once the commit lands.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The prefix git marks a comment line with: `core.commentChar`, `#` when it
/// is unset. `auto` also reads as `#`: git picks the first character of
/// `#;@!$%^&|:` its own message does not start a line with, and the
/// message it writes for a merge has none of them at a line start, so `#` is
/// what it picks — measured for a merge with `auto` set, git 2.54.0.
///
/// # Errors
/// `git config` will not run.
pub(crate) fn comment_prefix(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("config"),
            GitArg::Flag("--default"),
            GitArg::Flag("#"),
            GitArg::Flag("--get"),
            GitArg::Flag("core.commentChar"),
        ])
        .run(stop)?;
    let value = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    Ok(if value.is_empty() || value == b"auto" {
        b"#".to_vec()
    } else {
        value.to_vec()
    })
}

/// The `Conflicts:` list, in file order; `None` when the message has no such
/// block. A block with no path lines under it is `Some` and empty.
pub(crate) fn conflicts(message: &[u8], prefix: &[u8]) -> Option<Vec<PathBuf>> {
    let mut header = prefix.to_vec();
    header.extend_from_slice(b" Conflicts:");
    let mut item = prefix.to_vec();
    item.push(b'\t');
    let mut lines = message.split(|byte| *byte == b'\n');
    lines.by_ref().find(|line| *line == header.as_slice())?;
    Some(
        lines
            .map_while(|line| line.strip_prefix(item.as_slice()))
            .filter(|path| !path.is_empty())
            .map(|path| path_of(path.to_vec()))
            .collect(),
    )
}

/// The message with every comment line removed and trailing blank lines
/// dropped, ending in one newline — what the summary shows as git's message.
/// An all-comment message is empty.
pub(crate) fn without_comments(message: &[u8], prefix: &[u8]) -> Vec<u8> {
    let mut kept: Vec<&[u8]> = message
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.starts_with(prefix))
        .collect();
    while kept
        .last()
        .is_some_and(|line| line.iter().all(u8::is_ascii_whitespace))
    {
        kept.pop();
    }
    let mut out = kept.join(&b'\n');
    if !out.is_empty() {
        out.push(b'\n');
    }
    out
}

/// What a merge message's first line says was merged in: the quoted name in
/// `Merge branch 'x'`, `Merge remote-tracking branch 'x'`, `Merge tag 'x'` or
/// `Merge commit 'x'`, with ` of <url>` kept whole after it (a pull) and
/// ` into <branch>` dropped (the summary names the branch itself). `None`
/// for a first line git did not write in one of those forms.
pub(crate) fn merged_from(message: &[u8]) -> Option<Vec<u8>> {
    let first = message.split(|byte| *byte == b'\n').next()?;
    let rest = [
        &b"Merge branch '"[..],
        b"Merge remote-tracking branch '",
        b"Merge tag '",
        b"Merge commit '",
    ]
    .iter()
    .find_map(|lead| first.strip_prefix(*lead))?;
    let close = rest.iter().position(|byte| *byte == b'\'')?;
    let (name, tail) = (&rest[..close], &rest[close + 1..]);
    if name.is_empty() {
        return None;
    }
    let mut from = name.to_vec();
    if let Some(remote) = tail.strip_prefix(b" of ") {
        let remote = find(remote, b" into ").map_or(remote, |at| &remote[..at]);
        from.extend_from_slice(b" of ");
        from.extend_from_slice(remote);
    }
    Some(from)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn path_of(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_read_in_file_order() {
        let message = b"Merge branch 'feature'\n\n# Conflicts:\n#\tb.txt\n#\ta c.txt\n";
        assert_eq!(
            conflicts(message, b"#"),
            Some(vec![PathBuf::from("b.txt"), PathBuf::from("a c.txt")])
        );
    }

    #[test]
    fn no_block_is_none_and_another_prefix_is_not_ours() {
        assert_eq!(conflicts(b"Merge branch 'x'\n", b"#"), None);
        assert_eq!(conflicts(b"x\n\n; Conflicts:\n;\ta\n", b"#"), None);
        assert_eq!(
            conflicts(b"x\n\n; Conflicts:\n;\ta\n", b";"),
            Some(vec![PathBuf::from("a")])
        );
    }

    #[test]
    fn the_list_ends_at_the_first_other_line() {
        let message = b"x\n\n# Conflicts:\n#\ta\n#\n# more\n#\tb\n";
        assert_eq!(conflicts(message, b"#"), Some(vec![PathBuf::from("a")]));
    }

    #[test]
    fn comments_go_and_the_tail_is_one_newline() {
        let message = b"Merge branch 'f'\n\n# Conflicts:\n#\ta.txt\n";
        assert_eq!(without_comments(message, b"#"), b"Merge branch 'f'\n");
        assert_eq!(without_comments(b"# only\n", b"#"), b"");
    }

    #[test]
    fn the_merged_name_is_read_from_git_s_forms() {
        let from = |line: &[u8]| merged_from(line).map(|name| String::from_utf8(name).unwrap());
        assert_eq!(from(b"Merge branch 'feature'\n"), Some("feature".into()));
        assert_eq!(
            from(b"Merge branch 'feature' into main"),
            Some("feature".into())
        );
        assert_eq!(
            from(b"Merge branch 'main' of github.com:o/r"),
            Some("main of github.com:o/r".into())
        );
        assert_eq!(
            from(b"Merge branch 'main' of /x/up into topic"),
            Some("main of /x/up".into())
        );
        assert_eq!(
            from(b"Merge remote-tracking branch 'origin/x'"),
            Some("origin/x".into())
        );
        assert_eq!(from(b"Merge tag 'v1'"), Some("v1".into()));
        assert_eq!(from(b"my own message"), None);
    }
}
