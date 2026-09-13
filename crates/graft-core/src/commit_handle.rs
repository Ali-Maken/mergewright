//! Commit handles: a commit named by a small number instead of its id.
//!
//! A history-sized structure — the emitted order, the sets a walk keeps, the
//! promoted order — holds millions of entries. Holding a 33-byte `Oid` in each
//! is what made a million-commit history cost most of a gigabyte (ADR-0026).
//! A [`CommitHandle`] is a `u32` handed out by the one id table each open
//! repository has (`graft-git::history::CommitTable`); `git`, `graph` and `app`
//! all hold handles and ask that table for the id only when they need it.
//!
//! The shapes live here because `git` produces them and `graph` consumes them,
//! and those two may not depend on each other (DESIGN.md §8.2).
//!
//! [`HandleSet`] is the one container in this module. ADR-0007 keeps behaviour
//! out of `core`, and a bitset is no algorithm: it is the storage shape a set of
//! dense handles has, with nothing to decide — the same standing as `Vec`. It
//! lives beside the handle so that `git` and `graph` do not each grow a copy.

/// A commit, named by its place in the open repository's id table.
///
/// Dense: handles run from 0 with no gaps, so a structure indexed by handle is
/// a plain `Vec`. Stable for the life of the process: the table never renumbers
/// a commit. Meaningless outside the table that handed it out.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct CommitHandle(pub u32);

impl CommitHandle {
    /// The handle as an index into a `Vec` sized by handle.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A commit's parents, first parent first.
///
/// Almost every commit has one or two parents, so those cases are held inline:
/// a walk emitting a million links allocates only for octopus merges.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub enum Parents {
    /// A root commit. The default: no parents.
    #[default]
    None,
    /// An ordinary commit.
    One(CommitHandle),
    /// A merge of two lines.
    Two(CommitHandle, CommitHandle),
    /// An octopus merge: three parents or more.
    Many(Vec<CommitHandle>),
}

impl Parents {
    /// The parents in order, first parent first.
    pub fn iter(&self) -> impl Iterator<Item = CommitHandle> + '_ {
        let (inline, many): ([Option<CommitHandle>; 2], &[CommitHandle]) = match self {
            Parents::None => ([None, None], &[]),
            Parents::One(one) => ([Some(*one), None], &[]),
            Parents::Two(first, second) => ([Some(*first), Some(*second)], &[]),
            Parents::Many(many) => ([None, None], many),
        };
        inline.into_iter().flatten().chain(many.iter().copied())
    }

    /// How many parents there are.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Parents::None => 0,
            Parents::One(_) => 1,
            Parents::Two(..) => 2,
            Parents::Many(many) => many.len(),
        }
    }

    /// Whether this is a root commit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The first parent, if any.
    #[must_use]
    pub fn first(&self) -> Option<CommitHandle> {
        match self {
            Parents::None => None,
            Parents::One(first) | Parents::Two(first, _) => Some(*first),
            Parents::Many(many) => many.first().copied(),
        }
    }
}

impl FromIterator<CommitHandle> for Parents {
    fn from_iter<I: IntoIterator<Item = CommitHandle>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        let Some(first) = iter.next() else {
            return Parents::None;
        };
        let Some(second) = iter.next() else {
            return Parents::One(first);
        };
        let Some(third) = iter.next() else {
            return Parents::Two(first, second);
        };
        let mut many = vec![first, second, third];
        many.extend(iter);
        Parents::Many(many)
    }
}

/// One commit and its parents, by handle: everything the layout needs to place
/// a row, and nothing it needs to draw one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Link {
    /// The commit.
    pub commit: CommitHandle,
    /// Its parents, first parent first.
    pub parents: Parents,
}

/// A set of handles, one bit per handle.
///
/// Handles are dense, so a bitset over them costs an eighth of a byte per
/// commit in the repository — a million-commit set is 125 KiB — where a hash
/// set of the same handles would be tens of bytes per member.
#[derive(Clone, Default, Debug)]
pub struct HandleSet {
    /// Bit `h % 64` of word `h / 64` is set when handle `h` is a member.
    words: Vec<u64>,
    /// How many bits are set, kept so `len` is not a scan.
    len: usize,
}

impl HandleSet {
    /// An empty set. Allocates nothing until the first insert.
    #[must_use]
    pub fn new() -> Self {
        HandleSet::default()
    }

    /// Adds `handle`, growing the set as needed. `true` if it was not already
    /// a member.
    pub fn insert(&mut self, handle: CommitHandle) -> bool {
        let (word, bit) = Self::position(handle);
        if word >= self.words.len() {
            self.words.resize(word + 1, 0);
        }
        let fresh = self.words[word] & bit == 0;
        self.words[word] |= bit;
        if fresh {
            self.len += 1;
        }
        fresh
    }

    /// Whether `handle` is a member.
    #[must_use]
    pub fn contains(&self, handle: CommitHandle) -> bool {
        let (word, bit) = Self::position(handle);
        self.words.get(word).is_some_and(|w| w & bit != 0)
    }

    /// Removes `handle`. `true` if it was a member.
    pub fn remove(&mut self, handle: CommitHandle) -> bool {
        let (word, bit) = Self::position(handle);
        let Some(w) = self.words.get_mut(word) else {
            return false;
        };
        let present = *w & bit != 0;
        *w &= !bit;
        if present {
            self.len -= 1;
        }
        present
    }

    /// How many handles are members.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether no handle is a member.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Heap bytes the set holds, capacity included.
    #[must_use]
    pub fn approximate_bytes(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>()
    }

    /// The word a handle's bit lives in, and the bit within it.
    fn position(handle: CommitHandle) -> (usize, u64) {
        (handle.index() / 64, 1 << (handle.0 % 64))
    }
}

#[cfg(test)]
mod tests;
