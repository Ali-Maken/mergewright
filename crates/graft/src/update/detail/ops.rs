//! `impl DetailState`: everything that changes the pane's state in response
//! to an event — opening, following the selection, jobs arriving, scrolling,
//! resizing. Pure like the rest of `update`: every read is a [`Command`]
//! whose answer arrives as a [`Product`](crate::update::events::Product).

use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_git::contained::Contained;
use graft_git::diff::{DiffOptions, FileList};
use graft_ops::mistake_radar::Policy;
use graft_ops::mistake_radar::probes::EvilVerdict;
use graft_term::sanitize::{sanitize_line, sanitize_path_bytes};
use graft_ui::detail::{ContainedIn, DetailFocus, DiffView, FileListView};
use graft_ui::detail_pane::DetailLayout;

use crate::update::events::Command;

use super::state::{
    BELOW_MIN, DetailSpec, DetailState, DiffKey, GRAPH_MIN_COLUMNS, GRAPH_MIN_ROWS, Loading,
    MAX_LINES, RIGHT_MIN, stepped,
};

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
        selected: Option<Oid>,
        policy: &Policy,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        let wanted = match selected {
            Some(oid) if policy.severity(MistakeRadarKind::EvilMerge).is_some() => Some(oid),
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
        // Review M5 (m2-details-and-diffs round 1b): `L` applies to the file
        // it was pressed on, not to every file read for the rest of the
        // session.
        self.max_lines = MAX_LINES;
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
        if let Loading::Pending(id) = self.files {
            commands.push(Command::CancelJob(id));
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
            // Review M5: `L` (load all) is per file; moving off it resets
            // the cap for whatever the cursor lands on next.
            self.max_lines = MAX_LINES;
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
        let len = super::control::diff_content_len_of(self);
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
