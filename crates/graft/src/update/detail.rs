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
use graft_core::oid::Oid;
use graft_git::contained::Contained;
use graft_git::diff::{DiffOptions, DiffSpec, FileList};
use graft_term::sanitize::{sanitize_line, sanitize_path_bytes};
use graft_ui::detail::{ContainedIn, DetailFocus, DiffMode, DiffView, FileListView};

use super::events::{AppEvent, Command, Product};
use super::state::AppState;

/// Ids `update` hands out itself carry this bit; the job runner's own counter
/// never reaches it, so the two can never name the same job.
pub const LOCAL_JOB_BIT: u64 = 1 << 63;

/// Diffs kept for stepping back through files.
const DIFF_CACHE: usize = 8;
/// "Contained in" answers kept for stepping back through commits.
const CONTAINED_CACHE: usize = 64;
/// Lines of one file's diff read before it is cut short (`L` loads the rest).
const MAX_LINES: usize = 5_000;

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
            options: DiffOptions::default(),
            mode: DiffMode::default(),
            word_tint: true,
            max_lines: MAX_LINES,
            body_top: 0,
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
        // Starts, and anything `answer` did not route here.
        AppEvent::Started(..)
        | AppEvent::Produced(..)
        | AppEvent::Progress(..)
        | AppEvent::Core(_) => {}
    }
}

/// The pane's keys, answered before the graph's. `true` when the key was the pane's.
pub(super) fn detail_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let AppState {
        detail, local_jobs, ..
    } = next;
    let ids = &mut || local_jobs.next_id();
    let plain = key.modifiers == Modifiers::NONE;
    match key.code {
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
        KeyCode::Char('j') | KeyCode::Down
            if detail.shown && detail.focus == DetailFocus::Files =>
        {
            commands.extend(detail.move_files_cursor(1, ids));
            true
        }
        KeyCode::Char('k') | KeyCode::Up if detail.shown && detail.focus == DetailFocus::Files => {
            commands.extend(detail.move_files_cursor(-1, ids));
            true
        }
        KeyCode::Char('v') if plain && detail.shown && detail.focus != DetailFocus::Graph => {
            detail.mode = match detail.mode {
                DiffMode::Unified => DiffMode::SideBySide,
                DiffMode::SideBySide => DiffMode::Unified,
            };
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
