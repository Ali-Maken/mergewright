//! [`Command`]: what `update` asks the job runner to do, and the take
//! request it carries — split out of `events` (review m6-amend-16 M3).

use graft_core::job::JobId;
use graft_git::history::Order;

use crate::history::RowRequest;
use crate::update::detail::PaneSize;

/// Work for the job runner: something [`crate::update::update`] wants done that it
/// cannot do itself, because doing it would mean touching the world.
///
/// Plain data, with no closure and no handle in it, so that `update` stays a
/// function whose answer can be compared with `assert_eq!`. Turning a command
/// into the code that runs it is the job runner's business (`crate::jobs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the repository Graft was pointed at and check it is one.
    ///
    /// No path: which repository this run is about is settled before the loop
    /// starts and is the job runner's to hold (`crate::repos`), not something
    /// a keystroke gets to change. This is also what `Ctrl+R` (manual
    /// refresh, DESIGN.md §2.1) re-runs, and it is where anything cached
    /// about the repository is thrown away.
    OpenRepository,
    /// Find out whether the `git` on this machine is new enough (2.38,
    /// DESIGN.md §3.1). Runs once, at startup: it shells out, so it is a job.
    CheckSystemGit,
    /// Walk history in this order and bring the commits back, under an id
    /// `update` chose itself.
    ///
    /// The id is minted here rather than by the pool because
    /// [`crate::update::AppState::awaiting`] has to name the walk from the
    /// moment it is asked for: a `Ctrl+R` in the window before the pool's
    /// `Started` event lands would otherwise have no name to cancel, and the
    /// walk it meant to replace would stay parked on a worker
    /// (m5-history-and-ghost-preview/05).
    ///
    /// The order asked for is not always the order that comes back: without a
    /// commit-graph file a topological walk falls to commit date, and says so
    /// (ADR-0008).
    ReadHistory(JobId, Order),
    /// Read the refs for the badges, counting ahead/behind off the loop's
    /// thread, under an id `update` chose — so an answer can be told from
    /// an older read still in flight (m6-conflicts/12's `land_on_head`).
    ReadRefs(JobId),
    /// Read the working tree's staged/unstaged/untracked counts off the
    /// loop's thread, under an id `update` chose — so the answer to the read
    /// a write asked for can be told from an older one still in flight
    /// (`update::status_read`, review m6-amend-13 Minor 1).
    ReadWorkingTree(JobId),
    /// Work out the whole range's strict topological order, for a walk whose
    /// order is provisional (ADR-0008). Only the newest such job's answer is
    /// used.
    PromoteHistory,
    /// Hand history job `JobId` a request for rows. The rows come back as a
    /// [`crate::history::HistoryUpdate::Window`] tagged with the request's
    /// id.
    ShowRows(JobId, RowRequest),
    /// Stop a job whose result is no longer wanted. Cooperative: a job that has
    /// not started is dropped, and a running one stops at its next check.
    CancelJob(JobId),
    /// Job `JobId` is a write that cannot stop halfway
    /// (`update::remote::Stoppable::No`): no cancel may reach it, and the
    /// pool's shutdown waits for it to end rather than signalling its git
    /// child (m8-polish-and-ship/13b). Sent by `remote::launched`, ahead of
    /// the command that queues the job. The `String` is the write's
    /// description, so a shutdown that has to stop it can say which
    /// (m8/13b review fix).
    Unstoppable(JobId, String),
    /// Stop the unstoppable write `JobId` half-way: lift its mark and cancel
    /// it (SIGTERM, SIGKILL after 2 s, reaped). Sent **only** by the quit
    /// stop box's `y` (`update::quit`, m8/13b review fix); nothing else
    /// lifts the mark.
    ForceStop(JobId),
    /// Hand the user's answer to the credential question job `JobId` is
    /// waiting on (m4-remotes/06). Dropped, zeroed, if the job has ended.
    AnswerCredential(JobId, crate::update::credential::Answer),
    /// Tell job `JobId`'s credential question that the user cancelled it.
    /// `update` always sends [`Command::CancelJob`] beside it: an operation
    /// is never left half-authenticated.
    CancelCredential(JobId),
    /// Search history for a query off the loop's thread (`/`).
    Search(crate::search::Query),
    /// Ask the terminal to put this text on its clipboard (OSC 52, `y`).
    /// Written by the loop itself, to the terminal, through the session's
    /// transport: it is output, not a job, and never reaches the job runner.
    CopyToClipboard(String),
    /// Persist the detail pane's split (ADR-0028: `<`/`>` or a divider drag
    /// changed it). Written by the loop itself, to `<common_dir>/graft/
    /// layout` (`crate::layout::save_split`); a failure is expected and
    /// non-fatal (see that module's doc), never dispatched to jobs.
    SaveLayout(PaneSize),
    /// Run `git commit-graph write --reachable` as a job (`B` on the offer).
    /// Never cancelled once git starts (`graft_git::commit_graph`).
    WriteCommitGraph,
    /// Remember the user declined the commit-graph offer (`X`).
    DeclineCommitGraph,
    /// List the files a diff touches, under an id `update` chose.
    ListFiles(
        JobId,
        graft_git::diff::DiffSpec,
        graft_git::diff::DiffOptions,
    ),
    /// Read and assemble one file's diff, under an id `update` chose.
    ReadFileDiff(
        JobId,
        crate::update::detail::DiffKey,
        graft_git::diff::FileEntry,
    ),
    /// Ask which refs contain a commit, under an id `update` chose.
    ContainedIn(JobId, graft_core::oid::Oid),
    /// Read the Mistake Radar's default-branch context, under an id
    /// `update` chose: the default branch's tip and the branch itself, as
    /// `update` last knew them.
    RadarContext(JobId, graft_core::oid::Oid, graft_core::ref_name::RefName),
    /// Probe one merge for an evil resolution, under an id `update` chose.
    EvilProbe(JobId, graft_core::oid::Oid),
    /// Probe one branch's upstream for a rewrite, under an id `update`
    /// chose: the branch's short name and the tip it stood at. The
    /// remote-tracking ref itself is resolved by the job, from config
    /// `update` cannot read.
    ForcePushProbe(JobId, String, graft_core::oid::Oid),
    /// Probe the newest `window` commits for shared patch-ids, under an id
    /// `update` chose.
    DuplicatesProbe(JobId, u32),
    /// Read which commits only a stash reaches, under an id `update` chose:
    /// every entry's own commit, `stash@{0}` first (m7-power-tools/16).
    ReadStashOnly(JobId, Vec<graft_core::oid::Oid>),
    /// Build the Map view's per-branch bucket counts, under an id `update`
    /// chose (`M`, `update::map`).
    BuildMap(JobId, crate::map::MapRequest),
    /// Run one working-tree write, under an id `update` chose: `ops` decides
    /// what it does (checks, plans, journals, snapshots); `app` only wires
    /// the key to the call (the seam rule, `crates/graft/tests/it/
    /// seam_rule.rs`, m3-working-tree-undo-and-stash/04).
    ///
    /// Carries his keymap, cloned at dispatch: the job flattens a failure
    /// to one line, and the keys in it are spelled from his bindings
    /// (m8/10e).
    Operate(
        JobId,
        Box<graft_ops::operation::Operation>,
        std::sync::Arc<graft_ui::keymap::Keymap>,
    ),
    /// Carry the live merge/rebase/cherry-pick/revert forward, back a step,
    /// or all the way out, under an id `update` chose
    /// (`c`/`s`/`a` on the operation banner,
    /// m5-history-and-ghost-preview/09).
    ///
    /// Its own command rather than an [`Command::Operate`] with a
    /// `Resume`-shaped `Operation`: `graft_ops::apply::resume` takes no
    /// `Operation` at all — the operation is already on disk, in git's own
    /// state, and what this names is only which of the three ways out to
    /// take.
    /// Carries his keymap for the same reason as [`Command::Operate`].
    Resume(
        JobId,
        graft_ops::apply::Resume,
        std::sync::Arc<graft_ui::keymap::Keymap>,
    ),
    /// Work out what a history operation — or the journal's time machine —
    /// *would* do, so the Ghost Preview can draw it before anything runs
    /// (m5-history-and-ghost-preview/08). A **read**: `graft_ops::preview`
    /// plans, and the job reads whatever commits the picture needs and the
    /// screen does not already hold.
    PlanPreview(JobId, Box<crate::update::preview::PreviewRequest>),
    /// Run the journal's time machine: undo every entry back to and
    /// including this one, a step at a time (DESIGN §6.6,
    /// m5-history-and-ghost-preview/08). Reached only from a confirmed
    /// Ghost Preview, never straight from a key. The keymap spells the keys
    /// its failure words name (m8/10e), cloned at dispatch.
    RevertTo(
        JobId,
        graft_core::journal::EntryId,
        bool,
        std::sync::Arc<graft_ui::keymap::Keymap>,
    ),
    /// Take back the newest operation still in force (`u`), under an id
    /// `update` chose. `true` forces past the drift guard — the user has
    /// seen what moved and said go ahead (m3-working-tree-undo-and-stash/06).
    /// The keymap spells the keys its failure words name (m8/10e).
    Undo(JobId, bool, std::sync::Arc<graft_ui::keymap::Keymap>),
    /// Put back the newest undo still in force (`U`), otherwise exactly
    /// [`Command::Undo`] (m3-working-tree-undo-and-stash/06).
    Redo(JobId, bool, std::sync::Arc<graft_ui::keymap::Keymap>),
    /// Read the undo journal, under an id `update` chose. The job closes
    /// anything a previous run left `Pending` before it answers, so the
    /// browser never shows an entry that is waiting for a process that is
    /// gone (m3-working-tree-undo-and-stash/06).
    ReadJournal(JobId),
    /// J's stage 2: git's reflog records of `HEAD` and every local branch,
    /// the `$` rows (`jobs::journal_list::read_outside`, m7-power-tools/24).
    /// Cancelled when J closes first.
    ReadOutside {
        /// The id the answer comes back under.
        id: JobId,
    },
    /// J's stage 3: what nothing holds any more among the journal's and the
    /// records' old values, and the edits undos replaced — the `✗` rows
    /// (`jobs::journal_list::find_lost`, m7-power-tools/24). Asked once stage
    /// 2 has answered, with its records.
    FindLost {
        /// The id the answer comes back under.
        id: JobId,
        /// Stage 2's records, whose old values are candidates too.
        outside: std::sync::Arc<Vec<graft_ops::journal_list::OutsideRecord>>,
    },
    /// `⏎` on a `✗`, `$` or lost-edit row in J: read the facts
    /// `recover_scope` names, then ask `recover_for` what `list.rows[row]`
    /// puts back (`jobs::journal_list::recover_for`, m7-power-tools/24).
    RecoverFor {
        /// The id the answer comes back under.
        id: JobId,
        /// The list as it was on screen when `⏎` was pressed.
        list: std::sync::Arc<graft_ops::journal_list::JournalList>,
        /// The row `⏎` was pressed on, an index into `list.rows`.
        row: usize,
        /// His keymap, for the refusal's words (m8/10e).
        keys: std::sync::Arc<graft_ui::keymap::Keymap>,
    },
    /// Read the stash list, under an id `update` chose. Asked for when the
    /// `S` overlay opens and again after every write, because any of them
    /// may have moved `refs/stash` (m3-working-tree-undo-and-stash/06).
    ReadStashes(JobId),
    /// Read every branch the branch picker lists (`graft_ops::switch_list`),
    /// under an id `update` chose. Asked for when `C` opens the picker
    /// (m7-power-tools/10); cancelled when it closes first.
    ReadSwitchList(JobId),
    /// Read every branch for the palette's BRANCHES
    /// (`graft_git::switch_refs`), under an id `update` chose. Asked once
    /// when `⌃P`/`:` opens the palette; cancelled when it closes first
    /// (m8-polish-and-ship/12).
    ReadPaletteBranches(JobId),
    /// Match the palette's query against branch and tag names, under an id
    /// `update` chose. Asked on every query edit, the previous one
    /// cancelled (m8-polish-and-ship/12).
    PaletteNames(JobId, crate::jobs::palette::NamesRequest),
    /// Read the index now as the journal's recorder does
    /// (`graft_ops::snapshot::index_state`), under an id `update` chose:
    /// a stash clash's tie (ADR-0045 §2, amended 2026-09-25). Asked only
    /// while a clash candidate exists.
    ReadIndexState {
        /// The id the answer comes back under.
        id: JobId,
    },
    /// Read the remote facts `f`/`p`/`P` decide from, under an id `update`
    /// chose. Asked for wherever the refs are (m4-remotes/05).
    ReadRemotes(JobId),
    /// Read what a force-with-lease push would take off the remote: the
    /// commits `expected` reaches that `sent` does not, newest first, and
    /// how many (m7-power-tools/16b). A read; `Esc` does not cancel it.
    ReadLeaves {
        /// The id the answer comes back under.
        id: JobId,
        /// The tip git refused.
        sent: graft_core::oid::Oid,
        /// The tracking tip the lease names.
        expected: graft_core::oid::Oid,
    },
    /// Read `commit.template`'s file for the commit editor's empty body,
    /// under an id `update` chose (m3-working-tree-undo-and-stash/05).
    ReadCommitTemplate(JobId),
    /// Read `HEAD`'s commit message to prefill an amend, under an id
    /// `update` chose (m3-working-tree-undo-and-stash/05).
    ReadHeadMessage(JobId),
    /// Plan `a` into an older commit, or absorb (m7-power-tools/09). `head`
    /// is `Some` on the `HEAD` path: the job asks `rewrite::dependents` for
    /// it first, and answers "nothing built on it" before planning.
    PlanRewrite {
        /// The job's id.
        id: JobId,
        /// What to plan.
        request: graft_ops::rewrite::RewriteRequest,
        /// His branch and its tip, on the `HEAD` path only.
        head: Option<graft_ops::operation::Tip>,
    },
    /// Read what the rewrite editor opens on (`R`, m7-power-tools/08): `req`
    /// planned — `Open { from }` on the graph (his branch from `HEAD` down
    /// to its base, deeper to `from`), `FromRebase` in a rebase preview
    /// (that rebase's own todo, rebase-preview-keeps-drops) — with every
    /// row's whole message and the base row. A read; `Esc` cancels it.
    OpenRewrite {
        /// The id the answering [`Product::RewriteOpened`] comes back under.
        id: JobId,
        /// What the editor opens on.
        req: graft_ops::rewrite::RewriteRequest,
    },
    /// After a put-in landed: the commit `offset` first-parent steps below
    /// `HEAD`, for each offset (m7-power-tools/09).
    ResolveRowTags(JobId, Vec<u32>),
    /// Hand the terminal to `$EDITOR` on this path, at this line, and take
    /// it back when the editor exits (m3-working-tree-undo-and-stash/05).
    ///
    /// Carried out by the frame loop itself, never by the job runner:
    /// restoring the terminal and re-entering it is the loop's own
    /// business, and a worker thread has no business touching the modes the
    /// loop drew through (`event_loop::outside`). `CopyToClipboard` and
    /// `SaveLayout` are the precedent.
    OpenExternalEditor {
        /// The file to open.
        path: std::path::PathBuf,
        /// The line to put the cursor on, where the editor takes one.
        line: Option<u32>,
    },
    /// Write this message to the repository's `graft/COMMIT_EDITMSG`, hand
    /// the terminal to `$EDITOR` on it, and bring back what was saved
    /// (`Ctrl+E`, m3-working-tree-undo-and-stash/05). The path is the
    /// loop's: `update` knows no repository directory.
    EditCommitMessage {
        /// The id the answering [`Product::ExternalEditor`] comes back under,
        /// so a message from an editor the user has since closed is dropped.
        id: JobId,
        /// The message as the fields have it now.
        message: String,
    },
    /// Open `$EDITOR` on a conflicted file at its **first conflict
    /// marker** (`e` on the banner or on a `CONFLICTS` row,
    /// m5-history-and-ghost-preview/09).
    ///
    /// Apart from [`Command::OpenExternalEditor`], which carries a line
    /// `update` already knows, because finding the first `<<<<<<< ` means
    /// reading the file — and `update` opens no files. The loop is about to
    /// open this one regardless, so it does the read.
    OpenConflictEditor {
        /// The conflicted file, as git's porcelain names it.
        path: std::path::PathBuf,
    },
    /// Hand the terminal to `git mergetool` on one conflicted file
    /// (`M`, m5-history-and-ghost-preview/09). Carried out by the frame
    /// loop, like `OpenExternalEditor`, and for the same reason: the tool
    /// is a full-screen program and the modes Graft drew through are the
    /// loop's to put back.
    OpenMergetool {
        /// The conflicted file, as git's porcelain names it.
        path: std::path::PathBuf,
    },
    /// Hand the terminal to `git <kind> --continue` for a rebase or
    /// cherry-pick begun in the shell (`c`, m7-power-tools/07c): Graft has
    /// no plan of its own to resume, so git carries it on, its editor
    /// asking for a message when it wants one. Carried out by the frame
    /// loop, like `OpenMergetool`. Answered by `Product::ContinuedInGit`.
    ContinueInGit {
        /// The job this answers as.
        id: JobId,
        /// `Rebase` or `CherryPick` only.
        kind: graft_git::state::InProgress,
    },
    /// Suspend Graft the way `Ctrl+Z` does in any other terminal program
    /// (`keys.suspend_on_ctrl_z`, ADR-0002). Carried out by the frame loop,
    /// like `OpenExternalEditor`.
    Suspend,
    /// Draw in this theme for the rest of the run: the palette's
    /// `Theme: …` rows (m8-polish-and-ship/11, decision 6). Carried out by
    /// the frame loop, which owns the theme (`Session::choose`); not an
    /// operation, never journaled.
    Retheme(graft_ui::theme::ThemeKind),
    /// Read one conflicted path for the resolver (m6-conflicts/04), under an
    /// id `update` chose so a stale answer can be dropped.
    ///
    /// No status rides along: the job reads the working tree's status
    /// **itself**, fresh. The resolver re-reads its file after `e` and `M`
    /// precisely to find out whether the path is still unmerged, and a
    /// cached status — the one `AppState` is holding, read before the
    /// handover — could never say that it is not.
    OpenConflict(JobId, std::path::PathBuf),
    /// Take one side of one conflict block and write the file back
    /// (m6-conflicts/04), under an id `update` chose.
    TakeSide(JobId, TakeRequest),
    /// Attach the merge base to every block of the file the resolver has
    /// open (`B`, m6-conflicts/05), under an id `update` chose. The file
    /// rides along — a fresh clone of the resolver's own copy, bases and
    /// all it already carries preserved — because `graft_ops::conflict::base_for`
    /// takes `&mut ConflictFile` rather than reading the path a second time.
    LoadBase(
        JobId,
        std::path::PathBuf,
        Box<graft_git::conflict::ConflictFile>,
    ),
    /// Read the operation's conflict set for the resolver's file list
    /// (m6-conflicts/13), under an id `update` chose.
    ReadConflictSet(JobId),
    /// Read what a merge about to be committed changes, for the finish
    /// summary (`c` on a merge, m6-conflicts/19), under an id `update`
    /// chose so a stale answer can be dropped.
    ReadFinishSummary(JobId),
    /// Read one staged file's diff for the finish summary's `space`, under
    /// an id `update` chose (m6-conflicts/19 S4). Its own command rather
    /// than [`Command::ReadFileDiff`]: that one carries the detail pane's
    /// `DiffKey`, and its answer always goes to the detail pane.
    ReadFinishDiff(JobId, std::path::PathBuf),
    /// Blame one file, under an id `update` chose (a blame place,
    /// m7-power-tools/20). Cancellable: `Esc` while it reads stops git.
    ReadBlame(JobId, Box<crate::update::place::BlameRequest>),
    /// Trace the commits that changed some lines or a whole file, under an
    /// id `update` chose (a history place, m7-power-tools/21). Cancellable:
    /// `Esc` while it traces stops git. One answer per trace (21 ruling a).
    TraceHistory {
        /// The place's job.
        id: JobId,
        /// What to trace.
        request: Box<crate::update::place::TraceRequest>,
    },
    /// Read one commit's change to a file as the whole file (the history
    /// place's file pane, m7-power-tools/21). Cancellable.
    ReadWholeFile {
        /// The place's job.
        id: JobId,
        /// The commit.
        commit: graft_core::oid::Oid,
        /// The file's path in it.
        path: std::path::PathBuf,
        /// The path before, when the commit renamed it.
        old_path: Option<std::path::PathBuf>,
        /// Past this many lines the file is cut (`L` asks for all).
        max_lines: usize,
        /// `w`: read with git's `-w`, whitespace-only changes unmarked.
        ignore_whitespace: bool,
    },
}

/// One block's take, as the resolver asks for it.
///
/// The **document** travels, not a fingerprint: `graft_ops::conflict::apply_take`
/// applies the take to the copy the user is looking at and writes that back,
/// checking `file.fingerprint` against the bytes on disk as it goes. Re-reading
/// the path instead would drop every block already resolved and re-index the
/// rest, so a second take would land on the wrong block (design note, hard
/// choice 2; m6-conflicts/03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakeRequest {
    /// The conflicted file, as git's porcelain names it.
    pub path: std::path::PathBuf,
    /// The document the take was chosen against — the resolver's own copy,
    /// shared rather than cloned into the command.
    pub file: std::sync::Arc<graft_git::conflict::ConflictFile>,
    /// Where the take lands: one block, or every open one.
    pub at: TakeAt,
    /// Which side it keeps.
    pub take: graft_git::conflict::Take,
}

/// Where a take lands (m6-conflicts/16 D2): one block, counting `Conflict`
/// and `Resolved` regions in order, or every block still open
/// (`Ctrl+O`/`Ctrl+T`) — one queue entry, one write, one keypress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TakeAt {
    /// The `n`-th block.
    Block(usize),
    /// Every block still open in the document the take is applied to.
    Every,
}
