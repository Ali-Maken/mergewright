//! The detail pane's data: what it is about, its caches, and its own size.
//! No behaviour beyond `Default`/small accessors — see [`super::ops`] for
//! everything that changes this state in response to an event.

use std::collections::VecDeque;
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::diff::{DiffOptions, DiffSpec, FileDiff, FileList};
use graft_ops::mistake_radar::probes::EvilVerdict;
use graft_ui::detail::{
    ContainedIn, DetailFocus, DiffMode, DiffView, ExplainView, FileListView, WorkingTreeSection,
};
use graft_ui::detail_pane::DetailLayout;

use crate::update::events::Command;
use crate::update::state::AppState;

/// Diffs kept for stepping back through files.
pub(crate) const DIFF_CACHE: usize = 8;
/// "Contained in" answers kept for stepping back through commits.
pub(crate) const CONTAINED_CACHE: usize = 64;
/// Evil-merge verdicts kept for stepping back through merges. A merge's tree
/// is immutable, so a verdict never goes stale and a refresh keeps them.
pub(crate) const EVIL_CACHE: usize = 16;
/// Lines of one file's diff read before it is cut short (`L` loads the rest).
pub(crate) const MAX_LINES: usize = 5_000;
/// `<`/`>` (ADR-0028) move the focused divider by this many cells per press.
pub(crate) const RESIZE_STEP: u16 = 4;
/// The pane's own minimum, mirrored from `graft_ui::detail_pane::DetailLayout`
/// (private there): 42% of a 160-column screen never goes below this.
pub(crate) const RIGHT_MIN: u16 = 48;
/// The pane's own minimum in the stacked layout.
pub(crate) const BELOW_MIN: u16 = 8;
/// The graph never shrinks below this many columns for the pane's sake.
pub(crate) const GRAPH_MIN_COLUMNS: u16 = 40;
/// The graph never shrinks below this many rows for the pane's sake.
pub(crate) const GRAPH_MIN_ROWS: u16 = 6;

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
pub(crate) fn stepped(current: u16, grow: bool, min: u16, max: u16) -> u16 {
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
/// (m2-details-and-diffs/11): no base yet pins one and opens the pane on
/// `Compare { base, head: base }` (m7-14); the same
/// `oid` again clears it, no command; a different `oid` opens the compare
/// and **keeps** the base, so the compare stays live as the cursor moves
/// (`update::selection::follow_selection`).
pub fn toggle_compare(
    state: &mut AppState,
    row: usize,
    oid: Oid,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // A compare pinned while a blame trail waits under the graph is a new
    // task: the trail ends and the base it set aside is dropped, since he
    // just chose another (20's pinned-base ruling).
    let mut commands = Vec::new();
    let left_blame = crate::update::place::end_for_compare(state, &mut commands);
    commands.extend(match state.compare_base {
        None => {
            // m7-14: pinning opens the pane on the base itself, so the
            // next motion is already a live compare.
            // Focus stays where it was: the next `j`/`k` must move the
            // graph's cursor, which is what makes the compare live.
            state.compare_base = Some(CompareBase { oid, row });
            let focus = state.detail.focus;
            let opened = state.detail.open(
                DetailSpec::Compare {
                    base: oid,
                    head: oid,
                },
                next_id,
            );
            state.detail.focus = focus;
            opened
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
    });
    if left_blame && let Some(base) = state.compare_base {
        state.status = crate::update::state::Status::Waiting(format!(
            "left blame for compare base {}",
            crate::update::place::short(base.oid)
        ));
    }
    commands
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
    /// Which changes-pane group the row is in
    /// (m3-working-tree-undo-and-stash/04). This is not decoration: it picks
    /// the diff git is asked for — `Staged` reads the index against `HEAD`,
    /// the other two read the working tree against the index — so the same
    /// path staged *and* unstaged caches two different diffs. `None` for
    /// every spec but the working tree's.
    pub section: Option<WorkingTreeSection>,
    /// Context and whitespace options.
    pub options: DiffOptions,
    /// Lines read before cutting short.
    pub max_lines: usize,
}

/// A small map that forgets its oldest entry when full.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bounded<K, V> {
    pub(crate) entries: VecDeque<(K, V)>,
    pub(crate) cap: usize,
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

    /// Forget every entry whose key `drop` picks.
    pub fn forget(&mut self, mut drop: impl FnMut(&K) -> bool) {
        self.entries.retain(|(key, _)| !drop(key));
    }
}

/// The detail pane's state (`AppState::detail`); hidden by default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailState {
    /// What the pane is about, once opened.
    pub spec: Option<DetailSpec>,
    /// The cursor's commit the last time a pinned compare followed it
    /// (m7-14): the pane re-points only when the cursor has moved, so a
    /// compare picked by click or `x` is not undone by the same event's
    /// follow.
    pub compare_followed: Option<Oid>,
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
    /// The same diff as git parsed it, before the view model
    /// (m3-working-tree-undo-and-stash/04): `graft_git::diff::hunk_patch`
    /// builds a stage/discard patch from these bytes, never from the
    /// sanitised, syntax-highlighted [`DiffView`]. Always in step with
    /// `diff` — both come from one `Product::FileDiff`.
    pub diff_raw: Option<Arc<FileDiff>>,
    /// Recent diffs, newest last: the drawable view and the raw diff the
    /// patch builder needs, cached together so they can never drift apart.
    pub diff_cache: Bounded<DiffKey, (Arc<DiffView>, Arc<FileDiff>)>,
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
    /// The diff region's line cursor: a row of the flattened diff body
    /// (`graft_ui::detail::DiffView::body_row`), which `Space` and `d` act
    /// on in the changes pane. Every spec has it and draws it, so `j`/`k` in
    /// the diff region always mean the same thing (m7-power-tools/20b); only
    /// the changes pane writes from it.
    pub body_cursor: usize,
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
    /// Where the changes-pane cursor was when a refresh started, so the list
    /// that arrives can put it back (`ops::refresh`, `ops::files_arrived`).
    /// `None` at every other moment.
    pub kept_cursor: Option<KeptCursor>,
    /// The line `w`, `[`/`]` or `L` found the line cursor on, owed to the
    /// re-read diff when it arrives (m7-power-tools/20b). Any other diff
    /// shown first drops it.
    pub kept_line: Option<KeptLine>,
    /// The "contained in" query in flight, and its commit (G-settle): `load`
    /// cancels it when the pane moves to another commit, `abandon` when a
    /// burst starts, and `contained_arrived` drops any other id's answer.
    pub(crate) contained_job: Option<(JobId, Oid)>,
    /// `abandon` cancelled the spec's own reads mid-burst, so following the
    /// same spec again reads it again instead of keeping a pane with
    /// nothing coming.
    pub(crate) abandoned: bool,
}

/// Where the line cursor was when the same file was read again under new
/// options: its line, and how many rows below the view's top it sat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptLine {
    /// The line under the cursor.
    pub at: super::cursor::LineRef,
    /// Rows between the view's top and the cursor.
    pub offset: usize,
}

/// Where the changes-pane cursor was before a write's refresh.
///
/// Every write reloads the file list, and the fresh list's cursor starts on
/// the first file — usually one under STAGED. Staging the last unstaged hunk
/// of a file therefore moved the cursor to some other file in another
/// section, where the next `Space` means the **opposite** write. Nothing on
/// screen said the cursor had moved (m3/04 review M7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCursor {
    /// The file the cursor was on, as git spells it.
    pub path: Vec<u8>,
    /// The group it was in. Direction comes from this, so it is the half
    /// that must not be guessed wrong.
    pub section: Option<WorkingTreeSection>,
    /// How far down that group's files the cursor was, for when the file
    /// itself is gone from the group — staged, discarded or deleted.
    pub position: usize,
    /// The diff region's line cursor.
    pub body: usize,
}

impl Default for DetailState {
    fn default() -> Self {
        Self {
            spec: None,
            compare_followed: None,
            shown: false,
            focus: DetailFocus::Graph,
            files: Loading::Idle,
            list: None,
            diff: Loading::Idle,
            diff_raw: None,
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
            body_cursor: 0,
            pane_size: PaneSize::default(),
            dragging: false,
            explain_requested: None,
            explain: None,
            explain_highlight: Vec::new(),
            kept_cursor: None,
            kept_line: None,
            contained_job: None,
            abandoned: false,
        }
    }
}
