//! A commit as the rest of Graft sees it.
//!
//! `git` reads these out of a repository, `graph` lays them out into rows,
//! `ops` records them in the journal and `ui` draws them. Four crates that may
//! not depend on each other, so the shape lives here (DESIGN.md §3.1, §8.2).
//!
//! Everything repository-controlled is bytes, not `String`. A commit message,
//! an author name and an email are whatever the committer's editor wrote; git
//! records an `encoding` header when it is not UTF-8 and enforces nothing at
//! all when it is missing. Decoding here would mangle those repositories at
//! the boundary, where nothing can recover them. Turning bytes into text is
//! the display layer's job and goes through `graft_term`'s sanitiser
//! (PROMPT.md §6.3).

use crate::oid::Oid;

/// When something happened, as git stores it.
///
/// Git writes two numbers: seconds since the Unix epoch, which is absolute,
/// and the author's UTC offset at the time, which is the only record of what
/// the clock on their wall said. Keeping both means a row can be drawn in
/// either, and the offset is never reconstructed from the reader's own zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    /// Seconds since the Unix epoch, UTC.
    pub seconds: i64,
    /// The recorded UTC offset, in seconds east of Greenwich.
    pub offset_seconds: i32,
}

/// A name, an email and a moment: one side of git's `author`/`committer` pair.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identity {
    /// The display name, exactly as the repository stores it.
    pub name: Vec<u8>,
    /// The email, without the angle brackets git wraps it in.
    pub email: Vec<u8>,
    /// When this side of the commit was recorded.
    pub when: Timestamp,
}

/// One commit object.
///
/// `parents` is in git's own order, so `parents[0]` is the first parent — the
/// distinction the whole first-parent view rests on. A root commit has none,
/// and an octopus merge has more than two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// This commit's object id.
    pub id: Oid,
    /// The parents, first parent first.
    pub parents: Vec<Oid>,
    /// Who wrote the change.
    pub author: Identity,
    /// Who recorded it. A rebase or an amend moves the commit and leaves the
    /// author alone, so this is the one that orders history.
    pub committer: Identity,
    /// The full message, subject line and body together, as stored.
    pub message: Vec<u8>,
}

impl Commit {
    /// The date history is ordered by: the committer's, not the author's.
    ///
    /// `git log --date-order` and the graph both use the committer date,
    /// because that is the one that moves when a commit is rebased. Ordering
    /// by the author date would draw a rebased branch in the shape it had
    /// before the rebase.
    #[must_use]
    pub fn date(&self) -> Timestamp {
        self.committer.when
    }

    /// The first line of the message, with no trailing newline: what a graph
    /// row shows.
    ///
    /// Still bytes, and still needs sanitising before it reaches a terminal.
    #[must_use]
    pub fn subject(&self) -> &[u8] {
        let end = self
            .message
            .iter()
            .position(|&byte| byte == b'\n')
            .unwrap_or(self.message.len());
        &self.message[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(seconds: i64) -> Identity {
        Identity {
            name: b"Ada".to_vec(),
            email: b"ada@example.invalid".to_vec(),
            when: Timestamp {
                seconds,
                offset_seconds: 0,
            },
        }
    }

    fn commit(message: &[u8]) -> Commit {
        Commit {
            id: Oid::from_hex("0123456789abcdef0123456789abcdef01234567").unwrap(),
            parents: Vec::new(),
            author: identity(100),
            committer: identity(200),
            message: message.to_vec(),
        }
    }

    #[test]
    fn the_date_is_the_committer_date() {
        assert_eq!(commit(b"x").date().seconds, 200);
    }

    #[test]
    fn the_subject_stops_at_the_first_newline() {
        assert_eq!(commit(b"Fix it\n\nBecause.\n").subject(), b"Fix it");
    }

    #[test]
    fn a_one_line_message_is_all_subject() {
        assert_eq!(commit(b"Fix it").subject(), b"Fix it");
    }

    #[test]
    fn an_empty_message_has_an_empty_subject() {
        assert_eq!(commit(b"").subject(), b"");
    }

    #[test]
    fn a_message_that_is_not_utf8_survives_intact() {
        assert_eq!(commit(b"caf\xe9 fix\nbody").subject(), b"caf\xe9 fix");
    }
}
