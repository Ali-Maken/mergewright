//! Which live branch a name means (ease-pass/07b): the index `badges`
//! builds once per refs read, so the bar's `C switch to <b>` never scans
//! the refs per frame.

use std::collections::{HashMap, HashSet};

use mergewright_core::oid::Oid;

use super::{RefBadges, RefLabel};

/// Every live branch by name: local branches by their short name, remote-
/// tracking branches by their shorthand (`origin/main`) and by the name
/// after their remote (`main`, the first remote listed wins).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct LiveNames {
    /// Local short name or remote shorthand → its label.
    by_name: HashMap<Vec<u8>, RefLabel>,
    /// The name after a remote (`fix-toc` of `origin/fix-toc`) → that
    /// remote shorthand.
    by_label: HashMap<Vec<u8>, Vec<u8>>,
    /// Remote shorthands some local branch has as its upstream.
    tracked: HashSet<Vec<u8>>,
}

impl LiveNames {
    /// Notes a local branch's label.
    pub(super) fn local(&mut self, label: &RefLabel) {
        if let RefLabel::Branch { name, .. } = label {
            self.by_name.insert(name.clone(), label.clone());
        }
    }

    /// Notes that a local branch's upstream is the remote-tracking branch
    /// of shorthand `short`.
    pub(super) fn tracks(&mut self, short: &[u8]) {
        self.tracked.insert(short.to_vec());
    }

    /// Notes a remote-tracking branch, `short` its shorthand and `remote`
    /// the remote it belongs to.
    pub(super) fn remote(&mut self, short: &[u8], remote: &[u8]) {
        self.by_name
            .entry(short.to_vec())
            .or_insert_with(|| RefLabel::Remote(short.to_vec()));
        if let Some(label) = short
            .strip_prefix(remote)
            .and_then(|rest| rest.strip_prefix(b"/"))
        {
            self.by_label
                .entry(label.to_vec())
                .or_insert_with(|| short.to_vec());
        }
    }
}

impl RefBadges {
    /// The live branch `label` names (a merged branch's name from a merge
    /// subject, or a lane's name): the local branch of exactly that name,
    /// else the remote-tracking branch of exactly that shorthand, else a
    /// remote-tracking `<remote>/<label>`. `None` when no such branch lives
    /// (a deleted PR branch): the bar then offers no `C switch to`.
    #[must_use]
    pub(crate) fn live_branch(&self, label: &[u8]) -> Option<RefLabel> {
        if let Some(found) = self.live.by_name.get(label) {
            return Some(found.clone());
        }
        let short = self.live.by_label.get(label)?;
        Some(RefLabel::Remote(short.clone()))
    }

    /// Every remote-tracking branch with a badge of its own that no local
    /// branch has as its upstream, as its tip and shorthand (`origin/x`):
    /// a branch that lives only on a remote, a teammate's he never checked
    /// out (ease-pass 09 ruling R). Never `<remote>/HEAD`. Order is not
    /// meaningful.
    pub fn remote_only(&self) -> impl Iterator<Item = (Oid, &[u8])> {
        self.remotes()
            .filter(|(_, short)| !self.live.tracked.contains(*short))
    }
}
