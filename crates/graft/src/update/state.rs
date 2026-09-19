//! Everything Graft knows, as one value, plus the small types it is built
//! from.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::capabilities::RenderingTier;
use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;

use graft_ui::context_menu::ContextMenu;
use graft_ui::graph_screen::CommitGraphLine;

use crate::history::History;
use crate::repository::RepositoryFacts;
use crate::session::Clock;

use super::events::{CommitGraphOffer, GitCheck};

/// Screen rows that are not the graph: one line of header above it and one of
/// status below. The graph screen (DESIGN.md §1.3) draws within this.
pub const CHROME_ROWS: u16 = graft_ui::graph_screen::CHROME_ROWS;

/// Everything Graft knows, as one value.
///
/// Minimal for now, on purpose: this ticket builds the loop, not the
/// application. Screens, the repository and the selection arrive with the
/// tickets that need them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppState {
    /// Set once Graft is leaving, and why. The event loop reads it after every
    /// event and returns, which drops the `TerminalSession` and hands the
    /// terminal back — a fatal quit is reported only after that, on a terminal
    /// that is the user's again.
    pub quit: Option<Quit>,
    /// What a harness wrapping the run asked for; default in the shipped
    /// binary.
    pub harness: Harness,
    /// What the status line has to say.
    pub status: Status,
    /// The terminal's last reported size, in character cells.
    pub size: Size,
    /// The repository Graft was pointed at. Empty in a default state, which is
    /// only ever a starting point: `event_loop::run` fills it in before the
    /// first frame.
    pub repo_path: PathBuf,
    /// What the repository turned out to be, once the job that opened it has
    /// answered. `None` until then, which is the first frame.
    pub repository: Option<RepositoryFacts>,
    /// What the system `git` turned out to be, once that job has answered.
    pub system_git: Option<GitCheck>,
    /// Where Graft's offer to build a missing commit-graph file is
    /// (m1-the-graph/54, ADR-0008).
    pub commit_graph: CommitGraphOffer,
    /// The history job whose answer the screen is waiting for.
    ///
    /// One at a time, on purpose: asking again supersedes the previous ask,
    /// and a result carrying any other id belongs to a question that has
    /// since been withdrawn. That is what makes a stale result harmless — it
    /// is dropped here, before it can reach a screen.
    pub awaiting: Option<JobId>,
    /// The topological sort whose order the screen will promote to, while
    /// it runs (ADR-0008). A result from any other sort is dropped.
    pub promoting: Option<JobId>,
    /// The history walk the screen shows: its rows so far and where it is.
    /// `None` until one has been asked for.
    pub history: Option<History>,
    /// The refs on each commit, once the refs job has answered. Shared, not
    /// owned: every event clones the state, and a repository with 50k refs
    /// made copying them the dearest part of a frame (m1-the-graph/69).
    pub refs: Option<Arc<crate::refs::RefBadges>>,
    /// The working tree's full status, once that job has answered. `None`
    /// until then, and while it fails: a failed read goes on the status line
    /// rather than showing a stale count. Shared, not owned — the changes
    /// pane's file list is built from the same `Arc` rather than a copy
    /// (m3-working-tree-undo-and-stash/04, mirroring `AppState::refs`).
    pub working_tree: Option<Arc<graft_git::status::WorkingTreeStatus>>,
    /// The working-tree write whose answer the screen is waiting for
    /// (`Space`/`s`/`Ctrl+A`/`d` on the changes pane). One at a time: every
    /// other write key is a no-op while this is `Some`
    /// (m3-working-tree-undo-and-stash/04).
    pub operating: Option<JobId>,
    /// Which screen is on show. The graph, unless the user asked for the
    /// status screen — the only place tier and cell size are shown.
    pub screen: Screen,
    /// The user pressed `+`: every lane draws in its real column instead of
    /// folding behind `+N` (DESIGN.md §3.5). A view-time flag only — the
    /// rows are the same either way, so toggling it asks nothing of the
    /// history job.
    pub lanes_expanded: bool,
    /// Focus mode (`F`, DESIGN.md §5.2): rows off the selection's thread
    /// fade. A paint decision like `lanes_expanded`; the walk is not asked.
    pub focus: bool,
    /// The pointer's row: an absolute history row number, set by pointer
    /// movement over the graph (DESIGN.md §2.15, PROMPT.md §1.3;
    /// m1-the-graph/17, ADR-0023). `None` when the pointer is not over a
    /// row. While a row's subject is cut with `…`, the footer reveals it in
    /// full in place of the key hints (`graft_ui::graph_screen::graph_view`
    /// turns this into `GraphView::reveal`).
    pub hover: Option<usize>,
    /// The detail pane (ADR-0023: `Enter` opens it); hidden by default.
    pub detail: super::detail::DetailState,
    /// Ids for the jobs `update` names itself (the detail pane's).
    pub local_jobs: super::detail::LocalJobs,
    /// The tier-drop key (`F3`) forced this tier for the rest of the run, or
    /// `None` for whatever `Session::settle` decided. A view-time override
    /// like `lanes_expanded` — the rows are untouched, only how they draw
    /// changes (`graph.renderer`, m1-the-graph/19).
    pub renderer_override: Option<RenderingTier>,
    /// The tier `Session::settle` decided, copied in when the loop starts so
    /// `update` knows what `F3` drops from and whether row requests pad to
    /// pixel strips. `None` in a state built without a session (tests,
    /// headless), which draws as the Unicode tier would.
    pub settled_tier: Option<RenderingTier>,
    /// The `/` prompt while it is open: what has been typed so far.
    pub search_prompt: Option<String>,
    /// The search on screen, running or finished, until `Esc` closes it.
    pub search: Option<crate::search::Search>,
    /// The cursor rests on the uncommitted-changes row, not on a `History`
    /// row (`update::selection`, m2-details-and-diffs/06b). `true` only
    /// while [`AppState::uncommitted_row_shown`] holds — `selection::reconcile`
    /// clears it the moment that stops being so.
    pub on_working_tree: bool,
    /// The pinned base of a live compare (`x`, m2-details-and-diffs/11):
    /// `None` while nothing is pinned. Set by `super::detail::toggle_compare`
    /// and read by `super::selection::follow_selection` to keep the compare
    /// live as the cursor moves.
    pub compare_base: Option<super::detail::CompareBase>,
    /// The `.`/right-click popup (`context-actions-menu`,
    /// m2-details-and-diffs/12): the row it opened on (`update::context_menu::WORKING_TREE_ROW`
    /// for the uncommitted-changes row) and the menu itself. `None` while
    /// closed.
    pub context_menu: Option<(usize, ContextMenu)>,
    /// The Mistake Radar's own state: the default-branch context it needs
    /// before it can find anything, and the policy `[radar]` config resolved
    /// to (`update::radar`, m2-details-and-diffs/13).
    pub radar: super::radar::RadarState,
    /// Sticky once a row's marker has been `Some` this session: the marker
    /// column keeps showing even after the row that earned it scrolls
    /// off-screen and is evicted from `History::rows`'s window
    /// (`view::rows`, m2-details-and-diffs/13b). Set from `view::entries`,
    /// which is the only writer; everything else only reads it.
    pub radar_column_seen: std::cell::Cell<bool>,
    /// The Map view (`M`, DESIGN.md §5.1, m2-details-and-diffs/18): hidden
    /// until pressed, built fresh each time and thrown away on close.
    pub map: super::map::MapState,
    /// The overview strip (`O`, DESIGN.md §5.3, m2-details-and-diffs/20):
    /// off until asked (no config default, M8 polish). Drawn only while
    /// this holds and [`super::map::MapState::shown`] does not.
    pub strip_shown: bool,
    /// The confirmation box (`d` discard, PROMPT.md §3 "destructive
    /// actions"): `None` while closed. Modal, like the context menu — its
    /// own keys answer first and nothing else sees them
    /// (m3-working-tree-undo-and-stash/04).
    pub confirm: Option<super::confirm::PendingConfirm>,
    /// The commit editor (`c`, `a`): `None` while closed. Modal like the
    /// confirm box — while it is open every key is the editor's, bar `Ctrl+C`
    /// (m3-working-tree-undo-and-stash/05).
    pub commit_editor: Option<super::commit_editor::CommitEditor>,
    /// What `Esc` kept when it closed a non-empty editor, so the next `c`
    /// this session picks the message back up rather than losing it.
    pub commit_draft: Option<super::commit_editor::CommitEditor>,
    /// The merge, rebase, cherry-pick, revert or bisect the repository is
    /// partway through, as the last working-tree read found it. Commit and
    /// amend refuse while it is `Some` and the refusal names the shell
    /// command that finishes it (ADR-0034; M5 does the real handling).
    pub in_progress: Option<graft_git::state::InProgress>,
    /// The `HEAD`-message read an `a` is waiting on: the editor opens on its
    /// answer, so there is never a half-filled amend on screen. Any other
    /// id's answer is dropped (m3-working-tree-undo-and-stash/05).
    pub head_message: Option<JobId>,
    /// The `commit.template` read an open editor is waiting on, for the same
    /// reason.
    pub commit_template: Option<JobId>,
    /// The `Ctrl+E` handover an open editor is waiting on. What a `$EDITOR`
    /// saved is dropped unless this still names it: `Esc` in between means
    /// the user moved on, and dropping it beats overwriting a field they
    /// have since retyped.
    pub external_editor: Option<JobId>,
    /// `[keys] suspend_on_ctrl_z`: `Ctrl+Z` stops Graft instead of undoing
    /// (ADR-0002). Seeded from config at startup; off by default.
    pub suspend_on_ctrl_z: bool,
}

/// The screens Graft can show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    /// The history graph. What Graft opens on.
    #[default]
    Graph,
    /// What Graft opened and what it is drawing with.
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

impl AppState {
    /// The tier the graph draws at: `F3`'s override, else the settled one.
    #[must_use]
    pub fn tier(&self) -> RenderingTier {
        self.renderer_override
            .or(self.settled_tier)
            .unwrap_or(RenderingTier::Unicode)
    }

    /// How many rows of history the graph pane shows at the current size.
    /// Never zero, so a movement always has somewhere to go.
    #[must_use]
    pub fn graph_rows(&self) -> usize {
        let chrome = CHROME_ROWS
            .saturating_add(self.harness.banner_rows())
            .saturating_add(u16::from(self.commit_graph_line().is_some()));
        usize::from(self.size.rows.saturating_sub(chrome)).max(1)
    }

    /// The commit-graph offer's footer line, if it shows one. Waits for the
    /// git check before offering, so the line never offers and then takes
    /// it back.
    #[must_use]
    pub fn commit_graph_line(&self) -> Option<CommitGraphLine> {
        match self.commit_graph {
            CommitGraphOffer::Offered | CommitGraphOffer::Declining(_) => {
                match self.system_git.as_ref()? {
                    GitCheck::Ran(graft_git::version::SystemGit::Supported(_)) => {
                        Some(CommitGraphLine::Offered)
                    }
                    GitCheck::Ran(graft_git::version::SystemGit::TooOld { .. })
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
        self.commit_graph_line() == Some(CommitGraphLine::Offered)
            && self.commit_graph == CommitGraphOffer::Offered
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
    pub fn selected_spec(&self) -> Option<super::detail::DetailSpec> {
        if self.on_working_tree {
            return Some(super::detail::DetailSpec::WorkingTree);
        }
        self.history
            .as_ref()
            .and_then(crate::history::History::selected)
            .map(|row| super::detail::DetailSpec::Commit(row.commit.id))
    }

    /// The pane on screen is the changes pane — the only place the write
    /// keys mean anything (m3-working-tree-undo-and-stash/04). The footer
    /// offers them exactly here, so a key is never hinted where it does
    /// nothing.
    #[must_use]
    pub fn on_changes_pane(&self) -> bool {
        self.detail.shown && self.detail.spec == Some(super::detail::DetailSpec::WorkingTree)
    }
}

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

/// Why Graft is leaving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Quit {
    /// The user asked to.
    Requested,
    /// The user pressed a key the caller asked to have handed back
    /// (`Harness::hand_back`), so the loop stops for the caller to act.
    HandedBack(KeyCode),
    /// Graft cannot go on, and this is the line to print. The terminal is
    /// restored first: a message written into the alternate screen vanishes
    /// with it, which is how "it just exits and says nothing" happens.
    Fatal(String),
}

impl AppState {
    /// Is the loop finished?
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit.is_some()
    }
}

/// What the status line has to say.
///
/// Modelled as a state rather than as a string with an `error:` prefix: a
/// field that always looks like an error teaches the user to ignore it, which
/// is how a real failure gets missed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing to report.
    #[default]
    Ready,
    /// A background job failed, and this is what it said — a read, not
    /// something the user asked for. Shown until a later job succeeds
    /// (`JobFinished`) or `Esc` dismisses it; a failure is never dropped on
    /// the floor.
    Failed(String),
    /// A **write** the user asked for failed, and this is what it said: a
    /// hook that rejected a commit, a patch git would not apply, a discard
    /// that ran part-way.
    ///
    /// Apart from [`Status::Failed`] because the two are stale for different
    /// reasons. A read failure is superseded by the next read that works, so
    /// `JobFinished` clears it. A write failure is not: the refresh the
    /// failure itself triggers finishes a moment later, and clearing on that
    /// wiped a rejected hook's message off the screen before the user could
    /// read it (m3/04 review, `KNOWN_GAPS.md`). It stays until `Esc`
    /// (m3-working-tree-undo-and-stash/05).
    WriteFailed(String),
    /// `y` sent the selected commit's SHA to the terminal's clipboard
    /// request (OSC 52); `short` is its first seven hex digits. Says what
    /// Graft sent, not that the clipboard has it: nothing comes back to
    /// tell (ADR-0023). Cleared by the next key.
    Copied {
        /// The SHA's first seven hex digits.
        short: String,
    },
    /// A working-tree write is running: `<description>…`
    /// (m3-working-tree-undo-and-stash/04).
    Busy(String),
    /// A working-tree write finished, in the journal's own words (`stage
    /// src/a.rs`). Cleared by the next key, like `Copied`.
    Done(String),
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
