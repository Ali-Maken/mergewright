//! `impl DetailState`: everything that changes the pane's state in response
//! to an event — opening, following the selection, jobs arriving, scrolling,
//! resizing. Pure like the rest of `update`: every read is a [`Command`]
//! whose answer arrives as a [`Product`](crate::update::events::Product).

use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_git::contained::Contained;
use graft_git::diff::{DiffOptions, FileDiff, FileList};
use graft_ops::mistake_radar::Policy;
use graft_ops::mistake_radar::probes::EvilVerdict;
use graft_term::sanitize::{sanitize_line, sanitize_path_bytes};
use graft_ui::detail::{
    ContainedIn, DetailFocus, DiffMode, DiffView, FileListView, WorkingTreeSection,
};
use graft_ui::detail_pane::DetailLayout;

use crate::update::events::Command;

use super::cursor::{LineRef, line_row, row_line};
use super::state::{
    BELOW_MIN, DetailSpec, DetailState, DiffKey, GRAPH_MIN_COLUMNS, GRAPH_MIN_ROWS, KeptCursor,
    KeptLine, Loading, MAX_LINES, RIGHT_MIN, stepped,
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
            Some(spec) if self.shown => {
                // Where the user was, so the list that arrives can put them
                // back rather than dropping them on the first file
                // (m3/04 review M7).
                self.kept_cursor = self.cursor_to_keep();
                self.load(spec, next_id)
            }
            Some(_) | None => Vec::new(),
        }
    }

    /// A file changed on disk without an operation — a take in the
    /// resolver (m6-conflicts/85 B). Drops every cached working-tree diff of
    /// `path`, whatever its section, options or line cap; when the changes
    /// pane's cursor is on `path`, asks for its diff again. The cursor, the
    /// list and every other cached diff stay as they are.
    ///
    /// Without this the pane served the diff from before the write: the
    /// cache is keyed by spec and list entry, and nothing about either
    /// changes when the file does.
    pub fn file_written(
        &mut self,
        path: &std::path::Path,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        let written = path_bytes(path);
        // Cached entries index the list they were read against. With the
        // working tree's list on hand, only `path`'s go; without it (the pane
        // is on a commit) nothing can say which index was `path`, so every
        // working-tree diff goes — each is one read away.
        let list = (self.spec == Some(DetailSpec::WorkingTree))
            .then_some(self.list.as_ref())
            .flatten();
        let is_written = |entry: u32| {
            list.is_none_or(|list| {
                usize::try_from(entry)
                    .ok()
                    .and_then(|index| list.files.get(index))
                    .is_some_and(|file| file.path == written)
            })
        };
        self.diff_cache
            .forget(|key| key.spec == DetailSpec::WorkingTree && is_written(key.entry));
        let on_it = self.shown && list.is_some() && self.selected_entry().is_some_and(is_written);
        if !on_it {
            return Vec::new();
        }
        // The same place in the new diff: `body_cursor` is clamped to the
        // diff on show wherever it is read.
        let (top, cursor) = (self.body_top, self.body_cursor);
        let commands = self.show_cursor(next_id);
        self.body_top = top;
        self.body_cursor = cursor;
        commands
    }

    /// The changes-pane cursor as [`KeptCursor`] records it, or `None` off
    /// the changes pane or before its list has arrived.
    fn cursor_to_keep(&self) -> Option<KeptCursor> {
        if self.spec != Some(DetailSpec::WorkingTree) {
            return None;
        }
        let Loading::Ready(files) = &self.files else {
            return None;
        };
        let list = self.list.as_ref()?;
        let section = files.selected_section();
        let entry = files.selected_entry()?;
        let path = list.files.get(usize::try_from(entry).ok()?)?.path.clone();
        let position = section_files(files, list, section)
            .iter()
            .position(|(row, _)| *row == files.cursor)
            .unwrap_or(0);
        Some(KeptCursor {
            path,
            section,
            position,
            body: self.body_cursor,
        })
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
        self.body_cursor = 0;
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
                let kept = self.kept_cursor.take();
                let (files, same_file) = put_cursor_back(files, &list, kept.as_ref());
                self.files = Loading::Ready(files);
                self.list = Some(list);
                let commands = self.show_cursor(next_id);
                // The diff region's row only means anything while the file
                // under the cursor is the same one. The row index is kept,
                // not the hunk it named: after a stage that hunk is gone, and
                // the nearest honest answer is the same place in the file.
                // `body_cursor()` clamps it, and `body_top` follows it so the
                // row is on screen (`diff_body` skips its pinned heading for
                // the cursor's own row).
                if let (Some(kept), true) = (kept, same_file) {
                    self.body_cursor = kept.body;
                    self.body_top = kept.body;
                }
                commands
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

    /// Put the files cursor on `row` and ask for its diff, as a move onto it
    /// would; nothing when it is already there (its diff is on show or on
    /// its way). `show_line`'s selection (m7-power-tools/20b).
    pub(super) fn select_file_row(
        &mut self,
        row: usize,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        let Loading::Ready(files) = &mut self.files else {
            return Vec::new();
        };
        if files.cursor == row {
            return Vec::new();
        }
        Arc::make_mut(files).cursor = row;
        self.max_lines = MAX_LINES;
        self.show_cursor(next_id)
    }

    /// Show the diff under the files cursor: from the cache, or by asking.
    /// The message row and directories show no diff.
    fn show_cursor(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let mut commands = Vec::new();
        if let Loading::Pending(id) = self.diff {
            commands.push(Command::CancelJob(id));
        }
        self.diff = Loading::Idle;
        self.diff_raw = None;
        self.body_top = 0;
        self.body_cursor = 0;
        self.kept_line = None;
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
            section: self.selected_section(),
            options: self.options,
            max_lines: self.max_lines,
        };
        if let Some((view, raw)) = self.diff_cache.get(&key) {
            self.diff = Loading::Ready(Arc::clone(view));
            self.diff_raw = Some(Arc::clone(raw));
        } else {
            let id = next_id();
            self.diff = Loading::Pending(id);
            commands.push(Command::ReadFileDiff(id, key, file.clone()));
        }
        commands
    }

    /// A diff arrived; stale ones are dropped, good ones cached. The raw
    /// `FileDiff` rides alongside the drawable one: `hunk_patch` needs git's
    /// own bytes, and caching the two together is what stops them drifting.
    pub fn diff_arrived(
        &mut self,
        id: JobId,
        key: DiffKey,
        result: Result<(Arc<DiffView>, Arc<FileDiff>), String>,
    ) {
        if self.diff != Loading::Pending(id) {
            return;
        }
        match result {
            Ok((view, raw)) => {
                self.diff_cache
                    .insert(key, (Arc::clone(&view), Arc::clone(&raw)));
                self.diff = Loading::Ready(view);
                self.diff_raw = Some(raw);
                if let Some(kept) = self.kept_line.take() {
                    self.place_line(kept.at, kept.offset);
                }
            }
            Err(failure) => {
                self.kept_line = None;
                self.diff = Loading::Failed(failure);
                self.diff_raw = None;
            }
        }
    }

    /// Which changes-pane group the files cursor is in; `None` off the
    /// changes pane, or before its list has arrived.
    #[must_use]
    pub fn selected_section(&self) -> Option<WorkingTreeSection> {
        match &self.files {
            Loading::Ready(files) => files.selected_section(),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        }
    }

    /// The diff region's line cursor, clamped to the diff on show, or `None`
    /// when there is no diff to put a cursor in. Every spec has it
    /// (m7-power-tools/20b): blame's `B` needs "the line he's on" in a
    /// commit's diff as much as in the changes pane. Only the changes pane
    /// writes from it (`update::operate`'s readers check the spec).
    #[must_use]
    pub fn body_cursor(&self) -> Option<usize> {
        let Loading::Ready(diff) = &self.diff else {
            return None;
        };
        let last = diff.body_len().checked_sub(1)?;
        Some(self.body_cursor.min(last))
    }

    /// `j`/`k`/`PgUp`/`PgDn`/`g`/`G` in the diff region: move the line cursor
    /// by `delta` rows, clamped, and scroll `body_top` just enough to keep it
    /// on screen — the cursor leads, the view follows (design note, "Changes
    /// pane"). `rows` is the body region's height, `0` when it is not known
    /// yet, which leaves the old scroll-by-line behaviour intact.
    pub fn move_body_cursor(&mut self, delta: isize, content_len: usize, rows: usize) {
        let last = content_len.saturating_sub(1);
        self.body_cursor = self
            .body_cursor
            .min(last)
            .saturating_add_signed(delta)
            .min(last);
        if rows == 0 {
            self.scroll_body(delta, content_len);
            return;
        }
        if self.body_cursor < self.body_top {
            self.body_top = self.body_cursor;
        } else if self.body_cursor >= self.body_top + rows {
            self.body_top = self.body_cursor.saturating_sub(rows - 1);
        }
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
    ///
    /// The line cursor stays on the same side and line number of the file
    /// (m7-power-tools/20b): the rows move under new options, the line does
    /// not.
    pub fn set_options(
        &mut self,
        options: DiffOptions,
        next_id: &mut impl FnMut() -> JobId,
    ) -> Vec<Command> {
        let kept = self.line_to_keep();
        self.options = options;
        let commands = self.show_cursor(next_id);
        self.keep_line(kept);
        commands
    }

    /// `L`: read the whole of the current file, past [`MAX_LINES`], the line
    /// cursor kept on its line as [`Self::set_options`] keeps it.
    pub fn load_all(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let kept = self.line_to_keep();
        self.max_lines = usize::MAX;
        let commands = self.show_cursor(next_id);
        self.keep_line(kept);
        commands
    }

    /// The line under the line cursor and how far down the view it is, for
    /// a re-read of the same file to put back.
    fn line_to_keep(&self) -> Option<KeptLine> {
        let Loading::Ready(view) = &self.diff else {
            return None;
        };
        let cursor = self.body_cursor()?;
        Some(KeptLine {
            at: row_line(view, cursor, DiffMode::Unified)?,
            offset: cursor.saturating_sub(self.body_top),
        })
    }

    /// Put `kept` back now if the diff is on hand (a cache hit), else when it
    /// arrives (`diff_arrived`).
    fn keep_line(&mut self, kept: Option<KeptLine>) {
        match (kept, &self.diff) {
            (Some(kept), Loading::Ready(_)) => self.place_line(kept.at, kept.offset),
            (kept, _) => self.kept_line = kept,
        }
    }

    /// Put the line cursor on `at` (or the nearest row before it on that
    /// side), `offset` rows below the top of the view. Nothing without a
    /// diff on show.
    pub(super) fn place_line(&mut self, at: LineRef, offset: usize) {
        let Loading::Ready(view) = &self.diff else {
            return;
        };
        let Some(row) = line_row(view, at) else {
            return;
        };
        self.body_cursor = row;
        self.body_top = row.saturating_sub(offset);
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

/// Every visible file row of `section`, as `(row index, path)`, in row order.
///
/// `None` for `section` means "a list with no sections at all", which is
/// every list but the changes pane's — there it is the whole list.
fn section_files<'a>(
    files: &FileListView,
    list: &'a FileList,
    section: Option<WorkingTreeSection>,
) -> Vec<(usize, &'a [u8])> {
    files
        .visible()
        .filter(|(row, _)| files.section_at(*row) == section)
        .filter_map(|(row, view)| {
            let entry = usize::try_from(view.entry?).ok()?;
            Some((row, list.files.get(entry)?.path.as_slice()))
        })
        .collect()
}

/// Put the changes-pane cursor back where `kept` says it was, returning the
/// list to install and whether the cursor landed on the same file.
///
/// Same path in the same group wins. Failing that the cursor stays in that
/// group, at the same place down it — the file was just staged, discarded or
/// deleted, and the user is still working in that group. Only when the group
/// itself has gone does the fresh list's own cursor stand (m3/04 review M7).
fn put_cursor_back(
    files: Arc<FileListView>,
    list: &FileList,
    kept: Option<&KeptCursor>,
) -> (Arc<FileListView>, bool) {
    let Some(kept) = kept else {
        return (files, false);
    };
    let candidates = section_files(&files, list, kept.section);
    if candidates.is_empty() {
        return (files, false);
    }
    let (row, same_file) = match candidates
        .iter()
        .find(|(_, path)| *path == kept.path.as_slice())
    {
        Some((row, _)) => (*row, true),
        None => (candidates[kept.position.min(candidates.len() - 1)].0, false),
    };
    if row == files.cursor {
        return (files, same_file);
    }
    let mut owned = Arc::unwrap_or_clone(files);
    owned.cursor = row;
    (Arc::new(owned), same_file)
}

/// `path` as git spells it in a file list.
#[cfg(unix)]
fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

/// `path` as git spells it in a file list.
#[cfg(not(unix))]
fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    path.to_string_lossy().replace('\\', "/").into_bytes()
}
