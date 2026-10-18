//! Everything Mergewright knows, as one value, plus the small types it is built
//! from.

use std::path::PathBuf;
use std::sync::Arc;

use mergewright_core::capabilities::RenderingTier;
use mergewright_core::job::JobId;

use crate::history::History;
use crate::repository::RepositoryFacts;

use super::events::{CommitGraphOffer, GitCheck};

mod harness;
mod key_owner;
mod look;
mod queries;
mod quit;
mod reads;
mod screen;
mod stash_marks;
mod status;
mod stop;

pub use harness::{Banner, Harness};
pub(crate) use look::Look;
pub use quit::Quit;
pub(crate) use reads::IndexRead;
pub use reads::{HeadLanding, RefsRead};
pub use screen::{CHROME_ROWS, GraphOverlay, Screen, Size};
pub use stash_marks::StashOnlyMarks;
pub use status::Status;
pub use stop::{ClashState, Stop};

/// Everything Mergewright knows, as one value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppState {
    /// Set once Mergewright is leaving, and why. The event loop reads it after every
    /// event and returns, which drops the `TerminalSession` and hands the
    /// terminal back — a fatal quit is reported only after that, on a terminal
    /// that is the user's again.
    pub quit: Option<Quit>,
    /// `q y` over a write that cannot stop halfway: Mergewright quits once that
    /// write ends done, and stays if it stops, fails or ends incomplete
    /// (`update::quit`, m8-polish-and-ship/13b). Only meaningful while its
    /// job is `operating`; [`AppState::shown_status`] draws its line then.
    pub quit_when_done: Option<Box<super::quit::QuitWait>>,
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
    /// The repository Mergewright was pointed at. Empty in a default state, which is
    /// only ever a starting point: `event_loop::run` fills it in before the
    /// first frame.
    pub repo_path: PathBuf,
    /// What the repository turned out to be, once the job that opened it has
    /// answered. `None` until then, which is the first frame. Boxed: kept
    /// inline it put `AppState` past 8 KiB (ease-pass 06d's `head_commit`),
    /// and every state the tests line up in an array would be on the stack.
    pub repository: Option<Box<RepositoryFacts>>,
    /// An opening was asked beside a walk and has not answered: the walk's
    /// first request waits for its `HEAD` (ease-pass 06 D2).
    pub facts_pending: bool,
    /// What the system `git` turned out to be, once that job has answered.
    pub system_git: Option<GitCheck>,
    /// Where Mergewright's offer to build a missing commit-graph file is
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
    pub working_tree: Option<Arc<mergewright_git::status::WorkingTreeStatus>>,
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
    pub(crate) resuming: Option<(JobId, mergewright_ops::apply::Resume)>,
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
    /// full in place of the key hints (`mergewright_ui::graph_screen::graph_view`
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
    pub icons: mergewright_ui::graph_screen::Icons,
    /// `graph.max_lanes`: the configured lane cap, or `None` to let the
    /// width breakpoint decide. Resolved per frame against the room the
    /// graph has (`mergewright_ui::graph_screen::max_lanes_for`), because the cap
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
    /// The branch picker (`C`, m7-power-tools/10), while it is up or hidden
    /// under the switch preview it opened.
    pub picker: Option<super::picker::PickerState>,
    /// The command palette and help (`?`, `⌃P`, `:`; m8-polish-and-ship/11),
    /// while it is up: a text field over everything but the credential
    /// prompt. Boxed: its text field would grow every `AppState` by its
    /// size while it is closed.
    pub(crate) palette: Option<Box<super::palette::PaletteState>>,
    /// COMMANDS RUN RECENTLY: what the palette ran this session, newest
    /// first (m8-polish-and-ship/12, ADR-0061). In memory only.
    pub(crate) palette_recent: super::palette::RecentRuns,
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
    pub commit_editor: Option<Box<super::commit_editor::CommitEditor>>,
    /// What `Esc` kept, one slot per way in (`c`, `a`), each taken when its
    /// way in opens the screen, so a peek with `a` never costs `c`'s words
    /// (ease-pass/08 C15, 08s M1). The `c` slot is also where `u` gives back
    /// the message of the commit it took back (Q9 a).
    pub drafts: super::commit_editor::Drafts,
    /// The merge, rebase, cherry-pick, revert or bisect the repository is
    /// partway through, as the last working-tree read found it, with the step
    /// counts the banner words itself from. Commit and amend refuse while it
    /// is `Some` and the refusal names the keys that finish it (ADR-0034,
    /// m5-history-and-ghost-preview/09).
    pub in_progress: Option<mergewright_git::state::InProgressDetail>,
    /// `merge.tool` from git's config, as the same read found it. `None`
    /// means `M` refuses in words instead of launching `git mergetool` and
    /// letting git ask "which tool?" on a terminal Mergewright has just handed
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
    pub live_entries: Vec<mergewright_core::journal::EntryId>,
    /// From the launch's `Recovered::set_aside`
    /// ([`crate::jobs::undo::JournalRead::interrupted_edits`]): where each
    /// interrupted set-aside's edits were at recovery (30c). Lives as long
    /// as the notice (until `J`); empty after any later launch, when the
    /// notice keeps ` · edits in stash@{n}`.
    pub interrupted_edits: Vec<(mergewright_core::journal::EntryId, mergewright_ops::EditsAt)>,
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
    /// `mergewright_ops::clash::live_clash` finds none. Read by the resolver's
    /// names and close, and by the stop line while `stopped` is `None`.
    pub clash: Option<ClashState>,
    /// [`crate::update::clash::refresh_clash`] has run on a full set of
    /// facts (journal, working tree, refs) once: a clash it finds after this
    /// is this run's, one it finds before is from the last run.
    pub(crate) clash_facts_seen: bool,
    /// The `DropPutBack` the resolver's close launched after a popped
    /// stash's last file, the stash's position then, and why it was put
    /// back — what its answer's `✓ every file done · stash@{0} dropped`
    /// names (a set-aside's: `✓ every file done · your edits are back`,
    /// m6-conflicts/74).
    pub(crate) clash_drop: Option<(JobId, Option<usize>, mergewright_core::journal::PutBackWhy)>,
    /// The clash landing opened the resolver from the changes pane while
    /// the working tree held was still the one from before the put-back —
    /// clean, so the pane could not stay on it (`selection::reconcile`). The
    /// next working-tree read that shows the tree dirty lands the pane on
    /// `CONFLICTS` ([`crate::update::clash::refresh_and_land`]).
    pub(crate) clash_pane_owed: bool,
    /// The index now, as the clash's tie reads it (ADR-0045 §2, amended
    /// 2026-09-25; review 72+72b M2). Asked only while
    /// `mergewright_ops::clash::clash_candidate` finds one, one read out at a time;
    /// every journal, working-tree or refs read that lands makes the held
    /// answer stale ([`crate::update::clash::refresh_and_land`]).
    pub(crate) clash_index: IndexRead,
    /// A `--continue` finished (`resumed` → `Done`), or a write whose launch
    /// owed a landing did ([`AppState::lands`]): the graph cursor goes to
    /// the new `HEAD` — or the commit [`HeadLanding::on`] names — once the
    /// refresh that answer issued has both the fresh refs and the fresh walk
    /// ([`AppState::head_landing`]). Spent by `banner::land_on_head`.
    pub land_on_head: bool,
    /// The two reads [`AppState::land_on_head`] waits on, each `Some` until
    /// it has answered. Ids, not "the next answer": a refs read from before
    /// the `c` can land after `Done`, and it would put the cursor on the old
    /// `HEAD`. `Command::ReadRefs` carries an id `update` chose for exactly
    /// this (the refs answer had no id guard before).
    pub head_landing: HeadLanding,
    /// The write in `operating` whose finish owes a landing, and where
    /// (`operate::landing_of`): `HEAD` for one that moves it, the restored
    /// commit or the changes pane for a put-back, the new tip for a branch
    /// made without switching (m7-power-tools/03, /24). Its `Done` spends it
    /// as a `--continue` does ([`AppState::land_on_head`]). Decided at launch
    /// from the operation, never guessed from the answer. Beside `operating`
    /// rather than inside it, as `resuming` is: `operating` is a bare id read
    /// in hundreds of places.
    pub(crate) lands: Option<(JobId, super::operate::Landing)>,
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
    /// `git <kind> --continue` came back from the shell hand-over
    /// (m7-power-tools/07c) and its landing waits on the newest working-tree
    /// read: finished, stopped again, or still stopped after a non-zero
    /// exit — only the read can tell. Spent by
    /// `update::banner::land_after_continue_in_git`.
    pub continue_in_git_owed: Option<(mergewright_git::state::InProgress, Option<i32>)>,
    /// The `HEAD`-message read an `a` is waiting on: the editor opens on its
    /// answer, so there is never a half-filled amend on screen. Any other
    /// id's answer is dropped (m3-working-tree-undo-and-stash/05).
    pub head_message: Option<JobId>,
    /// [`AppState::head_message`] was asked by `a` at a rewrite's `edit`
    /// stop (m7-power-tools/07b): its answer amends at once, keeping the
    /// message, rather than opening the editor.
    pub edit_amend: bool,
    /// The `commit.template` read an open editor is waiting on, for the same
    /// reason.
    pub commit_template: Option<JobId>,
    /// The `Ctrl+E` handover an open editor is waiting on. What a `$EDITOR`
    /// saved is dropped unless this still names it: `Esc` in between means
    /// the user moved on, and dropping it beats overwriting a field they
    /// have since retyped.
    pub external_editor: Option<JobId>,
    /// `[keys] suspend_on_ctrl_z`: `Ctrl+Z` stops Mergewright instead of undoing
    /// (ADR-0002). Seeded from config at startup; off by default.
    pub suspend_on_ctrl_z: bool,
    /// His `[keys]` remaps, applied at the edge before any dispatcher sees a
    /// key (`update::edge`, ADR-0055). One per process, built from the
    /// config at startup (`config::keymap_from`); the default keymap until
    /// then and in tests.
    pub(crate) keymap: std::sync::Arc<mergewright_ui::keymap::Keymap>,
    /// Everything wrong with his config file, in words (`Arrival::first`); `?` lists them.
    pub(crate) config_problems: Vec<String>,
    /// The mode a walk starts in (ease-pass 06h); `switch_graph_mode` sets it.
    pub graph_mode: crate::config::MergedBranches,
    /// The bottom row reads `config: N problems · ? shows them` until the
    /// first key or click acknowledges it, as a `Done` is
    /// (`keys::acknowledge`). Set at startup only when there are problems.
    pub(crate) config_notice: bool,
    /// The first-launch line (m8/18d): the status row's fallback while
    /// `Some(config dir)`. A plain value, seeded on arrival from the shell's
    /// one switch (m8/18e); the first palette open sets it `None` and asks
    /// the loop to end the switch (`Command::FirstLaunchDone`). `None` by
    /// default (headless `run`, the gallery, every `Harness`).
    // Boxed, as `tour` below: one pointer keeps `AppState` the size it was
    // and under the stack-array size clippy guards (`large_stack_arrays`).
    pub(crate) first_launch: Option<Box<std::path::PathBuf>>,
    /// This place is the practice tour's repository (m8-polish-and-ship/19b):
    /// its ticks and the read that refreshes them. `None` everywhere else.
    /// Seeded at arrival (`event_loop::startup::arrive`), never later.
    // Boxed: `AppState` is copied whole on every event and kept under the
    // stack-array size clippy guards (`large_stack_arrays`); the tour is
    // `None` in every repository but one.
    pub(crate) tour: Option<Box<super::tour::TourState>>,
    /// The commit an arrival's landing went looking for (`update::
    /// lands_at_open` with `Some`, the tour's way back): kept until the walk
    /// has answered where it is, so a commit no longer there falls back to
    /// `HEAD` with a word (`tour::check_return`). `None` once settled.
    pub(crate) return_to: Option<mergewright_core::oid::Oid>,
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
    /// The live operation's picks for the resolver's `u` (ease-pass 05):
    /// beside the operation, so closing the resolver keeps them; memory
    /// only. Boxed: tests hold `AppState`s in arrays on the stack.
    pub picks: Box<super::resolver::Picks>,
    /// The finish summary (m6-conflicts/19), `None` while closed; modal, first
    /// in [`AppState::graph_overlay`]. Boxed for the reason `picks` is.
    pub finish: Option<Box<super::finish::FinishState>>,
    /// The conflicted path the banner's or changes pane's `e` handed to
    /// `$EDITOR`, until the terminal comes back: `Product::HandedOver`
    /// carries no path, and a clean return marks the file `edited` in
    /// [`Self::choices`] (m6-conflicts/19 scope 3). The resolver's own `e`
    /// needs none — its open path is the one.
    pub conflict_editing: Option<std::path::PathBuf>,
    /// `a` into an older commit and absorb: the plan in flight, and the
    /// put-in its preview is of (m7-power-tools/09).
    pub amend_into: super::amend_into::AmendInto,
    /// The rewrite editor (`R`, m7-power-tools/08): its open read, the
    /// editor on screen, and the one whose `y` is running.
    pub(crate) rewrite: super::rewrite_editor::RewriteState,
    /// Tags rows carry after their subject until the next write starts —
    /// `✎ changed` after a put-in (m7-power-tools/09).
    pub row_tags: super::row_tags::RowTags,
    /// Each folding merge's count, by merge id (m8-polish-and-ship/17b,
    /// ease-pass 06): a fact of the merge commit, so kept for the session
    /// and never recounted — opening and closing keep it too. Filled at
    /// rest by `update::fold_count::at_rest`, for the merges on screen.
    pub(crate) fold_counts:
        std::collections::BTreeMap<mergewright_core::oid::Oid, super::fold_count::FoldCount>,
    /// The cursor is mid-burst (G-settle): `settle` records a moved
    /// selection as owed instead of following it. Written only by the event
    /// loop, before each update (`event_loop::rest`); `false` everywhere
    /// else, so a test driving `update` directly sees every move followed.
    pub(crate) defer_follow: bool,
    /// What the pane and the evil probe last followed. Differs from
    /// `super::follow_key` while a follow is owed.
    pub(crate) followed: Option<super::FollowKey>,
    /// The commit `c` just made, while it is the last thing that happened
    /// (ease-pass/08 C13): its row's bar says `u` gives the edits back, and
    /// that undo gives its message back. Cleared by any other write
    /// landing, an undo, or the commit screen opening.
    pub last_commit: Option<Box<super::commit_editor::LastCommit>>,
    /// The merge commit the review's `y` just made (ease-pass/12 M18):
    /// while the cursor rests on it, it is still `HEAD` and its count is
    /// known, its row's bar is `RowBar::JustMerged`. Cleared by any other
    /// write landing.
    pub last_merge: Option<mergewright_core::oid::Oid>,
    /// Colours on or off, and the route the lanes are drawn by.
    pub(crate) look: Look,
}
