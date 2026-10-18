//! One row of the graph screen as plain data — its graph cells, branch,
//! message, labels, who and age, and the tag that can follow its message —
//! split out of `graph_screen` for size (ease-pass/08s M6).

use mergewright_graph::LaneRole;
use mergewright_term::SanitizedText;

use super::{Badge, FoldMark, MarkerView};
use crate::graph_column::GraphLine;

/// One row of the graph screen.
#[derive(Clone, Debug)]
pub struct GraphEntry<'a> {
    /// The graph column's cells for this row, and how strongly it paints.
    pub line: GraphLine<'a>,
    /// The refs on this commit. Drawn as muted labels after the message
    /// (`main is here`, `↑1 to push`, `tag v1.2`, ease-pass G7) and the
    /// source a drag picks a branch from (§2.15). Empty for most rows.
    pub badges: Vec<Badge>,
    /// The commit's subject, sanitised.
    pub subject: SanitizedText,
    /// The branch column (ease-pass G5): `Some` on the first row of each
    /// run of one branch, and on a row inside an opened merge.
    pub branch: Option<BranchCell>,
    /// The who column (ease-pass G8): `None` on a row with no commit.
    pub who: Option<WhoCell>,
    /// The message draws dim: the row was made on a branch the cursor's
    /// route does not run through (ease-pass G6), or sits inside an opened
    /// merge (G4).
    pub dim_message: bool,
    /// How long ago the commit was made, already formatted (`"2m"`, `"3h"`).
    /// Not repository text — a clock reading `app` computed — so it needs no
    /// sanitising.
    pub when: String,
    /// This row is the pinned compare base (`x`, m2-details-and-diffs/11):
    /// draws the accent `▏` left rule. At most one row is ever `true`.
    pub compare_marked: bool,
    /// The Mistake Radar marker for this commit, painted in the marker
    /// column (`view::radar::marker_for`, m2-details-and-diffs/13). `None`
    /// on almost every row.
    pub marker: Option<MarkerView>,
    /// A short tag drawn after the subject — `✎ changed` on a commit a
    /// rewrite just changed (m7-power-tools/09). `None` on almost every row.
    pub tag: Option<RowTag>,
    /// A folding merge's tail (ease-pass 06, mock G4), drawn muted one
    /// space after the subject. `None` on every row that is not a merge with
    /// a merged branch folded behind it, or opened.
    pub fold: Option<FoldMark>,
}

/// A tag a row carries after its subject until the next write starts
/// (m7-power-tools/09). The subject yields to it — cut with `…` first — and
/// it is left off when fewer than [`TAG_MIN_SUBJECT`] subject cells would be
/// left; the status line says the same thing in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowTag {
    /// `✎ changed` (ASCII `~ changed`): a commit a rewrite just changed.
    Changed,
    /// `HEAD · stopped here`: the commit a rewrite's `edit` stop handed him
    /// (m7-power-tools/07b), on `HEAD`'s row while the stop lasts.
    StoppedHere,
    /// `↺ put back` (ASCII `put back`): the commit a put-back from the
    /// journal brought back (m7-power-tools/24).
    PutBack,
    /// `○ <branch> was here` (ASCII `<branch> was here`): the tip a put-back
    /// moved `branch` off (m7-power-tools/24).
    WasHere {
        /// The branch, short name, already sanitised by the caller.
        branch: String,
    },
}

impl RowTag {
    /// The words drawn after the subject, in the tier the graph column
    /// draws in: `ascii` is `GraphView::ascii`, the switch the marker glyph
    /// mirrors too (badges' `Icons` has no ASCII form).
    #[must_use]
    pub fn words(&self, ascii: bool) -> std::borrow::Cow<'_, str> {
        use std::borrow::Cow;
        match (self, ascii) {
            (RowTag::Changed, false) => Cow::Borrowed("\u{270e} changed"),
            (RowTag::Changed, true) => Cow::Borrowed("~ changed"),
            // `·` is drawn in both tiers: the ASCII tier swaps glyphs, not
            // the middle dot every status line already uses.
            (RowTag::StoppedHere, _) => Cow::Borrowed("HEAD \u{b7} stopped here"),
            // The ASCII tier has no arrow or ring that reads as these, so
            // it says the words alone rather than borrow a graph glyph.
            (RowTag::PutBack, false) => Cow::Borrowed("\u{21ba} put back"),
            (RowTag::PutBack, true) => Cow::Borrowed("put back"),
            (RowTag::WasHere { branch }, false) => {
                Cow::Owned(format!("\u{25cb} {branch} was here"))
            }
            (RowTag::WasHere { branch }, true) => Cow::Owned(format!("{branch} was here")),
        }
    }
}

/// The fewest subject cells a tag leaves; below it the tag is not drawn.
pub const TAG_MIN_SUBJECT: u16 = 8;

/// Cells between the subject and its tag.
pub(crate) const TAG_GAP: u16 = 2;

/// The branch column's cell (ease-pass G5): the branch a run of rows was
/// made on, in its role's colour, bold unless it is [`LaneRole::Other`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchCell {
    /// The branch's short name, sanitised; on a row inside an opened merge,
    /// the last path segment of the merged branch's name.
    pub name: SanitizedText,
    /// What the branch is to the cursor row's branch: picks the colour.
    pub role: LaneRole,
    /// `HEAD` is on this branch: ` (you)` follows the name.
    pub you: bool,
    /// Drawn dim: a row inside an opened merge (G4).
    pub dim: bool,
}

/// The who column's cell (ease-pass G8): the author's first name, or `you`
/// on the user's own commits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhoCell {
    /// The first name, sanitised; `you` when [`Self::own`].
    pub name: SanitizedText,
    /// The commit is the user's own (`user.email`): drawn bold.
    pub own: bool,
}
