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

pub(crate) mod answer;
mod blame_state;
mod from;
mod history;
mod history_answer;
mod history_keys;
mod keys;
mod rows;
mod trail;
mod write;

#[cfg(test)]
pub(crate) mod tests;

use std::path::{Path, PathBuf};

use graft_core::oid::Oid;
use graft_git::blame::{BlameOptions, Rev};
use graft_term::sanitize::sanitize_path;
use graft_ui::blame::BlameRev;
use graft_ui::detail::{DetailFocus, WorkingTreeSection};

use super::detail::{CompareBase, DetailSpec, DiffCursor, LineRef, Loading, diff_cursor};
use super::events::Command;
use super::state::{AppState, Status};

pub(crate) use answer::blame_arrived;
pub use answer::{BlameAnswer, BlameFailure, BlameLoad, BlameRequest, CameFromRequest, Want};
pub use from::{BlameFrom, NoBlame, blame_from, came_from};
pub(crate) use history::{f_key, open_file_history};
pub use history::{FileLoad, HistoryFrom, HistoryState, TraceLoad, open_history};
pub use history_answer::{FUNCTION_MAX, TRACE_CAP, TraceAnswer, TraceFailure, TraceRequest};
pub(crate) use history_answer::{trace_arrived, whole_file_arrived};
pub use keys::place_key;
pub(crate) use keys::{live_global, place_mouse};
pub use trail::trail_back;
pub(crate) use trail::{after_commands, end_for_compare, esc_goes_back, show_again};
use trail::{cancel_reading, leave};
pub use write::is_write;

/// How many places the trail holds; a 17th drops the oldest.
pub const TRAIL_MAX: usize = 16;

/// One screen on the trail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// Who last changed each line of a file.
    Blame(Box<BlameState>),
    /// The commits that changed some lines, or a file (`L`, `F`).
    History(Box<HistoryState>),
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

    /// The blame place on screen, if one is.
    #[must_use]
    pub fn on_screen(&self) -> Option<&BlameState> {
        if !self.shown {
            return None;
        }
        self.top()
    }

    /// The newest place, if it is a blame.
    #[must_use]
    pub fn top(&self) -> Option<&BlameState> {
        match self.places.last()? {
            Place::Blame(blame) => Some(blame),
            Place::History(_) => None,
        }
    }

    fn top_mut(&mut self) -> Option<&mut BlameState> {
        match self.places.last_mut()? {
            Place::Blame(blame) => Some(blame),
            Place::History(_) => None,
        }
    }

    /// The history place on screen, if one is.
    #[must_use]
    pub fn history_on_screen(&self) -> Option<&HistoryState> {
        if !self.shown {
            return None;
        }
        self.top_history()
    }

    /// The newest place, if it is a history.
    #[must_use]
    pub fn top_history(&self) -> Option<&HistoryState> {
        match self.places.last()? {
            Place::History(history) => Some(history),
            Place::Blame(_) => None,
        }
    }

    fn top_history_mut(&mut self) -> Option<&mut HistoryState> {
        match self.places.last_mut()? {
            Place::History(history) => Some(history),
            Place::Blame(_) => None,
        }
    }

    /// Some place — blame or history — is on screen.
    #[must_use]
    pub fn place_shown(&self) -> bool {
        self.shown && self.is_live()
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
    /// The changes-pane section the file was selected in, which `at`
    /// counts in (`DiffCursor::section`); `None` off the changes pane.
    pub section: Option<WorkingTreeSection>,
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

/// `B` in the detail pane: open blame at the line he is on, or say why not.
/// Claimed on a file row, a heading or a diff line, either region — offer
/// or not: in the pane `B` is blame (keymap §2.5, m7-power-tools/36); the
/// offer's `B` lives only on the graph. `true` when `B` was taken.
pub(crate) fn b_key(next: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !claims_b(next) {
        return false;
    }
    let Some(cursor) = diff_cursor(next) else {
        return false;
    };
    let head = next.refs.as_ref().and_then(|refs| refs.head());
    let both = cursor.spec == DetailSpec::WorkingTree && staged_and_unstaged(next, &cursor.path);
    let from = match blame_from(&cursor, head, both, from::compare_split(next)) {
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
    let base = begin_trail(next, commands, &cursor);
    let opened = open_blame(next, from);
    settle_trail(next, base);
    commands.extend(opened);
    true
}

/// `B` and `F` start a new trail: whatever was on an old one is behind him
/// now. A base still set aside by it stays set aside, for this trail to put
/// back. Answers the base set aside, for [`settle_trail`].
fn begin_trail(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    cursor: &DiffCursor,
) -> Option<CompareBase> {
    let old = std::mem::take(&mut next.trail);
    cancel_reading(&old, commands);
    let pinned = next.compare_base.take();
    let base = pinned.or(old.origin.and_then(|origin| origin.base));
    next.trail.origin = Some(Origin {
        spec: cursor.spec,
        path: cursor.path.clone(),
        at: cursor.at,
        section: cursor.section,
        focus: next.detail.focus,
        base,
        spot: Spot::of(next),
    });
    base
}

/// After opening: refused before any place opened, nothing is set aside.
fn settle_trail(next: &mut AppState, base: Option<CompareBase>) {
    if !next.trail.is_live() {
        next.trail = Trail::default();
        next.compare_base = base;
    }
}

/// The pane would take `B`: shown and focused. [`b_key`]'s own guard, read
/// by the bar (`B blame`), so the two cannot disagree. The commit-graph
/// offer does not take it: its `B` answers only with the graph focused
/// (`commit_graph::offer_keys_live`, m7-power-tools/36).
fn claims_b(state: &AppState) -> bool {
    state.detail.shown && state.detail.focus != DetailFocus::Graph
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
        body: BlameLoad::Reading {
            job,
            want,
            came_from: came_from.clone(),
            ignore_listed: None,
        },
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

/// While the place is on screen it owns the keys (keymap §2.1).
#[must_use]
pub(crate) fn owns_keys(state: &AppState) -> bool {
    state.trail.place_shown()
}

/// The place's `/` prompt is open: a text field.
#[must_use]
pub(crate) fn prompt_open(state: &AppState) -> bool {
    state
        .trail
        .on_screen()
        .is_some_and(|blame| blame.find_prompt.is_some())
}
