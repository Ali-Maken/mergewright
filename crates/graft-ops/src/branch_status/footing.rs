//! Where a branch stands (ease-pass 01r, R2), decided before its route:
//! held by a long-lived line (on that line, or merged into it), merged
//! into an ordinary base, or its own. Only what git proves: kind,
//! containment, and where the branch entered a holder's first-parent line
//! ([`entry_into`]).

use graft_core::oid::Oid;
use graft_git::history_reads::{LineEntry, count_between, entry_into};
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};

use super::{Apart, Candidate, CandidateKind};

/// R2 case 1's deciding holder: the first long-lived line holding the
/// branch, by kind (trunk, then integration), then fewest commits of its
/// own, then name.
pub(super) struct Held<'a> {
    /// The holder and how far it is from the branch.
    pub(super) holder: &'a (Candidate, Apart),
    /// Where the branch entered the holder's line; `None` at the same tip.
    pub(super) entry: Option<LineEntry>,
}

impl Held<'_> {
    /// The holder's line runs straight through `tip`: the same commit, or
    /// the entry's first parent is `tip`. The branch has no commits of its
    /// own there: just made, left at an older commit, or fast-forwarded in.
    pub(super) fn on_line(&self, tip: Oid) -> bool {
        self.entry
            .as_ref()
            .is_none_or(|entry| entry.parents.first() == Some(&tip))
    }
}

/// Where a branch stands.
pub(super) enum Footing<'a> {
    /// On this long-lived line, with no commits of its own.
    OnLine(&'a (Candidate, Apart)),
    /// Taken in by a merge into `into`.
    Merged {
        /// The branch that took it in.
        into: &'a Candidate,
        /// Its own commits are those this commit lacks; `None`: it has
        /// none of its own (the nearest long-lived holder has it on its
        /// line).
        own_from: Option<Oid>,
    },
    /// Neither: not merged anywhere git can prove.
    Own,
}

/// The trunk and integration candidates holding every commit of the
/// branch, equal tips included.
fn long_lived_holders(
    measured: &[(Candidate, Apart)],
) -> impl Iterator<Item = &(Candidate, Apart)> {
    measured
        .iter()
        .filter(|(c, apart)| c.kind != CandidateKind::Other && apart.branch_only == 0)
}

/// R2 case 1: the deciding long-lived holder of `branch`, if any. One
/// [`entry_into`] read, none at an equal tip.
///
/// # Errors
/// [`RunError`] from a read, [`RunError::Stopped`] included.
pub(super) fn held_by_long_lived<'a>(
    repo: &RepoHandle,
    branch: &Candidate,
    measured: &'a [(Candidate, Apart)],
    stop: &dyn Stop,
) -> Result<Option<Held<'a>>, RunError> {
    let Some(holder) = long_lived_holders(measured).min_by(|(a, a_apart), (b, b_apart)| {
        (a.kind, a_apart.base_only, &a.name).cmp(&(b.kind, b_apart.base_only, &b.name))
    }) else {
        return Ok(None);
    };
    let entry = if holder.0.tip == branch.tip {
        None
    } else {
        Some(entry(repo, branch, &holder.0, stop)?)
    };
    Ok(Some(Held { holder, entry }))
}

/// R2 in full: case 1, then case 2, else its own.
///
/// # Errors
/// [`RunError`] from a read, [`RunError::Stopped`] included.
pub(super) fn footing<'a>(
    repo: &RepoHandle,
    branch: &Candidate,
    measured: &'a [(Candidate, Apart)],
    stop: &dyn Stop,
) -> Result<Footing<'a>, RunError> {
    let Some(held) = held_by_long_lived(repo, branch, measured, stop)? else {
        return Ok(match merged_into_ordinary(repo, branch, measured, stop)? {
            Some((into, under)) => Footing::Merged {
                into,
                own_from: Some(under),
            },
            None => Footing::Own,
        });
    };
    if held.on_line(branch.tip) {
        return Ok(Footing::OnLine(held.holder));
    }
    // Its own commits are read against the nearest long-lived holder's
    // entry, so an integration branch's own work is never counted as the
    // branch's when the trunk decided "merged".
    let nearest = long_lived_holders(measured).min_by(|(a, a_apart), (b, b_apart)| {
        (a_apart.base_only, a.kind, &a.name).cmp(&(b_apart.base_only, b.kind, &b.name))
    });
    let entry = match nearest {
        Some((near, _)) if near.name != held.holder.0.name => (near.tip != branch.tip)
            .then(|| entry(repo, branch, near, stop))
            .transpose()?,
        _ => held.entry,
    };
    let own_from = match entry {
        None => None,
        Some(entry) => match entry.parents.first() {
            Some(&under) if under == branch.tip => None,
            Some(&under) => Some(under),
            None => return Err(no_entry(branch, &held.holder.0)),
        },
    };
    Ok(Footing::Merged {
        into: &held.holder.0,
        own_from,
    })
}

/// R2 case 2: an ordinary holder counts only when its entry is a merge
/// whose first parent lies under the branch (strictly): it sat under the
/// branch when it took it in. A teammate cut from an older commit who then
/// synced has his own commit there, and does not count; nor does a branch
/// built straight on the tip that later merged something else. Among those
/// that count: the one every other counted holder holds, then fewest
/// commits of its own, then name. Answers the holder and its entry's first
/// parent.
fn merged_into_ordinary<'a>(
    repo: &RepoHandle,
    branch: &Candidate,
    measured: &'a [(Candidate, Apart)],
    stop: &dyn Stop,
) -> Result<Option<(&'a Candidate, Oid)>, RunError> {
    let mut counted: Vec<(&'a Candidate, usize, Oid)> = Vec::new();
    let ordinary = measured.iter().filter(|(c, apart)| {
        c.kind == CandidateKind::Other && apart.branch_only == 0 && c.tip != branch.tip
    });
    for (holder, apart) in ordinary {
        let entry = entry(repo, branch, holder, stop)?;
        let [under, _, ..] = entry.parents.as_slice() else {
            continue; // not a merge
        };
        if *under != branch.tip && count_between(repo, branch.tip, *under, stop)? == 0 {
            counted.push((holder, apart.base_only, *under));
        }
    }
    let mut best: Option<((bool, usize, &str), &'a Candidate, Oid)> = None;
    for &(holder, base_only, under) in &counted {
        let mut held_by_all = true;
        for &(other, _, _) in &counted {
            if other.name != holder.name && count_between(repo, other.tip, holder.tip, stop)? != 0 {
                held_by_all = false;
                break;
            }
        }
        let key = (!held_by_all, base_only, holder.name.as_str());
        if best.as_ref().is_none_or(|(best_key, ..)| key < *best_key) {
            best = Some((key, holder, under));
        }
    }
    Ok(best.map(|(_, holder, under)| (holder, under)))
}

/// Where `branch` entered `holder`'s line, for a holder the counts say
/// holds it at another commit.
fn entry(
    repo: &RepoHandle,
    branch: &Candidate,
    holder: &Candidate,
    stop: &dyn Stop,
) -> Result<LineEntry, RunError> {
    entry_into(repo, branch.tip, holder.tip, stop)?.ok_or_else(|| no_entry(branch, holder))
}

/// A holder the counts found holding the branch has no entry for it. The
/// reads are by commit id, so this is git contradicting itself.
fn no_entry(branch: &Candidate, holder: &Candidate) -> RunError {
    RunError::Unparsable(format!(
        "{} holds {} by count, but rev-list found no line entry",
        holder.name, branch.name
    ))
}
