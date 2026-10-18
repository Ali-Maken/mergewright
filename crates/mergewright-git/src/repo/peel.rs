//! Peeling: turning a ref or a tag chain into the commit it reaches.
//! The private half [`super::refs`]'s public queries call into.

use mergewright_core::oid::Oid;

use super::{KnownCommits, KnownTips, RepoHandle, TipStats, TipsError};

impl RepoHandle {
    /// The commit `reference` reaches, or `None` if it reaches none.
    ///
    /// Big repositories hold tens of thousands of refs naming far fewer
    /// commits — a branch and its remote-tracking ref usually agree — so each
    /// object is judged once per pass, from `peeled_cache`. A packed tag's
    /// `packed-refs` peeled line (`^<id>`) already names what it peels to, so
    /// the tag object is never read (m1-the-graph/58b). Symbolic refs go
    /// through [`peel_commit`](Self::peel_commit).
    pub(super) fn tip_of(
        &self,
        reference: &mut gix::Reference<'_>,
        peeled_cache: &mut KnownTips,
        known_commits: &dyn KnownCommits,
        stats: &mut TipStats,
    ) -> Result<Option<Oid>, TipsError> {
        let gix::refs::TargetRef::Object(target) = reference.target() else {
            return self.peel_commit(reference);
        };
        let target = target.to_owned();
        if let Some(tip) = peeled_cache.get(&target) {
            return Ok(*tip);
        }
        // git writes a peeled line only for a ref naming a tag object; the
        // comparison guards a writer that repeats the target.
        let tip = match reference.inner.peeled.filter(|peeled| *peeled != target) {
            Some(peeled) => {
                stats.tags += 1;
                stats.peeled_packed += 1;
                self.commit_at(peeled, peeled_cache, known_commits, stats)?
            }
            None => self.commit_at(target, peeled_cache, known_commits, stats)?,
        };
        peeled_cache.insert(target, tip);
        Ok(tip)
    }

    /// The commit object `id` reaches through any chain of tags, or `None`
    /// if the chain ends on a tree or a blob.
    ///
    /// Each id is asked of `known_commits` first: one it recognises is a
    /// commit, no object read at all (ADR-0026, m1-the-graph/58). Otherwise
    /// the object is read once, whole — the decode says its kind and, for a
    /// tag, its target, which is judged the same way. Where the table does
    /// not [cover the graph](KnownCommits::covers_the_graph), most unknown ids
    /// are commits, so the header is read first and a commit ends there
    /// without its body: 30d measured reading every tip whole as 3/4 of the
    /// graph's first paint on 50k refs.
    fn commit_at(
        &self,
        mut id: gix::ObjectId,
        peeled_cache: &mut KnownTips,
        known_commits: &dyn KnownCommits,
        stats: &mut TipStats,
    ) -> Result<Option<Oid>, TipsError> {
        let mut chain = Vec::new();
        let tip = loop {
            if let Some(tip) = peeled_cache.get(&id) {
                break *tip;
            }
            let oid = self.oid(id.as_bytes())?;
            if known_commits.is_known_commit(oid) {
                stats.known += 1;
                break Some(oid);
            }
            chain.push(id);
            if !known_commits.covers_the_graph() {
                stats.find_header += 1;
                // A header that cannot be read is not answered here: the
                // whole read below reports why.
                if let Ok(header) = self.repo.find_header(id)
                    && header.kind() == gix::object::Kind::Commit
                {
                    break Some(oid);
                }
            }
            stats.objects_read += 1;
            let object = self.repo.find_object(id).map_err(|why| TipsError::Read {
                git_dir: self.git_dir().to_path_buf(),
                source: Box::new(why),
            })?;
            match object.kind {
                gix::object::Kind::Commit => break Some(oid),
                gix::object::Kind::Tag => {
                    stats.tags += 1;
                    id = object
                        .to_tag_ref_iter()
                        .target_id()
                        .map_err(|why| TipsError::Read {
                            git_dir: self.git_dir().to_path_buf(),
                            source: Box::new(why),
                        })?;
                }
                // A tag on a tree or a blob: legal, and not a tip.
                gix::object::Kind::Tree | gix::object::Kind::Blob => break None,
            }
        };
        for judged in chain {
            peeled_cache.insert(judged, tip);
        }
        Ok(tip)
    }

    /// Peels one ref all the way to a commit, reading objects as it goes;
    /// `None` for a ref that names no commit at all.
    fn peel_commit(&self, reference: &mut gix::Reference<'_>) -> Result<Option<Oid>, TipsError> {
        match reference.peel_to_commit() {
            Ok(commit) => Ok(Some(self.oid(commit.id().as_bytes())?)),
            // A tag on a tree or a blob, and `refs/notes/*`, peel to
            // something that is not a commit. Legal, and not a tip: `gix`
            // reports the object it stopped on, which is how "this ref has
            // no commit" is told apart from "this ref cannot be read".
            Err(gix::reference::peel::to_kind::Error::PeelObject(
                gix::object::peel::to_kind::Error::NotFound { .. },
            )) => Ok(None),
            Err(other) => Err(TipsError::Read {
                git_dir: self.git_dir().to_path_buf(),
                source: Box::new(other),
            }),
        }
    }

    /// An object id from raw bytes, blaming this repository if it is the
    /// wrong length.
    pub(super) fn oid(&self, bytes: &[u8]) -> Result<Oid, TipsError> {
        Oid::from_bytes(bytes).map_err(|source| TipsError::Malformed {
            git_dir: self.git_dir().to_path_buf(),
            source,
        })
    }
}
