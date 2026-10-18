//! Absorb's plan: where his branch starts, the map ([`crate::absorb`]),
//! and the todo that melds each placed hunk into its commit.

use std::collections::BTreeMap;

use mergewright_core::oid::Oid;
use mergewright_core::on_disk::MESSAGE_TAG;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_git::version::GitVersion;

use crate::absorb::{self, AbsorbMap, StaysWhy};
use crate::operation::{Refusal, RepoFacts, Tip};

use super::dependents::line_of;
use super::plan::{head_of, refuse_merges, short_name};
use super::reads::{RangeCommit, read_range, unparsable};
use super::unpushed::{RangeStop, your_commits};
use super::{RewriteError, RewriteParams, TodoAction, TodoLine};

/// Where absorb's range starts, and the reason a hunk older than it stays
/// with.
struct AbsorbBase {
    base: Oid,
    below: StaysWhy,
}

/// Where his commits start ([`your_commits`], 06e ruling), refused when
/// there are none. A hunk whose lines are all older stays with the walk's
/// stop as its reason: below a pushed commit everything is pushed; below
/// the base is "older than your branch"; below a merge or the fallback's
/// cap is "outside your commits".
fn absorb_base(
    repo: &RepoHandle,
    head: &Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<AbsorbBase, RewriteError> {
    let range = your_commits(repo, head, default_branch, stop)?;
    if range.commits.is_empty() {
        let refusal = match range.stop {
            RangeStop::Pushed => Refusal::AllPushed {
                branch: short_name(head),
            },
            RangeStop::NotItsOwn => Refusal::NoCommitsOfItsOwn {
                branch: short_name(head),
            },
            RangeStop::Merge(merge) => Refusal::MergeInRange { merge },
            RangeStop::Root => Refusal::RootCommit { commit: head.oid },
            // The base is below `HEAD` and the cap is 10: neither can stop
            // the walk before it takes `HEAD`.
            RangeStop::Base | RangeStop::Cap => {
                return Err(unparsable(format!(
                    "absorb's range took no commit, stopped at {:?}",
                    range.stop
                ))
                .into());
            }
        };
        return Err(RewriteError::Refused(refusal));
    }
    let below = match range.stop {
        RangeStop::Pushed => StaysWhy::AlreadyPushed,
        RangeStop::Base => StaysWhy::BeforeYourBranch,
        RangeStop::NotItsOwn | RangeStop::Merge(_) | RangeStop::Root | RangeStop::Cap => {
            StaysWhy::OutsideYourCommits
        }
    };
    Ok(AbsorbBase {
        base: range.base,
        below,
    })
}

/// Where each staged hunk goes, over the range [`super::plan`]'s absorb
/// plans on: his commits, down to the first pushed one, the base, a merge
/// or the first commit (on the default or only branch with no base below
/// `HEAD`, at most [`super::ABSORB_FALLBACK_DEPTH`]; 06e ruling). What the
/// absorb preview lists. Reads only (objects aside).
///
/// # Errors
/// [`RewriteError::Refused`]: not on a branch; `HEAD` pushed
/// ([`Refusal::AllPushed`]); no commits of its own
/// ([`Refusal::NoCommitsOfItsOwn`]); `HEAD` a merge or the first commit.
/// [`RewriteError::Facts`] when a read fails; [`RewriteError::Stopped`].
pub fn absorb_plan_map(
    repo: &RepoHandle,
    facts: &RepoFacts,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<AbsorbMap, RewriteError> {
    let head = head_of(repo, facts, stop)?;
    let AbsorbBase { base, below } = absorb_base(repo, &head, default_branch, stop)?;
    Ok(absorb::mapped(repo, head.oid, base, below, stop)?.into_map())
}

/// Absorb: each staged hunk the map places ([`crate::absorb`]) goes into a
/// commit object with no ref (parent `HEAD`, one per target), planned as a
/// fixup right after its target — or, when they would empty it, the target
/// dropped. The rows start at the oldest target. Hunks that stay are left
/// where they are: the set-aside puts them back with the index, measured
/// from `index_base` (`HEAD` plus every placed hunk).
pub(super) fn absorb(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    let AbsorbBase { base, below } = absorb_base(repo, &head, default_branch, stop)?;
    let range = read_range(repo, base, head.oid, false, stop)?;
    let mapped = absorb::mapped(repo, head.oid, base, below, stop)?;
    let Some(first) = range.iter().position(|commit| {
        mapped
            .placed
            .iter()
            .any(|(_, target)| *target == commit.oid)
    }) else {
        return Err(RewriteError::Refused(Refusal::NothingToAbsorb));
    };
    // Safe to default: `first` is a position in `range`, so `get` always
    // finds a non-empty slice; nothing is swallowed.
    let rows = range.get(first..).unwrap_or_default();
    refuse_merges(rows)?;
    let Some(&(mut onto)) = rows.first().and_then(|commit| commit.parents.first()) else {
        return Err(unparsable("absorb's first target has no parent".to_owned()).into());
    };
    let line = line_of(repo, git, &head, default_branch, stop)?;
    let (mut todo, made) = absorb_rows(repo, head.oid, rows, &mapped, stop)?;
    // Every row dropped (the staged lines take the top commits out whole):
    // a todo of drops alone is refused as the branch going back to its
    // base. The commit below joins as a pick; replayed onto its own parent,
    // git keeps it as it is.
    if todo.iter().all(|line| line.action == TodoAction::Drop)
        && let Some(below) = first.checked_sub(1).and_then(|at| range.get(at))
    {
        refuse_merges(std::slice::from_ref(below))?;
        let Some(&parent) = below.parents.first() else {
            return Err(
                unparsable("the commit below absorb's target has no parent".to_owned()).into(),
            );
        };
        todo.insert(0, below.pick());
        onto = parent;
    }
    let every_placed: Vec<&absorb::Piece> = mapped.placed.iter().map(|(piece, _)| piece).collect();
    let index_base = absorb::commit_pieces(
        repo,
        head.oid,
        &every_placed,
        &format!("{MESSAGE_TAG}: absorb's index base"),
        stop,
    )?;
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages: BTreeMap::new(),
        made,
        index_base: Some(index_base),
        forks: Vec::new(),
    })
}

/// Absorb's todo over `rows`, and the commit objects its fixups meld in:
/// each row a pick, followed by a fixup of the pieces placed into it — or,
/// when they would leave it changing nothing, the row a drop.
fn absorb_rows(
    repo: &RepoHandle,
    head: Oid,
    rows: &[RangeCommit],
    mapped: &absorb::Mapped,
    stop: &dyn Stop,
) -> Result<(Vec<TodoLine>, Vec<Oid>), RewriteError> {
    let mut todo = Vec::with_capacity(rows.len() + mapped.placed.len());
    let mut made = Vec::new();
    for commit in rows {
        let pieces: Vec<&absorb::Piece> = mapped
            .placed
            .iter()
            .filter(|(_, target)| *target == commit.oid)
            .map(|(piece, _)| piece)
            .collect();
        if pieces.is_empty() {
            todo.push(commit.pick());
            continue;
        }
        // Melded in, the pieces would leave the commit changing nothing:
        // git stops on that fixup, so the commit is dropped instead (review
        // m7-06 M3). The rows above replay onto the same tree either way.
        if let Some(&parent) = commit.parents.first()
            && absorb::empties(repo, commit.oid, parent, &pieces, stop)?
        {
            todo.push(TodoLine {
                action: TodoAction::Drop,
                ..commit.pick()
            });
            continue;
        }
        todo.push(commit.pick());
        let subject = format!("fixup! {}", commit.subject);
        let oid = absorb::commit_pieces(repo, head, &pieces, &subject, stop)?;
        todo.push(TodoLine {
            action: TodoAction::Fixup,
            commit: oid,
            subject,
        });
        made.push(oid);
    }
    Ok((todo, made))
}
