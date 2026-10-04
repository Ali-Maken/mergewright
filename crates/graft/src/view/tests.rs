use graft_core::capabilities::RenderingTier;
use graft_core::oid::Oid;
use graft_git::version::{GitVersion, SystemGit};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::graph_column::{NodeKind, RowPaint};
use graft_ui::graph_screen::WalkView;
use graft_ui::status_screen::{HeadView, TierSource};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

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
pub(crate) fn draw(state: &AppState, columns: u16, rows: u16) -> String {
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
    assert!(screen.contains("⏎ open"), "{screen}");
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
    // The header names the repository, never the product (ADR-0058 §4);
    // with none open here, its walk state is what shows it is the header.
    assert!(lines[0].contains("5 commits"), "{screen}");
    assert!(!lines[0].contains("graft"), "{screen}");
    assert!(
        lines[1].contains("gallery 6/23 · foxtrot-merge · ] next · unicode"),
        "the title carries the tier in force: {screen}"
    );
    assert!(lines[2].contains("shows: a lane jump"), "{screen}");
    assert!(
        lines[5].contains("⏎ open"),
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
    assert!(screen.contains("⏎ open"), "{screen}");
    assert!(!screen.contains("renderer forced"), "{screen}");
}

#[test]
fn a_running_write_and_a_finished_one_both_say_so_in_the_footer() {
    // m3/04 review M3: `Status::Busy`/`Done` reached no drawing code at all,
    // so `Space` gave no acknowledgement and a second `Space` looked like a
    // dead key rather than a write already in flight.
    let running = AppState {
        history: Some(history_of(5, 1)),
        status: Status::Busy("stage src/a.rs".to_owned()),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };
    let screen = draw(&running, 60, 5);
    assert!(screen.contains("stage src/a.rs\u{2026}"), "{screen}");

    let done = AppState {
        status: Status::done("stage src/a.rs"),
        ..running
    };
    let screen = draw(&done, 60, 5);
    assert!(screen.contains("stage src/a.rs \u{2713}"), "{screen}");
}

/// A remote write's line ends in its key hint, drawn as the design note
/// words it — no `…` after `esc cancels` (m4-remotes/05).
#[test]
fn a_remote_write_ends_in_its_hint() {
    let running = AppState {
        history: Some(history_of(5, 1)),
        status: Status::Busy("fetch all remotes · receiving 45% · esc cancels".to_owned()),
        size: Size {
            columns: 60,
            rows: 5,
        },
        ..AppState::default()
    };
    let screen = draw(&running, 60, 5);
    assert!(
        screen.contains("fetch all remotes · receiving 45% · esc cancels "),
        "{screen}"
    );
    assert!(!screen.contains("esc cancels\u{2026}"), "{screen}");
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
    crate::refs::badges(
        &[],
        &state,
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no branches to count")
}

pub(crate) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes")
}

/// `state` with a 3-commit history (newest is `oid(3)`) and `HEAD`
/// attached to the newest commit.
fn state_with_head_and_counts(
    counts: std::sync::Arc<graft_git::status::WorkingTreeStatus>,
) -> AppState {
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

/// A synthetic working-tree status with `staged` staged entries, `unstaged`
/// unstaged (tracked) entries and `untracked` untracked entries — each on
/// its own made-up path, distinct so `WorkingTreeStatus::entry` can find any
/// one of them by path in a test that needs to.
pub(crate) fn counts(
    staged: usize,
    unstaged: usize,
    untracked: usize,
) -> std::sync::Arc<graft_git::status::WorkingTreeStatus> {
    counts_with_unmerged(staged, unstaged, untracked, 0)
}

/// [`counts`] plus `unmerged` conflicted paths — a porcelain v2 `u` record,
/// which is in none of staged, unstaged and untracked (m5/21).
pub(crate) fn counts_with_unmerged(
    staged: usize,
    unstaged: usize,
    untracked: usize,
    unmerged: usize,
) -> std::sync::Arc<graft_git::status::WorkingTreeStatus> {
    use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
    let mut entries = Vec::new();
    for i in 0..staged {
        entries.push(StatusEntry {
            path: format!("staged-{i}.txt").into(),
            original_path: None,
            staged: Some(Change::Modified),
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        });
    }
    for i in 0..unstaged {
        entries.push(StatusEntry {
            path: format!("unstaged-{i}.txt").into(),
            original_path: None,
            staged: None,
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        });
    }
    for i in 0..untracked {
        entries.push(StatusEntry {
            path: format!("untracked-{i}.txt").into(),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: true,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        });
    }
    for i in 0..unmerged {
        entries.push(StatusEntry {
            path: format!("unmerged-{i}.txt").into(),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: false,
            unmerged: true,
            unmerged_how: None,
            submodule: None,
        });
    }
    std::sync::Arc::new(WorkingTreeStatus { entries })
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

/// m5/21: an unmerged path is in none of staged, unstaged and untracked, so
/// a tree holding nothing else counted nothing and the row read
/// `Uncommitted changes ()` — empty parentheses at the one moment the
/// working tree most needs describing.
#[test]
fn unmerged_paths_alone_are_named_rather_than_counting_nothing() {
    let state = state_with_head_and_counts(counts_with_unmerged(0, 0, 0, 2));

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(
        view.rows[0].subject.as_str(),
        "Uncommitted changes (2 unmerged)"
    );
}

/// Unmerged leads: during a conflict it is what the user must act on, and
/// the other buckets keep their order behind it (m5/21, ticket item 3).
#[test]
fn unmerged_leads_the_count_and_the_other_buckets_follow() {
    let state = state_with_head_and_counts(counts_with_unmerged(1, 2, 3, 4));

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(
        view.rows[0].subject.as_str(),
        "Uncommitted changes (4 unmerged, 1 staged, 2 unstaged, 3 untracked)"
    );
}

/// Swept over all sixteen combinations of the four buckets: whenever the row
/// draws at all, it names something. The row drawing and the count being
/// empty can never happen together (m5/21).
#[test]
fn the_count_is_never_empty_while_the_row_draws() {
    for bits in 0..16u8 {
        let status = counts_with_unmerged(
            usize::from(bits & 1 != 0),
            usize::from(bits & 2 != 0),
            usize::from(bits & 4 != 0),
            usize::from(bits & 8 != 0),
        );
        let state = state_with_head_and_counts(status);

        let view = graph_view(&state, state.harness.clock.now_seconds());

        let drawn = view.rows[0].line.node == NodeKind::Uncommitted;
        assert_eq!(
            drawn,
            bits != 0,
            "bits {bits}: the row draws exactly when something is uncommitted"
        );
        if drawn {
            assert!(
                !view.rows[0].subject.as_str().contains("()"),
                "bits {bits}: {} counts nothing",
                view.rows[0].subject.as_str()
            );
        }
    }
}

/// m3-working-tree-undo-and-stash/04: the row is pinned at row 0 whenever
/// the tree is dirty. Scrolling away from `HEAD` used to hide the user's own
/// uncommitted work — the one row they can always act on (`KNOWN_GAPS.md`,
/// m2/06b). What `HEAD`'s position decides now is only whether the row hangs
/// off `HEAD`'s lane or stands alone.
#[test]
fn the_row_detaches_rather_than_hiding_when_head_is_not_on_top() {
    let attached = state_with_head_and_counts(counts(1, 0, 0));
    let view = graph_view(&attached, attached.harness.clock.now_seconds());
    assert_eq!(view.rows[0].line.node, NodeKind::Uncommitted);
    assert!(
        !view.rows[0].line.row.outgoing.is_empty(),
        "with HEAD on top the row hangs off HEAD's lane"
    );

    // HEAD points at a commit that is not row 0 of what is on screen —
    // scrolled down, say.
    let mut detached = state_with_head_and_counts(counts(1, 0, 0));
    detached.refs = Some(std::sync::Arc::new(refs_with_head(Some(oid(1)))));

    let view = graph_view(&detached, detached.harness.clock.now_seconds());

    assert_eq!(
        view.rows[0].line.node,
        NodeKind::Uncommitted,
        "the row still draws"
    );
    assert!(
        view.rows[0].line.row.outgoing.is_empty() && view.rows[0].line.row.lanes.is_empty(),
        "and draws alone: there is no lane to join when HEAD is off screen"
    );
    assert_eq!(
        view.rows[1].line.row.commit.id,
        oid(3),
        "the history it displaced follows it"
    );
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
        awaiting: Some(graft_core::job::JobId::new(1)),
        ..AppState::default()
    };

    assert_eq!(
        graph_view(&state, state.harness.clock.now_seconds()).walk,
        WalkView::Jumping { laid_out: 1500 }
    );
}

/// The header says the walk is reading (`Opening`, `Reading`, `Jumping`)
/// exactly when `update::walk_is_reading` says `Esc` would cancel it — one
/// predicate, both readers (m6-conflicts/85 A1). Every walk state the
/// screen can hold, each view against the predicate.
///
/// Not covered: no history and no walk awaited — a walk that failed or was
/// cancelled before `Started` (`KNOWN_GAPS` M6/24) — which still draws
/// `Opening` while `Esc` has nothing to stop.
#[test]
fn the_header_reads_reading_exactly_when_esc_would_cancel() {
    use crate::history::Window;
    let walk_id = graft_core::job::JobId::new(1);
    let with = |awaited: bool, edit: &dyn Fn(&mut History)| {
        let mut history = history_of(3, 0);
        history.phase = Phase::Streaming;
        history.rows = 500;
        edit(&mut history);
        AppState {
            history: Some(history),
            awaiting: awaited.then_some(walk_id),
            ..AppState::default()
        }
    };
    let opening = AppState {
        awaiting: Some(walk_id),
        ..AppState::default()
    };
    let states = vec![
        opening,
        with(true, &|history| history.outstanding = Some(0..40)),
        with(true, &|history| history.window = None::<Window>),
        with(true, &|history| history.jump = Some(4)),
        with(true, &|_| {}),
        with(true, &|history| history.phase = Phase::Complete),
        with(true, &|history| {
            history.phase = Phase::Complete;
            history.outstanding = Some(0..40);
        }),
        with(false, &|history| history.phase = Phase::Cancelled),
        with(false, &|history| {
            history.phase = Phase::Failed("gone".to_owned());
        }),
        with(false, &|history| history.phase = Phase::Complete),
    ];
    let mut seen = Vec::new();
    for state in &states {
        let walk = graph_view(state, 0).walk;
        let reads_reading = matches!(
            walk,
            WalkView::Opening | WalkView::Reading { .. } | WalkView::Jumping { .. }
        );
        assert_eq!(
            reads_reading,
            crate::update::walk_is_reading(state),
            "{walk:?}"
        );
        seen.push(std::mem::discriminant(&walk));
    }
    // Every WalkView the enum has was drawn at least once.
    for walk in [
        WalkView::Opening,
        WalkView::Reading { laid_out: 0 },
        WalkView::Paused { laid_out: 0 },
        WalkView::Jumping { laid_out: 0 },
        WalkView::Complete { rows: 0 },
        WalkView::Cancelled { laid_out: 0 },
        WalkView::Failed(sanitize_line("gone")),
    ] {
        assert!(seen.contains(&std::mem::discriminant(&walk)), "{walk:?}");
    }
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

/// m7-power-tools/47 Q1: with the tree dirty the uncommitted row takes one of
/// the graph's rows, so history gets one fewer. Held `j` past the bottom
/// must keep the cursor on a row the view draws, not one below it.
#[test]
fn held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row() {
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};

    let mut state = AppState {
        history: Some(history_of(60, 0)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(60))))),
        working_tree: Some(counts(1, 0, 0)),
        size: Size {
            columns: 80,
            rows: 20,
        },
        ..AppState::default()
    };
    assert!(state.uncommitted_row_shown());
    for press in 1..=40 {
        state = update(
            &state,
            AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('j')))),
        )
        .0;
        let view = graph_view(&state, state.harness.clock.now_seconds());
        let history = state.history.as_ref().expect("history");
        assert_eq!(history.cursor, press, "`j` moves one row");
        assert!(
            view.rows
                .iter()
                .any(|entry| entry.line.paint == RowPaint::Highlighted),
            "after {press} `j`: cursor {} top {} graph_rows {}, {} rows drawn, none of them the cursor's",
            history.cursor,
            history.top,
            state.graph_rows(),
            view.rows.len(),
        );
    }
}

/// The one row a one-row pane draws: asserts it is the cursor's, and says
/// what kind of row it is.
fn one_row_drawn(state: &AppState) -> NodeKind {
    let view = graph_view(state, state.harness.clock.now_seconds());
    assert_eq!(view.rows.len(), 1, "one row drawn");
    assert_eq!(
        view.rows[0].line.paint,
        RowPaint::Highlighted,
        "the cursor's"
    );
    view.rows[0].line.node
}

/// Ten commits, a dirty tree, and a graph pane one row tall: the uncommitted
/// row exists but is not pinned.
fn one_row_dirty_pane() -> AppState {
    let state = AppState {
        history: Some(history_of(10, 0)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(10))))),
        working_tree: Some(counts(1, 0, 0)),
        // Header and footer leave the graph one row.
        size: Size {
            columns: 120,
            rows: 3,
        },
        ..AppState::default()
    };
    assert_eq!(state.graph_pane_rows(), 1);
    assert!(
        state.uncommitted_row_shown(),
        "the row exists: `k` reaches it"
    );
    assert!(!state.uncommitted_row_pinned(), "but takes no screen row");
    assert_eq!(state.graph_rows(), 1);
    state
}

/// m7-power-tools/48, Decided 4: a one-row graph pane with a dirty tree does
/// not pin the uncommitted row; the one row draws the cursor's row, which is
/// the uncommitted row only while the cursor rests on it.
#[test]
fn a_one_row_pane_draws_the_cursor_row() {
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};

    let key = |state: &AppState, code: char| {
        update(
            state,
            AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(code)))),
        )
        .0
    };
    let drawn = one_row_drawn;
    let mut state = one_row_dirty_pane();
    for press in 1..=5 {
        state = key(&state, 'j');
        let history = state.history.as_ref().expect("history");
        assert_eq!((history.cursor, history.top), (press, press));
        assert_ne!(drawn(&state), NodeKind::Uncommitted, "after {press} `j`");
    }
    for _ in 0..5 {
        state = key(&state, 'k');
    }
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(0));
    assert_ne!(drawn(&state), NodeKind::Uncommitted);

    state = key(&state, 'k');
    assert!(
        state.on_working_tree,
        "`k` at the top reaches the working tree"
    );
    assert_eq!(drawn(&state), NodeKind::Uncommitted, "drawn in the one row");
    assert_eq!(
        state.history.as_ref().map(|h| h.top),
        Some(0),
        "top untouched"
    );

    state = key(&state, 'j');
    assert!(!state.on_working_tree);
    let history = state.history.as_ref().expect("history");
    assert_eq!(
        (history.cursor, history.top),
        (0, 0),
        "back on the same row"
    );
    assert_ne!(drawn(&state), NodeKind::Uncommitted);
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

/// An opened live compare draws its own header (`comparing head with base`), not
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
        // m7-14: the compare's words; its file list has not landed here.
        screen.contains("comparing"),
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
        // Read, with no stash: the evil probe waits for the refs (m6/66).
        refs: Some(std::sync::Arc::new(crate::refs::RefBadges::default())),
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

/// M2 review round 2, Major 3: the `⚠` on the graph row does not need the
/// pane. Round 1a made the probe follow the selection with the pane closed;
/// this is the other half — that the marker then paints on the merge's own
/// row, and on no other. TESTING.md's graph stop 8 asserts exactly this, so
/// it must rest on something observed rather than on a `Command` being
/// emitted.
#[test]
fn the_evil_marker_paints_on_the_graph_row_with_the_pane_closed() {
    use crate::update::detail::Loading;
    use graft_ops::mistake_radar::probes::EvilVerdict;

    let mut state = evil_merge_state();
    assert!(!state.detail.shown, "the pane starts closed");
    state.detail.evil_at = Some(oid(3));
    state.detail.evil = Loading::Ready(EvilVerdict::Evil {
        files: vec![b"evil.txt".to_vec()],
    });

    let screen = draw(&state, 100, 16);
    let row = |subject: &str| {
        screen
            .lines()
            .find(|line| line.contains(subject))
            .unwrap_or_else(|| panic!("no row for {subject}:\n{screen}"))
            .to_owned()
    };
    assert!(
        row("Merge branch 'feature'").contains('⚠'),
        "the merge row carries no marker:\n{screen}"
    );
    assert!(
        !row("Ship the release notes").contains('⚠'),
        "an ordinary row carries a marker:\n{screen}"
    );
}

/// Regenerates `docs/shots/m2-evil-merge.txt` (m2-details-and-diffs/14).
///
/// Asserted on every run; regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --lib capture_m2_evil_merge`.
#[test]
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
# capture_m2_evil_merge in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). Three synthetic commits: a merge (0303030) over
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
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-evil-merge.txt"
        ),
        format!("{header}{screen}"),
    );
}

/// Regenerates `docs/shots/m2-force-push.txt` (m2-details-and-diffs/15).
///
/// Asserted on every run; regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --lib capture_m2_force_push`.
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
        &[],
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
        Loading::Ready(crate::update::radar::Probed {
            tip,
            verdict: ForcePushVerdict::Rewritten {
                remote_ref: "origin/main".to_owned(),
                previous: oid(9),
                current: tip,
            },
        }),
    );

    (state, tip)
}

/// Regenerates `docs/shots/m2-duplicate-commit.txt` (m2-details-and-diffs/16).
///
/// Asserted on every run; regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --lib capture_m2_duplicate_commit`.
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
# capture_m2_duplicate_commit in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). Three synthetic commits; the middle one
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
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-duplicate-commit.txt"
        ),
        format!("{header}{screen}"),
    );
}

#[test]
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
# capture_m2_force_push in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). Three synthetic commits; `main` stands on the
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
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-force-push.txt"
        ),
        format!("{header}{screen}"),
    );
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
# capture_m2_explain_panel in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). Three synthetic commits: a root (0101010), its
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
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-explain-panel.txt"
        ),
        format!("{header}{screen}"),
    );
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
/// Asserted on every run; regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --lib capture_m2_map_view`.
#[test]
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
# capture_m2_map_view in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). Three synthetic branches: main (current, tip in
# the
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
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-map-view.txt"
        ),
        format!("{header}{screen}"),
    );
}

/// Regenerates `docs/shots/m2-map-view-loading.txt` (m2-details-and-diffs/19).
///
/// Asserted on every run; regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --lib capture_m2_map_view_loading`.
#[test]
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
# capture_m2_map_view_loading in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). `state.map.progress = Some((2, 5))`, no build
# landed yet.
#
# Judge by eye:
# - Header still draws (\"MAP · ...\").
# - Body: one centred line, \"building map… 2/5 branches\".
# - Footer: \"Esc cancels\", not the usual key hints.
#
";
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-map-view-loading.txt"
        ),
        format!("{header}{screen}"),
    );
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
/// Asserted on every run; regenerate on purpose with
/// `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --lib capture_m2_overview_strip`.
#[test]
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
# capture_m2_overview_strip in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). Seven synthetic commits mirroring
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
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m2-overview-strip.txt"
        ),
        combined,
    );
}

// m3-working-tree-undo-and-stash/04: the changes pane's captures. The user
// judges these by eye; no test here ticks "looks right", and the pipeline
// records no colour, so nothing in them can stand for a colour.

/// One diff line, as `git diff` would have parsed it.
fn diff_line(
    kind: graft_git::diff::LineKind,
    text: &str,
    old_no: Option<u32>,
    new_no: Option<u32>,
) -> graft_git::diff::DiffLine {
    graft_git::diff::DiffLine {
        kind,
        text: text.as_bytes().to_vec(),
        old_no,
        new_no,
        no_newline_at_eof: false,
    }
}

/// `src/api/routes.rs`'s two hunks.
fn routes_hunks() -> Vec<graft_git::diff::Hunk> {
    vec![routes_first_hunk(), routes_second_hunk()]
}

fn routes_first_hunk() -> graft_git::diff::Hunk {
    use graft_git::diff::{Hunk, LineKind};
    let line = diff_line;
    Hunk {
        old_start: 18,
        old_lines: 5,
        new_start: 18,
        new_lines: 6,
        heading: b"fn routes()".to_vec(),
        lines: vec![
            line(
                LineKind::Context,
                "    let mut app = Router::new();",
                Some(18),
                Some(18),
            ),
            line(
                LineKind::Removed,
                "    app.get(\"/health\", health);",
                Some(19),
                None,
            ),
            line(
                LineKind::Added,
                "    app.get(\"/health\", health);",
                None,
                Some(19),
            ),
            line(
                LineKind::Added,
                "    app.get(\"/ready\", ready);",
                None,
                Some(20),
            ),
            line(LineKind::Context, "    app", Some(20), Some(21)),
            line(LineKind::Context, "}", Some(21), Some(22)),
        ],
    }
}

fn routes_second_hunk() -> graft_git::diff::Hunk {
    use graft_git::diff::{Hunk, LineKind};
    let line = diff_line;
    Hunk {
        old_start: 44,
        old_lines: 4,
        new_start: 45,
        new_lines: 4,
        heading: b"fn ready()".to_vec(),
        lines: vec![
            line(
                LineKind::Context,
                "async fn ready() -> Json<Status> {",
                Some(44),
                Some(45),
            ),
            line(LineKind::Removed, "    Json(Status::Ok)", Some(45), None),
            line(
                LineKind::Added,
                "    Json(Status::from(&state))",
                None,
                Some(46),
            ),
            line(LineKind::Context, "}", Some(46), Some(47)),
        ],
    }
}

/// One file's diff, made by hand so the capture never depends on `git`.
fn routes_diff() -> graft_git::diff::FileDiff {
    use graft_git::diff::{FileDiff, FileEntry, FileStatus};
    FileDiff {
        entry: FileEntry {
            path: b"src/api/routes.rs".to_vec(),
            old_path: None,
            status: FileStatus::Modified,
            added: Some(3),
            removed: Some(2),
        },
        hunks: routes_hunks(),
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    }
}

/// The changes pane on a scripted dirty tree: one staged file, two unstaged,
/// one untracked, the diff region focused with the line cursor on the added
/// line of the first hunk.
fn scripted_file_lists() -> (graft_git::diff::FileList, graft_git::diff::FileList) {
    use graft_git::diff::{FileEntry, FileList, FileStatus};
    let file = |path: &str, status: FileStatus, added: u32, removed: u32| FileEntry {
        path: path.as_bytes().to_vec(),
        old_path: None,
        status,
        added: Some(added),
        removed: Some(removed),
    };
    (
        FileList {
            files: vec![file("src/auth/token.rs", FileStatus::Modified, 12, 4)],
            added: 12,
            removed: 4,
        },
        FileList {
            files: vec![
                file("src/api/routes.rs", FileStatus::Modified, 3, 2),
                file("src/auth/token.rs", FileStatus::Modified, 2, 0),
                file("notes.md", FileStatus::Untracked, 0, 0),
            ],
            added: 5,
            removed: 2,
        },
    )
}

/// The same tree as [`scripted_file_lists`], as `git status` reports it:
/// `src/auth/token.rs` is staged *and* edited again, so it has a row under
/// two sections.
fn scripted_status() -> graft_git::status::WorkingTreeStatus {
    use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
    let entry = |path: &str, staged: Option<Change>, unstaged: Option<Change>, untracked: bool| {
        StatusEntry {
            path: path.into(),
            original_path: None,
            staged,
            unstaged,
            untracked,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }
    };
    WorkingTreeStatus {
        entries: vec![
            entry("notes.md", None, None, true),
            entry("src/api/routes.rs", None, Some(Change::Modified), false),
            entry(
                "src/auth/token.rs",
                Some(Change::Modified),
                Some(Change::Modified),
                false,
            ),
        ],
    }
}

fn changes_pane_state(columns: u16, rows: u16) -> AppState {
    use graft_ui::detail::DetailFocus;

    let (staged, unstaged) = scripted_file_lists();
    let (view, list) = crate::jobs::detail::assemble_working_tree(
        &staged,
        &unstaged,
        &crate::jobs::detail::UnmergedShapes::new(),
    );
    let status = scripted_status();

    let mut state = AppState {
        history: Some(history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(3))))),
        working_tree: Some(std::sync::Arc::new(status)),
        on_working_tree: true,
        size: Size { columns, rows },
        ..AppState::default()
    };
    state.detail.shown = true;
    state.detail.spec = Some(crate::update::detail::DetailSpec::WorkingTree);
    state.detail.focus = DetailFocus::Diff;
    // The cursor sits on `src/api/routes.rs`, under UNSTAGED — the file
    // whose diff the body shows. Found by name rather than counted, because
    // the list nests each path under its directories.
    let mut view = view;
    view.cursor = view
        .rows
        .iter()
        .position(|row| row.name.as_str() == "routes.rs")
        .expect("routes.rs has a row");
    state.detail.files = crate::update::detail::Loading::Ready(std::sync::Arc::new(view));
    state.detail.list = Some(std::sync::Arc::new(list));
    let diff = routes_diff();
    state.detail.diff = crate::update::detail::Loading::Ready(std::sync::Arc::new(
        crate::jobs::detail::assemble_diff(&diff, false),
    ));
    state.detail.diff_raw = Some(std::sync::Arc::new(diff));
    // Body row 3 is the second `+` line of the first hunk.
    state.detail.body_cursor = 3;
    state
}

const CHANGES_PANE_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_changes_pane in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). A scripted dirty tree: src/auth/token.rs staged
# and
# edited again, src/api/routes.rs edited, notes.md untracked. The diff region
# has focus and the line cursor is on the second added line of hunk 1.
#
# Judge by eye:
# - Three section rows, STAGED (1) / UNSTAGED (2) / UNTRACKED (1), each with
#   its files indented under it. src/auth/token.rs appears twice, once under
#   STAGED and once under UNSTAGED: that is the point, not a duplicate.
#   **At 80x24 the file list has six rows and stops at UNSTAGED's
#   `▾ src/` / `▾ api/`**: routes.rs, token.rs's second row and the whole
#   UNTRACKED section are below the fold — scroll the list to judge them at
#   that size. All three sections are on screen at 120x40 and 160x48.
# - notes.md is a row at all (below the fold at 80x24, see above). Before
#   this milestone untracked files were invisible in this pane.
# - The line cursor: a ▎ in the sign column of one diff row, and that row
#   alone. It is what Space and d act on.
# - The footer offers `space stage` and `d discard`.
#
";

/// The confirm box is modal: while it is open nothing behind it answers a
/// key. A box the user cannot see would be a lock-up, so this checks it is
/// actually painted — the widget and the state existed a commit before the
/// draw did.
#[test]
fn the_confirm_box_is_drawn_over_everything_else() {
    let mut state = changes_pane_state(120, 40);
    assert!(crate::update::operate::request_discard_at_diff_cursor(
        &mut state
    ));
    let screen = draw(&state, 120, 40);
    assert!(
        screen.contains("Discard this line of src/api/routes.rs?"),
        "the question is on screen: {screen}"
    );
    assert!(
        screen.contains("git apply -R -"),
        "the exact command is on screen: {screen}"
    );
    assert!(
        screen.contains("[ Cancel ]"),
        "the safe choice is the one under the cursor: {screen}"
    );
}

/// Regenerates the changes pane's three captures
/// (m3-working-tree-undo-and-stash/04).
#[test]
fn capture_m3_changes_pane() {
    let mut shots = graft_fixtures::Captures::new();
    for (columns, rows) in [(160_u16, 48_u16), (120, 40), (80, 24)] {
        let state = changes_pane_state(columns, rows);
        let screen = draw(&state, columns, rows);
        assert!(screen.contains("STAGED"), "{screen}");
        shots.check(
            format!(
                "{}/../../docs/shots/m3-changes-pane-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            format!("{CHANGES_PANE_HEADER}=== {columns}x{rows} ===\n{screen}"),
        );
    }
    shots.finish();
}

/// Regenerates `docs/shots/m3-confirm-discard.txt`
/// (m3-working-tree-undo-and-stash/04).
#[test]
fn capture_m3_confirm_discard() {
    let mut state = changes_pane_state(120, 40);
    assert!(
        crate::update::operate::request_discard_at_diff_cursor(&mut state),
        "the box opens on the hunk under the cursor"
    );
    let screen = draw(&state, 120, 40);
    let header = "\
# m3-confirm-discard.txt: TestBackend capture (not a pty), 120x40, no colour
# recorded. Fixture: capture_m3_confirm_discard in
# crates/graft/src/view/tests.rs (asserted; regenerate on purpose).
# `d` pressed with the diff region focused and the line cursor on an added
# line of src/api/routes.rs.
#
# Judge by eye:
# - The box says what goes, in plain words, and counts the lines.
# - It shows the exact git command it will run.
# - `[ Cancel ]` — the safe choice — is the one under the cursor, in
#   brackets; `Discard` beside it is not.
# - The pane behind it is still readable; the box does not cover the file it
#   is asking about.
# - The hint bar leads with `y discard` (m6-conflicts/50): `y` takes
#   `[ Discard ]` from wherever the cursor is.
#
";
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-confirm-discard.txt"
        ),
        format!("{header}{screen}"),
    );
}

/// The confirm box's hint bar at `columns` × `rows`: the row that draws
/// `⏎ choose`, trimmed of `TestBackend`'s quotes and the bar's margin.
fn box_bar(state: &AppState, columns: u16, rows: u16) -> String {
    let screen = draw(state, columns, rows);
    let line = screen
        .lines()
        .find(|line| line.contains("⏎ choose"))
        .unwrap_or_else(|| panic!("no box bar drawn:\n{screen}"));
    line.trim_matches('"').trim().to_owned()
}

/// m6-conflicts/50: at 80×24 the bar under the push box and the discard
/// box still leads with the whole `y <button>` hint (ADR-0044 §3.1) — the
/// narrowest screen the user works at keeps the key that confirms.
#[test]
fn a_narrow_bar_keeps_the_y_hint() {
    use crate::update::confirm::{ConfirmThen, PendingConfirm};
    use graft_ops::operation::{Operation, PushParams};

    let mut discard = changes_pane_state(80, 24);
    assert!(
        crate::update::operate::request_discard_at_diff_cursor(&mut discard),
        "the discard box opens"
    );
    assert_eq!(
        box_bar(&discard, 80, 24),
        "y discard  ←/→ move  ⏎ choose  esc cancel"
    );

    let params = PushParams {
        branch: "refs/heads/main".into(),
        remote: "origin".into(),
        remote_branch: "refs/heads/main".into(),
        set_upstream: false,
        lease: None,
        source_commit: None,
    };
    let mut push = changes_pane_state(80, 24);
    push.confirm = Some(PendingConfirm {
        view: crate::update::remote::push_box(&params, Some(1), None, None),
        then: ConfirmThen::operate(Operation::Push(params)),
    });
    assert_eq!(
        box_bar(&push, 80, 24),
        "y push  ←/→ move  ⏎ choose  esc cancel"
    );
}

/// Regenerates `docs/shots/m3-confirm-delete-untracked.txt`: the whole-file
/// discard box, whose consequence line is now per class (m3/04 review M6).
#[test]
fn capture_m3_confirm_delete_untracked() {
    use crate::update::detail::Loading;
    use graft_ui::detail::DetailFocus;

    let mut state = changes_pane_state(120, 40);
    state.detail.focus = DetailFocus::Files;
    let Loading::Ready(files) = &state.detail.files else {
        panic!("the fixture's list is ready");
    };
    let mut view = (**files).clone();
    view.cursor = view
        .rows
        .iter()
        .position(|row| row.name.as_str() == "notes.md")
        .expect("notes.md has a row");
    state.detail.files = Loading::Ready(std::sync::Arc::new(view));
    assert!(
        crate::update::operate::request_discard(&mut state),
        "the box opens on the untracked file under the cursor"
    );
    let screen = draw(&state, 120, 40);
    let header = "\
# m3-confirm-delete-untracked.txt: TestBackend capture (not a pty), 120x40,
# no colour recorded. Fixture: capture_m3_confirm_delete_untracked in
# crates/graft/src/view/tests.rs (asserted; regenerate on purpose).
# `d` pressed with the files list focused and the cursor on notes.md, which
# is untracked.
#
# Judge by eye:
# - The box uses the word `Delete`, in the question and in the consequence:
#   `git clean -f` removes the file, it does not just undo an edit.
# - It says a backup is kept and that `u` undoes it.
# - It shows the exact git command it will run.
# - `[ Cancel ]` — the safe choice — is the one under the cursor, in
#   brackets; `Delete` beside it is not.
# - The hint bar leads with `y delete`, the button's own word
#   (m6-conflicts/50).
#
";
    assert!(screen.contains("y delete  ←/→ move"), "{screen}");
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-confirm-delete-untracked.txt"
        ),
        format!("{header}{screen}"),
    );
}

/// The state the commit editor's captures draw: the changes pane's scripted
/// dirty tree, with `c` pressed and a message half typed
/// (m3-working-tree-undo-and-stash/05).
fn commit_editor_state(columns: u16, rows: u16) -> AppState {
    use crate::update::{AppEvent, update};
    use graft_core::event::{Event, KeyCode, KeyPress};

    let mut state = changes_pane_state(columns, rows);
    // A branch for the title row: the count and the branch are the two facts
    // the title carries, and a fixture without one would capture half of it.
    state.repository = Some(RepositoryFacts {
        path: std::path::PathBuf::from("/code/api"),
        bare: false,
        head: Head::Branch(b"main".to_vec()),
        mailmap: graft_git::mailmap::Mailmap::default(),
        commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
    });
    // The detail pane stays open behind the editor, which is the point of
    // hard choice 4 in the design note.
    let (mut state, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('c')))),
    );
    for character in "fix the token refresh race".chars() {
        let (next, _) = update(
            &state,
            AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(character)))),
        );
        state = next;
    }
    let (mut state, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Tab))),
    );
    for character in "The old token was refreshed twice when two requests raced.".chars() {
        let (next, _) = update(
            &state,
            AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(character)))),
        );
        state = next;
    }
    state
}

const COMMIT_EDITOR_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_commit_editor in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). `c` on the scripted dirty tree, a subject typed,
# then Tab to the body and a sentence typed there.
#
# Judge by eye:
# - The title row reads COMMIT ── ⎇ main ── 1 file staged. Not \"looks
#   right\": the count must match the STAGED section of the changes-pane
#   capture beside it.
# - The subject row shows ▸Subject, the typed text, and a counter reading
#   26/50. The ┊ guide is at column 50 of the text, not at the screen edge.
# - The body row shows Body (no ▸: focus is on the body's text, and the
#   marker follows the focus) with its own ┊ guide at 72.
# - The options row: four [ ] boxes — amend, sign-off, no-verify, allow
#   empty — then [ Commit ]. None is ticked and none is refused, so no
#   reason follows the button.
# - The footer offers ⌃⏎/Commit, ⇥, ⌃E, ⌃S and esc.
";

/// The pane line, which is the one thing that differs between the two sizes.
/// One header claimed both readings at once before (m3/05 review, Minor 4).
fn commit_editor_pane_note(columns: u16) -> &'static str {
    if columns < 100 {
        "\
# - The editor has the whole area and the detail pane is not drawn. Under
#   100 columns the pane would be a tab, and the editor is modal, so the key
#   that selects the other tab never reaches it — an editor behind a tab
#   nobody can press is an editor that did not open.
#
"
    } else {
        "\
# - The detail pane is still there beside the editor, with its own header.
#   The editor replaced the graph, not the pane.
#
"
    }
}

/// Regenerates the commit editor's two captures
/// (m3-working-tree-undo-and-stash/05).
#[test]
fn capture_m3_commit_editor() {
    let mut shots = graft_fixtures::Captures::new();
    for (columns, rows) in [(160_u16, 48_u16), (80, 24)] {
        let state = commit_editor_state(columns, rows);
        let screen = draw(&state, columns, rows);
        assert!(screen.contains("COMMIT"), "{screen}");
        shots.check(
            format!(
                "{}/../../docs/shots/m3-commit-editor-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            format!(
                "{COMMIT_EDITOR_HEADER}{}=== {columns}x{rows} ===\n{screen}",
                commit_editor_pane_note(columns)
            ),
        );
    }
    shots.finish();
}

/// Regenerates `docs/shots/m3-commit-editor-rejected.txt` — a pre-commit
/// hook's refusal, drawn on the editor rather than on the graph behind it
/// (m3/05 review, Major 1).
#[test]
fn capture_m3_commit_editor_rejected() {
    let mut state = commit_editor_state(120, 40);
    state.status = crate::update::Status::sticky(
        "pre-commit: 2 files would be reformatted, run `cargo fmt`".to_owned(),
    );
    let screen = draw(&state, 120, 40);
    assert!(screen.contains("pre-commit"), "{screen}");
    let header = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_commit_editor_rejected in crates/graft/src/view/tests.rs
# (asserted). The same half-typed editor, after a pre-commit hook refused
# the commit.
#
# Judge by eye:
# - The hook's own words are on the footer row, in place of the key hints,
#   followed by `— esc dismisses`. Before this the reason went to the graph
#   screen's problem line, which is behind the editor: a rejected commit
#   looked to the user like nothing had happened at all.
# - The typed subject and body are still there. Esc dismisses the reason and
#   leaves the editor open; a second Esc closes it and keeps the draft.
# - The reason is drawn in the warning colour, which this capture cannot
#   record. The words are what has to carry it, and they do.
#
";
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-commit-editor-rejected.txt"
        ),
        format!("{header}=== 120x40 ===\n{screen}"),
    );
}

/// Regenerates `docs/shots/m3-commit-editor-refused.txt`
/// (m3-working-tree-undo-and-stash/05).
#[test]
fn capture_m3_commit_editor_refused() {
    let mut state = commit_editor_state(120, 40);
    // Nothing staged: the same editor, with the button dimmed and the reason
    // on its own row above it (m6-conflicts/40). The dimming is colour, which a capture cannot record — the
    // reason in characters is what makes it legible here.
    state.working_tree = Some(std::sync::Arc::new(
        graft_git::status::WorkingTreeStatus::default(),
    ));
    let screen = draw(&state, 120, 40);
    assert!(screen.contains("nothing staged"), "{screen}");
    let header = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_commit_editor_refused in crates/graft/src/view/tests.rs
# (asserted). The same editor with nothing staged.
#
# Judge by eye:
# - `nothing staged` sits on its own row directly above the options row.
#   The button is also dimmed, and no capture in this repo can show that
#   (CLAUDE.md) — the words are what has to carry it.
# - The title row still reads `nothing staged` as its count, so the reason
#   is visible even with the options row off the bottom of a short terminal.
#
=== 120x40 ===
";
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-commit-editor-refused.txt"
        ),
        format!("{header}{screen}"),
    );
}

/// Regenerates `docs/shots/m3-amend.txt` (m3-working-tree-undo-and-stash/05).
#[test]
fn capture_m3_amend() {
    use crate::update::commit_editor::CommitEditor;

    let mut state = changes_pane_state(120, 40);
    state.commit_editor = Some(CommitEditor::amend(
        "a1b2c3d".to_owned(),
        "fix the token refresh race\n\nThe old token was refreshed twice when two requests raced.\n",
    ));
    let screen = draw(&state, 120, 40);
    assert!(screen.contains("AMEND a1b2c3d"), "{screen}");
    let header = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_amend in crates/graft/src/view/tests.rs (asserted). `a` on the
# scripted tree, with HEAD's message already prefilled.
#
# Judge by eye:
# - The title row reads AMEND a1b2c3d, not COMMIT: which commit is being
#   rewritten is named, because an amend of the wrong one is the mistake
#   this milestone exists to prevent.
# - [x] amend is ticked in the options row, and the subject and body hold
#   HEAD's message split at its blank line.
#
=== 120x40 ===
";
    graft_fixtures::capture(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/shots/m3-amend.txt"),
        format!("{header}{screen}"),
    );
}

/// The moment every journal capture is drawn against, so the ages in them
/// never age (m3-working-tree-undo-and-stash/06).
const JOURNAL_NOW: i64 = 1_700_000_000;

/// One scripted journal entry.
fn journal_entry(
    ago_seconds: i64,
    kind: graft_core::journal::OperationKind,
    description: &str,
    touches: graft_core::journal::Touches,
    commands: &[&str],
) -> graft_core::journal::JournalEntry {
    let at_unix_ms = (JOURNAL_NOW - ago_seconds) * 1_000;
    graft_core::journal::JournalEntry {
        id: graft_core::journal::EntryId::new(at_unix_ms, [0; 10]),
        at_unix_ms,
        kind,
        description: description.to_owned(),
        status: graft_core::journal::EntryStatus::Committed,
        touches,
        head_before: graft_core::journal::HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(graft_core::journal::HeadRef::Branch(
            "refs/heads/main".to_owned(),
        )),
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: Some(graft_core::journal::IndexState::Merged(oid(1))),
        index_after: Some(graft_core::journal::IndexState::Merged(oid(2))),
        worktree_before: Some(oid(3)),
        worktree_after: Some(oid(4)),
        worktree_raw_before: None,
        backup_ref: Some("refs/graft/backup/main/01HF".to_owned()),
        pins: Vec::new(),
        commands: commands.iter().map(|line| (*line).to_owned()).collect(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
    }
}

/// A commit a previous run died in the middle of: recovery closes it as
/// `Aborted { "interrupted" }`, and it has no before-tree to restore, so it
/// carries the warning mark rather than the undo arrow.
fn torn_commit(touches: graft_core::journal::Touches) -> graft_core::journal::JournalEntry {
    use graft_core::journal::{EntryStatus, OperationKind};

    let mut torn = journal_entry(
        40 * 60,
        OperationKind::Commit,
        "commit \"WIP on refresh\"",
        touches,
        &["git commit -F -"],
    );
    torn.status = EntryStatus::Aborted {
        reason: graft_core::journal::INTERRUPTED.to_owned(),
    };
    torn.worktree_before = None;
    torn.head_after = None;
    torn.backup_ref = None;
    torn
}

/// The undo of `commit`. As the newest entry it is what `U` would put back,
/// and the commit under it then reads `undone`. Without this row no capture
/// showed the redo mark or `undone` at all (m3/06 review, Minor 12).
fn undo_of(commit: &graft_core::journal::JournalEntry) -> graft_core::journal::JournalEntry {
    use graft_core::journal::{OperationKind, Touches};

    let mut undo = journal_entry(
        60,
        OperationKind::Undo,
        "undo: commit \"Add OAuth provider\"",
        Touches {
            refs: vec!["refs/heads/main".to_owned()],
            index: true,
            worktree: false,
        },
        &["git update-ref refs/heads/main 1f3b9c2 8a7d4e1"],
    );
    undo.reverses = Some(commit.id.clone());
    undo
}

/// The journal browser's own state: six entries carrying every mark the
/// browser has — the next undo, the next redo, an entry already undone, one
/// interrupted by a dead process, and one that failed with its backup kept.
fn journal_entries() -> Vec<graft_core::journal::JournalEntry> {
    use graft_core::journal::{EntryStatus, OperationKind, Touches};

    let refs_and_index = Touches {
        refs: vec!["refs/heads/main".to_owned()],
        index: true,
        worktree: false,
    };
    let index_only = Touches {
        refs: Vec::new(),
        index: true,
        worktree: false,
    };
    let worktree = Touches {
        refs: Vec::new(),
        index: true,
        worktree: true,
    };
    let mut discard = journal_entry(
        3 * 3_600,
        OperationKind::Discard,
        "discard src/auth/refresh_test.rs",
        worktree,
        &["git --literal-pathspecs restore --worktree -- src/auth/refresh_test.rs"],
    );
    discard.status = EntryStatus::Aborted {
        reason: "error: unable to unlink 'src/auth/refresh_test.rs': Permission denied".to_owned(),
    };
    let torn = torn_commit(refs_and_index.clone());
    let commit = journal_entry(
        2 * 60,
        OperationKind::Commit,
        "commit \"Add OAuth provider\"",
        refs_and_index,
        &["git commit -F -"],
    );
    let undo = undo_of(&commit);

    vec![
        journal_entry(
            26 * 3_600,
            OperationKind::Stage,
            "stage src/api/routes.rs",
            index_only.clone(),
            &["git --literal-pathspecs add -- src/api/routes.rs"],
        ),
        discard,
        torn,
        journal_entry(
            14 * 60,
            OperationKind::Stage,
            "stage 3 files",
            index_only,
            &["git --literal-pathspecs add -- src/api/routes.rs src/auth/token.rs README.md"],
        ),
        commit,
        undo,
    ]
}

/// The journal browser open over the scripted repository.
fn journal_state(columns: u16, rows: u16) -> AppState {
    let mut state = changes_pane_state(columns, rows);
    state.repo_path = std::path::PathBuf::from("/code/api");
    state.harness.clock = crate::session::Clock::Fixed(JOURNAL_NOW);
    state.detail.shown = false;
    state.journal.entries = crate::update::detail::Loading::Ready(std::sync::Arc::new(
        crate::jobs::undo::JournalRead {
            entries: journal_entries(),
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        },
    ));
    // Opened as `J` opens it (m7/30o Minor 6), not by setting `shown`: the
    // notice is taken up and the rows are built from the entries read
    // before. The re-read it asks for never lands here; stages 2 and 3 are
    // answered empty below (m7/30p Minor 7), so the title is settled.
    let mut next = 0_u64;
    let commands = crate::update::journal::journal_enter(&mut state, &mut || {
        next += 1;
        graft_core::job::JobId::new(next)
    });
    assert!(
        matches!(
            commands.as_slice(),
            [
                crate::update::Command::ReadJournal(_),
                crate::update::Command::ReadOutside { .. }
            ]
        ),
        "{commands:?}"
    );
    let Some(crate::update::Command::ReadOutside { id: outside }) = commands.last() else {
        unreachable!("matched above: {commands:?}")
    };
    let mut asked = Vec::new();
    crate::update::journal::outside_landed(
        &mut state,
        &mut asked,
        *outside,
        &Ok(std::sync::Arc::new(Vec::new())),
        &mut || {
            next += 1;
            graft_core::job::JobId::new(next)
        },
    );
    let [crate::update::Command::FindLost { id: lost, outside }] = asked.as_slice() else {
        panic!("stage 2 asks for stage 3: {asked:?}")
    };
    assert!(outside.is_empty(), "{asked:?}");
    crate::update::journal::lost_landed(
        &mut state,
        *lost,
        &Ok(std::sync::Arc::new(crate::jobs::journal_list::LostFound {
            unreached: graft_git::history_reads::Unreached {
                lost: Vec::new(),
                checked: 0,
                capped: false,
            },
            edits: Vec::new(),
            scan: graft_ops::journal_list::LostScan::Done,
            subjects: Vec::new(),
        })),
    );
    state
}

const JOURNAL_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_journal in crates/graft/src/view/tests.rs (asserted; regenerate
# on purpose). `J` on a scripted journal of six entries, drawn
# against a fixed clock so the ages never move. Opened as `J` opens it,
# and the shell's steps and the lost-commit scan answered empty, so the
# title is settled. The bottom line has no interrupted-entry notice —
# looking at J is what it asked for.
#
# Judge by eye:
# - The title reads JOURNAL · what happened in this repo, newest first.
# - Newest first: the undo is the top row, the 26-hour-old stage the last.
# - Every mark the browser has is on screen, one row each, no row with two.
#   Not \"looks right\": the marks are `graft_ops`' own next_undo, next_redo
#   and in_force answers, so a mark on the wrong row means the key and the
#   browser disagree.
#     ← U          on the undo (top row): what `U` would put back.
#     undone       on the commit under it: `u` already took that one back.
#     ← u          on stage 3 files: the newest thing still in force.
#     interrupted  on commit \"WIP on refresh\": a run that died mid-write.
#                  It reads interrupted, and never failed:.
# - The discard row leads with failed: and git's own first line, and still
#   ends with backup saved — a write that failed part-way is still undoable.
# - The bar names what ⏎ does on the selected row (on the ← U row: ⏎ redo this),
#   then space command log and J/esc close; the legend sits above it.
";

const JOURNAL_EXPANDED_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_journal in crates/graft/src/view/tests.rs (asserted; regenerate
# on purpose). The same journal with the cursor moved onto the
# failed discard and `space` pressed.
#
# Judge by eye:
# - The cursor row is the discard, marked ▸ and painted.
# - Under it, indented: the whole failure reason, then the Command Log's one
#   `┆ git --literal-pathspecs restore …` line, under `commands Graft ran` — the exact command that ran,
#   not a description of it.
# - The rows below the expansion are pushed down, not overwritten.
";

/// Regenerates the journal browser's captures
/// (m3-working-tree-undo-and-stash/06).
#[test]
fn capture_m3_journal() {
    use graft_core::event::{Event, KeyCode, KeyPress};

    let mut shots = graft_fixtures::Captures::new();

    for (columns, rows) in [(160_u16, 48_u16), (80, 24)] {
        let state = journal_state(columns, rows);
        let screen = draw(&state, columns, rows);
        assert!(
            screen.contains("JOURNAL · what happened in this repo"),
            "{screen}"
        );
        assert!(screen.contains("← u"), "{screen}");
        assert!(screen.contains("← U"), "{screen}");
        assert!(screen.contains("undone"), "{screen}");
        assert!(screen.contains("interrupted"), "{screen}");
        // Opened as `J` opens it (m7/30o Minor 6): looking at J is what
        // the interrupted-entry notice asked for, so it is not on screen.
        assert!(!screen.contains("J to inspect"), "{screen}");
        // Stages 2 and 3 answered empty (m7/30p Minor 7): the title is
        // settled, not frozen on its loading state.
        let title = screen
            .lines()
            .find(|line| line.contains("JOURNAL ·"))
            .expect("J's title row is drawn");
        assert!(!title.contains("finding lost commits"), "{screen}");
        shots.check(
            format!(
                "{}/../../docs/shots/m3-journal-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            format!("{JOURNAL_HEADER}=== {columns}x{rows} ===\n{screen}"),
        );
    }

    // Down onto the failed discard — the fifth row, newest first — then
    // `space`.
    let mut state = journal_state(120, 32);
    let down = std::iter::repeat_n(KeyCode::Char('j'), 4);
    for code in down.chain(std::iter::once(KeyCode::Char(' '))) {
        let (next, _) = crate::update::update(
            &state,
            crate::update::AppEvent::Core(Event::Key(KeyPress::plain(code))),
        );
        state = next;
    }
    let screen = draw(&state, 120, 32);
    assert!(screen.contains("Permission denied"), "{screen}");
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-journal-expanded.txt"
        ),
        format!("{JOURNAL_EXPANDED_HEADER}=== 120x32 ===\n{screen}"),
    );
    shots.finish();
}

/// The stash overlay open over the scripted repository, with two entries
/// and the newest one's files under them.
fn stash_state(columns: u16, rows: u16) -> AppState {
    let mut state = changes_pane_state(columns, rows);
    state.repo_path = std::path::PathBuf::from("/code/api");
    state.harness.clock = crate::session::Clock::Fixed(JOURNAL_NOW);
    state.detail.shown = false;
    state.stash.shown = true;
    state.stash.entries = crate::update::detail::Loading::Ready(std::sync::Arc::new(vec![
        stash_entry(
            0,
            "WIP on main: token refresh",
            2 * 3_600,
            &["src/auth/token.rs", "src/auth/refresh.rs", "README.md"],
        ),
        stash_entry(
            1,
            "quick fix before pull",
            26 * 3_600,
            &["src/api/routes.rs"],
        ),
    ]));
    // The selected stash's files, as `Space` asks for them: a commit's file
    // list, so no STAGED/UNSTAGED sections — a stash is one commit.
    state.stash.preview = crate::update::detail::Loading::Ready(std::sync::Arc::new(
        crate::jobs::detail::assemble_files(&stash_files(), None),
    ));
    state
}

/// What `stash@{0}` changed, as `git diff-tree` reports it.
fn stash_files() -> graft_git::diff::FileList {
    use graft_git::diff::{FileEntry, FileList, FileStatus};
    let file = |path: &str, status: FileStatus, added: u32, removed: u32| FileEntry {
        path: path.as_bytes().to_vec(),
        old_path: None,
        status,
        added: Some(added),
        removed: Some(removed),
    };
    FileList {
        files: vec![
            file("README.md", FileStatus::Modified, 1, 1),
            file("src/auth/refresh.rs", FileStatus::Added, 31, 0),
            file("src/auth/token.rs", FileStatus::Modified, 12, 4),
        ],
        added: 44,
        removed: 5,
    }
}

fn stash_entry(
    index: usize,
    message: &str,
    ago_seconds: i64,
    paths: &[&str],
) -> graft_git::stash::StashEntry {
    graft_git::stash::StashEntry {
        index,
        commit: oid(u8::try_from(index).unwrap_or(0) + 20),
        message: message.to_owned(),
        at: graft_core::commit::Timestamp {
            seconds: JOURNAL_NOW - ago_seconds,
            offset_seconds: 0,
        },
        paths: paths.iter().map(std::path::PathBuf::from).collect(),
    }
}

const STASH_LIST_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_stash in crates/graft/src/view/tests.rs (asserted; regenerate
# on purpose). `S` on a scripted list of two stashes, `space`
# pressed on the newest, drawn against a fixed clock so the ages never move.
#
# Judge by eye:
# - The header counts them: STASHES (2).
# - Each row reads stash@{n}, the message in quotes, the age, then the file
#   count — and `1 file` is singular where the count is one.
# - The cursor row is marked with ▸ and painted to the right edge.
# - Under the list, the selected stash's files with +/− counts. This is a
#   file list, not a diff: the diff of a stash is the detail pane's view.
# - The footer offers ⏎ apply, p pop, s save, d drop, space files, esc close.
";

const STASH_SAVE_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_stash in crates/graft/src/view/tests.rs (asserted; regenerate
# on purpose). `s` pressed, a message typed, `tab` pressed once.
#
# Judge by eye:
# - The message line carries what was typed, with the cursor after it.
# - The scope is marked in text, not colour: (•) sits on `staged only`
#   because `tab` moved it there, and ( ) on `all incl. untracked`.
# - The line under it says the save is undoable. A capture cannot show
#   colour, so nothing here may depend on it.
# - The footer changes to ⏎ save, tab scope, esc back: while the prompt is
#   open the list's own keys are text.
";

const STASH_EMPTY_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_stash in crates/graft/src/view/tests.rs (asserted; regenerate
# on purpose). `S` on a repository with no stashes.
#
# Judge by eye:
# - The header reads STASHES (0) and the body says
#   `no stashes — s stashes the working tree`: the empty state is also the
#   instruction for leaving it.
# - The footer offers only what works here: s save, esc close.
";

/// Regenerates the stash overlay's captures
/// (m3-working-tree-undo-and-stash/06).
#[test]
fn capture_m3_stash() {
    use graft_core::event::{Event, KeyCode, KeyPress};

    let mut shots = graft_fixtures::Captures::new();

    let state = stash_state(120, 32);
    let screen = draw(&state, 120, 32);
    assert!(screen.contains("STASHES (2)"), "{screen}");
    assert!(screen.contains("stash@{1}"), "{screen}");
    assert!(screen.contains("1 file"), "{screen}");
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-stash-list.txt"
        ),
        format!("{STASH_LIST_HEADER}=== 120x32 ===\n{screen}"),
    );

    // `s`, a message, then `tab` onto `staged only`.
    let mut state = stash_state(120, 32);
    let mut keys = vec![KeyCode::Char('s')];
    keys.extend("token work".chars().map(KeyCode::Char));
    keys.push(KeyCode::Tab);
    for code in keys {
        let (next, _) = crate::update::update(
            &state,
            crate::update::AppEvent::Core(Event::Key(KeyPress::plain(code))),
        );
        state = next;
    }
    let screen = draw(&state, 120, 32);
    assert!(screen.contains("token work"), "{screen}");
    assert!(screen.contains("(\u{2022}) staged only"), "{screen}");
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-stash-save.txt"
        ),
        format!("{STASH_SAVE_HEADER}=== 120x32 ===\n{screen}"),
    );

    let mut state = stash_state(120, 32);
    state.stash.entries = crate::update::detail::Loading::Ready(std::sync::Arc::new(Vec::new()));
    state.stash.preview = crate::update::detail::Loading::Idle;
    let screen = draw(&state, 120, 32);
    assert!(screen.contains("no stashes"), "{screen}");
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-stash-empty.txt"
        ),
        format!("{STASH_EMPTY_HEADER}=== 120x32 ===\n{screen}"),
    );
    shots.finish();
}

const DRIFT_HEADER: &str = "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_m3_undo_drift in crates/graft/src/view/tests.rs (asserted;
# regenerate on purpose). The undo job answered Drift: the repository moved
# since the entry `u` would take back.
#
# Judge by eye:
# - The question names the entry, not the word \"operation\".
# - The consequence is `graft_ops`' own drift.explain() text, verbatim —
#   which fields moved, and that an unrecorded after-state is restored whole.
# - Under it, what the undo replaces and the key that brings it back
#   (m6-conflicts/77): `Undo replaces the whole working tree; U brings it
#   back.` The drift line takes three lines here, so `a backup keeps it, and`
#   is dropped to fit the box's four.
# - The buttons read [ Stop ] and Undo, with Stop under the cursor: the safe
#   choice is the one already selected, and neither says \"Discard\", which
#   would name the opposite of what the button does.
# - The command line shows what confirming would run.
";

/// Regenerates the undo drift guard's capture
/// (m3-working-tree-undo-and-stash/06).
#[test]
fn capture_m3_undo_drift() {
    let mut state = changes_pane_state(120, 32);
    let id = graft_core::job::JobId::new(7);
    state.operating = Some(id);
    let drift = graft_ops::undo::Drift {
        index: Some((
            graft_core::journal::IndexState::Merged(oid(1)),
            graft_core::journal::IndexState::Merged(oid(2)),
        )),
        unknown: vec!["the working tree"],
        // What the executor reads for an unknown working tree: all of it
        // is restored, so all of it counts as replaced (m6-conflicts/77).
        replaced: graft_ops::undo::Replaced {
            paths: Vec::new(),
            whole_tree: true,
            whole_index: false,
        },
        ..graft_ops::undo::Drift::default()
    };
    let mut next = 100;
    let _commands = crate::update::undo::undone(
        &mut state,
        id,
        crate::jobs::undo::Undone::Drift {
            description: "commit \"Add OAuth provider\"".to_owned(),
            drift,
            redo: false,
        },
        &mut || {
            next += 1;
            graft_core::job::JobId::new(next)
        },
    );
    let screen = draw(&state, 120, 32);
    assert!(screen.contains("Stop"), "{screen}");
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m3-undo-drift.txt"
        ),
        format!("{DRIFT_HEADER}=== 120x32 ===\n{screen}"),
    );
}

/// The undo drift box over `description`, with one uncommitted edit to
/// `path` in the way: the lines graft built for its message, and the screen
/// drawn at `columns`×`rows`.
fn drift_box_drawn(
    columns: u16,
    rows: u16,
    description: &str,
    path: &str,
) -> (Vec<String>, String) {
    let mut state = changes_pane_state(columns, rows);
    let id = graft_core::job::JobId::new(7);
    state.operating = Some(id);
    let drift = graft_ops::undo::Drift {
        refs: vec![("refs/heads/main".to_owned(), Some(oid(1)), Some(oid(2)))],
        head: true,
        worktree: true,
        replaced: graft_ops::undo::Replaced {
            paths: vec![std::path::PathBuf::from(path)],
            whole_tree: false,
            whole_index: false,
        },
        ..graft_ops::undo::Drift::default()
    };
    let mut next = 100;
    let _commands = crate::update::undo::undone(
        &mut state,
        id,
        crate::jobs::undo::Undone::Drift {
            description: description.to_owned(),
            drift,
            redo: false,
        },
        &mut || {
            next += 1;
            graft_core::job::JobId::new(next)
        },
    );
    let message = state
        .confirm
        .as_ref()
        .expect("the drift box is open")
        .view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect();
    (message, draw(&state, columns, rows))
}

/// m6-conflicts/77, the fit rule: a 60-character path under a long title.
/// graft cuts the path from the left (`…/fix.txt`), and drops `a backup
/// keeps it, and ` when the drift line above takes three lines, so the key
/// sentence reaches `brings it back.` whole and graft-ui's four-line cap
/// never cuts it. At 80×24 and 160×48.
#[test]
fn the_drift_box_line_is_never_cut() {
    let path = "services/billing/src/handlers/invoices/overdue/fix.txt";
    let path = format!("{}{path}", "a/".repeat((60 - path.len()) / 2));
    assert_eq!(path.len(), 60, "{path}");
    let titles = [
        // The drift line wraps to two lines: the long form fits.
        ("commit \"Add OAuth provider\"", true),
        // Three lines: the short form, key kept.
        (
            "commit \"Handle the overdue invoice reminder when the customer changed plans mid-cycle\"",
            false,
        ),
    ];
    for (columns, rows) in [(80, 24), (160, 48)] {
        for (description, long) in titles {
            let (message, screen) = drift_box_drawn(columns, rows, description, &path);
            let words = message.join(" ");
            let at = format!("{columns}x{rows}, {description}");
            assert!(message.len() <= 4, "{at}: over graft-ui's cap: {message:?}");
            assert!(words.ends_with("U brings it back."), "{at}: {words}");
            assert!(
                words.contains("your edit to \u{2026}/") && words.contains("/fix.txt;"),
                "{at}: the path is cut from the left: {words}"
            );
            assert!(!words.contains(&path), "{at}: the path is cut by graft");
            assert_eq!(words.contains("a backup keeps it"), long, "{at}: {words}");
            // Drawn, not only built: every line graft made is on screen as
            // it was made — graft-ui neither re-wrapped nor cut one.
            for line in &message {
                assert!(screen.contains(line.as_str()), "{at}: {line:?}\n{screen}");
            }
        }
    }
}

/// `P` after git refused a push as not a fast-forward, the refs current
/// and `HEAD` still the refused tip: the lease box open, listing `leaves`.
fn lease_box_state(columns: u16, rows: u16, leaves: crate::update::remote::Leaves) -> AppState {
    use crate::jobs::remote::RemoteFacts;
    use crate::update::detail::Loading;
    use graft_git::remote::{Remote, TrackedUpstream};

    let mut state = changes_pane_state(columns, rows);
    state.remotes = Loading::Ready(std::sync::Arc::new(RemoteFacts {
        remotes: vec![Remote {
            name: "origin".to_owned(),
            fetch_url: "git@example.invalid:team/api.git".to_owned(),
            push_url: None,
        }],
        head_branch: Some("refs/heads/main".to_owned()),
        head_upstream: Some(TrackedUpstream {
            remote: "origin".to_owned(),
            remote_branch: "refs/heads/main".to_owned(),
            tracking_ref: "refs/remotes/origin/main".to_owned(),
            tracking_tip: Some(
                Oid::from_hex("9f8e7d6c5b4a39281706f5e4d3c2b1a098765432").expect("40 hex digits"),
            ),
        }),
    }));
    // Nothing moved since the rejection: the refs are current and `HEAD`
    // is the tip git refused (m6-conflicts/49).
    state.refs_read = crate::update::RefsRead::Current(graft_core::job::JobId::new(1));
    let sent = state
        .refs
        .as_deref()
        .and_then(crate::refs::RefBadges::head)
        .expect("HEAD on a branch");
    state.remote.last_rejected_push = Some(crate::update::remote::RejectedPush {
        params: crate::update::remote::push_params(&state).expect("an upstream to push to"),
        sent: Some(sent),
        // What the lease takes off origin, read for this key
        // (m7-power-tools/16b).
        leaves: Some((
            sent,
            Oid::from_hex("9f8e7d6c5b4a39281706f5e4d3c2b1a098765432").expect("40 hex digits"),
            crate::update::remote::LeavesRead::Read(leaves),
        )),
    });
    assert!(
        crate::update::remote::push_key(&mut state),
        "P opens the box"
    );
    state
}

/// Regenerates `docs/shots/m4-remote-keys.txt` (m4-remotes/05): the lease
/// push box `P` opens after a rejection, then the footer while a fetch runs.
#[test]
fn capture_m4_remote_keys() {
    let mut state = lease_box_state(120, 40, crate::update::remote::Leaves::default());
    let push_box = draw(&state, 120, 40);

    state.confirm = None;
    state.operating = Some(graft_core::job::JobId::new(9));
    state.status =
        Status::Busy("fetch all remotes · Receiving objects 45% · esc cancels".to_owned());
    let fetching = draw(&state, 120, 40);

    let header = "\
# m4-remote-keys.txt: TestBackend capture (not a pty), 120x40, no colour
# recorded. Fixture: capture_m4_remote_keys in crates/graft/src/view/tests.rs
# (asserted; regenerate on purpose). Two screens.
#
# 1. `P` after git refused a push as not a fast-forward: the lease box.
#    Judge by eye: it names the tip it expects (9f8e7d6) and the one it
#    sends, says it is refused if origin moved since the fetch, says whose
#    history it rewrites, shows the exact git command, and `[ Cancel ]` is
#    under the cursor with `Force push` beside it.
# 2. The footer while `f` runs: the operation, git's own progress, and how
#    to stop it (`esc cancels`).
#
";
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m4-remote-keys.txt"
        ),
        format!("{header}{push_box}\n# ---- 2. while `f` runs ----\n\n{fetching}"),
    );
}

fn leaving(hex: char, subject: &str, author: &str) -> graft_git::history_reads::CommitSummary {
    graft_git::history_reads::CommitSummary {
        id: Oid::from_hex(&hex.to_string().repeat(40)).expect("40 hex digits"),
        parents: Vec::new(),
        subject: subject.to_owned(),
        author: graft_core::commit::Identity {
            name: author.as_bytes().to_vec(),
            email: b"dev@example.invalid".to_vec(),
            when: graft_core::commit::Timestamp {
                seconds: 0,
                offset_seconds: 0,
            },
        },
    }
}

/// Regenerates `docs/shots/m7/force-box-leaves-{120x40,80x24}.txt`
/// (m7-power-tools/16b): the lease box names the commits the force takes
/// off origin — five leave, three named, `+2 more`.
#[test]
fn capture_force_box_leaves() {
    for (columns, rows) in [(120, 40), (80, 24)] {
        let leaves = crate::update::remote::Leaves {
            newest: vec![
                leaving('c', "Fix typo in the retry docs", "Sara"),
                leaving(
                    'd',
                    "Retry the fetch when the remote hangs up mid-pack on a slow link",
                    "Ali Rahimi",
                ),
                leaving('e', "Add the fetch timeout", "Sara"),
            ],
            count: 5,
        };
        let state = lease_box_state(columns, rows, leaves);
        let message: Vec<&str> = state
            .confirm
            .as_ref()
            .expect("P opened the lease box")
            .view
            .message
            .iter()
            .map(SanitizedText::as_str)
            .collect();
        let screen = draw(&state, columns, rows);
        // Every row from the history warning on is one box row, never
        // wrapped or cut (line 1 may wrap at 80 columns: it always did).
        for line in &message[1..] {
            assert!(
                screen.contains(line),
                "{columns}x{rows}: {line:?} not drawn whole\n{screen}"
            );
        }
        assert!(screen.contains("[ Force push ]"), "{screen}");
        let header = format!(
            "\
# force-box-leaves-{columns}x{rows}.txt: TestBackend capture (not a pty), {columns}x{rows}, no colour
# recorded (colour unverified). Fixture: capture_force_box_leaves in
# crates/graft/src/view/tests.rs (asserted; regenerate on purpose).
#
# Right: line 1 (origin/main 9f8e7d6 → <ours> · refused if …), then
# `rewrites history for everyone who has 9f8e7d6`, then `leaves origin/main:`,
# then each commit's sha7, subject and author on its own row, aligned in
# columns (subject cut with … to fit), then `+2 more`; the buttons row whole.
#
"
        );
        graft_fixtures::capture(
            format!(
                "{}/../../docs/shots/m7/force-box-leaves-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            format!("{header}{screen}"),
        );
    }
}

/// A push running, with git's question open over it (m4-remotes/06).
fn credential_state(columns: u16, rows: u16, prompt: &str) -> AppState {
    credential_state_seeded(columns, rows, prompt, false)
}

/// The same, with the masked field's pulse seed pinned. Every test here
/// pins it: the seed is random in the running app, so a capture or an
/// assertion that did not pin it would flip between two rows at random
/// (`graft::jobs::askpass::pulse_seed`).
fn credential_state_seeded(columns: u16, rows: u16, prompt: &str, pulse_seed: bool) -> AppState {
    use crate::jobs::remote::RemoteFacts;
    use crate::update::detail::Loading;
    use graft_git::remote::{Remote, TrackedUpstream};
    use graft_ops::operation::{Operation, PushParams};

    let id = graft_core::job::JobId::new(9);
    let mut state = changes_pane_state(columns, rows);
    state.remotes = Loading::Ready(std::sync::Arc::new(RemoteFacts {
        remotes: vec![Remote {
            name: "origin".to_owned(),
            fetch_url: "http://user@127.0.0.1:8443/api.git".to_owned(),
            push_url: None,
        }],
        head_branch: Some("refs/heads/main".to_owned()),
        head_upstream: Some(TrackedUpstream {
            remote: "origin".to_owned(),
            remote_branch: "refs/heads/main".to_owned(),
            tracking_ref: "refs/remotes/origin/main".to_owned(),
            tracking_tip: None,
        }),
    }));
    let push = Operation::Push(PushParams {
        branch: "refs/heads/main".to_owned(),
        remote: "origin".to_owned(),
        remote_branch: "refs/heads/main".to_owned(),
        set_upstream: false,
        lease: None,
        source_commit: None,
    });
    state.operating = Some(id);
    state.remote.running = Some(crate::update::remote::Running {
        id,
        description: "push main → origin/main".to_owned(),
        op: Some(push),
        stoppable: crate::update::remote::Stoppable::Yes,
        opened_by: None,
    });
    state.status = Status::Busy("push main → origin/main · esc cancels".to_owned());
    crate::update::credential::credential_needed(&mut state, id, prompt, pulse_seed);
    state
}

/// Types `text` into the open prompt, the way a paste would — the public
/// entry point (`credential_paste`), not a direct poke at `PromptField`,
/// which keeps its editing methods private to `update::credential`.
fn type_into_prompt(state: &mut AppState, text: &str) {
    assert!(
        crate::update::credential::credential_paste(state, text),
        "the prompt is open"
    );
}

#[test]
fn the_credential_prompt_draws_over_everything_and_masks_what_is_typed() {
    let mut state = credential_state(120, 40, "Password for 'http://user@127.0.0.1:8443':");
    type_into_prompt(&mut state, "hunter2");

    let screen = draw(&state, 120, 40);

    assert!(screen.contains("Credentials"), "{screen}");
    assert!(screen.contains("127.0.0.1:8443"), "{screen}");
    assert!(
        screen.contains("Password for 'http://user@127.0.0.1:8443':"),
        "{screen}"
    );
    // A fixed 8 glyphs, whatever "hunter2" (7 characters) is — the row
    // never draws `len()` glyphs (ADR-0038 §2).
    assert_eq!(
        screen.chars().filter(|c| *c == '•' || *c == '◦').count(),
        graft_ui::text_input::SECRET_GLYPHS,
        "the field is masked at a fixed width: {screen}"
    );
    assert!(
        !screen.contains("hunter2"),
        "the typed password reached the screen: {screen}"
    );
    assert!(
        screen.contains("typed here only") && screen.contains("never stored by Graft"),
        "{screen}"
    );
    assert!(
        screen.contains("esc cancel push main → origin/main"),
        "the footer names the push Esc stops (m6-conflicts/78): {screen}"
    );
}

#[test]
fn a_username_ask_is_drawn_in_the_open() {
    let mut state = credential_state(120, 40, "Username for 'https://github.com':");
    type_into_prompt(&mut state, "alice");

    let screen = draw(&state, 120, 40);

    assert!(screen.contains("Credentials"), "{screen}");
    assert!(screen.contains("alice"), "a username is not a secret");
    assert!(!screen.contains('•'), "nothing to mask: {screen}");
}

#[test]
fn gits_words_and_a_remote_failure_cannot_carry_an_escape_to_the_screen() {
    // Remote-controlled text: git's prompt (a crafted URL) and the failure
    // line `ops` worded from git's stderr (a hostile pre-receive hook). Both
    // are drawn, and neither may put a control character in a cell
    // (m4/02 review m5).
    let hostile = "Password for 'http://u@h/\u{1b}[2J\u{7}\r':";
    let screen = draw(&credential_state(120, 40, hostile), 120, 40);
    assert!(!screen.contains('\u{1b}'), "{screen:?}");
    assert!(!screen.contains('\u{7}'), "{screen:?}");

    let mut failed = credential_state(120, 40, "Password for 'http://u@h':");
    failed.credential = None;
    failed.operating = None;
    failed.status = Status::sticky("push rejected: \u{1b}[31mhook said no\u{7}".to_owned());
    let screen = draw(&failed, 120, 40);
    assert!(screen.contains("hook said no"), "{screen}");
    assert!(!screen.contains('\u{1b}'), "{screen:?}");
    assert!(!screen.contains('\u{7}'), "{screen:?}");
}

/// Regenerates `docs/shots/m4-credential-prompt-in-app.txt` (m4-remotes/06):
/// git's password question over the running push, and the username ask.
#[test]
fn capture_m4_credential_prompt_in_app() {
    let ask = "Password for 'http://user@127.0.0.1:8443':";
    let mut password = credential_state_seeded(120, 40, ask, false);
    type_into_prompt(&mut password, "hunter2");
    let masked = draw(&password, 120, 40);

    // The same password, the same length, the other seed. The two rows
    // differ, which is the point: what the row rests at is the seed's
    // doing, not the password's.
    let mut other_seed = credential_state_seeded(120, 40, ask, true);
    type_into_prompt(&mut other_seed, "hunter2");
    let masked_other_seed = draw(&other_seed, 120, 40);

    let mut username = credential_state(80, 24, "Username for 'http://127.0.0.1:8443':");
    type_into_prompt(&mut username, "alice");
    let plain = draw(&username, 80, 24);

    let header = "\
# m4-credential-prompt-in-app.txt: TestBackend captures (not a pty), no
# colour recorded. Fixture: capture_m4_credential_prompt_in_app in
# crates/graft/src/view/tests.rs (asserted; regenerate on purpose).
# The prompt as it appears *in the app*, over a running push — the widget on
# its own is docs/shots/m4-credential-prompt-and-hints.txt.
#
# Judge by eye:
# 1. 120x40, password ask with a 7-character password typed: the box is centred over the
#    screen, the title names the host (not the user), git's own question is
#    on the first line, the field shows a fixed 8 mask glyphs (never the
#    typed length, ADR-0038 §2) and no plaintext, the note says the answer
#    is never stored, and the box's bottom border says what ⏎, ⌃U and Esc
#    do, ending `esc cancel push main → origin/main`: the push it cancels,
#    by name (m6-conflicts/78). The bottom row is the prompt's own keys,
#    `⏎ send  esc cancel`, in every frame: the push's running line waits
#    under it and comes back once the prompt is answered (m6-conflicts/69).
# 2. The same 7-character password again, with the prompt's pulse seed the
#    other way up. The first mask glyph differs between 1 and 2 and nothing
#    else does. That is the whole of ADR-0038 §2: the glyph flips on every
#    keystroke so a key visibly lands, and where it rests is decided by a
#    random bit drawn per prompt (`graft::jobs::askpass::pulse_seed`), not
#    by the length — so a screenshot of either row says nothing about how
#    long the password is. Both rows are 8 glyphs wide.
#    It does not hide the length from somebody who watches the row from the
#    moment the prompt opens and counts the flips.
# 3. 80x24, username ask with `alice` typed: the same box, the answer in the
#    open (a username is not a secret), no `•` anywhere.
#
";
    graft_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m4-credential-prompt-in-app.txt"
        ),
        format!(
            "{header}=== 120x40 · password · pulse seed down ===\n{masked}\n\
             === 120x40 · the same password · pulse seed up ===\n{masked_other_seed}\n\
             === 80x24 · username ===\n{plain}"
        ),
    );
}

/// Which of [`history_of`]'s commits the frame actually drew, by id.
///
/// Substring matching would make `Commit 5` a hit inside `Commit 50`, so the
/// digits are parsed rather than searched for.
fn commits_drawn(frame: &str) -> std::collections::BTreeSet<u8> {
    let mut drawn = std::collections::BTreeSet::new();
    for line in frame.lines() {
        let mut rest = line;
        while let Some(at) = rest.find("Commit ") {
            rest = &rest[at + "Commit ".len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(id) = digits.parse::<u8>() {
                drawn.insert(id);
            }
        }
    }
    drawn
}

/// A rebase plan with one ghost anchored on `anchor`, replaying `replayed`
/// and carrying `feature`'s badge onto the ghost.
pub(super) fn one_ghost_over(anchor: Oid, replayed: Oid) -> graft_ops::preview::PreviewPlan {
    use graft_ops::preview::{
        ConflictForecast, GhostCommit, HeadMove, PreviewPlan, RefMove, Target, UndoNote,
        WorktreeEffect, ghost_oid,
    };
    PreviewPlan {
        files: None,
        description: "rebase feature onto main".to_owned(),
        commands: vec!["git rebase --no-update-refs --no-autostash main".to_owned()],
        ghosts: vec![GhostCommit {
            oid: ghost_oid("replayed work", 0),
            parents: vec![Target::Real(anchor)],
            subject: "replayed work".to_owned(),
            author: graft_core::commit::Identity {
                name: b"Ada".to_vec(),
                email: b"ada@example.com".to_vec(),
                when: graft_core::commit::Timestamp {
                    seconds: 1_700_000_000,
                    offset_seconds: 0,
                },
            },
            replays: Some(replayed),
        }],
        ref_moves: vec![RefMove {
            name: "refs/heads/feature".to_owned(),
            from: Some(replayed),
            to: Some(Target::Ghost(0)),
        }],
        head: Some(HeadMove::ToBranch("refs/heads/feature".to_owned())),
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
        already_in: None,
        published: Vec::new(),
        left_behind: Vec::new(),
        left_behind_more: 0,
        forks: Vec::new(),
        forks_more: 0,
        messages: Vec::new(),
    }
}

/// A preview over a history the user has scrolled into, with one ghost
/// anchored inside the viewport, and the row numbers the fixture uses.
///
/// Returns the state with the preview open and its plan landed, the
/// viewport's top row, and `id_at`: the commit id `history_of` put on an
/// absolute row.
fn scrolled_preview(
    count: u8,
    top_at: impl Fn(usize) -> usize,
    ghost_row_offset: usize,
) -> (AppState, usize, usize) {
    scrolled_preview_with_cursor(count, top_at, ghost_row_offset, |_| 1)
}

/// [`scrolled_preview`], with the cursor placed `cursor_at(height)` rows
/// below the top of the viewport.
fn scrolled_preview_with_cursor(
    count: u8,
    top_at: impl Fn(usize) -> usize,
    ghost_row_offset: usize,
    cursor_at: impl Fn(usize) -> usize,
) -> (AppState, usize, usize) {
    use crate::update::preview::{Planned, PreviewSubject, open_preview, preview_produced};
    use graft_ops::operation::{Operation, RebaseParams, Source, Tip};

    let mut state = AppState {
        history: Some(history_of(count, 0)),
        size: Size {
            columns: 100,
            rows: 30,
        },
        ..AppState::default()
    };
    let height = state.graph_rows();
    let top = top_at(height);
    let id_at = |row: usize| u8::try_from(usize::from(count) - row).expect("row is inside history");
    let cursor_row = top + cursor_at(height);
    {
        let history = state.history.as_mut().expect("a history");
        history.top = top;
        history.cursor = cursor_row;
    }

    let anchor = oid(id_at(top + ghost_row_offset));
    let plan = one_ghost_over(anchor, oid(id_at(cursor_row)));

    let mut commands = Vec::new();
    let mut next = 0;
    assert!(
        open_preview(
            &mut state,
            &mut commands,
            PreviewSubject::Operation {
                op: Box::new(Operation::Rebase(RebaseParams::new(
                    Tip {
                        name: "refs/heads/feature".to_owned(),
                        oid: oid(id_at(cursor_row)),
                    },
                    Source::Branch("main".to_owned()),
                    anchor,
                ))),
                opened_by: "r",
            },
            false,
            &mut || {
                next += 1;
                graft_core::job::JobId::new(next)
            },
        ),
        "nothing is running, so the preview opens"
    );
    let job = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        &mut state,
        job,
        Ok(std::sync::Arc::new(Planned {
            plan,
            anchors: Vec::new(),
            runs: None,
        })),
    );
    (state, top, height)
}

/// Ticket 17 F1. Scrolled a screen or more into history, the preview must
/// draw the rows the user was looking at — not the top of the three-screen
/// window it fetched. Every other preview test sits at `top == 0`, where the
/// slice start saturates and the two are the same rows.
#[test]
fn a_preview_scrolled_into_history_draws_the_screen_the_user_is_on() {
    let (state, top, height) = scrolled_preview(120, |height| 2 * height, 3);

    let frame = draw(&state, 100, 30);
    let drawn = commits_drawn(&frame);
    let id_at = |row: usize| u8::try_from(120_usize - row).expect("row is inside history");

    assert!(
        drawn.contains(&id_at(top)),
        "the viewport's first row is drawn\n{frame}"
    );
    assert!(
        !drawn.contains(&id_at(top - height)),
        "the screen above the user's is not drawn\n{frame}"
    );
    assert!(
        drawn.contains(&id_at(top + 1)),
        "the commit the cursor is on is on screen\n{frame}"
    );
    assert!(
        frame.contains("replayed work"),
        "the ghost anchored inside the viewport is on screen\n{frame}"
    );
}

/// Ticket 17 F1, second tick: the same at the bottom of history, where the
/// three-screen fetch window runs out of rows below the viewport and the
/// slice ends early. The picture still starts at the user's own top row.
#[test]
fn a_preview_near_the_end_of_history_still_starts_at_the_users_top_row() {
    const COUNT: u8 = 120;
    // Eight rows from the bottom: the three-screen fetch window asks for
    // rows history does not have, so the slice clamps below the viewport.
    let (state, top, height) = scrolled_preview(COUNT, |_| usize::from(COUNT) - 8, 3);
    assert!(top > height, "the viewport is more than one screen down");

    let frame = draw(&state, 100, 30);
    let drawn = commits_drawn(&frame);
    let id_at = |row: usize| u8::try_from(usize::from(COUNT) - row).expect("inside history");

    assert!(
        drawn.contains(&id_at(top)),
        "the viewport's first row is drawn\n{frame}"
    );
    assert_eq!(
        drawn.iter().max(),
        Some(&id_at(top)),
        "nothing above the user's top row is drawn\n{frame}"
    );
    assert!(
        drawn.contains(&id_at(usize::from(COUNT) - 1)),
        "the oldest row the history has is on screen\n{frame}"
    );
    let oldest = id_at(usize::from(COUNT) - 1);
    let newest = id_at(top);
    assert_eq!(
        drawn.iter().copied().collect::<Vec<u8>>(),
        (oldest..=newest).collect::<Vec<u8>>(),
        "the drawn rows are the viewport's, contiguous, and no others\n{frame}"
    );
    assert!(
        frame.contains("replayed work"),
        "the ghost anchored inside the viewport is on screen\n{frame}"
    );
}

/// Review 08 Major 5: the preview highlighted whichever row happened to sit
/// at the *choice* cursor's index — row 0 over `Cancel`, row 1 over
/// `Confirm` — so the commit the user pressed the key on was never marked.
#[test]
fn a_preview_highlights_the_commit_the_cursor_is_on_not_the_choice_index() {
    let (state, top, _) = scrolled_preview(120, |height| 2 * height, 3);
    let cursor_id = oid(u8::try_from(120_usize - (top + 1)).expect("inside history"));

    let view = graph_view(&state, state.harness.clock.now_seconds());

    let highlighted: Vec<Oid> = view
        .rows
        .iter()
        .filter(|entry| entry.line.paint == RowPaint::Highlighted)
        .map(|entry| entry.line.row.commit.id)
        .collect();
    assert_eq!(
        highlighted,
        vec![cursor_id],
        "exactly the cursor's commit is highlighted"
    );
    assert_eq!(view.reveal, None, "a modal picture has no hovered row");
}

/// Review 08 Minor 10: while the plan is still in flight the slice is empty
/// and the graph keeps the rows it had — but the box already has the
/// keyboard, so the footer must be the preview's. It used to go on offering
/// `m merge  r rebase`, two keys the modal box swallows.
#[test]
fn the_footer_is_the_previews_from_the_moment_the_box_opens() {
    let mut state = AppState {
        history: Some(history_of(6, 0)),
        size: Size {
            columns: 100,
            rows: 30,
        },
        ..AppState::default()
    };
    let mut commands = Vec::new();
    let mut next = 0;
    assert!(crate::update::preview::open_preview(
        &mut state,
        &mut commands,
        crate::update::preview::PreviewSubject::RevertTo(graft_core::journal::EntryId::new(
            1, [0; 10]
        )),
        false,
        &mut || {
            next += 1;
            graft_core::job::JobId::new(next)
        },
    ));
    let preview = state.preview.as_ref().expect("a preview is open");
    assert!(preview.rows.is_empty(), "the plan has not landed yet");

    let view = graph_view(&state, state.harness.clock.now_seconds());

    assert_eq!(
        view.hint_context,
        graft_ui::graph_screen::HintContext::Preview
    );
}

/// The box docks over the bottom of the graph, so the picture is shorter
/// than the screen the rows were scrolled on. With the cursor near the
/// bottom of that screen, starting the picture at the user's top row alone
/// would push their own commit off the end of it — the F1 defect turned
/// around. The window slides down by exactly as much as it must.
#[test]
fn a_preview_keeps_the_cursors_commit_on_screen_when_the_box_takes_rows() {
    const COUNT: u8 = 120;
    let (state, top, height) =
        scrolled_preview_with_cursor(COUNT, |height| 2 * height, 3, |height| height - 1);
    let cursor = top + height - 1;

    let frame = draw(&state, 100, 30);
    let drawn = commits_drawn(&frame);
    let id_at = |row: usize| u8::try_from(usize::from(COUNT) - row).expect("inside history");

    assert!(
        drawn.contains(&id_at(cursor)),
        "the commit the key was pressed on is on screen\n{frame}"
    );
    assert!(
        drawn.iter().max() <= Some(&id_at(top)),
        "and nothing above the user's own top row is\n{frame}"
    );
}

/// A window of `lanes` occupied columns, all passing straight down past a
/// node in lane 0: enough columns that the lane cap decides how many draw.
pub(crate) fn history_with_lanes(lanes: usize) -> History {
    let mut history = history_of(3, 0);
    let window = history.window.as_mut().expect("history_of built a window");
    let mut rows: Vec<graft_graph::row::Row> = window.rows.to_vec();
    for row in &mut rows {
        row.lanes = (0..lanes)
            .map(|lane| {
                Some(graft_graph::identity::BranchIdentity::of_name(
                    format!("lane-{lane}").as_bytes(),
                ))
            })
            .collect();
    }
    window.rows = rows.into();
    history
}

/// ADR-0039's 2026-09-21 amendment, third instance (m5/21 item 4): **ask
/// how much room this thing has, not how big the screen is.** The lane cap
/// came off the screen's width, so a graph with the detail pane beside it on
/// a 160-column terminal folded to the twelve lanes of a wide screen while
/// standing in 92 columns — a narrow view by DESIGN.md's own breakpoint.
/// The header's `+N lanes` is the count made visible: eight drawn columns
/// (seven lanes and the marker) hide seven of fourteen, twelve hide three.
#[test]
fn the_lane_cap_comes_from_the_graphs_room_not_the_screens_width() {
    use graft_ui::detail::DetailFocus;

    let mut state = AppState {
        history: Some(history_with_lanes(14)),
        size: Size {
            columns: 160,
            rows: 12,
        },
        ..AppState::default()
    };
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;

    assert_eq!(
        super::frame::graph_columns(&state, ratatui::layout::Rect::new(0, 0, 160, 12)),
        92,
        "the pane leaves the graph 92 of the 160 columns"
    );
    let with_pane = draw(&state, 160, 12);
    assert!(
        with_pane.contains("+7 lanes"),
        "92 columns is a narrow view: eight drawn columns, seven of the \
         fourteen lanes folded away:\n{with_pane}"
    );

    // The same screen with no pane really is wide, and keeps its twelve.
    state.detail.shown = false;
    let full_width = draw(&state, 160, 12);
    assert!(
        full_width.contains("+3 lanes"),
        "160 columns to itself folds at twelve lanes:\n{full_width}"
    );
}

// ---------------------------------------------------------------------------
// The hit-test measures what the frame draws (m5/33, review M1)
// ---------------------------------------------------------------------------

/// Where `needle` ended up on the drawn frame: the column its first cell
/// starts at, and the screen row.
///
/// Read off the cells the renderer actually wrote — nothing here asks
/// `graft_ui` where a badge goes, so a hit-test that disagrees with the
/// renderer is caught rather than agreed with.
fn drawn_at(
    session: &Session,
    state: &AppState,
    columns: u16,
    rows: u16,
    needle: &str,
) -> (u16, u16) {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| view(session, state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    for y in 0..rows {
        let mut line = String::new();
        let mut cells: Vec<(usize, u16)> = Vec::new();
        for x in 0..columns {
            cells.push((line.len(), x));
            line.push_str(buffer[(x, y)].symbol());
        }
        if let Some(byte) = line.find(needle) {
            let (_, x) = cells
                .into_iter()
                .find(|(at, _)| *at == byte)
                .expect("a needle starts at a cell boundary");
            return (x, y);
        }
    }
    panic!(
        "{needle:?} is nowhere on the frame:\n{}",
        terminal.backend()
    );
}

/// Two local branches on the newest commit of a graph with more lanes than
/// any cap draws: the graph column is twelve lanes wide as a 160-column
/// frame draws it and eight as `DEFAULT_MAX_LANES` would have it, so every
/// badge sits eight columns apart between the two answers.
fn two_branches_on_a_wide_graph() -> AppState {
    use graft_core::ref_name::RefName;
    use graft_git::repo::HeadState;

    let alpha = RefName::new(b"refs/heads/alpha".to_vec()).expect("a valid ref name");
    let bravo = RefName::new(b"refs/heads/bravo".to_vec()).expect("a valid ref name");
    let refs = crate::refs::badges(
        &[(alpha.clone(), oid(3)), (bravo, oid(3))],
        &HeadState::Branch(alpha),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("plain data always builds");

    AppState {
        history: Some(history_with_lanes(14)),
        refs: Some(std::sync::Arc::new(refs)),
        size: Size {
            columns: 160,
            rows: 12,
        },
        ..AppState::default()
    }
}

/// Where `name`'s badge is in `RefBadges::on`, which is the index
/// `badge_index_at` answers in.
fn badge_index_of(state: &AppState, name: &str) -> usize {
    state
        .refs
        .as_deref()
        .expect("the refs are on hand")
        .on(oid(3))
        .iter()
        .position(|label| {
            matches!(label, crate::refs::RefLabel::Branch { name: on, .. }
                if on.as_slice() == name.as_bytes())
        })
        .unwrap_or_else(|| panic!("{name} has a badge"))
}

/// M5 review M1: the hit-test built its **own** `GraphView` — `Icons::default()`
/// and `DEFAULT_MAX_LANES` — while the frame overrode both from the session.
/// With Nerd icons, or past eight lanes on a wide screen, the cells the user
/// sees and the cells the drag measures came from different views and a drag
/// landed on the wrong badge, or on none.
#[test]
fn a_drag_lands_on_the_badge_the_frame_drew_under_the_pointer() {
    let mut state = two_branches_on_a_wide_graph();
    state.icons = graft_ui::graph_screen::Icons::Nerd;
    let session = session();

    // First that the glyph set reaches the frame at all. Without this the
    // test is only about the two sides *agreeing*: drop `icons` from the
    // builder and both draw and measure the plain form, agree on it, and
    // every assertion below still holds — which is exactly the half of the
    // bug that shipped. `\u{e725}` is the branch glyph, drawn only for
    // `Icons::Nerd`; `[alpha]` is the plain form of the same badge.
    let frame = draw(&state, 160, 12);
    assert!(
        frame.contains('\u{e725}') && !frame.contains("[alpha]"),
        "the frame draws the Nerd badges `state.icons` asks for:\n{frame}"
    );

    for name in ["alpha", "bravo"] {
        let expected = badge_index_of(&state, name);
        let (column, row) = drawn_at(&session, &state, 160, 12, name);
        for offset in 0..u16::try_from(name.len()).expect("a short name") {
            assert_eq!(
                super::badge_index_at(
                    &state,
                    column + offset,
                    row,
                    state.harness.clock.now_seconds()
                ),
                Some(expected),
                "the frame drew {name} at column {}, so a drag there picks {name} up",
                column + offset
            );
        }
    }
}

/// A graph with real rows behind an overlay: the push state the credential
/// tests already build, with the detail pane closed and enough history that
/// every row beside the box carries a subject, a date and a SHA. Without
/// that content beside the box a "nothing survives beside it" assertion
/// passes on an empty screen and proves nothing.
fn graph_behind_a_box(columns: u16, rows: u16) -> AppState {
    let mut state = credential_state(columns, rows, "Password for 'http://user@127.0.0.1:8443':");
    state.detail.shown = false;
    state.on_working_tree = false;
    state.history = Some(history_of(30, 0));
    state.refs = Some(std::sync::Arc::new(refs_with_head(Some(oid(30)))));
    state
}

/// The same graph with `P`'s push confirm open over it and no credential
/// question.
fn confirm_over_graph(columns: u16, rows: u16) -> AppState {
    let mut state = graph_behind_a_box(columns, rows);
    state.credential = None;
    state.operating = None;
    state.remote.running = None;
    state.status = Status::Ready;
    assert!(
        crate::update::remote::push_key(&mut state),
        "P opens the push confirm"
    );
    state
}

/// `state` drawn into a terminal of exactly this size, read back as the
/// character grid — one `String` per cell, straight out of the buffer.
///
/// `draw` returns `TestBackend`'s debug rendering, which frames each row in
/// quotes and marks style changes inline; an assertion about *cells* has to
/// read cells, so this goes to the buffer instead.
fn draw_cells(state: &AppState, columns: u16, rows: u16) -> Vec<Vec<String>> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| view(&session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer().clone();
    (0..rows)
        .map(|y| {
            (0..columns)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

/// ADR-0040: the credential prompt clears the band it occupies, at every
/// width, rather than snapping onto the columns like the confirm box.
///
/// It fragments the short-SHA column when it does not — at 80 the four-hex
/// tails ` b8c5`, `68ff`, `00bd` sat against its right border, reading as
/// short identifiers that are not — and nothing behind it serves the task:
/// the prompt is modal (`docs/design/02-keymap.md` §2.14) and its own
/// border names the push it cancels (`esc cancel the push`).
///
/// **Fails before the change:** `draw_credential` cleared `rect` alone, so
/// the graph kept drawing its own columns on the box's own rows either side
/// of it.
#[test]
fn the_credential_prompt_clears_the_band_it_stands_on_at_every_width() {
    for columns in [40u16, 60, 80, 100, 120, 160, 200] {
        let rows = 24;
        let state = graph_behind_a_box(columns, rows);
        let grid = draw_cells(&state, columns, rows);
        let rect = graft_ui::credential_prompt::credential_area(Rect::new(0, 0, columns, rows));

        // The graph really is behind it: without content on the box's own
        // rows there is nothing for the band to have cleared.
        let behind = draw_cells(&graph_with_no_overlay(columns, rows), columns, rows);
        assert!(
            (rect.y..rect.bottom()).any(|y| behind
                .get(usize::from(y))
                .is_some_and(|row| row.iter().any(|cell| cell.trim() != ""))),
            "{columns}x{rows}: nothing is drawn on the prompt's rows, so this proves nothing"
        );

        for y in rect.y..rect.bottom() {
            let row = &grid[usize::from(y)];
            for (x, cell) in row.iter().enumerate() {
                let x = u16::try_from(x).expect("a screen is narrower than u16::MAX");
                if x >= rect.x && x < rect.right() {
                    continue;
                }
                assert_eq!(
                    cell, " ",
                    "{columns}x{rows}: row {y} column {x} still holds {cell:?} beside the \
                     credential prompt {rect:?}"
                );
            }
        }
    }
}

/// The same graph with nothing over it.
fn graph_with_no_overlay(columns: u16, rows: u16) -> AppState {
    let mut state = graph_behind_a_box(columns, rows);
    state.credential = None;
    state.confirm = None;
    state.operating = None;
    state.remote.running = None;
    state.status = Status::Ready;
    state
}

/// The confirm box as it was actually drawn, read back out of the picture:
/// the rectangle its own rounded border encloses. Read from the grid rather
/// than recomputed, so this test cannot agree with the layout site's
/// arithmetic by repeating it — and so a box that blanks cells outside its
/// visible border fails here rather than being excused.
fn drawn_box_rect(grid: &[Vec<String>]) -> Rect {
    let mut top = None;
    let mut bottom = None;
    let mut left = 0u16;
    let mut right = 0u16;
    for (y, row) in grid.iter().enumerate() {
        let y = u16::try_from(y).expect("a screen is shorter than u16::MAX");
        if let Some(x) = row.iter().position(|cell| cell == "\u{256d}") {
            top = Some(y);
            left = u16::try_from(x).expect("a screen is narrower than u16::MAX");
            right = u16::try_from(
                row.iter()
                    .rposition(|cell| cell == "\u{256e}")
                    .expect("the top border ends in a corner"),
            )
            .expect("a screen is narrower than u16::MAX");
        }
        if row.iter().any(|cell| cell == "\u{2570}") {
            bottom = Some(y);
        }
    }
    let top = top.expect("a box is drawn");
    let bottom = bottom.expect("a box is closed");
    Rect {
        x: left,
        y: top,
        width: right - left + 1,
        height: bottom - top + 1,
    }
}

/// ADR-0040: a box that does not clear **covers**; it never alters. Drawn
/// with and without the confirm box, every cell outside the box's own
/// rectangle is identical.
///
/// **This checks the character grid only.** `TestBackend` records symbols
/// and this assertion reads symbols, so a dimming or tinting layer painted
/// over the rows beside the box would pass here unseen — captures carry no
/// colour attributes (CLAUDE.md rule 5). The claim is exactly "no cell's
/// character changed", and no more.
#[test]
fn a_box_that_does_not_clear_covers_and_never_alters() {
    for columns in [80u16, 120, 160] {
        let rows = 24;
        let with = draw_cells(&confirm_over_graph(columns, rows), columns, rows);
        let without = draw_cells(&graph_with_no_overlay(columns, rows), columns, rows);
        let rect = drawn_box_rect(&with);
        for (y, (drawn, plain)) in with.iter().zip(&without).enumerate() {
            let y = u16::try_from(y).expect("a screen is shorter than u16::MAX");
            for (x, (a, b)) in drawn.iter().zip(plain).enumerate() {
                let x = u16::try_from(x).expect("a screen is narrower than u16::MAX");
                let inside = x >= rect.x && x < rect.right() && y >= rect.y && y < rect.bottom();
                // The footer is the box's too: it names the keys the box
                // answers, not the graph's (rulings 07b, Q1;
                // `view::frame::tests::the_bottom_row_under_a_live_box_offers_only_its_keys`).
                let footer = y + 1 == rows;
                if inside || footer {
                    continue;
                }
                assert_eq!(
                    a, b,
                    "{columns}x{rows}: cell ({x}, {y}) is outside the confirm box {rect:?} and \
                     changed from {b:?} to {a:?}"
                );
            }
        }
    }
}

/// ADR-0040 clause 1: the confirm box does **not** clear — the rows beside
/// it are the graph it is about to act on, and seeing them is the M5
/// promise — and its left edge sits on the subject column's own boundary.
///
/// Before this, at 80 columns, the box started at column 8 and left
/// `●   Comm` beside it: the subject column, split. It passed on the right
/// only because "Push main → origin/main" happens to be the length it is.
#[test]
fn the_confirm_box_keeps_the_graph_beside_it_and_starts_on_the_subject_column() {
    for columns in [80u16, 120, 160] {
        let rows = 24;
        let with = draw_cells(&confirm_over_graph(columns, rows), columns, rows);
        let without = draw_cells(&graph_with_no_overlay(columns, rows), columns, rows);
        let rect = drawn_box_rect(&with);

        // Where the subject column starts, read off untouched rows rather
        // than assumed: the leftmost cell any row's content reaches past
        // the graph glyph. A row's own text may start further right — a ref
        // badge pushes the subject along — so it is the minimum across the
        // rows that says where the column itself begins.
        //
        // The **marker slot** sits between the two and is not the subject
        // column: it carries the Mistake Radar's `⚠`/`ⓘ`, the compare base's
        // `▏` and a drop target's `→` (m8-polish-and-ship/07). Counting those
        // made this read 2 instead of 4 and fail — the box had not moved.
        // So they are skipped by name.
        // The graph glyph sits after the two-cell gutter (ADR-0058 §4).
        let past_glyph = usize::from(graft_ui::graph_screen::GUTTER) + 1;
        let subject_x = without
            .iter()
            .filter(|row| row.concat().contains("Commit"))
            .filter_map(|row| {
                row.iter().skip(past_glyph).position(|cell| {
                    cell.trim() != "" && !matches!(cell.as_str(), "▏" | "→" | "⚠" | "ⓘ")
                })
            })
            .map(|index| {
                u16::try_from(index + past_glyph).expect("a screen is narrower than u16::MAX")
            })
            .min()
            .expect("the graph draws subjects");

        assert_eq!(
            rect.x, subject_x,
            "{columns}x{rows}: the box starts at column {} and the subject column starts at \
             {subject_x} — the box splits it",
            rect.x
        );
        // It covers rather than clears: on the box's own rows the SHA column
        // is still drawn whole beside it — or, where the box's natural width
        // reaches past the SHA column's gutter, the box runs to the frame's
        // edge and covers that column whole (ADR-0040: whole columns or
        // clear). At 80 columns that is the case since the one-cell right
        // margin (m8-polish-and-ship/06, ADR-0058 §4) moved the SHA column
        // a cell left of the centred 64-wide box's edge. Either way the
        // lanes left of the box survive, which a cleared band would not.
        let row = &with[usize::from(rect.y) + 1];
        let beside: String = row[usize::from(rect.right())..].concat();
        let sha_survives = beside
            .trim()
            .chars()
            .filter(char::is_ascii_hexdigit)
            .count()
            >= 7;
        let covers_to_the_edge = rect.right() == columns && beside.trim().is_empty();
        assert!(
            sha_survives || covers_to_the_edge,
            "{columns}x{rows}: right of the box is neither the SHA whole nor the frame's \
             edge: {beside:?}"
        );
        let left_of_box: String = row[..usize::from(rect.x)].concat();
        assert!(
            !left_of_box.trim().is_empty(),
            "{columns}x{rows}: nothing survives left of the box — it cleared instead of \
             covering: {left_of_box:?}"
        );
    }
}

/// The same graph with `P`'s push confirm open, and the detail pane open
/// beside it — the swept state review Q1 asked for: no test before this
/// ticket drew a box over a graph with a pane beside it, only over a
/// full-frame graph (`confirm_over_graph`) or over an empty one
/// (`pane_boundaries.rs`, `graft-ui`, no `AppState` to open a pane in).
fn confirm_over_graph_with_detail_pane(columns: u16, rows: u16) -> AppState {
    let mut state = confirm_over_graph(columns, rows);
    state.detail.shown = true;
    state
}

/// Review Q1, drawn at 160 — the width it named, and the narrowest at which
/// the detail pane sits beside the graph rather than below or behind a tab
/// (`DetailLayout::RIGHT_BREAKPOINT`).
///
/// **At 160 the box straddles the whole pane**: the push confirm's natural
/// width (64, centred on the full frame) already reaches past the graph's
/// own share of 160 columns, so `snap_to_columns` picks the frame's own
/// right edge — where the ADR-0040 amendment's gutter step is correctly
/// waived because nothing is beside the box there at all. So at 160 this
/// sweeps the general "nothing touches" property but does not exercise the
/// boundary `row_columns` special-cases (`pane.right()`) — that needs the
/// box to stay inside the graph's own share, which
/// [`the_confirm_box_leaves_a_gutter_before_a_wider_detail_pane`] below
/// covers, and says so rather than mislabel this as the swept case.
#[test]
fn the_confirm_box_over_the_graph_still_covers_whole_columns_with_the_detail_pane_open() {
    let columns = 160;
    let rows = 24;
    let with = draw_cells(
        &confirm_over_graph_with_detail_pane(columns, rows),
        columns,
        rows,
    );
    let rect = drawn_box_rect(&with);
    assert_eq!(
        rect.right(),
        columns,
        "{columns}x{rows}: expected the box to straddle out to the frame's own edge at this \
         width (natural width 64 already exceeds the graph's own share) — it did not, so this \
         test no longer covers what its comment claims and needs re-checking against the \
         narrower case instead"
    );
    for (y, row) in with.iter().enumerate() {
        let y = u16::try_from(y).expect("a screen is shorter than u16::MAX");
        if y < rect.top() || y >= rect.bottom() {
            continue;
        }
        if rect.x > 0 {
            let beside = &row[usize::from(rect.x) - 1];
            assert_eq!(
                beside,
                " ",
                "{columns}x{rows}: row {y} column {} sits against the box's left border and \
                 holds {beside:?}",
                rect.x - 1
            );
        }
    }
}

/// Review Q1's actual case: the box's natural width (64) fits inside the
/// graph's own share of the screen, so it snaps onto a boundary *inside*
/// the graph pane rather than straddling out to the frame's edge — the one
/// case `pane_boundaries.rs`'s sweep (no `AppState`, no detail pane) and
/// `the_confirm_box_over_the_graph_still_covers_whole_columns_with_the_detail_pane_open`
/// (160, straddles fully) both miss.
///
/// 450 columns puts the graph's own share (`DetailLayout::Right`, 42%,
/// `detail_pane::layout::gutter`'s blank column subtracted) comfortably past
/// the push confirm's natural right edge, so the box's right border lands on
/// a real interior boundary — `graph.right()`, the pane's own edge — and
/// `row_columns` has to hand `snap_to_columns` the boundary that already sits
/// behind `detail_pane`'s own blank gutter column rather than the raw pane
/// edge, or the amendment's gutter step lands one cell short of it: on the
/// short-SHA column's own last character.
///
/// **Mutation that reproduces the fault:** revert `row_columns` to push
/// `area.x`/`area.right()` only, without dropping `pane.right()` in favour of
/// the neighbouring pane's own start — this fails at row 8 (the top border),
/// naming a non-blank cell immediately right of the box.
#[test]
fn the_confirm_box_leaves_a_gutter_before_a_wider_detail_pane() {
    let columns = 450;
    let rows = 24;
    let with = draw_cells(
        &confirm_over_graph_with_detail_pane(columns, rows),
        columns,
        rows,
    );
    let rect = drawn_box_rect(&with);
    assert!(
        rect.right() < columns,
        "{columns}x{rows}: expected the box to stay inside the graph's own share rather than \
         straddle to the frame's edge — it reached {}, so this width no longer exercises the \
         boundary this test pins and needs picking again",
        rect.right()
    );
    for (y, row) in with.iter().enumerate() {
        let y = u16::try_from(y).expect("a screen is shorter than u16::MAX");
        if y < rect.top() || y >= rect.bottom() {
            continue;
        }
        let beside = &row[usize::from(rect.right())];
        assert_eq!(
            beside,
            " ",
            "{columns}x{rows}: row {y} column {} sits against the box's right border, with the \
             detail pane open beside it, and holds {beside:?} rather than the blank gutter \
             `detail_pane::layout::gutter` already reserves there",
            rect.right()
        );
    }
}

/// The state ticket 34 was found in: a preview box docked over a graph that
/// still has lanes on the rows it covers, at a real terminal width.
fn preview_over_lanes(columns: u16, rows: u16) -> AppState {
    let (mut state, _top, _height) = scrolled_preview(120, |height| 2 * height, 3);
    state.size = Size { columns, rows };
    state
}

/// A `Session` whose graph draws in pixels, as Ghostty's does.
fn pixel_session() -> Session {
    let mut session = session();
    session.rendering.tier = graft_core::capabilities::RenderingTier::Pixel;
    session.capabilities.pixel_protocol = Some(graft_core::capabilities::PixelProtocol::Kitty);
    session
}

/// m5-history-and-ghost-preview/34. **The user's fault, and it is not in the
/// character grid at all.**
///
/// At the pixel tier the lane column is not drawn into the buffer:
/// `view_with_graphics` hands back the rectangle the lane images belong in
/// and `event_loop::running::draw` writes those images *after* the frame, so
/// the terminal composites them over whatever the frame already put there. A
/// box docked over the graph is drawn under them. At ~200 columns that put a
/// lane glyph on the preview box's own `│` at column 0 and on the `u` of
/// `undo:` at column 2 — "the box has no left border", and `ndo: u takes
/// this back`.
///
/// **Why every existing test was green.** `border_rules.rs` (ticket 30) and
/// `pane_boundaries.rs` (29) check cells in a `Buffer`, and every `.txt`
/// capture is a character grid: the pixel layer is in neither. The two
/// layers only meet on the user's screen, so the only thing a test can hold
/// is the *geometry* — that the rectangle handed to the graphics layer and
/// the rectangle a box drew into do not overlap.
///
/// **Failed before the fix**, at every size here — e.g. 200x50 returned
/// `Rect { x: 0, y: 1, width: 4, height: 48 }` for the lanes against a box
/// at `Rect { x: 0, y: 38, width: 200, height: 11 }`, overlapping on
/// `Rect { x: 0, y: 38, width: 4, height: 11 }`: eleven rows of box with
/// lane images over their first four columns.
#[test]
fn the_lane_images_never_cover_a_box_drawn_over_the_graph() {
    use crate::view::view_with_graphics;
    use graft_ui::graph_screen::GraphicsStatus;

    let session = pixel_session();
    for rows in [24u16, 30, 48, 50] {
        for columns in 20u16..=240 {
            let state = preview_over_lanes(columns, rows);
            let mut terminal = Terminal::new(TestBackend::new(columns, rows))
                .expect("TestBackend reports its size");
            let mut lanes = None;
            terminal
                .draw(|frame| {
                    lanes = view_with_graphics(&session, &state, &GraphicsStatus::Ok, frame);
                })
                .expect("TestBackend cannot fail to draw");
            let Some(lanes) = lanes else { continue };
            let view = crate::update::preview::preview_box(&state).expect("a preview is open");
            let boxed =
                graft_ui::preview_box::preview_box_area(Rect::new(0, 0, columns, rows), &view);
            let overlap = lanes.intersection(boxed);
            assert!(
                overlap.is_empty(),
                "{columns}x{rows}: the lane images go in {lanes:?} and the preview box drew in \
                 {boxed:?} — they share {overlap:?}, which the terminal paints lanes over"
            );
        }
    }
}

/// The same rule for the modal boxes: the confirm box and the credential
/// prompt stand over the graph too, and the lane images were written over
/// them the same way.
#[test]
fn the_lane_images_never_cover_the_confirm_box_or_the_credential_prompt() {
    use crate::view::view_with_graphics;
    use graft_ui::graph_screen::GraphicsStatus;

    let session = pixel_session();
    for rows in [24u16, 30, 48] {
        for columns in [40u16, 60, 80, 100, 120, 160, 200, 240] {
            for (name, state) in [
                ("confirm", confirm_over_graph(columns, rows)),
                ("credential", graph_behind_a_box(columns, rows)),
            ] {
                let mut terminal = Terminal::new(TestBackend::new(columns, rows))
                    .expect("TestBackend reports its size");
                let mut lanes = None;
                terminal
                    .draw(|frame| {
                        lanes = view_with_graphics(&session, &state, &GraphicsStatus::Ok, frame);
                    })
                    .expect("TestBackend cannot fail to draw");
                let Some(lanes) = lanes else { continue };
                // The box as it was really drawn, read back out of the
                // picture rather than recomputed.
                let grid = draw_cells(&state, columns, rows);
                if !grid.iter().any(|row| row.iter().any(|c| c == "\u{256d}")) {
                    continue;
                }
                let boxed = drawn_box_rect(&grid);
                let overlap = lanes.intersection(boxed);
                assert!(
                    overlap.is_empty(),
                    "{name} {columns}x{rows}: lanes in {lanes:?}, box in {boxed:?}, \
                     sharing {overlap:?}"
                );
            }
        }
    }
}

/// The character-grid half of the same ticket, kept as the floor: over a
/// graph that really has lanes, at every width from 20 to 240 and with the
/// detail pane and the overview strip both ways, the preview box's own left
/// border is a border glyph on every one of its rows.
///
/// **Why ticket 30's sweep could not see the user's fault.** Three reasons,
/// none of them width — it already sweeps 20..=240:
/// 1. `border_rules.rs` renders each widget into a bare `Buffer` of its own
///    with nothing behind it, so "the graph drew into my border cell" is not
///    a question it can ask.
/// 2. `pane_boundaries.rs`'s box sweep does draw a real graph, but it checks
///    only the cells *beside* `rect.x` and `rect.right()`, and skips the
///    left check entirely when `rect.x == frame.x` — which is where the
///    preview box always is, since `preview_box_area` takes the full frame
///    width and never calls `snap_to_columns` at all.
/// 3. Neither can see the pixel layer, which is where the fault actually
///    was — see [`the_lane_images_never_cover_a_box_drawn_over_the_graph`].
///
/// This test was **green before the fix as well**: it is here so the grid
/// half cannot regress while the pixel half is being held, not as evidence
/// for the ticket.
///
/// **Fast tier: every layout, not every width** (m6-07 review, Major 5).
/// The full sweep is 2 × 2 × 4 × 221 = 3,536 whole frames at ~3.2 ms each
/// in a debug build — 11.5 s, with a flat profile (graph rows ~40 %, the
/// state build ~20 %, buffer diff ~15 %): no slow path, just volume. What
/// can move the box's left column is the layout, and the layout changes
/// only at the width breakpoints, so the fast tier draws each side of each
/// one plus a stride across the rest. The every-width sweep is kept, in the
/// heavy tier: [`the_preview_box_keeps_its_left_border_at_every_width`].
#[test]
fn the_preview_box_keeps_its_own_left_border_over_a_graph_with_lanes() {
    left_border_holds_over(&layout_widths(), &[24, 50]);
}

/// The exhaustive sweep the fast tier samples: every width 20..=240 at four
/// heights. Heavy tier (TESTING.md), ~12 s in a debug build.
#[cfg(feature = "heavy-tests")]
#[test]
fn the_preview_box_keeps_its_left_border_at_every_width() {
    let every: Vec<u16> = (20u16..=240).collect();
    left_border_holds_over(&every, &[24, 30, 40, 50]);
}

/// Each side of every width at which the frame's layout changes, the two
/// ends, and every 13th width between so no stretch goes unsampled.
///
/// The detail pane's edges are read off `effective_layout` itself, at both
/// sampled heights, so a moved breakpoint moves the sample with it. The
/// graph screen's own edges — the author column (100) and the wide lane
/// budget (160, `graph_screen/layout.rs`'s `breakpoint_lanes`) — are named.
fn layout_widths() -> Vec<u16> {
    let state = preview_over_lanes(120, 24);
    // The kind only (tab, below, right): a right column's width moves with
    // every width, which is proportion, not a layout change.
    let layout = |columns: u16, rows: u16| {
        std::mem::discriminant(&crate::update::detail::effective_layout(
            state.detail.focus,
            &state.detail.pane_size,
            (columns, rows),
        ))
    };
    let mut edges = vec![graft_ui::graph_screen::AUTHOR_BREAKPOINT, 160];
    for rows in [24u16, 50] {
        edges.extend((21u16..=240).filter(|&w| layout(w, rows) != layout(w - 1, rows)));
    }
    let mut widths: Vec<u16> = (20u16..=240).step_by(13).chain([240]).collect();
    for edge in edges {
        widths.extend([edge - 1, edge, edge + 1]);
    }
    widths.retain(|w| (20..=240).contains(w));
    widths.sort_unstable();
    widths.dedup();
    widths
}

/// The preview box's left column is a border glyph on every one of its
/// rows, with the detail pane and the overview strip both ways, at each of
/// `widths` × `heights`.
fn left_border_holds_over(widths: &[u16], heights: &[u16]) {
    for detail in [false, true] {
        for strip in [false, true] {
            for &rows in heights {
                for &columns in widths {
                    let mut state = preview_over_lanes(columns, rows);
                    state.detail.shown = detail;
                    state.strip_shown = strip;
                    let grid = draw_cells(&state, columns, rows);
                    let Some(top) = grid.iter().position(|r| r.iter().any(|c| c == "\u{256d}"))
                    else {
                        continue;
                    };
                    let bottom = grid
                        .iter()
                        .rposition(|r| r.iter().any(|c| c == "\u{2570}"))
                        .expect("a box that opened is closed");
                    let left = grid[top]
                        .iter()
                        .position(|c| c == "\u{256d}")
                        .expect("the row holding the corner holds it");
                    for (y, row) in grid.iter().enumerate().take(bottom + 1).skip(top) {
                        let cell = &row[left];
                        assert!(
                            matches!(cell.as_str(), "\u{256d}" | "\u{2502}" | "\u{2570}"),
                            "detail={detail} strip={strip} {columns}x{rows}: the box's left \
                             column {left} holds {cell:?} on row {y}"
                        );
                    }
                }
            }
        }
    }
}

/// Every cell of a drawn frame, row by row, **with its style** —
/// [`draw_cells`]'s sibling for assertions about attributes rather than
/// characters. `TestBackend`'s `Buffer` carries `fg`, `bg` and `modifier`
/// per cell; rule 5 is about what a `.txt` *capture* can prove, not about
/// what a test can read (m5-history-and-ghost-preview/35).
fn drawn_cells(state: &AppState, columns: u16, rows: u16) -> Vec<Vec<ratatui::buffer::Cell>> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| view(&session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer().clone();
    (0..rows)
        .map(|y| (0..columns).map(|x| buffer[(x, y)].clone()).collect())
        .collect()
}

/// A graph with `feature`'s badge picked up off history row 0 and the
/// pointer over `over`, as the state is between the button going down and
/// coming back up.
fn mid_drag(columns: u16, rows: u16, over: Option<usize>) -> AppState {
    use crate::update::drag::{DragSource, DragState};

    let mut state = graph_with_no_overlay(columns, rows);
    state.size = Size { columns, rows };
    state.on_working_tree = false;
    let history = state.history.as_mut().expect("a history");
    history.top = 0;
    history.cursor = 1;
    state.drag = Some(DragState {
        source: DragSource {
            name: "refs/heads/feature".to_owned(),
            short: "feature".to_owned(),
            at: oid(30),
            current: false,
        },
        from_row: 0,
        over,
    });
    state
}

/// m5-history-and-ghost-preview/35B. **A drag in progress, in the rendered
/// buffer, before the button comes up.**
///
/// The user: *"when I'm dragging there is no indication that I have held
/// something, until I release and a popup shows up."* Both halves that were
/// supposed to answer him existed and both were tested —
/// `RowPaint::Highlighted` on the row under the pointer (ticket 10) and
/// `HintContext::Dragging`'s `release drop on a row · esc cancel` (swept by
/// 31). **The chain broke at its last hop, the rendered attribute**, and in
/// two places:
///
/// - `view/entries.rs` painted the drop target `RowPaint::Highlighted` —
///   the cursor row's own paint — and *replaced* the cursor's highlight
///   with it. So the whole of a drag, on screen, was one faint background
///   raise (`#14161B` → `#232730`) moving from one row to another.
/// - Nothing at all marked the badge as picked up. Before the pointer first
///   moved, `over` is `None`, so there was no target row either, and the
///   footer was the only thing on screen that had changed.
///
/// - Every drag test was on `AppState`, not on a frame:
///   `update/drag/tests.rs` and `tests/it/name_and_drag.rs` assert
///   `state.drag…over == Some(row)`, and the one capture in that file is of
///   the drop menu **after** release. Nothing rendered a frame while
///   `state.drag` was `Some`. This test does.
///
/// Three marks are asserted, which is what 35 chose to draw:
/// 1. the row the badge is **over** — `→` in the accent colour, its own
///    `RowPaint::DropTarget`, so it is not the cursor's mark;
/// 2. the row the badge came **from** — dimmed, because the badge is not
///    there any more, it is in the user's hand. This is the one that
///    appears on the button *down*, before any motion;
/// 3. the cursor row — still carrying its gutter `▸`, because it has not
///    moved and comes back when the badge is dropped.
///
/// Plus the hint bar, which was right all along.
#[test]
fn a_drag_in_progress_is_visible_in_the_buffer_before_the_button_comes_up() {
    use ratatui::style::Modifier;

    const COLUMNS: u16 = 120;
    const ROWS: u16 = 24;

    let state = mid_drag(COLUMNS, ROWS, Some(4));
    let grid = draw_cells(&state, COLUMNS, ROWS);
    let painted = drawn_cells(&state, COLUMNS, ROWS);
    let cells = |y: usize| &painted[y];

    // Screen row of an absolute history row, read off the picture rather
    // than assumed: the header takes row 0 and the working-tree row takes
    // the next, so history row 0 is two down, not one.
    let first = grid
        .iter()
        .position(|row| row.concat().contains("Uncommitted changes"))
        .expect("the working-tree row is drawn")
        + 1;
    let row_of = |history_row: usize| first + history_row;

    // 1. The target row: `→` in the marker slot, in the accent colour
    //    (m8-polish-and-ship/07).
    let theme = session().theme;
    // The target row is raised onto the fill, so its accent is lifted to
    // stay legible there (ADR-0058 §3, `Theme::on_selection`).
    let accent =
        graft_ui::paint::color(theme.on_selection(theme.token(graft_ui::theme::Token::Accent)));
    let target = cells(row_of(4));
    let mark = target
        .iter()
        .position(|cell| cell.symbol() == "→")
        .expect("the row under the pointer is marked");
    assert_eq!(
        target[mark].fg, accent,
        "the drop mark is drawn in the accent colour, lifted onto the fill"
    );
    // And it is *not* the cursor's mark, so the two read as two things.
    assert!(
        !target.iter().any(|cell| cell.symbol() == "▸"),
        "the target row does not also carry the cursor's `▸`"
    );

    // 2. The source row is dimmed — the mark that lands on the button down.
    //    Asserted on the subject text, which is the row's own words: the
    //    lane glyph and the ref badge carry branch colours of their own and
    //    take the paint's modifier through their own styles.
    let subject_at = |y: usize| {
        grid[y]
            .iter()
            .position(|cell| cell.as_str() == "C")
            .expect("every row here has a `Commit N` subject")
    };
    let source = cells(row_of(0));
    assert!(
        source[subject_at(row_of(0))]
            .modifier
            .contains(Modifier::DIM),
        "the subject of the row the badge came from is dim:\n{:?}",
        grid[row_of(0)].concat()
    );
    let untouched = cells(row_of(2));
    assert!(
        !untouched[subject_at(row_of(2))]
            .modifier
            .contains(Modifier::DIM),
        "a row with nothing to do with the drag is not:\n{:?}",
        grid[row_of(2)].concat()
    );

    // 3. The cursor row keeps its gutter `▸` while the badge is in the air
    //    (m8-polish-and-ship/07: the cursor's mark is the gutter's cell 0).
    let cursor = cells(row_of(1));
    assert!(
        cursor[0].symbol() == "▸",
        "the cursor row is still findable during a drag:\n{:?}",
        grid[row_of(1)].concat()
    );
}

/// The fourth mark, and the one half of the chain that was right all along:
/// `HintContext::Dragging`'s bar. It reached the frame; it was simply the
/// only thing on screen that changed, and the footer is not where a hand on
/// the mouse is looking.
#[test]
fn the_hint_bar_names_the_gesture_while_a_badge_is_in_the_air() {
    const COLUMNS: u16 = 120;
    const ROWS: u16 = 24;

    let grid = draw_cells(&mid_drag(COLUMNS, ROWS, Some(4)), COLUMNS, ROWS);
    let footer = grid[usize::from(ROWS) - 1].concat();
    assert!(
        footer.contains("release") && footer.contains("drop on a row"),
        "the footer names the gesture: {footer:?}"
    );
    assert!(footer.contains("esc"), "and the way out: {footer:?}");
}

/// The button-down half on its own: with nothing yet under the pointer
/// (`over: None`, which is the state a press produces before any motion),
/// the screen still says a badge has been picked up.
///
/// This is the exact moment the user was describing. Before 35 the only
/// thing that changed on it was the footer.
#[test]
fn picking_a_badge_up_changes_the_screen_before_the_pointer_moves() {
    const COLUMNS: u16 = 120;
    const ROWS: u16 = 24;

    let mut state = mid_drag(COLUMNS, ROWS, None);
    let holding = state.drag.take();
    let before = draw(&state, COLUMNS, ROWS);
    state.drag = holding;
    let after = draw(&state, COLUMNS, ROWS);

    assert_ne!(
        before, after,
        "the press alone changes the picture — the source row fades and the footer swaps"
    );
}

/// A dead key found by review M6's width sweep once the fixtures were built
/// at the width whose bar they were checking (m5/33): under 100 columns the
/// detail pane is a **tab strip**, so there is no divider to drag — and the
/// graph screen's own footer went on offering `<>` because it read
/// "a pane is open" as "a divider is there". `view::frame` says the right
/// thing where it draws the Detail tab's bar; this is the Graph tab, which
/// keeps the graph screen's footer.
#[test]
fn the_resize_key_is_offered_only_where_there_is_a_divider_to_drag() {
    use graft_ui::detail::DetailFocus;

    let mut state = AppState {
        history: Some(history_of(5, 1)),
        size: Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Graph;

    let tabbed = draw(&state, 80, 24);
    assert!(
        tabbed.contains("[Graph]"),
        "80 columns with the pane open is the tab layout:\n{tabbed}"
    );
    assert!(
        !tabbed.contains("</>"),
        "the tab layout has no divider, so `<>` must not be offered:\n{tabbed}"
    );

    state.size = Size {
        columns: 160,
        rows: 24,
    };
    let split = draw(&state, 160, 24);
    assert!(
        split.contains("</>"),
        "160 columns is a real split, and its divider is draggable:\n{split}"
    );
}

mod bottom_row;
mod credential_footer;

/// m7-power-tools/07b: at a rewrite's `edit` stop `HEAD`'s row is tagged
/// `HEAD · stopped here`, from the stop itself — no other row, and no tag
/// once the stop is a conflict stop.
#[test]
fn the_edit_stop_tags_head_s_row_stopped_here() {
    use graft_ui::graph_screen::RowTag;
    let mut state = AppState {
        history: Some(history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs_with_head(Some(oid(3))))),
        size: Size {
            columns: 80,
            rows: 6,
        },
        ..AppState::default()
    };
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Rebase,
        step: Some((1, 2)),
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    state.stopped = Some(crate::update::Stop {
        origin: crate::update::LiveOrigin::ThisRun,
        again: false,
        kind: graft_git::state::InProgress::Rebase,
        paths_at_stop: 0,
        rewrite: Some(Ok(crate::update::banner::RewriteStop {
            progress: graft_ops::rewrite::RewriteProgress {
                step: (1, 2),
                current: None,
                pending: Vec::new(),
                branch: None,
                begun_in_graft: true,
                followers: Vec::new(),
                fork: None,
            },
            edit: Some(graft_ops::rewrite::EditStop {
                stopped: oid(3),
                head_on_it: true,
            }),
        })),
    });
    let view = graph_view(&state, state.harness.clock.now_seconds());
    let tags: Vec<Option<RowTag>> = view.rows.iter().map(|row| row.tag.clone()).collect();
    assert_eq!(tags, vec![Some(RowTag::StoppedHere), None, None]);

    // A conflict stop: no `edit` read, no tag.
    if let Some(stop) = state.stopped.as_mut() {
        stop.rewrite = None;
    }
    let view = graph_view(&state, state.harness.clock.now_seconds());
    assert!(view.rows.iter().all(|row| row.tag.is_none()));
}
