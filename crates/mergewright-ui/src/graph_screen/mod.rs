//! The screen Mergewright opens on: the history graph, one commit per row.
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
//! plain-data types the other files read, with [`view`] (the
//! [`GraphView`]) and [`entry`] (one row) beside it; [`layout`] the geometry
//! and header, [`lanes`] how many lanes fit, [`walk_text`] the walk's words,
//! [`row_columns`] where the row columns stand; [`footer`] the footer's text
//! (search, notices, the commit-graph offer, key hints); [`render`] the
//! actual drawing.

pub mod badges;
mod banner;
mod body_line;
mod carve;
pub mod columns;
mod compaction;
mod entry;
mod fold_mark;
mod footer;
mod header;
mod header_tail;
pub mod hints;
mod lanes;
mod layout;
mod render;
mod row_cells;
mod row_columns;
mod row_text;
#[cfg(test)]
mod tests;
mod view;
mod walk_text;

use mergewright_core::mistake_radar::Severity;
use mergewright_term::SanitizedText;

pub use badges::{Badge, BadgeKind, Icons, Tracking};
pub use banner::{BannerChain, BannerView, RewriteBannerView, banner_line};
pub use body_line::{BodyLine, body_line, commit_rows};
pub use carve::{CarveInputs, RowCarve, row_carve};
pub use columns::{
    absolute_date, first_name, initials, relative_date, relative_date_long, short_sha,
};
pub use compaction::Compaction;
pub(crate) use entry::TAG_GAP;
pub use entry::{BranchCell, GraphEntry, RowTag, SecondLine, TAG_MIN_SUBJECT, WhoCell};
pub use fold_mark::{FoldCount, FoldMark};
pub use header::{APP_NAME, HeaderView, header_line, tail_room};
pub use header_tail::LaneTally;
pub use hints::{
    ConfirmYes, Divider, Hint, HintContext, HintKey, HintState, LiveC, NOT_A_BINDING, Rung,
    StatusRow, StopWording, TabWord, WayToBanner, blame_place_list, every_context, every_hint,
    fit_status_line, hints_for, hints_text, status_row, stopped_row, stopped_row_before_keys,
};
pub use lanes::{lanes_that_fit, max_lanes_for};
pub use layout::{RowLayout, ScreenRows, row_at, row_layout, screen_rows};
pub use row_columns::{Columns, branch_width_for, snap_to_columns};
pub use row_text::{badges_that_fit, label_words, ref_spans};
pub use view::GraphView;

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

/// The branch column's width on a row 120 cells or wider (ease-pass G5).
pub const BRANCH_WIDE: u16 = 20;

/// The branch column's width on a row 100–119 cells wide.
pub const BRANCH_MIDDLE: u16 = 14;

/// The branch column's width on a row under 100 cells.
pub const BRANCH_NARROW: u16 = 12;

/// Cells the who column takes: a first name, or `you` (ease-pass G8).
pub const WHO_WIDTH: u16 = 6;

/// Cells the age column takes: `23h`, `9d`, `2w`, `10mo` (ease-pass G8).
pub const AGE_WIDTH: u16 = 4;

/// The blank cell between two row columns (the mock's `gap: 1ch`).
const COLUMN_GAP: u16 = 1;

/// How many hex digits `short_sha` is called with wherever a short SHA is
/// spelled (`y`, the detail pane, the bar); the row no longer draws one
/// (ease-pass G8).
pub const SHA_WIDTH: u16 = 7;

/// Screen rows that are not graph rows on a screen under 30 rows: the header
/// above, the key hints below. A harness banner ([`GraphView::banner`]) adds
/// one row per line. Taller screens add a blank row and a rule
/// ([`chrome_rows`]); read that, not this, wherever the height is known.
pub const CHROME_ROWS: u16 = 2;

/// From this many rows the chrome breathes: a blank row under the header and
/// a rule above the bar (ADR-0058 §4). Below it the rows go to the list.
pub const SPACED_FROM_ROWS: u16 = 30;

/// Does a screen `rows` tall draw the blank row and the rule?
#[must_use]
pub const fn spaced(rows: u16) -> bool {
    rows >= SPACED_FROM_ROWS
}

/// Screen rows that are not graph rows on a screen `rows` tall: the header
/// and the bar, plus the blank row and the rule when [`spaced`].
#[must_use]
pub const fn chrome_rows(rows: u16) -> u16 {
    if spaced(rows) {
        CHROME_ROWS + 2
    } else {
        CHROME_ROWS
    }
}

/// Cells left of every list row: the cursor glyph's slot and a gap
/// (ADR-0058 §4). The glyph itself is drawn by the cursor-row ticket.
pub const GUTTER: u16 = 2;

/// Cells right of every row: one of margin.
pub const RIGHT_MARGIN: u16 = 1;

/// Where the header says the screen is: the repository, its path and the
/// branch `HEAD` is on. Sanitised by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    /// The repository's name: its folder.
    pub repo: SanitizedText,
    /// Its path, `~`-abbreviated.
    pub path: SanitizedText,
    /// The branch `HEAD` is on, or `detached <sha7>`.
    pub branch: SanitizedText,
    /// The practice repository (ADR-0056): `◇` for `◆`.
    pub practice: bool,
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
    /// The stash is hidden.
    pub stash_hidden: bool,
    /// Focus mode is on (DESIGN.md §5.2): rows off the selection's thread
    /// fade. Nothing is left out, but the header says so, with its way out.
    pub focused: bool,
    /// The graph mode is Every branch (ease-pass 06h): nothing folds. Not
    /// one of the filters' words; the lane phrases say it.
    pub every_branch: bool,
}

impl Filters {
    /// The header's words for these filters: empty when nothing is left out.
    /// Focus names its way out, `graph.focus` and `app.back` spelled from
    /// `keys` (`focus (F/esc off)`), or `focus` alone when both are unbound.
    #[must_use]
    pub fn text(self, keys: &crate::keymap::Keymap) -> String {
        let mut parts = Vec::new();
        if self.first_parent {
            parts.push("first-parent".to_owned());
        }
        if self.stash_hidden {
            parts.push("stash hidden".to_owned());
        }
        if self.focused {
            let off = [
                crate::keymap::ActionId("graph.focus"),
                crate::keymap::ActionId("app.back"),
            ];
            parts.push(match keys.spell_keys(&off) {
                Some(off) => format!("focus ({off} off)"),
                None => "focus".to_owned(),
            });
        }
        parts.join(" · ")
    }
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

/// What the footer says about search: the prompt being typed. A search
/// that stands after `⏎` is the header's [`SearchTail`] instead (m8/15b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchLine {
    /// The `/` prompt: the query typed so far.
    Prompt(SanitizedText),
}

/// A running or finished search, as the header's tail draws it (m8/15b,
/// ticket 15 A2): its line verbatim, leftmost in the tail. Only a failed
/// search reads as a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchTail {
    /// Query, state, position and keys.
    pub text: SanitizedText,
    /// The search failed; drawn as a warning.
    pub failed: bool,
}

/// What the commit-graph offer's footer line says (m1-the-graph/54).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitGraphLine {
    /// No commit-graph: `B` builds one, `X` declines for this repository.
    Offered {
        /// The `B`/`X` keys are live and drawn: only while the graph has the
        /// keyboard (m7-power-tools/36). Without, the row is the fact alone.
        keys: bool,
    },
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
