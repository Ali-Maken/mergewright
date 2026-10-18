//! The put-back from the journal's list (ADR-0046 §3):
//! `Operation::Recover`. `journal_list::recover_for` chooses the target;
//! this is its check, plan, touches and words (m7-power-tools/23).
//!
//! - `MoveRef` off `HEAD`: `update-ref refs/heads/<b> <to> <expected>`, a
//!   compare-and-swap. On `HEAD`'s branch: `reset --hard <to>`, inside the
//!   ADR-0045 set-aside (`Operation::sets_aside`), after `check` found the
//!   tip still at `expected`.
//! - `Recreate`: `branch <name> <at>`.
//! - `Files`: `restore --source=<backup tree> --worktree -- <paths>`. The
//!   index is untouched. A path with his own changes is written over, not
//!   refused or set aside (the edit *is* the file): the entry's
//!   before-snapshot keeps what it replaced, and the preview names it
//!   (`FilesEffect::Changes::replaced`, m7/30o R1). An ignored file there,
//!   which the snapshot can't hold, is refused by `in_the_way`.
//!
//! `u` is the generic inverse from the entry's before-fields: the branch
//! back to `expected`, the made branch deleted, the files back from this
//! entry's own backup. No inverse code of its own.

use std::path::PathBuf;

use mergewright_core::journal::{EntryId, HeadRef, Touches};
use mergewright_core::oid::Oid;
use mergewright_git::run::GitArg;

use super::history::{branch_ref, new_ref, short, update_ref};
use super::stage::with_paths;
use super::{Invocation, Refusal, RepoFacts};

/// The thing a put-back changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoverTarget {
    /// Move an existing branch back: `name` (short) to `to`, but only while
    /// it still sits at `expected` — a compare-and-swap, so a branch that
    /// moved on since is never moved over.
    MoveRef {
        /// The branch, short name (`main`).
        name: String,
        /// Where it goes back to.
        to: Oid,
        /// Where it must be now.
        expected: Oid,
    },
    /// Make a deleted branch again at its lost tip.
    Recreate {
        /// The branch, short name (`spike`).
        name: String,
        /// Its tip.
        at: Oid,
    },
    /// Put back an edit an undo (or redo) replaced, from that entry's
    /// backup tree. The index is untouched.
    Files {
        /// The undo or redo that replaced it.
        entry: EntryId,
        /// Its backup's working-tree tree, which holds the edit.
        backup_tree: Oid,
        /// The files to put back.
        paths: Vec<PathBuf>,
    },
}

/// A put-back, and the event it undoes in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverParams {
    /// What changes.
    pub target: RecoverTarget,
    /// The event it puts back, as the journal's description ends:
    /// `the shell's reset`, `the shell's branch -D`, `undo of "commit x"`.
    pub because: String,
}

/// Why a row of the journal's list has no put-back of its own
/// ([`super::Refusal::NoPutBack`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoPutBack {
    /// A row Mergewright journaled: `u` undoes it, and `⏎` on it is the time
    /// machine's revert-to (ADR-0044 §2), never a put-back.
    Mergewright,
    /// The shell's switch: it moved no branch, so there is nothing to put
    /// back. The app's switch goes back instead.
    Switch {
        /// The branch (or commit) `HEAD` was on before, as git named it.
        back_to: String,
    },
    /// The step made the ref: before it there was nothing to go back to.
    Created {
        /// The branch, short name.
        name: String,
    },
    /// No such row: the list changed under the cursor.
    Nothing,
}

/// A put-back's `your files:` line in its preview (note J1): what the
/// working tree gains and loses. `PreviewPlan::files` carries it for a
/// put-back and is `None` for every other operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesEffect {
    /// Nothing on disk changes: `your files: unchanged (main isn't checked
    /// out)`.
    Unchanged {
        /// Why, in words: `main isn't checked out`.
        not_checked_out: String,
    },
    /// Files change: `your files: src/api/limit.rs comes back · 1 unsaved
    /// file set aside, put back after`.
    Changes {
        /// Files written back: added, or changed to the put-back's copy.
        come_back: Vec<PathBuf>,
        /// Files removed: absent from what is put back.
        go: Vec<PathBuf>,
        /// How many changed files of his are set aside and put back after.
        set_aside: usize,
        /// His changed files the write goes over: `what's in src/a.rs now is
        /// replaced`. A file put-back's paths changed against `HEAD`
        /// (staged, unstaged or untracked); its entry's before-snapshot
        /// keeps them, so `u` brings them back (m7/30o R1). Empty for every
        /// other put-back.
        replaced: Vec<PathBuf>,
        /// Those of `replaced` whose index entry differs from `HEAD`: the
        /// put-back writes the working tree only, so his staged copy stays
        /// (`src/a.rs stays staged as you had it`, m7/30p R2). A subset of
        /// `replaced`; empty for every other put-back.
        still_staged: Vec<PathBuf>,
    },
}

// ── the operation's properties ──────────────────────────────────────────

/// `HEAD` is on the branch a `MoveRef` moves: the move is a `reset --hard`,
/// with his edits set aside around it.
pub(super) fn moves_checked_out(params: &RecoverParams, facts: &RepoFacts) -> bool {
    match &params.target {
        RecoverTarget::MoveRef { name, .. } => {
            matches!(&facts.head, HeadRef::Branch(head) if *head == branch_ref(name))
        }
        RecoverTarget::Recreate { .. } | RecoverTarget::Files { .. } => false,
    }
}

/// The refs `check` reads: the branch, for the two targets that have one.
pub(super) fn refs(params: &RecoverParams) -> Vec<String> {
    match &params.target {
        RecoverTarget::MoveRef { name, .. } | RecoverTarget::Recreate { name, .. } => {
            vec![branch_ref(name)]
        }
        RecoverTarget::Files { .. } => vec![],
    }
}

/// `put main back to 4c5d6e7 (the shell's reset)`, `made spike again at
/// 9a8b7c6 (the shell's branch -D)`, `put back your edit to src/a.rs`.
pub(super) fn describe(params: &RecoverParams) -> String {
    let because = &params.because;
    match &params.target {
        RecoverTarget::MoveRef { name, to, .. } => {
            format!("put {name} back to {} ({because})", short(*to))
        }
        RecoverTarget::Recreate { name, at } => {
            format!("made {name} again at {} ({because})", short(*at))
        }
        RecoverTarget::Files { paths, .. } => match paths.as_slice() {
            [] => "put back your edit".to_owned(),
            [one] => format!("put back your edit to {}", one.display()),
            [first, rest @ ..] => format!(
                "put back your edit to {} +{} more",
                first.display(),
                rest.len()
            ),
        },
    }
}

/// The branch still where the list saw it (the compare-and-swap, before
/// anything runs), the commit not held again, the name free, a file
/// put-back with something to put back.
pub(super) fn check(params: &RecoverParams, facts: &RepoFacts) -> Result<(), Refusal> {
    match &params.target {
        RecoverTarget::MoveRef { name, to, expected } => {
            let full = branch_ref(name);
            match facts.refs.get(&full).copied().flatten() {
                None => Err(Refusal::NoSuchRef { name: full }),
                Some(now) if now == *to => Err(Refusal::HeldAgain {
                    commit: *to,
                    by: name.clone(),
                }),
                Some(now) if now == *expected => Ok(()),
                Some(now) => Err(Refusal::MovedOnSince {
                    name: name.clone(),
                    now,
                }),
            }
        }
        RecoverTarget::Recreate { name, at } => {
            if facts.refs.get(&branch_ref(name)).copied().flatten() == Some(*at) {
                return Err(Refusal::HeldAgain {
                    commit: *at,
                    by: name.clone(),
                });
            }
            new_ref(&branch_ref(name), name, facts)
        }
        RecoverTarget::Files { paths, .. } => {
            // His changes at these paths are written over, not refused:
            // the entry's before-snapshot keeps them (tracked content and
            // untracked files), so `u` brings them back, and the preview
            // names them (`FilesEffect::Changes::replaced`, m7/30o R1). An
            // ignored file there the snapshot can't hold: `in_the_way`
            // refuses that, in the preview and again at `y`.
            if paths.is_empty() {
                Err(Refusal::NoPutBack(NoPutBack::Nothing))
            } else {
                Ok(())
            }
        }
    }
}

pub(super) fn plan(params: &RecoverParams, facts: &RepoFacts) -> Vec<Invocation> {
    match &params.target {
        RecoverTarget::MoveRef { name, to, expected } => {
            if moves_checked_out(params, facts) {
                vec![Invocation::new(vec![
                    GitArg::Flag("reset"),
                    GitArg::Flag("--hard"),
                    GitArg::Oid(*to),
                ])]
            } else {
                vec![update_ref(&branch_ref(name), *to, *expected)]
            }
        }
        RecoverTarget::Recreate { name, at } => vec![Invocation::new(vec![
            GitArg::Flag("branch"),
            GitArg::Ref(name.clone().into_bytes()),
            GitArg::Oid(*at),
        ])],
        RecoverTarget::Files {
            backup_tree, paths, ..
        } => vec![Invocation::new(with_paths(
            vec![
                GitArg::Flag("restore"),
                GitArg::Word(format!("--source={backup_tree}")),
                GitArg::Flag("--worktree"),
            ],
            paths,
        ))],
    }
}

/// The branch — with the index and the working tree when it is checked out
/// (`Operation::touches` adds `refs/stash` when edits are set aside) — or
/// the working tree alone for files.
pub(super) fn touches(params: &RecoverParams, facts: &RepoFacts) -> Touches {
    match &params.target {
        RecoverTarget::MoveRef { .. } | RecoverTarget::Recreate { .. } => {
            let on_head = moves_checked_out(params, facts);
            Touches {
                refs: refs(params),
                index: on_head,
                worktree: on_head,
            }
        }
        RecoverTarget::Files { .. } => Touches {
            refs: vec![],
            index: false,
            worktree: true,
        },
    }
}

/// The commit a `MoveRef` moves the branch off: nothing else may hold it,
/// and `u` puts the branch back on it, so the entry pins it past `gc`.
pub(crate) fn pins(params: &RecoverParams) -> Vec<Oid> {
    match &params.target {
        RecoverTarget::MoveRef { expected, .. } => vec![*expected],
        RecoverTarget::Recreate { .. } | RecoverTarget::Files { .. } => vec![],
    }
}
