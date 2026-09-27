//! The `git blame` command both passes run, built in one place so they blame
//! the same thing (m7-power-tools/18e).

use std::path::Path;

use super::ignore_revs::FoundIgnoreRevs;
use super::{BlameOptions, Rev, path_bytes};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation};

/// Which of blame's two output formats to ask for.
#[derive(Clone, Copy)]
pub(super) enum Format {
    /// `--porcelain`: lines, commits, and (git ≥ 2.50) marks.
    Porcelain,
    /// `-l -s`: one line per final line, marks before the full id (18e).
    Human,
}

/// `git blame` with the options both passes share, so they blame the same
/// thing. The human format's configs are pinned (2.38.5 `builtin/blame.c`):
/// `blame.blankBoundary=false` keeps the id where `-b` would blank it
/// (`emit_other`, line 472); `blame.coloring=none` and `color.ui=never` keep
/// colour codes off the front of the line (lines 449-469, 735-740, 1181).
pub(super) fn blame_invocation(
    repo: &RepoHandle,
    rev: &Rev,
    path: &Path,
    opts: BlameOptions,
    found: Option<&FoundIgnoreRevs>,
    format: Format,
) -> GitInvocation {
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("-c"),
        GitArg::Flag("blame.markIgnoredLines=true"),
        GitArg::Flag("-c"),
        GitArg::Flag("blame.markUnblamableLines=true"),
    ]);
    invocation = match format {
        Format::Porcelain => invocation.args([GitArg::Flag("blame"), GitArg::Flag("--porcelain")]),
        Format::Human => invocation.args([
            GitArg::Flag("-c"),
            GitArg::Flag("blame.blankBoundary=false"),
            GitArg::Flag("-c"),
            GitArg::Flag("blame.coloring=none"),
            GitArg::Flag("-c"),
            GitArg::Flag("color.ui=never"),
            GitArg::Flag("blame"),
            GitArg::Flag("-l"),
            GitArg::Flag("-s"),
        ]),
    };
    invocation = invocation.arg(GitArg::Flag("-M"));
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = match (found, opts.ignore_revs) {
        // Passed even when it could not be read here: git then fails with
        // its own words, rather than this reader quietly blaming without it.
        (Some(found), true) => invocation.arg(GitArg::Glued(
            "--ignore-revs-file=",
            path_bytes(&found.path),
        )),
        // Asked to pass it, and there is none at the root: a configured
        // `blame.ignoreRevsFile` is git's to honour.
        (None, true) => invocation,
        // `--no-ignore-revs-file` drops the configured `blame.ignoreRevsFile`
        // before git opens it. An empty `--ignore-revs-file=` does not: git
        // still opens a configured file first, and a missing one is fatal
        // ("could not open object name list"), probed on git 2.54 (18d).
        (_, false) => invocation.arg(GitArg::Flag("--no-ignore-revs-file")),
    };
    if let Rev::Commit(commit) = rev {
        invocation = invocation.arg(GitArg::Oid(*commit));
    }
    invocation
        .arg(GitArg::Separator)
        .arg(GitArg::Path(path_bytes(path)))
}
