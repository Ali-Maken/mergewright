//! The screen Graft opens on: the history graph, one commit per row.
//!
//! M1's first graph screen, deliberately plain: a header line saying where
//! the walk is, the rows the viewport shows (the Unicode graph column and one
//! subject column), and a footer of key hints. Columns and badges replace the
//! subject column in later tickets; the full chrome of DESIGN.md §1.1–§1.3
//! arrives with them.
//!
//! Like every screen it renders plain data: a [`GraphView`] that `app`
//! assembles from the rows the walk last sent. It never asks for rows and
//! never scrolls — the viewport arrives already cut.
//!
//! Split across files by concern (m1-the-graph/73a): this file holds the
//! plain-data types the other files read; [`layout`] the geometry and header;
//! [`footer`] the footer's text (search, notices, the commit-graph offer,
//! key hints); [`render`] the actual drawing.

pub mod badges;
pub mod columns;
mod footer;
pub mod hints;
mod layout;
mod render;
#[cfg(test)]
mod tests;

use graft_core::mistake_radar::Severity;
use graft_graph::fold::{Fold, Keep};
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use graft_term::SanitizedText;

pub use badges::{Badge, BadgeKind, Icons, Tracking};
pub use columns::{absolute_date, initials, relative_date, short_sha};
pub use hints::{
    ConfirmYes, Divider, Hint, HintContext, HintState, LiveC, Rung, StopWording, TabWord,
    every_context, every_hint, fit_status_line, hints_for, hints_text, stopped_row,
    stopped_row_before_keys,
};
pub use layout::{
    Columns, RowLayout, ScreenRows, lanes_that_fit, max_lanes_for, row_at, row_layout, screen_rows,
    snap_to_columns,
};
pub use render::badge_spans;

use crate::graph_column::GraphLine;
use crate::theme::Theme;

/// Cells the marker slot always takes: the cursor rule, the Mistake Radar
/// glyph, then a gap. Reserved whether or not a row has either, so the
/// columns after it never move when a cursor or a marker appears.
///
/// It was two — glyph, gap — until m5-history-and-ghost-preview/35 gave the
/// cursor's rule the first cell and left the glyph to take the rest, which
/// spent the gap: `▏⚠Merge branch 'feature'`, a marker touching the subject
/// it marks. Two things with no gap between them read as one thing
/// (m5-history-and-ghost-preview/30), so the rule gets a cell of its own
/// rather than borrowing the gap's. The subject pays one column for it at
/// every width; it is elastic and truncates one character earlier.
const MARKER_WIDTH: u16 = 3;

/// Cells inside [`MARKER_WIDTH`] that belong to the cursor rule, reserved on
/// every row so the glyph beside it never moves between a cursor row and its
/// neighbour.
const RULE_WIDTH: u16 = 1;

/// Cells the author chip takes: two initials, then a gap.
const AUTHOR_WIDTH: u16 = 3;

/// Cells the relative-date column takes: its widest form (`"999d"`), then a
/// gap.
const WHEN_WIDTH: u16 = 5;

/// Cells the short-SHA column takes, and how many hex digits `short_sha`
/// should be called with to fill it exactly (`DESIGN.md`'s mock-ups show
/// `a1b2c3d`) — rightmost, so no trailing gap.
pub const SHA_WIDTH: u16 = 7;

/// The gap left of the trailing columns, between them and the subject.
const SUBJECT_GAP: u16 = 1;

/// Below this width the author chip drops (DESIGN.md §1.2/§1.3's
/// narrow/medium boundary); ref badges (m1-the-graph/11) drop at the same
/// width.
pub const AUTHOR_BREAKPOINT: u16 = 100;

/// Screen rows that are not graph rows: the header above, the key hints
/// below. A harness banner ([`GraphView::banner`]) adds one row per line.
pub const CHROME_ROWS: u16 = 2;

/// Everything the graph screen draws.
#[derive(Clone, Debug)]
pub struct GraphView<'a> {
    /// The repository's path, once it is open.
    pub location: Option<SanitizedText>,
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
    /// How many lanes the graph may draw before folding the rest behind
    /// `+N`, and whether the user has expanded them (DESIGN.md §3.5).
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
    /// What the app layer did to the renderer tier the user did not ask to
    /// be told twice — the tier-drop key said what it did. Takes the
    /// footer's place under [`GraphView::problem`], above the hints.
    pub notice: Option<SanitizedText>,
    /// The search prompt or the search's state, in the footer above a notice
    /// and under a problem; `None` when there is no search (m1-the-graph/26).
    pub search: Option<SearchLine>,
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
}

/// A problem the footer shows in place of the key hints: a failure, read or
/// write (`Status::Failed`/`Status::Sticky`). Split in two so the width
/// rule (`hints::fit_status_line`, ADR-0037 §4) can keep `keys` whole and
/// cut `reason` first — a blind right-to-left cut at 80 columns once lost
/// the very key the message was naming (`m4-07-acceptance` M2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemLine {
    /// What went wrong, in words.
    pub reason: SanitizedText,
    /// The key(s) that answer it, worded as they'd read in the hint bar
    /// (`"p pull · P force"`). `None` when the message names none.
    pub keys: Option<SanitizedText>,
}

/// The operation banner: `REBASING step 3 of 7` — the state and the step,
/// said once ([`banner_line`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerView {
    /// The operation's name, upper-case: `REBASING`, `MERGING`,
    /// `CHERRY-PICKING`, `REVERTING`.
    pub kind: SanitizedText,
    /// The step git is on and how many there are, when the operation has
    /// steps (a rebase/cherry-pick/revert sequence). `None` for a plain
    /// merge, which has none to count.
    pub step: Option<(u32, u32)>,
    /// The first conflicted path, once a conflict has landed. `None` before
    /// that — an in-progress operation the user has not hit trouble in yet.
    ///
    /// **Never drawn.** It is read by `hint_context` alone, to decide
    /// whether `M mergetool` and `e edit` have anything to act on. The
    /// banner used to name it, four rows above a changes pane listing every
    /// conflicted file under `CONFLICTS (n)` — the same fact twice on a
    /// 24-row screen (m5/20 item 1, the user's own fix).
    pub conflict: Option<SanitizedText>,
    /// The banner has taken focus (a conflict just landed, ADR-0037 §3):
    /// its slot draws reversed so a key-driven user notices without a
    /// click.
    pub focused: bool,
    /// `s` (skip) is offered — a rebase/cherry-pick/revert sequence can
    /// skip a step; a plain merge cannot. Affects only the hint bar
    /// (`hint-bar-redesign`'s group, not this one's `graph_screen`
    /// footer) — never the banner's own text or buttons.
    pub skippable: bool,
}

/// `banner`'s text: `MERGING`, or `REBASING step 3 of 7` when the operation
/// counts steps.
///
/// **One wording at every width**, and no `width` to word it to. The two
/// earlier defects here were one defect: a `width >= 100` tier chose between
/// `— conflict in notes.md` and `— conflict notes.md`, and the width it was
/// given is the banner's own region, which is 120 cells on a 120-column
/// screen (detail pane below) and about 92 on a 160-column one (detail pane
/// beside). So the same repository in the same state spelled itself two
/// ways, and the *wider* terminal got the terser spelling. Counting from
/// the banner's own room was right (ADR-0039's amendment); wording a
/// fourteen-cell fact in two ways was not. The text is now short enough to
/// fit everywhere, so there is nothing left to tier.
///
/// No `⚠` either: a stopped merge is a state to finish, not a failure, and
/// CLAUDE.md's rule is that a status field must never read as an error when
/// nothing is wrong. `⚠` stays for the Mistake Radar's real findings
/// (`GraphScreen::marker_glyph`, `OverviewStrip`, the Map screen), which is
/// what keeps it meaning something.
#[must_use]
pub fn banner_line(banner: &BannerView) -> String {
    let mut line = String::from(banner.kind.as_str());
    if let Some((step, total)) = banner.step {
        line.push_str(&format!(" step {step} of {total}"));
    }
    line
}

/// A Mistake Radar marker, painted on a row rather than laid out with it
/// (`docs/design-notes/m2-details-and-diffs.md`, hard-to-reverse choice 2):
/// only the severity the glyph and its colour need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkerView {
    /// How loud the marker draws: `⚠`/warning colour, or `ⓘ`/muted.
    pub severity: Severity,
}

/// What the terminal said about the graph images, named by state: `gfx ok`
/// when it reported nothing wrong, `gfx: <message>` when it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphicsStatus {
    /// No image command failed since the last wake.
    Ok,
    /// The terminal refused an image command, and said this.
    Failed(SanitizedText),
}

/// What the history on screen leaves out (PROMPT.md §1.3). The default is
/// nothing: every commit, every parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    /// Only first parents are followed.
    pub first_parent: bool,
    /// How many merges are drawn folded.
    pub folded: usize,
    /// The stash is hidden.
    pub stash_hidden: bool,
    /// Focus mode is on (DESIGN.md §5.2): rows off the selection's thread
    /// fade. Nothing is left out, but the header says so, with its way out.
    pub focused: bool,
}

impl Filters {
    /// The header's words for these filters: empty when nothing is left out.
    #[must_use]
    pub fn text(self) -> String {
        let mut parts = Vec::new();
        if self.first_parent {
            parts.push("first-parent".to_owned());
        }
        match self.folded {
            0 => {}
            1 => parts.push("1 merge folded".to_owned()),
            many => parts.push(format!("{many} merges folded")),
        }
        if self.stash_hidden {
            parts.push("stash hidden".to_owned());
        }
        if self.focused {
            parts.push("focus (F/Esc off)".to_owned());
        }
        parts.join(" · ")
    }
}

/// Lane compaction as the screen applies it (DESIGN.md §3.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compaction {
    /// `graph.max_lanes`: the most lanes drawn, the `+N` marker included.
    pub max_lanes: usize,
    /// The user pressed the expand key: every lane draws in its real column
    /// until they press it again. The rows are the same either way — the
    /// fold is a lens over them, never a re-layout.
    pub expanded: bool,
    /// Columns a fold weighs above the ordinary rank: the selected commit's
    /// own column, reserved integration lanes and the selection's thread
    /// (ADR-0025). A priority, not a veto — one still folds if the budget
    /// cannot hold it.
    pub keep: Keep,
}

impl Compaction {
    /// Compaction at `max_lanes`, collapsed, with nothing kept but lane 0.
    #[must_use]
    pub fn at(max_lanes: usize) -> Self {
        Compaction {
            max_lanes,
            expanded: false,
            keep: Keep::default(),
        }
    }

    /// The fold for `rows`, whether or not it is applied.
    pub(super) fn plan<'a>(&self, rows: impl IntoIterator<Item = &'a Row>) -> Fold {
        let rows: Vec<Row> = rows.into_iter().cloned().collect();
        Fold::plan(self.max_lanes, &self.keep, &rows)
    }
}

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
    /// (`graft_core::oid::Oid`'s own doc comment).
    pub sha: String,
    /// This row is the pinned compare base (`x`, m2-details-and-diffs/11):
    /// draws the accent `▏` left rule. At most one row is ever `true`.
    pub compare_marked: bool,
    /// The Mistake Radar marker for this commit, painted in the marker
    /// column (`view::radar::marker_for`, m2-details-and-diffs/13). `None`
    /// on almost every row.
    pub marker: Option<MarkerView>,
}

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

/// What the header says about the order the rows are in, when it is not the
/// final one (ADR-0008). Absent when there is nothing to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderNotice {
    /// Rows are in commit-date order while the topological order is worked
    /// out; `Esc` stops that and keeps this order.
    Sorting,
    /// The sort was stopped or failed: the rows stay in commit-date order.
    Provisional,
}

/// Where the history walk is, as the header says it. Each state is worded
/// for itself: a walk that is fine never reads as an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkView {
    /// No walk has started yet: the repository is still opening.
    Opening,
    /// The walk is working: the screen asked for rows it has not had yet.
    /// The one state (with `Opening` and `Jumping`) in which `Esc` stops it.
    Reading {
        /// Rows laid out so far.
        laid_out: usize,
    },
    /// Not every commit has a row, and nothing is asked: the walk waits for
    /// the screen to want more. `Esc` leaves it alone — nothing is running.
    Paused {
        /// Rows laid out so far.
        laid_out: usize,
    },
    /// `G` is walking to the end of history; this many rows so far.
    Jumping {
        /// Rows laid out so far.
        laid_out: usize,
    },
    /// Every commit has a row.
    Complete {
        /// How many rows history has.
        rows: usize,
    },
    /// The user stopped the walk; the rows already laid out stay.
    Cancelled {
        /// Rows laid out when it stopped.
        laid_out: usize,
    },
    /// The walk broke, and this is why.
    Failed(SanitizedText),
}

/// The graph screen, ready to render.
#[derive(Clone, Copy, Debug)]
pub struct GraphScreen<'a> {
    view: &'a GraphView<'a>,
    theme: &'a Theme,
}

impl<'a> GraphScreen<'a> {
    /// Draw `view` in `theme`.
    #[must_use]
    pub fn new(view: &'a GraphView<'a>, theme: &'a Theme) -> Self {
        GraphScreen { view, theme }
    }
}

/// What the footer says about search: the prompt being typed, or how the
/// search stands. Only a failed search reads as a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchLine {
    /// The `/` prompt: the query typed so far.
    Prompt(SanitizedText),
    /// A running or finished search: its line, and whether it failed.
    Status {
        /// Query, state, position and keys.
        text: SanitizedText,
        /// The search failed; drawn as a warning.
        failed: bool,
    },
}

/// What the commit-graph offer's footer line says (m1-the-graph/54).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitGraphLine {
    /// No commit-graph: `B` builds one, `X` declines for this repository.
    Offered,
    /// No commit-graph, and no usable git to build one with.
    NoGitToBuild,
    /// `git commit-graph write` is running.
    Writing,
    /// The write finished; the file holds this many commits.
    Written {
        /// Commits the new file holds.
        commits: u32,
    },
}
