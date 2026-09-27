//! Step 6 itself: the apply and the drop, and why `S` stays listed when it
//! does.

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::run::{GitArg, GitInvocation, RunError};

use crate::facts::FactsError;

use super::put_back::{self, PutBackRan};
use super::{
    Context, EditsAt, listed_at, reason_for, rebuild, short, staging_clause, where_listed,
};

/// What step 6 came to.
pub(super) enum Returned {
    /// Put back with nothing unmerged, and `S` dropped. `unstaged`: the
    /// staged paths whose staging did not come back (git could not reinstate
    /// the index, [`PutBackRan::Unstaged`]); empty when the index came back.
    Back { unstaged: Vec<PathBuf> },
    /// Put back with these paths unmerged; `S` kept.
    Clashed(Vec<PathBuf>),
    /// `S` kept for another reason.
    Kept(Kept),
}

/// Why `S` is still in the list after step 6.
pub(super) enum Kept {
    /// git failed: the apply (nothing came back), or the drop after a clean
    /// apply (`back`: the edits are in the tree **and** in the list;
    /// `unstaged` as [`Returned::Back`]'s).
    Git {
        error: RunError,
        back: bool,
        unstaged: Vec<PathBuf>,
    },
    /// The status could not be read after the apply, so whether it clashed
    /// is unknown; `S` is not dropped on a guess.
    Unread(FactsError),
    /// The apply was clean, but the stash list could not be read to find
    /// `S`'s position to drop it: the edits are back **and** `S` is still
    /// listed (73 review, m3).
    Unlisted {
        why: FactsError,
        unstaged: Vec<PathBuf>,
    },
}

impl Kept {
    pub(super) fn into_error(self) -> Result<RunError, FactsError> {
        match self {
            Kept::Git { error, .. } => Ok(error),
            Kept::Unread(why) | Kept::Unlisted { why, .. } => Err(why),
        }
    }

    /// Where the edits are, when it is known (ADR-0045 §5 amended), from
    /// `listed`, the one read of the stash list [`close`] made.
    pub(super) fn edits(&self, listed: &Result<Option<usize>, FactsError>) -> Option<EditsAt> {
        let at = listed.as_ref().ok().copied().flatten();
        match self {
            // Not back: the copy in the list is the only place they are. An
            // unfound or unreadable copy has no position to name.
            Kept::Git { back: false, .. } => at.map(|n| EditsAt::Stash { n }),
            Kept::Git { back: true, .. } => Some(EditsAt::Back { copy: at }),
            // Whether the put-back clashed is unknown: no place is claimed.
            Kept::Unread(_) => None,
            Kept::Unlisted { .. } => Some(EditsAt::Back { copy: None }),
        }
    }

    /// The entry's reason: what went wrong, then where the edits are — and,
    /// when they are back without their staging, ` · staging not kept:
    /// <paths>` (ADR-0045 amendment 2026-09-26 b).
    pub(super) fn words(&self, listed: &Result<Option<usize>, FactsError>, stash: Oid) -> String {
        match self {
            Kept::Git {
                error, back: false, ..
            } => {
                format!("{} · {}", reason_for(error), where_listed(listed, stash))
            }
            Kept::Git {
                error,
                back: true,
                unstaged,
            } => format!(
                "your edits are back, and the set-aside copy could not be dropped: {} · {}{}",
                reason_for(error),
                match listed {
                    Ok(Some(at)) => format!("a copy stays in stash@{{{at}}}"),
                    _ => where_listed(listed, stash),
                },
                staging_clause(unstaged)
            ),
            Kept::Unread(why) => format!(
                "the status after putting your edits back could not be read: {why} · {}",
                where_listed(listed, stash)
            ),
            // The list is what could not be read, so the copy is named by
            // its commit rather than by a second read of the same list.
            Kept::Unlisted { why, unstaged } => format!(
                "your edits are back; the set-aside copy could not be found to drop ({why}) · \
                 it is stash commit {}{}",
                short(stash),
                staging_clause(unstaged)
            ),
        }
    }
}

/// Step 6 itself: the apply ([`put_back::run_put_back`]) and the drop.
///
/// With `index_base` — absorb, gone through — and `HEAD`'s tree the one the
/// plan said it would land on, the input is `S` rebuilt on the base
/// ([`rebuild::from_index_base`]); otherwise `S` itself. Either goes back
/// with `--index`, falling back to a plain apply when git cannot reinstate
/// the index (ADR-0045 §4 step 6, amended 2026-09-26 b). `S` is the listed
/// copy either way, and the one dropped.
pub(super) fn put_back(
    ctx: &Context<'_>,
    stash: Oid,
    index_base: Option<Oid>,
    commands: &mut Vec<String>,
) -> Returned {
    let input = match index_base {
        Some(base) => match rebuild::from_index_base(ctx, stash, base, commands) {
            Ok(rebuilt) => rebuilt.unwrap_or(stash),
            // Nothing is applied yet: `S` is listed and untouched.
            Err(error) => {
                return Returned::Kept(Kept::Git {
                    error,
                    back: false,
                    unstaged: Vec::new(),
                });
            }
        },
        None => stash,
    };
    let unstaged = match put_back::run_put_back(ctx, input, commands) {
        Ok(PutBackRan::Whole) => Vec::new(),
        Ok(PutBackRan::Unstaged { paths }) => paths,
        Ok(PutBackRan::Clashed { paths }) => return Returned::Clashed(paths),
        Ok(PutBackRan::Failed { error }) => {
            return Returned::Kept(Kept::Git {
                error,
                back: false,
                unstaged: Vec::new(),
            });
        }
        Err(why) => return Returned::Kept(Kept::Unread(why)),
    };
    match drop_by_id(ctx, stash, commands) {
        Ok(()) => Returned::Back { unstaged },
        Err(Dropped::Git(error)) => Returned::Kept(Kept::Git {
            error,
            back: true,
            unstaged,
        }),
        Err(Dropped::Unlisted(why)) => Returned::Kept(Kept::Unlisted { why, unstaged }),
    }
}

/// Why the drop did not happen.
enum Dropped {
    Git(RunError),
    /// The stash list could not be read to find `S`'s position.
    Unlisted(FactsError),
}

/// `stash drop stash@{n}`, `n` looked up by `S`'s commit id now — git will
/// not drop by a bare commit id (72's `git_refuses_a_stash_drop_by_bare_commit_id`).
/// `S` not listed at all means something else already took it out: there
/// is nothing left to drop.
fn drop_by_id(ctx: &Context<'_>, stash: Oid, commands: &mut Vec<String>) -> Result<(), Dropped> {
    let Some(at) = listed_at(ctx, stash).map_err(Dropped::Unlisted)? else {
        return Ok(());
    };
    let drop = GitInvocation::new(ctx.repo).args([
        GitArg::Flag("stash"),
        GitArg::Flag("drop"),
        GitArg::Word(format!("stash@{{{at}}}")),
    ]);
    commands.push(drop.command_line());
    drop.run(&()).map(|_output| ()).map_err(Dropped::Git)
}
