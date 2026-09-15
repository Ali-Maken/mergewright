use graft_core::capabilities::RenderingTier;
use graft_core::oid::Oid;
use graft_git::version::{GitVersion, SystemGit};
use graft_term::sanitize_line;
use graft_ui::graph_column::{NodeKind, RowPaint};
use graft_ui::graph_screen::WalkView;
use graft_ui::status_screen::{HeadView, TierSource};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::entries::badge_view;
use super::*;
use crate::history::{History, Phase};
use crate::repository::{Head, RepositoryFacts};
use crate::session::Session;
use crate::session::for_tests::session;
use crate::update::Size;
use crate::update::{AppState, Banner, GitCheck, Harness, Screen, Status};

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
            commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
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
pub(crate) fn history_of(count: u8, cursor: usize) -> History {
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
            promoted: false,
        }),
        rows: usize::from(count),
        phase: Phase::Complete,
        cursor,
        reanchor: None,
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

    let view = graph_view(&state, state.harness.clock.now_seconds());
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

#[test]
fn a_harness_banner_sits_under_the_header_with_the_tier_and_costs_rows() {
    let mut state = AppState {
        history: Some(history_of(5, 1)),
        size: Size {
            columns: 80,
            rows: 6,
        },
        harness: Harness {
            banner: Some(Banner {
                title: String::from("gallery 6/23 · foxtrot-merge · ] next"),
                lines: vec![String::from("shows: a lane jump")],
            }),
            ..Harness::default()
        },
        ..AppState::default()
    };
    assert_eq!(state.graph_rows(), 2, "header, two banner rows, footer");
    let screen = draw(&state, 80, 6);
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[0].contains(" graft"), "{screen}");
    assert!(
        lines[1].contains("gallery 6/23 · foxtrot-merge · ] next · unicode"),
        "the title carries the tier in force: {screen}"
    );
    assert!(lines[2].contains("shows: a lane jump"), "{screen}");
    assert!(
        lines[5].contains("q quit"),
        "the hints keep the footer: {screen}"
    );

    state.renderer_override = Some(RenderingTier::Ascii);
    let screen = draw(&state, 80, 6);
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[1].contains("] next · ascii "), "{screen}");
    assert!(lines[5].contains("renderer: ascii (F3 cycles)"), "{screen}");
}

#[test]
fn the_tier_cycle_key_forces_ascii_and_says_so_in_the_footer() {
    let state = AppState {
        history: Some(history_of(5, 1)),
        size: Size {
            columns: 60,
            rows: 5,
        },
        renderer_override: Some(RenderingTier::Ascii),
        ..AppState::default()
    };

    let view = graph_view(&state, state.harness.clock.now_seconds());
    assert!(
        !view.ascii,
        "graph_view alone does not know the session's tier"
    );

    let screen = draw(&state, 60, 5);
    assert!(screen.contains("renderer: ascii (F3 cycles)"), "{screen}");
    assert!(
        !screen.contains('╭'),
        "the graph column drew Unicode: {screen}"
    );
}

#[test]
fn every_graph_renderer_config_value_draws_as_it_says() {
    use crate::config::{Config, Graph, Loaded, RendererChoice};
    use crate::session::Clock;
    use crate::session::for_tests::capabilities;
    use crate::update::Harness;

    // `history_of` dates its commits `1..=count` seconds since the Unix
    // epoch, so the date column reads decades old; fixed so the
    // snapshot below never drifts with real time (m1-the-graph/53).
    let state = AppState {
        history: Some(history_of(5, 1)),
        size: Size {
            columns: 60,
            rows: 5,
        },
        harness: Harness {
            clock: Clock::Fixed(2_000_000_000),
            ..Harness::default()
        },
        ..AppState::default()
    };

    // `Auto` and `Unicode` both draw Unicode here: the detected terminal
    // is a Unicode one, and `Unicode` asks for exactly that. `Pixel` on
    // this silent terminal attempts Kitty (m1-the-graph/24d): the column
    // is left blank for images, so neither glyph set shows.
    for (renderer, ascii, pixel) in [
        (RendererChoice::Auto, false, false),
        (RendererChoice::Unicode, false, false),
        (RendererChoice::Pixel, false, true),
        (RendererChoice::Ascii, true, false),
    ] {
        let loaded = Loaded {
            config: Config {
                graph: Graph {
                    renderer,
                    ..Graph::default()
                },
                ..Config::default()
            },
            ..Loaded::default()
        };
        let session = Session::settle(&loaded, capabilities());
        let mut terminal =
            Terminal::new(TestBackend::new(60, 5)).expect("TestBackend reports its size");
        terminal
            .draw(|frame| view(&session, &state, frame))
            .expect("TestBackend cannot fail to draw");
        let screen = terminal.backend().to_string();

        insta::assert_snapshot!(format!("renderer-{renderer:?}"), screen);
        assert_eq!(
            screen.contains('●'),
            !ascii && !pixel,
            "{renderer:?}: {screen}"
        );
        assert_eq!(screen.contains('*'), ascii, "{renderer:?}: {screen}");
    }
}

#[test]
fn with_no_override_the_footer_keeps_the_ordinary_hints() {
    let state = AppState {
        history: Some(history_of(5, 1)),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };

    let screen = draw(&state, 60, 5);
    assert!(screen.contains("q quit"), "{screen}");
    assert!(!screen.contains("renderer forced"), "{screen}");
}

/// A history like [`history_of`], but the newest row's subject is
/// `subject` verbatim instead of `"Commit N\x1b[31m"` — for forcing a
/// reveal to actually show something.
fn history_with_subject(count: u8, cursor: usize, subject: &str) -> History {
    use graft_core::commit::{Commit, Identity, Timestamp};
    use graft_graph::layout::layout;
    use graft_graph::seeds::Seeds;

    let oid = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes");
    let commits: Vec<Commit> = (1..=count)
        .rev()
        .enumerate()
        .map(|(position, id)| {
            let who = Identity {
                name: b"Test".to_vec(),
                email: b"test@example.invalid".to_vec(),
                when: Timestamp {
                    seconds: i64::from(id),
                    offset_seconds: 0,
                },
            };
            let message = if position == 0 {
                subject.as_bytes().to_vec()
            } else {
                format!("Commit {id}").into_bytes()
            };
            Commit {
                id: oid(id),
                parents: if id == 1 { vec![] } else { vec![oid(id - 1)] },
                author: who.clone(),
                committer: who,
                message,
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
            promoted: false,
        }),
        rows: usize::from(count),
        phase: Phase::Complete,
        cursor,
        reanchor: None,
        ..History::started()
    }
}

/// An escape sequence in a subject renders inert in the footer reveal
/// (m1-the-graph/17's fixture, ADR-0023): a hover on a row long enough
/// to need the `…` may not let a raw escape byte through in its place.
#[test]
fn a_reveal_renders_an_escape_sequence_inert() {
    let subject = "A very long subject with an escape \x1b[31min it that needs the ellipsis to fit";
    let history = history_with_subject(3, 0, subject);
    let state = AppState {
        hover: Some(0),
        history: Some(history),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };

    assert_eq!(
        graph_view(&state, state.harness.clock.now_seconds()).reveal,
        Some(0),
        "row 0 has arrived"
    );
    let screen = draw(&state, 60, 5);
    assert!(!screen.contains('\u{1b}'), "{screen}");
    assert!(screen.contains("A very long subject"), "{screen}");
}

/// Hover and the keyboard peek both point at the same absolute row, so
/// both routes name the identical row in `reveal` (m1-the-graph/17,
/// ADR-0023) — the acceptance criterion this proves.
#[test]
fn hover_and_peek_name_the_same_reveal_row() {
    let history = history_of(4, 2);
    let base = AppState {
        history: Some(history),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };

    // The keyboard peeks the current selection (cursor).
    let keyboard = AppState {
        peek: true,
        ..base.clone()
    };
    // A hover over that same row sets the identical absolute index.
    let hover = AppState {
        hover: Some(2),
        ..base.clone()
    };

    let from_keyboard = graph_view(&keyboard, keyboard.harness.clock.now_seconds())
        .reveal
        .expect("row 2 has arrived");
    let from_hover = graph_view(&hover, hover.harness.clock.now_seconds())
        .reveal
        .expect("row 2 has arrived");
    assert_eq!(from_keyboard, from_hover);
}

/// Refs whose `HEAD` resolves to `head`, or is unborn when `head` is
/// `None`.
pub(crate) fn refs_with_head(head: Option<Oid>) -> crate::refs::RefBadges {
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

pub(crate) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes")
}

/// `state` with a 3-commit history (newest is `oid(3)`) and `HEAD`
/// attached to the newest commit.
fn state_with_head_and_counts(counts: graft_git::status::WorkingTreeCounts) -> AppState {
    AppState {
        history: Some(history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(3))))),
        working_tree: Some(counts),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    }
}

pub(crate) fn counts(
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

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(view.rows.len(), 3, "no virtual row taken from the budget");
    assert_eq!(view.rows[0].line.row.commit.id, oid(3));
}

#[test]
fn staged_only_names_only_staged() {
    let state = state_with_head_and_counts(counts(3, 0, 0));

    let view = graph_view(&state, state.harness.clock.now_seconds());

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

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(
        view.rows[0].subject.as_str(),
        "Uncommitted changes (2 unstaged)"
    );
}

#[test]
fn untracked_only_names_only_untracked() {
    let state = state_with_head_and_counts(counts(0, 0, 4));

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(
        view.rows[0].subject.as_str(),
        "Uncommitted changes (4 untracked)"
    );
}

#[test]
fn all_three_are_named_together() {
    let state = state_with_head_and_counts(counts(1, 2, 3));

    let view = graph_view(&state, state.harness.clock.now_seconds());

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
    state.refs = Some(std::sync::Arc::new(refs_with_head(Some(oid(1)))));

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(view.rows.len(), 3, "no virtual row");
    assert_eq!(view.rows[0].line.row.commit.id, oid(3));
}

#[test]
fn an_unborn_branch_with_staged_files_still_shows_the_row() {
    // No commits at all yet, so nothing else could be "on top" of it.
    let state = AppState {
        history: Some(history_of(0, 0)),
        refs: Some(std::sync::Arc::new(refs_with_head(None))),
        working_tree: Some(counts(2, 0, 0)),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };

    let view = graph_view(&state, state.harness.clock.now_seconds());

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

    let view = graph_view(&state, state.harness.clock.now_seconds());

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
        graph_view(&state, state.harness.clock.now_seconds()).walk,
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
    let id =
        Oid::from_hex("4f1a9c02be7711223344556677889900aabbccdd").expect("forty hex characters");

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
    let mut terminal = Terminal::new(TestBackend::new(0, 0)).expect("TestBackend reports its size");

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

/// The search line draws the query sanitised: an escape sequence typed or
/// pasted into the prompt, or matched in a subject, is inert on screen.
#[test]
fn a_search_line_with_an_escape_sequence_renders_inert() {
    use crate::search::{Query, Search, SearchState};

    let mut search = Search::new(
        Query::parse("Commit 3\x1b[31m").expect("a query"),
        0,
        (0, 0),
    );
    search.state = SearchState::Done { hits: 1 };
    let state = AppState {
        history: Some(history_of(5, 2)),
        search: Some(search),
        size: Size {
            columns: 80,
            rows: 8,
        },
        ..AppState::default()
    };
    let screen = draw(&state, 80, 8);
    assert!(!screen.contains('\x1b'), "{screen}");
    assert!(screen.contains("1 match"), "{screen}");

    let prompt = AppState {
        search_prompt: Some("x\x1b[2J".to_owned()),
        search: None,
        ..state
    };
    let screen = draw(&prompt, 80, 8);
    assert!(!screen.contains('\x1b'), "{screen}");
    assert!(screen.contains(" /x"), "{screen}");
}
