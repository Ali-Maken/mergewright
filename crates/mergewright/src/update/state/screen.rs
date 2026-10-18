//! The screen being shown, the overlays that replace the graph, and the
//! terminal size the layout is cut from.

/// Screen rows that are not the graph below 30 rows: one line of header above
/// it and one of status below. From 30 rows a blank row and a rule join them
/// (`mergewright_ui::graph_screen::chrome_rows`, ADR-0058 §4), which is what
/// [`AppState::graph_pane_rows`] reads.
pub const CHROME_ROWS: u16 = mergewright_ui::graph_screen::CHROME_ROWS;

/// The screens Mergewright can show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    /// The history graph. What Mergewright opens on.
    #[default]
    Graph,
    /// What Mergewright opened and what it is drawing with.
    Status,
}

impl Screen {
    /// The other screen: `F2` swaps between the two.
    #[must_use]
    pub fn toggled(self) -> Screen {
        match self {
            Screen::Graph => Screen::Status,
            Screen::Status => Screen::Graph,
        }
    }
}

/// An overlay that **replaces** the graph screen: it takes the graph's whole
/// area and carries no operation banner of its own.
///
/// See [`AppState::graph_overlay`], which is the only place the conditions
/// live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphOverlay {
    /// The finish summary (`c` on a merge, m6-conflicts/19). Covers the
    /// detail pane too, and keeps the banner line, as the resolver does.
    Finish,
    /// The conflict resolver (`⏎` on a conflicted file, m6-conflicts/04).
    /// The one overlay that takes the detail pane's area too.
    Resolver,
    /// A blame place (`B`, m7-power-tools/20): the whole frame, its own bar.
    Place,
    /// The rewrite editor (`R`, m7-power-tools/08): the whole frame, its
    /// own bar.
    Rewrite,
    /// The commit message editor (`c`).
    CommitEditor,
    /// The Map view (`M`).
    Map,
    /// The undo journal browser (`J`).
    Journal,
    /// The stash list (`S`).
    Stash,
}

/// A terminal size in character cells.
///
/// Either number can be zero. A pty that has not been sized yet reports `0×0`,
/// and that is what silently broke the graph spike's layout (ADR-0012), so the
/// size is carried as-is and every consumer is expected to cope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    /// Width in character cells.
    pub columns: u16,
    /// Height in character cells.
    pub rows: u16,
}
