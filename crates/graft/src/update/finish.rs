//! The finish summary (m6-conflicts/19): `c` on a merge whose conflicts are
//! all resolved shows what will land — the files the user resolved and what
//! each came out as, the files git combined on its own, and the message —
//! and only `⏎` commits it (keymap §2.6b).
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

/// The summary read came back.
pub(crate) fn summary_landed(
    state: &mut AppState,
    id: JobId,
    summary: Result<Arc<FinishSummary>, String>,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    // A read for a summary since closed (and perhaps reopened): the open
    // one has its own read in flight, which is the one to show.
    if finish.job != Some(id) {
        return;
    }
    finish.job = None;
    match summary {
        Ok(summary) => {
            // A restored draft is the user's words; so is anything typed
            // while the read ran. Git's message fills only empty fields.
            let untouched = finish.subject.is_empty() && finish.body.is_empty();
            if !finish.from_draft && untouched {
                let (subject, body) = split_message(&String::from_utf8_lossy(&summary.message));
                finish.subject = TextField::new(&subject);
                finish.body = TextArea::new(&body);
            }
            finish.cursor = if summary.by_git.is_empty() {
                0
            } else {
                summary.by_you.len()
            };
            finish.summary = Loading::Ready(summary);
        }
        Err(why) => finish.summary = Loading::Failed(why),
    }
    keep_cursor_on_screen(state);
}

/// A row's diff came back.
pub(crate) fn diff_landed(state: &mut AppState, id: JobId, diff: Result<Arc<DiffView>, String>) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    // Dropping a stale answer is safe: `space` on another row (or on this
    // one again) replaced or collapsed the diff it was for, and the diff on
    // screen now has its own read in flight or none at all.
    if finish.diff_job != Some(id) {
        return;
    }
    finish.diff_job = None;
    if let Some((_, shown)) = finish.expanded.as_mut() {
        *shown = match diff {
            Ok(diff) => Loading::Ready(diff),
            Err(why) => Loading::Failed(why),
        };
    }
    keep_cursor_on_screen(state);
}

/// A job the summary waits on failed or panicked: say so where its answer
/// would have gone, so nothing reads `reading…` forever.
pub(crate) fn job_ended(state: &mut AppState, id: JobId, failure: &dyn std::fmt::Display) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    if finish.job == Some(id) {
        finish.job = None;
        finish.summary = Loading::Failed(failure.to_string());
    }
    if finish.diff_job == Some(id) {
        finish.diff_job = None;
        if let Some((_, shown)) = finish.expanded.as_mut() {
            *shown = Loading::Failed(failure.to_string());
        }
    }
}

/// `$EDITOR` handed the message back. `true` when it was the summary's.
///
/// **Strips nothing** (S3): a `#123 fixes login` line the user wrote in the
/// editor is part of the message.
pub(crate) fn editor_returned(
    state: &mut AppState,
    id: JobId,
    text: Result<String, String>,
) -> bool {
    let Some(finish) = state.finish.as_mut() else {
        return false;
    };
    if finish.editing != Some(id) {
        return false;
    }
    finish.editing = None;
    state.external_editor = None;
    match text {
        Ok(text) => {
            let (subject, body) = split_message(&text);
            finish.subject = TextField::new(&subject);
            finish.body = TextArea::new(&body);
            finish.notice = None;
        }
        // The fields still hold what went out: nothing was lost.
        Err(why) => finish.notice = Some((why, true)),
    }
    true
}

/// A `--continue` came back; acts only on the one the summary started.
pub(crate) fn resume_answered(
    state: &mut AppState,
    id: JobId,
    outcome: &Operated,
) -> Option<FinishAnswer> {
    let finish = state.finish.as_mut()?;
    if finish.committing != Some(id) {
        return None;
    }
    finish.committing = None;
    if let Some(reason) = refusal(outcome) {
        finish.notice = Some((reason, true));
        return Some(FinishAnswer::Kept);
    }
    if let Operated::Done(_) = outcome {
        let words = match &finish.summary {
            Loading::Ready(summary) => Some(format!(
                "merged {} into {}",
                String::from_utf8_lossy(&summary.from),
                String::from_utf8_lossy(&summary.into)
            )),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        };
        state.finish = None;
        state.finish_draft = None;
        state.external_editor = None;
        return Some(FinishAnswer::Merged(words));
    }
    close_keeping_draft(state);
    Some(FinishAnswer::Closed)
}

/// git said no to the commit and nothing moved: a hook refused it, or a
/// precondition did. The words to show, else `None`.
fn refusal(outcome: &Operated) -> Option<String> {
    match outcome {
        Operated::Failed { error, refused, .. } => Some(
            refused
                .as_ref()
                .map_or_else(|| error.clone(), graft_ops::operation::Refusal::explain),
        ),
        // A `--continue` that stopped with nothing unmerged is git refusing
        // to commit (`jobs::operate::Operated::Conflicted::stalled`).
        Operated::Conflicted {
            stalled: Some(words),
            paths,
            ..
        } if paths.is_empty() => Some(words.clone()),
        Operated::Done(_) | Operated::Incomplete { .. } | Operated::Conflicted { .. } => None,
    }
}

/// The summary's keys while it is open. `false` only for `Ctrl+C` (and
/// when it is closed): every other key is the summary's (§2.1).
pub fn finish_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(finish) = state.finish.as_ref() else {
        return false;
    };
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    // The text on screen is what is being written: nothing changes it.
    if finish.committing.is_some() {
        return true;
    }
    // Below the widget's floor only one line is drawn: nothing on screen to
    // act on, so only `Esc` acts.
    if !fits(state) {
        if key.code == KeyCode::Escape && !key.modifiers.control && !key.modifiers.alt {
            escape(state);
        }
        return true;
    }
    if key.modifiers.control {
        control_key(state, commands, key, next_id);
        return true;
    }
    if key.modifiers.alt {
        return true;
    }
    match key.code {
        KeyCode::Escape => escape(state),
        // `Shift+Tab` arrives as its own code; P11 is not approved, so it
        // does nothing here.
        KeyCode::Tab if !key.modifiers.shift => with_finish(state, |finish| {
            finish.focus = finish.focus.next();
        }),
        _ => match finish.focus {
            FinishFocus::Files => files_key(state, commands, key, next_id),
            FinishFocus::Subject => subject_key(state, commands, key, next_id),
            FinishFocus::Body => body_key(state, key),
        },
    }
    true
}

/// `Ctrl`-held keys: they mean the same wherever the focus is.
fn control_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    match key.code {
        KeyCode::Enter => commit(state, commands, next_id),
        KeyCode::Char('e') => edit_outside(state, commands, next_id),
        // §2.10's word moves, in whichever field has focus.
        KeyCode::Left => with_finish(state, |finish| match finish.focus {
            FinishFocus::Subject => finish.subject.word_left(),
            FinishFocus::Body => finish.body.word_left(),
            FinishFocus::Files => {}
        }),
        KeyCode::Right => with_finish(state, |finish| match finish.focus {
            FinishFocus::Subject => finish.subject.word_right(),
            FinishFocus::Body => finish.body.word_right(),
            FinishFocus::Files => {}
        }),
        _ => {}
    }
}

/// Keys while the file lists have focus.
fn files_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    match key.code {
        KeyCode::Enter => commit(state, commands, next_id),
        KeyCode::Char('j') => move_cursor(state, 1),
        KeyCode::Char('k') => move_cursor(state, -1),
        KeyCode::Char(' ') => toggle_diff(state, commands, next_id),
        _ => {}
    }
}

/// Keys while the subject has focus: `⏎` commits (§2.6b).
fn subject_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    if key.code == KeyCode::Enter {
        commit(state, commands, next_id);
        return;
    }
    with_finish(state, |finish| {
        let field = &mut finish.subject;
        match key.code {
            KeyCode::Char(character) => field.insert(character),
            KeyCode::Backspace => field.backspace(),
            KeyCode::Delete => field.delete(),
            KeyCode::Left => field.left(),
            KeyCode::Right => field.right(),
            KeyCode::Home => field.home(),
            KeyCode::End => field.end(),
            _ => {}
        }
    });
}

/// Keys while the body has focus: `⏎` is a newline (§2.6b).
fn body_key(state: &mut AppState, key: KeyPress) {
    with_finish(state, |finish| {
        let area = &mut finish.body;
        match key.code {
            KeyCode::Enter => area.newline(),
            KeyCode::Char(character) => area.insert(character),
            KeyCode::Backspace => area.backspace(),
            KeyCode::Delete => area.delete(),
            KeyCode::Left => area.left(),
            KeyCode::Right => area.right(),
            KeyCode::Up => area.up(),
            KeyCode::Down => area.down(),
            KeyCode::Home => area.home(),
            KeyCode::End => area.end(),
            _ => {}
        }
    });
}

/// `j`/`k`: the cursor over both lists, kept on screen.
fn move_cursor(state: &mut AppState, by: isize) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let rows = match &finish.summary {
        Loading::Ready(summary) => summary.by_you.len() + summary.by_git.len(),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => 0,
    };
    if rows == 0 {
        return;
    }
    finish.cursor = finish.cursor.saturating_add_signed(by).min(rows - 1);
    keep_cursor_on_screen(state);
}

/// The path the cursor row names, when the summary is read.
fn row_path(finish: &FinishState, row: usize) -> Option<PathBuf> {
    let Loading::Ready(summary) = &finish.summary else {
        return None;
    };
    match summary.by_you.get(row) {
        Some(file) => Some(file.path.clone()),
        None => summary
            .by_git
            .get(row - summary.by_you.len())
            .map(|file| file.path.clone()),
    }
}

/// `space`: show the cursor row's staged diff under it, or hide it.
fn toggle_diff(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let row = finish.cursor;
    if finish.expanded.as_ref().is_some_and(|(at, _)| *at == row) {
        finish.expanded = None;
        finish.diff_job = None;
        return;
    }
    let Some(path) = row_path(finish, row) else {
        return;
    };
    let id = next_id();
    finish.expanded = Some((row, Loading::Pending(id)));
    finish.diff_job = Some(id);
    commands.push(Command::ReadFinishDiff(id, path));
    keep_cursor_on_screen(state);
}

/// `⏎` (files, subject) or `Ctrl+Enter`: commit.
fn commit(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let what = match &finish.summary {
        Loading::Idle | Loading::Pending(_) => {
            finish.notice = Some(("still reading · wait".to_owned(), false));
            return;
        }
        // Nothing to compare against: git's own message, as the line says.
        Loading::Failed(_) => Resume::Continue,
        Loading::Ready(summary) => {
            if finish.subject.text().trim().is_empty() {
                finish.notice = Some(("the message is empty · ⇥ to write one".to_owned(), true));
                return;
            }
            let message = compose(finish.subject.text(), &finish.body.text());
            if message.trim_end() == String::from_utf8_lossy(&summary.message).trim_end() {
                Resume::Continue
            } else {
                Resume::ContinueWith {
                    message: message.into_bytes(),
                }
            }
        }
    };
    if super::operate::operation_in_flight(state) {
        with_finish(state, |finish| {
            finish.notice = Some(("a write is already running".to_owned(), true));
        });
        return;
    }
    if super::banner::resume(state, commands, what, next_id) {
        let operating = state.operating;
        with_finish(state, |finish| {
            finish.committing = operating;
            finish.notice = None;
        });
    } else {
        with_finish(state, |finish| {
            finish.notice = Some(("no merge is in progress".to_owned(), true));
        });
    }
}

/// `Ctrl+E`: the whole message to `$EDITOR`, the way the commit editor
/// hands it over.
fn edit_outside(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let id = next_id();
    let message = compose(finish.subject.text(), &finish.body.text());
    finish.editing = Some(id);
    state.external_editor = Some(id);
    commands.push(Command::EditCommitMessage { id, message });
}

/// `Esc`: close. What the user typed goes to `finish_draft`; from the
/// banner, the banner gets the keyboard back.
fn escape(state: &mut AppState) {
    let back_to_banner = state
        .finish
        .as_ref()
        .is_some_and(|finish| finish.return_to == ReturnTo::Banner);
    close_keeping_draft(state);
    if back_to_banner {
        state.banner_focus = true;
    }
}

/// Close the summary, keeping the message as `finish_draft` when it is the
/// user's — changed from git's, or typed before git's arrived.
fn close_keeping_draft(state: &mut AppState) {
    let Some(finish) = state.finish.take() else {
        return;
    };
    // Whatever `$EDITOR` is about to hand back belongs to a summary that is
    // no longer on screen.
    if finish.editing.is_some() {
        state.external_editor = None;
    }
    let body = finish.body.text();
    let composed = compose(finish.subject.text(), &body);
    let the_users = match &finish.summary {
        Loading::Ready(summary) => {
            composed.trim_end() != String::from_utf8_lossy(&summary.message).trim_end()
        }
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => {
            !finish.subject.is_empty() || !finish.body.is_empty()
        }
    };
    state.finish_draft = the_users.then(|| (finish.subject.text().to_owned(), body));
}

/// A bracketed paste, into the focused field. `true` whenever the summary
/// is open: it is modal, so a paste is meant for it even with the file
/// lists focused and nowhere to put the text.
pub(crate) fn paste(state: &mut AppState, text: &str) -> bool {
    let Some(finish) = state.finish.as_mut() else {
        return false;
    };
    if finish.committing.is_some() {
        return true;
    }
    match finish.focus {
        FinishFocus::Subject => finish.subject.insert_str(text),
        FinishFocus::Body => finish.body.insert_str(text),
        FinishFocus::Files => {}
    }
    true
}

/// The mouse while the summary is open (§2.15). Claims every event: nothing
/// behind a modal answers a click.
pub(crate) fn finish_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(finish) = state.finish.as_ref() else {
        return false;
    };
    if finish.committing.is_some() || !fits(state) || super::resolver::box_open(state) {
        return true;
    }
    match mouse.kind {
        MouseKind::ScrollUp => scroll(state, false),
        MouseKind::ScrollDown => scroll(state, true),
        MouseKind::Down(MouseButton::Left) => click(state, commands, mouse, next_id),
        _ => {}
    }
    true
}

/// The wheel: the list moves, the cursor stays where it is.
fn scroll(state: &mut AppState, down: bool) {
    let most = {
        let Some(view) = crate::view::finish::finish_view(state) else {
            return;
        };
        graft_ui::finish_summary::max_top(&view, crate::view::finish::summary_area(state))
    };
    with_finish(state, |finish| {
        finish.top = if down {
            finish.top.saturating_add(WHEEL_LINES).min(most)
        } else {
            finish.top.saturating_sub(WHEEL_LINES)
        };
    });
}

/// A left click: a row takes the cursor and toggles its diff; the subject
/// or the body takes the focus.
fn click(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
    next_id: &mut impl FnMut() -> JobId,
) {
    let landed = {
        let Some(view) = crate::view::finish::finish_view(state) else {
            return;
        };
        graft_ui::finish_summary::hit(
            &view,
            crate::view::finish::summary_area(state),
            mouse.column,
            mouse.row,
        )
    };
    match landed {
        Some(graft_ui::finish_summary::FinishHit::Row(row)) => {
            with_finish(state, |finish| {
                finish.cursor = row;
                finish.focus = FinishFocus::Files;
            });
            toggle_diff(state, commands, next_id);
            keep_cursor_on_screen(state);
        }
        Some(graft_ui::finish_summary::FinishHit::Subject) => {
            with_finish(state, |finish| finish.focus = FinishFocus::Subject);
        }
        Some(graft_ui::finish_summary::FinishHit::Body) => {
            with_finish(state, |finish| finish.focus = FinishFocus::Body);
        }
        None => {}
    }
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

/// A message split the way git splits one: the first line is the subject;
/// the rest, after one blank line if there is one, is the body.
#[must_use]
pub(crate) fn split_message(text: &str) -> (String, String) {
    let text = text.replace("\r\n", "\n");
    match text.split_once('\n') {
        Some((first, rest)) => {
            let rest = rest.strip_prefix('\n').unwrap_or(rest);
            (
                first.trim_end().to_owned(),
                rest.trim_end_matches('\n').to_owned(),
            )
        }
        None => (text.trim_end().to_owned(), String::new()),
    }
}

/// The message the fields hold: the subject, then a blank line and the body
/// when the body is not blank.
#[must_use]
pub(crate) fn compose(subject: &str, body: &str) -> String {
    if body.trim().is_empty() {
        subject.to_owned()
    } else {
        format!("{subject}\n\n{body}")
    }
}

/// What a resolved file's row says about it: what the user chose when that
/// was recorded, else what the result is. First match wins (ticket 19,
/// scope 3).
#[must_use]
pub(crate) fn by_you_words(
    choices: Option<&Choices>,
    result: ResultIs,
    names: &SideNames,
) -> String {
    let name = |side: Side| match side {
        Side::Ours => names.ours.as_str(),
        Side::Theirs => names.theirs.as_str(),
    };
    if let Some(choices) = choices {
        if let Some(side) = choices.whole {
            return if result == ResultIs::Deleted {
                format!("deleted · as {}", name(side))
            } else {
                format!("kept · {}", name(side))
            };
        }
        if choices.edited {
            return "edited in your editor".to_owned();
        }
        let count = |take: Take| {
            choices
                .takes
                .iter()
                .filter(|landed| **landed == Some(take))
                .count()
        };
        let kinds: Vec<(usize, &str)> = [
            (count(Take::Ours), names.ours.as_str()),
            (count(Take::Theirs), names.theirs.as_str()),
            (count(Take::Both), "both"),
        ]
        .into_iter()
        .filter(|(taken, _)| *taken > 0)
        .collect();
        match kinds.as_slice() {
            [] => {}
            [(taken, label)] => {
                return format!("{label} in {taken} of {} conflicts", choices.takes.len());
            }
            _ => {
                return kinds
                    .iter()
                    .map(|(taken, label)| format!("{label} in {taken}"))
                    .collect::<Vec<_>>()
                    .join(", ");
            }
        }
    }
    match result {
        ResultIs::Yours => format!("same as {}", names.ours.as_str()),
        ResultIs::Theirs => format!("same as {}", names.theirs.as_str()),
        ResultIs::Deleted => "deleted".to_owned(),
        ResultIs::Combined => "combined".to_owned(),
    }
}

#[cfg(test)]
mod tests;
