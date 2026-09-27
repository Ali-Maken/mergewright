//! A blame job's request and answer, and what the place does with it.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::blame::{Blame, BlameOptions, Rev};
use graft_ui::blame::BlameRowView;

use super::from::NoBlame;
use super::{Place, keys, leave, no_blame_words, rows, short};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// A blame place's body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameLoad {
    /// The job is running.
    Reading {
        /// The job; `Esc` cancels it.
        job: JobId,
        /// The line the cursor goes to when it answers.
        want: Want,
        /// `^`'s second read, kept so `w`/`i` before the answer ask it too.
        came_from: Option<CameFromRequest>,
        /// How many revisions the ignore file listed in the last answer
        /// (`None` before one, or when there is no file): the ignore line
        /// and `i` stay through a re-read.
        ignore_listed: Option<usize>,
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
    let asked_line = index
        .checked_sub(1)
        .and_then(|newer| state.trail.places.get(newer))
        .and_then(|Place::Blame(newer)| newer.cursor_line());
    let Place::Blame(blame) = &mut state.trail.places[index];
    let BlameLoad::Reading {
        want, came_from, ..
    } = &blame.body
    else {
        return;
    };
    let (want, came_from) = (*want, came_from.clone());
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
                Want::CameFrom => {
                    let Some(line) = answer.line else {
                        // The job read the parent but not where the line
                        // came from: say so, not a guess at line 1.
                        blame.body = BlameLoad::Failed(cannot_tell(asked_line, came_from));
                        return;
                    };
                    rows::row_of_line(&answer.blame, line)
                }
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
        Err(BlameFailure::Untracked) => {
            let why = NoBlame::Untracked(blame.path.clone());
            refuse(state, commands, index, &why);
        }
        Err(BlameFailure::Binary) => {
            let why = NoBlame::Binary(blame.path.clone());
            refuse(state, commands, index, &why);
        }
    }
}

/// `^` answered with no line: the words name the newer blame's line (the
/// one `^` was pressed on) and the commit that last changed it.
fn cannot_tell(line: Option<u32>, came_from: Option<CameFromRequest>) -> String {
    let line = line.or_else(|| came_from.as_ref().map(|asked| asked.line));
    match (line, came_from) {
        (Some(line), Some(asked)) => format!(
            "can't tell where line {line} came from in {} · esc back",
            short(asked.commit)
        ),
        _ => "can't tell where the line came from · esc back".to_owned(),
    }
}

/// git refused the place at `index`: it comes off the trail (leaving it if
/// it was the last) and the status line says why.
fn refuse(state: &mut AppState, commands: &mut Vec<Command>, index: usize, why: &NoBlame) {
    state.trail.places.remove(index);
    if state.trail.is_live() {
        state.trail.shown = true;
    } else {
        leave(state, commands, true);
    }
    state.status = Status::sticky(no_blame_words(why));
}
