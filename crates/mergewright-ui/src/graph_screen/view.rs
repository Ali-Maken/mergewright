//! [`GraphView`]: everything the graph screen draws, as plain data `app`
//! assembles — split out of `graph_screen` for size (ease-pass/08s M6).

use mergewright_term::SanitizedText;

use super::hints::{self, HintContext, StopWording};
use super::{
    BannerView, CommitGraphLine, Compaction, Filters, GraphEntry, GraphicsStatus, Icons, Location,
    OrderNotice, ProblemLine, SearchLine, SearchTail, WalkView,
};

/// Everything the graph screen draws.
#[derive(Clone, Debug)]
pub struct GraphView<'a> {
    /// His keymap: the footer spells every key from it (ticket m8/10).
    pub keys: &'a crate::keymap::Keymap,
    /// Where the screen is, once the repository is open.
    pub location: Option<Location>,
    /// The terminal is tall enough for the blank row and the rule
    /// ([`spaced`] of the terminal's rows, which `graft`'s geometry mirrors
    /// read too — not of this widget's area, which a tab strip or a pane
    /// below can shorten).
    pub spaced: bool,
    /// Where the history walk is.
    pub walk: WalkView,
    /// Something went wrong and has not been superseded. Takes the footer's
    /// place while it lasts; absent when nothing is wrong.
    pub problem: Option<ProblemLine>,
    /// The rows in the viewport, top first. The cursor's row is the one
    /// painted [`RowPaint::Highlighted`].
    pub rows: Vec<GraphEntry<'a>>,
    /// Which glyphs ref badges draw with (`ui.icons`).
    pub icons: Icons,
    /// The lane cap, and whether the user lifted it (DESIGN.md §3.5).
    pub compaction: Compaction,
    /// What the history shown leaves out; said in the header so a folded or
    /// first-parent graph is never mistaken for the whole one.
    pub filters: Filters,
    /// The row whose full subject the footer reveals in place of the key
    /// hints: an index into [`GraphView::rows`], from a hover. `None` shows
    /// the ordinary hints. A row whose subject was not cut with `…` draws
    /// nothing extra even when named here — nothing to reveal.
    pub reveal: Option<usize>,
    /// The order notice, while the rows are not yet in their final order.
    pub order: Option<OrderNotice>,
    /// Draw the graph column with the ASCII glyph table rather than Unicode
    /// box-drawing (`graph.renderer`, ticket m1-the-graph/19; DESIGN.md §4.2,
    /// ADR-0016).
    pub ascii: bool,
    /// The pixel tier draws the graph: the column keeps its width but is left
    /// blank, for the terminal's images to fill (ticket m1-the-graph/24c).
    pub pixel: bool,
    /// How the terminal took the last graph images, while the pixel tier
    /// draws them; `None` at every other tier.
    pub graphics: Option<GraphicsStatus>,
    /// `N commit(s) to push` while `HEAD`'s branch is ahead of its upstream
    /// (ease-pass/08 C13), at the top line's right end; `None` when level,
    /// with no upstream, or not read yet — never a word when there is
    /// nothing to say.
    pub push: Option<SanitizedText>,
    /// What the app layer did to the renderer tier the user did not ask to
    /// be told twice — the tier-drop key said what it did. Takes the
    /// footer's place under [`GraphView::problem`], above the hints.
    pub notice: Option<SanitizedText>,
    /// Where [`GraphView::notice`] stands: in the bar's row, or — a done
    /// line over a landing bar, `graft`'s [`hints::status_row`] answer — on
    /// its own row directly above the bar, the graph a row shorter
    /// (ease-pass/08s M4).
    pub status_row: hints::StatusRow,
    /// The search prompt while `/` is open, in the footer above a notice and
    /// under a problem; `None` otherwise (m1-the-graph/26). Once `⏎` closes
    /// the field the search's line is [`GraphView::search_tail`] (m8/15b).
    pub search: Option<SearchLine>,
    /// A search that stands with its field closed: its line in the header's
    /// tail, leftmost, so the footer is the place's keys again (m8/15b,
    /// ticket 15 A2). `None` with no search, or while the prompt is open.
    pub search_tail: Option<SearchTail>,
    /// The pinned compare base's footer line (`x`, m2-details-and-diffs/11):
    /// `"compare base <sha> — move, x again clears"`, in the same footer
    /// slot as [`GraphView::search`]/[`GraphView::notice`] — stacked under
    /// both, above the revealed subject and the key hints. `None` while no
    /// base is pinned.
    pub compare_footer: Option<SanitizedText>,
    /// Why the live operation stopped, worded by `graft` each frame
    /// (`merge stopped · 1 file conflicted`, m6-conflicts/12). Drawn ahead
    /// of the key hints on their row ([`stopped_row`]) — the last rung of
    /// the footer, so a problem, a search, a notice, the compare base and a
    /// reveal all still win. `None` while nothing is stopped.
    pub stopped: Option<StopWording>,
    /// The commit-graph offer's footer line (m1-the-graph/54): directly above
    /// the hints, below a notice. Costs one graph row while `Some`; `None`
    /// for a repository with a commit-graph, the common case.
    pub commit_graph: Option<CommitGraphLine>,
    /// Lines a harness pins under the header, above the rows, each fitted to
    /// the width: the gallery says which stop this is and what to look for
    /// (m1-the-graph/38). The first line is the title, drawn in the accent;
    /// the rest in the info colour. Every line costs a graph row. Empty in
    /// the shipped binary.
    pub banner: Vec<SanitizedText>,
    /// The detail pane is on screen **and its divider exists to drag**: the
    /// footer adds a `<> resize` hint, which is the only time that key does
    /// anything (`detail-resize-and-persistence`).
    ///
    /// The second half is not decoration. Under 100 columns the pane is a
    /// tab strip, not a split, and `<>` moves nothing — `view::frame` says
    /// exactly that where it draws the Detail tab's own bar, while the
    /// Graph tab kept the graph screen's footer, which read "a pane is
    /// open" as "a divider is there" and drew a dead key at 40 and 80
    /// columns (m5-history-and-ghost-preview/33, found by review M6's
    /// width sweep). The field is named for what the footer needs so the
    /// two cannot come apart again.
    pub divider_shown: bool,
    /// Where the cursor or focus is, for the footer's hint list
    /// (`hint-bar-redesign`, ADR-0037 §4): replaces the old `changes_pane`
    /// flag with the context [`hints::hints_for`] needs to pick the three
    /// most relevant keys at any width.
    pub hint_context: HintContext,
    /// The marker column draws: `app` has seen a Mistake Radar finding this
    /// session, on a row or on a ref (`view::radar`). Sticky once true — the
    /// column never appears and disappears as the same session scrolls
    /// (`docs/design-notes/m2-details-and-diffs.md`, marker column).
    pub marker_column: bool,
    /// The operation banner: replaces the header's ahead/behind/walk slot
    /// while `Some` — a merge/rebase/cherry-pick/revert is `in_progress`
    /// (DESIGN.md §1.13, ADR-0038 §3). `None` draws the header as it always
    /// has.
    pub banner_op: Option<BannerView>,
    /// The practice tour's card (ADR-0056 §3, ticket m8/19b): three rows
    /// under the banner, above the rows. `None` outside the tour.
    pub tour: Option<crate::tour_card::TourCardView>,
    /// The still-to-apply block at a rewrite stop, drawn boxed between the
    /// banner and the rows (m7-power-tools/07c). `None`, or empty, draws
    /// nothing and costs no row.
    pub pending: Option<crate::still_to_apply::PendingView>,
}
