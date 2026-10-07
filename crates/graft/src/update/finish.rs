//! The finish summary (m6-conflicts/19): `c` on a merge whose conflicts are
//! all resolved shows what will land — the files the user resolved and what
//! each came out as, the files git combined on its own, and the message —
//! and only `y` (from the file list) or `⏎` (from the subject) commits it
//! (keymap §2.6b).
//!
//! Modal while open: [`finish_key`] runs before the resolver and the banner
//! and claims every key but `Ctrl+C`. The summary is read off the loop's
//! thread (`jobs::finish`) under an id chosen here; so is the one diff a
//! row can show under it.
//!
//! **The app strips nothing from the message** (S3). The fields start from
//! `graft_ops`' `FinishSummary::message`, which already has git's comment
//! lines removed; whatever the user types or brings back from `$EDITOR` —
//! a `#123 fixes login` line included — is committed as it stands, through
//! `Resume::ContinueWith`'s `--cleanup=whitespace`.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_git::conflict::Take;
use graft_ops::apply::Resume;
use graft_ops::finish::{FinishSummary, ResultIs};
use graft_ops::operation::Side;
use graft_ui::conflict_resolver::SideNames;
use graft_ui::detail::DiffView;
use graft_ui::text_input::{TextArea, TextField};

use super::detail::Loading;
use super::events::Command;
use super::state::AppState;
use crate::jobs::operate::Operated;

/// Lines one wheel notch moves the summary: the resolver's step.
const WHEEL_LINES: usize = 3;

/// What the user chose for one conflicted file, for the summary's words
/// (`by_you_words`). Recorded by the resolver's answers (ticket 35); cleared
/// when the operation ends.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Choices {
    /// One entry per conflict block of the document: the take that landed,
    /// or `None` for a block not taken.
    pub takes: Vec<Option<Take>>,
    /// The whole file resolved as one side (`o`/`t` on a whole-file shape).
    pub whole: Option<Side>,
    /// `$EDITOR` or the mergetool had the file.
    pub edited: bool,
}

/// What has the keyboard in the summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishFocus {
    /// The file lists.
    Files,
    /// The subject field.
    Subject,
    /// The body area.
    Body,
}

impl FinishFocus {
    /// `⇥`: files → subject → body → files.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Files => Self::Subject,
            Self::Subject => Self::Body,
            Self::Body => Self::Files,
        }
    }

    /// `⇧⇥`: the way back — files → body → subject → files.
    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Files => Self::Body,
            Self::Subject => Self::Files,
            Self::Body => Self::Subject,
        }
    }
}

/// Where `Esc` goes back to: where `c` was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnTo {
    /// The operation banner, focused.
    Banner,
    /// The resolver (finish-from-the-resolver, a later ticket).
    Resolver,
}

/// The open summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinishState {
    /// What the merge will commit, once read.
    pub summary: Loading<Arc<FinishSummary>>,
    /// The summary read in flight; an answer under any other id is stale.
    pub job: Option<JobId>,
    /// The cursor row, counted over `by_you` then `by_git`.
    pub cursor: usize,
    /// The first list line on screen.
    pub top: usize,
    /// The row whose staged diff is shown under it.
    pub expanded: Option<(usize, Loading<Arc<DiffView>>)>,
    /// The diff read in flight; an answer under any other id is stale.
    pub diff_job: Option<JobId>,
    /// The message's first line.
    pub subject: TextField,
    /// The rest of the message.
    pub body: TextArea,
    /// What has the keyboard.
    pub focus: FinishFocus,
    /// Where `Esc` goes.
    pub return_to: ReturnTo,
    /// The `--continue` running under this id.
    pub committing: Option<JobId>,
    /// `$EDITOR` has the message under this id.
    pub editing: Option<JobId>,
    /// A line above the bar, and whether it is a problem.
    pub notice: Option<(String, bool)>,
    /// The fields came from `finish_draft`: the summary landing must not
    /// overwrite them with git's message.
    pub from_draft: bool,
}

/// How [`resume_answered`] handled a `--continue` the summary started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FinishAnswer {
    /// It committed: the summary is closed. The words are the status line
    /// (`merged feature into main`), when the summary was read.
    Merged(Option<String>),
    /// git refused (a hook): the summary stays open with the reason.
    Kept,
    /// Anything else: the summary is closed, the draft kept, and the banner
    /// takes it from here as it always has.
    Closed,
}

/// Open the summary over the banner (or, later, the resolver) and start
/// reading it. `false` when one is already open.
pub fn open_finish(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    from: ReturnTo,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.finish.is_some() {
        return false;
    }
    let id = next_id();
    let draft = state.finish_draft.take();
    let from_draft = draft.is_some();
    let (subject, body) = draft.unwrap_or_default();
    state.finish = Some(FinishState {
        summary: Loading::Pending(id),
        job: Some(id),
        cursor: 0,
        top: 0,
        expanded: None,
        diff_job: None,
        subject: TextField::new(&subject),
        body: TextArea::new(&body),
        focus: FinishFocus::Files,
        return_to: from,
        committing: None,
        editing: None,
        notice: None,
        from_draft,
    });
    commands.push(Command::ReadFinishSummary(id));
    true
}

/// After a resize: the cursor row back on screen.
pub(crate) fn resized(state: &mut AppState) {
    keep_cursor_on_screen(state);
}

/// Is there room to draw the summary at all?
#[must_use]
pub fn fits(state: &AppState) -> bool {
    graft_ui::finish_summary::fits(crate::view::finish::summary_area(state))
}

/// `top` moved as little as it must for the cursor row (and its diff) to be
/// on screen — the same arithmetic the widget draws with.
fn keep_cursor_on_screen(state: &mut AppState) {
    let top = {
        let Some(view) = crate::view::finish::finish_view(state) else {
            return;
        };
        graft_ui::finish_summary::top_for_cursor(&view, crate::view::finish::summary_area(state))
    };
    with_finish(state, |finish| finish.top = top);
}

fn with_finish(state: &mut AppState, f: impl FnOnce(&mut FinishState)) {
    if let Some(finish) = state.finish.as_mut() {
        f(finish);
    }
}

mod answers;
mod choices;
mod keys;
mod mouse;
mod words;

pub(crate) use answers::{
    diff_landed, editor_returned, job_ended, resume_answered, summary_landed,
};
pub(crate) use choices::{forget, record_edited, record_take, record_whole};
pub use keys::finish_key;
pub(crate) use keys::paste;
pub(crate) use mouse::finish_mouse;
pub(crate) use words::{by_you_words, compose, split_message};

#[cfg(test)]
pub(crate) mod tests;
