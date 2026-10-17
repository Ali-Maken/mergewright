//! **The pick stack** (ease-pass 05): every take a keypress landed, newest
//! last, so `u` in the resolver puts the last one back — exactly, byte for
//! byte — and the one before it next.
//!
//! A pick is a file write inside the live operation, never a journal entry
//! (keymap §2.6a): the stack lives in memory only, beside the operation's
//! state ([`AppState::picks`](crate::update::AppState::picks)), not inside
//! the resolver's, so closing the resolver keeps it. What makes a pick safe
//! is the index, not a backup: until a file is staged, git's conflict stages
//! for it stay in the index, so the conflict can always be rebuilt (`d`).
//!
//! Each pick carries the document its write left (`after`): `u` is applied
//! to that document and written against its fingerprint, so a file switch
//! or a closed resolver — whose re-read collapses every taken block and
//! re-indexes the rest — never moves a pick onto the wrong block.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::conflict::{BlockState, ConflictFile};
use graft_ops::conflict::Restore;

/// One keypress's worth of takes: `o`/`t`/`b` one block; `O`/`T`,
/// `⌃O`/`⌃T` every block it changed — so one `u` undoes the whole keypress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    /// The conflicted file, as git's porcelain names it.
    pub path: PathBuf,
    /// Every block the keypress changed, each as it was before.
    pub blocks: Vec<Restore>,
    /// The document as this pick's write left it: what the untake is
    /// applied to, and its fingerprint what the bytes on disk must still be.
    pub after: Arc<ConflictFile>,
}

impl Pick {
    /// The pick that took `landed` from `was` (the document the take was
    /// applied to): every block whose state differs, with its state in
    /// `was`. `None` when nothing changed — a re-take of the same side.
    #[must_use]
    pub fn between(path: &Path, was: &ConflictFile, landed: Arc<ConflictFile>) -> Option<Self> {
        let blocks: Vec<Restore> = was
            .blocks()
            .into_iter()
            .zip(landed.blocks())
            .filter(|(before, now)| before.state != now.state)
            .map(|(before, _)| Restore {
                block: before.index,
                before: match before.state {
                    BlockState::Open => None,
                    BlockState::Took(take) => Some(take),
                },
            })
            .collect();
        (!blocks.is_empty()).then(|| Pick {
            path: path.to_path_buf(),
            blocks,
            after: landed,
        })
    }

    /// The block the cursor lands on once this pick is undone: its first.
    #[must_use]
    pub fn first_block(&self) -> Option<usize> {
        self.blocks.iter().map(|restore| restore.block).min()
    }
}

/// The operation's picks, oldest first, and the untake `u` has out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picks {
    stack: Vec<Pick>,
    /// The untake in flight and the pick it puts back: off the stack while
    /// it runs, back on top only if it could not write (an I/O failure).
    undoing: Option<(JobId, Pick)>,
    /// Where an untake that landed on another file puts the cursor once
    /// that file's read lands: the document it left, and the block.
    landing: Option<Landing>,
}

/// An untake's landing on a file the resolver was not showing.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Landing {
    path: PathBuf,
    file: Arc<ConflictFile>,
    block: Option<usize>,
}

impl Picks {
    /// A pick landed.
    pub fn push(&mut self, pick: Pick) {
        self.stack.push(pick);
    }

    /// The last pick, off the stack.
    pub fn pop(&mut self) -> Option<Pick> {
        self.stack.pop()
    }

    /// The last pick.
    #[must_use]
    pub fn last(&self) -> Option<&Pick> {
        self.stack.last()
    }

    /// No pick to undo.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// How many picks `path` has on the stack.
    #[must_use]
    pub fn in_file(&self, path: &Path) -> usize {
        self.stack.iter().filter(|pick| pick.path == path).count()
    }

    /// Drops `path`'s picks: the file was staged, edited by hand, or changed
    /// outside. The picks in other files keep their order.
    pub fn seal(&mut self, path: &Path) {
        self.stack.retain(|pick| pick.path != path);
        if self.landing.as_ref().is_some_and(|land| land.path == path) {
            self.landing = None;
        }
    }

    /// Drops every pick: the operation finished, aborted or went on to its
    /// next stop, or the stash clash closed.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.landing = None;
    }

    /// What a fresh read of `path` means for its picks (ticket ease-pass/05r,
    /// review ease-pass-05 M1 part 2 and M2). `read`: the document read,
    /// `None` when the file is no marker document at all (a whole-file card).
    ///
    /// The bytes still the ones the newest pick in the file wrote → that
    /// pick's `after`, so the taken clashes stay on screen (the read itself
    /// has them collapsed into plain text). Other bytes → someone else wrote
    /// the file since, every pick in it was chosen against a document that
    /// is gone, and they are dropped — said by the caller, never kept: an
    /// untake over changed bytes would write over his edit.
    pub fn reconcile(&mut self, path: &Path, read: Option<&ConflictFile>) -> Reread {
        let Some(newest) = self.stack.iter().rev().find(|pick| pick.path == path) else {
            return Reread::NoPicks;
        };
        match read {
            Some(read) if read.fingerprint == newest.after.fingerprint => {
                Reread::Picked(Arc::clone(&newest.after))
            }
            _ => {
                let count = self.in_file(path);
                self.seal(path);
                Reread::Dropped { count }
            }
        }
    }

    /// The untake in flight, if any.
    #[must_use]
    pub fn undoing(&self) -> Option<JobId> {
        self.undoing.as_ref().map(|(id, _)| *id)
    }

    /// Pops the last pick into the untake `id` sends; `None` with nothing
    /// to undo, or with an untake already out.
    pub fn start_undo(&mut self, id: JobId) -> Option<Pick> {
        if self.undoing.is_some() {
            return None;
        }
        let pick = self.stack.pop()?;
        self.undoing = Some((id, pick.clone()));
        Some(pick)
    }

    /// The untake `id` answered: its pick, or `None` when `id` is not the
    /// one out.
    pub fn finish_undo(&mut self, id: JobId) -> Option<Pick> {
        self.undoing
            .take_if(|(out, _)| *out == id)
            .map(|(_, pick)| pick)
    }

    /// An untake that wrote nothing and was not stale: its pick goes back on
    /// top, where it was — nothing could land a pick while it was out.
    pub fn put_back(&mut self, pick: Pick) {
        self.stack.push(pick);
    }

    /// Remember where the cursor goes once `path`'s read lands.
    pub fn land_after_read(&mut self, path: &Path, file: Arc<ConflictFile>, block: Option<usize>) {
        self.landing = Some(Landing {
            path: path.to_path_buf(),
            file,
            block,
        });
    }

    /// The landing waiting on `path`'s read, spent: the document the untake
    /// left and the cursor's block — `None` when there is none, or when the
    /// read found other bytes than the untake wrote.
    pub fn take_landing(
        &mut self,
        path: &Path,
        on_disk: Oid,
    ) -> Option<(Arc<ConflictFile>, Option<usize>)> {
        let landing = self.landing.take_if(|land| land.path == path)?;
        (landing.file.fingerprint == on_disk).then_some((landing.file, landing.block))
    }
}

/// What a fresh read of a file means for its picks ([`Picks::reconcile`]).
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reread {
    /// No pick in this file: show the read.
    NoPicks,
    /// Bytes equal the newest pick's `after`: show that document, taken
    /// clashes kept.
    Picked(Arc<ConflictFile>),
    /// Bytes differ: the file's `count` picks were dropped; say so
    /// ([`dropped_words`](super::dropped_words)).
    Dropped {
        /// How many picks went.
        count: usize,
    },
}

/// One block as `n` and the advance see it: its index, and whether it is
/// still open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockAt {
    /// The block's index, as `ConflictFile::take` counts.
    pub index: usize,
    /// No side taken yet.
    pub open: bool,
}

/// The blocks of `file`, as [`BlockAt`]s.
#[must_use]
pub fn blocks_of(file: &ConflictFile) -> Vec<BlockAt> {
    file.blocks()
        .into_iter()
        .map(|at| BlockAt {
            index: at.index,
            open: at.state == BlockState::Open,
        })
        .collect()
}

/// First open block, top-down — where a take moves the cursor, so a file is
/// solved in order whichever block was taken last (the mock's
/// `C.findIndex(c => !c.pick && c.file === f)`).
#[must_use]
pub fn next_open_in_file(blocks: &[BlockAt]) -> Option<usize> {
    blocks.iter().find(|at| at.open).map(|at| at.index)
}

/// (file index, block index) of the first open block in list order — `n`,
/// from the top of the whole operation (the mock's `nextOpen`).
#[must_use]
pub fn next_open(files: &[(PathBuf, Vec<BlockAt>)]) -> Option<(usize, usize)> {
    files
        .iter()
        .enumerate()
        .find_map(|(file, (_, blocks))| next_open_in_file(blocks).map(|block| (file, block)))
}

#[cfg(test)]
mod tests;
