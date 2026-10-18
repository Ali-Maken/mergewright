//! What the branch picker lists: every local and remote-tracking branch with
//! its tip, subject, author, date and upstream, plus `@{-1}` — the branch git
//! itself would switch back to (m7-power-tools/10a).
//!
//! Up to three commands, all through [`GitInvocation`]:
//! - one `for-each-ref` over `refs/heads` and `refs/remotes`, NUL-separated
//!   fields, one record per line (no field can hold a newline: ref names
//!   refuse control characters, `%(subject)` is one line);
//! - `rev-parse --verify --quiet @{-1}`: exit 1 and no output with no
//!   previous checkout or a deleted previous branch, exit 0 when there is
//!   one (measured on 2.38.5 and 2.54);
//! - then `check-ref-format --branch @{-1}`, the name `git switch -` uses: a
//!   branch name, or a commit id when the previous checkout was detached.
//!   It counts only when `refs/heads/<name>` is in the listing.

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RawOutput, RunError, Stop};

/// One branch as the picker shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefListing {
    /// Full: `refs/heads/x` or `refs/remotes/origin/x`.
    pub name: RefName,
    /// The commit the branch points at.
    pub tip: Oid,
    /// The tip's subject line, lossily decoded (display only).
    pub subject: String,
    /// The tip's author name, lossily decoded (display only).
    pub author: String,
    /// The tip's committer date, unix seconds.
    pub when: i64,
    /// `%(upstream)`, full; `None` when unset. Only local rows have one.
    pub upstream: Option<RefName>,
}

/// Every branch, and the one before the last switch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchRefs {
    /// Local then remote-tracking branches, in git's ref order; symrefs
    /// (`origin/HEAD`) left out.
    pub refs: Vec<RefListing>,
    /// `@{-1}` as a branch; `None` when there is none, it was detached, or it
    /// has since been deleted.
    pub previous: Option<RefName>,
}

const FORMAT: &str = "--format=%(refname)%00%(objectname)%00%(symref)%00%(upstream)%00\
%(subject)%00%(authorname)%00%(committerdate:unix)";

/// Reads [`SwitchRefs`].
///
/// # Errors
/// [`RunError`] from the runner (git missing, stopped, failing), or
/// `Unparsable` for a record that does not have the asked-for shape.
pub fn switch_refs(repo: &RepoHandle, stop: &dyn Stop) -> Result<SwitchRefs, RunError> {
    let listed = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag(FORMAT),
            GitArg::Ref(b"refs/heads".to_vec()),
            GitArg::Ref(b"refs/remotes".to_vec()),
        ])
        .run(stop)?;
    let refs = parse_listing(&listed.stdout)?;
    let previous = previous_branch(repo, &refs, stop)?;
    Ok(SwitchRefs { refs, previous })
}

/// `@{-1}` as a listed local branch. `rev-parse` says whether there is a
/// previous checkout at all; its name is then `check-ref-format --branch`'s,
/// which is how `git switch -` reads it. `rev-parse --symbolic-full-name`
/// cannot give the name: it prints nothing when a tag shares it, and names
/// the tag when only the tag is left (both seen on 2.38.5 and 2.54).
fn previous_branch(
    repo: &RepoHandle,
    refs: &[RefListing],
    stop: &dyn Stop,
) -> Result<Option<RefName>, RunError> {
    let verify = GitInvocation::new(repo).args([
        GitArg::Flag("rev-parse"),
        GitArg::Flag("--verify"),
        GitArg::Flag("--quiet"),
        GitArg::Word("@{-1}".to_owned()),
    ]);
    if !has_previous(&verify.run_raw(stop)?, &verify.command_line())? {
        return Ok(None);
    }
    let named = GitInvocation::new(repo)
        .args([
            GitArg::Flag("check-ref-format"),
            GitArg::Flag("--branch"),
            GitArg::Word("@{-1}".to_owned()),
        ])
        .run(stop)?;
    Ok(listed_branch(trim_newline(&named.stdout), refs))
}

/// Whether `rev-parse --verify --quiet @{-1}` found a previous checkout.
/// Exit 1 with no output is `--quiet`'s "none" (no previous checkout, or its
/// branch deleted); exit 0 is "there is one" (a branch, a detached commit,
/// or a name a tag shares); any other exit is git failing.
fn has_previous(raw: &RawOutput, command: &str) -> Result<bool, RunError> {
    match raw.code {
        Some(1) if raw.output.stdout.is_empty() => Ok(false),
        Some(0) => Ok(true),
        Some(status) => Err(RunError::Exited {
            status,
            stderr_first_line: crate::commit_graph::first_line(&raw.output.stderr),
            command: command.to_owned(),
        }),
        // `run_raw` gives no code only when it killed git at the output cap.
        None => Err(RunError::Unparsable(format!(
            "{command}: output passed the runner's cap"
        ))),
    }
}

/// `refs/heads/<name>` when the listing has it: a detached previous checkout
/// names a commit id, and a deleted branch is not listed.
fn listed_branch(name: &[u8], refs: &[RefListing]) -> Option<RefName> {
    let mut full = b"refs/heads/".to_vec();
    full.extend_from_slice(name);
    refs.iter()
        .find(|listing| listing.name.as_bytes() == full.as_slice())
        .map(|listing| listing.name.clone())
}

fn trim_newline(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\n").unwrap_or(bytes)
}

fn parse_listing(stdout: &[u8]) -> Result<Vec<RefListing>, RunError> {
    let mut refs = Vec::new();
    for record in stdout.split(|&b| b == b'\n') {
        if record.is_empty() {
            continue; // the empty tail after the last newline
        }
        let fields: Vec<&[u8]> = record.split(|&b| b == 0).collect();
        let [name, oid, symref, upstream, subject, author, when] = fields[..] else {
            return Err(unparsable(record, "not seven fields"));
        };
        if !symref.is_empty() {
            continue;
        }
        let name = RefName::new(name).map_err(|_| unparsable(record, "ref name"))?;
        let tip = std::str::from_utf8(oid)
            .ok()
            .and_then(|hex| Oid::from_hex(hex).ok())
            .ok_or_else(|| unparsable(record, "object id"))?;
        let upstream = if upstream.is_empty() {
            None
        } else {
            Some(RefName::new(upstream).map_err(|_| unparsable(record, "upstream"))?)
        };
        let when = std::str::from_utf8(when)
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| unparsable(record, "date"))?;
        refs.push(RefListing {
            name,
            tip,
            subject: String::from_utf8_lossy(subject).into_owned(),
            author: String::from_utf8_lossy(author).into_owned(),
            when,
            upstream,
        });
    }
    Ok(refs)
}

fn unparsable(record: &[u8], what: &str) -> RunError {
    RunError::Unparsable(format!(
        "for-each-ref record ({what}): {:?}",
        String::from_utf8_lossy(record)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::GitOutput;

    fn exited(code: i32, stdout: &[u8], stderr: &[u8]) -> RawOutput {
        RawOutput {
            output: GitOutput {
                stdout: stdout.to_vec(),
                stderr: stderr.to_vec(),
                truncated: false,
            },
            code: Some(code),
        }
    }

    /// Review 18/19/27 Minor 2 (mutation M1): only `--quiet`'s exit 1 means
    /// "no previous"; git failing any other way is an error, not "none".
    #[test]
    fn a_git_failure_reading_the_previous_checkout_is_an_error() {
        let failed = exited(128, b"", b"fatal: not a git repository\n");
        match has_previous(&failed, "git rev-parse") {
            Err(RunError::Exited {
                status,
                stderr_first_line,
                ..
            }) => {
                assert_eq!(status, 128);
                assert_eq!(stderr_first_line, "fatal: not a git repository");
            }
            other => panic!("a failure must not read as no previous: {other:?}"),
        }
        assert!(!has_previous(&exited(1, b"", b""), "c").expect("none"));
        assert!(has_previous(&exited(0, b"", b""), "c").expect("one"));
    }

    /// Review 18/19/27 Minor 2 (mutation M4): a record without the seven
    /// asked-for fields is an error, not skipped.
    #[test]
    fn a_record_of_six_fields_is_an_error() {
        let six =
            b"refs/heads/a\x00e83c5163316f89bfbde7d9ab23ca2e25604af290\x00\x00\x00subject\x00Ann\n";
        match parse_listing(six) {
            Err(RunError::Unparsable(why)) => assert!(why.contains("seven"), "{why}"),
            other => panic!("a six-field record must be refused: {other:?}"),
        }
    }
}
