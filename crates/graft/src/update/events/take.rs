//! [`TakeRequest`]: one take the resolver asks for, carried by
//! `Command::TakeSide` — split out of `command` (ease-pass/08s M6).

/// One block's take, as the resolver asks for it.
///
/// The **document** travels, not a fingerprint: `graft_ops::conflict::apply_take`
/// applies the take to the copy the user is looking at and writes that back,
/// checking `file.fingerprint` against the bytes on disk as it goes. Re-reading
/// the path instead would drop every block already resolved and re-index the
/// rest, so a second take would land on the wrong block (design note, hard
/// choice 2; m6-conflicts/03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakeRequest {
    /// The conflicted file, as git's porcelain names it.
    pub path: std::path::PathBuf,
    /// The document the take was chosen against — the resolver's own copy,
    /// shared rather than cloned into the command.
    pub file: std::sync::Arc<graft_git::conflict::ConflictFile>,
    /// Where the take lands: one block, or every open one.
    pub at: TakeAt,
    /// Which side it keeps.
    pub take: graft_git::conflict::Take,
}

/// Where a take lands (m6-conflicts/16 D2): one block, counting `Conflict`
/// and `Resolved` regions in order, or every block still open
/// (`Ctrl+O`/`Ctrl+T`) — one queue entry, one write, one keypress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TakeAt {
    /// The `n`-th block.
    Block(usize),
    /// Every block still open in the document the take is applied to.
    Every,
}
