//! The end of the loop: `view(state)` (ARCHITECTURE.md).
//!
//! One pure function from what Graft knows to a frame. It reads and draws; it
//! never changes anything, so what is on screen is always a direct function of
//! the state.
//!
//! This is also the boundary the sanitiser guards. Everything here that came
//! out of a repository — a path, a branch name, a failure git wrote — crosses
//! into `ui` as [`SanitizedText`], which only `term` can construct, so a branch
//! called `ma<ESC>[31min` reaches the terminal as the picture of an escape and
//! never as the escape (DESIGN.md §8.2).

use graft_git::version::SystemGit;
use graft_term::{SanitizedText, sanitize_line, sanitize_path, sanitize_path_bytes};
use graft_ui::status_screen::{HeadView, RepositoryView, StatusScreen, StatusView};
use ratatui::Frame;

use crate::repository::{Head, RepositoryFacts};
use crate::session::Session;
use crate::update::{AppState, GitCheck, Status};

/// How much of a commit id a detached `HEAD` shows. Long enough to be unique
/// in any repository a person works in, short enough to read back.
const ABBREVIATED_ID: usize = 12;

/// Draw the current state.
pub fn view(session: &Session, state: &AppState, frame: &mut Frame) {
    let area = frame.area();
    if area.is_empty() {
        // Nothing to draw into. Not an error — a terminal can legitimately be
        // this size for a moment (ADR-0012) — so there is nothing to report.
        return;
    }

    let screen = status_view(session, state);
    frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
}

/// Everything the status screen draws, assembled from the two things `app`
/// knows: what was settled at startup, and what has happened since.
///
/// Separate from [`view`] so it can be asserted on directly — a view model is
/// data, and comparing data beats reading cells out of a buffer.
#[must_use]
pub fn status_view(session: &Session, state: &AppState) -> StatusView {
    StatusView {
        repository: state.repository.as_ref().map(repository_view),
        rendering: session.rendering,
        notes: notes(session, state),
        problem: match &state.status {
            Status::Ready => None,
            Status::Failed(message) => Some(sanitize_line(message)),
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

#[cfg(test)]
mod tests {
    use graft_core::capabilities::RenderingTier;
    use graft_core::oid::Oid;
    use graft_git::version::GitVersion;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::session::for_tests::session;
    use crate::update::Size;

    /// Draw `state` into a terminal of exactly this size and return what the
    /// cells ended up holding.
    fn draw(state: &AppState, columns: u16, rows: u16) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
        terminal
            .draw(|frame| view(&session(), state, frame))
            .expect("TestBackend cannot fail to draw");
        terminal.backend().to_string()
    }

    fn open(head: Head) -> AppState {
        AppState {
            repo_path: std::path::PathBuf::from("/code/api"),
            repository: Some(RepositoryFacts {
                path: std::path::PathBuf::from("/code/api"),
                bare: false,
                head,
            }),
            ..AppState::default()
        }
    }

    #[test]
    fn an_open_repository_reaches_the_screen() {
        let screen = draw(&open(Head::Branch(b"main".to_vec())), 60, 12);

        assert!(screen.contains("/code/api"), "{screen}");
        assert!(screen.contains("main"), "{screen}");
        assert!(screen.contains("unicode"), "{screen}");
    }

    #[test]
    fn the_first_frame_says_it_is_still_opening() {
        let screen = draw(&AppState::default(), 60, 12);

        assert!(screen.contains("opening"), "{screen}");
    }

    #[test]
    fn a_branch_name_with_an_escape_in_it_cannot_reach_the_terminal() {
        // The whole point of the `SanitizedText` boundary, checked end to end
        // rather than only in `ui`.
        let state = open(Head::Branch(b"ma\x1b[31min".to_vec()));

        let screen = draw(&state, 60, 12);

        assert!(!screen.contains('\u{1b}'), "an escape reached the cells");
        assert!(screen.contains('\u{241b}'), "{screen}");
    }

    #[test]
    fn a_detached_head_is_abbreviated() {
        let id = Oid::from_hex("4f1a9c02be7711223344556677889900aabbccdd")
            .expect("forty hex characters");

        let view = status_view(&session(), &open(Head::Detached(id)));

        assert_eq!(
            view.repository.expect("the repository is open").head,
            HeadView::Detached(sanitize_line("4f1a9c02be77"))
        );
    }

    #[test]
    fn a_git_that_is_new_enough_says_nothing_at_all() {
        let state = AppState {
            system_git: Some(GitCheck::Ran(SystemGit::Supported(GitVersion::MINIMUM))),
            ..open(Head::Branch(b"main".to_vec()))
        };

        let view = status_view(&session(), &state);

        assert!(view.notes.is_empty(), "{:?}", view.notes);
        assert_eq!(view.problem, None);
    }

    #[test]
    fn an_old_git_and_a_missing_one_both_get_a_note() {
        let old = AppState {
            system_git: Some(GitCheck::Ran(SystemGit::TooOld {
                found: GitVersion {
                    major: 2,
                    minor: 30,
                    patch: 1,
                },
                minimum: GitVersion::MINIMUM,
            })),
            ..AppState::default()
        };
        let missing = AppState {
            system_git: Some(GitCheck::Unavailable("no git on PATH".to_owned())),
            ..AppState::default()
        };

        let old = status_view(&session(), &old);
        let missing = status_view(&session(), &missing);

        assert_eq!(old.notes.len(), 1);
        assert!(old.notes[0].as_str().contains("2.30.1"), "{:?}", old.notes);
        assert_eq!(missing.notes.len(), 1);
        assert!(
            missing.notes[0].as_str().contains("no git on PATH"),
            "{:?}",
            missing.notes
        );
    }

    #[test]
    fn a_failure_becomes_the_problem_line_and_is_sanitised() {
        let state = AppState {
            status: Status::Failed("job 3 failed: no such \u{1b}[31mref".to_owned()),
            ..AppState::default()
        };

        let view = status_view(&session(), &state);
        let problem = view.problem.expect("a failure must be shown");

        assert!(!problem.as_str().contains('\u{1b}'), "{problem:?}");
        assert!(problem.as_str().contains("no such"), "{problem:?}");
    }

    #[test]
    fn what_the_session_settled_is_what_the_screen_shows() {
        let session = session();

        let view = status_view(&session, &AppState::default());

        assert_eq!(view.rendering.tier, RenderingTier::Unicode);
        assert!(!view.rendering.tier_forced);
    }

    #[test]
    fn a_terminal_with_no_cells_at_all() {
        // ADR-0012's degenerate pty: the size a pty reports before anyone has
        // sized it. The point of the test is that drawing returns at all.
        let mut terminal =
            Terminal::new(TestBackend::new(0, 0)).expect("TestBackend reports its size");

        terminal
            .draw(|frame| view(&session(), &AppState::default(), frame))
            .expect("TestBackend cannot fail to draw");

        assert_eq!(terminal.backend().buffer().area.area(), 0);
    }

    #[test]
    fn a_terminal_two_cells_square() {
        // Far too small for the screen, and still not a panic.
        let screen = draw(&open(Head::Branch(b"main".to_vec())), 2, 2);

        assert_eq!(screen.lines().count(), 2, "{screen}");
    }

    #[test]
    fn drawing_does_not_change_the_state() {
        let state = AppState {
            status: Status::Ready,
            size: Size {
                columns: 40,
                rows: 4,
            },
            ..open(Head::Branch(b"main".to_vec()))
        };
        let before = state.clone();

        let _ = draw(&state, 40, 12);

        assert_eq!(state, before);
    }
}
