//! [`TabWord`]: where one `⇥` lands from the changes pane.

/// Where one `⇥` lands from the changes pane, as its hint names it
/// (m6-conflicts/70): `⇥ diff`, `⇥ banner` or `⇥ graph`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabWord {
    /// The graph: from the diff with no banner drawn.
    Graph,
    /// The diff: from the file list.
    Diff,
    /// The operation banner: from the diff while an operation is live.
    Banner,
}

impl TabWord {
    /// Every form, for [`crate::graph_screen::hints::every_context`].
    pub const ALL: [TabWord; 3] = [TabWord::Graph, TabWord::Diff, TabWord::Banner];

    /// The hint's verb: `graph`, `diff` or `banner`.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            TabWord::Graph => "graph",
            TabWord::Diff => "diff",
            TabWord::Banner => "banner",
        }
    }
}
