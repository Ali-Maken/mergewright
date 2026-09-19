//! Building [`StatusView`]: what was settled at startup, and what has
//! happened since.

use graft_git::version::SystemGit;
use graft_term::{SanitizedText, sanitize_line, sanitize_path, sanitize_path_bytes};
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
        problem: match &state.status {
            Status::Ready | Status::Copied { .. } | Status::Busy(_) | Status::Done(_) => None,
            Status::Failed(message) | Status::WriteFailed(message) => Some(sanitize_line(message)),
        },
        history: state.history.as_ref().map(history_view),
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
