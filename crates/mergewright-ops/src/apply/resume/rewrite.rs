//! What a resume does differently for a rewrite (ADR-0047): the entry is
//! found by its saved plan, git's messages come from the plan's helper, and
//! an `edit` row's stop is a stop, not a finish.

use std::path::Path;

use mergewright_core::journal::{HeadRef, JournalEntry};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation};
use mergewright_git::state::InProgress;

use crate::facts::FactsError;
use crate::rewrite::saved::{self, CLEANUP};

use super::super::engine::plan_of;
use super::super::{ApplyError, Context};
use super::unmerged_paths;

/// A rewrite with branches built on his runs git on the top of the line
/// (ADR-0047 §3), so git's `head-name` is the top branch while the entry's
/// `head_before` is his: the entry's saved plan says both.
pub(super) fn started_on_its_line(
    repo: &RepoHandle,
    entry: &JournalEntry,
    started_on: &HeadRef,
) -> Result<bool, FactsError> {
    let HeadRef::Branch(git_started_on) = started_on else {
        return Ok(false);
    };
    Ok(match plan_of(repo, &entry.id)? {
        Some((_, plan)) => {
            plan.top() == git_started_on && entry.head_before == HeadRef::Branch(plan.home)
        }
        None => false,
    })
}

/// The resume's git command. With a rewrite's saved plan, its editor helper
/// answers the messages git asks for from here on — a reword or squash row
/// still to come, a picked commit after its conflict — under the plan's
/// cleanup (ADR-0047). Without one, `GIT_EDITOR=true`: git's own message
/// stands.
pub(in crate::apply) fn resume_invocation(
    repo: &RepoHandle,
    kind: InProgress,
    args: Vec<GitArg>,
    plan_dir: Option<&Path>,
    message: Option<Vec<u8>>,
) -> GitInvocation {
    let mut invocation = GitInvocation::new(repo).args(args);
    match plan_dir {
        Some(dir) => {
            invocation = invocation.env("GIT_EDITOR", saved::editor(dir));
            for (key, value) in CLEANUP {
                invocation = invocation.env(key, value);
            }
        }
        None => invocation = invocation.env("GIT_EDITOR", "true"),
    }
    // Stopped hook-first, as a local plan line is (`run::plan_git`,
    // m8-polish-and-ship/13c): a `--continue` stuck in its commit hook is
    // ended and git leaves the operation live, so `c` is offered again. Not
    // a rebase's, for the reason `engine::rewrite_git` gives: its hooks
    // failing do not stop it.
    let invocation = if kind == InProgress::Rebase {
        invocation
    } else {
        invocation.stop_hooks_first()
    };
    match message {
        Some(message) => invocation.stdin(message),
        None => invocation,
    }
}

/// git stopped on a rewrite's `edit` row: the same stop `apply` reports,
/// with whatever the status lists (nothing, for an `edit`).
pub(super) fn stopped_on_edit(
    ctx: &mut Context<'_>,
    kind: InProgress,
    mut entry: JournalEntry,
    commands: Vec<String>,
) -> Result<ApplyError, ApplyError> {
    ctx.journal.hold_commands(&entry.id, commands.clone());
    let paths = unmerged_paths(ctx, &())?;
    entry.commands = commands;
    Ok(ApplyError::Conflicted {
        entry: Box::new(entry),
        kind,
        paths,
    })
}
