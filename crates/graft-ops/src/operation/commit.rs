//! Commit and amend.

use graft_core::journal::HeadRef;
use graft_git::run::GitArg;
use graft_git::status::WorkingTreeStatus;

use super::{CommitParams, Invocation, Refusal};

/// The first non-blank line, trailing whitespace trimmed; empty when the
/// message has none.
pub(super) fn subject(message: &str) -> &str {
    message
        .lines()
        .map(str::trim_end)
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

pub(super) fn check_commit(
    params: &CommitParams,
    status: &WorkingTreeStatus,
) -> Result<(), Refusal> {
    if status.staged().next().is_none() && !params.allow_empty {
        return Err(Refusal::NothingStaged);
    }
    check_subject(params)
}

/// An amend with nothing staged is a reword, so only the subject is checked.
pub(super) fn check_amend(params: &CommitParams, head: &HeadRef) -> Result<(), Refusal> {
    if matches!(head, HeadRef::Unborn(_)) {
        return Err(Refusal::UnbornHead);
    }
    check_subject(params)
}

fn check_subject(params: &CommitParams) -> Result<(), Refusal> {
    if subject(&params.message).is_empty() {
        Err(Refusal::EmptySubject)
    } else {
        Ok(())
    }
}

/// `commit -F -` with the message on stdin, never `-m`: a message is data,
/// and `-F -` takes it byte for byte, however many lines it has.
pub(super) fn plan(params: &CommitParams, amend: bool) -> Invocation {
    let mut args = vec![
        GitArg::Flag("commit"),
        GitArg::Flag("-F"),
        GitArg::Word("-".to_owned()),
    ];
    if amend {
        args.push(GitArg::Flag("--amend"));
    }
    if params.sign_off {
        args.push(GitArg::Flag("--signoff"));
    }
    if params.no_verify {
        args.push(GitArg::Flag("--no-verify"));
    }
    if params.allow_empty {
        args.push(GitArg::Flag("--allow-empty"));
    }
    Invocation::with_stdin(args, message_bytes(&params.message))
}

/// `\n` line ends and exactly one trailing newline.
fn message_bytes(message: &str) -> Vec<u8> {
    let mut text = message.replace("\r\n", "\n");
    let end = text.trim_end_matches('\n').len();
    text.truncate(end);
    text.push('\n');
    text.into_bytes()
}

/// The ref a commit moves: the branch by full name, or `HEAD` itself when
/// detached. An unborn branch is named too — the commit creates it.
pub(super) fn head_ref_name(head: &HeadRef) -> String {
    match head {
        HeadRef::Branch(name) | HeadRef::Unborn(name) => name.clone(),
        HeadRef::Detached(_) => "HEAD".to_owned(),
    }
}
