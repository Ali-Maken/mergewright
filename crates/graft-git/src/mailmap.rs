//! Resolving a commit's author against the repository's `.mailmap`.
//!
//! A person can commit under several names and emails — a work address, a
//! personal one, a typo fixed halfway through a project's life — and
//! `.mailmap` is git's own record of which of those are the same person.
//! Graft's author chip (m1-the-graph/10) needs the canonical identity, not
//! whatever one commit happened to record, so two identities the file maps
//! together draw the same initials in the same colour.
//!
//! Reads go through `gix` and nothing else (ADR-0011): this is a thin wrapper
//! over [`gix::mailmap::Snapshot`], not a second `.mailmap` parser.

use gix::actor::SignatureRef;
use gix::bstr::BStr;
use graft_core::commit::Identity;

/// A repository's resolved `.mailmap`.
///
/// [`RepoHandle::mailmap`](crate::repo::RepoHandle::mailmap) merges every
/// source git itself would (the working tree's `.mailmap`, `mailmap.blob`,
/// `mailmap.file`) and never fails: a missing or broken `.mailmap` resolves
/// every identity to itself, exactly as git treats it as optional.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mailmap(gix::mailmap::Snapshot);

impl Mailmap {
    /// Wraps an already-loaded snapshot.
    #[must_use]
    pub(crate) fn from_snapshot(snapshot: gix::mailmap::Snapshot) -> Self {
        Mailmap(snapshot)
    }

    /// Parses `.mailmap` text directly, with no repository involved.
    ///
    /// For this crate's unit tests: building a [`Mailmap`] this way needs no
    /// temporary repository, just the file's own bytes.
    #[cfg(test)]
    #[must_use]
    pub fn from_bytes(text: &[u8]) -> Self {
        Mailmap(gix::mailmap::Snapshot::from_bytes(text))
    }

    /// The canonical name and email for `identity`, as `.mailmap` says.
    ///
    /// An identity `.mailmap` says nothing about resolves to itself: an
    /// unmapped author is not a failure, just a person with one identity.
    #[must_use]
    pub fn resolve(&self, identity: &Identity) -> (Vec<u8>, Vec<u8>) {
        let signature = SignatureRef {
            name: BStr::new(&identity.name),
            email: BStr::new(&identity.email),
            time: "",
        };
        let resolved = self.0.resolve(signature);
        (resolved.name.into(), resolved.email.into())
    }
}

#[cfg(test)]
mod tests;
