use graft_core::capabilities::RenderingTier;
use graft_core::oid::Oid;
use graft_git::version::{GitVersion, SystemGit};
use graft_term::{SanitizedText, sanitize_line};
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

/// Explain's fade (`m2-details-and-diffs/17`) dims every row outside
/// `explain_highlight`, including the cursor row itself — it always wins
/// over the ordinary highlight/Focus-mode paint while open.
#[test]
fn explain_open_fades_rows_outside_its_highlight() {
    let mut state = AppState {
        history: Some(history_of(3, 0)),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };
    state.detail.explain = Some(graft_ui::detail::ExplainView {
        title: sanitize_line("FOXTROT MERGE"),
        severity: graft_core::mistake_radar::Severity::Info,
        paragraphs: Vec::new(),
        sketch: Vec::new(),
    });
    state.detail.explain_highlight = vec![oid(2)];

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(view.rows.len(), 3);
    assert_eq!(
        view.rows[0].line.paint,
        RowPaint::Dimmed,
        "cursor row (oid 3), off the highlight, fades too"
    );
    assert_ne!(
        view.rows[1].line.paint,
        RowPaint::Dimmed,
        "oid 2 is the explanation's own highlight"
    );
    assert_eq!(view.rows[2].line.paint, RowPaint::Dimmed, "oid 1 fades");
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

/// A hover names its row in `reveal` (m1-the-graph/17); `Enter` opens the
/// detail pane now, so the hover is the only reveal (ADR-0023).
#[test]
fn hover_names_the_reveal_row() {
    let history = history_of(4, 2);
    let base = AppState {
        history: Some(history),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };
    let hover = AppState {
        hover: Some(2),
        ..base.clone()
    };
    assert_eq!(
        graph_view(&base, base.harness.clock.now_seconds()).reveal,
        None
    );
    assert!(
        graph_view(&hover, hover.harness.clock.now_seconds())
            .reveal
            .is_some(),
        "row 2 has arrived and is revealed"
    );
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

#[test]
fn on_working_tree_highlights_the_virtual_row_and_not_heads() {
    let mut state = state_with_head_and_counts(counts(1, 0, 0));
    state.on_working_tree = true;

    let view = graph_view(&state, state.harness.clock.now_seconds());

    let highlighted: Vec<_> = view
        .rows
        .iter()
        .filter(|entry| entry.line.paint == RowPaint::Highlighted)
        .collect();
    assert_eq!(
        highlighted.len(),
        1,
        "{:?}",
        view.rows.iter().map(|e| e.line.paint).collect::<Vec<_>>()
    );
    assert_eq!(view.rows[0].line.node, NodeKind::Uncommitted);
    assert_eq!(view.rows[0].line.paint, RowPaint::Highlighted);
}

#[test]
fn a_pinned_compare_base_marks_exactly_its_row_and_fills_the_footer() {
    let history = history_of(5, 1);
    let base_oid = history.row_at(2).expect("row 2 is laid out").commit.id;
    let state = AppState {
        history: Some(history),
        compare_base: Some(crate::update::detail::CompareBase {
            oid: base_oid,
            row: 2,
        }),
        size: Size {
            columns: 60,
            rows: 7,
        },
        ..AppState::default()
    };

    let view = graph_view(&state, state.harness.clock.now_seconds());
    let marked: Vec<usize> = view
        .rows
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.compare_marked)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(marked, vec![2], "only the pinned row is marked");

    let short = graft_ui::graph_screen::short_sha(&base_oid.to_string(), 7);
    assert_eq!(
        view.compare_footer.as_ref().map(SanitizedText::as_str),
        Some(format!("compare base {short} — move, x again clears").as_str())
    );

    let screen = draw(&state, 60, 7);
    assert!(screen.contains("compare base"), "{screen}");
}

/// An opened live compare draws its own header (`COMPARE base → head`), not
/// the cursor's plain-commit header: `detail_view` (m2-details-and-diffs/11)
/// must read the pane's own `DetailState::spec`, since `AppState::selected_spec`
/// never names `Compare` and the two disagree once a base is pinned.
#[test]
fn an_opened_compare_draws_its_own_header_not_the_cursor_commit() {
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};

    let key = |code| AppEvent::Core(Event::Key(KeyPress::plain(code)));
    let state = AppState {
        history: Some(history_of(5, 3)),
        size: Size {
            columns: 140,
            rows: 14,
        },
        ..AppState::default()
    };

    // Cursor starts on row 3; `x` pins it as the base.
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    // Move up two rows onto row 1, then `x` opens the compare and keeps
    // the base pinned.
    let (moved, _) = update(&pinned, key(KeyCode::Char('k')));
    let (moved, _) = update(&moved, key(KeyCode::Char('k')));
    let (opened, _) = update(&moved, key(KeyCode::Char('x')));

    let screen = draw(&opened, 140, 14);
    assert!(
        screen.contains("COMPARE"),
        "the pane's own compare spec, not the cursor's commit: {screen}"
    );
    assert!(!screen.contains("COMMIT"), "{screen}");
}

/// Three synthetic commits — a merge over two parents — with an empty
/// default chain, so the only Mistake Radar finding the capture can show is
/// the evil-merge one.
#[cfg(test)]
fn evil_merge_state() -> AppState {
    use graft_core::commit::{Commit, Identity, Timestamp};
    use graft_graph::layout::layout;
    use graft_graph::seeds::Seeds;

    let who = |seconds| Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    };
    let commit = |id: u8, parents: Vec<Oid>, message: &str| Commit {
        id: oid(id),
        parents,
        author: who(i64::from(id)),
        committer: who(i64::from(id)),
        message: message.as_bytes().to_vec(),
    };
    let commits = vec![
        commit(3, vec![oid(2), oid(1)], "Merge branch 'feature'"),
        commit(2, vec![oid(1)], "Ship the release notes"),
        commit(1, Vec::new(), "Start the feature branch"),
    ];
    let rows = layout(&commits, Seeds::default());
    let history = History {
        window: Some(crate::history::Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            done: true,
            rows: rows.into(),
            promoted: false,
        }),
        rows: 3,
        phase: Phase::Complete,
        cursor: 0,
        reanchor: None,
        ..History::started()
    };
    AppState {
        history: Some(history),
        repo_path: std::path::PathBuf::from("/code/api"),
        size: Size {
            columns: 100,
            rows: 16,
        },
        radar: radar_state_with_empty_default_chain(),
        ..AppState::default()
    }
}

/// A `RadarState` with an empty default chain: no foxtrot or backward
/// finding, so a capture built on it shows only whatever else it sets up
/// (the evil-merge line, in [`evil_merge_state`]).
#[cfg(test)]
fn radar_state_with_empty_default_chain() -> crate::update::radar::RadarState {
    use crate::update::detail::Loading;
    use graft_ops::mistake_radar::Policy;

    crate::update::radar::RadarState {
        context: Loading::Ready(std::sync::Arc::new(
            crate::jobs::radar::RadarContext::of_ids([], Vec::new()),
        )),
        policy: Policy::default(),
        force_push: std::collections::HashMap::new(),
        duplicates: Loading::Idle,
        duplicate_findings: std::collections::HashMap::new(),
        ..Default::default()
    }
}

/// Regenerates `docs/shots/m2-evil-merge.txt` (m2-details-and-diffs/14).
///
/// `#[ignore]`d: it writes a file. Run it by hand with
/// `cargo test -p graft --lib capture_m2_evil_merge -- --ignored`.
#[test]
#[ignore = "writes docs/shots/m2-evil-merge.txt; run by hand to regenerate"]
fn capture_m2_evil_merge() {
    use crate::update::detail::Loading;
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};
    use graft_ops::mistake_radar::probes::EvilVerdict;

    let merge = oid(3);
    let state = evil_merge_state();
    // `Enter` opens the pane on the merge and asks for the probe; the
    // verdict itself is injected, because a capture runs no `git`.
    let (mut opened, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Enter))),
    );
    assert_eq!(opened.detail.evil_at, Some(merge), "the merge is probed");
    assert!(matches!(opened.detail.evil, Loading::Pending(_)));
    opened.detail.evil = Loading::Ready(EvilVerdict::Evil {
        files: vec![b"src/config.rs".to_vec()],
    });

    let screen = draw(&opened, 100, 16);
    assert!(screen.contains("evil merge"), "{screen}");
    let header = "\
# m2-evil-merge.txt: TestBackend capture (not a pty), 100x16. Fixture:
# capture_m2_evil_merge in crates/graft/src/view/tests.rs (#[ignore]d, run
# by hand to regenerate). Three synthetic commits: a merge (0303030) over
# two parents, opened in the detail pane, with the evil-merge probe's
# verdict injected as Evil { files: [\"src/config.rs\"] } — the verdict a
# real `git show --remerge-diff` would give for a resolution the merge
# invented.
#
# Judge by eye:
# - Merge row's marker: a ⚠ beside the merge glyph, and no marker on the
#   two ordinary rows.
# - Detail pane header: the radar line reads \"evil merge\" — one line, the
#   finding. No \"evil merge: unchecked\", \"checking\" or \"clean\" beside it.
# - The pane body reads \"reading…\": no file-list job runs in a capture, so
#   the diff never arrives. Only the header line is what this shot is about.
#
";
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-evil-merge.txt"
        ),
        format!("{header}{screen}"),
    )
    .expect("docs/shots is writable");
}

/// Regenerates `docs/shots/m2-force-push.txt` (m2-details-and-diffs/15).
///
/// `#[ignore]`d: it writes a file. Run it by hand with
/// `cargo test -p graft --lib capture_m2_force_push -- --ignored`.
/// The state `capture_m2_force_push` draws: three synthetic commits, `main`
/// on the middle one tracking `origin/main`, and the force-push verdict a
/// rewritten remote would give — injected, because a capture runs no `git`.
fn force_push_state() -> (AppState, Oid) {
    use crate::update::detail::Loading;
    use graft_core::ref_name::RefName;
    use graft_git::repo::HeadState;
    use graft_ops::mistake_radar::probes::ForcePushVerdict;

    let tip = oid(2);
    let mut state = evil_merge_state();
    // `main` stands on the ordinary commit, not the merge: this shot is
    // about the force-push line, so no evil-merge line shares the header.
    state
        .history
        .as_mut()
        .expect("the fixture has history")
        .cursor = 1;
    let main = RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name");
    let origin_main = RefName::new(b"refs/remotes/origin/main".to_vec()).expect("a valid ref name");
    let mut upstreams = std::collections::HashMap::new();
    upstreams.insert(main.clone(), origin_main.clone());
    let refs = crate::refs::badges(
        &[(main.clone(), tip), (origin_main, oid(9))],
        &HeadState::Branch(main),
        &upstreams,
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 2,
                behind: 2,
            })
        },
    )
    .expect("plain data always builds");
    state.refs = Some(std::sync::Arc::new(refs));
    // The verdict a real `git merge-base --is-ancestor` would give after the
    // remote was rewritten; a capture runs no `git`.
    state.radar.force_push.insert(
        "main".to_owned(),
        Loading::Ready(ForcePushVerdict::Rewritten {
            remote_ref: "origin/main".to_owned(),
            previous: oid(9),
            current: tip,
        }),
    );

    (state, tip)
}

/// Regenerates `docs/shots/m2-duplicate-commit.txt` (m2-details-and-diffs/16).
///
/// `#[ignore]`d: it writes a file. Run it by hand with
/// `cargo test -p graft --lib capture_m2_duplicate_commit -- --ignored`.
/// The state `capture_m2_duplicate_commit` draws: three synthetic commits,
/// with the middle one (0202020) carrying a `DuplicateCommit` finding whose
/// copy is the tip (0303030) — the finding a real duplicates probe would
/// give after finding two commits share a patch-id; injected, because a
/// capture runs no `git`.
fn duplicate_commit_state() -> (AppState, Oid) {
    use graft_core::mistake_radar::{MistakeRadarKind, Severity};
    use graft_ops::mistake_radar::{Evidence, Finding};

    let duplicate = oid(2);
    let mut state = evil_merge_state();
    // The duplicate stands on the ordinary commit, not the merge: this shot
    // is about the duplicate-commit line, so no evil-merge line shares the
    // header.
    state
        .history
        .as_mut()
        .expect("the fixture has history")
        .cursor = 1;
    state.radar.duplicate_findings.insert(
        duplicate,
        Finding {
            kind: MistakeRadarKind::DuplicateCommit,
            severity: Severity::Info,
            at: duplicate,
            evidence: Evidence::Duplicate {
                copies: vec![oid(3)],
            },
        },
    );

    (state, duplicate)
}

#[test]
#[ignore = "writes docs/shots/m2-duplicate-commit.txt; run by hand to regenerate"]
fn capture_m2_duplicate_commit() {
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};

    let (state, _duplicate) = duplicate_commit_state();
    let (opened, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Enter))),
    );
    let screen = draw(&opened, 100, 16);
    assert!(screen.contains("duplicate commit"), "{screen}");
    let header = "\
# m2-duplicate-commit.txt: TestBackend capture (not a pty), 100x16. Fixture:
# capture_m2_duplicate_commit in crates/graft/src/view/tests.rs (#[ignore]d,
# run by hand to regenerate). Three synthetic commits; the middle one
# (0202020) is opened in the detail pane, with a DuplicateCommit finding
# injected as Evidence::Duplicate { copies: [0303030] } — the finding a
# real duplicates probe would give after two commits shared a patch-id.
#
# Judge by eye:
# - The middle row (0202020) carries a ⚠ marker; the other two rows carry
#   none.
# - Detail pane header: the radar line reads \"duplicate commit\".
# - The pane body reads \"reading…\": no file-list job runs in a capture, so
#   the diff never arrives. Only the header line is what this shot is about.
#
";
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-duplicate-commit.txt"
        ),
        format!("{header}{screen}"),
    )
    .expect("docs/shots is writable");
}

#[test]
#[ignore = "writes docs/shots/m2-force-push.txt; run by hand to regenerate"]
fn capture_m2_force_push() {
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};

    let (state, _tip) = force_push_state();
    let (opened, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Enter))),
    );
    let screen = draw(&opened, 100, 16);
    assert!(screen.contains("force-pushed remote"), "{screen}");
    let header = "\
# m2-force-push.txt: TestBackend capture (not a pty), 100x16. Fixture:
# capture_m2_force_push in crates/graft/src/view/tests.rs (#[ignore]d, run
# by hand to regenerate). Three synthetic commits; `main` stands on the
# middle one (0202020), which is opened in the detail pane, with the
# force-push probe's verdict injected as Rewritten { remote_ref:
# \"origin/main\", previous: 0909090, current: 0202020 } — the verdict a real
# `git merge-base --is-ancestor` would give after the remote was rewritten
# under us. `main` tracks `origin/main`, two commits apart each way — what a
# rewritten remote leaves behind.
#
# Judge by eye:
# - The `main` row (0202020) carries a ⚠ marker; the other two rows carry
#   none.
# - Detail pane header: the radar line reads \"force-pushed remote\".
# - No \"force-push: not checkable\" line beside it: a verdict that became a
#   finding never also says the check did not run.
# - The pane body reads \"reading…\": no file-list job runs in a capture, so
#   the diff never arrives. Only the header line is what this shot is about.
#
";
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-force-push.txt"
        ),
        format!("{header}{screen}"),
    )
    .expect("docs/shots is writable");
}

/// The `main` history [`foxtrot_explain_state`] draws: a root, its real
/// tip, and a merge (`0303030`) whose *first* parent is the root, not the
/// tip — with `walk.default_branch` set to `main`, which `repo_facts`
/// needs before `classify_commit` will name anything a foxtrot.
#[cfg(test)]
fn foxtrot_explain_history() -> History {
    use crate::history::WalkFacts;
    use graft_core::commit::{Commit, Identity, Timestamp};
    use graft_core::ref_name::RefName;
    use graft_git::history::Order;
    use graft_graph::layout::layout;
    use graft_graph::seeds::Seeds;

    let who = |seconds| Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    };
    let commit = |id: u8, parents: Vec<Oid>, message: &str| Commit {
        id: oid(id),
        parents,
        author: who(i64::from(id)),
        committer: who(i64::from(id)),
        message: message.as_bytes().to_vec(),
    };
    let commits = vec![
        commit(3, vec![oid(1), oid(2)], "Merge branch 'feature'"),
        commit(2, vec![oid(1)], "Ship the release notes"),
        commit(1, Vec::new(), "Start the feature branch"),
    ];
    let rows = layout(&commits, Seeds::default());
    let mut history = History {
        window: Some(crate::history::Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            done: true,
            rows: rows.into(),
            promoted: false,
        }),
        rows: 3,
        phase: Phase::Complete,
        cursor: 0,
        reanchor: None,
        ..History::started()
    };
    history.walk = Some(WalkFacts {
        requested: Order::Topological,
        order: Order::Topological,
        default_branch: Some(RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name")),
        integration: std::sync::Arc::from([]),
    });
    history
}

/// `foxtrot_explain_history`'s state: `state.radar.context` ready, with a
/// default chain of exactly the merge and its real tip — the two facts
/// `classify_commit`'s `Foxtrot` rule needs beyond `default_branch` itself
/// (`crates/graft-ops/src/mistake_radar/rules.rs`).
#[cfg(test)]
fn foxtrot_explain_state() -> AppState {
    use crate::jobs::radar::RadarContext;
    use crate::update::detail::Loading;
    use crate::update::radar::RadarState;
    use graft_ops::mistake_radar::Policy;

    AppState {
        history: Some(foxtrot_explain_history()),
        repo_path: std::path::PathBuf::from("/code/api"),
        size: Size {
            columns: 100,
            rows: 16,
        },
        radar: RadarState {
            context: Loading::Ready(std::sync::Arc::new(RadarContext::of_ids(
                [oid(3), oid(2)],
                Vec::new(),
            ))),
            policy: Policy::default(),
            force_push: std::collections::HashMap::new(),
            duplicates: Loading::Idle,
            duplicate_findings: std::collections::HashMap::new(),
            ..Default::default()
        },
        ..AppState::default()
    }
}

#[test]
#[ignore = "writes docs/shots/m2-explain-panel.txt; run by hand to regenerate"]
fn capture_m2_explain_panel() {
    use crate::update::detail::{LOCAL_JOB_BIT, open_explain};
    use graft_core::job::JobId;

    let merge = oid(3);
    let mut state = foxtrot_explain_state();
    state.detail.explain_requested = Some(merge);
    let mut counter: u64 = 0;
    let _ = open_explain(&mut state, &mut || {
        counter += 1;
        JobId::new(LOCAL_JOB_BIT | counter)
    });
    assert!(state.detail.explain.is_some(), "a foxtrot finding exists");

    let screen = draw(&state, 160, 24);
    assert!(screen.contains("FOXTROT MERGE"), "{screen}");
    assert!(screen.contains("Esc close"), "{screen}");
    let header = "\
# m2-explain-panel.txt: TestBackend capture (not a pty), 160x24 — wide
# enough for the `Right` breakpoint, so the pane's full height shows the
# sketch and the footer without scrolling. Fixture:
# capture_m2_explain_panel in crates/graft/src/view/tests.rs (#[ignore]d,
# run by hand to regenerate). Three synthetic commits: a root (0101010), its
# real branch tip (0202020), and a merge (0303030) whose first parent is
# the root, not the tip — classify_commit's Foxtrot shape. `open_explain`
# is called directly (as the context-actions-menu's `Explain` choice would
# call it) to open the pane on the merge with the panel already showing.
#
# Judge by eye:
# - The merge row (0303030) carries a warning marker.
# - The pane shows FOXTROT MERGE, its paragraphs, a 5-line sketch (label,
#   connector, label, connector, label) and \"Esc close\", replacing the
#   files+body area.
# - `view::entries`'s Explain fade (every row outside the explanation's
#   highlight dims) is a colour-only effect — this plain-text capture
#   cannot show it; `explain_open_fades_rows_outside_its_highlight`
#   (crates/graft/src/view/tests.rs) is what proves that behaviour.
#
";
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-explain-panel.txt"
        ),
        format!("{header}{screen}"),
    )
    .expect("docs/shots is writable");
}

/// `state.map.shown` (`map-view-screen`, m2-details-and-diffs/19): the Map
/// view replaces the lane graph — no graph rows, no strip column — and the
/// graph path draws exactly as it did before this ticket when the Map view
/// is closed. The detail pane, when open, keeps its own area either way
/// (`detail_pane_keeps_its_area_when_the_map_view_is_open` below).
#[test]
fn map_shown_replaces_the_graph_screen_entirely() {
    let mut state = AppState {
        history: Some(history_of(3, 0)),
        size: Size {
            columns: 60,
            rows: 8,
        },
        ..AppState::default()
    };

    let hidden = draw(&state, 60, 8);
    assert!(!hidden.contains("MAP ·"), "{hidden}");

    state.map.shown = true;
    let shown = draw(&state, 60, 8);
    assert!(shown.contains("MAP ·"), "{shown}");
    assert!(
        !shown.contains("Commit 2"),
        "the graph's rows do not draw under the Map view: {shown}"
    );
}

/// `state.map.shown` with `state.detail.shown` (`map-view-screen`,
/// m2-details-and-diffs/19): the detail pane keeps its own area instead of
/// the Map view taking the whole frame, and the narrow tier's tab strip
/// reads `Map` where it otherwise reads `Graph`.
#[test]
fn detail_pane_keeps_its_area_when_the_map_view_is_open() {
    use graft_ui::detail::DetailFocus;

    let mut state = AppState {
        history: Some(history_of(3, 0)),
        size: Size {
            columns: 90,
            rows: 24,
        },
        ..AppState::default()
    };
    state.map.shown = true;
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;

    // Narrow width (< the `Below` breakpoint): `Tab` layout, so the strip
    // draws and only the focused side's content follows it.
    let shown = draw(&state, 90, 24);
    assert!(
        shown.contains("[Detail]"),
        "the tab strip reads Map beside Detail: {shown}"
    );
    assert!(shown.contains("Map"), "{shown}");
    assert!(
        !shown.contains("MAP ·"),
        "Files focus shows the pane, not the Map body: {shown}"
    );
}

/// `O` (`AppState::strip_shown`, `overview-strip`, m2-details-and-diffs/20):
/// the graph screen loses its last column to the strip, but every other
/// column — including the subject/author/date/SHA columns `Columns::compute`
/// lays out right-to-left from `area.right()` — starts in exactly the same
/// screen column as it does with the strip off. Only the last cell of the
/// last such column (whatever `area.right()`'s rightmost cell held) is
/// sacrificed to the strip's glyph.
// `TestBackend::to_string()` (`buffer_view`) wraps each row in a literal
// `"…"` pair; strip that framing before comparing cell content.
fn row_cells(line: &str) -> &str {
    line.strip_prefix('"')
        .and_then(|line| line.strip_suffix('"'))
        .unwrap_or(line)
}

#[test]
fn strip_narrows_the_graph_column_without_moving_the_trailing_columns() {
    let mut state = AppState {
        history: Some(history_of(20, 0)),
        size: Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };

    let off = draw(&state, 80, 24);
    state.strip_shown = true;
    let on = draw(&state, 80, 24);

    let off_lines: Vec<&str> = off.lines().collect();
    let on_lines: Vec<&str> = on.lines().collect();
    assert_eq!(off_lines.len(), on_lines.len(), "same row count");

    for (row, (off_line, on_line)) in off_lines.iter().zip(&on_lines).enumerate() {
        let off_chars: Vec<char> = row_cells(off_line).chars().collect();
        let on_chars: Vec<char> = row_cells(on_line).chars().collect();
        assert_eq!(
            off_chars.len(),
            on_chars.len(),
            "row {row} is the same width on and off"
        );
        let last = off_chars.len().saturating_sub(1);
        assert_eq!(
            &off_chars[..last],
            &on_chars[..last],
            "row {row}: every column but the last stays put ({off_line:?} vs {on_line:?})"
        );
    }
    assert!(
        on.contains('┆') || on.contains('█') || on.contains('●') || on.contains('◉'),
        "the strip drew a glyph in the freed column: {on}"
    );
}

/// Regenerates `docs/shots/m2-map-view.txt` (m2-details-and-diffs/19).
///
/// `#[ignore]`d: it writes a file. Run it by hand with
/// `cargo test -p graft --lib capture_m2_map_view -- --ignored`.
#[test]
#[ignore = "writes docs/shots/m2-map-view.txt; run by hand to regenerate"]
#[allow(clippy::too_many_lines)] // one hand-built fixture plus its capture header, not logic
fn capture_m2_map_view() {
    use crate::map::{MapBranch, MapData, MapMark};
    use crate::update::detail::Loading;
    use graft_core::mistake_radar::Severity;
    use graft_core::ref_name::RefName;
    use graft_graph::identity::BranchIdentity;

    fn branch(name: &str, byte: u8, current: bool, default: bool, tip_date: i64) -> MapBranch {
        MapBranch {
            name: RefName::new(format!("refs/heads/{name}")).expect("a valid ref name"),
            identity: BranchIdentity::of_name(&[byte]),
            tip: oid(byte),
            tip_date,
            counts: vec![0, 1, 2, 7, 0, 0, 3, 0, 1, 0],
            is_current: current,
            is_default: default,
            merged_into: None,
            marks: Vec::new(),
            complete: true,
        }
    }

    const DAY: i64 = 86_400;
    let mut main = branch("main", 1, true, true, 9 * DAY);
    let mut feature = branch("feature/auth", 2, false, false, 6 * DAY);
    feature.counts = vec![0, 0, 3, 4, 0, 0, 0, 0, 0, 0];
    feature.merged_into = Some((0, 3));
    let mut release = branch("release/2.3", 3, false, false, 9 * DAY);
    release.counts = vec![0, 0, 0, 0, 0, 0, 0, 1, 2, 0];
    release.marks = vec![
        (7, MapMark::Tag(b"v1.0".to_vec())),
        (8, MapMark::Radar(Severity::Warning)),
    ];
    main.counts = vec![0, 1, 2, 7, 0, 0, 3, 0, 1, 4];

    let data = MapData {
        branches: vec![main, feature, release],
        more: 2,
        from: 0,
        to: 10 * DAY,
        buckets: 10,
        complete: false,
    };

    let mut state = AppState {
        size: Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    state.map.shown = true;
    state.map.data = Loading::Ready(std::sync::Arc::new(data));

    let screen = draw(&state, 80, 24);
    assert!(screen.contains("MAP ·"), "{screen}");
    let header = "\
# m2-map-view.txt: TestBackend capture (not a pty), 80x24. Fixture:
# capture_m2_map_view in crates/graft/src/view/tests.rs (#[ignore]d, run by
# hand to regenerate). Three synthetic branches: main (current, tip in the
# newest bucket), feature/auth (merged into main's row, elbow at bucket 3),
# release/2.3 (a v1.0 tag and a Warning radar mark), plus 2 more local
# branches beyond the row cap (footer's \"+2 more\"), and complete: false
# (footer's \" · partial\").
#
# Judge by eye:
# - Header: \"MAP · main · last N days|months\", second line \"BRANCH ... NOW\".
# - main's row: `\u{25b8}` prefix, bar ending in its tallest bucket, trailing ` \u{25cf}`.
# - feature/auth's row: bar ends in an elbow (`\u{25cf}\u{256e}`) at bucket 3.
# - release/2.3's row: a `\u{2534}` mark with `[v1.0]` after the bar, a `\u{26a0}` mark
#   one bucket later.
# - Footer: `+2 more` and ` · partial`, both present.
#
";
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-map-view.txt"
        ),
        format!("{header}{screen}"),
    )
    .expect("docs/shots is writable");
}

/// Regenerates `docs/shots/m2-map-view-loading.txt` (m2-details-and-diffs/19).
///
/// `#[ignore]`d: it writes a file. Run it by hand with
/// `cargo test -p graft --lib capture_m2_map_view_loading -- --ignored`.
#[test]
#[ignore = "writes docs/shots/m2-map-view-loading.txt; run by hand to regenerate"]
fn capture_m2_map_view_loading() {
    let mut state = AppState {
        size: Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    state.map.shown = true;
    state.map.data = crate::update::detail::Loading::Pending(graft_core::job::JobId::new(1));
    state.map.progress = Some((2, 5));

    let screen = draw(&state, 80, 24);
    assert!(screen.contains("building map"), "{screen}");
    let header = "\
# m2-map-view-loading.txt: TestBackend capture (not a pty), 80x24. Fixture:
# capture_m2_map_view_loading in crates/graft/src/view/tests.rs (#[ignore]d,
# run by hand to regenerate). `state.map.progress = Some((2, 5))`, no build
# landed yet.
#
# Judge by eye:
# - Header still draws (\"MAP · ...\").
# - Body: one centred line, \"building map… 2/5 branches\".
# - Footer: \"Esc cancels\", not the usual key hints.
#
";
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-map-view-loading.txt"
        ),
        format!("{header}{screen}"),
    )
    .expect("docs/shots is writable");
}

/// Two feature branches, each merged into `main`, `main`'s tip as `HEAD`:
/// mirrors `graft_fixtures::Shape::FeatureBranches`'s shape without a real
/// checkout (`view::strip::tests::feature_branches_state`'s twin — this
/// module cannot see that private `#[cfg(test)]` item, so it is rebuilt
/// here rather than shared).
fn strip_capture_state() -> AppState {
    use graft_core::commit::{Commit, Identity, Timestamp};
    use graft_graph::layout::layout;
    use graft_graph::seeds::Seeds;

    fn who(seconds: i64) -> Identity {
        Identity {
            name: b"Test".to_vec(),
            email: b"test@example.invalid".to_vec(),
            when: Timestamp {
                seconds,
                offset_seconds: 0,
            },
        }
    }
    fn commit(id: u8, parents: Vec<Oid>, message: &str) -> Commit {
        Commit {
            id: oid(id),
            parents,
            author: who(i64::from(id)),
            committer: who(i64::from(id)),
            message: message.as_bytes().to_vec(),
        }
    }

    let commits = vec![
        commit(9, vec![oid(8), oid(4)], "Merge branch 'topic-b'"),
        commit(8, vec![oid(7)], "topic-b work"),
        commit(7, vec![oid(4)], "topic-b start"),
        commit(4, vec![oid(3), oid(2)], "Merge branch 'topic-a'"),
        commit(3, vec![oid(1)], "topic-a work"),
        commit(2, vec![oid(1)], "topic-a start"),
        commit(1, Vec::new(), "root"),
    ];
    let rows = layout(&commits, Seeds::default());
    let row_count = rows.len();
    let history = History {
        window: Some(crate::history::Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            done: true,
            rows: rows.into(),
            promoted: false,
        }),
        rows: row_count,
        phase: Phase::Complete,
        cursor: 0,
        reanchor: None,
        ..History::started()
    };
    AppState {
        history: Some(history),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(9))))),
        size: Size {
            columns: 80,
            rows: 24,
        },
        strip_shown: true,
        ..AppState::default()
    }
}

/// Regenerates `docs/shots/m2-overview-strip.txt` (m2-details-and-diffs/20).
///
/// `#[ignore]`d: it writes a file. Run it by hand with
/// `cargo test -p graft --lib capture_m2_overview_strip -- --ignored`.
#[test]
#[ignore = "writes docs/shots/m2-overview-strip.txt; run by hand to regenerate"]
fn capture_m2_overview_strip() {
    let state = strip_capture_state();
    let unicode = draw(&state, 80, 24);
    assert!(unicode.contains('●') || unicode.contains('◉'), "{unicode}");

    let mut ascii_state = state.clone();
    ascii_state.renderer_override = Some(RenderingTier::Ascii);
    let ascii = draw(&ascii_state, 80, 24);
    assert!(ascii.contains('*') || ascii.contains('o'), "{ascii}");

    let header = "\
# m2-overview-strip.txt: TestBackend capture (not a pty), 80x24. Fixture:
# capture_m2_overview_strip in crates/graft/src/view/tests.rs (#[ignore]d,
# run by hand to regenerate). Seven synthetic commits mirroring
# graft_fixtures::Shape::FeatureBranches: two feature branches each merged
# into main, main's tip (0909090) is HEAD. `strip_shown = true` (as `O`
# would set it). First block is the Unicode tier, second is ASCII
# (`F3`/`renderer_override`). The strip is bucketed over the whole 24-row
# screen (header and footer included), not just the 7 history rows, so a
# history this short only lights a handful of cells — the rest read as
# `█`/`#` (on screen) or blank (past `rows_total`).
#
# Judge by eye:
# - The graph column is one cell narrower than an ordinary capture; the
#   rightmost screen column carries the strip instead.
# - Row 0 (the header row): `●`/`*` — HEAD's row (0909090, also a merge;
#   HEAD outranks merge in the priority order).
# - Row 10: `◉`/`o` — the other merge (0404040), proportionally mapped
#   further down since 7 rows spread thin over 24 cells.
# - Rows 1-9ish: `█`/`#`, the rows currently on screen.
# - The last couple of rows before the footer: blank — past `rows_total`.
#
";
    let combined = format!("{header}\n=== unicode ===\n{unicode}\n=== ascii ===\n{ascii}");
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-overview-strip.txt"
        ),
        combined,
    )
    .expect("docs/shots is writable");
}
