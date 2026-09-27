//! `show_line`: one call that lands the graph, the detail pane, a file and a
//! line of it together (m7-power-tools/20b) — blame's `⏎`, history's `⏎`
//! and the trail's last `Esc` all end here.
//!
//! Three reads may stand between the call and the landing: the graph's seek
//! (when the commit is off the rows on hand), the spec's file list, and the
//! file's diff. [`LineLanding`] is what is still owed while they run; it
//! lives on [`AppState::line_landing`] and moves one [`LandingStage`] at a
//! time in [`advance`], which `update::settle` calls after every event. One
//! landing at a time: a new `show_line` replaces it, and a key or click that
//! moves the graph cursor or the pane's selection drops it
//! ([`LandingWatch`]).

use std::path::{Path, PathBuf};

use graft_core::oid::Oid;
use graft_git::diff::{FileDiff, FileList, LineKind};
use graft_term::sanitize::sanitize_path_bytes;
use graft_ui::detail::{DetailFocus, FileListView, FileRowKind, WorkingTreeSection};

use crate::update::events::Command;
use crate::update::state::{AppState, Status};

use super::cursor::{LineRef, Side, heading_row, line_row};
use super::state::{DetailSpec, Loading};

/// A `show_line` still waiting on a read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineLanding {
    /// What the pane lands on.
    pub spec: DetailSpec,
    /// The file to select, new-side path.
    pub path: PathBuf,
    /// The line to put the cursor on; `None` means the file's first change.
    pub at: Option<LineRef>,
    /// `WorkingTree` only: the changes-pane section `at` counts in. `None`
    /// is the file on disk, placed by where the line lives (20m): a path
    /// with both a Staged and an Unstaged row is placed on the Unstaged
    /// diff, or — a line unchanged since the index — on the Staged one.
    pub section: Option<WorkingTreeSection>,
    /// How far it has got.
    pub stage: LandingStage,
    /// The focus to leave the pane in once landed; `None` is the diff.
    /// Leaving a blame trail sets it, to put back the focus `B` was
    /// pressed from (m7-power-tools/20).
    pub then_focus: Option<DetailFocus>,
}

/// Where a [`LineLanding`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandingStage {
    /// The graph is seeking this commit (`History::seek` asked the walk).
    Seeking(Oid),
    /// The graph is on the spec's row; the pane opens on the spec next.
    Opening,
    /// The pane is on the spec; waiting for its file list to select `path`.
    Selecting,
    /// `path` is selected; waiting for its diff to put the cursor on `at`.
    Placing,
}

/// Land the graph cursor on `spec`'s row, the detail pane on `spec` with
/// `path` selected, and — once the diff has arrived — the line cursor on
/// `at` with the diff focused. See the module note for the reads it waits on.
///
/// `Commit(c)` seeks `c`; `Compare { head, .. }` seeks `head`, a pinned base
/// staying pinned; `WorkingTree` selects the uncommitted-changes row.
///
/// `section` is for `WorkingTree` only (`None` for the others): which file
/// `at` counts in. `Some(s)` is the path's row in section `s`, `at` in that
/// row's own numbering (no such row any more → the path's first row, its
/// first change). `None` is the file on disk: a path with one row takes it
/// as given; a partly staged one is placed where the line lives — an
/// unstaged line on the Unstaged row, a line unchanged since the index on
/// the Staged row at its index number ([`disk_line_in`]).
pub fn show_line(
    state: &mut AppState,
    spec: DetailSpec,
    path: &Path,
    at: Option<LineRef>,
    section: Option<WorkingTreeSection>,
) -> Vec<Command> {
    debug_assert!(
        section.is_none() || spec == DetailSpec::WorkingTree,
        "a section is the changes pane's"
    );
    let mut commands = Vec::new();
    state.line_landing = None;
    // A pinned base turns the pane into a compare the moment the cursor
    // reaches `c`, so a commit's own diff can't open while one is pinned
    // (20's pinned-base ruling). Say so; move nothing. The base's own row
    // is the exception: `follow_selection` leaves it on its own diff.
    if let (DetailSpec::Commit(commit), Some(base)) = (spec, state.compare_base)
        && commit != base.oid
    {
        state.status = Status::sticky(format!(
            "can't show {} on its own while compare base {} is pinned · x on the base clears it",
            short(commit),
            short(base.oid)
        ));
        return commands;
    }
    let first = match spec {
        DetailSpec::WorkingTree => {
            if !state.uncommitted_row_shown() {
                state.status = Status::sticky("can't show the uncommitted changes: there are none");
                return commands;
            }
            state.on_working_tree = true;
            LandingStage::Opening
        }
        DetailSpec::Commit(commit) | DetailSpec::Compare { head: commit, .. } => {
            match seek(state, &mut commands, commit) {
                Ok(reached) => reached,
                Err(words) => {
                    cannot_show(state, commit, words);
                    return commands;
                }
            }
        }
    };
    state.line_landing = Some(LineLanding {
        spec,
        path: path.to_path_buf(),
        at,
        section,
        stage: first,
        then_focus: None,
    });
    advance(state, &mut commands);
    commands
}

/// Ask the graph for `commit`: on it already, or waiting for the walk.
/// `Err` carries the seek's own words for why it cannot.
fn seek(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    commit: Oid,
) -> Result<LandingStage, &'static str> {
    let height = state.graph_rows();
    let Some(history) = state.history.as_mut() else {
        return Err("no history has loaded");
    };
    let request = history.seek(commit, height);
    if let (Some(request), Some(walk)) = (request, state.awaiting) {
        commands.push(Command::ShowRows(walk, request));
    }
    if history.seeking == Some(commit) {
        return Ok(LandingStage::Seeking(commit));
    }
    if history.selected().map(|row| row.commit.id) == Some(commit) {
        state.on_working_tree = false;
        return Ok(LandingStage::Opening);
    }
    Err("the history walk has stopped")
}

/// Drop the pending landing, and with it the graph's seek it started: the
/// walk's answer would otherwise still put the cursor on the commit
/// (`History::seek` moves it itself), and the pane would follow. The
/// withdrawn find is replaced by a request for the rows where the viewport
/// is, as `Esc`'s `G` rung does (`update::keys::escape_key`).
pub(crate) fn abandon(state: &mut AppState, commands: &mut Vec<Command>) {
    let Some(landing) = state.line_landing.take() else {
        return;
    };
    let LandingStage::Seeking(commit) = landing.stage else {
        return;
    };
    let height = state.graph_rows();
    if let Some(walk) = state.awaiting
        && let Some(history) = state.history.as_mut()
        && history.seeking == Some(commit)
        && let Some(request) = history.withdraw_jump(height)
    {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// Move the pending landing on as far as the state now allows. Called after
/// every event (`update::settle`) and by [`show_line`] itself.
pub(crate) fn advance(state: &mut AppState, commands: &mut Vec<Command>) {
    while let Some(landing) = state.line_landing.clone() {
        let next = match landing.stage {
            LandingStage::Seeking(commit) => sought(state, commit),
            LandingStage::Opening => Some(open(state, commands, landing.spec)),
            LandingStage::Selecting => select(state, commands, &landing),
            LandingStage::Placing => {
                if let Some(staged) = index_line(state, &landing) {
                    // A disk line unchanged since the index: on to the
                    // Staged row, at its index number.
                    if let Some(pending) = state.line_landing.as_mut() {
                        pending.section = Some(WorkingTreeSection::Staged);
                        pending.at = Some(staged);
                        pending.stage = LandingStage::Selecting;
                    }
                    continue;
                }
                place(state, landing.spec, landing.at);
                if state.line_landing.is_none()
                    && let Some(focus) = landing.then_focus
                {
                    state.detail.focus = focus;
                }
                return;
            }
        };
        match next {
            Some(same) if same == landing.stage => return,
            Some(moved_on) => {
                if let Some(pending) = state.line_landing.as_mut() {
                    pending.stage = moved_on;
                }
            }
            None => state.line_landing = None,
        }
    }
}

/// `Seeking`: still waiting, reached, or missed (said, and dropped).
fn sought(state: &mut AppState, commit: Oid) -> Option<LandingStage> {
    let history = state.history.as_ref()?;
    if history.seeking == Some(commit) {
        return Some(LandingStage::Seeking(commit));
    }
    if history.selected().map(|row| row.commit.id) == Some(commit) {
        state.on_working_tree = false;
        return Some(LandingStage::Opening);
    }
    // The walk answered and the commit is not in this view (the words the
    // search uses for the same miss).
    cannot_show(state, commit, "not in this view");
    None
}

/// `Opening`: the pane on the spec, the file list focused while it loads.
fn open(state: &mut AppState, commands: &mut Vec<Command>, spec: DetailSpec) -> LandingStage {
    let AppState {
        detail, local_jobs, ..
    } = state;
    commands.extend(detail.open(spec, &mut || local_jobs.next_id()));
    LandingStage::Selecting
}

/// `Selecting`: once the list is in, the cursor on `path`'s row — unfolding
/// any folded directory it is inside (20m B) — and, on the changes pane, on
/// the row [`LineLanding::section`] names. A path the spec does not touch
/// leaves the pane as it opened and says so.
fn select(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    landing: &LineLanding,
) -> Option<LandingStage> {
    let detail = &state.detail;
    if !detail.shown || detail.spec != Some(landing.spec) {
        return None;
    }
    let files = match &detail.files {
        Loading::Pending(_) => return Some(LandingStage::Selecting),
        Loading::Ready(files) => files,
        Loading::Idle | Loading::Failed(_) => return None,
    };
    let list = detail.list.as_ref()?;
    let wanted = path_bytes(&landing.path);
    // Every row, folded or not, with the changes-pane section it is in.
    let rows: Vec<(usize, Option<WorkingTreeSection>)> = (0..files.rows.len())
        .filter(|&row| row_is(files, list, row, &wanted))
        .map(|row| (row, files.section_at(row)))
        .collect();
    let in_section = |wanted: WorkingTreeSection| {
        rows.iter()
            .find_map(|&(row, section)| (section == Some(wanted)).then_some(row))
    };
    let row = rows.first().map(|&(first, _)| match landing.section {
        Some(section) => in_section(section).ok_or(first),
        // On disk: a partly staged path is read on its Unstaged row first
        // (`index_line` moves it on to the Staged row when the line is not
        // an unstaged one). An Unmerged row, or a path with one row, is
        // its first.
        None => match (
            in_section(WorkingTreeSection::Unmerged),
            in_section(WorkingTreeSection::Staged),
            in_section(WorkingTreeSection::Unstaged),
        ) {
            (None, Some(_), Some(unstaged)) => Ok(unstaged),
            _ => Ok(first),
        },
    });
    let row = match row {
        Some(Ok(row)) => row,
        Some(Err(first)) => {
            // No row in the section asked for: the tree changed from
            // outside since. The path's first row, at its first change.
            if let Some(pending) = state.line_landing.as_mut() {
                pending.at = None;
            }
            first
        }
        None => {
            say_missing(state, landing.spec, &wanted);
            return None;
        }
    };
    let AppState {
        detail, local_jobs, ..
    } = state;
    if let Loading::Ready(files) = &mut detail.files {
        unfold_to(std::sync::Arc::make_mut(files), row);
    }
    commands.extend(detail.select_file_row(row, &mut || local_jobs.next_id()));
    Some(LandingStage::Placing)
}

/// Row `row` of `files` is the file `wanted`.
fn row_is(files: &FileListView, list: &FileList, row: usize, wanted: &[u8]) -> bool {
    files
        .rows
        .get(row)
        .and_then(|found| found.entry)
        .and_then(|entry| usize::try_from(entry).ok())
        .and_then(|entry| list.files.get(entry))
        .is_some_and(|file| file.path == wanted)
}

/// Unfold every folded directory `row` sits inside, with the pane's own
/// unfold (`FileListView::toggle_dir`), so `row` is visible.
fn unfold_to(files: &mut FileListView, row: usize) {
    let Some(mut depth) = files.rows.get(row).map(|found| found.depth) else {
        return;
    };
    for above in (0..row).rev() {
        let Some(found) = files.rows.get(above) else {
            continue;
        };
        if found.depth >= depth {
            continue;
        }
        depth = found.depth;
        if matches!(found.kind, FileRowKind::Dir { collapsed: true }) {
            files.toggle_dir(above);
        }
    }
}

/// On a disk landing (`WorkingTree`, no section) with the Unstaged row of a
/// partly staged path selected and its diff in: the line's number on the
/// Staged diff when it is not an unstaged line. `None` otherwise — the
/// landing places as it stands.
fn index_line(state: &AppState, landing: &LineLanding) -> Option<LineRef> {
    let detail = &state.detail;
    if landing.spec != DetailSpec::WorkingTree
        || landing.section.is_some()
        || detail.spec != Some(DetailSpec::WorkingTree)
        || detail.selected_section() != Some(WorkingTreeSection::Unstaged)
        || !matches!(detail.diff, Loading::Ready(_))
    {
        return None;
    }
    let at = landing.at.filter(|at| at.side == Side::New)?;
    let (Loading::Ready(files), Some(list)) = (&detail.files, detail.list.as_ref()) else {
        return None;
    };
    let wanted = path_bytes(&landing.path);
    let staged_too = (0..files.rows.len()).any(|row| {
        files.section_at(row) == Some(WorkingTreeSection::Staged)
            && row_is(files, list, row, &wanted)
    });
    if !staged_too {
        return None;
    }
    match disk_line_in(detail.diff_raw.as_ref()?, at.line) {
        DiskLine::Unstaged(_) => None,
        DiskLine::Index(line) => Some(LineRef {
            side: Side::New,
            line,
        }),
    }
}

/// Where a line of the file on disk lives, read off the Unstaged diff
/// (index → disk) of a partly staged file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiskLine {
    /// An uncommitted, unstaged line: this `New` number on the Unstaged diff.
    Unstaged(u32),
    /// Unchanged since the index: this `New` number on the Staged diff.
    Index(u32),
}

/// Disk line `disk_line` against `unstaged`, the file's index → disk diff:
/// an added line is `Unstaged` at its own number; a context line is `Index`
/// at its old number; a line outside every hunk is `Index`, shifted back by
/// what the hunks above it added or took away. A diff cut short, the line
/// below what it shows, is taken as `Unstaged` — it cannot be told — and
/// the landing puts the cursor on the nearest row, as `line_row` does.
pub(crate) fn disk_line_in(unstaged: &FileDiff, disk_line: u32) -> DiskLine {
    let mut shown_to = 0;
    for line in unstaged.hunks.iter().flat_map(|hunk| &hunk.lines) {
        let Some(new_no) = line.new_no else {
            continue;
        };
        shown_to = shown_to.max(new_no);
        if new_no == disk_line {
            return match (line.kind, line.old_no) {
                (LineKind::Context, Some(old_no)) => DiskLine::Index(old_no),
                _ => DiskLine::Unstaged(disk_line),
            };
        }
    }
    if unstaged.truncated.is_some() && disk_line > shown_to {
        return DiskLine::Unstaged(disk_line);
    }
    // The hunks wholly above the line. git writes a hunk with no new lines
    // (a pure deletion) at the line *before* it, so that line is its end.
    let shift: i64 = unstaged
        .hunks
        .iter()
        .filter(|hunk| {
            let end = if hunk.new_lines == 0 {
                hunk.new_start
            } else {
                hunk.new_start + hunk.new_lines - 1
            };
            end < disk_line
        })
        .map(|hunk| i64::from(hunk.new_lines) - i64::from(hunk.old_lines))
        .sum();
    let line = i64::from(disk_line) - shift;
    // Positive for any diff git writes: the shift counts only hunks above.
    DiskLine::Index(u32::try_from(line).unwrap_or(1))
}

/// Say `wanted` is not among `spec`'s files; the pane stays as it opened.
fn say_missing(state: &mut AppState, spec: DetailSpec, wanted: &[u8]) {
    let words = match spec {
        DetailSpec::Commit(commit) => format!("{}'s files", short(commit)),
        DetailSpec::Compare { base, head } => {
            format!("the files of {}..{}", short(base), short(head))
        }
        DetailSpec::WorkingTree => "the uncommitted changes".to_owned(),
    };
    // A path is the repository's bytes: sanitised before it is words.
    let path = sanitize_path_bytes(wanted);
    state.status = Status::sticky(format!("{} isn't among {words}", path.as_str()));
}

/// `Placing`: once the diff is in, the cursor on `at` with the diff focused
/// and the view scrolled to show it. Always ends the landing, unless the
/// diff is still on its way.
fn place(state: &mut AppState, spec: DetailSpec, at: Option<LineRef>) {
    let detail = &state.detail;
    if !detail.shown || detail.spec != Some(spec) {
        state.line_landing = None;
        return;
    }
    let view = match &detail.diff {
        Loading::Pending(_) => return,
        Loading::Ready(view) => std::sync::Arc::clone(view),
        Loading::Idle | Loading::Failed(_) => {
            state.line_landing = None;
            return;
        }
    };
    state.line_landing = None;
    let at = at.or_else(|| view.hunks.iter().find_map(super::cursor::first_change));
    let Some(row) = at.and_then(|at| line_row(&view, at)) else {
        state.detail.focus = DetailFocus::Diff;
        return;
    };
    state.detail.focus = DetailFocus::Diff;
    // Read after the focus moves: below 100 columns the focus picks the tab.
    let rows = super::super::geometry::diff_body_rows(state);
    let heading = heading_row(&view, row);
    state.detail.body_cursor = row;
    state.detail.body_top = if rows == 0 || row - heading < rows {
        heading
    } else {
        row - rows / 3
    };
}

/// Say the graph cannot reach `commit`, in the seek's words; nothing moves.
fn cannot_show(state: &mut AppState, commit: Oid, words: &str) {
    state.status = Status::sticky(format!(
        "can't show {} on the graph: {words}",
        short(commit)
    ));
}

/// Seven hex digits, as the design note writes them.
fn short(oid: Oid) -> String {
    let mut hex = oid.to_string();
    hex.truncate(7);
    hex
}

/// What a key or click may move out from under a landing: the graph cursor
/// (or its seek), the uncommitted-changes row, and the pane's selection.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Selection {
    graph: Option<(usize, Option<Oid>)>,
    on_working_tree: bool,
    shown: bool,
    spec: Option<DetailSpec>,
    file: Option<usize>,
}

impl Selection {
    fn of(state: &AppState) -> Self {
        Self {
            graph: state
                .history
                .as_ref()
                .map(|history| (history.cursor, history.seeking)),
            on_working_tree: state.on_working_tree,
            shown: state.detail.shown,
            spec: state.detail.spec,
            file: match &state.detail.files {
                Loading::Ready(files) => Some(files.cursor),
                Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
            },
        }
    }
}

/// The landing and the [`Selection`] before a key or click is answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LandingWatch {
    landing: Option<LineLanding>,
    selection: Selection,
}

impl LandingWatch {
    /// Take note before a key or click is answered.
    pub(crate) fn of(state: &AppState) -> Self {
        Self {
            landing: state.line_landing.clone(),
            selection: Selection::of(state),
        }
    }

    /// After it: the same landing still pending, and something it was
    /// moving has moved — the user went elsewhere, so it is dropped. A
    /// landing the key itself started (a new `show_line`) is not.
    pub(crate) fn drop_if_moved(self, state: &mut AppState) {
        if self.landing.is_some()
            && state.line_landing == self.landing
            && Selection::of(state) != self.selection
        {
            state.line_landing = None;
        }
    }
}

/// `path` as git spells it in a file list.
#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

/// `path` as git spells it in a file list.
#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().replace('\\', "/").into_bytes()
}

#[cfg(test)]
mod tests;
