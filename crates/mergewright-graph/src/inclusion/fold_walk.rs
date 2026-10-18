//! [`FoldWalk`]: merged branches folded by default, decided one commit at a
//! time in the order the walk emits them (ease-pass, hard choice 2).
//!
//! **Shown** is the first-parent closure of three kinds of root: the keep
//! tips (trunk, `HEAD`, integration branches), every tip nothing points at,
//! and each **opened** merge's second and later parents. Everything else is
//! **hidden**, behind the merge whose second parent first reached it.
//!
//! A commit's verdict is decided from the children already emitted. Every
//! reason to show is final, so in a topological order (a commit-graph, or
//! after promotion) the verdict is exact. In the provisional date order a
//! backward clock can emit a parent before its child: "hidden" is then the
//! only verdict a late child can contradict, and [`FoldWalk::take_late`]
//! reports it so the caller can force the commit shown and decide again.
//!
//! A merge's edge into its second parent is drawn only when that parent is
//! shown. Whether it will be is not always known when the merge is emitted
//! (a sync merge emitted before main's line reaches the commit it merged), so
//! the edge is left out then, and [`FoldWalk::take_rejoined`] reports it once
//! the parent turns out shown; the caller draws it again from the merge's
//! row down ([`FoldWalk::rejoin`]). Nothing else changes: a merge's edges
//! never decide another commit's verdict.
//!
//! State is dense per handle (ADR-0026): a byte and a `u32` per commit, plus
//! one bit per commit in each of six [`HandleSet`]s; the maps hold only open
//! merges, opened merges and dropped edges outside the common shape.

use std::collections::HashMap;

use mergewright_core::commit_handle::{CommitHandle, HandleSet, Link, Parents};

mod pointing;
#[cfg(test)]
mod rule_pinned;
#[cfg(test)]
mod tests;

/// One admitted commit's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admit {
    /// Drawn. `inside`: the innermost opened merge whose side shows it, or
    /// `None` when something other than an opened merge shows it.
    Shown {
        /// The opened merge whose side this commit is on.
        inside: Option<CommitHandle>,
    },
    /// Folded away. `behind`: the merge it hides behind — the one to open
    /// next to show it (a hidden merge's own `behind` carries on the chain).
    Hidden {
        /// The merge this commit hides behind.
        behind: CommitHandle,
    },
}

/// A reason to treat a commit specially, given before it is admitted:
/// [`FoldWalk::new`]'s lists, or [`FoldWalk::mark`] for a commit the walk
/// learned a handle for later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Always shown: the trunk, `HEAD` or an integration branch's tip.
    Keep,
    /// A merge whose merged side is shown.
    Opened,
    /// Shown: the provisional order hid it too early ([`FoldWalk::take_late`]).
    Forced,
}

/// Where one commit stands. Stored as a byte and a `u32` per handle
/// ([`FoldWalk::get`] / [`FoldWalk::set`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stand {
    /// Nothing has pointed at it yet.
    Unseen,
    /// A shown child (or an opened merge) wants it shown, `inside` the
    /// candidate's side.
    Wanted { inside: Option<CommitHandle> },
    /// Only hidden commits or a folding merge point at it; it would hide
    /// `behind` that merge.
    Pointed { behind: CommitHandle },
    /// Admitted and drawn.
    Shown { inside: Option<CommitHandle> },
    /// Admitted and folded away.
    Hidden { behind: CommitHandle },
}

/// A `link` slot with no commit in it.
const NONE: u32 = u32::MAX;

/// A shown merge's second parent not yet admitted, and whether the merge's
/// row left the edge to it out.
#[derive(Clone, Copy, Debug)]
struct Waiting {
    merge: CommitHandle,
    dropped: bool,
}

/// The streaming fold predicate, fed in emitted order. Exact when the order
/// is topological; see the module docs for the provisional order.
#[derive(Clone, Debug, Default)]
pub struct FoldWalk {
    keep: HandleSet,
    opened: HandleSet,
    forced: HandleSet,
    /// Per handle: which [`Stand`], as a byte.
    state: Vec<u8>,
    /// Per handle: the commit the [`Stand`] names, or [`NONE`].
    link: Vec<u32>,
    /// Each shown opened merge's own `inside`: how opened merges nest.
    nest: HashMap<CommitHandle, Option<CommitHandle>>,
    /// Shown merges' second parents not yet admitted.
    waiting: HashMap<CommitHandle, Vec<Waiting>>,
    /// How many second parents each shown merge still waits for.
    unsettled: HashMap<CommitHandle, u32>,
    /// Shown merges with something hidden or opened behind them.
    folding: HandleSet,
    /// Shown merges whose every second parent has been admitted.
    settled: HandleSet,
    /// Two-parent shown merges whose row leaves out the second parent.
    second_dropped: HandleSet,
    /// Every other shown commit's left-out parents.
    dropped: HashMap<CommitHandle, Vec<CommitHandle>>,
    late: Vec<CommitHandle>,
    rejoined: Vec<(CommitHandle, CommitHandle)>,
    rejoins: Vec<(CommitHandle, CommitHandle)>,
}

impl FoldWalk {
    /// A walk where `keep` are always shown, `opened` show their merged
    /// side and `forced` are shown whatever points at them.
    #[must_use]
    pub fn new(keep: &[CommitHandle], opened: &[CommitHandle], forced: &[CommitHandle]) -> Self {
        let mut walk = FoldWalk::default();
        for (handles, mark) in [
            (keep, Mark::Keep),
            (opened, Mark::Opened),
            (forced, Mark::Forced),
        ] {
            for &handle in handles {
                walk.mark(handle, mark);
            }
        }
        walk
    }

    /// Gives `commit` a [`Mark`]. Takes effect only if `commit` has not been
    /// admitted, nor (for [`Mark::Keep`] and [`Mark::Forced`]) named as a
    /// merge's second parent yet: marks belong before the walk reaches them.
    pub fn mark(&mut self, commit: CommitHandle, mark: Mark) {
        let set = match mark {
            Mark::Keep => &mut self.keep,
            Mark::Opened => &mut self.opened,
            Mark::Forced => &mut self.forced,
        };
        set.insert(commit);
    }

    /// Decides `commit`, whose `parents` (first first) are all it is
    /// pointed through. Call in emitted order, once per commit; a commit
    /// admitted again keeps its first verdict.
    pub fn admit(&mut self, commit: CommitHandle, parents: &[CommitHandle]) -> Admit {
        let admit = match self.get(commit) {
            Stand::Shown { inside } => return Admit::Shown { inside },
            Stand::Hidden { behind } => return Admit::Hidden { behind },
            _ if self.keep.contains(commit) => Admit::Shown { inside: None },
            Stand::Wanted { inside } => Admit::Shown { inside },
            _ if self.forced.contains(commit) => Admit::Shown { inside: None },
            Stand::Pointed { behind } => Admit::Hidden { behind },
            // Nothing has pointed at it: a tip of its own.
            Stand::Unseen => Admit::Shown { inside: None },
        };
        match admit {
            Admit::Shown { inside } => {
                self.set(commit, Stand::Shown { inside });
                if parents.len() > 1 && self.opened.contains(commit) {
                    self.nest.insert(commit, inside);
                }
            }
            Admit::Hidden { behind } => self.set(commit, Stand::Hidden { behind }),
        }
        self.settle(commit, admit);
        match admit {
            Admit::Shown { inside } => self.point_from_shown(commit, inside, parents),
            Admit::Hidden { behind } => self.point_from_hidden(commit, behind, parents),
        }
        admit
    }

    /// `link` as its row draws it: without the parents its admission left
    /// out (a folding merge's second parent; a parent the provisional order
    /// hid too early).
    #[must_use]
    pub fn trim(&self, mut link: Link) -> Link {
        if self.second_dropped.contains(link.commit) {
            link.parents = link.parents.first().map_or(Parents::None, Parents::One);
        } else if let Some(dropped) = self.dropped.get(&link.commit) {
            link.parents = link
                .parents
                .iter()
                .filter(|parent| !dropped.contains(parent))
                .collect();
        }
        link
    }

    /// Hidden commits a later-emitted shown child reaches (by its first
    /// parent, or as an opened merge's second): in date order only. Force
    /// them shown and decide again from the first of them. Drains.
    pub fn take_late(&mut self) -> Vec<CommitHandle> {
        std::mem::take(&mut self.late)
    }

    /// `(merge, parent)`: a shown merge's row left its edge to `parent` out
    /// when nothing yet said `parent` was shown, and it turned out shown.
    /// [`FoldWalk::rejoin`] each and lay out again from the merge. Drains.
    pub fn take_rejoined(&mut self) -> Vec<(CommitHandle, CommitHandle)> {
        std::mem::take(&mut self.rejoined)
    }

    /// Draws `merge`'s edge to `parent` from now on ([`FoldWalk::trim`]).
    pub fn rejoin(&mut self, merge: CommitHandle, parent: CommitHandle) {
        if !self.second_dropped.remove(merge)
            && let Some(dropped) = self.dropped.get_mut(&merge)
        {
            dropped.retain(|&left_out| left_out != parent);
            if dropped.is_empty() {
                self.dropped.remove(&merge);
            }
        }
        self.rejoins.push((merge, parent));
    }

    /// Every edge [`FoldWalk::rejoin`] has drawn again, oldest first.
    #[must_use]
    pub fn rejoins(&self) -> &[(CommitHandle, CommitHandle)] {
        &self.rejoins
    }

    /// `Some(true)` once a second parent of shown `merge` was admitted
    /// hidden, or shown only because `merge` is opened; `Some(false)` once
    /// all were admitted and none was; `None` before, and for anything that
    /// is not a shown merge.
    #[must_use]
    pub fn folds(&self, merge: CommitHandle) -> Option<bool> {
        if self.folding.contains(merge) {
            Some(true)
        } else if self.settled.contains(merge) {
            Some(false)
        } else {
            None
        }
    }

    /// The merge a walked hidden commit sits behind; `None` if shown or not
    /// walked.
    #[must_use]
    pub fn behind(&self, commit: CommitHandle) -> Option<CommitHandle> {
        match self.get(commit) {
            Stand::Hidden { behind } => Some(behind),
            _ => None,
        }
    }

    /// The innermost opened merge whose side shows a walked commit; `None`
    /// if hidden, not walked, or shown for another reason.
    #[must_use]
    pub fn inside(&self, commit: CommitHandle) -> Option<CommitHandle> {
        match self.get(commit) {
            Stand::Shown { inside } => inside,
            _ => None,
        }
    }

    /// Heap bytes held, capacity included.
    #[must_use]
    pub fn approximate_bytes(&self) -> usize {
        let sets = [
            &self.keep,
            &self.opened,
            &self.forced,
            &self.folding,
            &self.settled,
            &self.second_dropped,
        ];
        let handle = std::mem::size_of::<CommitHandle>();
        sets.iter()
            .map(|set| set.approximate_bytes())
            .sum::<usize>()
            + self.state.capacity()
            + self.link.capacity() * std::mem::size_of::<u32>()
            + self.nest.capacity() * 3 * handle
            + self.waiting.capacity() * (handle + std::mem::size_of::<Vec<Waiting>>())
            + self.unsettled.capacity() * 2 * handle
            + self.dropped.capacity() * (handle + std::mem::size_of::<Vec<CommitHandle>>())
            + (self.rejoins.capacity() + self.rejoined.capacity()) * 2 * handle
    }

    /// Decodes `commit`'s [`Stand`]; a handle never set is unseen. The
    /// byte says which `Stand`, the `u32` the commit it names: [`NONE`]
    /// only where the `Stand` may name none, since [`FoldWalk::set`] stores
    /// every `Pointed`/`Hidden` with its merge.
    fn get(&self, commit: CommitHandle) -> Stand {
        let index = commit.index();
        let link = self.link.get(index).copied().unwrap_or(NONE);
        let named = (link != NONE).then_some(CommitHandle(link));
        match self.state.get(index).copied().unwrap_or(0) {
            0 => Stand::Unseen,
            1 => Stand::Wanted { inside: named },
            2 => Stand::Pointed {
                behind: CommitHandle(link),
            },
            3 => Stand::Shown { inside: named },
            _ => Stand::Hidden {
                behind: CommitHandle(link),
            },
        }
    }

    fn set(&mut self, commit: CommitHandle, stand: Stand) {
        let index = commit.index();
        if index >= self.state.len() {
            self.state.resize(index + 1, 0);
            self.link.resize(index + 1, NONE);
        }
        let (byte, link) = match stand {
            Stand::Unseen => (0, None),
            Stand::Wanted { inside } => (1, inside),
            Stand::Pointed { behind } => (2, Some(behind)),
            Stand::Shown { inside } => (3, inside),
            Stand::Hidden { behind } => (4, Some(behind)),
        };
        self.state[index] = byte;
        self.link[index] = link.map_or(NONE, |link| link.0);
    }
}
