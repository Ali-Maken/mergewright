//! The merge's review (ease-pass/12; m6-conflicts/19 before it): once a
//! merge's clashes are all solved, `y` (or `c` on the banner) shows what
//! will land — every clashed file and how it was solved, how many files git
//! combined on its own, and git's message, read-only — and only `y` commits
//! it (keymap §2.6b).
//!
//! Modal while open: [`finish_key`] runs before the resolver and the banner
//! and claims every key but `Ctrl+C`. The review is read off the loop's
//! thread (`jobs::finish`) under an id chosen here, and drawn only once
//! that read lands; so is the one diff a row can show under it.
//!
//! **The merge commits git's own message** (ease-pass/12 decision 5): the
//! read's `FinishSummary::message` — `MERGE_MSG` with its comment lines
//! removed — through `Resume::Continue`, as `git commit --no-edit` would.

use std::path::PathBuf;
use std::sync::Arc;

use mergewright_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use mergewright_core::job::JobId;
use mergewright_git::conflict::Take;
use mergewright_ops::apply::Resume;
use mergewright_ops::finish::{FinishSummary, ResultIs};
use mergewright_ops::operation::Side;
use mergewright_ui::detail::DiffView;

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
    /// Where `Esc` goes.
    pub return_to: ReturnTo,
    /// The `--continue` running under this id.
    pub committing: Option<JobId>,
    /// A line above the bar, and whether it is a problem.
    pub notice: Option<(String, bool)>,
    /// The files `y` stages inside the finish (ease-pass/05b): the
    /// review's read fills it; empty until then.
    pub solved: Vec<PathBuf>,
    /// The commits the merge brings in (`HEAD..MERGE_HEAD`), asked beside
    /// the review's read; `None` until it lands or when it failed (`?`).
    pub incoming: Option<u32>,
}

/// The review's read (ease-pass/05b): what the merge will commit, and the
/// files its `y` stages, both from `mergewright_ops::finish::merge_review`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reviewed {
    /// What the merge will commit, as if `solved` were staged.
    pub summary: Arc<FinishSummary>,
    /// The unmerged files solved on disk: staged inside the finish.
    pub solved: Vec<PathBuf>,
    /// `HEAD..MERGE_HEAD`, counted the way a folded merge's tail is
    /// (`mergewright_git::history_reads::count_between`); `None` when it failed.
    pub incoming: Option<u32>,
}

impl From<Arc<FinishSummary>> for Reviewed {
    /// A summary with nothing left to stage: every path already staged.
    fn from(summary: Arc<FinishSummary>) -> Self {
        Self {
            summary,
            solved: Vec::new(),
            incoming: None,
        }
    }
}

/// Why the review has nothing to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewFailed {
    /// Files still have a clash open: no review; where it was asked for
    /// says how many (ease-pass/05b; clashes counted since 05s).
    StillOpen {
        /// How many files.
        files: usize,
        /// Their open clashes; a whole-file card is one.
        clashes: usize,
    },
    /// The read failed, in `mergewright_ops`' own words: shown in the review.
    Failed(String),
}

impl From<String> for ReviewFailed {
    fn from(why: String) -> Self {
        Self::Failed(why)
    }
}

/// How [`resume_answered`] handled a `--continue` the summary started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FinishAnswer {
    /// It committed: the summary is closed. The words are the status line
    /// (`Merged feature into main.`), when the review was read.
    Merged(Option<String>),
    /// git refused (a hook): the summary stays open with the reason.
    Kept,
    /// Anything else: the review is closed and the banner takes it from
    /// here as it always has.
    Closed,
}

/// Open the summary over the banner (or the resolver) and start reading
/// it: the review drawn `reading…` until its read lands. `false` when one
/// is already open. The app's doors ask with [`ask_review`] instead, which
/// draws nothing until the answer; this stays for a view of the read in
/// flight.
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
    open_reading(state, from, id);
    commands.push(Command::ReadFinishSummary(id));
    true
}

/// Ask the merge review's read (ease-pass/05s): nothing is drawn until it
/// lands (`answers::summary_landed`) — `Ready` or `Failed` opens the
/// review, `StillOpen` says how many where it was asked for. `false`, and
/// nothing sent, while a review is open or a read is already out.
pub(crate) fn ask_review(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    from: ReturnTo,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.finish.is_some() || state.picks.review_asked() {
        return false;
    }
    let id = next_id();
    state.picks.ask_review(id, from);
    commands.push(Command::ReadFinishSummary(id));
    true
}

/// The review, open from `from`, waiting on the read `id`. The doors
/// (`ask_review`) call it only from the read's answer, which fills it in
/// the same update: no frame draws it waiting (ease-pass/12 decision 7).
fn open_reading(state: &mut AppState, from: ReturnTo, id: JobId) {
    state.finish = Some(Box::new(FinishState {
        summary: Loading::Pending(id),
        job: Some(id),
        cursor: 0,
        top: 0,
        expanded: None,
        diff_job: None,
        return_to: from,
        committing: None,
        notice: None,
        solved: Vec::new(),
        incoming: None,
    }));
}

/// `Some(RowBar::JustMerged)` while the cursor rests on the merge the
/// review just made, it is still `HEAD`, and its count is known (M18);
/// before the count lands the row keeps its own bar (G's), and a
/// fast-forward — no merge commit, never `last_merge` — never has one.
#[must_use]
pub(crate) fn just_merged_bar(
    state: &AppState,
    cursor: Option<mergewright_core::oid::Oid>,
) -> Option<mergewright_ui::graph_screen::hints::RowBar> {
    let merge = state.last_merge?;
    if cursor != Some(merge) || crate::update::commit_editor::head_oid(state) != Some(merge) {
        return None;
    }
    match state.fold_counts.get(&merge)? {
        crate::update::fold_count::FoldCount::Counted(inside) => {
            Some(mergewright_ui::graph_screen::hints::RowBar::JustMerged {
                inside: u32::try_from(*inside).unwrap_or(u32::MAX),
            })
        }
        crate::update::fold_count::FoldCount::Counting
        | crate::update::fold_count::FoldCount::Failed => None,
    }
}

/// After a resize: the cursor row back on screen.
pub(crate) fn resized(state: &mut AppState) {
    keep_cursor_on_screen(state);
}

/// Is there room to draw the summary at all?
#[must_use]
pub fn fits(state: &AppState) -> bool {
    mergewright_ui::finish_summary::fits(crate::view::finish::summary_area(state))
}

/// `top` moved as little as it must for the cursor row (and its diff) to be
/// on screen — the same arithmetic the widget draws with.
fn keep_cursor_on_screen(state: &mut AppState) {
    let top = {
        let Some(view) = crate::view::finish::finish_view(state) else {
            return;
        };
        mergewright_ui::finish_summary::top_for_cursor(
            &view,
            crate::view::finish::summary_area(state),
        )
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

pub(crate) use answers::{diff_landed, job_ended, resume_answered, summary_landed};
pub(crate) use choices::{forget, record_edited, record_take, record_whole};
pub use keys::finish_key;
pub(crate) use mouse::finish_mouse;
pub(crate) use words::{how_solved, subject_of};

#[cfg(test)]
pub(crate) mod tests;
