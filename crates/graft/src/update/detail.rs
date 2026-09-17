//! The detail pane's state: what it is showing, the jobs it is waiting on, and
//! the small caches that make stepping back through files and commits free.
//!
//! Pure like the rest of `update`: job ids come from the caller's counter, and
//! every read is a [`Command`] whose answer arrives as a [`Product`]. An answer
//! whose id is not the one still pending is dropped: the user has moved on.

use std::collections::VecDeque;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_git::contained::Contained;
use graft_git::diff::{DiffOptions, DiffSpec, FileList};
use graft_ops::mistake_radar::Policy;
use graft_ops::mistake_radar::probes::EvilVerdict;
use graft_term::sanitize::{sanitize_line, sanitize_path_bytes};
use graft_ui::detail::{
    ContainedIn, DetailFocus, DiffMode, DiffView, ExplainView, FileListView, LineKindView,
};
use graft_ui::detail_pane::DetailLayout;

use crate::history::History;

use super::events::{AppEvent, Command, Product};
use super::state::AppState;

/// Ids `update` hands out itself carry this bit; the job runner's own counter
/// never reaches it, so the two can never name the same job.
pub const LOCAL_JOB_BIT: u64 = 1 << 63;

/// Diffs kept for stepping back through files.
const DIFF_CACHE: usize = 8;
/// "Contained in" answers kept for stepping back through commits.
const CONTAINED_CACHE: usize = 64;
/// Evil-merge verdicts kept for stepping back through merges. A merge's tree
/// is immutable, so a verdict never goes stale and a refresh keeps them.
const EVIL_CACHE: usize = 16;
/// Lines of one file's diff read before it is cut short (`L` loads the rest).
const MAX_LINES: usize = 5_000;
/// `PageUp`/`PageDown` step, in rows, for the files list and the diff body.
const PAGE_STEP: isize = 10;
/// `]`'s and `[`'s ceiling on context lines: past this, more context stops
/// changing what most diffs show.
const MAX_CONTEXT: u8 = 20;

/// `<`/`>` (ADR-0028) move the focused divider by this many cells per press.
const RESIZE_STEP: u16 = 4;
/// The pane's own minimum, mirrored from `graft_ui::detail_pane::DetailLayout`
/// (private there): 42% of a 160-column screen never goes below this.
const RIGHT_MIN: u16 = 48;
/// The pane's own minimum in the stacked layout.
const BELOW_MIN: u16 = 8;
/// The graph never shrinks below this many columns for the pane's sake.
const GRAPH_MIN_COLUMNS: u16 = 40;
/// The graph never shrinks below this many rows for the pane's sake.
const GRAPH_MIN_ROWS: u16 = 6;

/// The detail pane's size, remembered per orientation (ADR-0028): a column
/// count set side by side means nothing as a row count once the window
/// narrows into the stacked layout, so the two never share one field.
/// `None` in either field means "the percentage default", not zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaneSize {
    /// `Right` layout's divider position, in columns.
    pub right_columns: Option<u16>,
    /// `Below` layout's divider position, in rows.
    pub below_rows: Option<u16>,
}

impl PaneSize {
    /// The value for `layout`'s orientation; `None` for `Tab`, which has no
    /// divider to remember a size for.
    #[must_use]
    pub fn for_layout(&self, layout: &DetailLayout) -> Option<u16> {
        match layout {
            DetailLayout::Right { .. } => self.right_columns,
            DetailLayout::Below { .. } => self.below_rows,
            DetailLayout::Tab { .. } => None,
        }
    }
}

/// `current` stepped by [`RESIZE_STEP`] toward `grow`, clamped to `[min,
/// max]`. A press already at a bound returns `current` unchanged.
fn stepped(current: u16, grow: bool, min: u16, max: u16) -> u16 {
    let moved = if grow {
        current.saturating_add(RESIZE_STEP)
    } else {
        current.saturating_sub(RESIZE_STEP)
    };
    moved.clamp(min.min(max), max)
}

/// What the pane is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetailSpec {
    /// One commit against its first parent.
    Commit(Oid),
    /// `head` against `base`.
    Compare {
        /// The older side.
        base: Oid,
        /// The newer side.
        head: Oid,
    },
    /// The working tree against `HEAD`.
    WorkingTree,
}

impl DetailSpec {
    /// The git diff this spec asks for.
    #[must_use]
    pub fn diff_spec(self) -> DiffSpec {
        match self {
            Self::Commit(commit) => DiffSpec::Commit {
                commit,
                against: None,
            },
            Self::Compare { base, head } => DiffSpec::Range { base, head },
            Self::WorkingTree => DiffSpec::WorkingTreeVsHead,
        }
    }
}

/// A pinned base for a live compare (`x`, m2-details-and-diffs/11): the row
/// stays marked (`graft_ui::graph_screen::GraphEntry::compare_marked`) and
/// the pane follows the cursor as `head` — a fresh `DetailSpec::Compare`
/// against `oid` each time it moves — until cleared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompareBase {
    /// The base side of the compare: what every `head` is measured against.
    pub oid: Oid,
    /// The absolute history row `oid` is on, for `GraphEntry::compare_marked`.
    pub row: usize,
}

/// `x` on a graph row, or `Shift`/`Ctrl`+click on a second row
/// (m2-details-and-diffs/11): no base yet pins one, no command; the same
/// `oid` again clears it, no command; a different `oid` opens the compare
/// and **keeps** the base, so the compare stays live as the cursor moves
/// (`update::selection::follow_selection`).
pub fn toggle_compare(
    state: &mut AppState,
    row: usize,
    oid: Oid,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match state.compare_base {
        None => {
            state.compare_base = Some(CompareBase { oid, row });
            Vec::new()
        }
        Some(base) if base.oid == oid => {
            state.compare_base = None;
            Vec::new()
        }
        Some(base) => state.detail.open(
            DetailSpec::Compare {
                base: base.oid,
                head: oid,
            },
            next_id,
        ),
    }
}

/// A value a job is fetching.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Loading<T> {
    /// Nothing asked for.
    #[default]
    Idle,
    /// Asked for; only this job's answer is wanted.
    Pending(JobId),
    /// Arrived.
    Ready(T),
    /// The job failed, and why.
    Failed(String),
}

/// Everything that decides one file's diff; the diff cache's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiffKey {
    /// What the pane is about.
    pub spec: DetailSpec,
    /// Index into the spec's git file list.
    pub entry: u32,
    /// Context and whitespace options.
    pub options: DiffOptions,
    /// Lines read before cutting short.
    pub max_lines: usize,
}

/// A small map that forgets its oldest entry when full.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bounded<K, V> {
    entries: VecDeque<(K, V)>,
    cap: usize,
}

impl<K: Eq, V> Bounded<K, V> {
    /// An empty map holding at most `cap` entries.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(cap),
            cap,
        }
    }

    /// The value for `k`, if still held.
    pub fn get(&self, k: &K) -> Option<&V> {
        self.entries
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, value)| value)
    }

    /// Hold `v` for `k`, replacing an older value and evicting the oldest entry when full.
    pub fn insert(&mut self, k: K, v: V) {
        self.entries.retain(|(key, _)| *key != k);
        if self.entries.len() >= self.cap {
            self.entries.pop_front();
        }
        self.entries.push_back((k, v));
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// The detail pane's state (`AppState::detail`); hidden by default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailState {
    /// What the pane is about, once opened.
    pub spec: Option<DetailSpec>,
    /// The pane is on screen.
    pub shown: bool,
    /// Which region has the keys.
    pub focus: DetailFocus,
    /// The changed-file tree.
    pub files: Loading<Arc<FileListView>>,
    /// The git file list behind `files`; diff reads name their entry from it.
    pub list: Option<Arc<FileList>>,
    /// The diff under the files cursor.
    pub diff: Loading<Arc<DiffView>>,
    /// Recent diffs, newest last.
    pub diff_cache: Bounded<DiffKey, Arc<DiffView>>,
    /// Recent "contained in" answers, whatever the selection now is.
    pub contained: Bounded<Oid, ContainedIn>,
    /// The open commit's evil-merge verdict (`m2-details-and-diffs/14`).
    /// Tied to `spec`, not to an oid of its own: `probe_evil` sets it in the
    /// same step that sets `spec`, and every other spec leaves it `Idle`.
    pub evil: Loading<EvilVerdict>,
    /// Verdicts already probed this session, so revisiting a merge is free.
    pub evil_cache: Bounded<Oid, EvilVerdict>,
    /// Which commit `evil` is about; `None` when nothing is being probed.
    /// `spec` alone cannot say so: the probe outlives none of it, but it does
    /// start one step later, and `view::radar` asks per row.
    pub evil_at: Option<Oid>,
    /// Context and whitespace options for every read.
    pub options: DiffOptions,
    /// Unified or side by side.
    pub mode: DiffMode,
    /// Changed words tinted inside paired lines.
    pub word_tint: bool,
    /// Lines read per file before cutting short.
    pub max_lines: usize,
    /// First body line on screen.
    pub body_top: usize,
    /// The pane's size, per orientation (ADR-0028); `<common_dir>/graft/layout`
    /// at startup, written on every resize.
    pub pane_size: PaneSize,
    /// A divider drag is in progress: set by a `Down` on the seam, cleared by
    /// `Up` (whichever button — a second button going down mid-drag must not
    /// leave this stuck `true`). Never persisted; a fresh run always starts
    /// with no drag in flight.
    pub dragging: bool,
    /// `Explain` chosen from the context menu on a marked row
    /// (`context-actions-menu`, m2-details-and-diffs/12): the row's `Oid`,
    /// until `open_explain` reads it, builds `DetailView.explain` from it,
    /// and clears it (`m2-details-and-diffs/17`).
    pub explain_requested: Option<Oid>,
    /// The open Explain panel, when there is one (`m2-details-and-diffs/17`).
    pub explain: Option<ExplainView>,
    /// The commits `Explanation::highlight` named: the graph fades every
    /// other row while `explain` is `Some` (`m2-details-and-diffs/17`).
    pub explain_highlight: Vec<Oid>,
}

impl Default for DetailState {
    fn default() -> Self {
        Self {
            spec: None,
            shown: false,
            focus: DetailFocus::Graph,
            files: Loading::Idle,
            list: None,
            diff: Loading::Idle,
            diff_cache: Bounded::new(DIFF_CACHE),
            contained: Bounded::new(CONTAINED_CACHE),
            evil: Loading::Idle,
            evil_cache: Bounded::new(EVIL_CACHE),
            evil_at: None,
            options: DiffOptions::default(),
            mode: DiffMode::default(),
            word_tint: true,
            max_lines: MAX_LINES,
            body_top: 0,
            pane_size: PaneSize::default(),
            dragging: false,
            explain_requested: None,
            explain: None,
            explain_highlight: Vec::new(),
        }
    }
}

impl DetailState {
    /// Show the pane for `spec` with the file list focused. Re-opening what is
    /// already on show asks for nothing.
    pub fn open(&mut self, spec: DetailSpec, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let already = self.shown && self.spec == Some(spec);
        self.shown = true;
        self.focus = DetailFocus::Files;
        if already {
            Vec::new()
        } else {
            self.load(spec, next_id)
        }
    }

    /// Hide the pane; the keys go back to the graph.
    pub fn close(&mut self) {
        self.shown = false;
        self.focus = DetailFocus::Graph;
    }

    /// The git file-list index under the files cursor.
    #[must_use]
    pub fn selected_entry(&self) -> Option<u32> {
        match &self.files {
            Loading::Ready(files) => files.selected_entry(),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        }
    }

    /// Point the pane at `spec` (the selection moved), keeping focus. Nothing
    /// while hidden: the pane costs nothing when off.
    pub fn follow(
        &mut self,
        spec: DetailSpec,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        if !self.shown || self.spec == Some(spec) {
            return Vec::new();
        }
        self.load(spec, next_id)
    }

    /// Forget cached reads and ask again for what is on show.
    pub fn refresh(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        self.diff_cache.clear();
        self.contained.clear();
        match self.spec {
            Some(spec) if self.shown => self.load(spec, next_id),
            Some(_) | None => Vec::new(),
        }
    }

    /// Start (or answer from cache) the evil-merge probe for whatever the
    /// pane is now about.
    ///
    /// Idempotent: called once at the tail of every `update`
    /// (`super::selection::follow_selection`), so it must be free to call
    /// when nothing moved. A probe already running or answered for this same
    /// commit is left alone; anything else cancels it.
    ///
    /// Separate from `open`/`follow`/`load` on purpose: the decision needs
    /// the Mistake Radar's policy, which lives on `AppState::radar`, not on
    /// the pane, and threading a `&Policy` through `load`'s sixteen call
    /// sites would buy nothing.
    ///
    /// Nothing is probed when the pane is hidden, when it is about a compare
    /// or the working tree, or when the evil-merge kind is off: a probe is a
    /// `git` spawn, and nothing would read its answer.
    pub fn probe_evil(
        &mut self,
        policy: &Policy,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        let wanted = match self.spec {
            Some(DetailSpec::Commit(oid))
                if self.shown && policy.severity(MistakeRadarKind::EvilMerge).is_some() =>
            {
                Some(oid)
            }
            _ => None,
        };
        // Already running or answered for this very commit: nothing to do.
        if self.evil_at == wanted
            && !matches!(self.evil, Loading::Idle | Loading::Failed(_))
            && wanted.is_some()
        {
            return Vec::new();
        }
        let mut commands = Vec::new();
        if let Loading::Pending(id) = self.evil {
            commands.push(Command::CancelJob(id));
        }
        self.evil = Loading::Idle;
        self.evil_at = wanted;
        let Some(oid) = wanted else {
            return commands;
        };
        if let Some(verdict) = self.evil_cache.get(&oid) {
            self.evil = Loading::Ready(verdict.clone());
            return commands;
        }
        let id = next_id();
        self.evil = Loading::Pending(id);
        commands.push(Command::EvilProbe(id, oid));
        commands
    }

    /// An evil-merge verdict arrived; one for a probe already superseded is
    /// dropped. The verdict is cached whichever it is: a merge's tree cannot
    /// change, so even `NotCheckable` will not read differently next time.
    pub fn evil_probed(&mut self, id: JobId, oid: Oid, verdict: EvilVerdict) {
        if self.evil != Loading::Pending(id) {
            return;
        }
        self.evil_cache.insert(oid, verdict.clone());
        self.evil = Loading::Ready(verdict);
    }

    /// Ask for `spec`'s file list, dropping whatever was pending.
    fn load(&mut self, spec: DetailSpec, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let mut commands = self.cancel_pending();
        let id = next_id();
        self.spec = Some(spec);
        self.files = Loading::Pending(id);
        self.list = None;
        self.body_top = 0;
        commands.push(Command::ListFiles(id, spec.diff_spec(), self.options));
        if let DetailSpec::Commit(oid) = spec
            && self.contained.get(&oid).is_none()
        {
            self.contained.insert(oid, ContainedIn::Checking);
            commands.push(Command::ContainedIn(next_id(), oid));
        }
        commands
    }

    fn cancel_pending(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        for loading in [&self.files] {
            if let Loading::Pending(id) = loading {
                commands.push(Command::CancelJob(*id));
            }
        }
        if let Loading::Pending(id) = self.diff {
            commands.push(Command::CancelJob(id));
        }
        self.diff = Loading::Idle;
        commands
    }

    /// A file list arrived; stale ones are dropped.
    pub fn files_arrived(
        &mut self,
        id: JobId,
        result: Result<(Arc<FileListView>, Arc<FileList>), String>,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        if self.files != Loading::Pending(id) {
            return Vec::new();
        }
        match result {
            Ok((files, list)) => {
                self.files = Loading::Ready(files);
                self.list = Some(list);
                self.show_cursor(next_id)
            }
            Err(failure) => {
                self.files = Loading::Failed(failure);
                Vec::new()
            }
        }
    }

    /// Move the files cursor and ask for the file it lands on.
    pub fn move_files_cursor(
        &mut self,
        delta: isize,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        let Loading::Ready(files) = &mut self.files else {
            return Vec::new();
        };
        let before = files.cursor;
        Arc::make_mut(files).move_cursor(delta);
        if files.cursor == before {
            Vec::new()
        } else {
            self.show_cursor(next_id)
        }
    }

    /// Show the diff under the files cursor: from the cache, or by asking.
    /// The message row and directories show no diff.
    fn show_cursor(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let mut commands = Vec::new();
        if let Loading::Pending(id) = self.diff {
            commands.push(Command::CancelJob(id));
        }
        self.diff = Loading::Idle;
        self.body_top = 0;
        let (Some(spec), Some(entry), Some(list)) = (self.spec, self.selected_entry(), &self.list)
        else {
            return commands;
        };
        let Some(file) = usize::try_from(entry)
            .ok()
            .and_then(|index| list.files.get(index))
        else {
            return commands;
        };
        let key = DiffKey {
            spec,
            entry,
            options: self.options,
            max_lines: self.max_lines,
        };
        if let Some(diff) = self.diff_cache.get(&key) {
            self.diff = Loading::Ready(Arc::clone(diff));
        } else {
            let id = next_id();
            self.diff = Loading::Pending(id);
            commands.push(Command::ReadFileDiff(id, key, file.clone()));
        }
        commands
    }

    /// A diff arrived; stale ones are dropped, good ones cached.
    pub fn diff_arrived(&mut self, id: JobId, key: DiffKey, result: Result<Arc<DiffView>, String>) {
        if self.diff != Loading::Pending(id) {
            return;
        }
        self.diff = match result {
            Ok(diff) => {
                self.diff_cache.insert(key, Arc::clone(&diff));
                Loading::Ready(diff)
            }
            Err(failure) => Loading::Failed(failure),
        };
    }

    /// A "contained in" answer arrived; kept whatever the selection now is.
    pub fn contained_arrived(&mut self, oid: Oid, result: Result<Contained, String>) {
        let names = |names: &[Vec<u8>]| {
            names
                .iter()
                .map(|name| sanitize_path_bytes(name))
                .collect::<Vec<_>>()
        };
        let answer = match result {
            Ok(found) => {
                let mut branches = names(&found.branches);
                branches.extend(names(&found.remotes));
                ContainedIn::Known {
                    branches,
                    tags: names(&found.tags),
                    more: found.more,
                }
            }
            Err(failure) => ContainedIn::Failed(sanitize_line(&failure)),
        };
        self.contained.insert(oid, answer);
    }

    /// `Tab` (forward) or `Shift+Tab`: Graph → Files → Diff → Graph.
    pub fn cycle_focus(&mut self, forward: bool) {
        self.focus = match (self.focus, forward) {
            (DetailFocus::Graph, true) | (DetailFocus::Diff, false) => DetailFocus::Files,
            (DetailFocus::Files, true) | (DetailFocus::Graph, false) => DetailFocus::Diff,
            (DetailFocus::Diff, true) | (DetailFocus::Files, false) => DetailFocus::Graph,
        };
    }

    /// `w`/`[`/`]`: new context/whitespace options, re-reading the current
    /// file under them. Cached identically to any other options change — no
    /// new mechanism (`show_cursor` already keys the cache on `self.options`).
    pub fn set_options(
        &mut self,
        options: DiffOptions,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        self.options = options;
        self.show_cursor(next_id)
    }

    /// `L`: read the whole of the current file, past [`MAX_LINES`].
    pub fn load_all(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        self.max_lines = usize::MAX;
        self.show_cursor(next_id)
    }

    /// `j`/`k`/`PgUp`/`PgDn`/`g`/`G`/wheel in the `Diff` region: move
    /// `body_top` by `delta` lines, clamped to `[0, content_len - 1]`.
    /// `isize::MIN`/`isize::MAX` jump to the top/bottom (`saturating_add_signed`
    /// already clamps at the ends).
    pub fn scroll_body(&mut self, delta: isize, content_len: usize) {
        let max = content_len.saturating_sub(1);
        self.body_top = self.body_top.saturating_add_signed(delta).min(max);
    }

    /// Wheel over the diff region: same move as [`Self::scroll_body`], with
    /// the content length looked up from whatever diff is currently loaded
    /// (`0` while none is, which clamps `body_top` straight back to `0`).
    pub fn scroll_body_by(&mut self, delta: isize) {
        let len = diff_content_len_of(self);
        self.scroll_body(delta, len);
    }

    /// `<` (`grow = false`) / `>` (`grow = true`) on the focused divider
    /// (ADR-0028): the layout's one divider, columns in `Right`, rows in
    /// `Below`. No-op — `false`, nothing changed — in `Tab` (no divider),
    /// while the pane is hidden, or already at a bound. `size` is the
    /// terminal's (columns, rows).
    pub fn resize_pane(&mut self, layout: &DetailLayout, grow: bool, size: (u16, u16)) -> bool {
        if !self.shown {
            return false;
        }
        match *layout {
            DetailLayout::Tab { .. } => false,
            DetailLayout::Right { columns } => {
                let max = size.0.saturating_sub(GRAPH_MIN_COLUMNS).max(RIGHT_MIN);
                let next = stepped(columns, grow, RIGHT_MIN, max);
                if next == columns {
                    return false;
                }
                self.pane_size.right_columns = Some(next);
                true
            }
            DetailLayout::Below { rows } => {
                let max = size.1.saturating_sub(GRAPH_MIN_ROWS).max(BELOW_MIN);
                let next = stepped(rows, grow, BELOW_MIN, max);
                if next == rows {
                    return false;
                }
                self.pane_size.below_rows = Some(next);
                true
            }
        }
    }

    /// Divider drag: set the orientation's cells straight from the pointer,
    /// same clamps and no-op cases as [`Self::resize_pane`].
    pub fn set_pane_cells(&mut self, layout: &DetailLayout, cells: u16, size: (u16, u16)) -> bool {
        if !self.shown {
            return false;
        }
        match *layout {
            DetailLayout::Tab { .. } => false,
            DetailLayout::Right { columns } => {
                let max = size.0.saturating_sub(GRAPH_MIN_COLUMNS).max(RIGHT_MIN);
                let next = cells.clamp(RIGHT_MIN, max);
                if next == columns {
                    return false;
                }
                self.pane_size.right_columns = Some(next);
                true
            }
            DetailLayout::Below { rows } => {
                let max = size.1.saturating_sub(GRAPH_MIN_ROWS).max(BELOW_MIN);
                let next = cells.clamp(BELOW_MIN, max);
                if next == rows {
                    return false;
                }
                self.pane_size.below_rows = Some(next);
                true
            }
        }
    }
}

/// Answers `explain_requested`, if there is one this tick
/// (`m2-details-and-diffs/17`): opens the pane on that commit, then resolves
/// the finding it was marked for and turns it into `DetailState::explain`.
///
/// The `(parents, subject)` lookup is exactly `view::detail::commit_header`'s
/// own — the selected row filtered by id — not a second one: a marked row is
/// always the row a context menu was opened on, which is the selection.
/// Nothing to explain (the marker's finding is stale, e.g. policy changed
/// underneath) clears `explain_requested` and leaves `explain` alone rather
/// than erroring.
pub fn open_explain(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let Some(oid) = state.detail.explain_requested.take() else {
        return Vec::new();
    };
    let commands = state.detail.open(DetailSpec::Commit(oid), next_id);
    let facts = state
        .history
        .as_ref()
        .and_then(History::selected)
        .filter(|row| row.commit.id == oid)
        .map(|row| {
            (
                row.commit.parents.clone(),
                String::from_utf8_lossy(row.commit.subject()).into_owned(),
            )
        });
    let Some((parents, subject)) = facts else {
        return commands;
    };
    let Some(finding) = crate::view::radar::findings_for(state, oid, &parents, &subject)
        .into_iter()
        .next()
    else {
        return commands;
    };
    let short = |o: Oid| {
        graft_ui::graph_screen::short_sha(
            &o.to_string(),
            usize::from(graft_ui::graph_screen::SHA_WIDTH),
        )
    };
    let explanation = graft_ops::mistake_radar::explain::explain(&finding, &short);
    state
        .detail
        .explain_highlight
        .clone_from(&explanation.highlight);
    let mut view = crate::view::build_explain_view(&explanation);
    view.severity = finding.severity;
    state.detail.explain = Some(view);
    commands
}

/// Close the Explain panel, keeping the pane itself open on the same commit
/// (`Esc`'s first press while Explain is open, `m2-details-and-diffs/17`).
pub fn close_explain(state: &mut AppState) {
    state.detail.explain = None;
    state.detail.explain_highlight.clear();
}

/// Content rows [`graft_ui::detail_pane::diff_body`] draws for `view`: one
/// heading row per hunk plus its non-heading lines, plus a truncation-footer
/// row when the diff was cut short. Mirrors that widget's private row count
/// so `scroll_body` clamps to what will actually be on screen.
fn diff_content_len(view: &DiffView) -> usize {
    let lines: usize = view
        .hunks
        .iter()
        .map(|hunk| {
            1 + hunk
                .lines
                .iter()
                .filter(|line| line.kind != LineKindView::HunkHeading)
                .count()
        })
        .sum();
    lines + usize::from(view.truncated.is_some())
}

/// Hands out ids for jobs `update` names itself (see [`LOCAL_JOB_BIT`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalJobs(u64);

impl LocalJobs {
    /// The next unused id.
    pub fn next_id(&mut self) -> JobId {
        self.0 += 1;
        JobId::new(LOCAL_JOB_BIT | self.0)
    }
}

/// A detail job's event. The pane names its jobs itself, so a start needs
/// no answer; products are checked against what is still pending.
pub(super) fn detail_event(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    let AppState {
        detail, local_jobs, ..
    } = next;
    match event {
        AppEvent::Produced(id, Product::Files(result)) => {
            commands.extend(detail.files_arrived(id, result, &mut || local_jobs.next_id()));
        }
        AppEvent::Produced(id, Product::FileDiff(key, result)) => {
            detail.diff_arrived(id, key, result);
        }
        AppEvent::Produced(_, Product::ContainedIn(oid, result)) => {
            detail.contained_arrived(oid, result);
        }
        AppEvent::Produced(id, Product::EvilProbe(oid, verdict)) => {
            detail.evil_probed(id, oid, verdict);
        }
        // Starts, and anything `answer` did not route here.
        AppEvent::Started(..)
        | AppEvent::Produced(..)
        | AppEvent::Progress(..)
        | AppEvent::Core(_) => {}
    }
}

/// The pane's actual layout for `size`: the breakpoint's kind first (the
/// percentage default), then that kind's saved override, if any, folded back
/// in. Two steps because which override applies depends on the kind the
/// first step already picked (`PaneSize::for_layout`).
#[must_use]
pub fn effective_layout(
    focus: DetailFocus,
    pane_size: &PaneSize,
    size: (u16, u16),
) -> DetailLayout {
    let kind = DetailLayout::compute(size.0, size.1, focus, None);
    let override_cells = pane_size.for_layout(&kind);
    DetailLayout::compute(size.0, size.1, focus, override_cells)
}

/// `Esc`'s two priority arms, answered before anything else `Esc` does:
/// closing the Explain panel first (`m2-details-and-diffs/17`, the pane
/// stays, on the same commit), then clearing a pinned compare base
/// (`m2-details-and-diffs/11`, closing the pane in the same press when it
/// is shown — ADR-0023's own `Escape if detail.shown` arm, further down
/// [`detail_key`]). `true` when either one answered the key.
fn escape_priority(next: &mut AppState, key: KeyPress) -> bool {
    if key.code != KeyCode::Escape {
        return false;
    }
    if next.detail.explain.is_some() {
        close_explain(next);
        return true;
    }
    if next.compare_base.take().is_some() {
        if next.detail.shown {
            next.detail.close();
        }
        return true;
    }
    false
}

/// The pane's keys, answered before the graph's. `true` when the key was the pane's.
pub(super) fn detail_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let plain = key.modifiers == Modifiers::NONE;
    // `<`/`>` (ADR-0028): the focused divider, whichever region has focus,
    // whenever the pane is shown — the pane being shown is what makes the
    // divider exist. A no-op (nothing changes, no command) in the `Tab`
    // layout, hidden, or against a bound: `resize_pane` says so itself.
    if plain && next.detail.shown && matches!(key.code, KeyCode::Char('<' | '>')) {
        let grow = key.code == KeyCode::Char('>');
        let size = (next.size.columns, next.size.rows);
        let layout = effective_layout(next.detail.focus, &next.detail.pane_size, size);
        if next.detail.resize_pane(&layout, grow, size) {
            commands.push(Command::SaveLayout(next.detail.pane_size));
        }
        return true;
    }
    if escape_priority(next, key) {
        return true;
    }
    let AppState {
        detail, local_jobs, ..
    } = next;
    let ids = &mut || local_jobs.next_id();
    match key.code {
        KeyCode::Enter if plain && detail.shown && detail.focus == DetailFocus::Diff => {
            // Diff already has focus: Enter has nothing further to open.
            true
        }
        KeyCode::Enter if plain && detail.shown && detail.focus == DetailFocus::Files => {
            detail.focus = DetailFocus::Diff;
            true
        }
        KeyCode::Enter if plain => {
            let Some(spec) = next.selected_spec() else {
                return false;
            };
            let AppState {
                detail, local_jobs, ..
            } = next;
            commands.extend(detail.open(spec, &mut || local_jobs.next_id()));
            true
        }
        KeyCode::Escape if detail.shown => {
            detail.close();
            true
        }
        KeyCode::Tab if detail.shown => {
            detail.cycle_focus(plain);
            true
        }
        KeyCode::BackTab if detail.shown => {
            detail.cycle_focus(false);
            true
        }
        _ if detail.shown && detail.focus == DetailFocus::Files => {
            files_key(detail, commands, ids, key.code, plain)
        }
        _ if detail.shown && detail.focus == DetailFocus::Diff => {
            diff_key(detail, commands, ids, key.code, plain)
        }
        _ => false,
    }
}

/// Keys active with the files list focused: cursor movement, paging, home/end
/// and `Space` to fold or unfold a directory row.
fn files_key(
    detail: &mut DetailState,
    commands: &mut Vec<Command>,
    ids: &mut impl FnMut() -> JobId,
    code: KeyCode,
    plain: bool,
) -> bool {
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            commands.extend(detail.move_files_cursor(1, ids));
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            commands.extend(detail.move_files_cursor(-1, ids));
            true
        }
        KeyCode::PageDown => {
            commands.extend(detail.move_files_cursor(PAGE_STEP, ids));
            true
        }
        KeyCode::PageUp => {
            commands.extend(detail.move_files_cursor(-PAGE_STEP, ids));
            true
        }
        KeyCode::Char('g') if plain => {
            commands.extend(detail.move_files_cursor(isize::MIN, ids));
            true
        }
        KeyCode::Char('G') if plain => {
            commands.extend(detail.move_files_cursor(isize::MAX, ids));
            true
        }
        KeyCode::Char(' ') if plain => {
            if let Loading::Ready(files) = &mut detail.files {
                let cursor = files.cursor;
                Arc::make_mut(files).toggle_dir(cursor);
            }
            true
        }
        _ => diff_option_key(detail, commands, ids, code, plain),
    }
}

/// Keys active with the diff body focused: scrolling, plus the same
/// view-option keys `files_key` falls back to (both panes share them).
fn diff_key(
    detail: &mut DetailState,
    commands: &mut Vec<Command>,
    ids: &mut impl FnMut() -> JobId,
    code: KeyCode,
    plain: bool,
) -> bool {
    let delta = match code {
        KeyCode::Char('j') | KeyCode::Down => Some(1),
        KeyCode::Char('k') | KeyCode::Up => Some(-1),
        KeyCode::PageDown => Some(PAGE_STEP),
        KeyCode::PageUp => Some(-PAGE_STEP),
        KeyCode::Char('g') if plain => Some(isize::MIN),
        KeyCode::Char('G') if plain => Some(isize::MAX),
        _ => None,
    };
    if let Some(delta) = delta {
        let len = diff_content_len_of(detail);
        detail.scroll_body(delta, len);
        return true;
    }
    diff_option_key(detail, commands, ids, code, plain)
}

/// View-option keys shared by both the files and diff panes (anywhere the
/// pane is shown and not focused on the graph): mode, whitespace, context and
/// the full-file load.
fn diff_option_key(
    detail: &mut DetailState,
    commands: &mut Vec<Command>,
    ids: &mut impl FnMut() -> JobId,
    code: KeyCode,
    plain: bool,
) -> bool {
    if !plain {
        return false;
    }
    match code {
        KeyCode::Char('v') => {
            detail.mode = match detail.mode {
                DiffMode::Unified => DiffMode::SideBySide,
                DiffMode::SideBySide => DiffMode::Unified,
            };
            true
        }
        KeyCode::Char('w') => {
            let mut options = detail.options;
            options.ignore_whitespace = !options.ignore_whitespace;
            commands.extend(detail.set_options(options, ids));
            true
        }
        // `word_tint` has no place in `graft_git::diff::DiffOptions` (a
        // git-query option, not a display one) — `W` flips it directly and
        // re-renders from the diff already in hand, no re-fetch.
        KeyCode::Char('W') => {
            detail.word_tint = !detail.word_tint;
            true
        }
        KeyCode::Char('[') => {
            let mut options = detail.options;
            options.context = options.context.saturating_sub(1);
            commands.extend(detail.set_options(options, ids));
            true
        }
        KeyCode::Char(']') => {
            let mut options = detail.options;
            options.context = options.context.saturating_add(1).min(MAX_CONTEXT);
            commands.extend(detail.set_options(options, ids));
            true
        }
        KeyCode::Char('L') => {
            commands.extend(detail.load_all(ids));
            true
        }
        _ => false,
    }
}

/// The diff body's current length, for [`DetailState::scroll_body`]'s clamp;
/// `0` while nothing is loaded, so a scroll key is a harmless no-op then.
fn diff_content_len_of(detail: &DetailState) -> usize {
    match &detail.diff {
        Loading::Ready(diff) => diff_content_len(diff),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_ui::detail::{FileRow, FileRowKind, FileStatusView, HunkView, LineView};

    fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
    }

    fn next_id_from(counter: &mut u64) -> JobId {
        *counter += 1;
        JobId::new(LOCAL_JOB_BIT | *counter)
    }

    /// `history_of(3, 0)` with a ready, empty radar context, so
    /// `findings_for` can run without a real repository: `AppState` for
    /// the `open_explain`/`close_explain`/Esc-priority tests
    /// (`m2-details-and-diffs/17`).
    fn state_with_duplicate_finding(mark: Oid, copy: Oid) -> AppState {
        let mut state = AppState {
            history: Some(crate::view::tests::history_of(3, 0)),
            ..AppState::default()
        };
        state.radar.context = Loading::Ready(Arc::new(crate::jobs::radar::RadarContext {
            default_chain: std::collections::HashSet::new(),
            previous_default_tips: Vec::new(),
        }));
        state.radar.duplicate_findings.insert(
            mark,
            graft_ops::mistake_radar::Finding {
                kind: MistakeRadarKind::DuplicateCommit,
                severity: graft_core::mistake_radar::Severity::Warning,
                at: mark,
                evidence: graft_ops::mistake_radar::Evidence::Duplicate { copies: vec![copy] },
            },
        );
        state
    }

    /// `open_explain` on a marked, selected commit resolves its finding,
    /// opens the pane on it, and fills `explain`/`explain_highlight`.
    #[test]
    fn open_explain_resolves_the_selected_rows_finding() {
        let cursor = oid(3); // `history_of(3, 0)`'s cursor row.
        let mut state = state_with_duplicate_finding(cursor, oid(9));
        state.detail.explain_requested = Some(cursor);
        let mut counter = 0;

        let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));

        assert_eq!(state.detail.explain_requested, None);
        assert_eq!(state.detail.spec, Some(DetailSpec::Commit(cursor)));
        let explain = state.detail.explain.expect("a finding was found");
        assert!(!explain.title.as_str().is_empty());
        assert!(!state.detail.explain_highlight.is_empty());
    }

    /// A stale request — the marked commit no longer has a finding — opens
    /// the pane (the commit is still real) but leaves `explain` alone
    /// rather than erroring.
    #[test]
    fn open_explain_with_no_finding_opens_the_pane_but_not_explain() {
        let cursor = oid(3);
        let mut state = AppState {
            history: Some(crate::view::tests::history_of(3, 0)),
            ..AppState::default()
        };
        state.radar.context = Loading::Ready(Arc::new(crate::jobs::radar::RadarContext {
            default_chain: std::collections::HashSet::new(),
            previous_default_tips: Vec::new(),
        }));
        state.detail.explain_requested = Some(cursor);
        let mut counter = 0;

        let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));

        assert_eq!(state.detail.explain_requested, None);
        assert_eq!(state.detail.spec, Some(DetailSpec::Commit(cursor)));
        assert!(state.detail.explain.is_none());
    }

    /// Nothing requested this tick: a no-op, no commands.
    #[test]
    fn open_explain_with_nothing_requested_is_a_no_op() {
        let mut state = AppState::default();
        let mut counter = 0;

        let commands = open_explain(&mut state, &mut || next_id_from(&mut counter));

        assert!(commands.is_empty());
        assert!(state.detail.explain.is_none());
    }

    /// `close_explain` clears the view and the highlight, and nothing else.
    #[test]
    fn close_explain_clears_the_view_and_highlight() {
        let mut state = state_with_duplicate_finding(oid(3), oid(9));
        state.detail.explain_requested = Some(oid(3));
        let mut counter = 0;
        let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));
        assert!(state.detail.explain.is_some());

        close_explain(&mut state);

        assert!(state.detail.explain.is_none());
        assert!(state.detail.explain_highlight.is_empty());
    }

    /// `Esc` closes Explain first, on its own, leaving the pane open on the
    /// same commit; a second `Esc` then falls through to closing the pane.
    #[test]
    fn escape_closes_explain_before_anything_else_esc_does() {
        let mut state = state_with_duplicate_finding(oid(3), oid(9));
        state.detail.explain_requested = Some(oid(3));
        let mut counter = 0;
        let mut commands = Vec::new();
        let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));
        assert!(state.detail.explain.is_some());
        state.detail.shown = true;

        let handled = detail_key(&mut state, &mut commands, KeyPress::plain(KeyCode::Escape));

        assert!(handled);
        assert!(state.detail.explain.is_none(), "first Esc closes Explain");
        assert!(state.detail.shown, "the pane itself stays open");

        let handled_again = detail_key(&mut state, &mut commands, KeyPress::plain(KeyCode::Escape));
        assert!(handled_again);
        assert!(!state.detail.shown, "second Esc closes the pane");
    }

    /// `toggle_compare`'s first branch: no base yet pins one, no command.
    /// A commit spec probes once; the same merge asked about again is
    /// answered from the cache without a second `git` spawn.
    #[test]
    fn a_commit_spec_probes_once_and_a_revisit_is_a_cache_hit() {
        let mut detail = DetailState::default();
        let policy = Policy::default();
        let mut counter = 0;
        let mut ids = || next_id_from(&mut counter);

        detail.shown = true;
        detail.spec = Some(DetailSpec::Commit(oid(1)));
        let commands = detail.probe_evil(&policy, &mut ids);
        let [Command::EvilProbe(id, probed)] = commands.as_slice() else {
            panic!("a fresh merge asks for one probe, got {commands:?}");
        };
        assert_eq!(*probed, oid(1));
        assert_eq!(detail.evil, Loading::Pending(*id));

        detail.evil_probed(*id, oid(1), EvilVerdict::Clean);
        assert_eq!(detail.evil, Loading::Ready(EvilVerdict::Clean));

        // Away to another commit, then back.
        detail.spec = Some(DetailSpec::Commit(oid(2)));
        let away = detail.probe_evil(&policy, &mut ids);
        assert!(matches!(away.as_slice(), [Command::EvilProbe(..)]));
        detail.spec = Some(DetailSpec::Commit(oid(1)));
        let back = detail.probe_evil(&policy, &mut ids);
        assert!(
            back.iter()
                .all(|command| !matches!(command, Command::EvilProbe(..))),
            "a cached verdict asks for nothing, got {back:?}"
        );
        assert_eq!(detail.evil, Loading::Ready(EvilVerdict::Clean));
    }

    /// Moving off a merge before its probe answers cancels it, and the late
    /// answer changes nothing.
    #[test]
    fn moving_off_a_pending_probe_cancels_it_and_drops_its_late_answer() {
        let mut detail = DetailState::default();
        let policy = Policy::default();
        let mut counter = 0;
        let mut ids = || next_id_from(&mut counter);

        detail.shown = true;
        detail.spec = Some(DetailSpec::Commit(oid(1)));
        let first = detail.probe_evil(&policy, &mut ids);
        let [Command::EvilProbe(stale, _)] = first.as_slice() else {
            panic!("expected one probe, got {first:?}");
        };
        let stale = *stale;

        detail.spec = Some(DetailSpec::Commit(oid(2)));
        let moved = detail.probe_evil(&policy, &mut ids);
        assert!(
            moved.contains(&Command::CancelJob(stale)),
            "the superseded probe is cancelled, got {moved:?}"
        );

        detail.evil_probed(stale, oid(1), EvilVerdict::Evil { files: vec![] });
        assert!(
            matches!(detail.evil, Loading::Pending(id) if id != stale),
            "the stale answer is dropped, leaving the newer probe pending"
        );
        assert!(detail.evil_cache.get(&oid(1)).is_none());
    }

    /// Nothing is probed with the evil-merge kind off, with the pane hidden,
    /// or on a spec that is not one commit.
    #[test]
    fn no_probe_with_the_kind_off_the_pane_hidden_or_a_compare_spec() {
        let mut counter = 0;
        let mut ids = || next_id_from(&mut counter);
        let mut off = Policy::default();
        off.set(MistakeRadarKind::EvilMerge, None);

        let mut detail = DetailState {
            shown: true,
            spec: Some(DetailSpec::Commit(oid(1))),
            ..DetailState::default()
        };
        assert_eq!(detail.probe_evil(&off, &mut ids), Vec::new());
        assert_eq!(detail.evil, Loading::Idle);

        let policy = Policy::default();
        detail.shown = false;
        assert_eq!(detail.probe_evil(&policy, &mut ids), Vec::new());

        detail.shown = true;
        detail.spec = Some(DetailSpec::Compare {
            base: oid(1),
            head: oid(2),
        });
        assert_eq!(detail.probe_evil(&policy, &mut ids), Vec::new());
        detail.spec = Some(DetailSpec::WorkingTree);
        assert_eq!(detail.probe_evil(&policy, &mut ids), Vec::new());
        assert_eq!(detail.evil, Loading::Idle);
    }

    /// A refresh keeps the verdicts: a merge's tree is immutable, so no
    /// answer can go stale.
    #[test]
    fn a_refresh_keeps_the_evil_cache() {
        let mut detail = DetailState::default();
        let mut counter = 0;
        detail.evil_cache.insert(oid(1), EvilVerdict::Clean);
        let _ = detail.refresh(&mut || next_id_from(&mut counter));
        assert_eq!(detail.evil_cache.get(&oid(1)), Some(&EvilVerdict::Clean));
    }

    #[test]
    fn toggle_compare_with_no_base_pins_one_and_emits_nothing() {
        let mut state = super::super::state::AppState::default();
        let mut ids = 0u64;
        let commands = toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
        assert!(commands.is_empty());
        assert_eq!(
            state.compare_base,
            Some(CompareBase {
                oid: oid(1),
                row: 0
            })
        );
    }

    /// `toggle_compare`'s second branch: the same oid again clears it, no
    /// command.
    #[test]
    fn toggle_compare_on_the_same_oid_clears_it_and_emits_nothing() {
        let mut state = super::super::state::AppState::default();
        let mut ids = 0u64;
        toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
        let commands = toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
        assert!(commands.is_empty());
        assert_eq!(state.compare_base, None);
    }

    /// `toggle_compare`'s third branch: a different oid opens the compare
    /// and keeps the base set.
    #[test]
    fn toggle_compare_on_a_different_oid_opens_compare_and_keeps_the_base() {
        let mut state = super::super::state::AppState::default();
        let mut ids = 0u64;
        toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
        let commands = toggle_compare(&mut state, 1, oid(2), &mut || next_id_from(&mut ids));
        assert!(!commands.is_empty());
        assert_eq!(
            state.detail.spec,
            Some(DetailSpec::Compare {
                base: oid(1),
                head: oid(2)
            })
        );
        assert_eq!(
            state.compare_base,
            Some(CompareBase {
                oid: oid(1),
                row: 0
            }),
            "the base stays live as the cursor moves"
        );
    }

    /// `Esc` clears a pinned base and closes the pane in the same press
    /// (ADR-0023's existing `Escape` order, m2-details-and-diffs/11).
    #[test]
    fn escape_clears_a_pinned_base_and_closes_the_pane_in_one_press() {
        let mut state = super::super::state::AppState {
            compare_base: Some(CompareBase {
                oid: oid(1),
                row: 0,
            }),
            ..Default::default()
        };
        state.detail.shown = true;
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Escape)
        ));

        assert_eq!(state.compare_base, None);
        assert!(!state.detail.shown);
    }

    /// `Esc` clears a pinned base even while the pane never opened (`x`
    /// pins a base with no command, so the pane can be hidden the whole
    /// time).
    #[test]
    fn escape_clears_a_pinned_base_while_the_pane_is_hidden() {
        let mut state = super::super::state::AppState {
            compare_base: Some(CompareBase {
                oid: oid(1),
                row: 0,
            }),
            ..Default::default()
        };
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Escape)
        ));

        assert_eq!(state.compare_base, None);
        assert!(!state.detail.shown);
    }

    /// An empty, valid diff — enough for the option/scroll keys to act on.
    fn empty_diff_view() -> DiffView {
        DiffView {
            path: sanitize_line("f"),
            old_path: None,
            status: FileStatusView::Modified,
            hunks: Vec::new(),
            binary: None,
            truncated: None,
            untracked: false,
        }
    }

    #[test]
    fn angle_keys_resize_the_right_pane_and_leave_below_alone() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        state.size = super::super::state::Size {
            columns: 200,
            rows: 50,
        };
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('>'))
        ));

        let grown = state.detail.pane_size.right_columns.expect("resized once");
        assert!(grown > 0);
        assert_eq!(state.detail.pane_size.below_rows, None);
        assert_eq!(commands, vec![Command::SaveLayout(state.detail.pane_size)]);

        commands.clear();
        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('<'))
        ));
        let shrunk = state.detail.pane_size.right_columns.expect("still set");
        assert_eq!(shrunk, grown - RESIZE_STEP);
    }

    #[test]
    fn angle_keys_resize_rows_in_the_below_layout() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        state.size = super::super::state::Size {
            columns: 120,
            rows: 50,
        };
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('>'))
        ));

        assert!(state.detail.pane_size.below_rows.is_some());
        assert_eq!(state.detail.pane_size.right_columns, None);
    }

    #[test]
    fn angle_keys_are_a_no_op_in_the_tab_layout() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        state.size = super::super::state::Size {
            columns: 80,
            rows: 50,
        };
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('>'))
        ));

        assert_eq!(state.detail.pane_size, PaneSize::default());
        assert!(commands.is_empty());
    }

    #[test]
    fn angle_keys_are_a_no_op_with_the_pane_hidden() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = false;
        state.size = super::super::state::Size {
            columns: 200,
            rows: 50,
        };
        let mut commands = Vec::new();

        assert!(!detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('>'))
        ));
        assert_eq!(state.detail.pane_size, PaneSize::default());
        assert!(commands.is_empty());
    }

    #[test]
    fn plus_and_minus_never_resize_the_pane() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        state.size = super::super::state::Size {
            columns: 200,
            rows: 50,
        };
        let mut commands = Vec::new();

        detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('+')),
        );
        detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('-')),
        );

        assert_eq!(state.detail.pane_size, PaneSize::default());
    }

    #[test]
    fn set_pane_cells_and_for_layout_round_trip() {
        let size = (200, 50);
        let layout = DetailLayout::compute(size.0, size.1, DetailFocus::Files, None);
        let mut state = DetailState {
            shown: true,
            ..DetailState::default()
        };

        assert!(state.set_pane_cells(&layout, 60, size));
        assert_eq!(
            state.pane_size.for_layout(&layout),
            state.pane_size.right_columns
        );
        assert_eq!(state.pane_size.right_columns, Some(60));
    }

    #[test]
    fn bounded_forgets_its_oldest_entry_and_replaces_in_place() {
        let mut map = Bounded::new(2);
        map.insert(1, "a");
        map.insert(2, "b");
        map.insert(1, "a2");
        map.insert(3, "c");
        assert_eq!(map.get(&2), None);
        assert_eq!(map.get(&1), Some(&"a2"));
        assert_eq!(map.get(&3), Some(&"c"));
        map.clear();
        assert_eq!(map.get(&3), None);
    }

    #[test]
    fn v_toggles_side_by_side_and_back_but_is_ignored_from_graph_focus() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Diff;
        let mut commands = Vec::new();
        let key = KeyPress::plain(KeyCode::Char('v'));

        assert!(detail_key(&mut state, &mut commands, key));
        assert_eq!(state.detail.mode, DiffMode::SideBySide);
        assert!(commands.is_empty());

        assert!(detail_key(&mut state, &mut commands, key));
        assert_eq!(state.detail.mode, DiffMode::Unified);

        state.detail.focus = DetailFocus::Graph;
        assert!(!detail_key(&mut state, &mut commands, key));
        assert_eq!(state.detail.mode, DiffMode::Unified);
    }

    #[test]
    fn focus_cycles_both_ways() {
        let mut state = DetailState::default();
        state.cycle_focus(true);
        assert_eq!(state.focus, DetailFocus::Files);
        state.cycle_focus(true);
        state.cycle_focus(true);
        assert_eq!(state.focus, DetailFocus::Graph);
        state.cycle_focus(false);
        assert_eq!(state.focus, DetailFocus::Diff);
    }

    #[test]
    fn enter_is_a_no_op_once_diff_already_has_focus() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Diff;
        let mut commands = Vec::new();
        let key = KeyPress::plain(KeyCode::Enter);

        assert!(detail_key(&mut state, &mut commands, key));
        assert_eq!(state.detail.focus, DetailFocus::Diff);
        assert!(commands.is_empty());
    }

    #[test]
    fn bracket_keys_step_context_and_reissue_read_file_diff() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Diff;
        state.detail.diff = Loading::Ready(Arc::new(empty_diff_view()));
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char(']'))
        ));
        assert_eq!(
            state.detail.options.context,
            DiffOptions::default().context + 1
        );

        commands.clear();
        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('['))
        ));
        assert_eq!(state.detail.options.context, DiffOptions::default().context);
    }

    #[test]
    fn w_toggles_ignore_whitespace_and_capital_w_toggles_word_tint_locally() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Diff;
        state.detail.diff = Loading::Ready(Arc::new(empty_diff_view()));
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('w'))
        ));
        assert!(state.detail.options.ignore_whitespace);

        let before = state.detail.word_tint;
        commands.clear();
        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('W'))
        ));
        assert_eq!(state.detail.word_tint, !before);
        assert!(
            commands.is_empty(),
            "W is a local display toggle, no re-fetch"
        );
    }

    #[test]
    fn capital_l_loads_the_full_file() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Diff;
        state.detail.diff = Loading::Ready(Arc::new(empty_diff_view()));
        state.detail.max_lines = MAX_LINES;
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('L'))
        ));
        assert_eq!(state.detail.max_lines, usize::MAX);
    }

    #[test]
    fn space_folds_a_directory_row_in_the_files_list() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Files;
        let mut files = FileListView::default();
        files.rows.push(FileRow {
            depth: 0,
            name: sanitize_line("src/"),
            kind: FileRowKind::Dir { collapsed: false },
            entry: None,
        });
        files.rows.push(FileRow {
            depth: 1,
            name: sanitize_line("lib.rs"),
            kind: FileRowKind::File {
                status: FileStatusView::Modified,
                added: Some(1),
                removed: Some(1),
            },
            entry: Some(0),
        });
        files.cursor = 0;
        state.detail.files = Loading::Ready(Arc::new(files));
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char(' '))
        ));
        let Loading::Ready(files) = &state.detail.files else {
            unreachable!()
        };
        assert_eq!(files.rows[0].kind, FileRowKind::Dir { collapsed: true });
    }

    #[test]
    fn g_and_capital_g_jump_the_diff_body_to_its_ends() {
        let mut state = super::super::state::AppState::default();
        state.detail.shown = true;
        state.detail.focus = DetailFocus::Diff;
        state.detail.diff = Loading::Ready(Arc::new(DiffView {
            hunks: vec![HunkView {
                heading: sanitize_line("@@ -1,3 +1,3 @@"),
                lines: (0..5)
                    .map(|_| LineView {
                        kind: LineKindView::Context,
                        old_no: None,
                        new_no: None,
                        text: sanitize_line(""),
                        words: Vec::new(),
                        syntax: Vec::new(),
                        pair: None,
                    })
                    .collect(),
            }],
            ..empty_diff_view()
        }));
        let mut commands = Vec::new();

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('G'))
        ));
        assert!(state.detail.body_top > 0);

        assert!(detail_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Char('g'))
        ));
        assert_eq!(state.detail.body_top, 0);
    }
}
