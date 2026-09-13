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

use graft_git::status::WorkingTreeCounts;
use graft_git::version::SystemGit;
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use graft_term::{SanitizedText, sanitize_line, sanitize_path, sanitize_path_bytes};
use graft_ui::graph_column::{GraphLine, NodeKind, RowPaint};
use graft_ui::graph_screen::{
    AuthorChip, Badge, BadgeKind, Compaction, GraphEntry, GraphScreen, GraphView, SHA_WIDTH,
    Tracking, WalkView, initials, relative_date, short_sha,
};
use graft_ui::status_screen::{
    HeadView, HistoryState, HistoryView, RepositoryView, StatusScreen, StatusView,
};
use ratatui::Frame;

use crate::history::{History, Phase};
use crate::refs::{RefBadges, RefLabel, Upstream};
use crate::repository::{Head, RepositoryFacts};
use crate::session::Session;
use crate::update::{AppState, GitCheck, Screen, Status};

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

    match state.screen {
        Screen::Graph => {
            let mut screen = graph_view(state);
            screen.icons = session.icons;
            screen.compaction.max_lanes = session.max_lanes;
            frame.render_widget(GraphScreen::new(&screen, &session.theme), area);
        }
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
        }
    }
}

/// Everything the graph screen draws: the rows in the viewport, the cursor
/// among them, and where the walk is.
///
/// The rows are borrowed from the window the walk last sent; only the
/// subjects, which came out of the repository, are copied — sanitised.
#[must_use]
pub fn graph_view(state: &AppState) -> GraphView<'_> {
    let location = state
        .repository
        .as_ref()
        .map(|facts| sanitize_path(&facts.path));
    let problem = match &state.status {
        Status::Ready => None,
        Status::Failed(message) => Some(sanitize_line(message)),
    };
    let Some(history) = &state.history else {
        return GraphView {
            location,
            problem,
            walk: WalkView::Opening,
            rows: Vec::new(),
            icons: graft_ui::graph_screen::Icons::default(),
            compaction: Compaction::at(crate::config::DEFAULT_MAX_LANES),
        };
    };

    let walk = match &history.phase {
        Phase::Streaming if history.jump.is_some() => WalkView::Jumping {
            laid_out: history.rows,
        },
        Phase::Streaming => WalkView::Reading {
            laid_out: history.rows,
        },
        Phase::Complete => WalkView::Complete { rows: history.rows },
        Phase::Cancelled => WalkView::Cancelled {
            laid_out: history.rows,
        },
        Phase::Failed(reason) => WalkView::Failed(sanitize_line(reason)),
    };
    let rows = visible_rows(state, history);

    GraphView {
        location,
        problem,
        walk,
        rows,
        icons: graft_ui::graph_screen::Icons::default(),
        compaction: compaction(state, history),
    }
}

/// How the graph folds its lanes: the session's limit is filled in by
/// [`view`], the expansion flag is the user's, and the reserved integration
/// lanes (`1..=n`, after lane 0) are pinned so a fold never hides a lane a
/// branch was promised. The selected commit's ancestry joins the pins once
/// selection exists (ticket 16).
fn compaction(state: &AppState, history: &History) -> Compaction {
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    Compaction {
        max_lanes: crate::config::DEFAULT_MAX_LANES,
        expanded: state.lanes_expanded,
        pinned: (1..=integration).collect(),
    }
}

/// The rows the viewport shows: the uncommitted-changes row, when the
/// working tree is dirty and `HEAD` is the topmost visible commit, then the
/// history rows the walk last sent.
fn visible_rows<'a>(state: &'a AppState, history: &'a History) -> Vec<GraphEntry<'a>> {
    let mailmap = state.repository.as_ref().map(|facts| &facts.mailmap);
    // A clock reading, not repository text — the moment "now" is drawn at,
    // for the relative-date column. `duration_since` only fails when the
    // system clock reads before 1970, in which case every commit is at least
    // that old and `relative_date` reads it the same as any far-past commit.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        });
    // The topmost visible row, independent of viewport height: whether the
    // uncommitted row attaches depends only on what is at the very top, not
    // on how many rows are shown below it.
    let top_row = history.visible(1).first();
    let dirty = state.working_tree.filter(|counts| !counts.is_clean());
    let uncommitted = dirty.filter(|_| head_is_topmost(state.refs.as_ref(), top_row));
    // The virtual row takes one of the viewport's rows when it draws:
    // `History::visible` knows nothing about it.
    let height = if uncommitted.is_some() {
        state.graph_rows().saturating_sub(1)
    } else {
        state.graph_rows()
    };
    uncommitted
        .map(|counts| uncommitted_entry(top_row, counts))
        .into_iter()
        .chain(
            history
                .visible(height)
                .iter()
                .zip(history.top..)
                .map(|(row, index)| {
                    row_entry(
                        row,
                        index == history.cursor,
                        mailmap,
                        state.refs.as_ref(),
                        now,
                    )
                }),
        )
        .collect()
}

/// Is `HEAD` the topmost visible row? The uncommitted row attaches only
/// then (PROMPT.md §1.3): otherwise it would draw beside a commit it is not
/// actually on top of, once the viewport has scrolled.
///
/// `refs` unresolved (the job has not answered yet) reads as "no": showing
/// the row and then possibly retracting it once the answer arrives would
/// flicker, and a clean-looking first frame is the safer guess.
fn head_is_topmost(refs: Option<&RefBadges>, top: Option<&Row>) -> bool {
    let Some(refs) = refs else {
        return false;
    };
    match refs.head() {
        // An unborn branch has no commit for the row to sit on top of, so
        // it is trivially the topmost thing there is.
        None => true,
        Some(head_id) => top.is_some_and(|row| row.commit.id == head_id),
    }
}

/// The uncommitted-changes row's [`GraphEntry`]: `head_row` — `HEAD`'s row,
/// or `None` on an unborn branch — gives it the same lane and lane
/// occupancy `HEAD`'s row has, so the connector line reads as continuing
/// down into it. Author, date and SHA stay blank (DESIGN.md §1.1's mockup):
/// there is no commit behind this row to show them for.
fn uncommitted_entry<'a>(head_row: Option<&Row>, counts: WorkingTreeCounts) -> GraphEntry<'a> {
    let mut line = GraphLine::owned(uncommitted_row(head_row));
    line.node = NodeKind::Uncommitted;
    GraphEntry {
        badges: Vec::new(),
        line,
        subject: sanitize_line(&uncommitted_subject(counts)),
        author: AuthorChip {
            initials: sanitize_line(""),
            identity: BranchIdentity::of_name(b""),
        },
        when: String::new(),
        sha: String::new(),
    }
}

/// A `Row` with no commit behind it, drawn only to give the uncommitted
/// row's node the same connector-line geometry `GraphColumn` gives a real
/// one. Never reaches a ref comparison or a selection target: `view` builds
/// it fresh every frame and nothing else holds it.
fn uncommitted_row(head_row: Option<&Row>) -> Row {
    let blank = graft_core::commit::Identity {
        name: Vec::new(),
        email: Vec::new(),
        when: graft_core::commit::Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    // Never a real object id — all zero bytes, at whatever length this
    // repository's ids are — so it can never collide with a commit a ref or
    // the selection could name.
    let id_len = head_row.map_or(graft_core::oid::Oid::SHA1_BYTES, |row| {
        row.commit.id.as_bytes().len()
    });
    let id = graft_core::oid::Oid::from_bytes(&vec![0u8; id_len])
        .unwrap_or_else(|_| unreachable!("id_len is always a hash length Oid::from_bytes accepts"));
    Row {
        commit: graft_core::commit::Commit {
            id,
            parents: Vec::new(),
            author: blank.clone(),
            committer: blank,
            message: Vec::new(),
        },
        lane: head_row.map_or(0, |row| row.lane),
        incoming: Vec::new(),
        outgoing: head_row.map_or_else(Vec::new, |row| vec![(row.lane, row.commit.id)]),
        lanes: head_row.map_or_else(Vec::new, |row| row.lanes.clone()),
        marker: None,
    }
}

/// The subject text the uncommitted row shows: which of staged, unstaged
/// and untracked are non-zero, and how many of each (DESIGN.md §1.1's
/// mockup: `"Uncommitted changes (3 staged)"`).
fn uncommitted_subject(counts: WorkingTreeCounts) -> String {
    let mut parts = Vec::new();
    if counts.staged > 0 {
        parts.push(format!("{} staged", counts.staged));
    }
    if counts.unstaged > 0 {
        parts.push(format!("{} unstaged", counts.unstaged));
    }
    if counts.untracked > 0 {
        parts.push(format!("{} untracked", counts.untracked));
    }
    format!("Uncommitted changes ({})", parts.join(", "))
}

/// One row's [`GraphEntry`]: the graph cells, the sanitised subject, and the
/// author/date/SHA columns m1-the-graph/10 adds — the author's identity
/// resolved through `mailmap` first, so two identities mapped to one person
/// share a chip and a colour.
fn row_entry<'a>(
    row: &'a graft_graph::row::Row,
    highlighted: bool,
    mailmap: Option<&graft_git::mailmap::Mailmap>,
    refs: Option<&RefBadges>,
    now: i64,
) -> GraphEntry<'a> {
    let mut line = GraphLine::plain(row);
    if let Some(refs) = refs {
        // HEAD wins over the stash and over the parent-derived shapes: it is
        // the one node a person looks for first.
        if refs.head() == Some(row.commit.id) {
            line.node = NodeKind::Head;
        } else if refs.stash() == Some(row.commit.id) {
            line.node = NodeKind::Stash;
        }
    }
    if highlighted {
        line.paint = RowPaint::Highlighted;
    }
    let (name, email) = match mailmap {
        Some(mailmap) => mailmap.resolve(&row.commit.author),
        None => (
            row.commit.author.name.clone(),
            row.commit.author.email.clone(),
        ),
    };
    let badges = refs.map_or_else(Vec::new, |refs| {
        refs.on(row.commit.id).iter().map(badge_view).collect()
    });
    GraphEntry {
        badges,
        line,
        subject: sanitize_line(&String::from_utf8_lossy(row.commit.subject())),
        author: AuthorChip {
            initials: sanitize_line(&initials(&String::from_utf8_lossy(&name))),
            identity: BranchIdentity::of_name(&email),
        },
        when: relative_date(now, row.commit.date().seconds),
        sha: short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH)),
    }
}

/// One ref's badge, its names sanitised on the way into `ui`: ref names are
/// bytes a repository chose, and a branch called `ma<ESC>[31min` must reach
/// the terminal as a picture of the escape.
fn badge_view(label: &RefLabel) -> Badge {
    match label {
        RefLabel::DetachedHead => Badge {
            name: sanitize_line("HEAD"),
            kind: BadgeKind::Head,
        },
        RefLabel::Branch {
            name,
            current,
            upstream,
        } => Badge {
            name: sanitize_path_bytes(name),
            kind: BadgeKind::Branch {
                identity: BranchIdentity::of_name(name),
                current: *current,
                tracking: match upstream {
                    Upstream::None => Tracking::Untracked,
                    Upstream::InSync { remote } => Tracking::InSync {
                        remote: sanitize_path_bytes(remote),
                    },
                    Upstream::Apart(counts) => Tracking::Apart {
                        ahead: counts.ahead,
                        behind: counts.behind,
                    },
                },
            },
        },
        RefLabel::Remote(name) => Badge {
            name: sanitize_path_bytes(name),
            kind: BadgeKind::Remote {
                identity: BranchIdentity::of_name(name),
            },
        },
        RefLabel::Tag(name) => Badge {
            name: sanitize_path_bytes(name),
            kind: BadgeKind::Tag,
        },
        RefLabel::Stash => Badge {
            name: sanitize_line("stash"),
            kind: BadgeKind::Stash,
        },
    }
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

#[cfg(test)]
mod tests {
    use graft_core::capabilities::RenderingTier;
    use graft_core::oid::Oid;
    use graft_git::version::GitVersion;
    use graft_ui::status_screen::TierSource;
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
            screen: Screen::Status,
            repo_path: std::path::PathBuf::from("/code/api"),
            repository: Some(RepositoryFacts {
                path: std::path::PathBuf::from("/code/api"),
                bare: false,
                head,
                mailmap: graft_git::mailmap::Mailmap::default(),
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

    /// A history of `count` commits, newest first, already laid out and
    /// sent as one window.
    fn history_of(count: u8, cursor: usize) -> History {
        use graft_core::commit::{Commit, Identity, Timestamp};
        use graft_graph::layout::layout;
        use graft_graph::seeds::Seeds;

        let oid = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes");
        let commits: Vec<Commit> = (1..=count)
            .rev()
            .map(|id| {
                let who = Identity {
                    name: b"Test".to_vec(),
                    email: b"test@example.invalid".to_vec(),
                    when: Timestamp {
                        seconds: i64::from(id),
                        offset_seconds: 0,
                    },
                };
                Commit {
                    id: oid(id),
                    parents: if id == 1 { vec![] } else { vec![oid(id - 1)] },
                    author: who.clone(),
                    committer: who,
                    message: format!("Commit {id}\x1b[31m").into_bytes(),
                }
            })
            .collect();
        let rows = layout(&commits, Seeds::default());
        History {
            window: Some(crate::history::Window {
                request: 0,
                first: 0,
                laid_out: rows.len(),
                done: true,
                rows: rows.into(),
            }),
            rows: usize::from(count),
            phase: Phase::Complete,
            cursor,
            ..History::started()
        }
    }

    #[test]
    fn graft_opens_on_the_graph_with_the_cursor_row_highlighted() {
        let state = AppState {
            history: Some(history_of(5, 1)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let view = graph_view(&state);
        let screen = draw(&state, 60, 5);

        assert_eq!(view.rows.len(), 3, "rows minus the chrome");
        assert_eq!(view.walk, WalkView::Complete { rows: 5 });
        assert_eq!(view.rows[1].line.paint, RowPaint::Highlighted);
        assert_eq!(view.rows[0].line.paint, RowPaint::Normal);
        assert!(screen.contains("Commit 5"), "{screen}");
        assert!(screen.contains("q quit"), "{screen}");
        assert!(
            !screen.contains('\u{1b}'),
            "a subject's escape reached the cells"
        );
    }

    /// Refs whose `HEAD` resolves to `head`, or is unborn when `head` is
    /// `None`.
    fn refs_with_head(head: Option<Oid>) -> crate::refs::RefBadges {
        let state = match head {
            Some(id) => graft_git::repo::HeadState::Detached(id),
            None => graft_git::repo::HeadState::Unborn(
                graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec())
                    .expect("a valid ref name"),
            ),
        };
        crate::refs::badges(&[], &state, &std::collections::HashMap::new(), |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        })
        .expect("no branches to count")
    }

    fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes")
    }

    /// `state` with a 3-commit history (newest is `oid(3)`) and `HEAD`
    /// attached to the newest commit.
    fn state_with_head_and_counts(counts: graft_git::status::WorkingTreeCounts) -> AppState {
        AppState {
            history: Some(history_of(3, 0)),
            refs: Some(refs_with_head(Some(oid(3)))),
            working_tree: Some(counts),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        }
    }

    fn counts(
        staged: usize,
        unstaged: usize,
        untracked: usize,
    ) -> graft_git::status::WorkingTreeCounts {
        graft_git::status::WorkingTreeCounts {
            staged,
            unstaged,
            untracked,
        }
    }

    #[test]
    fn a_clean_tree_draws_no_uncommitted_row() {
        let state = state_with_head_and_counts(counts(0, 0, 0));

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 3, "no virtual row taken from the budget");
        assert_eq!(view.rows[0].line.row.commit.id, oid(3));
    }

    #[test]
    fn staged_only_names_only_staged() {
        let state = state_with_head_and_counts(counts(3, 0, 0));

        let view = graph_view(&state);

        assert_eq!(view.rows[0].line.node, NodeKind::Uncommitted);
        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (3 staged)"
        );
        assert_eq!(view.rows[0].author.initials.as_str(), "");
        assert_eq!(view.rows[0].when, "");
        assert_eq!(view.rows[0].sha, "");
        // The virtual row plus history rows, one fewer than a full viewport
        // (graph_rows() is 3 here; the row it draws takes one of them).
        assert_eq!(view.rows.len(), 3, "1 virtual row + 2 history rows");
        assert_eq!(view.rows[1].line.row.commit.id, oid(3));
    }

    #[test]
    fn unstaged_only_names_only_unstaged() {
        let state = state_with_head_and_counts(counts(0, 2, 0));

        let view = graph_view(&state);

        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (2 unstaged)"
        );
    }

    #[test]
    fn untracked_only_names_only_untracked() {
        let state = state_with_head_and_counts(counts(0, 0, 4));

        let view = graph_view(&state);

        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (4 untracked)"
        );
    }

    #[test]
    fn all_three_are_named_together() {
        let state = state_with_head_and_counts(counts(1, 2, 3));

        let view = graph_view(&state);

        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (1 staged, 2 unstaged, 3 untracked)"
        );
    }

    #[test]
    fn the_row_is_hidden_when_head_is_not_the_topmost_visible_commit() {
        // HEAD points at a commit that is not row 0 of what is on screen —
        // scrolled down, say. The row must not draw beside the wrong commit.
        let mut state = state_with_head_and_counts(counts(1, 0, 0));
        state.refs = Some(refs_with_head(Some(oid(1))));

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 3, "no virtual row");
        assert_eq!(view.rows[0].line.row.commit.id, oid(3));
    }

    #[test]
    fn an_unborn_branch_with_staged_files_still_shows_the_row() {
        // No commits at all yet, so nothing else could be "on top" of it.
        let state = AppState {
            history: Some(history_of(0, 0)),
            refs: Some(refs_with_head(None)),
            working_tree: Some(counts(2, 0, 0)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].line.node, NodeKind::Uncommitted);
        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (2 staged)"
        );
    }

    #[test]
    fn a_dirty_tree_before_refs_have_loaded_draws_no_row() {
        // Attachment cannot be decided without knowing HEAD; showing the row
        // and then possibly retracting it once refs answer would flicker.
        let state = AppState {
            history: Some(history_of(3, 0)),
            refs: None,
            working_tree: Some(counts(1, 0, 0)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 3);
    }

    #[test]
    fn a_pending_jump_reads_as_one() {
        let mut history = history_of(2, 0);
        history.phase = Phase::Streaming;
        history.jump = Some(4);
        history.rows = 1500;
        let state = AppState {
            history: Some(history),
            ..AppState::default()
        };

        assert_eq!(
            graph_view(&state).walk,
            WalkView::Jumping { laid_out: 1500 }
        );
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
    fn a_branch_name_with_an_escape_sequence_renders_inert() {
        let label = crate::refs::RefLabel::Branch {
            name: b"ma\x1b[31min".to_vec(),
            current: false,
            upstream: crate::refs::Upstream::InSync {
                remote: b"or\x1b]0;pwned\x07igin".to_vec(),
            },
        };

        let text = badge_view(&label).text(graft_ui::graph_screen::Icons::Plain);

        assert!(!text.contains('\x1b'), "{text:?}");
        assert!(!text.contains('\x07'), "{text:?}");
        assert!(
            text.contains("[31min"),
            "the rest of the name is kept: {text:?}"
        );
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
        assert_eq!(view.rendering.tier_source, TierSource::Detected);
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
