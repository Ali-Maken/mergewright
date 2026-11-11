//! Building a `git fast-import` stream.
//!
//! PROMPT.md §7.1 says fixtures are generated with `fast-import`, not with
//! thousands of `git commit` calls, and the corpus needs byte-identical
//! object ids run to run. Both fall out of the same property: a stream states
//! every input to every object — parents, tree, message, author, committer
//! and both timestamps — so nothing is left to the machine's clock, locale or
//! config.
//!
//! Everything here is bytes, not `String`. A fixture whose ref names and
//! paths are not valid UTF-8 is one of the shapes the corpus has to contain.
//!
//! The one thing `fast-import` will not do for you is merge. A commit with
//! two `from`/`merge` parents starts from the *first* parent's tree and
//! applies only the changes the stream spells out, so every merge here lists
//! the resulting tree explicitly. That is a feature for this corpus: an evil
//! merge is written the same way as an honest one, with different contents.

#[cfg(feature = "heavy-tests")]
use std::io::{self, Write};

use crate::git;

/// A reference to an object earlier in the same stream.
///
/// `fast-import` has no way to name an object it has not written yet, so the
/// stream numbers them as it goes and refers back by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark(u32);

/// The identity on every commit and tag in the corpus, unless a shape
/// deliberately overrides it.
pub const IDENTITY: &str = git::IDENTITY;

/// The first commit's timestamp: 2020-01-01T00:00:00Z, in git's raw format.
const EPOCH: u64 = 1_577_836_800;

/// Seconds between one commit and the next.
///
/// A minute apart, in the order the stream writes them, so date order and
/// stream order agree and a topological test has something to disagree with.
const STEP: u64 = 60;

/// A `fast-import` stream under construction.
pub struct Stream {
    bytes: Vec<u8>,
    next_mark: u32,
    next_time: u64,
}

/// A file in a commit: its path and its whole content.
pub type FileContent<'a> = (&'a [u8], &'a [u8]);

impl Stream {
    /// An empty stream.
    pub fn new() -> Self {
        Stream {
            bytes: Vec::new(),
            next_mark: 0,
            next_time: EPOCH,
        }
    }

    /// Records a commit and returns the mark that names it.
    ///
    /// `parents` is in git's own order: the first is the first parent, and
    /// every later one is a merge parent. An empty slice makes a root commit,
    /// which is how orphan branches begin.
    pub fn commit(
        &mut self,
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
        files: &[FileContent<'_>],
    ) -> Mark {
        self.commit_by(IDENTITY, refname, message, parents, files)
    }

    /// [`commit`](Self::commit), dated `ahead` seconds later than its place in
    /// the stream says — a commit made on a machine whose clock ran fast.
    ///
    /// The one way to make commit-date order and topological order disagree:
    /// a child committed after this one in the stream is dated *earlier* than
    /// it. The clock carries on from the stream's own time, not this date.
    pub fn commit_ahead(
        &mut self,
        ahead: u64,
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
        files: &[FileContent<'_>],
    ) -> Mark {
        let when = self.tick() + ahead;
        self.commit_at(IDENTITY, when, refname, message, parents, files)
    }

    /// [`commit`](Self::commit), with the author and committer spelled out.
    ///
    /// Only the Unicode shape needs this: an identity with non-ASCII letters
    /// in it is one of the things a graph row has to render.
    pub fn commit_by(
        &mut self,
        identity: &str,
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
        files: &[FileContent<'_>],
    ) -> Mark {
        let when = self.tick();
        self.commit_at(identity, when, refname, message, parents, files)
    }

    /// [`commit`](Self::commit), authored and committed at the exact Unix
    /// time `when` rather than the stream's own synthetic clock.
    ///
    /// Every other commit in the corpus is timed off `EPOCH`/`STEP` so
    /// object ids are byte-identical run to run (this module's doc
    /// comment). The one shape that needs a commit to sit at a real
    /// wall-clock age — "36 hours ago", read back by `relative_date` at
    /// render time — cannot use that clock and stay honest about the age,
    /// so it calls this instead; `next_time` is left untouched, since a
    /// shape built entirely with explicit times never reads it.
    pub fn commit_at_time(
        &mut self,
        when: u64,
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
        files: &[FileContent<'_>],
    ) -> Mark {
        self.commit_at(IDENTITY, when, refname, message, parents, files)
    }

    /// A commit by `identity` at the exact Unix time `when` (`by`) that also
    /// deletes `deleted` — a rename is a delete of the old path plus the new
    /// path in `files`, and git's `-M` finds it from the content. For the
    /// trace shape (m7-power-tools/18), whose file is written by four people
    /// over a year and renamed once. `next_time` is left untouched, as in
    /// [`commit_at_time`](Self::commit_at_time).
    pub fn commit_full(
        &mut self,
        (identity, when): (&str, u64),
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
        files: &[FileContent<'_>],
        deleted: &[&[u8]],
    ) -> Mark {
        let mark = self.commit_head(identity, when, refname, message, parents);
        for path in deleted {
            self.bytes.extend_from_slice(b"D ");
            self.bytes.extend_from_slice(path);
            self.bytes.push(b'\n');
        }
        self.write_files(files);
        mark
    }

    /// A commit by `identity`, authored and committed at `when`.
    fn commit_at(
        &mut self,
        identity: &str,
        when: u64,
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
        files: &[FileContent<'_>],
    ) -> Mark {
        let mark = self.commit_head(identity, when, refname, message, parents);
        self.write_files(files);
        mark
    }

    /// Every file of a commit, then the blank line that ends the command.
    fn write_files(&mut self, files: &[FileContent<'_>]) {
        for (path, content) in files {
            self.bytes.extend_from_slice(b"M 100644 inline ");
            self.bytes.extend_from_slice(path);
            self.bytes.push(b'\n');
            self.write_data(content);
        }
        self.bytes.push(b'\n');
    }

    /// A commit command up to its file changes: header, identity, message
    /// and parents.
    fn commit_head(
        &mut self,
        identity: &str,
        when: u64,
        refname: &[u8],
        message: &[u8],
        parents: &[Mark],
    ) -> Mark {
        let mark = self.reserve_mark();

        self.bytes.extend_from_slice(b"commit ");
        self.bytes.extend_from_slice(refname);
        self.bytes.push(b'\n');
        self.write_mark_line(mark);
        self.write_line(&format!("author {identity} {when} +0000"));
        self.write_line(&format!("committer {identity} {when} +0000"));
        self.write_data(message);

        if let Some((first, rest)) = parents.split_first() {
            self.write_line(&format!("from :{}", first.0));
            for parent in rest {
                self.write_line(&format!("merge :{}", parent.0));
            }
        }
        mark
    }

    /// Points `refname` at an object already in the stream, creating no new
    /// object. This is how a branch is forked and how a lightweight tag is
    /// made.
    pub fn point(&mut self, refname: &[u8], at: Mark) {
        self.bytes.extend_from_slice(b"reset ");
        self.bytes.extend_from_slice(refname);
        self.bytes.push(b'\n');
        self.write_line(&format!("from :{}", at.0));
        self.bytes.push(b'\n');
    }

    /// Records an annotated tag object at `refs/tags/<name>`.
    ///
    /// `name` is the short name: `fast-import`'s `tag` command prefixes
    /// `refs/tags/` itself.
    pub fn annotated_tag(&mut self, name: &[u8], target: Mark, message: &[u8]) {
        let when = self.tick();
        self.bytes.extend_from_slice(b"tag ");
        self.bytes.extend_from_slice(name);
        self.bytes.push(b'\n');
        self.write_line(&format!("from :{}", target.0));
        self.write_line(&format!("tagger {IDENTITY} {when} +0000"));
        self.write_data(message);
        self.bytes.push(b'\n');
    }

    /// The finished stream, terminated with the `done` command that makes
    /// `git fast-import --done` prove the stream was not truncated.
    pub fn finish(mut self) -> Vec<u8> {
        self.bytes.extend_from_slice(b"done\n");
        self.bytes
    }

    /// Writes everything buffered so far to `out` and empties the buffer,
    /// keeping the mark counter and the clock exactly where they were.
    ///
    /// This is what lets the heavy tier's 100k-commit fixture be generated
    /// without ever holding the whole stream in memory — a stream of any size
    /// can be written a chunk at a time. Marks must keep counting across the
    /// drain, which is why this takes `&mut self` rather than consuming the
    /// stream: `fast-import` numbers objects for the whole stream, not per
    /// chunk.
    ///
    /// Call it only *between* commands. Every method here appends one whole
    /// command, so any point after one of them returns is a boundary; there
    /// is no way to be part-way through a command from outside.
    ///
    /// Behind the feature because the heavy tier is its only caller: the
    /// named corpus's streams are kilobytes and are written in one go.
    #[cfg(feature = "heavy-tests")]
    pub fn drain_into(&mut self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&self.bytes)?;
        self.bytes.clear();
        Ok(())
    }

    /// Drains the rest of the stream to `out` and terminates it with `done`,
    /// which is what makes a truncated stream an error rather than a partial
    /// import.
    #[cfg(feature = "heavy-tests")]
    pub fn finish_into(self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&self.finish())
    }

    fn reserve_mark(&mut self) -> Mark {
        self.next_mark += 1;
        Mark(self.next_mark)
    }

    fn tick(&mut self) -> u64 {
        let now = self.next_time;
        self.next_time += STEP;
        now
    }

    fn write_mark_line(&mut self, mark: Mark) {
        self.write_line(&format!("mark :{}", mark.0));
    }

    fn write_line(&mut self, line: &str) {
        self.bytes.extend_from_slice(line.as_bytes());
        self.bytes.push(b'\n');
    }

    /// A length-prefixed payload, and nothing after it.
    ///
    /// `fast-import` allows one optional newline after a `data` block, and
    /// every command here already ends with the blank line that separates it
    /// from the next. Writing a newline here too would supply both, and the
    /// spare one reaches the top-level parser as an empty command.
    fn write_data(&mut self, payload: &[u8]) {
        self.write_line(&format!("data {}", payload.len()));
        self.bytes.extend_from_slice(payload);
    }
}

impl Default for Stream {
    fn default() -> Self {
        Self::new()
    }
}
