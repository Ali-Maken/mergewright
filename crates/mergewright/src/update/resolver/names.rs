//! The one name each side of a conflict goes by (ADR-0043 §4,
//! m6-conflicts/10): `yours · main`, `theirs · feature`, `upstream · main`,
//! `picked · 3f2a1b7 Add OAuth`. Every place the resolver names a side —
//! card labels, took words, whole-file keys, notices — reads it from
//! [`side_names`], never from git's marker labels (absent on whole-file
//! shapes, and carrying git's own parentheses).

use mergewright_core::oid::Oid;
use mergewright_git::history_reads::CommitSummary;
use mergewright_git::state::InProgress;
use mergewright_term::{sanitize_line, sanitize_path_bytes};
use mergewright_ui::conflict_resolver::{SideNames, SideRoles};

use crate::refs::RefLabel;
use crate::update::AppState;

/// Hex digits of a commit id shown when no ref names it — the seven every
/// other line in Mergewright shows (`update::preview::short`).
const SHORT_OID: usize = 7;

/// Each side's one name for the operation `state` is in.
///
/// | op | ours | theirs |
/// |---|---|---|
/// | merge, pull | `yours · <HEAD>` | `theirs · <MERGE_HEAD>` |
/// | rebase | `upstream · <onto>` | `yours · <sha7> <subject>` |
/// | cherry-pick | `yours · <HEAD>` | `picked · <sha7> <subject>` |
/// | revert | `yours · <HEAD>` | `reverting · <sha7> <subject>` |
/// | stash pop | `yours · <HEAD>` | `stashed · stash@{n}`, or `stashed` (shell pop) |
/// | none | `yours · <HEAD>` | `theirs` |
///
/// Stash pop is the resolver open with no operation in progress — the same
/// test the old explainer used: a conflicted `git stash pop` leaves no
/// marker git writes for it. Nothing git keeps says which entry was popped;
/// Mergewright's own journal does for a put-back it ran (`AppState::clash`), and
/// a shell pop's theirs is the role alone. A name that cannot be found (refs not read
/// yet, a head file git has not written) leaves the role word alone too; a
/// head file naming a commit git cannot read shows that failure instead.
#[must_use]
pub fn side_names(state: &AppState) -> SideNames {
    let head = || state.refs.as_deref().and_then(crate::refs::RefBadges::head);
    let detail = state.in_progress.as_ref();
    // Theirs' name, or why it could not be read: the error is drawn where
    // the name would be, not dropped (the operation itself still reads).
    let incoming = |name: &dyn Fn(&CommitSummary) -> String| match detail.map(|d| &d.theirs) {
        Some(Ok(Some(commit))) => Some(name(commit)),
        Some(Err(error)) => Some(error.to_string()),
        Some(Ok(None)) | None => None,
    };
    let (roles, ours, theirs) = match detail.map(|detail| detail.kind) {
        Some(InProgress::Merge) => (
            SideRoles::Merge,
            head().map(|id| branch_like(state, id)),
            incoming(&|commit| branch_like(state, commit.id)),
        ),
        Some(InProgress::Rebase) => (
            SideRoles::Rebase,
            detail
                .and_then(|detail| detail.onto)
                .map(|id| branch_like(state, id)),
            incoming(&commit_like),
        ),
        Some(InProgress::CherryPick) => (
            SideRoles::CherryPick,
            head().map(|id| branch_like(state, id)),
            incoming(&commit_like),
        ),
        Some(InProgress::Revert) => (
            SideRoles::Revert,
            head().map(|id| branch_like(state, id)),
            incoming(&commit_like),
        ),
        // Edits Mergewright set aside around a rebase or switch, clashing as they
        // came back (m6-conflicts/74): ours is the branch as it now stands,
        // `branch · main`; theirs is `yours · edits set aside`, never the
        // stash — he never named one (ADR-0043 §4 amended, m6-conflicts/84).
        None if state.resolver.is_some() && crate::update::clash::is_set_aside_clash(state) => (
            SideRoles::SetAside,
            head().map(|id| branch_like(state, id)),
            Some("edits set aside".to_owned()),
        ),
        // Theirs is the stash Mergewright recorded putting back (`stash@{0}`,
        // m6-conflicts/72b); a pop made in the shell has no record, and
        // keeps the role alone.
        None if state.resolver.is_some() => (
            SideRoles::StashPop,
            head().map(|id| branch_like(state, id)),
            crate::update::clash::stash_name(state),
        ),
        Some(InProgress::Bisect) | None => (
            SideRoles::Merge,
            head().map(|id| branch_like(state, id)),
            None,
        ),
    };
    SideNames {
        ours: sanitize_line(&named(roles.ours(), ours)),
        theirs: sanitize_line(&named(roles.theirs(), theirs)),
        roles,
    }
}

/// `o <ours>  t <theirs>` in the operation's role words — the keys a
/// status line offers for a card (`o upstream  t yours` in a rebase), never
/// git's `ours`/`theirs`, which a rebase swaps.
#[must_use]
pub(crate) fn side_keys(state: &AppState) -> String {
    let roles = side_names(state).roles;
    format!("o {}  t {}", roles.ours(), roles.theirs())
}

/// `role · name`, or the role alone when there is no name.
fn named(role: &str, name: Option<String>) -> String {
    match name {
        Some(name) => format!("{role} · {name}"),
        None => role.to_owned(),
    }
}

/// A side that is a branch tip: a local branch on `id` (the current one
/// first, else the alphabetically first), else a remote-tracking badge,
/// else the short id.
fn branch_like(state: &AppState, id: Oid) -> String {
    let labels = state.refs.as_deref().map_or(&[][..], |refs| refs.on(id));
    let mut locals: Vec<(bool, String)> = labels
        .iter()
        .filter_map(|label| match label {
            RefLabel::Branch { name, current, .. } => {
                Some((!*current, sanitize_path_bytes(name).as_str().to_owned()))
            }
            _ => None,
        })
        .collect();
    // `false` sorts first: the current branch, then by name.
    locals.sort();
    if let Some((_, name)) = locals.into_iter().next() {
        return name;
    }
    let mut remotes: Vec<String> = labels
        .iter()
        .filter_map(|label| match label {
            RefLabel::Remote(name) => Some(sanitize_path_bytes(name).as_str().to_owned()),
            _ => None,
        })
        .collect();
    remotes.sort();
    remotes.into_iter().next().unwrap_or_else(|| short(id))
}

/// A side that is one commit: `<sha7> <subject>`.
fn commit_like(commit: &CommitSummary) -> String {
    format!("{} {}", short(commit.id), commit.subject)
}

/// `id`'s first seven hex digits.
fn short(id: Oid) -> String {
    id.to_string().chars().take(SHORT_OID).collect()
}

/// Operation states for tests that need [`side_names`] to find real names.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::sync::Arc;

    use mergewright_core::commit::{Identity, Timestamp};
    use mergewright_core::oid::Oid;
    use mergewright_core::ref_name::RefName;
    use mergewright_git::history_reads::CommitSummary;
    use mergewright_git::repo::HeadState;
    use mergewright_git::state::{InProgress, InProgressDetail};

    use crate::refs::RefBadges;

    /// An id of twenty `byte`s: its short form is `byte` in hex, seven digits.
    pub(crate) fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
    }

    /// A commit with just an id and a subject.
    pub(crate) fn commit(id: Oid, subject: &str) -> CommitSummary {
        CommitSummary {
            id,
            parents: Vec::new(),
            subject: subject.to_owned(),
            author: Identity {
                name: b"Test".to_vec(),
                email: b"test@example.invalid".to_vec(),
                when: Timestamp {
                    seconds: 0,
                    offset_seconds: 0,
                },
            },
        }
    }

    /// `main` at `oid(2)`, `feature` at `oid(3)`; `HEAD` on `main`, or
    /// detached at `oid(9)` (no branch there) when `detached`.
    pub(crate) fn refs(detached: bool) -> Arc<RefBadges> {
        let name = |full: &str| RefName::new(full.as_bytes().to_vec()).expect("a valid ref name");
        let head = if detached {
            HeadState::Detached(oid(9))
        } else {
            HeadState::Branch(name("refs/heads/main"))
        };
        let badges = crate::refs::badges(
            &[
                (name("refs/heads/main"), oid(2)),
                (name("refs/heads/feature"), oid(3)),
            ],
            &head,
            &std::collections::HashMap::new(),
            &[],
            |_, _| {
                Ok(mergewright_git::history::AheadBehind {
                    ahead: 0,
                    behind: 0,
                })
            },
        )
        .expect("no ahead/behind walk can fail here");
        Arc::new(badges)
    }

    /// `kind` in progress, bringing in `theirs`; a rebase lands on `main`.
    pub(crate) fn operation(kind: InProgress, theirs: Option<CommitSummary>) -> InProgressDetail {
        InProgressDetail {
            kind,
            step: None,
            head_name: None,
            onto: (kind == InProgress::Rebase).then(|| oid(2)),
            theirs: Ok(theirs),
        }
    }
}
