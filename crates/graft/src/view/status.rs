//! Building [`StatusView`]: what was settled at startup, and what has
//! happened since.

use graft_git::version::SystemGit;
use graft_term::{SanitizedText, sanitize_line, sanitize_path, sanitize_path_bytes};
use graft_ui::graph_screen::ProblemLine;
use graft_ui::status_screen::{HeadView, HistoryState, HistoryView, RepositoryView, StatusView};

use crate::history::{History, Phase};
use crate::repository::{Head, RepositoryFacts};
use crate::session::Session;
use crate::update::{AppState, GitCheck, Status};

/// How much of a commit id a detached `HEAD` shows. Long enough to be unique
/// in any repository a person works in, short enough to read back.
const ABBREVIATED_ID: usize = 12;

/// Everything the status screen draws, assembled from the two things `app`
/// knows: what was settled at startup, and what has happened since.
///
/// Separate from [`super::view`] so it can be asserted on directly — a view
/// model is data, and comparing data beats reading cells out of a buffer.
#[must_use]
pub fn status_view(session: &Session, state: &AppState) -> StatusView {
    StatusView {
        repository: state.repository.as_ref().map(repository_view),
        rendering: session.rendering,
        notes: notes(session, state),
        problem: problem(&state.status),
        history: state.history.as_ref().map(history_view),
    }
}

/// The one reading of [`Status`] that is a problem worth putting in front of
/// the user: a failure, read or write. Every screen that shows one — the
/// status screen, the commit editor — asks here (the graph footer asks
/// [`graph_problem`] instead, so it can keep a sticky failure's keys whole),
/// so they cannot drift apart (m3/05 review).
#[must_use]
pub(crate) fn problem(status: &Status) -> Option<SanitizedText> {
    match status {
        Status::Ready | Status::Copied { .. } | Status::Busy(_) | Status::Done(_) => None,
        Status::Failed(message) => Some(sanitize_line(message)),
        Status::Sticky { reason, keys: None } => Some(sanitize_line(reason)),
        Status::Sticky {
            reason,
            keys: Some(keys),
        } => Some(sanitize_line(&format!("{reason} · {keys}"))),
    }
}

/// What a failure line says when it carries no keys of its own. `Esc`
/// really does clear a `Failed`/`Sticky` status (`update::keys::key_event`),
/// and a failure that names no key at all is the one footer ADR-0037 §4
/// rules out — the bar must never be keyless (m5-03 review, Major 2).
const DISMISS: &str = "esc dismiss";

/// The graph footer's reading of [`Status`]: the same problem, but with the
/// reason and keys kept apart so [`graft_ui::graph_screen::hints::fit_status_line`]
/// can cut the reason first and keep the keys whole (ADR-0037 §4,
/// `hint-bar-redesign`), and with [`DISMISS`] standing in whenever the
/// failure carries no keys — the common case (a hook rejected the commit,
/// `$EDITOR` failed, staging refused on a conflict).
#[must_use]
pub(crate) fn graph_problem(status: &Status) -> Option<ProblemLine> {
    match status {
        Status::Ready | Status::Copied { .. } | Status::Busy(_) | Status::Done(_) => None,
        Status::Failed(message) => Some(ProblemLine {
            reason: sanitize_line(message),
            keys: Some(sanitize_line(DISMISS)),
        }),
        Status::Sticky { reason, keys } => Some(ProblemLine {
            reason: sanitize_line(reason),
            keys: Some(sanitize_line(keys.as_deref().unwrap_or(DISMISS))),
        }),
    }
}

/// How far the walk has got. A failure's text came out of git, so it is
/// sanitised like any other.
fn history_view(history: &History) -> HistoryView {
    HistoryView {
        rows: history.rows,
        state: match &history.phase {
            Phase::Streaming => HistoryState::Streaming,
            Phase::Complete => HistoryState::Complete,
            Phase::Cancelled => HistoryState::Cancelled,
            Phase::Failed(reason) => HistoryState::Failed(sanitize_line(reason)),
        },
    }
}

/// The repository, with every string it carries sanitised on the way.
fn repository_view(facts: &RepositoryFacts) -> RepositoryView {
    RepositoryView {
        path: sanitize_path(&facts.path),
        bare: facts.bare,
        head: match &facts.head {
            // Ref names are the bytes git stored, which are only mostly UTF-8.
            Head::Branch(name) => HeadView::Branch(sanitize_path_bytes(name)),
            Head::Unborn(name) => HeadView::Unborn(sanitize_path_bytes(name)),
            // Hex out of `Oid`, so there is nothing to escape — sanitised
            // anyway, because "this one is safe" is how the one that isn't
            // gets through.
            Head::Detached(id) => {
                let hex = id.to_string();
                HeadView::Detached(sanitize_line(&hex[..ABBREVIATED_ID.min(hex.len())]))
            }
            Head::Unreadable(why) => HeadView::Unreadable(sanitize_line(why)),
        },
    }
}

/// The advisories, in the order they matter: the machine's git first, then the
/// config file.
///
/// A git that is new enough produces no note at all. There is no permanent
/// "git ok" row, because a field that is always there is a field nobody reads
/// (CLAUDE.md).
fn notes(session: &Session, state: &AppState) -> Vec<SanitizedText> {
    let mut notes = Vec::new();

    match &state.system_git {
        // Still running, or fine. Either way there is nothing to say.
        None | Some(GitCheck::Ran(SystemGit::Supported(_))) => {}
        Some(GitCheck::Ran(SystemGit::TooOld { found, minimum })) => notes.push(sanitize_line(
            &format!("git {found} is older than the {minimum} Graft needs"),
        )),
        Some(GitCheck::Unavailable(why)) => {
            notes.push(sanitize_line(&format!("git could not be run: {why}")));
        }
    }

    notes.extend(session.notes.iter().cloned());
    notes
}
