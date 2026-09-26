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
use graft_term::sanitize::sanitize_path_bytes;
use graft_ui::detail::DetailFocus;

use crate::update::events::Command;
use crate::update::state::{AppState, Status};

use super::cursor::{LineRef, heading_row, line_row};
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
pub fn show_line(
    state: &mut AppState,
    spec: DetailSpec,
    path: &Path,
    at: Option<LineRef>,
) -> Vec<Command> {
    let mut commands = Vec::new();
    state.line_landing = None;
    // A pinned base turns the pane into a compare the moment the cursor
    // reaches `c`, so a commit's own diff can't open while one is pinned
    // (20's pinned-base ruling). Say so; move nothing.
    if let (DetailSpec::Commit(commit), Some(base)) = (spec, state.compare_base) {
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

/// Move the pending landing on as far as the state now allows. Called after
/// every event (`update::settle`) and by [`show_line`] itself.
pub(crate) fn advance(state: &mut AppState, commands: &mut Vec<Command>) {
    while let Some(landing) = state.line_landing.clone() {
        let next = match landing.stage {
            LandingStage::Seeking(commit) => sought(state, commit),
            LandingStage::Opening => Some(open(state, commands, landing.spec)),
            LandingStage::Selecting => select(state, commands, &landing),
            LandingStage::Placing => {
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

/// `Selecting`: once the list is in, the cursor on `path`'s row; a path the
/// spec does not touch leaves the pane as it opened and says so.
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
    let row = files.visible().find_map(|(row, view)| {
        let entry = usize::try_from(view.entry?).ok()?;
        (list.files.get(entry)?.path == wanted).then_some(row)
    });
    let Some(row) = row else {
        let words = match landing.spec {
            DetailSpec::Commit(commit) => format!("{}'s files", short(commit)),
            DetailSpec::Compare { base, head } => {
                format!("the files of {}..{}", short(base), short(head))
            }
            DetailSpec::WorkingTree => "the uncommitted changes".to_owned(),
        };
        // A path is the repository's bytes: sanitised before it is words.
        let path = sanitize_path_bytes(&wanted);
        state.status = Status::sticky(format!("{} isn't among {words}", path.as_str()));
        return None;
    };
    let AppState {
        detail, local_jobs, ..
    } = state;
    commands.extend(detail.select_file_row(row, &mut || local_jobs.next_id()));
    Some(LandingStage::Placing)
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
