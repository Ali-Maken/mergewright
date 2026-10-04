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
        problem: problem(state.shown_status()),
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
        Status::Ready
        | Status::Copied { .. }
        | Status::Busy(_)
        | Status::Waiting(_)
        | Status::Done { .. } => None,
        Status::Failed(message) => Some(sanitize_line(message)),
        Status::Sticky { reason, keys: None } => Some(sanitize_line(reason)),
        Status::Sticky {
            reason,
            keys: Some(keys),
        } => Some(sanitize_line(&format!("{reason} · {keys}"))),
    }
}

/// The one rendering of a [`Status::Done`]: `<what> ✓`, then ` · <also>`,
/// then ` · <keys>`, each part only when present. The tick says `what`
/// finished, so it goes after `what` and never after what else is true or
/// the key still to press (ticket 41; review m6-final-fixes-74 m3). Every
/// draw site calls this, and so does the resolver's notice row, so none can
/// place the tick apart.
#[must_use]
pub(crate) fn done_line(what: &str, also: Option<&str>, keys: Option<&str>) -> String {
    let mut line = format!("{what} ✓");
    for part in [also, keys].into_iter().flatten() {
        line.push_str(" · ");
        line.push_str(part);
    }
    line
}

/// What a failure line says when it carries no keys of its own. `Esc`
/// really does clear a `Failed`/`Sticky` status (`update::keys::key_event`),
/// and a failure that names no key at all is the one footer ADR-0037 §4
/// rules out — the bar must never be keyless (m5-03 review, Major 2).
pub(crate) const DISMISS: &str = "esc dismiss";

/// The graph footer's reading of [`Status`]: the same problem, but with the
/// reason and keys kept apart so [`graft_ui::graph_screen::hints::fit_status_line`]
/// can cut the reason first and keep the keys whole (ADR-0037 §4,
/// `hint-bar-redesign`), and with [`DISMISS`] standing in whenever the
/// failure carries no keys — the common case (a hook rejected the commit,
/// `$EDITOR` failed, staging refused on a conflict).
#[must_use]
pub(crate) fn graph_problem(status: &Status) -> Option<ProblemLine> {
    match status {
        Status::Ready
        | Status::Copied { .. }
        | Status::Busy(_)
        | Status::Waiting(_)
        | Status::Done { .. } => None,
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

/// The bottom row's problem line while a modal is open: only the modal's
/// own refusal — today the name prompt's (`NamePrompt::refusal`: an empty
/// name, or `⏎` refused while a write runs or git is mid-operation, worded
/// afresh each frame so it goes with its cause — m6-conflicts/78). No keys:
/// the modal's own keys are in its box, and `esc` there cancels the modal
/// rather than dismissing a line.
#[must_use]
pub(crate) fn modal_problem(state: &AppState) -> Option<ProblemLine> {
    let refusal = crate::update::name_prompt::refusal_words(state)?;
    Some(ProblemLine {
        reason: sanitize_line(&refusal),
        keys: None,
    })
}

/// The bottom row's problem line, whoever owns the row: the open modal's
/// own refusal while one stands ([`modal_problem`]), else the status's
/// ([`graph_problem`] of [`AppState::shown_status`]: a failure that arrived
/// under a box and was replaced before the row drew it, else the status —
/// m6-conflicts/78).
#[must_use]
pub(crate) fn bottom_row_problem(state: &AppState) -> Option<ProblemLine> {
    if state.modal_owns_bottom_row() {
        modal_problem(state)
    } else {
        graph_problem(state.shown_status())
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

    notes.extend(
        state
            .config_problems
            .iter()
            .map(|problem| sanitize_line(problem)),
    );
    notes.extend(session.notes.iter().cloned());
    notes
}

#[cfg(test)]
mod tests {
    use super::done_line;

    #[test]
    fn done_line_puts_the_tick_after_what_and_before_keys() {
        assert_eq!(done_line("x", None, None), "x ✓");
        assert_eq!(
            done_line("x", None, Some("c finishes the merge")),
            "x ✓ · c finishes the merge"
        );
    }

    /// Review m6-final-fixes-74 M4: a fact that is not a key (`your edits
    /// are back`) rides in `also`, drawn after the tick and before any key —
    /// never in `keys`, where the key-driven paths replace it.
    #[test]
    fn done_line_places_also_before_keys() {
        assert_eq!(
            done_line("rebase", Some("your edits are back"), None),
            "rebase ✓ · your edits are back"
        );
        assert_eq!(
            done_line("rebase", Some("your edits are back"), Some("u undoes")),
            "rebase ✓ · your edits are back · u undoes"
        );
    }

    /// A wait is not a problem: neither the status screen nor the graph
    /// footer's problem line shows it (review 47-49, m1).
    #[test]
    fn a_wait_is_never_drawn_as_a_problem() {
        let wait = super::Status::Waiting("still reading the refs — try again".to_owned());
        assert_eq!(super::problem(&wait), None);
        assert!(super::graph_problem(&wait).is_none());
    }
}
