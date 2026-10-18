//! Where one conflicted file stands, by the resolver's rule (ease-pass/05b,
//! hard choice 3): **solved is the resolver's classification, never "no
//! markers"**. A binary or delete/modify conflict has no markers and is not
//! solved; a marker document with every clash picked is, though git still
//! calls it unmerged until the merge's `y` stages it.

use super::Opened;

/// One conflicted path's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileState {
    /// Something is still to choose: a marker document with `open` clashes
    /// left, or a whole-file card (`Opened.file == None`: a delete/modify,
    /// add/add of a binary, …), whose `open` is 0 — still open.
    Open {
        /// Clashes still open; 0 for a whole-file card.
        open: usize,
    },
    /// Unmerged, a marker document, no clash open: every one picked, or the
    /// markers edited away by hand. Staged at the merge's `y`.
    Solved,
    /// git no longer calls it unmerged: staged (a whole-file choice, a
    /// stage from the shell).
    Done,
}

impl FileState {
    /// Open clashes this file counts for; a whole-file card (`Open { open:
    /// 0 }`) is one. The one count the resolver's words and the review's
    /// refusal both say.
    #[must_use]
    pub fn open_clashes(self) -> usize {
        match self {
            Self::Open { open } => open.max(1),
            Self::Solved | Self::Done => 0,
        }
    }
}

/// `opened`'s state, `unmerged` being whether git's status still calls the
/// path unmerged. The one rule the resolver, the review and a relaunch read.
#[must_use]
pub fn file_state(opened: &Opened, unmerged: bool) -> FileState {
    if !unmerged {
        return FileState::Done;
    }
    match &opened.file {
        Some(file) if file.open() == 0 => FileState::Solved,
        Some(file) => FileState::Open { open: file.open() },
        None => FileState::Open { open: 0 },
    }
}

#[cfg(test)]
mod tests;
