//! Ref-reading queries: every ref, tips, upstreams — the public half of
//! reading refs. Peeling internals live in [`super::peel`].

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;

use super::{
    KnownCommits, KnownTips, RefEntry, RefSnapshot, RefTarget, RefsError, RepoHandle, TipStats,
    TipsError,
};

impl RepoHandle {
    /// Every ref in the repository, sorted by full name.
    ///
    /// Branches, tags, remote-tracking branches, `refs/stash` and anything
    /// else the ref store holds — but not `HEAD`, which is
    /// [`head`](Self::head)'s job because its three states are not a ref's
    /// three states.
    ///
    /// An annotated tag's target is the *tag object*, not the commit it
    /// points at: peeling costs an object read per tag, and the caller that
    /// needs the commit is the one that should pay for it.
    ///
    /// *Every* ref means Mergewright's own too — `refs/graft/**` is in here. That
    /// is what this method is for, and it is why nothing drawn on screen
    /// should be built from it: use [`ref_snapshot`](Self::ref_snapshot),
    /// which leaves Mergewright's bookkeeping out.
    ///
    /// # Errors
    /// [`RefsError`] if the ref store cannot be read, or holds a name or an
    /// id that git's own rules reject.
    pub fn references(&self) -> Result<Vec<RefEntry>, RefsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| RefsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let malformed = |source| RefsError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };

        let platform = self.repo.references().map_err(|why| read(Box::new(why)))?;
        let iter = platform.all().map_err(|why| read(Box::new(why)))?;
        let mut found = Vec::new();
        for reference in iter {
            let reference = reference.map_err(read)?;
            let target = match reference.target() {
                gix::refs::TargetRef::Object(id) => {
                    RefTarget::Object(Oid::from_bytes(id.as_bytes()).map_err(malformed)?)
                }
                gix::refs::TargetRef::Symbolic(name) => {
                    RefTarget::Symbolic(RefName::new(name.as_bstr().to_vec()).map_err(malformed)?)
                }
            };
            let name = RefName::new(reference.name().as_bstr().to_vec()).map_err(malformed)?;
            found.push(RefEntry { name, target });
        }
        found.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(found)
    }

    /// One ref by full name, as the store holds it: `None` when it is absent.
    ///
    /// Where [`references`](Self::references) reads the whole store to answer
    /// about anything, this answers about one name — a loose-ref file open, or
    /// a lookup in `packed-refs`. The journal's protocol asks about one or two
    /// names several times per operation, and on a repository with 50k refs a
    /// full pass to answer that is the bookkeeping outweighing the work
    /// (m3/03 review, M6).
    ///
    /// The target is **not** peeled and a symbolic ref is **not** followed:
    /// both are the caller's choice, as in `references`.
    ///
    /// # Errors
    /// [`RefsError`] if the ref store cannot be read, or holds a name or an id
    /// that git's own rules reject.
    pub fn reference(&self, name: &RefName) -> Result<Option<RefTarget>, RefsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| RefsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let malformed = |source| RefsError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let partial = gix::refs::PartialName::try_from(gix::bstr::BString::from(name.as_bytes()))
            .map_err(|why| read(Box::new(why)))?;
        let found = self
            .repo
            .try_find_reference(&partial)
            .map_err(|why| read(Box::new(why)))?;
        let Some(reference) = found else {
            return Ok(None);
        };
        Ok(Some(match reference.target() {
            gix::refs::TargetRef::Object(id) => {
                RefTarget::Object(Oid::from_bytes(id.as_bytes()).map_err(malformed)?)
            }
            gix::refs::TargetRef::Symbolic(target) => {
                RefTarget::Symbolic(RefName::new(target.as_bstr().to_vec()).map_err(malformed)?)
            }
        }))
    }

    /// Every commit a ref reaches, `HEAD` included: where a walk starts.
    ///
    /// Annotated tags are peeled, which costs an object read each, and that
    /// is the point — a walk seeded from the tag object rather than the
    /// commit it names would find nothing.
    ///
    /// A ref that does not lead to a commit is skipped, not an error: a tag
    /// on a blob and `refs/notes/commits` are both legal, and neither has a
    /// place in a commit graph. A ref that cannot be *read* is an error, and
    /// is returned as one.
    ///
    /// The result is sorted and free of duplicates, so two refs at the same
    /// commit seed the walk once.
    ///
    /// # Errors
    /// [`TipsError`] if the ref store cannot be read, or a ref cannot be
    /// peeled for a reason other than not naming a commit.
    pub fn commit_tips(&self, known: &dyn KnownCommits) -> Result<Vec<Oid>, TipsError> {
        Ok(self.ref_snapshot(known)?.tips)
    }

    /// [`references`](Self::references), [`commit_tips`](Self::commit_tips)
    /// and [`peeled_references`](Self::peeled_references) from one pass over the ref store.
    ///
    /// Reading refs is one file per loose ref (~11 µs each): at 6,500 loose
    /// refs a pass costs ~70 ms, and first paint used to pay it four times
    /// (m1-the-graph/55). Whoever needs both the refs and the tips reads them
    /// here, once. Each half is exactly what its own method returns.
    ///
    /// `known` says which ids are already known to be commits (ADR-0026's
    /// commit-graph table): a tip it recognises costs no object read at all.
    /// Pass `&()` where nothing already knows — every tip is header-parsed as
    /// before (m1-the-graph/58).
    ///
    /// **Mergewright's own refs are left out.** This is the repository as Mergewright
    /// *draws* it, and `refs/graft/**` — the undo journal's backup chains —
    /// is bookkeeping the user never made
    /// ([`RefName::is_mergewright_internal`]). Seeding the walk from them put two
    /// synthetic commits above `HEAD` per undoable write, forever
    /// (m3-07 review, C1). Whoever wants the ref store as it really is calls
    /// [`references`](Self::references), which still answers for every ref;
    /// undo and the journal read their backups by exact name through
    /// [`reference`](Self::reference), which is untouched.
    ///
    /// # Errors
    /// [`TipsError`] if the ref store cannot be read, holds a name or an id
    /// git's own rules reject, or a ref cannot be peeled for a reason other
    /// than not naming a commit.
    pub fn ref_snapshot(&self, known: &dyn KnownCommits) -> Result<RefSnapshot, TipsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| TipsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let malformed = |source| TipsError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };

        let platform = self.repo.references().map_err(|why| read(Box::new(why)))?;
        let iter = platform.all().map_err(|why| read(Box::new(why)))?;

        let mut refs = Vec::new();
        let mut tips = Vec::new();
        let mut peeled = Vec::new();
        let mut peeled_cache = KnownTips::new();
        let mut stats = TipStats::default();
        for reference in iter {
            let mut reference = reference.map_err(read)?;
            let name = RefName::new(reference.name().as_bstr().to_vec()).map_err(malformed)?;
            // Before any work: peeling a backup chain costs object reads for
            // a row that must never be drawn.
            if name.is_mergewright_internal() {
                continue;
            }
            stats.refs += 1;
            let target = match reference.target() {
                gix::refs::TargetRef::Object(id) => RefTarget::Object(self.oid(id.as_bytes())?),
                gix::refs::TargetRef::Symbolic(name) => {
                    RefTarget::Symbolic(RefName::new(name.as_bstr().to_vec()).map_err(malformed)?)
                }
            };
            let tip = self.tip_of(&mut reference, &mut peeled_cache, known, &mut stats)?;
            if let Some(id) = tip {
                tips.push(id);
                peeled.push((name.clone(), id));
            }
            refs.push(RefEntry { name, target });
        }
        // `HEAD` is not in the ref store's iteration, and on a detached
        // `HEAD` it is the only thing holding its commit. An unborn `HEAD`
        // peels to nothing and adds no tip, which is the right answer.
        let mut head = self.repo.head().map_err(|why| read(Box::new(why)))?;
        if let Some(id) = head.try_peel_to_id().map_err(|why| read(Box::new(why)))? {
            tips.push(self.oid(id.as_bytes())?);
        }

        refs.sort_by(|left, right| left.name.cmp(&right.name));
        peeled.sort_by(|left: &(RefName, Oid), right| left.0.cmp(&right.0));
        tips.sort_unstable();
        tips.dedup();
        Ok(RefSnapshot {
            refs,
            tips,
            peeled,
            stats,
        })
    }

    /// Every ref that reaches a commit, with the commit it peels to, sorted
    /// by full name.
    ///
    /// Where [`references`](Self::references) leaves an annotated tag
    /// pointing at its tag object, this follows it to the commit — what a
    /// ref badge is drawn on. Refs that peel to a tree or a blob, and
    /// symbolic refs whose target is gone, are left out: they sit on no
    /// commit.
    ///
    /// `known` is exactly [`ref_snapshot`](Self::ref_snapshot)'s: this never
    /// reads a tip's header itself, so it costs nothing extra to thread
    /// through.
    ///
    /// # Errors
    /// [`TipsError`] if the ref store cannot be read or a ref cannot be
    /// peeled.
    pub fn peeled_references(
        &self,
        known: &dyn KnownCommits,
    ) -> Result<Vec<(RefName, Oid)>, TipsError> {
        Ok(self.ref_snapshot(known)?.peeled)
    }

    /// The remote-tracking ref `branch` pulls from, as git's
    /// `branch.<name>.remote` and `branch.<name>.merge` settings and the
    /// remote's fetch refspecs name it — `refs/remotes/origin/main` for
    /// `refs/heads/main` set up the usual way. `None` when the branch has no
    /// upstream configured. The ref named may not exist.
    ///
    /// # Errors
    /// [`RefsError`] if the configuration names an upstream that cannot be
    /// resolved (an unknown remote, a malformed refspec).
    pub fn upstream(&self, branch: &RefName) -> Result<Option<RefName>, RefsError> {
        let read = |source: Box<dyn std::error::Error + Send + Sync>| RefsError::Read {
            git_dir: self.git_dir().to_path_buf(),
            source,
        };
        let full = gix::refs::FullName::try_from(gix::bstr::BString::from(branch.as_bytes()))
            .map_err(|why| read(Box::new(why)))?;
        match self
            .repo
            .branch_remote_tracking_ref_name(full.as_ref(), gix::remote::Direction::Fetch)
        {
            None => Ok(None),
            Some(Err(why)) => Err(read(Box::new(why))),
            Some(Ok(tracking)) => {
                RefName::new(tracking.as_bstr().to_vec())
                    .map(Some)
                    .map_err(|source| RefsError::Malformed {
                        git_dir: self.git_dir().to_path_buf(),
                        source,
                    })
            }
        }
    }
}
