//! The shape of one capability, as read from `docs/capabilities/*.md`.

use std::fmt;
use std::path::PathBuf;

/// One capability: a distinct, user-visible thing the application can do.
///
/// Everything here is read from a capability's markdown entry — the prose
/// (`how`, `behaviour`, `decided_in`, `status_prose`) and the fenced `meta`
/// block right after its heading (`id`, `area`, `milestone`, `status`). See
/// `docs/capabilities/README.md` for the schema this mirrors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    /// Stable id, e.g. `M2-07`. Unique across the whole database.
    pub id: String,
    /// The heading text after the id, e.g. "Focus and keys inside the
    /// details pane".
    pub title: String,
    /// What part of the product this belongs to (not a milestone) — e.g.
    /// `graph`, `diffs`, `undo`, `remotes`.
    pub area: String,
    /// The milestone slug it arrived in, e.g. `m2-details-and-diffs`.
    pub milestone: String,
    /// Machine status, one of the four `CLAUDE.md` names.
    pub status: Status,
    /// What's specifically unverified about an otherwise-`works` (or
    /// `partial`) capability — e.g. "colour, captures record character
    /// grids only". `None` for most capabilities. This is **not** a
    /// synonym for `status: unverified`: a capability can work, and be
    /// proven to work, while carrying one named aspect nobody has checked
    /// (usually how it looks on a real screen, which a captured terminal
    /// stream can never settle — `CLAUDE.md` rule 5). See
    /// `docs/capabilities/README.md`, "the unverified-aspect field".
    pub unverified_aspect: Option<String>,
    /// The free-text prose after `**Status:**` — the detail a bare enum
    /// can't carry (what's unverified, what's known-broken, and why).
    pub status_prose: String,
    /// Test names cited under `**Proven by:**`, exactly as written
    /// (`crate::module::tests::name`).
    pub tests: Vec<String>,
    /// Capture paths cited under `**Looks like:**`, exactly as written
    /// (`docs/shots/...`).
    pub captures: Vec<String>,
    /// The raw `**Decided in:**` prose, kept whole for display.
    pub decided_in: String,
    /// File-like citations pulled out of `decided_in`: explicit paths
    /// (backtick-quoted, ending `.md`) and bare `ADR-NNNN` references,
    /// resolved to the path the checker expects to find on disk.
    pub decisions: Vec<Decision>,
    /// The source file this capability was read from, relative to the repo
    /// root.
    pub source_file: PathBuf,
    /// 1-based line number of this capability's `### ` heading, for error
    /// messages that point straight at the spot.
    pub source_line: usize,
}

/// A citation pulled from `**Decided in:**`, resolved to a path to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// What the prose said (an ADR number, or the path as written).
    pub cited: String,
    /// The path the checker resolves it to, relative to the repo root.
    /// For an `ADR-NNNN` reference this is a glob prefix
    /// (`docs/adr/NNNN-`) rather than a full filename, since the prose
    /// never spells out the slug.
    pub resolved: DecisionPath,
}

/// How a [`Decision`] is checked against the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionPath {
    /// An exact path, relative to the repo root, that must exist.
    Exact(String),
    /// A directory and numeric prefix (`docs/adr`, `"0016"`): satisfied if
    /// any entry in the directory starts with the prefix.
    GlobPrefix {
        /// Directory to search, relative to the repo root.
        dir: String,
        /// Filename prefix an entry of `dir` must start with.
        prefix: String,
    },
}

/// Machine status. Preserves the distinction `CLAUDE.md` rule 4 requires:
/// broken and unverified are never the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Works and has been verified (by test, or by eye where a test can't
    /// reach — see `status_prose`).
    Works,
    /// Part of it works and is verified; part is unverified, unwired, or
    /// intentionally narrower than the original ambition.
    Partial,
    /// Believed correct, but nothing has verified it — colour, a real
    /// terminal, a real network peer, and the like.
    Unverified,
    /// Known not to work as specified, disclosed rather than hidden.
    Broken,
}

impl Status {
    /// Parses the machine `status:` field of a `meta` block.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "works" => Some(Self::Works),
            "partial" => Some(Self::Partial),
            "unverified" => Some(Self::Unverified),
            "broken" => Some(Self::Broken),
            _ => None,
        }
    }

    /// The lowercase word this status renders as, matching the schema.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Works => "works",
            Self::Partial => "partial",
            Self::Unverified => "unverified",
            Self::Broken => "broken",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
