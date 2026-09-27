//! Crate-private object plumbing: resolving a revision to an id (absorb and
//! the set-aside rebuild), and writing a commit object with no ref (the
//! rebuild only; absorb writes its commits with `-F -`) (m7/06d). Objects
//! only; no ref, index or file is written.

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

/// `git rev-parse --verify <rev>`, as an id.
pub(crate) fn resolve(repo: &RepoHandle, rev: &str, stop: &dyn Stop) -> Result<Oid, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-parse"),
            GitArg::Flag("--verify"),
            GitArg::Word(rev.to_owned()),
        ])
        .run(stop)?;
    printed_oid(&output.stdout, &format!("rev-parse {rev}"))
}

/// `git commit-tree <tree> -p … -m <message>`, not yet run: the caller
/// records its command line when it keeps one, then runs it and reads the
/// id with [`printed_oid`]. git's own identity config applies.
pub(crate) fn commit_tree(
    repo: &RepoHandle,
    tree: Oid,
    parents: &[Oid],
    message: &str,
) -> GitInvocation {
    let mut args = vec![GitArg::Flag("commit-tree"), GitArg::Oid(tree)];
    for parent in parents {
        args.push(GitArg::Flag("-p"));
        args.push(GitArg::Oid(*parent));
    }
    args.push(GitArg::Flag("-m"));
    args.push(GitArg::Word(message.to_owned()));
    GitInvocation::new(repo).args(args)
}

/// The one id git printed; `what` names the command in the error.
pub(crate) fn printed_oid(stdout: &[u8], what: &str) -> Result<Oid, RunError> {
    let printed = String::from_utf8_lossy(stdout);
    Oid::from_hex(printed.trim())
        .map_err(|_| RunError::Unparsable(format!("{what} printed {printed:?}")))
}
