//! What an admitted commit tells its parents — [`FoldWalk::admit`]'s second
//! half: which parents are wanted shown, which hide behind what, and which
//! waiting merges learn that they fold or that their edge rejoins.

use mergewright_core::commit_handle::CommitHandle;

use super::{Admit, FoldWalk, Stand, Waiting};

impl FoldWalk {
    /// A shown commit's parents: its first parent (and an opened merge's
    /// others) are wanted; a folding merge's others hide behind it.
    pub(super) fn point_from_shown(
        &mut self,
        commit: CommitHandle,
        inside: Option<CommitHandle>,
        parents: &[CommitHandle],
    ) {
        let opened = parents.len() > 1 && self.opened.contains(commit);
        let mut dropped = Vec::new();
        let mut unsettled = 0;
        let mut folds = false;
        for (index, &parent) in parents.iter().enumerate() {
            let left_out = if index == 0 {
                self.want(parent, inside)
            } else if opened {
                self.want(parent, Some(commit))
            } else if self.will_show(parent) {
                false
            } else {
                if self.get(parent) == Stand::Unseen {
                    self.set(parent, Stand::Pointed { behind: commit });
                }
                true
            };
            if left_out {
                dropped.push(parent);
            }
            if index > 0 {
                match self.get(parent) {
                    Stand::Hidden { .. } => folds = true,
                    Stand::Shown { .. } => {}
                    _ => {
                        unsettled += 1;
                        self.waiting.entry(parent).or_default().push(Waiting {
                            merge: commit,
                            dropped: left_out,
                        });
                    }
                }
            }
        }
        if parents.len() > 1 {
            if folds {
                self.folding.insert(commit);
            }
            if unsettled == 0 {
                self.settled.insert(commit);
            } else {
                self.unsettled.insert(commit, unsettled);
            }
        }
        if parents.len() == 2 && dropped == [parents[1]] {
            self.second_dropped.insert(commit);
        } else if !dropped.is_empty() {
            self.dropped.insert(commit, dropped);
        }
    }

    /// A hidden commit's parents hide behind what it hides behind — or, for
    /// an unopened merge's second parents, behind the merge itself.
    pub(super) fn point_from_hidden(
        &mut self,
        commit: CommitHandle,
        behind: CommitHandle,
        parents: &[CommitHandle],
    ) {
        let opened = self.opened.contains(commit);
        for (index, &parent) in parents.iter().enumerate() {
            let candidate = if index == 0 || opened { behind } else { commit };
            if self.get(parent) == Stand::Unseen {
                self.set(parent, Stand::Pointed { behind: candidate });
            }
        }
    }

    /// `commit` was just admitted: every shown merge waiting on it as a
    /// second parent learns whether it folds, and whether its edge rejoins.
    pub(super) fn settle(&mut self, commit: CommitHandle, admit: Admit) {
        let Some(waiting) = self.waiting.remove(&commit) else {
            return;
        };
        for Waiting { merge, dropped } in waiting {
            let folds = match admit {
                Admit::Hidden { .. } => true,
                Admit::Shown { inside } => inside == Some(merge),
            };
            if folds {
                self.folding.insert(merge);
            }
            if dropped && matches!(admit, Admit::Shown { .. }) {
                self.rejoined.push((merge, commit));
            }
            if let Some(left) = self.unsettled.get_mut(&merge) {
                *left -= 1;
                if *left == 0 {
                    self.unsettled.remove(&merge);
                    self.settled.insert(merge);
                }
            }
        }
    }

    /// Whether the walk so far shows `commit`, or has said it will. Marks
    /// are left out on purpose: a merge's row must not depend on a mark
    /// given to a commit below it, or changing that mark would change rows
    /// above the first one the change names. A marked commit a folding
    /// merge dropped is drawn again through [`FoldWalk::take_rejoined`].
    fn will_show(&self, commit: CommitHandle) -> bool {
        matches!(self.get(commit), Stand::Wanted { .. } | Stand::Shown { .. })
    }

    /// A shown commit's `parent` it draws: wanted shown, `inside` the
    /// `candidate` side (or the side enclosing both, if already wanted).
    /// `true` when the parent was already admitted hidden — a child emitted
    /// after its parent (date order) — so the row leaves it out and the
    /// parent is reported late.
    fn want(&mut self, parent: CommitHandle, candidate: Option<CommitHandle>) -> bool {
        match self.get(parent) {
            Stand::Unseen | Stand::Pointed { .. } => {
                self.set(parent, Stand::Wanted { inside: candidate });
            }
            Stand::Wanted { inside } => {
                let outer = self.outer(inside, candidate);
                self.set(parent, Stand::Wanted { inside: outer });
            }
            // Already drawn, above its child (date order): drawn as the
            // layout always has.
            Stand::Shown { .. } => {}
            Stand::Hidden { .. } => {
                self.late.push(parent);
                return true;
            }
        }
        false
    }

    /// The candidate of two that encloses the other — `None` (shown for
    /// another reason) encloses everything; of two unrelated opened sides,
    /// the first to reach the commit.
    fn outer(
        &self,
        first: Option<CommitHandle>,
        second: Option<CommitHandle>,
    ) -> Option<CommitHandle> {
        let (Some(a), Some(b)) = (first, second) else {
            return None;
        };
        if self.encloses(b, a) {
            Some(b)
        } else {
            Some(a)
        }
    }

    /// Whether opened merge `outer` is `inner` or encloses it.
    fn encloses(&self, outer: CommitHandle, mut inner: CommitHandle) -> bool {
        loop {
            if inner == outer {
                return true;
            }
            match self.nest.get(&inner) {
                Some(Some(up)) => inner = *up,
                _ => return false,
            }
        }
    }
}
