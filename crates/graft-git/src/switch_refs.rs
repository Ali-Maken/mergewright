//! What the branch picker lists: every local and remote-tracking branch with
//! its tip, subject, author, date and upstream, plus `@{-1}` — the branch git
//! itself would switch back to (m7-power-tools/10a).
//!
//! Two commands, both through [`GitInvocation`]:
//! - one `for-each-ref` over `refs/heads` and `refs/remotes`, NUL-separated
//!   fields, one record per line (no field can hold a newline: ref names
//!   refuse control characters, `%(subject)` is one line);
//! - `rev-parse --verify --quiet --symbolic-full-name @{-1}`, measured on git
//!   2.54: exit 1 and no output with no previous checkout or a deleted
//!   previous branch; exit 0 and no output when the previous was detached;
//!   exit 0 and `refs/heads/<x>` otherwise.

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

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
    let previous = previous_branch(repo, stop)?;
    Ok(SwitchRefs { refs, previous })
}

fn previous_branch(repo: &RepoHandle, stop: &dyn Stop) -> Result<Option<RefName>, RunError> {
    let invocation = GitInvocation::new(repo).args([
        GitArg::Flag("rev-parse"),
        GitArg::Flag("--verify"),
        GitArg::Flag("--quiet"),
        GitArg::Flag("--symbolic-full-name"),
        GitArg::Word("@{-1}".to_owned()),
    ]);
    let raw = invocation.run_raw(stop)?;
    match raw.code {
        // No previous checkout, or its branch is gone: `--quiet` says so by
        // exit 1 alone.
        Some(1) if raw.output.stdout.is_empty() => Ok(None),
        Some(0) => {
            let name = trim_newline(&raw.output.stdout);
            if name.is_empty() {
                return Ok(None); // the previous checkout was detached
            }
            RefName::new(name)
                .map(Some)
                .map_err(|why| RunError::Unparsable(format!("@{{-1}}: {why}")))
        }
        Some(status) => Err(RunError::Exited {
            status,
            stderr_first_line: String::from_utf8_lossy(&raw.output.stderr)
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or_default()
                .to_owned(),
            command: invocation.command_line(),
        }),
        None => Err(RunError::Signalled {
            command: invocation.command_line(),
        }),
    }
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
