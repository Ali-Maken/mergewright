//! Places (m7-power-tools/20; design note pass 2b, hard choice 4): the
//! blame screen `B` opens over the graph and the pane, and the trail of
//! them `^` stacks up, so `Esc` walks back the way he came.
//!
//! A shown place is a **modal pane** (keymap §2.1): it takes every key but
//! the live globals, before the menu, undo and the panes
//! (`update::keys::key_event`). `⏎` hides it and lands on the commit; the
//! trail stays, and one `Esc` from the graph or the pane shows it again.
//! Popping the last place puts him back where `B` was pressed
//! ([`Origin`]). Any write ends the trail: the lines it points at may have
//! moved ([`is_write`]).
//!
//! A pinned compare base is **set aside** while the trail lives (20's
//! pinned-base ruling): moved into [`Origin::base`] when `B` opens a place,
//! pinned again when the trail ends.

mod from;
mod keys;
mod rows;
mod write;

#[cfg(test)]
pub(crate) mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::blame::{Blame, BlameOptions, Rev};
use graft_term::sanitize::sanitize_path;
use graft_ui::blame::{BlameRev, BlameRowView};
use graft_ui::detail::DetailFocus;

use super::detail::{CompareBase, DetailSpec, LineRef, Loading, diff_cursor, show_line};
use super::events::Command;
use super::state::{AppState, Status};

pub use from::{BlameFrom, NoBlame, blame_from, came_from};
pub use keys::place_key;
pub(crate) use keys::{live_global, place_mouse};
pub use write::is_write;

/// How many places the trail holds; a 17th drops the oldest.
pub const TRAIL_MAX: usize = 16;

/// One screen on the trail. History-place adds `History`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// Who last changed each line of a file.
    Blame(Box<BlameState>),
}

/// The places he has opened since `B`, newest last (`AppState::trail`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trail {
    /// Newest last; empty when there is no trail.
    pub places: Vec<Place>,
    /// The newest place is on screen. False after `⏎` landed on a commit:
    /// the trail waits under the graph for `Esc`.
    pub shown: bool,
    /// Where `B` was pressed; `None` with no trail.
    pub origin: Option<Origin>,
}

impl Trail {
    /// There is a trail to go back along.
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.places.is_empty()
    }

    /// The trail is live but under the graph (`esc back to blame`).
    #[must_use]
    pub fn hidden(&self) -> bool {
        self.is_live() && !self.shown
    }

    /// The place on screen, if one is.
    #[must_use]
    pub fn on_screen(&self) -> Option<&BlameState> {
        if !self.shown {
            return None;
        }
        self.top()
    }

    /// The newest place.
    #[must_use]
    pub fn top(&self) -> Option<&BlameState> {
        match self.places.last()? {
            Place::Blame(blame) => Some(blame),
        }
    }

    fn top_mut(&mut self) -> Option<&mut BlameState> {
        match self.places.last_mut()? {
            Place::Blame(blame) => Some(blame),
        }
    }
}

/// Where `B` was pressed: what leaving the trail puts back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    /// The pane's spec.
    pub spec: DetailSpec,
    /// The file selected.
    pub path: PathBuf,
    /// The line under the cursor.
    pub at: Option<LineRef>,
    /// The pane's focus.
    pub focus: DetailFocus,
    /// The compare base pinned when `B` was pressed, set aside while the
    /// trail lives.
    pub base: Option<CompareBase>,
    /// Everything that moves when he goes somewhere else: equal on leaving
    /// means nothing moved, and only the trail is dropped.
    spot: Spot,
}

/// The graph cursor, the pane's spec, file, focus and line cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Spot {
    graph: Option<usize>,
    on_working_tree: bool,
    shown: bool,
    spec: Option<DetailSpec>,
    file: Option<usize>,
    focus: DetailFocus,
    body_cursor: usize,
}

impl Spot {
    fn of(state: &AppState) -> Self {
        Self {
            graph: state.history.as_ref().map(|history| history.cursor),
            on_working_tree: state.on_working_tree,
            shown: state.detail.shown,
            spec: state.detail.spec,
            file: match &state.detail.files {
                Loading::Ready(files) => Some(files.cursor),
                Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
            },
            focus: state.detail.focus,
            body_cursor: state.detail.body_cursor,
        }
    }
}

/// One blame place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameState {
    /// The file blamed, in the version blamed.
    pub path: PathBuf,
    /// The version blamed.
    pub rev: Rev,
    /// The title's last words.
    pub title: BlameRev,
    /// `w` and `i`.
    pub options: BlameOptions,
    /// Reading, failed, or the lines.
    pub body: BlameLoad,
    /// Index of the cursor row.
    pub cursor: usize,
    /// Index of the first row drawn.
    pub top: usize,
    /// The active find, as typed.
    pub find: Option<String>,
    /// The `/` prompt while it is open.
    pub find_prompt: Option<String>,
    /// `space`: the gutter coloured by age.
    pub heatmap: bool,
    /// The last click's row and time (ms), for a double-click.
    pub last_click: Option<(usize, i64)>,
}

/// A blame place's body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameLoad {
    /// The job is running.
    Reading {
        /// The job; `Esc` cancels it.
        job: JobId,
        /// The line the cursor goes to when it answers.
        want: Want,
    },
    /// git failed; its words.
    Failed(String),
    /// The blame, and its rows as the picture draws them.
    Ready {
        /// What git said.
        blame: Arc<Blame>,
        /// One row per line.
        rows: Arc<[BlameRowView]>,
    },
}

/// Where the cursor goes when a blame arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    /// This final line (clamped).
    Line(u32),
    /// The first line not committed yet (the first line when none is).
    FirstUncommitted,
    /// Where the job's `came_from` says (`^`).
    CameFrom,
}

/// What a blame job reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameRequest {
    /// The version.
    pub rev: Rev,
    /// The file.
    pub path: PathBuf,
    /// `-w`, the ignore list.
    pub options: BlameOptions,
    /// For `^`: also read `commit`'s diff of the file, and answer where
    /// `line` came from ([`came_from`]).
    pub came_from: Option<CameFromRequest>,
}

/// `^`'s second read: the diff that made the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameFromRequest {
    /// The commit that last changed the line.
    pub commit: Oid,
    /// The file's path in it.
    pub path: PathBuf,
    /// The file's path in its parent.
    pub old_path: PathBuf,
    /// The line, counted in `commit`'s version.
    pub line: u32,
}

/// A blame job's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameAnswer {
    /// The blame.
    pub blame: Blame,
    /// For `^`: the parent's line the cursor goes to.
    pub line: Option<u32>,
}

/// Why a blame job gave no blame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameFailure {
    /// Not in the version asked for.
    Untracked,
    /// A binary file.
    Binary,
    /// git's words.
    Failed(String),
}

/// `B` in the detail pane: open blame at the line he is on, or say why not.
/// Claimed on a file row, a heading or a diff line, either region — not
/// while the commit-graph offer can be acted on (keymap §2.1). `true` when
/// `B` was taken.
pub(crate) fn b_key(next: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !claims_b(next) {
        return false;
    }
    let Some(cursor) = diff_cursor(next) else {
        return false;
    };
    let head = next.refs.as_ref().and_then(|refs| refs.head());
    let both = cursor.spec == DetailSpec::WorkingTree && staged_and_unstaged(next, &cursor.path);
    let from = match blame_from(&cursor, head, both) {
        Ok(from) => from,
        Err(why) => {
            next.status = Status::sticky(no_blame_words(&why));
            return true;
        }
    };
    if let BlameFrom::WorkingTree { line: None, .. } = from {
        next.status = Status::Waiting(
            "staged and unstaged edits both move this file's lines · on its first uncommitted line"
                .to_owned(),
        );
    }
    // `B` starts a new trail: whatever was on an old one is behind him now.
    // A base still set aside by it stays set aside, for this trail to put
    // back.
    let old = std::mem::take(&mut next.trail);
    cancel_reading(&old, commands);
    let pinned = next.compare_base.take();
    let base = pinned.or(old.origin.and_then(|origin| origin.base));
    next.trail.origin = Some(Origin {
        spec: cursor.spec,
        path: cursor.path.clone(),
        at: cursor.at,
        focus: next.detail.focus,
        base,
        spot: Spot::of(next),
    });
    let opened = open_blame(next, from);
    if !next.trail.is_live() {
        // Refused before any place opened: nothing is set aside.
        next.trail = Trail::default();
        next.compare_base = base;
    }
    commands.extend(opened);
    true
}

/// The pane would take `B`: shown, focused, and not under the commit-graph
/// offer. [`b_key`]'s own guard, read by the bar (`B blame`), so the two
/// cannot disagree.
fn claims_b(state: &AppState) -> bool {
    state.detail.shown
        && state.detail.focus != DetailFocus::Graph
        && !state.commit_graph_actionable()
}

/// `B` would answer here: the bar's `B blame` (`view::rows::hint_context`).
#[must_use]
pub fn blamable(state: &AppState) -> bool {
    claims_b(state) && diff_cursor(state).is_some()
}

/// The file has edits both staged and unstaged.
fn staged_and_unstaged(state: &AppState, path: &Path) -> bool {
    state.working_tree.as_ref().is_some_and(|status| {
        status
            .entries
            .iter()
            .any(|entry| entry.path == path && entry.staged.is_some() && entry.unstaged.is_some())
    })
}

/// The status words for a refusal (B1).
fn no_blame_words(why: &NoBlame) -> String {
    match why {
        NoBlame::Untracked(path) => {
            format!(
                "{} isn't committed yet · no blame",
                sanitize_path(path).as_str()
            )
        }
        NoBlame::Binary(path) => format!("{} is binary · no blame", sanitize_path(path).as_str()),
        NoBlame::DiffNotLoaded => "wait for the diff to load · then B".to_owned(),
    }
}

/// Open a blame place for `from` on the trail and start its job. Pushed on
/// top: the caller (`B`) starts a fresh trail around it.
pub fn open_blame(state: &mut AppState, from: BlameFrom) -> Vec<Command> {
    let (rev, path, title, want) = match from {
        BlameFrom::Commit { commit, path, line } => (
            Rev::Commit(commit),
            path,
            BlameRev::AsOf(short(commit)),
            Want::Line(line),
        ),
        BlameFrom::Parent { commit, path, line } => {
            let Some(parent) = first_parent(state, commit) else {
                state.status = Status::sticky(format!(
                    "{} has no parent here · nothing before it",
                    short(commit)
                ));
                return Vec::new();
            };
            (
                Rev::Commit(parent),
                path,
                BlameRev::Before(short(commit)),
                Want::Line(line),
            )
        }
        BlameFrom::WorkingTree { path, line } => (
            Rev::WorkingTree,
            path,
            BlameRev::Now,
            line.map_or(Want::FirstUncommitted, Want::Line),
        ),
    };
    open_place(state, rev, path, title, want, None)
}

/// Push a place reading `rev`'s `path` and ask for it.
fn open_place(
    state: &mut AppState,
    rev: Rev,
    path: PathBuf,
    title: BlameRev,
    want: Want,
    came_from: Option<CameFromRequest>,
) -> Vec<Command> {
    let job = state.local_jobs.next_id();
    let options = BlameOptions::default();
    let place = BlameState {
        path: path.clone(),
        rev,
        title,
        options,
        body: BlameLoad::Reading { job, want },
        cursor: 0,
        top: 0,
        find: None,
        find_prompt: None,
        heatmap: false,
        last_click: None,
    };
    if state.trail.places.len() >= TRAIL_MAX {
        state.trail.places.remove(0);
    }
    state.trail.places.push(Place::Blame(Box::new(place)));
    state.trail.shown = true;
    vec![Command::ReadBlame(
        job,
        Box::new(BlameRequest {
            rev,
            path,
            options,
            came_from,
        }),
    )]
}

/// `c`'s first parent, from the graph row it is on.
fn first_parent(state: &AppState, commit: Oid) -> Option<Oid> {
    let row = state.history.as_ref()?.selected()?;
    (row.commit.id == commit)
        .then(|| row.commit.parents.first().copied())
        .flatten()
}

/// Seven hex digits.
pub(crate) fn short(oid: Oid) -> String {
    let mut hex = oid.to_string();
    hex.truncate(7);
    hex
}

/// `Esc` back along the trail: drop the newest place (stopping its job if
/// it is still reading); popping the last one leaves the trail. `None` when
/// there is no trail to go back along.
///
/// Not the spec's `-> bool`: leaving the trail lands the pane back where
/// `B` was pressed, which is `show_line`'s commands.
pub fn trail_back(state: &mut AppState) -> Option<Vec<Command>> {
    let popped = state.trail.places.pop()?;
    let mut commands = Vec::new();
    if let Place::Blame(blame) = &popped
        && let BlameLoad::Reading { job, .. } = blame.body
    {
        commands.push(Command::CancelJob(job));
    }
    if state.trail.is_live() {
        state.trail.shown = true;
    } else {
        leave(state, &mut commands, true);
    }
    Some(commands)
}

/// End the trail: pin the set-aside base again and, with `restore`, put
/// the graph and the pane back where `B` was pressed if anything moved.
fn leave(state: &mut AppState, commands: &mut Vec<Command>, restore: bool) {
    let trail = std::mem::take(&mut state.trail);
    cancel_reading(&trail, commands);
    let Some(origin) = trail.origin else {
        return;
    };
    if let Some(base) = origin.base {
        state.compare_base = Some(base);
    }
    if !restore {
        return;
    }
    // A `⏎` landing still on its way (the walk seeking the commit) is the
    // trail's; he has left it, so it must not land later.
    state.line_landing = None;
    if Spot::of(state) == origin.spot {
        return;
    }
    commands.extend(show_line(state, origin.spec, &origin.path, origin.at));
    match state.line_landing.as_mut() {
        Some(landing) => landing.then_focus = Some(origin.focus),
        None => state.detail.focus = origin.focus,
    }
}

/// Stop every job still reading on `trail`.
fn cancel_reading(trail: &Trail, commands: &mut Vec<Command>) {
    for place in &trail.places {
        let Place::Blame(blame) = place;
        if let BlameLoad::Reading { job, .. } = blame.body {
            commands.push(Command::CancelJob(job));
        }
    }
}

/// After every event: a write among `commands` ends the trail and pins the
/// set-aside base again (20's pinned-base ruling). Nothing is restored: the
/// write lands where it lands.
pub(crate) fn after_commands(state: &mut AppState, commands: &mut Vec<Command>) {
    if state.trail.is_live() && commands.iter().any(is_write) {
        leave(state, commands, false);
    }
}

/// `Esc` with the trail hidden under the graph or the pane: show it again.
/// `false` with nothing hidden.
pub(crate) fn show_again(state: &mut AppState) -> bool {
    if !state.trail.hidden() {
        return false;
    }
    state.trail.shown = true;
    true
}

/// A new compare base pinned (`x`, `Shift`/`Ctrl`+click) while the trail
/// is hidden: a new task, so the trail ends and the set-aside base is
/// dropped (20's pinned-base ruling). Call before the pin; `true` when a
/// trail ended, for the caller to say so once the pin is known.
pub(crate) fn end_for_compare(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !state.trail.is_live() {
        return false;
    }
    if let Some(origin) = state.trail.origin.as_mut() {
        origin.base = None;
    }
    leave(state, commands, false);
    true
}

/// A blame job answered. A stale id (the place was popped, or `w`/`i`
/// asked again) is dropped.
pub(crate) fn blame_arrived(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: &Result<Arc<BlameAnswer>, BlameFailure>,
) {
    let Some(index) = state.trail.places.iter().position(|place| {
        let Place::Blame(blame) = place;
        matches!(blame.body, BlameLoad::Reading { job, .. } if job == id)
    }) else {
        return;
    };
    let Place::Blame(blame) = &mut state.trail.places[index];
    let BlameLoad::Reading { want, .. } = blame.body else {
        return;
    };
    match answer {
        Ok(answer) => {
            let rows: Arc<[BlameRowView]> = rows::blame_rows(&answer.blame).into();
            let target = match want {
                Want::Line(line) => rows::row_of_line(&answer.blame, line),
                Want::FirstUncommitted => answer
                    .blame
                    .lines
                    .iter()
                    .position(|line| line.commit.is_none())
                    .unwrap_or(0),
                Want::CameFrom => rows::row_of_line(&answer.blame, answer.line.unwrap_or(1)),
            };
            blame.cursor = target;
            blame.top = 0;
            blame.body = BlameLoad::Ready {
                blame: Arc::new(answer.blame.clone()),
                rows,
            };
            keys::keep_in_view(state);
        }
        Err(BlameFailure::Failed(words)) => blame.body = BlameLoad::Failed(words.clone()),
        Err(refused @ (BlameFailure::Untracked | BlameFailure::Binary)) => {
            let path = blame.path.clone();
            let words = no_blame_words(&match refused {
                BlameFailure::Binary => NoBlame::Binary(path),
                BlameFailure::Untracked | BlameFailure::Failed(_) => NoBlame::Untracked(path),
            });
            state.trail.places.remove(index);
            if state.trail.is_live() {
                state.trail.shown = true;
            } else {
                leave(state, commands, true);
            }
            state.status = Status::sticky(words);
        }
    }
}

/// While the place is on screen it owns the keys (keymap §2.1).
#[must_use]
pub(crate) fn owns_keys(state: &AppState) -> bool {
    state.trail.on_screen().is_some()
}

/// The place's `/` prompt is open: a text field.
#[must_use]
pub(crate) fn prompt_open(state: &AppState) -> bool {
    state
        .trail
        .on_screen()
        .is_some_and(|blame| blame.find_prompt.is_some())
}
