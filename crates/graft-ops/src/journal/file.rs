//! The journal file: open it, append to it, recover from a crash.
//!
//! Append-only JSONL under `<git-path graft>/undo-journal.jsonl`, which is
//! per-worktree by git's own layout rules (ADR-0010). Two lines per entry:
//! the `Pending` line goes down *before* the operation runs, the outcome
//! line after it, so a crash between them leaves a `Pending` entry that
//! [`Journal::unresolved`] finds on the next launch (DESIGN.md §6.2 step 3).
//!
//! Every append is `O_APPEND` plus `fsync`: the line is on disk before the
//! operation it describes starts. Nothing here spawns `git`.
//!
//! **Who may close a `Pending` entry.** Its own writer, with an outcome; or
//! recovery, as `interrupted`, once that writer is gone. Recovery tells the
//! two apart with an advisory lock on the journal file itself
//! ([`FileLock`]): a writer holds it shared from its first `Pending` line
//! (or, resuming, from before it touches git) until it drops the
//! [`Journal`]; recovery closes anything only while it holds it alone. The
//! kernel drops a dead process's lock, so a crashed run's entries are still
//! recovered, and two `Journal`s in one process contend exactly as two
//! processes do — `J`'s read job and a running operation's job included
//! (m8-polish-and-ship/58).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, EntryStatus, JournalEntry};
use graft_core::on_disk::Generation;
use graft_git::file_lock::FileLock;
use graft_git::repo::RepoHandle;
use graft_git::run::RunError;

use super::{JournalError, Parsed, parse, render_outcome, render_pending};
use crate::undo::RepoStateNow;

/// The journal file's name under the per-worktree names-in-force directory
/// ([`journal_path`]).
const FILE_NAME: &str = "undo-journal.jsonl";

// The reason recorded for an entry whose process died before it could write
// an outcome. One definition, in `core`, because every reader that tells the
// user "interrupted" rather than "failed" has to match on this exact word
// (m3/06 review, Major 2) — `JournalEntry::is_interrupted` is that reader.
use graft_core::journal::INTERRUPTED;

/// An open journal: the file's path and the entries read from it.
///
/// The entries are kept in memory so the undo/redo cursor (`next_undo`,
/// `next_redo`) can be read without re-parsing; every append updates both the
/// file and that copy, so they never disagree. The copy is read again from
/// the file whenever this `Journal` takes the lock (module docs): another
/// `Journal` may have appended between the open and the lock.
#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    entries: Vec<JournalEntry>,
    torn_tail: Option<usize>,
    /// The names in force the entries were read under ([`Journal::open_as`]),
    /// so a re-read maps them the same way.
    names: Option<Generation>,
    /// The writer's shared lock, held from [`Journal::claim`] until drop.
    writing: Option<FileLock>,
}

/// The journal held alone, by [`Journal::alone`]: no other `Journal`, in
/// this process or any other, is part-way through writing an entry while it
/// is held. Dropping it lets writers in again.
#[derive(Debug)]
pub struct Alone {
    _held: FileLock,
}

impl Journal {
    /// Open the journal at `path`, creating its directory and the file if
    /// they are missing.
    ///
    /// A torn last line — a crash part-way through an append — is **cut off
    /// the file here**, not just reported. It has to be: the torn bytes carry
    /// no complete record and have no trailing newline, so appending after
    /// them would splice the next line onto the wreckage, and every later
    /// parse would then see a torn line that is not last and refuse the whole
    /// file. [`Journal::torn_tail`] keeps the line number so a caller can say
    /// it happened.
    ///
    /// Cut only with the journal held alone (`cut_torn_tail`): while any
    /// writer holds it, the line may be that writer's own append, unfinished,
    /// and it is left exactly as a re-read under the lock leaves it, with
    /// [`Journal::torn_tail`] `None`.
    pub fn open(path: &Path) -> Result<Journal, JournalError> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                File::create(path)?;
                Vec::new()
            }
            Err(why) => return Err(why.into()),
        };
        let mut parsed = parse(&bytes)?;
        if parsed.torn_tail.is_some() {
            parsed = cut_torn_tail(path, parsed)?;
        }
        Ok(Journal {
            path: path.to_path_buf(),
            entries: parsed.entries,
            torn_tail: parsed.torn_tail,
            names: None,
            writing: None,
        })
    }

    /// [`open`](Journal::open), with every entry's ref names read as
    /// `names` spells them (`crate::rename::map_entry`): a journal written
    /// before the rename names its backups under the old prefix, and the
    /// refs are now under the new one. The file's bytes are never rewritten.
    /// Every production reader opens through here, with the repository's
    /// names in force.
    pub fn open_as(path: &Path, names: Generation) -> Result<Journal, JournalError> {
        let mut journal = Journal::open(path)?;
        crate::rename::map_entries(&mut journal.entries, names);
        journal.names = Some(names);
        Ok(journal)
    }

    /// Take the writer's lock, shared with other writers, before this
    /// `Journal` writes an entry's first line or acts on a `Pending` entry
    /// it did not open itself (a resume). Waits — a kernel wait, not a poll
    /// — while recovery holds the journal [`alone`](Journal::alone), then
    /// reads the file again, so the copy is current as of the lock: an entry
    /// recovery closed just before is seen closed, never closed twice.
    ///
    /// Held until this `Journal` drops. Taking it again is a no-op.
    pub fn claim(&mut self) -> Result<(), JournalError> {
        if self.writing.is_some() {
            return Ok(());
        }
        let held = FileLock::shared(&self.path)?;
        self.reread()?;
        self.writing = Some(held);
        Ok(())
    }

    /// This `Journal` holds the writer's lock ([`Journal::claim`]).
    pub(crate) fn claimed(&self) -> bool {
        self.writing.is_some()
    }

    /// The journal alone, or `None` while any other `Journal` holds the
    /// writer's lock — some operation, in this process or another, has an
    /// entry it is still writing. On success the copy is read again from the
    /// file, so it holds every outcome a writer appended before letting go,
    /// and a torn last line is cut as [`open`](Journal::open) cuts one: an
    /// `open` made while a writer held the journal left it, and recovery's
    /// outcome lines must not splice onto it (58r).
    ///
    /// [`JournalError::StillWriting`] when this `Journal` holds the writer's
    /// lock itself ([`Journal::claim`]): it may be part-way through an entry,
    /// and the lock is what keeps recovery off that entry, so it is kept and
    /// the request refused (review 58, Minor 1). Recovery opens a `Journal`
    /// of its own.
    pub fn alone(&mut self) -> Result<Option<Alone>, JournalError> {
        if self.writing.is_some() {
            return Err(JournalError::StillWriting);
        }
        let Some(held) = FileLock::try_exclusive(&self.path)? else {
            return Ok(None);
        };
        let parsed = read_cutting_torn_tail(&self.path)?;
        self.torn_tail = parsed.torn_tail.or(self.torn_tail);
        self.hold(parsed.entries);
        Ok(Some(Alone { _held: held }))
    }

    /// Replace the held copy with the file as it stands.
    ///
    /// Reads only — unlike [`open`](Journal::open) it never cuts a torn
    /// tail: under a shared lock another writer may be part-way through its
    /// own append, and that line is unfinished, not wreckage. `open` and
    /// [`alone`](Journal::alone) cut one a crash left, holding the journal
    /// alone.
    fn reread(&mut self) -> Result<(), JournalError> {
        let bytes = std::fs::read(&self.path)?;
        self.hold(parse(&bytes)?.entries);
        Ok(())
    }

    /// Keep `entries` as the held copy, mapped under the same names as at open.
    fn hold(&mut self, mut entries: Vec<JournalEntry>) {
        if let Some(names) = self.names {
            crate::rename::map_entries(&mut entries, names);
        }
        self.entries = entries;
    }

    /// The journal file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every entry, oldest first.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// The 1-based line number of a torn last line cut off at [`open`] (or
    /// later, by [`Journal::alone`]), if there was one. Nothing was lost by it — the record it held never
    /// finished being written — but the caller may want to say so.
    ///
    /// [`open`]: Journal::open
    pub fn torn_tail(&self) -> Option<usize> {
        self.torn_tail
    }

    /// Write an entry's `Pending` line and remember it.
    ///
    /// Returns `DuplicatePending` rather than writing a second opening line
    /// for an id the file already holds. Takes the writer's lock first
    /// ([`Journal::claim`]), so recovery leaves this entry alone until this
    /// `Journal` drops.
    pub fn append_pending(&mut self, entry: &JournalEntry) -> Result<(), JournalError> {
        self.claim()?;
        if self.entries.iter().any(|held| held.id == entry.id) {
            return Err(JournalError::DuplicatePending(entry.id.clone()));
        }
        let line = render_pending(entry)?;
        self.append(&line)?;
        self.entries.push(entry.clone());
        Ok(())
    }

    /// Write an entry's outcome line and replace the held copy with it.
    ///
    /// Refuses an entry still marked `Pending` (`render_outcome`'s own rule),
    /// an id the file never opened, and a second outcome for one id.
    pub(crate) fn append_outcome(&mut self, entry: &JournalEntry) -> Result<(), JournalError> {
        let position = self
            .entries
            .iter()
            .position(|held| held.id == entry.id)
            .ok_or_else(|| JournalError::OutcomeWithoutPending(entry.id.clone()))?;
        if self.entries[position].status != EntryStatus::Pending {
            return Err(JournalError::DuplicateOutcome(entry.id.clone()));
        }
        let line = render_outcome(entry)?;
        // The file first, the held copy second: a failed write must not leave
        // the two disagreeing.
        self.append(&line)?;
        self.entries[position] = entry.clone();
        Ok(())
    }

    /// Remember the command lines an entry has run so far, without writing
    /// anything.
    ///
    /// A history operation stopped on a conflict keeps its entry `Pending`,
    /// and only the outcome line carries `commands` — so between the stop
    /// and [`crate::apply::resume`] the lines have nowhere on disk to live.
    /// They are held here instead, and the outcome line `resume` finally
    /// writes carries the plan's lines and the resume's together. A relaunch
    /// while the conflict is open loses the plan's lines: the entry is still
    /// `Pending` in the file, and nothing may be appended to an entry that
    /// has not finished (m5/07).
    ///
    /// An id the journal does not hold is ignored: the caller is recording
    /// what an entry ran, not asserting the entry exists.
    pub fn hold_commands(&mut self, id: &EntryId, commands: Vec<String>) {
        if let Some(entry) = self.entries.iter_mut().find(|held| held.id == *id) {
            entry.commands = commands;
        }
    }

    /// Entries with no outcome: an operation whose process died between the
    /// two lines (DESIGN.md §6.2 step 3).
    pub fn unresolved(&self) -> Vec<&JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.status == EntryStatus::Pending)
            .collect()
    }

    /// Close an unresolved entry as `Aborted { reason: "interrupted" }`.
    ///
    /// `after` is the repository as it stands now, read by the caller (the
    /// journal has no repository of its own). It matters: an entry whose
    /// after-fields are all `None` is in force with an `Incomplete` inverse
    /// for its refs and an `Everything`-scoped restore for its trees, so `u`
    /// would either refuse or offer to overwrite the whole working tree.
    /// Pass `None` only when the repository genuinely could not be read —
    /// that is the case `PathScope::Everything` and `Drift::unknown` exist
    /// for, and the user is asked before it runs.
    pub fn mark_interrupted(
        &mut self,
        id: &EntryId,
        after: Option<&RepoStateNow>,
    ) -> Result<(), JournalError> {
        let held = self
            .entries
            .iter()
            .find(|held| held.id == *id)
            .ok_or_else(|| JournalError::OutcomeWithoutPending(id.clone()))?;
        if held.status != EntryStatus::Pending {
            return Err(JournalError::DuplicateOutcome(id.clone()));
        }
        let mut closed = held.clone();
        closed.status = EntryStatus::Aborted {
            reason: INTERRUPTED.to_owned(),
        };
        if let Some(now) = after {
            closed.head_after = Some(now.head.clone());
            closed.refs_after = closed
                .refs_before
                .keys()
                .map(|name| (name.clone(), now.refs.get(name).copied().flatten()))
                .collect();
            if closed.touches.index {
                // An unmerged index is recorded as its own state
                // (`IndexState::Unmerged`), not a gap, so `drift` can
                // compare it (m6-conflicts/08).
                closed.index_after = Some(now.index);
            } else {
                closed.index_after = closed.index_before;
            }
            if closed.touches.worktree {
                closed.worktree_after = Some(now.worktree_tree);
            } else {
                closed.worktree_after = closed.worktree_before;
            }
            closed.merge_after.clone_from(&now.merge);
        }
        self.append_outcome(&closed)
    }

    /// One `O_APPEND` write plus `fsync`, so the line is durable before the
    /// operation it opens starts running.
    fn append(&self, line: &[u8]) -> Result<(), JournalError> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        file.write_all(line)?;
        file.sync_all()?;
        Ok(())
    }
}

/// Cut the torn last line `seen` found off the file at `path` — but only
/// while no writer holds the journal (m8-polish-and-ship/58r).
///
/// A writer holds the writer's lock for every line it appends
/// ([`Journal::claim`]), so a torn line found while the journal is held
/// alone is a dead run's wreckage. Found while a writer holds it, it may be
/// that writer's append, not finished yet: cutting it would lose the
/// writer's line — a `Pending` one leaves its outcome with no opening, and
/// every later read refuses the file. That line is left, and not reported as
/// cut. (Whether a reader can see a partial `O_APPEND` write at all is
/// unverified on APFS and ext4; this holds either way.)
///
/// The file is read again under the lock: the line that looked torn may have
/// been finished between the first read and the lock.
fn cut_torn_tail(path: &Path, seen: Parsed) -> Result<Parsed, JournalError> {
    let Some(_alone) = FileLock::try_exclusive(path)? else {
        return Ok(Parsed {
            torn_tail: None,
            ..seen
        });
    };
    read_cutting_torn_tail(path)
}

/// Read the journal at `path`, cutting a torn last line off the file. Only
/// with the journal held alone: see [`cut_torn_tail`].
fn read_cutting_torn_tail(path: &Path) -> Result<Parsed, JournalError> {
    let bytes = std::fs::read(path)?;
    let parsed = parse(&bytes)?;
    if parsed.torn_tail.is_some() {
        let keep = match bytes.iter().rposition(|byte| *byte == b'\n') {
            Some(last_newline) => last_newline + 1,
            None => 0,
        };
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(keep as u64)?;
        file.sync_all()?;
    }
    Ok(parsed)
}

/// The journal's path for a repository: `<dir>/undo-journal.jsonl` resolved
/// through `git rev-parse --git-path`, which is `.git/<dir>/…` in the main
/// worktree and `.git/worktrees/<name>/<dir>/…` in a linked one (ADR-0010).
/// `<dir>` is the repository's names in force, `graft` or `tributary`
/// (ADR-0057, Amendment §4).
pub fn journal_path(repo: &RepoHandle) -> Result<PathBuf, RunError> {
    journal_path_in(repo, repo.on_disk_names())
}

/// [`journal_path`] under `names` rather than the handle's: the rename
/// writes its entry into the new journal while its handle still reads the
/// old names (`crate::rename`).
pub fn journal_path_in(repo: &RepoHandle, names: Generation) -> Result<PathBuf, RunError> {
    let dir = names.dir();
    repo.git_path(Path::new(&format!("{dir}/{FILE_NAME}")))
}

#[cfg(test)]
mod tests;
