//! Submodule reads (m7-power-tools/13a): the commits a submodule pointer
//! move pulled in or rewound, as `git diff --submodule=log` lists them.

use std::path::Path;

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;

/// The commits a submodule pointer move pulled in or rewound, newest first
/// on each side, in git's own order. Every commit: the diff pane's cap is
/// the caller's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubmoduleLog {
    /// Reachable from `to`, not from `from`: `+` rows.
    pub pulled_in: Vec<(Oid, String)>,
    /// Reachable from `from`, not from `to`: `−` rows (a pointer moved back
    /// or sideways).
    pub rewound: Vec<(Oid, String)>,
}

/// The commits between the submodule at `path` (relative to `repo`'s
/// working tree) moving from `from` to `to`.
///
/// The same walk `git diff --submodule=log` does (git's
/// `prepare_submodule_diff_summary`: `from...to`, left-right, first parent
/// only), run in the submodule's own repository so it takes the two ids
/// directly, not two superproject trees that record them — and keeps each
/// commit's id, which the summary drops. The oracle test
/// `submodule_log_lists_pulled_in_and_rewound` holds it to git's summary.
///
/// The submodule's repository is named by `--git-dir=<path>/.git`, never
/// found by walking up: a submodule not set up has no `.git`, and a walk
/// would land in the superproject and read its history instead.
///
/// # Errors
/// - [`FactsError::Stopped`] when `stop` asks.
/// - [`FactsError::History`] with git's own exit for everything else —
///   measured (13a's NOTES): a submodule not set up and a commit the
///   submodule does not hold both exit 128.
pub fn submodule_log(
    repo: &RepoHandle,
    path: &Path,
    from: Oid,
    to: Oid,
    stop: &dyn Stop,
) -> Result<SubmoduleLog, FactsError> {
    let top = repo.workdir().unwrap_or_else(|| repo.git_dir());
    let git_dir = top.join(path).join(".git");
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Glued(
                "--git-dir=",
                git_dir.as_os_str().as_encoded_bytes().to_vec(),
            ),
            GitArg::Flag("log"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-show-signature"),
            GitArg::Flag("--no-decorate"),
            GitArg::Flag("--left-right"),
            GitArg::Flag("--first-parent"),
            GitArg::Flag("-z"),
            GitArg::Word("--format=%m%H %s".to_owned()),
            GitArg::Word(format!("{from}...{to}")),
            GitArg::Separator,
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    if output.truncated {
        return Err(unparsable("the submodule's log passed the output cap"));
    }
    parse(&output.stdout)
}

/// `<m><id> <subject>` records, NUL-separated; `<m>` is `<` (only on
/// `from`) or `>` (only on `to`).
fn parse(stdout: &[u8]) -> Result<SubmoduleLog, FactsError> {
    let mut log = SubmoduleLog::default();
    for record in stdout.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        let shown = || String::from_utf8_lossy(record).into_owned();
        let (side, rest) = record
            .split_first()
            .ok_or_else(|| unparsable(&format!("an empty log record: {:?}", shown())))?;
        let space = rest
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or_else(|| unparsable(&format!("a log record with no subject: {:?}", shown())))?;
        let (hex, subject) = rest.split_at(space);
        let id = std::str::from_utf8(hex)
            .ok()
            .and_then(|hex| Oid::from_hex(hex).ok())
            .ok_or_else(|| unparsable(&format!("a log record with no id: {:?}", shown())))?;
        // A subject is shown, never parsed or written back: a byte that is
        // not UTF-8 (a commit in a legacy encoding git could not re-encode)
        // becomes U+FFFD rather than hiding the commit.
        let subject = String::from_utf8_lossy(&subject[1..]).into_owned();
        match side {
            b'>' => log.pulled_in.push((id, subject)),
            b'<' => log.rewound.push((id, subject)),
            _ => {
                return Err(unparsable(&format!(
                    "a log record on no side: {:?}",
                    shown()
                )));
            }
        }
    }
    Ok(log)
}

fn unparsable(what: &str) -> FactsError {
    FactsError::History(RunError::Unparsable(what.to_owned()))
}
