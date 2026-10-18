//! What the update arms and the view ask of [`AppState`]: the tier, the
//! graph's rows, the Uncommitted row, focus and the overlay on screen.

use mergewright_core::capabilities::RenderingTier;

use mergewright_ui::graph_screen::CommitGraphLine;

use crate::update::events::{CommitGraphOffer, GitCheck};

use super::{AppState, GraphOverlay, IndexRead};

impl AppState {
    /// The tier the graph draws at: `F3`'s override, else the settled one.
    #[must_use]
    pub fn tier(&self) -> RenderingTier {
        self.renderer_override
            .or(self.settled_tier)
            .unwrap_or(RenderingTier::Unicode)
    }

    /// How many rows of history the graph pane shows at the current size:
    /// [`Self::graph_pane_rows`] less the uncommitted row when it is pinned
    /// on top ([`Self::uncommitted_row_pinned`]). Never zero, so a movement
    /// always has somewhere to go. Every scroller passes this as the
    /// viewport height and the views draw exactly this many history rows,
    /// so the two can no longer disagree (m7-power-tools/48).
    #[must_use]
    pub fn graph_rows(&self) -> usize {
        self.graph_pane_rows() - usize::from(self.uncommitted_row_pinned())
    }

    /// Does the uncommitted row take the graph pane's top row? It is shown
    /// ([`Self::uncommitted_row_shown`]) and the pane has room for it and
    /// one history row. Every place that offsets drawn rows or click rows
    /// by the uncommitted row reads this. In a one-row pane it is false:
    /// the single row draws the cursor's row, which is the uncommitted row
    /// only while the cursor rests on it (m7-power-tools/48, Decided 4).
    #[must_use]
    pub fn uncommitted_row_pinned(&self) -> bool {
        self.uncommitted_row_shown() && self.graph_pane_rows() >= 2
    }

    /// Rows of the graph pane: the terminal less chrome, banner,
    /// commit-graph line, tabs, preview box and pending block. Never zero.
    /// The mouse's bounds check reads this; scrollers do not.
    #[must_use]
    pub fn graph_pane_rows(&self) -> usize {
        let chrome = mergewright_ui::graph_screen::chrome_rows(self.size.rows)
            .saturating_add(self.harness.banner_rows())
            .saturating_add(u16::from(self.commit_graph_line().is_some()))
            // Below 100 columns with the graph as the tab shown, the
            // `[Graph] Detail` strip is a row the graph never gets — and
            // so is the operation banner's row when one is drawn above it.
            // Neither was a term here, so `j`/`k` and every mouse row ran
            // past what the graph actually drew (review M1).
            .saturating_add(crate::update::banner::tab_rows_above_graph(self))
            // The tour card's three rows (m8/19b), carved under the banner.
            .saturating_add(crate::update::tour::card_rows(self))
            // The Ghost Preview's box takes rows out of the graph as the footer
            // and the commit-graph line do: rows the graph never drew, not rows
            // drawn and painted over, which hid the very ghosts it describes.
            .saturating_add(crate::update::preview::preview_rows_taken(self))
            // The done line's own row above a landing bar (ease-pass/08s M4).
            .saturating_add(crate::update::commit_editor::done_row(self).rows_taken())
            // A rewrite stop's still-to-apply block, drawn between the
            // header and the rows (review m7-10-to-07c Major 2).
            .saturating_add(crate::update::banner::pending_rows_taken(self));
        usize::from(self.size.rows.saturating_sub(chrome)).max(1)
    }

    /// What the graph screen's row carve (`mergewright_ui::graph_screen::row_carve`)
    /// asks of this state, as plain values: the reads the view's `GraphView`
    /// is built from, so `geometry::graph_carve` carves the rows drawn.
    #[must_use]
    pub(crate) fn carve_inputs(&self) -> mergewright_ui::graph_screen::CarveInputs {
        mergewright_ui::graph_screen::CarveInputs {
            status_row: crate::update::commit_editor::done_row(self),
            spaced: mergewright_ui::graph_screen::spaced(self.size.rows),
            banner_rows: self.harness.banner_rows(),
            commit_graph: self.commit_graph_line().is_some(),
            tour: self.tour.is_some(),
            pending_rows: crate::update::banner::pending_view(self).map_or(0, |view| {
                mergewright_ui::still_to_apply::still_to_apply_height(&view)
            }),
        }
    }

    /// The commit-graph offer's footer line, if it shows one. Waits for the
    /// git check before offering, so the line never offers and then takes
    /// it back.
    #[must_use]
    pub fn commit_graph_line(&self) -> Option<CommitGraphLine> {
        match self.commit_graph {
            CommitGraphOffer::Offered | CommitGraphOffer::Declining(_) => {
                match self.system_git.as_ref()? {
                    GitCheck::Ran(mergewright_git::version::SystemGit::Supported(_)) => {
                        Some(CommitGraphLine::Offered {
                            keys: crate::update::commit_graph::offer_keys_live(self),
                        })
                    }
                    GitCheck::Ran(mergewright_git::version::SystemGit::TooOld { .. })
                    | GitCheck::Unavailable(_) => Some(CommitGraphLine::NoGitToBuild),
                }
            }
            CommitGraphOffer::Writing(_) => Some(CommitGraphLine::Writing),
            CommitGraphOffer::Written { commits } => Some(CommitGraphLine::Written { commits }),
            CommitGraphOffer::NotNeeded
            | CommitGraphOffer::Declined
            | CommitGraphOffer::Failed
            | CommitGraphOffer::Suppressed => None,
        }
    }

    /// The offer can be acted on: offered, with a git new enough to build.
    pub(in crate::update) fn commit_graph_actionable(&self) -> bool {
        self.commit_graph == CommitGraphOffer::Offered
            && matches!(
                self.system_git,
                Some(GitCheck::Ran(
                    mergewright_git::version::SystemGit::Supported(_)
                ))
            )
    }

    /// Does the uncommitted-changes row draw at the top of the viewport this
    /// frame? Whenever the working tree is dirty
    /// (m3-working-tree-undo-and-stash/04, design note "Changes pane").
    ///
    /// It used to require `HEAD` to be the topmost visible row, which meant
    /// scrolling down made the user's own uncommitted work disappear from
    /// the screen — the one row they can always act on, gone because they
    /// scrolled (`KNOWN_GAPS.md`, m2/06b). The row is pinned at row 0 now;
    /// what `HEAD`'s position still decides is only whether it is drawn
    /// hanging off `HEAD`'s lane or as a lone `◌`
    /// ([`Self::uncommitted_row_attached`]).
    ///
    /// A clean tree, or no status read yet, is still "no".
    #[must_use]
    pub fn uncommitted_row_shown(&self) -> bool {
        let Some(status) = self.working_tree.as_deref() else {
            return false;
        };
        !status.is_clean()
    }

    /// An index read for a stash clash's tie is out
    /// ([`AppState::clash_index`]); the harness waits read it, as they read
    /// [`AppState::refs_read`].
    #[must_use]
    pub fn clash_index_out(&self) -> bool {
        matches!(self.clash_index, IndexRead::Pending { .. })
    }

    /// Does the row sit on top of `HEAD`'s own lane this frame (the
    /// m1-the-graph/12b attachment rule): attached only while `HEAD` (or an
    /// unborn branch) is the topmost visible row. Otherwise the row still
    /// draws, as a lone `◌` with no lane line under it — there is no lane
    /// to join when the commit it would hang from is off screen.
    ///
    /// Refs unresolved (the job has not answered yet) reads as "not
    /// attached": guessing a lane and then retracting it would flicker.
    #[must_use]
    pub fn uncommitted_row_attached(&self) -> bool {
        let Some(history) = &self.history else {
            return false;
        };
        let Some(refs) = self.refs.as_deref() else {
            return false;
        };
        match refs.head() {
            None => true,
            Some(head_id) => history
                .visible(1)
                .first()
                .is_some_and(|row| row.commit.id == head_id),
        }
    }

    /// What `Enter` opens, and what a shown pane follows: the working-tree
    /// row while the cursor rests on it, else the commit under `History`'s
    /// cursor, else nothing (no history yet).
    #[must_use]
    pub fn selected_spec(&self) -> Option<crate::update::detail::DetailSpec> {
        if self.on_working_tree {
            return Some(crate::update::detail::DetailSpec::WorkingTree);
        }
        self.history
            .as_ref()
            .and_then(crate::history::History::selected)
            .map(|row| crate::update::detail::DetailSpec::Commit(row.commit.id))
    }

    /// The pane on screen is the changes pane — the only place the write
    /// keys mean anything (m3-working-tree-undo-and-stash/04). The footer
    /// offers them exactly here, so a key is never hinted where it does
    /// nothing.
    #[must_use]
    pub fn on_changes_pane(&self) -> bool {
        self.detail.shown
            && self.detail.spec == Some(crate::update::detail::DetailSpec::WorkingTree)
    }

    /// **Has the graph the keyboard?** `S`, `J`, `M` and `O` each open
    /// something that stands *where the graph stands*, so each is answered
    /// only while nothing else has focus — `stash::stash_key`,
    /// `journal::journal_key`, `map::map_key` and `strip::strip_key` all
    /// ask here rather than each spelling the condition out.
    ///
    /// `view::rows::hint_context` asks here too, which is the point of the
    /// function: the four keys were guarded four times over and the hint
    /// bar drew them anyway, so a focused pane advertised four keys that
    /// did nothing (m5/31). One answer, read by the handlers and by the bar.
    #[must_use]
    pub fn graph_has_focus(&self) -> bool {
        self.detail.focus == mergewright_ui::detail::DetailFocus::Graph
    }

    /// **Which overlay is standing where the graph would be**, if any.
    ///
    /// The one enumeration of them. `view::frame::draw_screen` matches on
    /// it to decide what to draw, and `update::banner::graph_screen_on_frame`
    /// asks it whether there is a graph for the operation banner to ride on
    /// — two answers that must agree, and used to be two lists that could
    /// not (review M3, m5/31). A fifth overlay is a fifth variant, and the
    /// frame's match stops compiling until it is drawn; the banner needs no
    /// edit at all, because its condition is this function.
    ///
    /// Order is the frame's precedence, not a preference: the commit editor
    /// covers the Map, the Map covers the journal, and so on down.
    #[must_use]
    pub fn graph_overlay(&self) -> Option<GraphOverlay> {
        if self.finish.is_some() {
            // Above the resolver: finish-from-the-resolver opens the summary
            // with the resolver still open underneath, and `Esc` goes back
            // to it (m6-conflicts/19).
            Some(GraphOverlay::Finish)
        } else if self.resolver.is_some() {
            // Above the commit editor and the three overlays alike: the
            // resolver is the only one that covers the **detail pane** as
            // well as the graph, and `⏎` on a `CONFLICTS` row reaches
            // `detail::keys` with the Map, the journal or the stash list
            // already up. It refuses to open over a confirm box or the
            // editor (`resolver::open_resolver`), so the order below it is
            // untouched.
            Some(GraphOverlay::Resolver)
        } else if self.trail.place_shown() {
            // A place (blame or history) covers the graph and the pane, as the resolver
            // does; it never opens over the editor or a box (its `B` is
            // the pane's, and those claim the keys first).
            Some(GraphOverlay::Place)
        } else if self.rewrite.editor.is_some() {
            // Never opens over the others above (its `R` is a graph key,
            // and each of them claims keys first); modal, the whole frame
            // (m7-power-tools/08).
            Some(GraphOverlay::Rewrite)
        } else if self.commit_editor.is_some() {
            Some(GraphOverlay::CommitEditor)
        } else if self.map.shown {
            Some(GraphOverlay::Map)
        } else if self.journal.shown {
            Some(GraphOverlay::Journal)
        } else if self.stash.shown {
            Some(GraphOverlay::Stash)
        } else {
            None
        }
    }
}
