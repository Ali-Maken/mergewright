//! Everything Graft knows, as one value, plus the small types it is built
//! from.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::capabilities::RenderingTier;
use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;

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
    /// A failure that arrived while a modal owned the bottom row and was
    /// replaced before the row drew it — the review's `f`, `b`, fetch
    /// fails, `topic ⏎` (m6-conflicts/78, Decided 1). Drawn in place of
    /// `status` once the modal closes (`shown_status`), dismissed by the
    /// next key (`Esc` only dismisses). `None` whenever `status` itself
    /// still holds it once the row is the status's again. One slot: two
    /// failures under one box keep the later.
    pub(crate) unseen: Option<Status>,
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
    /// Whether [`AppState::refs`] answers the last refs read asked for.
    /// Asked beside every `Command::ReadRefs` (the launch, `keys::refresh_key`),
    /// answered where the refs land (`took_fact`). `P` reads it: the lease is
    /// offered only on current refs (m6-conflicts/49).
    pub refs_read: RefsRead,
    /// The working tree's full status, once that job has answered. `None`
    /// until then, and while it fails: a failed read goes on the status line
    /// rather than showing a stale count. Shared, not owned — the changes
    /// pane's file list is built from the same `Arc` rather than a copy
    /// (m3-working-tree-undo-and-stash/04, mirroring `AppState::refs`).
    pub working_tree: Option<Arc<graft_git::status::WorkingTreeStatus>>,
    /// The write whose answer the screen is waiting for. One at a time:
    /// set only by `update::operate::claim_write`, which refuses in words
    /// while this is `Some` (m3-working-tree-undo-and-stash/04;
    /// m6-conflicts/78).
    pub operating: Option<JobId>,
    /// The `--continue`/`--skip`/`--abort` that `operating` is waiting for,
    /// kept until its answer: the answer's entry alone cannot say what was
    /// asked — a failed continue closes `Aborted` too — so the closing words
    /// key on both (m6-conflicts/68). Set only by `banner::resume`; taken by
    /// `banner::resumed`, or by `operate::write_job_ended` when the job dies
    /// without answering.
    pub(crate) resuming: Option<(JobId, graft_ops::apply::Resume)>,
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
    /// Which glyphs ref badges draw with: `ui.icons`, seeded at startup
    /// (`crate::config::IconSet::glyphs`), plain unless the config file asks
    /// for Nerd Font icons (never detected, PROMPT.md §5).
    ///
    /// Here rather than on `Session` because the glyph set decides how wide
    /// each badge is, so it decides where a **drag** has to hit one. The
    /// frame and the mouse hit-test build the same `GraphView` from this one
    /// field; when it lived on `Session` the hit-test could not see it and
    /// measured a badge the frame never drew (m5-history-and-ghost-preview/33,
    /// review M1).
    pub icons: graft_ui::graph_screen::Icons,
    /// `graph.max_lanes`: the configured lane cap, or `None` to let the
    /// width breakpoint decide. Resolved per frame against the room the
    /// graph has (`graft_ui::graph_screen::max_lanes_for`), because the cap
    /// moves the graph column's width and so every badge's columns — which
    /// is why it lives beside [`AppState::icons`] rather than on `Session`.
    pub max_lanes: Option<usize>,
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
    /// The commits only a stash reaches, which the graph marks
    /// (m7-power-tools/16): read after every refs read that lists a stash.
    pub stash_only: StashOnlyMarks,
    /// The `.`/right-click popup (`context-actions-menu`,
    /// m2-details-and-diffs/12): the row it opened on (`update::context_menu::WORKING_TREE_ROW`
    /// for the uncommitted-changes row), the menu itself, and what each of
    /// its history rows means. `None` while closed.
    pub context_menu: Option<super::context_menu::OpenMenu>,
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
    /// The undo journal (`J`, ADR-0034, m3-working-tree-undo-and-stash/06):
    /// read at launch whether or not the browser is open, because the
    /// interrupted-entry notice depends on it.
    pub journal: super::journal::JournalState,
    /// The stash overlay as this run has read it, and its own cursor
    /// (m3-working-tree-undo-and-stash/06).
    pub stash: super::stash::StashState,
    /// The configured remotes, `HEAD`'s branch and its upstream: what `f`,
    /// `p` and `P` decide from (m4-remotes/05). Read with the refs; a
    /// re-read keeps the last answer on screen until the new one lands
    /// (`RemoteState::reading` holds the id in flight).
    pub remotes: super::detail::Loading<std::sync::Arc<crate::jobs::remote::RemoteFacts>>,
    /// The remote keys' own state: the read in flight, the write they
    /// launched, and the last rejected push (m4-remotes/05).
    pub remote: super::remote::RemoteState,
    /// When the last focus-gained refresh ran, Unix milliseconds. A
    /// terminal reports focus in pairs and in bursts, and every report is a
    /// working-tree read (m3-working-tree-undo-and-stash/06).
    pub last_focus_refresh: Option<i64>,
    /// The overview strip (`O`, DESIGN.md §5.3, m2-details-and-diffs/20):
    /// off until asked (no config default, M8 polish). Drawn only while
    /// this holds and [`super::map::MapState::shown`] does not.
    pub strip_shown: bool,
    /// The confirmation box (`d` discard, PROMPT.md §3 "destructive
    /// actions"): `None` while closed. Modal, like the context menu — its
    /// own keys answer first and nothing else sees them
    /// (m3-working-tree-undo-and-stash/04).
    pub confirm: Option<super::confirm::PendingConfirm>,
    /// git's question for the running fetch, pull or push — a username,
    /// password, passphrase or host-key answer (m4-remotes/06): `None` while
    /// nothing is asked. Modal, drawn over everything, and a text field: it
    /// is in [`AppState::text_field_open`]. Its `Debug` never prints what was
    /// typed.
    pub credential: Option<super::credential::CredentialPrompt>,
    /// The name prompt (`b` branch, `t` tag, `b` in the stash overlay):
    /// `None` while closed. Modal, and a text field — it is in
    /// [`AppState::text_field_open`].
    pub name_prompt: Option<super::name_prompt::NamePrompt>,
    /// A branch badge picked up with the left button and not yet dropped:
    /// `None` when nothing is in the air. Mouse-only; any key cancels it
    /// (m5-history-and-ghost-preview/10).
    pub drag: Option<super::drag::DragState>,
    /// The commit editor (`c`, `a`): `None` while closed. Modal like the
    /// confirm box — while it is open every key is the editor's, bar `Ctrl+C`
    /// (m3-working-tree-undo-and-stash/05).
    pub commit_editor: Option<super::commit_editor::CommitEditor>,
    /// What `Esc` kept when it closed a non-empty editor, so the next `c`
    /// this session picks the message back up rather than losing it.
    pub commit_draft: Option<super::commit_editor::CommitEditor>,
    /// The merge, rebase, cherry-pick, revert or bisect the repository is
    /// partway through, as the last working-tree read found it, with the step
    /// counts the banner words itself from. Commit and amend refuse while it
    /// is `Some` and the refusal names the keys that finish it (ADR-0034,
    /// m5-history-and-ghost-preview/09).
    pub in_progress: Option<graft_git::state::InProgressDetail>,
    /// `merge.tool` from git's config, as the same read found it. `None`
    /// means `M` refuses in words instead of launching `git mergetool` and
    /// letting git ask "which tool?" on a terminal Graft has just handed
    /// over, where the question has no way back to the user.
    pub mergetool: Option<String>,
    /// The operation banner has the keyboard: `c`/`s`/`a`/`M`/`e` act on the
    /// operation and every other key is swallowed
    /// (`docs/design/02-keymap.md` §2.1, §2.6).
    ///
    /// Set on the event that lands a conflict and at launch when the journal
    /// read finds a live entry, so a key-driven user never has to discover
    /// the banner; cleared by `Tab`/`BackTab`/`Esc` and whenever
    /// [`AppState::in_progress`] goes back to `None`. Never `true` without a
    /// banner to focus — [`crate::update::banner::banner_view`] is the one
    /// thing that draws it, and it answers `None` without `in_progress`.
    pub banner_focus: bool,
    /// A `--continue`/`--skip`/`--abort` answered `Done` and
    /// [`AppState::in_progress`] was cleared on that answer's word, ahead of
    /// the working-tree re-read (gap row 569). The next working-tree read
    /// spends it: agreeing is silent, and still finding the operation is
    /// said on the status line rather than quietly undoing the answer.
    pub ended_by_answer: bool,
    /// The journal entries git is still part-way through, as the last
    /// journal read found them
    /// ([`crate::jobs::undo::JournalRead::live`], m5/09 seam A).
    ///
    /// The whole point is the difference between "an operation was
    /// interrupted" and "an operation is standing right there": both are a
    /// `Pending` line with no outcome, and only git's own state tells them
    /// apart. An entry named here is **not** announced as interrupted and is
    /// not drawn as one — it is the banner's, and `c`/`a` end it.
    pub live_entries: Vec<graft_core::journal::EntryId>,
    /// From the launch's `Recovered::set_aside`
    /// ([`crate::jobs::undo::JournalRead::interrupted_edits`]): where each
    /// interrupted set-aside's edits were at recovery (30c). Lives as long
    /// as the notice (until `J`); empty after any later launch, when the
    /// notice keeps ` · edits in stash@{n}`.
    pub interrupted_edits: Vec<(graft_core::journal::EntryId, graft_ops::EditsAt)>,
    /// **The live operation stopped, and why** (m6-conflicts/12). A state,
    /// not a [`Status`]: `Status` is one slot, and during a live operation a
    /// refused `c`, a failed resume, `Busy` and `Done` all take it — a stop
    /// line living there was wiped by any of them and could not come back
    /// after `Esc` dismissed a real problem.
    ///
    /// Set by `operate::operated`'s `Conflicted`, by `banner::resumed`'s
    /// `Conflicted` (stopping again) and by `relaunch::land_on_live_operation`.
    /// Cleared by `undo::operation_ended`, and by a working-tree read that
    /// finds no operation at all (it ended in another terminal). `Esc`, a
    /// refresh and a `JobFinished` leave it. The footer words it each frame
    /// through `banner::stop_reason`, from the live read.
    pub stopped: Option<Stop>,
    /// **A stash put back that clashed, live now** (ADR-0045 §2,
    /// m6-conflicts/72b). Its own field, never a [`Stop`]: a `Stop` is an
    /// operation git has part-way (`Stop::kind`), and everything that reads
    /// `stopped` — the banner, `c`/`a`, `live_operation` — means that. A
    /// clash has no git state and no continue or abort.
    ///
    /// Set by `operate::operated`'s `Clashed` arm (`ThisRun`, so the landing
    /// never waits for a read) and by [`crate::update::clash::refresh_clash`]
    /// when a journal, working-tree or refs read lands (`PreviousRun` when
    /// the launch's reads find one); cleared by the same refresh once
    /// `graft_ops::clash::live_clash` finds none. Read by the resolver's
    /// names and close, and by the stop line while `stopped` is `None`.
    pub clash: Option<ClashState>,
    /// [`crate::update::clash::refresh_clash`] has run on a full set of
    /// facts (journal, working tree, refs) once: a clash it finds after this
    /// is this run's, one it finds before is from the last run.
    pub(crate) clash_facts_seen: bool,
    /// The `DropPutBack` the resolver's close launched after a popped
    /// stash's last file, the stash's position then, and why it was put
    /// back — what its answer's `every file done · stash@{0} dropped ✓`
    /// names (a set-aside's: `every file done · your edits are back ✓`,
    /// m6-conflicts/74).
    pub(crate) clash_drop: Option<(JobId, Option<usize>, graft_core::journal::PutBackWhy)>,
    /// The clash landing opened the resolver from the changes pane while
    /// the working tree held was still the one from before the put-back —
    /// clean, so the pane could not stay on it (`selection::reconcile`). The
    /// next working-tree read that shows the tree dirty lands the pane on
    /// `CONFLICTS` ([`crate::update::clash::refresh_and_land`]).
    pub(crate) clash_pane_owed: bool,
    /// The index now, as the clash's tie reads it (ADR-0045 §2, amended
    /// 2026-09-25; review 72+72b M2). Asked only while
    /// `graft_ops::clash::clash_candidate` finds one, one read out at a time;
    /// every journal, working-tree or refs read that lands makes the held
    /// answer stale ([`crate::update::clash::refresh_and_land`]).
    pub(crate) clash_index: IndexRead,
    /// A `--continue` finished (`resumed` → `Done`), or a write whose launch
    /// said it moves `HEAD` did ([`AppState::lands_on_head`]): the graph cursor goes
    /// to the new `HEAD` once the refresh that answer issued has both the
    /// fresh refs and the fresh walk ([`AppState::head_landing`]). Spent by
    /// `banner::land_on_head`.
    pub land_on_head: bool,
    /// The two reads [`AppState::land_on_head`] waits on, each `Some` until
    /// it has answered. Ids, not "the next answer": a refs read from before
    /// the `c` can land after `Done`, and it would put the cursor on the old
    /// `HEAD`. `Command::ReadRefs` carries an id `update` chose for exactly
    /// this (the refs answer had no id guard before).
    pub head_landing: HeadLanding,
    /// The write in `operating` whose finish moves `HEAD`
    /// (`Operation::moves_head`), so its `Done` owes the same landing as a
    /// `--continue` ([`AppState::land_on_head`], m7-power-tools/03). Decided
    /// at launch from the operation, never guessed from the answer. Beside
    /// `operating` rather than inside it, as `resuming` is: `operating` is
    /// a bare id read in hundreds of places.
    pub(crate) lands_on_head: Option<JobId>,
    /// A `show_line` still waiting on the graph's seek, the file list or the
    /// diff (`update::detail::show_line`, m7-power-tools/20b). Moved on by
    /// `update::detail::advance` after every event; dropped by a key or
    /// click that moves the graph cursor or the pane's selection.
    pub line_landing: Option<super::detail::LineLanding>,
    /// The blame places opened since `B`, and where `B` was pressed
    /// (`update::place`, m7-power-tools/20). Empty with no trail.
    pub trail: super::place::Trail,
    /// The launch found an operation live while a modal owned the keys, so
    /// it set [`AppState::stopped`] and left the keyboard alone. The focus
    /// landing runs on the first working-tree read after the modal closes;
    /// cleared then, or by `undo::operation_ended`.
    pub live_landing_owed: bool,
    /// An undo or redo answered `Done` with no operation live; the newest
    /// working-tree read after it decides whether it put one back — a
    /// finish taken back leaves git mid-merge again — and if so the banner
    /// takes the keyboard, as a launch into that state would (ticket 41).
    /// Spent by `update::undo::land_after_undo`.
    pub(crate) undo_landing_owed: bool,
    /// The newest working-tree read asked for and not yet answered, or
    /// `None` when [`AppState::working_tree`] is as fresh as anything
    /// asked for. While it is `Some`, what the status says may be stale —
    /// a write that just landed is not in it yet — so `c` does not count
    /// on it (`update::status_read`).
    pub status_read: Option<JobId>,
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
    /// The open Ghost Preview (`m`, `r`, a `.` menu verb, the journal's
    /// time machine): `None` while closed (m5-history-and-ghost-preview/08).
    ///
    /// Modal like the confirm box — while it is open every key is the
    /// preview's bar `Ctrl+C`, and the graph draws the plan's rows instead
    /// of the walk's. **Not** in [`AppState::text_field_open`]: nothing is
    /// typed into it, and its own handler runs before `u` regardless.
    pub preview: Option<super::preview::PreviewState>,
    /// The conflict resolver (`⏎` on a `CONFLICTS` row or on the focused
    /// banner, m6-conflicts/04): `None` while closed.
    ///
    /// Modal like the Ghost Preview, and for the same reason: it is a
    /// picture of a file the user is deciding about, and nothing may act
    /// over it. Its handler runs right after the preview's and before the
    /// banner's, so a resolver opened *from* the banner keeps the keyboard
    /// the banner would otherwise claim.
    pub resolver: Option<super::resolver::ResolverState>,
    /// What the user chose for each conflicted file, for the finish
    /// summary's words (m6-conflicts/19). Cleared when the operation ends.
    pub choices: std::collections::BTreeMap<std::path::PathBuf, super::finish::Choices>,
    /// The finish summary (`c` on a merge, m6-conflicts/19): `None` while
    /// closed. Modal, and first in [`AppState::graph_overlay`].
    pub finish: Option<super::finish::FinishState>,
    /// The summary's message as `Esc` left it, `(subject, body)`, when it
    /// was the user's: the next `c` puts it back. Apart from
    /// `commit_draft` (ticket 19 S7). Cleared when the operation ends.
    pub finish_draft: Option<(String, String)>,
    /// The conflicted path the banner's or changes pane's `e` handed to
    /// `$EDITOR`, until the terminal comes back: `Product::HandedOver`
    /// carries no path, and a clean return marks the file `edited` in
    /// [`Self::choices`] (m6-conflicts/19 scope 3). The resolver's own `e`
    /// needs none — its open path is the one.
    pub conflict_editing: Option<std::path::PathBuf>,
    /// `a` into an older commit and absorb: the plan in flight, and the
    /// put-in its preview is of (m7-power-tools/09).
    pub amend_into: super::amend_into::AmendInto,
    /// Tags rows carry after their subject until the next write starts —
    /// `✎ changed` after a put-in (m7-power-tools/09).
    pub row_tags: super::row_tags::RowTags,
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
            .saturating_add(u16::from(self.commit_graph_line().is_some()))
            // Below 100 columns with the graph as the tab shown, the
            // `[Graph] Detail` strip is a row the graph never gets — and
            // so is the operation banner's row when one is drawn above it.
            // Neither was a term here, so `j`/`k` and every mouse row ran
            // past what the graph actually drew (review M1).
            .saturating_add(super::banner::tab_rows_above_graph(self))
            // The Ghost Preview's box takes rows out of the graph the same
            // way the footer and the commit-graph line do, so the rows it
            // covers are rows the graph never drew — rather than rows drawn
            // and then painted over, which is how a preview box ends up
            // hiding the very ghosts it is describing.
            .saturating_add(super::preview::preview_rows_taken(self));
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
        self.detail.focus == graft_ui::detail::DetailFocus::Graph
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
    /// The commit message editor (`c`).
    CommitEditor,
    /// The Map view (`M`).
    Map,
    /// The undo journal browser (`J`).
    Journal,
    /// The stash list (`S`).
    Stash,
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

    /// Is a text field taking the keyboard right now?
    ///
    /// Every global binding — `u`, `U`, `Ctrl+Z`, `Esc` on a failure — asks
    /// this before it fires. A letter typed into a field must reach the
    /// field, not a keybinding: spelling `update` as a stash message once
    /// launched an undo at the `u` (m3/06 review, Critical 1). Structural
    /// ordering was the old answer and it broke the moment a fourth text
    /// field was added below the undo check, so the rule is a predicate
    /// naming every field instead. **A new text field belongs in this list.**
    #[must_use]
    pub fn text_field_open(&self) -> bool {
        self.search_prompt.is_some()
            || super::place::prompt_open(self)
            || self.commit_editor.is_some()
            || self.stash.prompt.is_some()
            || self.credential.is_some()
            || self.name_prompt.is_some()
            || self
                .finish
                .as_ref()
                .is_some_and(|finish| finish.focus != super::finish::FinishFocus::Files)
    }

    /// Is a modal pane or box taking the keyboard right now
    /// (`docs/design/02-keymap.md` §2.1)?
    ///
    /// The graph's `/`, `n` and `N` ask this before the search answers
    /// them: the search used to run ahead of the whole modal chain, so a
    /// `/` typed into a commit subject opened an invisible prompt and took
    /// the rest of the line with it (review m6-07, Major 1). Every text
    /// field, both boxes, the resolver and the focused banner: the panes
    /// that claim every key but `Ctrl+C`. **A new one belongs in this
    /// list**; `update::search::tests::slash_belongs_to_every_modal_that_is_up`
    /// sweeps it.
    ///
    /// Not the Map, journal or stash overlay: each claims only the keys it
    /// shows (§2.1), and the bar behind each offers `/ search`, which works
    /// there (`update::tests::the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows`).
    #[must_use]
    pub fn modal_owns_keys(&self) -> bool {
        self.text_field_open()
            || self.confirm.is_some()
            || self.preview.is_some()
            || self.resolver.is_some()
            || self.finish.is_some()
            || super::place::owns_keys(self)
            || (self.banner_focus && super::banner::banner_view(self).is_some())
    }

    /// True while a modal stands over the bottom row: the credential
    /// prompt, the name prompt, a confirm box (the stash drop box and the
    /// banner's abort box among them), or the Ghost Preview. Every rung of
    /// the bottom row asks it before drawing a status: the modal owns the
    /// row, so a status from before it — a push rejection, a stale `Busy`, a
    /// `Done` — never sits under keys it does not belong to (m6-conflicts/69,
    /// Decided 1). It **hides**; it never clears: `esc` on the modal draws
    /// the status again (Decided 2). On the state, not in `view`, because
    /// `update` asks it too: nothing is acknowledged, and no failure let go,
    /// while the row cannot show it (m6-conflicts/78).
    #[must_use]
    pub(crate) fn modal_owns_bottom_row(&self) -> bool {
        self.credential.is_some()
            || self.name_prompt.is_some()
            || self.confirm.is_some()
            || self.preview.is_some()
    }

    /// What the status rungs draw once no modal owns the row: a failure
    /// that arrived under a box and was replaced before the row drew it
    /// (`unseen`), while it stands; else `status` (m6-conflicts/78).
    #[must_use]
    pub(crate) fn shown_status(&self) -> &Status {
        self.unseen.as_ref().unwrap_or(&self.status)
    }
}

/// A live operation stopped: what the footer's reason is worded from
/// (`banner::stop_reason`). The live working-tree read overrides `kind`'s
/// step and `paths_at_stop` while it names the same kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stop {
    /// Whose operation this is: this run's, a previous run's, or one begun
    /// in the shell.
    pub origin: super::relaunch::LiveOrigin,
    /// A rebase that stopped on a later step after a `c` (`stopped again`).
    pub again: bool,
    /// The operation git is part-way through.
    pub kind: graft_git::state::InProgress,
    /// Unmerged paths when it stopped — the count until a read says more.
    pub paths_at_stop: usize,
}

/// A stash put back that clashed, and whose it is (m6-conflicts/72b):
/// [`AppState::clash`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashState {
    /// The entry, the stash, its position now and why it was put back.
    pub clash: graft_ops::clash::StashClash,
    /// This run's clash, or one the launch found (`PreviousRun`).
    pub origin: super::relaunch::LiveOrigin,
}

/// Whether [`AppState::refs`] is the answer to the last refs read asked for.
///
/// Every refs answer is stored whatever its id (`took_fact`), so an older read
/// can land after a newer one; this is what tells the two apart without
/// dropping the answer (`head_landing` waits on its own id).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefsRead {
    /// No read asked, the read asked failed, or an older answer overwrote a
    /// newer one.
    #[default]
    Unknown,
    /// This read was asked and has not answered.
    Pending(JobId),
    /// `refs` holds this read's answer, and no read has been asked since.
    Current(JobId),
}

impl RefsRead {
    /// A `Command::ReadRefs(id)` was issued: it supersedes whatever was
    /// asked or held before.
    pub fn asked(&mut self, id: JobId) {
        *self = Self::Pending(id);
    }

    /// A refs answer under `id` landed (`ok`: it was stored).
    ///
    /// `Ok`: the read asked → `Current`; while a newer read is pending the
    /// answer is older and stays `Pending`; over a current answer it is an
    /// older one overwriting it → `Unknown`. `Err`: the read asked failed →
    /// `Unknown` (the refs held are from before it); any other stays.
    pub fn answered(&mut self, id: JobId, ok: bool) {
        *self = match (*self, ok) {
            (Self::Pending(asked), true) if asked == id => Self::Current(id),
            (Self::Pending(asked), false) if asked == id => Self::Unknown,
            (Self::Current(_), true) => Self::Unknown,
            (other, _) => other,
        };
    }
}

/// The index read behind a stash clash's tie ([`AppState::clash_index`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum IndexRead {
    /// Nothing held that is current: no read asked, or a read of the
    /// journal, working tree or refs landed since the last answer.
    #[default]
    Unknown,
    /// This read is out. `stale`: another read landed while it was out, so
    /// its answer is not judged — one more is asked in its place.
    Pending {
        /// The read's job id; any other answer is dropped.
        id: JobId,
        /// A journal, working-tree or refs read landed after it was asked.
        stale: bool,
    },
    /// The answer, current with the other reads held.
    Read(Result<graft_core::journal::IndexState, String>),
}

/// The reads a pending [`AppState::land_on_head`] still waits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeadLanding {
    /// The refresh's refs read, until it answers.
    pub refs: Option<JobId>,
    /// The refresh's history walk, until it — or a walk a later refresh put
    /// in its place — starts (`banner::walk_started`).
    pub walk: Option<JobId>,
}

/// Which commits only a stash reaches (`graft_git::history_reads::only_in_stash`,
/// m7-power-tools/16): the graph draws those rows `□`, dimmed, badged
/// `only in stash@{n}` (`view::rows`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StashOnlyMarks {
    /// The read in flight, set when a refs read with a stash answers. An
    /// answer under any other id is stale (the refs moved again) and dropped.
    pub job: Option<JobId>,
    /// The newest answer: each commit and the newest entry `n` (`stash@{n}`)
    /// reaching it. Kept while the next read runs — the refs a read follows
    /// rarely change what only a stash holds, and dropping the marks for a
    /// moment on every refresh would flicker them. Emptied when a refs read
    /// lists no stash, and when a read fails. Shared: the state is cloned
    /// per event.
    pub commits: Arc<std::collections::HashMap<graft_core::oid::Oid, usize>>,
}

impl StashOnlyMarks {
    /// `stash@{n}` for `id`, when only a stash reaches it.
    #[must_use]
    pub fn entry(&self, id: graft_core::oid::Oid) -> Option<usize> {
        self.commits.get(&id).copied()
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
    /// **A failure that must outlive the refresh it triggers**, and this is
    /// what it said: a hook that rejected a commit, a patch git would not
    /// apply, a discard that ran part-way, an amend whose `HEAD` message
    /// could not be read, an `$EDITOR` that would not start.
    ///
    /// Apart from [`Status::Failed`] because the two are stale for different
    /// reasons. A read that failed on its own is superseded by the next read
    /// that works, so `JobFinished` clears it. One of these is not: the
    /// refresh the failure itself triggers finishes a moment later, and
    /// clearing on that wiped a rejected hook's message off the screen
    /// before the user could read it (m3/04 review, `KNOWN_GAPS.md`). It
    /// stays until `Esc` (m3-working-tree-undo-and-stash/05).
    ///
    /// It was called `WriteFailed`, which named the commonest case rather
    /// than the rule and then read as a lie at the three sites that use it
    /// for a failed *read* (m3/05 review, Major 7). What it means is
    /// "sticky": dismissed by hand, never by a later job.
    ///
    /// Split into `reason` and `keys` (`hint-bar-redesign`, ADR-0037 §4) so
    /// the graph footer can keep `keys` whole and cut `reason` first when
    /// the line does not fit — a blind cut once lost the very key the
    /// message was naming (`push rejected: … · p pull · P force` at 80
    /// columns, `m4-07-acceptance` M2). `keys` is `None` for the (common)
    /// message with no key to keep.
    Sticky {
        /// What went wrong, in words.
        reason: String,
        /// The key(s) that answer it, worded as they'd read in the hint bar
        /// (`"p pull · P force"`). Only `remote_failed` builds `Some` today
        /// — it parses `ops`' "reason · keys" convention out of the failure
        /// line it is handed, since `graft-ops` is out of this ticket's
        /// crate set.
        keys: Option<String>,
    },
    /// `y` sent the selected commit's SHA to the terminal's clipboard
    /// request (OSC 52); `short` is its first seven hex digits. Says what
    /// Graft sent, not that the clipboard has it: nothing comes back to
    /// tell (ADR-0023). Cleared by the next key.
    Copied {
        /// The SHA's first seven hex digits.
        short: String,
    },
    /// A key could not act yet because a read it needs is still in flight
    /// (`still reading the refs — try again in a moment`). Nothing is
    /// wrong, so it is drawn as a notice, never as a failure (CLAUDE.md: a
    /// status must never read as an error when nothing is wrong; review
    /// 47-49, m1). Cleared by the next key, like `Copied`.
    Waiting(String),
    /// A working-tree write is running: `<description>…`
    /// (m3-working-tree-undo-and-stash/04).
    Busy(String),
    /// It finished: a working-tree write in the journal's own words (`stage
    /// src/a.rs`), or a verdict (`every file done`). Drawn `<what> ✓`, then
    /// ` · <also>` when something else is true after it (`your edits are
    /// back`), then ` · <keys>` when there is a next key to name (`c
    /// finishes the merge`), by `view::status::done_line` — the one place
    /// that draws a `Done`. Cleared by the next key, like `Copied`.
    ///
    /// The keys are kept apart, as `Sticky`'s are, because the tick is the
    /// renderer's: glued into `what`, it landed after the key words
    /// (`… · c finishes the merge ✓`, ticket 41). `what` never holds `✓`.
    Done {
        /// What finished, without the tick.
        what: String,
        /// What else is true once it finished, drawn after the tick and
        /// before the keys (`your edits are back`, m6-conflicts/84). Never
        /// a key: a caller that re-words the keys (`undo::land_after_undo`)
        /// leaves it alone.
        also: Option<String>,
        /// The next key, worded as it reads in the hint bar, if any.
        keys: Option<String>,
    },
}

impl Status {
    /// A sticky failure with no keys to keep whole — the common case; every
    /// site that once wrote `Status::Sticky(reason)` writes this now.
    pub fn sticky(reason: impl Into<String>) -> Status {
        Status::Sticky {
            reason: reason.into(),
            keys: None,
        }
    }

    /// `Done` with no keys: drawn `<what> ✓`.
    pub fn done(what: impl Into<String>) -> Status {
        Status::Done {
            what: what.into(),
            also: None,
            keys: None,
        }
    }

    /// `Done` whose line names the next key: drawn `<what> ✓ · <keys>`.
    pub fn done_with_keys(what: impl Into<String>, keys: impl Into<String>) -> Status {
        Status::Done {
            what: what.into(),
            also: None,
            keys: Some(keys.into()),
        }
    }

    /// A failure — `Failed` or `Sticky` — the kind that stays until `Esc`
    /// dismisses it, and so the kind a box must not let go unseen
    /// (m6-conflicts/78).
    #[must_use]
    pub(crate) fn is_failure(&self) -> bool {
        matches!(self, Status::Failed(_) | Status::Sticky { .. })
    }
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
