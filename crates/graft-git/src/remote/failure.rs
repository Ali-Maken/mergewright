//! Telling a failed fetch, pull or push apart, from the whole of what git
//! wrote to stderr.
//!
//! [`crate::run::RunError`] keeps only git's first stderr line, and the line
//! that matters often comes later: a push prints `To <url>` first and its
//! `[rejected] … (stale info)` after. So [`classify_failure`] reads every line.
//! It is plain English string matching, stable because the runner sets
//! `LC_ALL=C`. Anything it does not recognise is [`RemoteFailure::Other`] with
//! git's own line, verbatim — never hidden, never given a name it has not
//! earned.

use super::parse_progress;

/// Why a remote operation failed, as far as git's stderr says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteFailure {
    /// The remote wanted credentials and did not accept (or never got) them.
    /// `target` is the URL or host git named; empty when it named none.
    AuthFailed {
        /// The URL or host.
        target: String,
    },
    /// A push the remote refused because the remote branch has commits the
    /// local one lacks (`non-fast-forward`, `fetch first`).
    NonFastForward,
    /// A `--force-with-lease` push refused because the remote branch moved
    /// since the lease was taken.
    StaleInfo,
    /// The remote's own hooks or policy refused the push.
    RemoteRejected {
        /// git's reason, verbatim: `pre-receive hook declined`.
        reason: String,
    },
    /// No remote by that name, or the URL is not a repository.
    NoSuchRemote {
        /// The name or URL git quoted.
        name: String,
    },
    /// The remote could not be reached at all.
    Unreachable {
        /// The host or URL git named; empty when it named none.
        target: String,
    },
    /// A `merge --ff-only` (the pull) found the branches diverged.
    NotFastForward,
    /// A pull would overwrite uncommitted (or untracked) files.
    WouldOverwrite {
        /// The paths git listed, as it printed them.
        paths: Vec<String>,
    },
    /// The user stopped it. Never produced by [`classify_failure`]: the
    /// caller sets it from [`crate::run::RunError::Stopped`].
    Cancelled,
    /// Nothing above; git's most telling line, verbatim.
    Other {
        /// The first `fatal:`/`error:` line, else the first non-empty line
        /// that is not progress.
        first_line: String,
    },
}

/// One recogniser: does this line say it, and what does it say?
type Rule = fn(&str, &[&str]) -> Option<RemoteFailure>;

/// Checked in order, each against every line; the first rule that matches
/// any line wins.
const RULES: [Rule; 8] = [
    auth_failed,
    stale_info,
    non_fast_forward,
    remote_rejected,
    no_such_remote,
    unreachable,
    not_fast_forward,
    would_overwrite,
];

/// Classifies a failed remote command from its full stderr.
#[must_use]
pub fn classify_failure(stderr: &[u8]) -> RemoteFailure {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text
        .split(['\n', '\r'])
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect();
    for rule in RULES {
        for line in &lines {
            if let Some(failure) = rule(line, &lines) {
                return failure;
            }
        }
    }
    RemoteFailure::Other {
        first_line: first_line(&lines),
    }
}

fn first_line(lines: &[&str]) -> String {
    let telling = lines
        .iter()
        .find(|line| line.starts_with("fatal:") || line.starts_with("error:"));
    let first = || {
        lines
            .iter()
            .find(|line| parse_progress(line.as_bytes()).is_none())
    };
    telling
        .or_else(first)
        .map_or_else(String::new, |line| (*line).to_owned())
}

/// The text between the first `'` after `marker` and the next `'`.
fn quoted_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let rest = &line[line.find(marker)? + marker.len()..];
    let rest = &rest[rest.find('\'')? + 1..];
    Some(&rest[..rest.find('\'')?])
}

/// The URL git quoted in any `unable to access '<url>'` or `Authentication
/// failed for '<url>'` line. Empty when git named none: an absent name, not a
/// failure to read one.
fn named_url(lines: &[&str]) -> String {
    lines
        .iter()
        .find_map(|line| {
            quoted_after(line, "unable to access")
                .or_else(|| quoted_after(line, "Authentication failed for"))
        })
        .unwrap_or_default()
        .to_owned()
}

fn auth_failed(line: &str, lines: &[&str]) -> Option<RemoteFailure> {
    let target = if let Some(url) = [
        "Authentication failed for",
        "could not read Username for",
        "could not read Password for",
    ]
    .iter()
    .find_map(|marker| quoted_after(line, marker))
    {
        url.to_owned()
    } else if line.contains("Invalid username or password") {
        named_url(lines)
    } else if let Some(at) = line.find(": Permission denied (publickey") {
        // `git@github.com: Permission denied (publickey).`
        line[..at].to_owned()
    } else if line.contains("Host key verification failed") {
        String::new()
    } else {
        return None;
    };
    Some(RemoteFailure::AuthFailed { target })
}

fn stale_info(line: &str, _: &[&str]) -> Option<RemoteFailure> {
    (line.contains("[rejected]") && line.contains("(stale info)"))
        .then_some(RemoteFailure::StaleInfo)
}

fn non_fast_forward(line: &str, _: &[&str]) -> Option<RemoteFailure> {
    (line.contains("[rejected]")
        && (line.contains("(non-fast-forward)") || line.contains("(fetch first)")))
    .then_some(RemoteFailure::NonFastForward)
}

fn remote_rejected(line: &str, _: &[&str]) -> Option<RemoteFailure> {
    if line.contains("[remote rejected]") {
        // ` ! [remote rejected] main -> main (pre-receive hook declined)`
        let reason = line
            .rfind('(')
            .and_then(|open| line[open + 1..].strip_suffix(')'))
            .unwrap_or(line);
        return Some(RemoteFailure::RemoteRejected {
            reason: reason.to_owned(),
        });
    }
    line.contains("pre-receive hook declined")
        .then(|| RemoteFailure::RemoteRejected {
            reason: "pre-receive hook declined".to_owned(),
        })
}

fn no_such_remote(line: &str, _: &[&str]) -> Option<RemoteFailure> {
    // `fatal: 'nope' does not appear to be a git repository`
    if let Some(end) = line.find("' does not appear to be a git repository") {
        let start = line[..end].rfind('\'')? + 1;
        return Some(RemoteFailure::NoSuchRemote {
            name: line[start..end].to_owned(),
        });
    }
    // `fatal: repository 'http://h/x.git/' not found` — an HTTP 404.
    if line.ends_with("' not found")
        && let Some(url) = quoted_after(line, "repository ")
    {
        return Some(RemoteFailure::NoSuchRemote {
            name: url.to_owned(),
        });
    }
    // `error: No such remote: 'nope'` (git 2.54 `remote`), `No such remote 'x'`.
    let name = quoted_after(line, "No such remote")?;
    Some(RemoteFailure::NoSuchRemote {
        name: name.to_owned(),
    })
}

fn unreachable(line: &str, lines: &[&str]) -> Option<RemoteFailure> {
    let after = |marker: &str| line.find(marker).map(|at| &line[at + marker.len()..]);
    let target = if let Some(host) = after("Could not resolve host: ") {
        host.to_owned()
    } else if let Some(rest) =
        after("ssh: connect to host ").or_else(|| after("Failed to connect to "))
    {
        // `ssh: connect to host h port 22: …`, `Failed to connect to h port 80 after …`
        rest.split(" port ").next().unwrap_or(rest).to_owned()
    } else if [
        "Connection refused",
        "Connection timed out",
        "Network is unreachable",
        "Operation timed out",
        "Could not resolve proxy",
    ]
    .iter()
    .any(|phrase| line.contains(phrase))
    {
        named_url(lines)
    } else {
        return None;
    };
    Some(RemoteFailure::Unreachable { target })
}

fn not_fast_forward(line: &str, _: &[&str]) -> Option<RemoteFailure> {
    line.contains("Not possible to fast-forward, aborting")
        .then_some(RemoteFailure::NotFastForward)
}

fn would_overwrite(line: &str, lines: &[&str]) -> Option<RemoteFailure> {
    // Tracked (`Your local changes to the following files …`) and untracked
    // (`The following untracked working tree files …`): both stop the pull.
    let heading = line.contains("Your local changes to the following files would be overwritten")
        || line.contains("untracked working tree files would be overwritten");
    if !heading {
        return None;
    }
    let paths = lines
        .iter()
        .skip_while(|other| !std::ptr::eq(**other, line))
        .skip(1)
        .map_while(|path| path.strip_prefix('\t'))
        .map(str::to_owned)
        .collect();
    Some(RemoteFailure::WouldOverwrite { paths })
}

#[cfg(test)]
mod tests;
