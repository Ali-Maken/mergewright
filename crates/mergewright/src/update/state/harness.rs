//! What a harness wrapping `event_loop::run` asks for, and the banner it
//! pins under the header.

use mergewright_core::event::{KeyCode, KeyPress};

use crate::session::Clock;

/// What a harness wrapping `event_loop::run` asks for (the graph gallery,
/// m1-the-graph/35). The default asks for nothing, which is what the
/// shipped binary passes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harness {
    /// Lines pinned under the header for the whole run: the gallery says
    /// which stop this is and what to look for (m1-the-graph/38). Each
    /// line costs a graph row.
    pub banner: Option<Banner>,
    /// Keys the loop does not interpret: pressing one sets `quit` to
    /// [`Quit::HandedBack`] so the caller can act on it (the gallery steps
    /// to the next repository). Not consulted while the search prompt is
    /// open, where every printable key is text.
    pub hand_back: Vec<KeyCode>,
    /// Keys pressed for the user before the first frame, in order
    /// (m2-details-and-diffs/23): a gallery stop opens the pane or marks a
    /// compare with them. Fed one at a time by `update::prelude`, each only
    /// once the first window and the refs answer are on hand and nothing an
    /// earlier key started is still in flight. Empty in the shipped binary.
    pub prelude: Vec<KeyPress>,
    /// The instant every relative date on screen is drawn against
    /// (m1-the-graph/53). `Wall` for the shipped binary; a gallery corpus
    /// stop or a snapshot test asks for `Fixed` instead, so its capture
    /// never drifts with real time.
    pub clock: Clock,
    /// Offer to build a missing commit-graph file (m1-the-graph/54). `true`
    /// in the shipped binary; the gallery sets `false`, because its corpus
    /// is borrowed read-only.
    pub offer_commit_graph: bool,
}

impl Default for Harness {
    fn default() -> Self {
        Harness {
            banner: None,
            hand_back: Vec::new(),
            prelude: Vec::new(),
            clock: Clock::default(),
            offer_commit_graph: true,
        }
    }
}

impl Harness {
    /// Screen rows the banner takes, none without one.
    #[must_use]
    pub fn banner_rows(&self) -> u16 {
        self.banner.as_ref().map_or(0, Banner::rows)
    }
}

/// What a harness pins under the header (m1-the-graph/38). The view appends
/// ` · <tier>` to the title for the rendering tier in force, since only the
/// run knows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Banner {
    /// The first line, in the accent colour: where the user is and which
    /// keys move on.
    pub title: String,
    /// The lines under it, in the info colour: what to look for.
    pub lines: Vec<String>,
}

impl Banner {
    /// Screen rows the banner takes: the title and every line.
    #[must_use]
    pub fn rows(&self) -> u16 {
        u16::try_from(self.lines.len().saturating_add(1)).unwrap_or(u16::MAX)
    }
}
