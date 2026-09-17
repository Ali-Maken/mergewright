use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::{JobFailure, JobId};

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_git::history::Order;
use graft_git::version::SystemGit;
use graft_graph::{Follow, Row};

use super::*;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use crate::history::{
    History, HistoryUpdate, Phase, Promotion, RowRequest, View, WalkFacts, Wanted, Window,
};
use crate::repository::{Head, RepositoryFacts};
use crate::search::{Search, SearchState};
use graft_ui::graph_screen::{CommitGraphLine, OrderNotice};

fn identity() -> Identity {
    Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    }
}

fn key(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

#[test]
fn graft_opens_on_the_graph_and_f2_swaps_to_the_status_screen_and_back() {
    let state = AppState::default();
    assert_eq!(state.screen, Screen::Graph);

    let (status, commands) = update(&state, key(KeyCode::Function(2)));
    assert_eq!(status.screen, Screen::Status);
    assert!(commands.is_empty(), "{commands:?}");

    let (graph, _) = update(&status, key(KeyCode::Function(2)));
    assert_eq!(graph.screen, Screen::Graph);
}

/// `F3` from `settled` and on, `presses` times: the override after each.
fn f3_presses(settled: Option<RenderingTier>, presses: usize) -> Vec<Option<RenderingTier>> {
    let mut state = AppState {
        settled_tier: settled,
        ..AppState::default()
    };
    (0..presses)
        .map(|_| {
            let (next, commands) = update(&state, key(KeyCode::Function(3)));
            assert!(commands.is_empty(), "a view-time flag: {commands:?}");
            state = next;
            state.renderer_override
        })
        .collect()
}

#[test]
fn f3_on_a_unicode_terminal_toggles_unicode_and_ascii() {
    // No session (`settled_tier` None) counts as Unicode, the same way.
    for settled in [Some(RenderingTier::Unicode), None] {
        assert_eq!(
            f3_presses(settled, 3),
            vec![Some(RenderingTier::Ascii), None, Some(RenderingTier::Ascii)],
            "back at the top the override clears: nothing extra to say"
        );
    }
}

#[test]
fn f3_on_an_ascii_only_terminal_does_nothing() {
    assert_eq!(f3_presses(Some(RenderingTier::Ascii), 2), vec![None, None]);
}

#[test]
fn at_the_pixel_tier_history_asks_for_whole_strips() {
    let state = AppState {
        settled_tier: Some(RenderingTier::Pixel),
        ..AppState::default()
    };
    let (started, _) = update(&state, AppEvent::Started(JobId::new(1), Task::ReadHistory));
    let history = started.history.expect("a walk started");
    assert_eq!(
        history.strip_rows,
        Some(graft_graph::raster::cache::STRIP_ROWS)
    );

    let (unicode, _) = update(
        &AppState::default(),
        AppEvent::Started(JobId::new(1), Task::ReadHistory),
    );
    assert_eq!(unicode.history.expect("a walk started").strip_rows, None);
}

#[test]
fn f3_cycles_pixels_unicode_ascii_and_back_to_whole_strips() {
    let state = AppState {
        settled_tier: Some(RenderingTier::Pixel),
        ..AppState::default()
    };
    let (started, _) = update(&state, AppEvent::Started(JobId::new(1), Task::ReadHistory));
    let strip_rows = |state: &AppState| state.history.as_ref().and_then(|h| h.strip_rows);

    let (unicode, _) = update(&started, key(KeyCode::Function(3)));
    assert_eq!(unicode.tier(), RenderingTier::Unicode);
    assert_eq!(strip_rows(&unicode), None);

    let (ascii, _) = update(&unicode, key(KeyCode::Function(3)));
    assert_eq!(ascii.tier(), RenderingTier::Ascii);
    assert_eq!(strip_rows(&ascii), None);

    let (pixel, _) = update(&ascii, key(KeyCode::Function(3)));
    assert_eq!(pixel.renderer_override, None, "the settled tier again");
    assert_eq!(pixel.tier(), RenderingTier::Pixel);
    assert_eq!(
        strip_rows(&pixel),
        Some(graft_graph::raster::cache::STRIP_ROWS),
        "requests pad to whole strips again"
    );
}

/// A walk in flight whose window holds a plain commit (row 0) and a
/// merge (row 1), the cursor on row `cursor`.
fn merge_on_screen(id: JobId, cursor: usize) -> (AppState, Oid) {
    let commit = |byte: u8, parents: Vec<Oid>| Row {
        commit: Commit {
            id: Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
            parents,
            author: identity(),
            committer: identity(),
            message: b"a commit".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    };
    let parent = |byte| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("an oid");
    let plain = commit(1, vec![parent(2)]);
    let merge = commit(2, vec![parent(3), parent(4)]);
    let merge_id = merge.commit.id;
    let window = Window {
        request: 1,
        first: 0,
        rows: vec![plain, merge].into(),
        laid_out: 2,
        done: false,
        promoted: false,
    };
    let (mut state, _) = update(
        &awaiting(id),
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    state.history.as_mut().expect("a walk").cursor = cursor;
    (state, merge_id)
}

/// The view the last `ShowRows` asks for.
fn asked_view(commands: &[Command]) -> View {
    match commands {
        [Command::ShowRows(_, request)] => request.view.clone(),
        other => panic!("expected one row request, got {other:?}"),
    }
}

#[test]
fn space_folds_the_merge_under_the_cursor_and_unfolds_it() {
    let id = JobId::new(5);
    let (state, merge) = merge_on_screen(id, 1);

    let (folded, commands) = update(&state, key(KeyCode::Char(' ')));
    let view = asked_view(&commands);
    assert_eq!(view.inclusion.folded, [merge].into());
    assert_eq!(folded.history.as_ref().map(|h| &h.view), Some(&view));
    assert_eq!(
        folded.history.as_ref().map(|h| h.cursor),
        Some(1),
        "stays put"
    );

    let (unfolded, commands) = update(&folded, key(KeyCode::Char(' ')));
    assert_eq!(asked_view(&commands), View::default());
    assert_eq!(unfolded.history.map(|h| h.view), Some(View::default()));
}

#[test]
fn space_off_a_merge_changes_nothing() {
    let id = JobId::new(5);
    let (state, _) = merge_on_screen(id, 0);
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.history, state.history);
}

#[test]
fn one_switches_to_first_parent_view_from_the_top_and_back() {
    let id = JobId::new(6);
    let (state, _) = merge_on_screen(id, 1);

    let (straight, commands) = update(&state, key(KeyCode::Char('1')));
    assert_eq!(asked_view(&commands).follow, Follow::FirstParent);
    let history = straight.history.as_ref().expect("a walk");
    assert_eq!((history.top, history.cursor), (0, 0));

    let (full, commands) = update(&straight, key(KeyCode::Char('1')));
    assert_eq!(asked_view(&commands), View::default());
    assert_eq!(full.history.map(|h| h.view), Some(View::default()));
}

/// Every event clones the state (`update` returns a new one), so a large
/// immutable part must be shared by that clone, never copied: with 50k
/// refs a copy and its drop cost a millisecond a frame (m1-the-graph/69).
#[test]
fn the_next_state_shares_the_ref_badges_instead_of_copying_them() {
    let (state, _) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(3),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (next, _) = update(&state, key(KeyCode::Char('j')));

    let before = state.refs.as_ref().expect("the refs answer was taken");
    let after = next.refs.as_ref().expect("the badges carried over");
    assert!(Arc::ptr_eq(before, after), "the badges were copied");
}

#[test]
fn capital_h_hides_the_stash_and_shows_it_again() {
    let id = JobId::new(7);
    let (mut state, _) = merge_on_screen(id, 0);
    let (none, commands) = update(&state, key(KeyCode::Char('H')));
    assert!(
        commands.is_empty(),
        "no stash, nothing to hide: {commands:?}"
    );
    assert_eq!(none.history, state.history);

    let stash = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("an oid");
    let name = graft_core::ref_name::RefName::new("refs/stash").expect("valid");
    let badges = crate::refs::badges(
        &[(name, stash)],
        &graft_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        |_, _| Err("no upstreams here".to_owned()),
    )
    .expect("badges");
    assert_eq!(badges.stash(), Some(stash));
    state.refs = Some(Arc::new(badges));

    let (hidden, commands) = update(&state, key(KeyCode::Char('H')));
    assert_eq!(
        asked_view(&commands).inclusion.hidden_stashes,
        [stash].into()
    );
    let (shown, commands) = update(&hidden, key(KeyCode::Char('H')));
    assert_eq!(asked_view(&commands), View::default());
    assert_eq!(shown.history.map(|h| h.view), Some(View::default()));
}

#[test]
fn plus_expands_and_collapses_the_lanes_without_touching_the_walk() {
    let id = JobId::new(3);
    let (state, _) = update(&awaiting(id), rows(id, 4));
    let before = state.history.clone();

    let (expanded, commands) = update(&state, key(KeyCode::Char('+')));
    assert!(expanded.lanes_expanded);
    assert!(
        commands.is_empty(),
        "expanding is a view decision: {commands:?}"
    );
    assert_eq!(expanded.history, before, "the rows are not laid out again");

    let (folded, commands) = update(&expanded, key(KeyCode::Char('+')));
    assert!(!folded.lanes_expanded);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(folded.history, before);
}

#[test]
fn capital_f_toggles_focus_and_esc_leaves_it_without_touching_the_walk() {
    let id = JobId::new(3);
    let (state, _) = update(&awaiting(id), rows(id, 4));
    let before = state.history.clone();

    let (focused, commands) = update(&state, key(KeyCode::Char('F')));
    assert!(focused.focus);
    assert!(commands.is_empty(), "focus is paint: {commands:?}");
    assert_eq!(focused.history, before);

    let (left, commands) = update(&focused, key(KeyCode::Escape));
    assert!(!left.focus);
    assert!(commands.is_empty(), "Esc leaves focus first: {commands:?}");
    assert_eq!(left.awaiting, Some(id), "the walk goes on");
    assert_eq!(left.history, before);
}

#[test]
fn a_click_selects_the_row_under_the_pointer() {
    let id = JobId::new(20);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);

    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(1));
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id)
    );
}

#[test]
fn a_click_under_a_harness_banner_lands_on_the_row_the_banner_pushed_down() {
    let id = JobId::new(20);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 7,
    };
    state.harness.banner = Some(Banner {
        title: String::from("gallery 3/23"),
        lines: vec![String::from("shows: a merge")],
    });
    assert_eq!(state.graph_rows(), 3, "header, two banner rows, footer");

    // The screen row a click on the header would have hit without the
    // banner is now the banner's title: nothing selected.
    let on_banner = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, on_banner);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 2 + 1,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id)
    );
}

#[test]
fn the_wheel_scrolls_the_viewport_and_leaves_the_selection_alone() {
    let id = JobId::new(21);
    let (mut state, _) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };

    let scroll_down = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: Modifiers::NONE,
    }));
    let (down, _) = update(&state, scroll_down);
    assert_eq!(down.history.as_ref().map(|history| history.top), Some(1));
    assert_eq!(down.history.as_ref().map(|history| history.cursor), Some(0));

    let scroll_up = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: Modifiers::NONE,
    }));
    let (up, _) = update(&down, scroll_up);
    assert_eq!(up.history.as_ref().map(|history| history.top), Some(0));
}

#[test]
fn hover_names_the_row_under_the_pointer() {
    let id = JobId::new(22);
    let (mut state, _) = merge_on_screen(id, 1);
    state.size = Size {
        columns: 80,
        rows: 5,
    };

    let hover = core(Event::Mouse(MouseInput {
        kind: MouseKind::Moved,
        column: 5,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (hovered, _) = update(&state, hover);
    assert_eq!(hovered.hover, Some(1), "hover names the row under it");
}

/// `y` on the selected row copies its full SHA and says what was sent
/// (ADR-0023 decision 4).
#[test]
fn y_on_a_selected_row_copies_the_full_sha_and_says_so() {
    let (state, _) = merge_on_screen(JobId::new(31), 0);
    let (copied, commands) = update(&state, key(KeyCode::Char('y')));
    let sha = "01".repeat(20);
    assert_eq!(commands, vec![Command::CopyToClipboard(sha)]);
    assert_eq!(
        copied.status,
        Status::Copied {
            short: "0101010".to_owned()
        }
    );
}

#[test]
fn y_with_no_rows_sends_nothing_and_leaves_the_status_alone() {
    let state = AppState::default();
    let (next, commands) = update(&state, key(KeyCode::Char('y')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.status, Status::Ready);
}

#[test]
fn the_next_key_clears_the_copy_notice() {
    let (state, _) = merge_on_screen(JobId::new(32), 0);
    let (copied, _) = update(&state, key(KeyCode::Char('y')));
    let (moved, _) = update(&copied, key(KeyCode::Char('j')));
    assert_eq!(moved.status, Status::Ready);
}

/// A failure is never dropped for a notice: the SHA is still sent, the
/// failure stays on the status line.
#[test]
fn y_sends_the_sha_but_keeps_a_failure_on_screen() {
    let (mut state, _) = merge_on_screen(JobId::new(33), 0);
    state.status = Status::Failed("the refs could not be read".to_owned());
    let (next, commands) = update(&state, key(KeyCode::Char('y')));
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(next.status, state.status);
}

/// `x` on the selected row pins it as a compare base (m2-details-and-diffs/11):
/// no command yet, nothing to open until a second row is picked.
#[test]
fn x_on_a_row_with_no_base_pins_it_and_sends_nothing() {
    let (state, _) = merge_on_screen(JobId::new(40), 0);
    let plain_id = Oid::from_bytes(&[1; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let (next, commands) = update(&state, key(KeyCode::Char('x')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        next.compare_base.map(|base| base.oid),
        Some(plain_id),
        "row 0 is the plain commit"
    );
}

/// `x` again on the same row clears the pinned base.
#[test]
fn x_on_the_same_row_again_clears_the_base() {
    let (state, _) = merge_on_screen(JobId::new(41), 0);
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let (cleared, commands) = update(&pinned, key(KeyCode::Char('x')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(cleared.compare_base, None);
}

/// `x` on a second row opens the compare and keeps the base pinned, so the
/// pane can keep following the cursor.
#[test]
fn x_on_a_different_row_opens_the_compare_and_keeps_the_base() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(42), 0);
    state.focus = true;
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let base_id = pinned.compare_base.map(|base| base.oid);

    let (moved, _) = update(&pinned, key(KeyCode::Char('j')));
    let (opened, commands) = update(&moved, key(KeyCode::Char('x')));

    assert!(!commands.is_empty(), "opening a compare asks for its diff");
    assert_eq!(opened.compare_base.map(|base| base.oid), base_id);
    assert!(opened.detail.shown);
    assert_eq!(
        opened.detail.spec,
        Some(DetailSpec::Compare {
            base: base_id.expect("pinned above"),
            head: merge_id,
        })
    );
}

/// `x` on the working-tree row is a no-op: a base can only be a commit.
#[test]
fn x_on_the_working_tree_row_is_a_no_op() {
    let (mut state, _) = merge_on_screen(JobId::new(43), 0);
    state.on_working_tree = true;
    let (next, commands) = update(&state, key(KeyCode::Char('x')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.compare_base, None);
}

/// `Esc` clears a pinned base and, when the pane is open, closes it in the
/// same press (ADR-0023's existing `Escape` order).
#[test]
fn esc_clears_a_pinned_base_and_closes_the_open_pane() {
    let (state, _) = merge_on_screen(JobId::new(44), 0);
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let (moved, _) = update(&pinned, key(KeyCode::Char('j')));
    let (opened, _) = update(&moved, key(KeyCode::Char('x')));
    assert!(opened.detail.shown);

    let (closed, _) = update(&opened, key(KeyCode::Escape));
    assert_eq!(closed.compare_base, None);
    assert!(!closed.detail.shown);
}

/// `Shift`+click on a second row does the same as `x`.
#[test]
fn shift_click_on_a_second_row_opens_the_compare() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(45), 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    }));
    let (next, commands) = update(&pinned, click);
    assert!(!commands.is_empty(), "opening a compare asks for its diff");
    assert!(next.detail.shown);
    assert_eq!(
        next.detail.spec,
        Some(DetailSpec::Compare {
            base: pinned.compare_base.expect("pinned above").oid,
            head: merge_id,
        })
    );
}

/// `Ctrl`+click does the same as `Shift`+click.
#[test]
fn ctrl_click_on_a_second_row_opens_the_compare() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(46), 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }));
    let (next, commands) = update(&pinned, click);
    assert!(!commands.is_empty(), "opening a compare asks for its diff");
    assert_eq!(
        next.detail.spec,
        Some(DetailSpec::Compare {
            base: pinned.compare_base.expect("pinned above").oid,
            head: merge_id,
        })
    );
}

/// Moving the selection inside the rows on hand asks the history job for
/// nothing: the thread and its fade are worked out from those rows, so a
/// selection change makes no layout or walk call.
#[test]
fn a_selection_change_inside_the_window_asks_for_no_rows() {
    let id = JobId::new(3);
    let (mut state, _) = update(&awaiting(id), rows_for(id, 1, 0..30, 500));
    state.focus = true;
    for code in [KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Char('k')] {
        let (moved, commands) = update(&state, key(code));
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
        state = moved;
    }
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));
}

fn core(event: Event) -> AppEvent {
    AppEvent::Core(event)
}

fn facts(path: &str, branch: &str) -> RepositoryFacts {
    RepositoryFacts {
        path: PathBuf::from(path),
        bare: false,
        head: Head::Branch(branch.as_bytes().to_vec()),
        mailmap: graft_git::mailmap::Mailmap::default(),
        commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
    }
}

#[test]
fn q_quits() {
    let (next, commands) = update(&AppState::default(), key(KeyCode::Char('q')));

    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(commands.is_empty());
}

#[test]
fn control_c_quits() {
    let event = core(Event::Key(KeyPress::control(KeyCode::Char('c'))));

    let (next, _) = update(&AppState::default(), event);

    assert_eq!(next.quit, Some(Quit::Requested));
}

#[test]
fn control_q_is_not_quit() {
    // Only the bare `q` is bound. A modified one belongs to whatever
    // binding claims it later, and must not fall through to quitting.
    let event = core(Event::Key(KeyPress::control(KeyCode::Char('q'))));

    let (next, _) = update(&AppState::default(), event);

    assert_eq!(next.quit, None);
}

#[test]
fn a_bare_c_is_not_quit() {
    let (next, _) = update(&AppState::default(), key(KeyCode::Char('c')));

    assert_eq!(next.quit, None);
}

#[test]
fn unbound_events_change_nothing() {
    let before = AppState::default();
    let unbound = [
        key(KeyCode::Char('z')),
        key(KeyCode::Function(7)),
        core(Event::Paste("hello".to_owned())),
        core(Event::FocusGained),
        core(Event::FocusLost),
        core(Event::JobCancelled(JobId::new(1))),
    ];

    for event in unbound {
        let (next, commands) = update(&before, event.clone());

        assert_eq!(next, before, "{event:?} changed the state");
        assert!(commands.is_empty(), "{event:?} asked for work");
    }
}

#[test]
fn resize_is_remembered_including_a_degenerate_one() {
    let (next, _) = update(
        &AppState::default(),
        core(Event::Resize {
            columns: 0,
            rows: 0,
        }),
    );

    assert_eq!(
        next.size,
        Size {
            columns: 0,
            rows: 0
        }
    );

    let (next, _) = update(
        &next,
        core(Event::Resize {
            columns: 120,
            rows: 40,
        }),
    );

    assert_eq!(
        next.size,
        Size {
            columns: 120,
            rows: 40
        }
    );
}

#[test]
fn a_failed_job_reaches_the_status_line() {
    let failure = JobFailure::Failed("not a git repository".to_owned());

    let (next, commands) = update(
        &AppState::default(),
        core(Event::JobFailed(JobId::new(9), failure)),
    );

    assert_eq!(
        next.status,
        Status::Failed("job 9 failed: not a git repository".to_owned())
    );
    assert!(commands.is_empty());
}

#[test]
fn a_panicking_job_says_so_and_names_its_report() {
    let failure = JobFailure::Panicked {
        message: "lane 3 vanished".to_owned(),
        report: Some("/state/graft/crash-1.log".to_owned()),
    };

    let (next, _) = update(
        &AppState::default(),
        core(Event::JobFailed(JobId::new(2), failure)),
    );

    assert_eq!(
        next.status,
        Status::Failed(
            "job 2 failed: panicked: lane 3 vanished — report: /state/graft/crash-1.log".to_owned()
        )
    );
}

#[test]
fn a_job_that_worked_clears_a_stale_failure() {
    let failed = AppState {
        status: Status::Failed("job 1 failed: not a git repository".to_owned()),
        ..AppState::default()
    };

    let (next, _) = update(&failed, core(Event::JobFinished(JobId::new(2))));

    assert_eq!(next.status, Status::Ready);
}

#[test]
fn a_cancelled_job_leaves_the_status_line_alone() {
    // Cancellation is something Graft asked for. Reporting it would put an
    // error in front of the user when nothing is wrong (CLAUDE.md).
    let failed = AppState {
        status: Status::Failed("job 1 failed: not a git repository".to_owned()),
        ..AppState::default()
    };

    let (next, _) = update(&failed, core(Event::JobCancelled(JobId::new(1))));

    assert_eq!(next, failed);
}

#[test]
fn control_r_asks_for_the_repository_to_be_re_read() {
    let state = AppState {
        repo_path: PathBuf::from("/code/api"),
        ..AppState::default()
    };

    let (next, commands) = update(
        &state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );

    assert_eq!(
        commands,
        vec![
            Command::OpenRepository,
            Command::ReadHistory(Order::Topological),
            Command::ReadRefs,
            Command::ReadWorkingTree,
        ]
    );
    // Asking for work is all it does: the state is the job's to change,
    // once the job comes back.
    assert_eq!(next, state);
}

#[test]
fn a_bare_r_asks_for_nothing() {
    // `r` is rebase on the history screen (DESIGN.md §2.3), not refresh.
    let (_, commands) = update(&AppState::default(), key(KeyCode::Char('r')));

    assert!(commands.is_empty());
}

#[test]
fn the_same_event_always_gives_the_same_answer() {
    // Purity's observable half: no clock, no counter, no hidden state, so
    // repeating a call repeats the answer.
    let state = AppState {
        quit: None,
        harness: Harness::default(),
        status: Status::Failed("earlier".to_owned()),
        size: Size {
            columns: 80,
            rows: 24,
        },
        repo_path: PathBuf::from("/code/api"),
        repository: Some(facts("/code/api", "main")),
        system_git: None,
        commit_graph: CommitGraphOffer::NotNeeded,
        awaiting: None,
        promoting: None,
        history: None,
        refs: None,
        working_tree: None,
        screen: Screen::Graph,
        lanes_expanded: false,
        focus: false,
        hover: None,
        detail: DetailState::default(),
        local_jobs: LocalJobs::default(),
        renderer_override: None,
        settled_tier: None,
        search_prompt: None,
        search: None,
        on_working_tree: false,
        compare_base: None,
        context_menu: None,
        radar: crate::update::radar::RadarState::default(),
        radar_column_seen: std::cell::Cell::new(false),
    };

    let first = update(&state, key(KeyCode::Char('q')));
    let second = update(&state, key(KeyCode::Char('q')));

    assert_eq!(first, second);
    assert_eq!(state.quit, None, "the input state was mutated");
}

#[test]
fn an_opened_repository_becomes_what_the_screen_shows() {
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(1),
            Product::Repository(Ok(facts("/code/api", "main"))),
        ),
    );

    assert_eq!(next.repository, Some(facts("/code/api", "main")));
    assert_eq!(next.status, Status::Ready);
    assert!(commands.is_empty());
}

#[test]
fn a_repository_that_opened_clears_an_earlier_failure() {
    let failed = AppState {
        status: Status::Failed("could not re-read the repository".to_owned()),
        repository: Some(facts("/code/api", "main")),
        ..AppState::default()
    };

    let (next, _) = update(
        &failed,
        AppEvent::Produced(
            JobId::new(2),
            Product::Repository(Ok(facts("/code/api", "release"))),
        ),
    );

    assert_eq!(next.repository, Some(facts("/code/api", "release")));
    assert_eq!(next.status, Status::Ready);
}

#[test]
fn a_path_that_is_not_a_repository_ends_the_run() {
    // Nothing was ever open, so there is nothing to show. Graft leaves and
    // says why, on a terminal it has handed back.
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(1),
            Product::Repository(Err("/tmp is not inside a git repository".to_owned())),
        ),
    );

    assert_eq!(
        next.quit,
        Some(Quit::Fatal(
            "/tmp is not inside a git repository".to_owned()
        ))
    );
    assert!(commands.is_empty());
}

#[test]
fn a_failed_refresh_of_an_open_repository_only_reaches_the_status_line() {
    let open = AppState {
        repository: Some(facts("/code/api", "main")),
        ..AppState::default()
    };

    let (next, _) = update(
        &open,
        AppEvent::Produced(
            JobId::new(3),
            Product::Repository(Err("the repository went away".to_owned())),
        ),
    );

    assert_eq!(next.quit, None, "a refresh must not end the run");
    assert_eq!(
        next.status,
        Status::Failed("the repository went away".to_owned())
    );
    // The repository already read stays on screen: stale beats blank.
    assert_eq!(next.repository, Some(facts("/code/api", "main")));
}

#[test]
fn what_the_system_git_is_is_remembered() {
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(4),
            Product::SystemGit(GitCheck::Unavailable("no git on PATH".to_owned())),
        ),
    );

    assert_eq!(
        next.system_git,
        Some(GitCheck::Unavailable("no git on PATH".to_owned()))
    );
    assert!(commands.is_empty());
}

/// What a walk says first: the order it is in.
fn started(requested: Order, order: Order) -> AppEvent {
    AppEvent::Progress(
        JobId::new(0),
        Progress::History(HistoryUpdate::Started(Box::new(WalkFacts {
            requested,
            order,
            default_branch: None,
            integration: Vec::new().into(),
        }))),
    )
}

/// The same event, from job `id`.
fn from(id: JobId, event: AppEvent) -> AppEvent {
    match event {
        AppEvent::Progress(_, progress) => AppEvent::Progress(id, progress),
        other => other,
    }
}

/// Job `id`'s answer to the first request, with `n` rows laid out. The
/// rows themselves do not matter here, only whether they land, so they are
/// copies of one row.
fn rows(id: JobId, n: usize) -> AppEvent {
    let row = Row {
        commit: Commit {
            id: Oid::from_bytes(&[7; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
            parents: Vec::new(),
            author: identity(),
            committer: identity(),
            message: b"a commit".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    };
    AppEvent::Progress(
        id,
        Progress::History(HistoryUpdate::Window(Window {
            request: 1,
            first: 0,
            rows: vec![row; n.min(3)].into(),
            laid_out: n,
            done: false,
            promoted: false,
        })),
    )
}

/// The state of an application with a walk in flight.
fn awaiting(id: JobId) -> AppState {
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Started(id, Task::ReadHistory),
    );

    // A default state is zero rows tall, which shows one row: the first
    // screen, and one more screen below it.
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(1, Wanted::Rows(0..4)))],
        "a walk that has started is asked for its first screen"
    );
    assert_eq!(next.awaiting, Some(id));
    assert_eq!(next.history.as_ref().map(|history| history.asked), Some(1));
    next
}

/// Where the walk on screen is, and how many rows it has.
fn progress(state: &AppState) -> (Phase, usize) {
    let history = state.history.as_ref().expect("a walk was asked for");
    (history.phase.clone(), history.rows)
}

#[test]
fn rows_land_as_they_arrive_and_the_walk_completes() {
    let id = JobId::new(4);
    let state = awaiting(id);

    let (state, _) = update(&state, rows(id, 500));
    assert_eq!(progress(&state), (Phase::Streaming, 500));
    let (state, _) = update(&state, rows(id, 512));
    assert_eq!(progress(&state), (Phase::Streaming, 512));

    let (state, commands) = update(&state, core(Event::JobFinished(id)));
    assert_eq!(progress(&state), (Phase::Complete, 512));
    assert_eq!(state.awaiting, None);
    assert!(commands.is_empty());
}

fn request(id: u64, wanted: Wanted) -> RowRequest {
    RowRequest {
        id,
        wanted,
        view: View::default(),
        promote: None,
    }
}

fn sized(columns: u16, rows: u16) -> AppEvent {
    core(Event::Resize { columns, rows })
}

/// Moving past the rows held asks the walk for the rows there, and
/// only then: moving within them asks for nothing.
#[test]
fn moving_past_the_window_asks_for_the_rows_there() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), sized(80, 12));
    // Ten graph rows: the resize asked for them, and they arrive.
    let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));

    let (state, commands) = update(&state, key(KeyCode::Char('j')));
    assert!(commands.is_empty(), "row 1 is held: {commands:?}");
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));

    // Page 1 (10..20) is drawn from the rows held, but the second page
    // after it is not held: ask now, three screens either side, so
    // neither waits.
    let (_, commands) = update(&state, key(KeyCode::PageDown));
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(3, Wanted::Rows(0..50)))]
    );
}

/// `G` on a walk not laid out to its end asks for the last rows and leaves
/// the viewport where it is; `Esc` then withdraws the jump — not the walk.
#[test]
fn esc_withdraws_a_jump_before_it_cancels_the_walk() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), sized(80, 12));
    let (state, _) = update(&state, rows_for(id, 2, 0..30, 500));

    let (jumping, commands) = update(&state, key(KeyCode::Char('G')));
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(3, Wanted::Last(20)))]
    );
    let history = jumping.history.as_ref().expect("a walk");
    assert_eq!((history.top, history.cursor, history.jump), (0, 0, Some(3)));

    let (withdrawn, commands) = update(&jumping, key(KeyCode::Escape));
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(4, Wanted::Rows(0..40)))]
    );
    assert_eq!(withdrawn.awaiting, Some(id), "the walk goes on");
    assert_eq!(withdrawn.history.as_ref().and_then(|h| h.jump), None);

    let (after, _) = update(&withdrawn, rows_for(id, 3, 480..500, 500));
    assert_eq!(after, withdrawn, "the jump's answer does not land");
}

/// Job `id`'s answer to request `request`: `span` rows, `laid_out` so far.
fn rows_for(id: JobId, request: u64, span: Range<usize>, laid_out: usize) -> AppEvent {
    let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) =
        rows(id, span.len())
    else {
        unreachable!("`rows` answers with a window");
    };
    let one = window.rows[0].clone();
    AppEvent::Progress(
        id,
        Progress::History(HistoryUpdate::Window(Window {
            request,
            first: span.start,
            rows: vec![one; span.len()].into(),
            laid_out,
            done: false,
            promoted: false,
        })),
    )
}

#[test]
fn esc_cancels_the_walk_the_screen_is_waiting_for() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), rows(id, 500));

    let (next, commands) = update(&state, key(KeyCode::Escape));

    assert_eq!(commands, vec![Command::CancelJob(id)]);
    assert_eq!(
        next.awaiting, None,
        "nothing is waited on once `Esc` has been pressed"
    );
    assert_eq!(progress(&next), (Phase::Cancelled, 500));
}

#[test]
fn esc_with_no_walk_running_does_nothing() {
    let before = AppState::default();

    let (next, commands) = update(&before, key(KeyCode::Escape));

    assert_eq!(next, before);
    assert!(commands.is_empty());
}

/// The race `Esc` has to survive: the walk lays out another chunk between
/// the key press and the cancel flag being read, and sends it anyway. It
/// must not appear, and neither may the walk's ending undo the cancel.
#[test]
fn no_chunk_lands_after_esc() {
    let id = JobId::new(4);
    let (state, _) = update(&awaiting(id), rows(id, 500));
    let (cancelled, _) = update(&state, key(KeyCode::Escape));

    let (next, commands) = update(&cancelled, rows(id, 500));
    assert_eq!(progress(&next), (Phase::Cancelled, 500));
    assert!(commands.is_empty());

    for ending in [Event::JobFinished(id), Event::JobCancelled(id)] {
        let (after, _) = update(&next, core(ending));
        assert_eq!(progress(&after), (Phase::Cancelled, 500));
    }
}

/// Two walks in flight: the older one's rows are about a question nobody
/// is asking any more.
#[test]
fn a_superseded_walk_loses_to_the_newer_one() {
    let old = JobId::new(1);
    let new = JobId::new(2);
    let (state, _) = update(&awaiting(old), rows(old, 3));
    let (state, _) = update(&state, AppEvent::Started(new, Task::ReadHistory));
    assert_eq!(progress(&state), (Phase::Streaming, 0));

    let (state, _) = update(&state, rows(old, 500));
    assert_eq!(
        progress(&state),
        (Phase::Streaming, 0),
        "stale rows dropped"
    );

    let (state, _) = update(&state, rows(new, 2));
    assert_eq!(progress(&state), (Phase::Streaming, 2));
}

/// ADR-0008 in the state: a repository with no commit-graph file answers
/// in commit-date order, and the screen is told the order is not final.
#[test]
fn a_walk_that_fell_back_to_date_order_says_it_is_provisional() {
    let id = JobId::new(7);

    let (next, _) = update(
        &awaiting(id),
        from(id, started(Order::Topological, Order::CommitDate)),
    );
    let walk = next.history.and_then(|history| history.walk);
    assert!(walk.expect("the walk said what it is").is_provisional());

    let (next, _) = update(
        &awaiting(id),
        from(id, started(Order::Topological, Order::Topological)),
    );
    let walk = next.history.and_then(|history| history.walk);
    assert!(!walk.expect("the walk said what it is").is_provisional());
}

/// A provisional walk and the topological sort it asked for, started as
/// job `sort`.
fn sorting(walk: JobId, sort: JobId) -> AppState {
    let (next, commands) = update(
        &awaiting(walk),
        from(walk, started(Order::Topological, Order::CommitDate)),
    );
    assert!(
        commands.contains(&Command::PromoteHistory),
        "a provisional walk asks for its order: {commands:?}"
    );
    let (next, _) = update(&next, AppEvent::Started(sort, Task::PromoteHistory));
    assert_eq!(next.promoting, Some(sort));
    next
}

fn order() -> std::sync::Arc<[graft_core::commit_handle::CommitHandle]> {
    vec![graft_core::commit_handle::CommitHandle(7)].into()
}

/// A walk already in the order asked for has nothing to promote.
#[test]
fn a_walk_in_its_final_order_asks_for_no_sort() {
    let id = JobId::new(7);
    let (_, commands) = update(
        &awaiting(id),
        from(id, started(Order::Topological, Order::Topological)),
    );
    assert!(!commands.contains(&Command::PromoteHistory));
}

/// The order from a sort that has been replaced by a newer one never
/// reaches the walk; the newest sort's does.
#[test]
fn a_superseded_sort_is_never_applied() {
    let (walk, old, new) = (JobId::new(1), JobId::new(2), JobId::new(3));
    let state = sorting(walk, old);
    let (state, commands) = update(&state, AppEvent::Started(new, Task::PromoteHistory));
    assert_eq!(commands, vec![Command::CancelJob(old)]);

    let (state, commands) = update(
        &state,
        AppEvent::Produced(old, Product::TopologicalOrder(order())),
    );
    assert!(commands.is_empty(), "{commands:?}");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.promotion, Some(Promotion::Sorting));

    let (state, commands) = update(
        &state,
        AppEvent::Produced(new, Product::TopologicalOrder(order())),
    );
    let [Command::ShowRows(to, request)] = commands.as_slice() else {
        panic!("the newest order goes to the walk: {commands:?}");
    };
    assert_eq!(*to, walk);
    assert!(request.promote.is_some());
    assert_eq!(state.promoting, None);
}

/// `Esc` while sorting stops the sort and keeps the rows where they are,
/// and the header says the order stays provisional.
#[test]
fn esc_while_sorting_keeps_the_provisional_order_and_says_so() {
    let (walk, sort) = (JobId::new(1), JobId::new(2));
    let state = sorting(walk, sort);
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(
        crate::view::order_notice(history),
        Some(OrderNotice::Sorting)
    );

    let (state, commands) = update(&state, key(KeyCode::Escape));
    assert_eq!(commands, vec![Command::CancelJob(sort)]);
    assert_eq!(state.awaiting, Some(walk), "the walk itself goes on");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.promotion, Some(Promotion::Kept));
    assert_eq!(
        crate::view::order_notice(history),
        Some(OrderNotice::Provisional)
    );

    // The sort's late answer changes nothing.
    let (state, commands) = update(
        &state,
        AppEvent::Produced(sort, Product::TopologicalOrder(order())),
    );
    assert!(commands.is_empty(), "{commands:?}");
    let (_, commands) = update(&state, core(Event::JobCancelled(sort)));
    assert!(commands.is_empty(), "{commands:?}");
}

/// A sort that fails leaves the order provisional, not an error.
#[test]
fn a_failed_sort_leaves_the_order_provisional() {
    let (walk, sort) = (JobId::new(1), JobId::new(2));
    let (state, _) = update(
        &sorting(walk, sort),
        core(Event::JobFailed(sort, JobFailure::Failed("oom".to_owned()))),
    );
    assert_eq!(state.promoting, None);
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.phase, Phase::Streaming);
    assert_eq!(history.promotion, Some(Promotion::Kept));
}

/// A walk that failed or was cancelled by something other than `Esc` —
/// shutdown, say — must not leave the screen waiting for ever, and says
/// which it was.
#[test]
fn a_walk_that_ends_without_finishing_says_how() {
    let id = JobId::new(3);

    let (cancelled, _) = update(&awaiting(id), core(Event::JobCancelled(id)));
    assert_eq!(cancelled.awaiting, None);
    assert_eq!(progress(&cancelled), (Phase::Cancelled, 0));

    let (failed, _) = update(
        &awaiting(id),
        core(Event::JobFailed(
            id,
            JobFailure::Failed("the pack is corrupt".to_owned()),
        )),
    );
    assert_eq!(failed.awaiting, None);
    let (phase, _) = progress(&failed);
    assert!(
        matches!(&phase, Phase::Failed(reason) if reason.contains("the pack is corrupt")),
        "{phase:?}"
    );
}

/// Purity's other half, which no runtime test can observe: that `update`
/// cannot reach the world at all.
///
/// This module is small and self-contained precisely so this check is
/// possible — it reads its own source (every file under `update/` but
/// this one) and fails if anything that talks to the outside (a file, a
/// clock, a thread, a terminal) has been imported or called. A reviewer
/// would have to notice; the test cannot help noticing.
#[test]
fn update_cannot_reach_the_world() {
    // Scan the directory rather than a hand-kept file list, so a new
    // `update/*.rs` file is included automatically instead of silently
    // skipped.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/update");
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(dir).expect("update/ exists") {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("utf-8 file name")
            .to_owned();
        if name == "tests.rs" {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        // `mod.rs` names its own test module at the very end; strip that
        // file's tail, the same way a single-file module used to strip its
        // `#[cfg(test)]`. Every other file is pure end to end and is kept
        // whole — stripping used to happen only after joining every file,
        // which threw away everything past `mod.rs`'s own marker.
        let contents = if name == "mod.rs" {
            contents
                .split_once("#[cfg(test)]")
                .expect("mod.rs declares the test module")
                .0
                .to_owned()
        } else {
            contents
        };
        sources.push(contents);
    }
    assert!(
        sources.len() >= 7,
        "expected every non-test file under update/, found {}: \
         the directory scan is broken",
        sources.len()
    );
    let code = sources.join("\n");

    for forbidden in [
        "std::io",
        "std::fs",
        "std::time",
        "std::thread",
        "std::process",
        "std::net",
        "std::env",
        "SystemTime",
        "Instant",
        "print!",
        "println!",
        "eprint!",
        "eprintln!",
        "crossterm",
        "ratatui",
    ] {
        assert!(
            !code.contains(forbidden),
            "`update` must stay pure, but its module mentions `{forbidden}`"
        );
    }
}

// Search (m1-the-graph/26).

use crate::search::Query;

fn hit(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn found(id: JobId, hits: &[u8]) -> AppEvent {
    AppEvent::Progress(
        id,
        Progress::Search(hits.iter().map(|byte| hit(*byte)).collect()),
    )
}

fn typed(state: &AppState, text: &str) -> (AppState, Vec<Command>) {
    let mut state = state.clone();
    let mut commands = Vec::new();
    for code in std::iter::once(KeyCode::Char('/'))
        .chain(text.chars().map(KeyCode::Char))
        .chain(std::iter::once(KeyCode::Enter))
    {
        let (next, more) = update(&state, key(code));
        state = next;
        commands.extend(more);
    }
    (state, commands)
}

/// A walk `walk` with rows 0..30 on hand, commit `i + 1` on row `i`, of 500
/// laid out; ten graph rows.
fn on_hand(walk: JobId) -> AppState {
    let (state, _) = update(&awaiting(walk), sized(80, 12));
    let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(mut window))) =
        rows_for(walk, 2, 0..30, 500)
    else {
        unreachable!("`rows_for` answers with a window");
    };
    window.rows = (0..30u8)
        .map(|at| {
            let mut row = window.rows[0].clone();
            row.commit.id = hit(at + 1);
            row
        })
        .collect();
    let (state, _) = update(
        &state,
        AppEvent::Progress(walk, Progress::History(HistoryUpdate::Window(window))),
    );
    state
}

fn search_of(state: &AppState) -> &Search {
    state.search.as_ref().expect("a search")
}

#[test]
fn slash_opens_a_prompt_that_takes_every_letter_and_enter_searches() {
    let state = on_hand(JobId::new(4));
    let (state, _) = update(&state, key(KeyCode::Char('/')));
    let (state, _) = update(&state, key(KeyCode::Char('q')));
    assert_eq!(state.quit, None, "q is text while the prompt is open");
    let (state, _) = update(&state, key(KeyCode::Char('x')));
    let (state, _) = update(&state, key(KeyCode::Backspace));
    assert_eq!(state.search_prompt.as_deref(), Some("q"));

    let (state, commands) = update(&state, key(KeyCode::Enter));
    assert_eq!(state.search_prompt, None);
    assert_eq!(
        commands,
        vec![Command::Search(Query::parse("q").expect("q"))]
    );
    assert_eq!(search_of(&state).state, SearchState::Running { hits: 0 });
}

#[test]
fn an_empty_query_or_esc_closes_the_prompt_without_searching() {
    let state = on_hand(JobId::new(4));
    let (closed, commands) = typed(&state, "  ");
    assert_eq!(
        (closed.search_prompt.clone(), closed.search.clone()),
        (None, None)
    );
    assert!(commands.is_empty());
    let (open, _) = update(&state, key(KeyCode::Char('/')));
    let (closed, commands) = update(&open, key(KeyCode::Escape));
    assert_eq!((closed.search_prompt, commands), (None, vec![]));
}

/// Hits count as partial while the job runs, the first is jumped to at
/// once, and the count settles when the job finishes.
#[test]
fn the_count_is_partial_while_running_and_settles_when_done() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, _) = update(&state, found(job, &[3]));
    assert_eq!(search_of(&state).state.describe(), "1 match so far");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.cursor, 2, "the first hit is selected at once");
    assert!(search_of(&state).line().contains("(1/1)"));

    let (state, _) = update(&state, found(job, &[12, 20]));
    assert_eq!(search_of(&state).state.describe(), "3 matches so far");
    let (state, _) = update(&state, core(Event::JobFinished(job)));
    assert_eq!(search_of(&state).state, SearchState::Done { hits: 3 });
    assert_eq!(search_of(&state).state.describe(), "3 matches");
}

#[test]
fn n_and_capital_n_walk_the_hits_and_wrap() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, _) = update(&state, found(job, &[3, 12, 20]));
    let cursor = |state: &AppState| state.history.as_ref().map(|h| h.cursor);
    assert_eq!(cursor(&state), Some(2));
    let (state, _) = update(&state, key(KeyCode::Char('n')));
    assert_eq!(cursor(&state), Some(11));
    let (state, _) = update(&state, key(KeyCode::Char('n')));
    let (state, _) = update(&state, key(KeyCode::Char('n')));
    assert_eq!(cursor(&state), Some(2), "n wraps past the last hit");
    let (state, _) = update(&state, key(KeyCode::Char('N')));
    assert_eq!(cursor(&state), Some(19), "N wraps before the first");
}

/// A hit not in the rows on hand is a find request to the walk; the
/// answer lands the cursor on it.
#[test]
fn a_far_hit_asks_the_walk_to_lay_out_to_it() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, commands) = update(&state, found(job, &[200]));
    let [Command::ShowRows(to, asked)] = commands.as_slice() else {
        panic!("one request: {commands:?}");
    };
    assert_eq!(*to, walk);
    assert!(matches!(asked.wanted, Wanted::Find { commit, .. } if commit == hit(200)));
    assert_eq!(search_of(&state).seeking, Some(hit(200)));
}

/// Results from a superseded search job are dropped: its start is
/// cancelled, and its hits never count.
#[test]
fn a_superseded_search_jobs_hits_are_discarded() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    // Typed again before the first search's job said it started.
    let (state, _) = typed(&state, "bug");
    let (old, new) = (JobId::new(9), JobId::new(10));
    let (state, commands) = update(&state, AppEvent::Started(old, Task::Search));
    assert_eq!(commands, vec![Command::CancelJob(old)]);
    let (state, _) = update(&state, AppEvent::Started(new, Task::Search));
    let (state, _) = update(&state, found(old, &[3]));
    assert_eq!(
        search_of(&state).hits(),
        0,
        "the old job's hits are dropped"
    );
    let (state, _) = update(&state, core(Event::JobFinished(old)));
    assert_eq!(search_of(&state).state, SearchState::Running { hits: 0 });
    assert_eq!(search_of(&state).job, Some(new));
    assert_eq!(search_of(&state).query.text(), "bug");
}

#[test]
fn esc_cancels_a_running_search_and_restores_the_selection() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));
    let (state, _) = update(&state, found(job, &[6]));
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(5));

    let (state, commands) = update(&state, key(KeyCode::Escape));
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
    assert_eq!(
        state.history.as_ref().map(|h| (h.cursor, h.top)),
        Some((0, 0))
    );
    assert_eq!(search_of(&state).state, SearchState::Cancelled { hits: 1 });
    assert_eq!(state.awaiting, Some(walk), "the walk goes on");

    let (state, _) = update(&state, core(Event::JobCancelled(job)));
    assert_eq!(search_of(&state).state, SearchState::Cancelled { hits: 1 });
    let (state, _) = update(&state, key(KeyCode::Escape));
    assert_eq!(state.search, None, "a second Esc closes the line");
}

/// No matches, cancelled and failed are three states that read
/// differently, and a failure stays on the search's line.
#[test]
fn no_matches_cancelled_and_failed_are_distinct() {
    let walk = JobId::new(4);
    let (state, _) = typed(&on_hand(walk), "fix");
    let job = JobId::new(9);
    let (state, _) = update(&state, AppEvent::Started(job, Task::Search));

    let (none, _) = update(&state, core(Event::JobFinished(job)));
    let (cancelled, _) = update(&state, core(Event::JobCancelled(job)));
    let failure = JobFailure::Failed("walk broke".to_owned());
    let (failed, _) = update(&state, core(Event::JobFailed(job, failure)));

    let lines = [none, cancelled, failed].map(|state| search_of(&state).state.describe());
    assert_eq!(lines[0], "no matches");
    assert_eq!(lines[1], "search cancelled, 0 matches found");
    assert_eq!(lines[2], "search failed: walk broke");
}

#[test]
fn a_handed_back_key_ends_the_run_naming_the_key() {
    let state = AppState {
        harness: Harness {
            hand_back: vec![KeyCode::Char(']')],
            ..Harness::default()
        },
        ..AppState::default()
    };
    let (next, commands) = update(&state, key(KeyCode::Char(']')));
    assert_eq!(next.quit, Some(Quit::HandedBack(KeyCode::Char(']'))));
    assert!(commands.is_empty());
    assert!(next.should_quit());

    let (next, _) = update(&state, key(KeyCode::Char('j')));
    assert_eq!(next.quit, None, "other keys are the graph's");
}

#[test]
fn an_open_search_prompt_keeps_a_handed_back_key_as_text() {
    let state = AppState {
        harness: Harness {
            hand_back: vec![KeyCode::Char(']')],
            ..Harness::default()
        },
        search_prompt: Some(String::from("fox")),
        ..AppState::default()
    };
    let (next, _) = update(&state, key(KeyCode::Char(']')));
    assert_eq!(next.quit, None);
    assert_eq!(next.search_prompt.as_deref(), Some("fox]"));
}

mod commit_graph_offer {
    use super::*;
    use crate::repository::CommitGraphFact;
    use graft_git::commit_graph::Written;
    use graft_git::version::GitVersion;
    use std::time::Duration;

    fn opened(fact: CommitGraphFact, git: GitCheck, harness: Harness) -> AppState {
        let state = AppState {
            harness,
            ..AppState::default()
        };
        let mut repository = facts("/code/api", "main");
        repository.commit_graph = fact;
        let (state, _) = update(&state, AppEvent::Produced(job(1), Product::SystemGit(git)));
        let (state, _) = update(
            &state,
            AppEvent::Produced(job(2), Product::Repository(Ok(repository))),
        );
        state
    }

    fn job(n: u64) -> JobId {
        JobId::new(n)
    }

    fn supported() -> GitCheck {
        GitCheck::Ran(SystemGit::Supported(GitVersion::MINIMUM))
    }

    fn offered() -> AppState {
        opened(
            CommitGraphFact::Absent { declined: false },
            supported(),
            Harness::default(),
        )
    }

    #[test]
    fn a_missing_file_with_a_usable_git_is_offered() {
        let state = offered();
        assert_eq!(state.commit_graph, CommitGraphOffer::Offered);
        assert_eq!(state.commit_graph_line(), Some(CommitGraphLine::Offered));
    }

    #[test]
    fn b_writes_and_the_written_line_lasts_one_key() {
        let (writing, commands) = update(&offered(), key(KeyCode::Char('B')));
        assert_eq!(writing.commit_graph, CommitGraphOffer::Writing(None));
        assert_eq!(commands, vec![Command::WriteCommitGraph]);
        let (again, commands) = update(&writing, key(KeyCode::Char('B')));
        assert!(commands.is_empty(), "a second B starts nothing");

        let (writing, _) = update(&again, AppEvent::Started(job(7), Task::WriteCommitGraph));
        assert_eq!(
            writing.commit_graph,
            CommitGraphOffer::Writing(Some(job(7)))
        );
        let written = Written {
            commits: 200,
            elapsed: Duration::from_millis(5),
        };
        let (done, _) = update(
            &writing,
            AppEvent::Produced(job(7), Product::CommitGraph(Ok(written))),
        );
        assert_eq!(
            done.commit_graph,
            CommitGraphOffer::Written { commits: 200 }
        );
        assert_eq!(done.status, Status::Ready);

        let (next, _) = update(&done, key(KeyCode::Char('j')));
        assert_eq!(next.commit_graph, CommitGraphOffer::NotNeeded);
        assert_eq!(next.commit_graph_line(), None);
    }

    #[test]
    fn a_failed_write_puts_git_s_words_in_the_problem_line() {
        let (writing, _) = update(&offered(), key(KeyCode::Char('B')));
        let (writing, _) = update(&writing, AppEvent::Started(job(7), Task::WriteCommitGraph));
        let why = "commit-graph write failed (exit 128): fatal: Unable to create lock";
        let (failed, _) = update(
            &writing,
            AppEvent::Produced(job(7), Product::CommitGraph(Err(why.to_owned()))),
        );
        assert_eq!(failed.commit_graph, CommitGraphOffer::Failed);
        assert_eq!(failed.commit_graph_line(), None);
        let Status::Failed(problem) = &failed.status else {
            panic!("{:?}", failed.status);
        };
        assert!(problem.contains("exit 128") && problem.contains("fatal: Unable"));
        let (still, commands) = update(&failed, key(KeyCode::Char('B')));
        assert!(commands.is_empty(), "no retry this run");
        assert_eq!(still.commit_graph, CommitGraphOffer::Failed);
    }

    #[test]
    fn x_declines_and_a_failed_decline_offers_again() {
        let (declining, commands) = update(&offered(), key(KeyCode::Char('X')));
        assert_eq!(declining.commit_graph, CommitGraphOffer::Declining(None));
        assert_eq!(commands, vec![Command::DeclineCommitGraph]);
        let (declining, _) = update(
            &declining,
            AppEvent::Started(job(8), Task::DeclineCommitGraph),
        );

        let (declined, _) = update(
            &declining,
            AppEvent::Produced(job(8), Product::CommitGraphDeclined(Ok(()))),
        );
        assert_eq!(declined.commit_graph, CommitGraphOffer::Declined);
        assert_eq!(declined.commit_graph_line(), None);

        let (refused, _) = update(
            &declining,
            AppEvent::Produced(
                job(8),
                Product::CommitGraphDeclined(Err("permission denied".to_owned())),
            ),
        );
        assert_eq!(refused.commit_graph, CommitGraphOffer::Offered);
        assert_eq!(
            refused.status,
            Status::Failed("could not remember the choice: permission denied".to_owned())
        );
    }

    #[test]
    fn nothing_is_offered_with_a_file_a_decline_or_a_harness_that_says_no() {
        let present = opened(
            CommitGraphFact::Present { commits: 5 },
            supported(),
            Harness::default(),
        );
        assert_eq!(present.commit_graph, CommitGraphOffer::NotNeeded);
        let declined = opened(
            CommitGraphFact::Absent { declined: true },
            supported(),
            Harness::default(),
        );
        assert_eq!(declined.commit_graph, CommitGraphOffer::NotNeeded);
        let suppressed = opened(
            CommitGraphFact::Absent { declined: false },
            supported(),
            Harness {
                offer_commit_graph: false,
                ..Harness::default()
            },
        );
        assert_eq!(suppressed.commit_graph, CommitGraphOffer::Suppressed);
        assert_eq!(suppressed.commit_graph_line(), None);
    }

    #[test]
    fn without_a_usable_git_the_line_says_why_and_offers_nothing() {
        for git in [
            GitCheck::Unavailable("no git on PATH".to_owned()),
            GitCheck::Ran(SystemGit::for_version(GitVersion {
                major: 2,
                minor: 30,
                patch: 0,
            })),
        ] {
            let state = opened(
                CommitGraphFact::Absent { declined: false },
                git,
                Harness::default(),
            );
            assert_eq!(
                state.commit_graph_line(),
                Some(CommitGraphLine::NoGitToBuild)
            );
            for letter in ['B', 'X'] {
                let (_, commands) = update(&state, key(KeyCode::Char(letter)));
                assert!(commands.is_empty(), "{letter} with no git: {commands:?}");
            }
        }
    }

    #[test]
    fn an_unreadable_file_is_a_problem_not_an_offer() {
        let state = opened(
            CommitGraphFact::Unreadable("commit-graph unreadable: bad signature".to_owned()),
            supported(),
            Harness::default(),
        );
        assert_eq!(state.commit_graph, CommitGraphOffer::NotNeeded);
        assert_eq!(
            state.status,
            Status::Failed("commit-graph unreadable: bad signature".to_owned())
        );
    }

    #[test]
    fn b_and_x_do_nothing_outside_the_offer() {
        let states = [
            CommitGraphOffer::NotNeeded,
            CommitGraphOffer::Writing(Some(job(3))),
            CommitGraphOffer::Written { commits: 1 },
            CommitGraphOffer::Declining(Some(job(3))),
            CommitGraphOffer::Declined,
            CommitGraphOffer::Failed,
            CommitGraphOffer::Suppressed,
        ];
        for offer in states {
            let state = AppState {
                commit_graph: offer,
                ..offered()
            };
            for letter in ['B', 'X'] {
                let (_, commands) = update(&state, key(KeyCode::Char(letter)));
                assert!(commands.is_empty(), "{letter} in {offer:?}: {commands:?}");
            }
        }
    }

    #[test]
    fn the_line_costs_a_graph_row_while_shown() {
        let mut state = offered();
        state.size = Size {
            columns: 80,
            rows: 24,
        };
        let with = state.graph_rows();
        state.commit_graph = CommitGraphOffer::Declined;
        assert_eq!(state.graph_rows(), with + 1);
    }
}

// --- The detail pane (m2-details-and-diffs/06, ADR-0023) ---

use std::sync::Arc as DetailArc;

use graft_git::diff::{DiffOptions, FileEntry, FileList, FileStatus};
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::{DetailFocus, DiffView, FileListView, FileRow, FileRowKind, FileStatusView};

use super::detail::{DetailSpec, DetailState, LOCAL_JOB_BIT, Loading, LocalJobs};

fn list_commands(commands: &[Command]) -> Vec<(JobId, graft_git::diff::DiffSpec)> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::ListFiles(id, spec, _) => Some((*id, *spec)),
            _ => None,
        })
        .collect()
}

/// Two files, flat, as a files job would hand them back.
fn two_files() -> (DetailArc<FileListView>, DetailArc<FileList>) {
    let entry = |path: &str| FileEntry {
        path: path.into(),
        old_path: None,
        status: FileStatus::Modified,
        added: Some(1),
        removed: Some(1),
    };
    let row = |name: &str, index| FileRow {
        depth: 0,
        name: sanitize_line(name),
        kind: FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        },
        entry: Some(index),
    };
    let view = FileListView {
        rows: vec![row("a.rs", 0), row("b.rs", 1)],
        files: 2,
        added: 2,
        removed: 2,
        ..FileListView::default()
    };
    let list = FileList {
        files: vec![entry("a.rs"), entry("b.rs")],
        added: 2,
        removed: 2,
    };
    (DetailArc::new(view), DetailArc::new(list))
}

fn diff_of(path: &str) -> DetailArc<DiffView> {
    DetailArc::new(DiffView {
        path: sanitize_line(path),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        untracked: false,
    })
}

#[test]
fn enter_opens_the_pane_on_the_selected_commit_and_asks_for_its_files() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let selected = state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| row.commit.id)
        .expect("row 0 has arrived");
    let (opened, commands) = update(&state, key(KeyCode::Enter));
    assert!(opened.detail.shown);
    assert_eq!(opened.detail.focus, DetailFocus::Files);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert_eq!(
        lists[0].1,
        graft_git::diff::DiffSpec::Commit {
            commit: selected,
            against: None
        }
    );
    assert_ne!(
        lists[0].0.raw() & LOCAL_JOB_BIT,
        0,
        "update names the job itself"
    );
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::ContainedIn(_, oid) if *oid == selected))
    );

    let (again, commands) = update(&opened, key(KeyCode::Escape));
    assert!(!again.detail.shown);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn a_second_enter_on_the_same_row_asks_nothing() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (opened, _) = update(&state, key(KeyCode::Enter));
    let (mut graph, _) = update(&opened, key(KeyCode::Tab));
    graph.detail.focus = DetailFocus::Graph;
    let (again, commands) = update(&graph, key(KeyCode::Enter));
    assert!(again.detail.shown);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn moving_the_graph_selection_while_shown_cancels_and_reloads() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (mut opened, first) = update(&state, key(KeyCode::Enter));
    let old = list_commands(&first)[0].0;
    opened.detail.focus = DetailFocus::Graph;
    let (moved, commands) = update(&opened, key(KeyCode::Char('j')));
    assert!(commands.contains(&Command::CancelJob(old)), "{commands:?}");
    assert_eq!(list_commands(&commands).len(), 1, "{commands:?}");
    assert_ne!(moved.detail.files, opened.detail.files);

    let (mut hidden, _) = update(&state, key(KeyCode::Char('j')));
    hidden.detail = DetailState::default();
    let (_, commands) = update(&hidden, key(KeyCode::Char('k')));
    assert!(
        list_commands(&commands).is_empty(),
        "a hidden pane costs nothing"
    );
}

#[test]
fn a_stale_file_list_changes_nothing() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (opened, _) = update(&state, key(KeyCode::Enter));
    let old_answer = AppEvent::Produced(
        JobId::new(LOCAL_JOB_BIT | 0x3e7),
        Product::Files(Ok(two_files())),
    );
    let (next, commands) = update(&opened, old_answer);
    assert_eq!(next.detail, opened.detail);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn files_arriving_ask_for_the_first_diff_and_revisits_hit_the_cache() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (opened, first) = update(&state, key(KeyCode::Enter));
    let id = list_commands(&first)[0].0;
    let (listed, commands) = update(
        &opened,
        AppEvent::Produced(id, Product::Files(Ok(two_files()))),
    );
    let Loading::Ready(files) = &listed.detail.files else {
        panic!("files ready")
    };
    assert_eq!(files.cursor, 0);
    let reads: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            Command::ReadFileDiff(id, key, _) => Some((*id, *key)),
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 1, "{commands:?}");
    let (first_id, first_key) = reads[0];
    assert_eq!(first_key.entry, 0);
    assert_eq!(first_key.options, DiffOptions::default());
    let (shown, _) = update(
        &listed,
        AppEvent::Produced(first_id, Product::FileDiff(first_key, Ok(diff_of("a.rs")))),
    );
    assert!(matches!(shown.detail.diff, Loading::Ready(_)));

    let (down, commands) = update(&shown, key(KeyCode::Char('j')));
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::ReadFileDiff(_, key, _) if key.entry == 1)),
        "{commands:?}"
    );
    let (back, commands) = update(&down, key(KeyCode::Char('k')));
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ReadFileDiff(..))),
        "{commands:?}"
    );
    assert!(
        matches!(back.detail.diff, Loading::Ready(_)),
        "cache hit shows at once"
    );

    let (diff_focus, _) = update(&back, key(KeyCode::Enter));
    assert_eq!(diff_focus.detail.focus, DetailFocus::Diff);
    assert!(
        matches!(back.detail.spec, Some(DetailSpec::Commit(_))),
        "spec kept"
    );
}

/// [`merge_on_screen`] plus a dirty tree and `HEAD` at row 0 — the
/// uncommitted-changes row is shown, cursor at the top, nothing selected on
/// it yet.
fn dirty_on_screen(id: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(id, 0);
    let head_id = state
        .history
        .as_ref()
        .and_then(History::selected)
        .map(|row| row.commit.id)
        .expect("row 0 has arrived");
    state.working_tree = Some(graft_git::status::WorkingTreeCounts {
        staged: 1,
        unstaged: 1,
        untracked: 1,
    });
    let badges = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(head_id),
        &std::collections::HashMap::new(),
        |_, _| unreachable!("no branches on this fixture to count"),
    )
    .expect("no refs to fail reading");
    state.refs = Some(Arc::new(badges));
    state
}

#[test]
fn k_from_the_top_row_selects_the_working_tree_row_and_asks_history_for_nothing() {
    let state = dirty_on_screen(JobId::new(40));
    let (next, commands) = update(&state, key(KeyCode::Char('k')));
    assert!(next.on_working_tree);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn enter_on_the_working_tree_row_opens_the_pane_on_it() {
    let mut state = dirty_on_screen(JobId::new(41));
    state.on_working_tree = true;
    let (opened, commands) = update(&state, key(KeyCode::Enter));
    assert!(opened.detail.shown);
    assert_eq!(opened.detail.focus, DetailFocus::Files);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert_eq!(lists[0].1, graft_git::diff::DiffSpec::WorkingTreeVsHead);
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ContainedIn(..))),
        "the working tree is not a commit to ask branch containment about: {commands:?}"
    );
}

#[test]
fn j_off_the_row_with_the_pane_shown_follows_to_heads_row() {
    let mut state = dirty_on_screen(JobId::new(42));
    state.on_working_tree = true;
    let (mut opened, _) = update(&state, key(KeyCode::Enter));
    opened.detail.focus = DetailFocus::Graph;
    let (moved, commands) = update(&opened, key(KeyCode::Char('j')));
    assert!(!moved.on_working_tree);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert_eq!(
        lists[0].1,
        graft_git::diff::DiffSpec::Commit {
            commit: state
                .history
                .as_ref()
                .and_then(History::selected)
                .map(|row| row.commit.id)
                .expect("row 0"),
            against: None
        }
    );
}

#[test]
fn a_clean_tree_while_shown_clears_the_flag_and_follows_off_it() {
    let mut state = dirty_on_screen(JobId::new(43));
    state.on_working_tree = true;
    let (opened, _) = update(&state, key(KeyCode::Enter));
    let (clean, commands) = update(
        &opened,
        AppEvent::Produced(
            JobId::new(1),
            Product::WorkingTree(Ok(graft_git::status::WorkingTreeCounts {
                staged: 0,
                unstaged: 0,
                untracked: 0,
            })),
        ),
    );
    assert!(!clean.on_working_tree);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert!(matches!(
        lists[0].1,
        graft_git::diff::DiffSpec::Commit { .. }
    ));
}

#[test]
fn y_on_the_working_tree_row_is_a_no_op() {
    let mut state = dirty_on_screen(JobId::new(44));
    state.on_working_tree = true;
    let (next, commands) = update(&state, key(KeyCode::Char('y')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.status, state.status);
}

#[test]
fn a_click_on_the_first_body_row_selects_the_working_tree_row() {
    let mut state = dirty_on_screen(JobId::new(45));
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);
    assert!(next.on_working_tree);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));
}

#[test]
fn a_click_one_row_lower_selects_the_top_history_row_and_clears_the_flag() {
    let mut state = dirty_on_screen(JobId::new(46));
    state.on_working_tree = true;
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);
    assert!(!next.on_working_tree);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));
}

// --- Divider drag and wheel routing (m2-details-and-diffs/10b, ADR-0028) ---

#[test]
fn a_click_on_the_divider_starts_a_drag_and_up_ends_it() {
    let (mut state, _) = merge_on_screen(JobId::new(50), 0);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = Size {
        columns: 200,
        rows: 50,
    };
    // The current selection is already "followed": no `follow_selection`
    // fetch fires behind the click and confuses the command assertions
    // below.
    state.detail.spec = state.selected_spec();
    // Right layout, 42% of 200 = 84 columns; the divider sits at column
    // 200 - 84 = 116, the pane's first column.
    let down = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 116,
        row: 10,
        modifiers: Modifiers::NONE,
    }));
    let (dragging, commands) = update(&state, down);
    assert!(dragging.detail.dragging, "the divider column starts a drag");
    assert!(
        commands.is_empty(),
        "no resize yet, just the grab: {commands:?}"
    );
    assert_eq!(
        dragging.history.as_ref().map(|history| history.cursor),
        state.history.as_ref().map(|history| history.cursor),
        "the divider swallows the click; nothing selected under it"
    );

    let drag = core(Event::Mouse(MouseInput {
        kind: MouseKind::Drag(MouseButton::Left),
        column: 100,
        row: 20,
        modifiers: Modifiers::NONE,
    }));
    let (dragged, commands) = update(&dragging, drag.clone());
    assert_eq!(dragged.detail.pane_size.right_columns, Some(100));
    assert_eq!(
        commands,
        vec![Command::SaveLayout(dragged.detail.pane_size)]
    );

    let up = core(Event::Mouse(MouseInput {
        kind: MouseKind::Up(MouseButton::Left),
        column: 100,
        row: 20,
        modifiers: Modifiers::NONE,
    }));
    let (released, _) = update(&dragged, up);
    assert!(!released.detail.dragging);

    // With the drag over, further motion at the same column no longer
    // resizes: the guard is `next.detail.dragging`, not the column.
    let (stayed, commands) = update(&released, drag);
    assert_eq!(stayed.detail.pane_size, released.detail.pane_size);
    assert!(commands.is_empty());
}

#[test]
fn wheel_over_the_files_region_moves_its_cursor_not_the_graph() {
    let (mut state, _) = merge_on_screen(JobId::new(51), 0);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = Size {
        columns: 200,
        rows: 50,
    };
    let (files, list) = two_files();
    state.detail.files = Loading::Ready(files);
    state.detail.list = Some(list);
    state.detail.spec = state.selected_spec();
    let before_top = state.history.as_ref().map(|history| history.top);

    // Right layout: pane starts at column 116, header is 5 rows, so row 6
    // is the files region's first row.
    let wheel = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 150,
        row: 6,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, wheel);
    assert_eq!(next.detail.files, {
        let Loading::Ready(files) = &state.detail.files else {
            unreachable!()
        };
        let mut files = (**files).clone();
        files.move_cursor(WHEEL_LINES);
        Loading::Ready(std::sync::Arc::new(files))
    });
    assert_eq!(
        next.history.as_ref().map(|history| history.top),
        before_top,
        "the graph viewport does not move"
    );
}

#[test]
fn wheel_over_the_pane_never_scrolls_the_graph_underneath_it() {
    let (mut state, _) = merge_on_screen(JobId::new(52), 0);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = Size {
        columns: 200,
        rows: 50,
    };
    let before_top = state.history.as_ref().map(|history| history.top);

    // Body region: past the (empty) files list, still inside the pane.
    let wheel = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 150,
        row: 10,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, wheel);
    assert_eq!(
        next.history.as_ref().map(|history| history.top),
        before_top,
        "wheel over the diff body must not fall through to the graph scroll"
    );
}
