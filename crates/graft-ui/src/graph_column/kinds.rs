//! A node's kind (its shape) and a row's paint (its strength): the two
//! facts about a row the caller decides and the widget draws.

use graft_graph::row::Row;
use ratatui::style::Modifier;

/// What kind of commit a node is, which decides its shape.
///
/// Shape, not colour: a special commit is recognisable with colour off
/// and for any colour vision (PROMPT.md §1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// An ordinary commit with one parent.
    Commit,
    /// A commit with two or more parents.
    Merge,
    /// A commit with no parents.
    Root,
    /// The commit `HEAD` points at. Wins over merge and root: where you are
    /// matters more than how the commit was made.
    Head,
    /// A stash entry.
    Stash,
    /// The working tree's uncommitted changes, drawn above `HEAD`.
    Uncommitted,
    /// A commit only a stash reaches — the base a stash was made on, after
    /// that base was rebased away (m7-power-tools/16). The caller says so
    /// (`graft::view::rows`); its row is dimmed and badged too.
    StashOnly,
    /// No node at all: a row that is not a commit, drawn only so a real
    /// lane passing it keeps an unbroken line. The Ghost Preview's
    /// `after rebase` label row is the one that uses it
    /// (`graft::view::preview`, m5/20 item 4) — the label needs a row of
    /// its own to read as a separator, and a row of its own must not put a
    /// commit glyph where there is no commit.
    Absent,
}

impl NodeKind {
    /// The kind git's commit gives a row: its real parents, not the edges
    /// this view draws, so a merge folded by default or trimmed by the
    /// first-parent view still draws `◉` (ease-pass 06f; 06 D1). `HEAD`,
    /// stash and uncommitted are facts about refs, not the graph, so the
    /// caller says those.
    #[must_use]
    pub fn of(row: &Row) -> NodeKind {
        match row.commit.parents.len() {
            0 => NodeKind::Root,
            1 => NodeKind::Commit,
            _ => NodeKind::Merge,
        }
    }

    /// The glyph this kind of node draws as.
    #[must_use]
    pub fn glyph(self) -> char {
        match self {
            NodeKind::Commit => '●',
            NodeKind::Merge => '◉',
            NodeKind::Root => '◇',
            NodeKind::Head => '◆',
            NodeKind::Stash => '■',
            NodeKind::Uncommitted => '◌',
            // The stash's `■`, emptied. Not `◇`: that is the root's.
            NodeKind::StashOnly => '□',
            NodeKind::Absent => ' ',
        }
    }
}

/// How strongly a row paints, set by selection and search (ticket 16), or
/// that it is not a real commit at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowPaint {
    /// As designed.
    #[default]
    Normal,
    /// Pushed back: outside the current focus.
    Dimmed,
    /// Brought forward: the selection, or a search hit.
    Highlighted,
    /// The row a dragged badge is over — where it would land if the button
    /// came up now (`docs/design/02-keymap.md` §2.15).
    ///
    /// Its own variant rather than [`RowPaint::Highlighted`] because during
    /// a drag **both** rows are on screen at once: the cursor has not moved
    /// and comes back when the badge is dropped, so the two have to be told
    /// apart. They were the same paint until
    /// m5-history-and-ghost-preview/35, which is why a drag looked like
    /// nothing happening — the only change on screen was the same faint
    /// raise moving from one row to another.
    DropTarget,
    /// A Ghost Preview row: a commit that does not exist yet, or a real
    /// parent clipped at the preview slice's edge and drawn as an anchor
    /// rather than loaded (ADR-0038 §5). The node draws as a hollow anchor
    /// glyph never used by a real [`NodeKind`]; the ghost's own edges dash
    /// and paint [`Token::TextMuted`](crate::theme::Token::TextMuted)
    /// regardless of the lane's branch colour — an anchor must never be
    /// mistaken for a real parent, at any tier.
    ///
    /// Which edges are the ghost's is [`GraphLine::ghost_lanes`](super::GraphLine::ghost_lanes)'s to say.
    /// A real lane passing through a ghost row keeps its solid glyph and
    /// its colour (DESIGN.md §1.7's `┊│┊●b┈`): the before and the after are
    /// one picture, and a real lane that dashes because a ghost is drawn
    /// beside it leaves the user unable to tell what is real.
    Ghost,
}

impl RowPaint {
    pub(super) fn modifier(self) -> Modifier {
        match self {
            RowPaint::Normal | RowPaint::Ghost => Modifier::empty(),
            RowPaint::Dimmed => Modifier::DIM,
            RowPaint::Highlighted | RowPaint::DropTarget => Modifier::BOLD,
        }
    }
}
