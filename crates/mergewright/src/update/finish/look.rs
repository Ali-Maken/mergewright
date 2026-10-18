//! The review's read-only look (ease-pass/12b): `⏎`/`space` on a row opens
//! it, `esc` goes back to the review with the cursor kept, `y` commits as
//! the review's `y` does. Nothing else acts: the look changes nothing.

use std::path::PathBuf;
use std::sync::Arc;

use mergewright_git::diff::Origin;

use super::keys::{asks, commit};
use super::{AppState, Command, JobId, KeyPress, Loading, keep_cursor_on_screen, with_finish};

/// One combined file's lines since the merge base, `(added, deleted)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideCounts {
    /// The file.
    pub path: PathBuf,
    /// Merge base → `HEAD`.
    pub yours: (u32, u32),
    /// Merge base → `MERGE_HEAD`.
    pub theirs: (u32, u32),
}

/// One combined file's sample: `(line, text, side it came from)`; `None`
/// for a line neither side added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sample {
    /// The file.
    pub path: PathBuf,
    /// Its lines, in order.
    pub lines: Vec<(u32, String, Option<Origin>)>,
}

/// What the look asks to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookAsk {
    /// A clashed file's result on disk.
    Clashed(PathBuf),
    /// The files git combined on its own.
    Combined(Vec<PathBuf>),
}

/// What the look read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookRead {
    /// The clashed file's text, as it will land.
    Clashed(String),
    /// Each combined file's counts, and one sample.
    Combined {
        /// One per file, in the review's order.
        files: Vec<SideCounts>,
        /// The sample, when a file had a hunk to show.
        sample: Option<Sample>,
    },
}

/// The open look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    /// The clashed file looked at; `None` on the combined row.
    pub path: Option<PathBuf>,
    /// The read.
    pub read: Loading<Arc<LookRead>>,
}

/// `⏎`/`space`: open the look on the cursor row and start its read.
pub(super) fn open_look(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let Loading::Ready(summary) = &finish.summary else {
        return;
    };
    let ask = if let Some(file) = summary.by_you.get(finish.cursor) {
        LookAsk::Clashed(file.path.clone())
    } else if !summary.by_git.is_empty() {
        LookAsk::Combined(summary.by_git.iter().map(|f| f.path.clone()).collect())
    } else {
        return;
    };
    let path = match &ask {
        LookAsk::Clashed(path) => Some(path.clone()),
        LookAsk::Combined(_) => None,
    };
    let id = next_id();
    finish.look = Some(Look {
        path,
        read: Loading::Pending(id),
    });
    finish.look_job = Some(id);
    commands.push(Command::ReadFinishLook(id, ask));
}

/// A key while the look is open: `esc` back, `y` commits; nothing else
/// acts.
pub(super) fn look_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    if asks(key, "app.back") {
        close_look(state);
    } else if asks(key, "confirm.yes") {
        // The review's own `y`, from the review the look sits on: what the
        // commit shows while it runs is the review, as from the list.
        close_look(state);
        commit(state, commands, next_id);
    }
}

/// Back to the review, the cursor where it was.
pub(super) fn close_look(state: &mut AppState) {
    with_finish(state, |finish| {
        finish.look = None;
        finish.look_job = None;
    });
    keep_cursor_on_screen(state);
}

/// The look's read came back; a stale one (the look closed, or another
/// opened) is dropped.
pub(crate) fn look_landed(state: &mut AppState, id: JobId, read: Result<Arc<LookRead>, String>) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    // Dropping a stale answer is safe: the look it was for is gone, and
    // the look on screen has its own read in flight or none.
    if finish.look_job != Some(id) {
        return;
    }
    finish.look_job = None;
    if let Some(look) = finish.look.as_mut() {
        look.read = match read {
            Ok(read) => Loading::Ready(read),
            Err(why) => Loading::Failed(why),
        };
    }
}
