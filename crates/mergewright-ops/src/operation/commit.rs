//! Commit and amend.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mergewright_core::journal::HeadRef;
use mergewright_git::run::GitArg;
use mergewright_git::status::WorkingTreeStatus;

use super::stage::with_paths;
use super::{Chosen, CommitParams, Include, Invocation, Part, Refusal, RepoFacts};

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
    match &params.include {
        Include::Staged if status.staged().next().is_none() && !params.allow_empty => {
            return Err(Refusal::NothingStaged);
        }
        Include::Chosen(files) if files.is_empty() && !params.allow_empty => {
            return Err(Refusal::NothingChosen);
        }
        Include::Staged | Include::Chosen(_) => {}
    }
    check_subject(params)
}

/// An amend with nothing staged is a reword, so only the subject is checked;
/// an amend of chosen files needs one chosen.
pub(super) fn check_amend(params: &CommitParams, head: &HeadRef) -> Result<(), Refusal> {
    if matches!(head, HeadRef::Unborn(_)) {
        return Err(Refusal::UnbornHead);
    }
    if let Include::Chosen(files) = &params.include
        && files.is_empty()
        && !params.allow_empty
    {
        return Err(Refusal::NothingChosen);
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

/// The commit's lines. [`Include::Staged`], and a choice that is exactly
/// what is staged, run today's one line on the real index, hooks and all.
/// Any other choice is built on a scratch copy of the index (ease-pass/04):
///
/// 1. scratch: the staged entries not chosen go back to `HEAD` (`rm --cached`
///    on an unborn branch), a rename by both its paths;
/// 2. scratch: `add -A` the files chosen whole — modified, deleted and new
///    alike; a staged part stays as the copy has it;
/// 3. scratch: the commit line; its hooks see the scratch index, as with
///    `git commit <paths>`;
/// 4. real: the chosen paths reset to the new `HEAD`, every other entry
///    untouched.
///
/// A hook that fails stops the plan at 3, before `HEAD` or the real index
/// moved. Every path goes through `--literal-pathspecs`.
pub(super) fn plan(params: &CommitParams, amend: bool, facts: &RepoFacts) -> Vec<Invocation> {
    let commit = commit_line(params, amend);
    let files = match &params.include {
        Include::Chosen(files) if !as_staged(files, &facts.status) => files,
        Include::Staged | Include::Chosen(_) => return vec![commit],
    };
    let chosen: BTreeSet<&Path> = files.iter().map(|file| file.path.as_path()).collect();
    let held: Vec<PathBuf> = facts
        .status
        .staged()
        .filter(|entry| !chosen.contains(entry.path.as_path()))
        .flat_map(|entry| std::iter::once(&entry.path).chain(entry.original_path.as_ref()))
        .cloned()
        .collect();
    let whole: Vec<PathBuf> = files
        .iter()
        .filter(|file| file.part == Part::Whole)
        .map(|file| file.path.clone())
        .collect();
    let chosen: Vec<PathBuf> = chosen.into_iter().map(Path::to_path_buf).collect();

    let mut plan = Vec::new();
    if !held.is_empty() {
        let unstage = if matches!(facts.head, HeadRef::Unborn(_)) {
            // `-f`: a staged new file edited since would make `rm --cached`
            // refuse, and this index is a copy.
            vec![
                GitArg::Flag("rm"),
                GitArg::Flag("--cached"),
                GitArg::Flag("-f"),
                GitArg::Flag("-q"),
            ]
        } else {
            vec![GitArg::Flag("reset"), GitArg::Flag("-q")]
        };
        plan.push(Invocation::new(with_paths(unstage, &held)).on_scratch());
    }
    if !whole.is_empty() {
        let add = vec![GitArg::Flag("add"), GitArg::Flag("-A")];
        plan.push(Invocation::new(with_paths(add, &whole)).on_scratch());
    }
    plan.push(commit.on_scratch());
    if !chosen.is_empty() {
        let reset = vec![GitArg::Flag("reset"), GitArg::Flag("-q")];
        plan.push(Invocation::new(with_paths(reset, &chosen)));
    }
    plan
}

/// The choice is exactly what is staged: every staged path, each as staged,
/// nothing else.
fn as_staged(files: &[Chosen], status: &WorkingTreeStatus) -> bool {
    let staged: BTreeSet<&Path> = status.staged().map(|entry| entry.path.as_path()).collect();
    let chosen: BTreeSet<&Path> = files.iter().map(|file| file.path.as_path()).collect();
    files.iter().all(|file| file.part == Part::StagedPart) && chosen == staged
}

/// `commit -F -` with the message on stdin, never `-m`: a message is data,
/// and `-F -` takes it byte for byte, however many lines it has.
fn commit_line(params: &CommitParams, amend: bool) -> Invocation {
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
