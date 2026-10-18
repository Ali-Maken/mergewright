//! One row of the graph screen as plain data — its graph cells, badges,
//! subject, author chip, date and SHA, and the tag that can stand in front
//! of its subject — split out of `graph_screen` for size (ease-pass/08s M6).

use mergewright_graph::identity::BranchIdentity;
use mergewright_term::SanitizedText;

use super::{Badge, FoldMark, MarkerView};
use crate::graph_column::GraphLine;

/// One row of the graph screen.
#[derive(Clone, Debug)]
pub struct GraphEntry<'a> {
    /// The graph column's cells for this row, and how strongly it paints.
    pub line: GraphLine<'a>,
    /// The refs on this commit, in the order they are drawn. Empty for
    /// most rows.
    pub badges: Vec<Badge>,
    /// The commit's subject, sanitised.
    pub subject: SanitizedText,
    /// The author's chip: initials and the identity that colours it.
    pub author: AuthorChip,
    /// How long ago the commit was made, already formatted (`"2m"`, `"3h"`).
    /// Not repository text — a clock reading `app` computed — so it needs no
    /// sanitising.
    pub when: String,
    /// The commit's short SHA, lowercase hex. Not repository text either:
    /// hex drawn from a fixed digit alphabet is safe as-is
    /// (`mergewright_core::oid::Oid`'s own doc comment).
    pub sha: String,
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

/// An author's chip: two initials, coloured by the author's identity.
///
/// The colour comes from [`BranchIdentity`] and the same lane palette a
/// branch draws in ([`Theme::branch`](crate::theme::Theme::branch)) — reused
/// rather than given its own palette, since both are "a stable colour for a
/// hash of some bytes" and a second palette would only have to be proven
/// colourblind-safe all over again.
#[derive(Clone, Debug)]
pub struct AuthorChip {
    /// The two initials drawn in the chip, sanitised.
    pub initials: SanitizedText,
    /// The identity the chip's colour comes from — of the author's
    /// `.mailmap`-resolved email, so two identities mapped to one person
    /// share a colour along with their chip.
    pub identity: BranchIdentity,
}
