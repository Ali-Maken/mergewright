//! Where a line or a file came from (m7-power-tools/18): `git log -L` for a
//! range of lines, `git log --follow` for a whole file, the function around
//! a line, a working-tree line's place in `HEAD`, and one commit's change to
//! a file shown as the whole file.
//!
//! Every spawn goes through [`crate::run::GitInvocation`], so each read is
//! cancellable. The runner hands stdout over only when git exits, so a trace
//! reaches its `sink` as **one batch at the end**, not streamed: a stopped
//! trace has sent no rows.

mod follow;
mod function;
mod log;
pub(crate) mod shallow;
mod sizes;
mod whole;

use std::fmt;
use std::path::PathBuf;

use graft_core::oid::Oid;

use crate::diff::ParseError;
use crate::run::RunError;

pub use follow::parse_follow_log;
pub use function::{enclosing_function, map_to_head};
pub use log::{file_history, line_history, parse_line_log};
pub use sizes::{BlobSizes, blob_sizes};
pub use whole::whole_file;

/// The end reason when a trace reaches a shallow clone's boundary: what
/// lies past it was never fetched, so no start can be named (m7/18c).
pub const SHALLOW_END: &str = "this clone is shallow; nothing older was fetched";

/// The lines a history follows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineRange {
    /// A function, by git's funcname rules: its header as git prints it
    /// (leading whitespace trimmed), and its 1-indexed inclusive lines.
    Function {
        /// The header line git names the function by.
        name: String,
        /// First line (the header's).
        start: u32,
        /// Last line.
        end: u32,
    },
    /// Plain lines, 1-indexed inclusive — the blame block the app falls
    /// back to.
    Block {
        /// First line.
        start: u32,
        /// Last line.
        end: u32,
    },
}

impl LineRange {
    /// The inclusive 1-indexed lines, whichever kind.
    #[must_use]
    pub fn lines(&self) -> (u32, u32) {
        match self {
            LineRange::Function { start, end, .. } | LineRange::Block { start, end } => {
                (*start, *end)
            }
        }
    }
}

/// One commit in a trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceCommit {
    /// The commit.
    pub id: Oid,
    /// Its author's name.
    pub author: String,
    /// Its author time, Unix seconds.
    pub when: i64,
    /// Its subject line.
    pub subject: String,
    /// The file's path in this commit.
    pub path: PathBuf,
    /// The path before, when this commit renamed the file.
    pub renamed_from: Option<PathBuf>,
    /// Lines added: the range's only (`line_history`), or the file's.
    pub added: u32,
    /// Lines removed, the same way.
    pub removed: u32,
    /// `line_history` only: where the range sits in this commit, 1-indexed
    /// inclusive, from git's own `-L` hunk.
    pub range_at: Option<(u32, u32)>,
}

/// Why a trace ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceEnd {
    /// The last commit sent added the file (or the whole range).
    Start {
        /// That commit.
        added_by: Oid,
    },
    /// git stopped before the start, and said why (its first line); or,
    /// with no word from git, this reader's own words for where the history
    /// ran out.
    GitStops(String),
    /// `cap` rows were sent and more exist.
    Capped {
        /// How many were sent.
        shown: usize,
    },
    /// The caller's stop fired; git was killed. Rows already sent stay the
    /// sink's.
    Stopped,
}

/// Why a trace failed outright.
#[derive(Debug)]
pub enum TraceError {
    /// git could not be run, or was killed from outside.
    Run(RunError),
    /// git's log did not parse.
    Parse(ParseError),
    /// `<common_dir>/shallow` exists but could not be read (m7/18c).
    Shallow {
        /// The file.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// git's output passed the runner's cap; refused rather than read short
    /// (18e).
    TooLarge {
        /// The cap, in bytes.
        cap: usize,
    },
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceError::Run(error) => write!(f, "{error}"),
            TraceError::Parse(error) => write!(
                f,
                "git log's output made no sense at line {}: {}",
                error.line, error.what
            ),
            TraceError::Shallow { path, source } => {
                write!(f, "couldn't read {}: {source}", path.display())
            }
            TraceError::TooLarge { cap } => write!(
                f,
                "too large to trace: git's output is over {} MiB",
                cap / (1024 * 1024)
            ),
        }
    }
}

impl std::error::Error for TraceError {}

impl From<RunError> for TraceError {
    fn from(error: RunError) -> Self {
        TraceError::Run(error)
    }
}

#[cfg(test)]
mod tests;
