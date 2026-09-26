//! A reflog record in plain words (note J1's table). git's own subject sits
//! beside them on the row, so the words never need to carry all of it.

use graft_core::oid::Oid;
use graft_git::reflog::ReflogRecord;

use super::steps::{branch_short, checkout_names, is_hex_id};
use crate::operation::short;

/// What a record says happened, in plain words.
///
/// `<b>` is the record's branch, short; a `HEAD` record says `HEAD` here —
/// the list's rows name the branch `HEAD` was on where git's logs say
/// (`build` passes it). `back` is [`super::OutsideRecord::back`].
///
/// | git's subject | Words |
/// |---|---|
/// | `commit: <s>` | `committed "<s>" on <b>` |
/// | `commit (amend): <s>` | `amended "<s>" on <b>` |
/// | `reset: moving to …` | `reset <b> <N> commits back`, else `moved <b> to <sha7>` |
/// | `checkout: moving from a to b` | `switch from a to b` |
/// | `merge …: Fast-forward`, `pull: Fast-forward` | `fast-forwarded <b> <N> commits` |
/// | `branch: Created from …` | `made <b> at <sha7>` |
/// | `rebase (finish): …` | `rebased <b> onto <upstream, else sha7>` |
/// | `cherry-pick: <s>` | `took "<s>" onto <b>` |
/// | anything else | git's subject, unchanged |
pub fn words(record: &ReflogRecord, back: Option<usize>) -> String {
    let branch = branch_short(&record.refname).unwrap_or(&record.refname);
    words_on(record, back, branch, None)
}

/// [`words`] with the branch and a rebase's upstream the list worked out.
pub(super) fn words_on(
    record: &ReflogRecord,
    back: Option<usize>,
    branch: &str,
    onto: Option<&str>,
) -> String {
    let subject = record.subject.as_str();
    if let Some(said) = subject.strip_prefix("commit: ") {
        return format!("committed \"{said}\" on {branch}");
    }
    if let Some(said) = subject.strip_prefix("commit (amend): ") {
        return format!("amended \"{said}\" on {branch}");
    }
    if let Some(said) = subject.strip_prefix("cherry-pick: ") {
        return format!("took \"{said}\" onto {branch}");
    }
    if is_reset(subject) {
        return match back {
            Some(count) => format!("reset {branch} {} back", commits(count)),
            None => format!("moved {branch} to {}", sha7(record.new)),
        };
    }
    if let Some((from, to)) = checkout_names(subject) {
        return format!("switch from {} to {}", name_or_sha7(from), name_or_sha7(to));
    }
    if is_fast_forward(subject) {
        return match back {
            Some(count) => format!("fast-forwarded {branch} {}", commits(count)),
            None => format!("fast-forwarded {branch} to {}", sha7(record.new)),
        };
    }
    if subject.starts_with("branch: Created from ") {
        return format!("made {branch} at {}", sha7(record.new));
    }
    if let Some(words) = rebase_words(subject, onto) {
        return words;
    }
    subject.to_owned()
}

/// `rebase (finish): refs/heads/<b> onto <sha>` (the branch's record) or
/// `rebase (finish): returning to refs/heads/<b>` (`HEAD`'s).
fn rebase_words(subject: &str, onto: Option<&str>) -> Option<String> {
    let rest = subject
        .strip_prefix("rebase (finish): ")
        .or_else(|| subject.strip_prefix("rebase -i (finish): "))?;
    let (full, said_onto) = match rest.strip_prefix("returning to ") {
        Some(full) => (full, None),
        None => match rest.split_once(" onto ") {
            Some((full, upstream)) => (full, Some(upstream)),
            None => (rest, None),
        },
    };
    let branch = branch_short(full).unwrap_or(full);
    // The upstream's name, when the start record gave one, reads better than
    // the id the finish record carries.
    let upstream = onto
        .filter(|name| !is_hex_id(name))
        .or(said_onto)
        .map(name_or_sha7);
    Some(match upstream {
        Some(upstream) => format!("rebased {branch} onto {upstream}"),
        None => format!("rebased {branch}"),
    })
}

pub(super) fn is_reset(subject: &str) -> bool {
    subject.starts_with("reset: moving to ")
}

pub(super) fn is_fast_forward(subject: &str) -> bool {
    subject.ends_with(": Fast-forward")
        && (subject.starts_with("merge ") || subject.starts_with("pull"))
}

/// `1 commit`, `3 commits`.
fn commits(count: usize) -> String {
    if count == 1 {
        "1 commit".to_owned()
    } else {
        format!("{count} commits")
    }
}

fn sha7(oid: Option<Oid>) -> String {
    oid.map_or_else(|| "nothing".to_owned(), short)
}

/// A name as typed, or a full hex id cut to seven.
fn name_or_sha7(name: &str) -> String {
    if is_hex_id(name) {
        name.chars().take(7).collect()
    } else {
        name.to_owned()
    }
}

/// The shell command a step's subject names, for a put-back's `because`:
/// `reset`, `commit --amend`, `switch`, `rebase`.
pub(super) fn shell_command(subject: &str) -> String {
    if subject.is_empty() {
        return "update-ref".to_owned();
    }
    let head = subject.split(':').next().unwrap_or(subject);
    let command = if head == "commit (amend)" {
        "commit --amend"
    } else if head.starts_with("commit") {
        "commit"
    } else if head.starts_with("checkout") {
        "switch"
    } else if head.starts_with("rebase") {
        "rebase"
    } else if head.starts_with("merge") {
        "merge"
    } else if head.starts_with("pull") {
        "pull"
    } else {
        head.split(' ').next().unwrap_or(head)
    };
    command.to_owned()
}
