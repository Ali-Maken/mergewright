use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::{JobFailure, JobId};

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_git::history::Order;
use graft_git::version::SystemGit;
use graft_graph::{Follow, Row};

use super::geometry::HEADER_ROWS;
use super::mouse::WHEEL_LINES;
use super::*;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use crate::history::{
    History, HistoryUpdate, Phase, Promotion, RowRequest, View, WalkFacts, Wanted, Window,
};
use crate::repository::{Head, RepositoryFacts};
use crate::search::{Search, SearchState};
use graft_ui::graph_screen::{CommitGraphLine, Divider, HintContext, OrderNotice};

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

/// The pool announcing a walk the screen asked for.
///
/// `update` names the walk when it builds `Command::ReadHistory`, so a
/// `Started` is only ever taken under the name the screen is already holding
/// — feeding one out of nowhere is a walk nobody asked for
/// (m5-history-and-ghost-preview/05).
fn walk_started(state: &AppState, id: JobId) -> (AppState, Vec<Command>) {
    let asked = AppState {
        awaiting: Some(id),
        ..state.clone()
    };
    update(&asked, AppEvent::Started(id, Task::ReadHistory))
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
    let (started, _) = walk_started(&state, JobId::new(1));
    let history = started.history.expect("a walk started");
    assert_eq!(
        history.strip_rows,
        Some(graft_graph::raster::cache::STRIP_ROWS)
    );

    let (unicode, _) = walk_started(&AppState::default(), JobId::new(1));
    assert_eq!(unicode.history.expect("a walk started").strip_rows, None);
}

#[test]
fn f3_cycles_pixels_unicode_ascii_and_back_to_whole_strips() {
    let state = AppState {
        settled_tier: Some(RenderingTier::Pixel),
        ..AppState::default()
    };
    let (started, _) = walk_started(&state, JobId::new(1));
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
    // The evil-merge probe rides along on any event while the cursor rests
    // on a merge (M2 review M3), so the row request is picked out rather
    // than expected to be the only command.
    let mut rows = commands.iter().filter_map(|command| match command {
        Command::ShowRows(_, request) => Some(request.view.clone()),
        _ => None,
    });
    let view = rows
        .next()
        .unwrap_or_else(|| panic!("expected one row request, got {commands:?}"));
    assert!(
        rows.next().is_none(),
        "expected one row request, got {commands:?}"
    );
    view
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

/// m2-details-and-diffs/21 (m1-the-graph/54): a walk restart — `Ctrl+R`'s
/// among them — puts the cursor back on the commit it was on, the same
/// `reanchor` idiom `change_view` already uses. `update` cannot tell a
/// rebound table (a rebuilt id table on a changed commit-graph) apart from
/// an ordinary M1 refresh — that fact is only known on the job-runner thread
/// (`RepoStore::refresh`'s `Refreshed`) — so every restart reanchors.
#[test]
fn a_walk_restart_reanchors_on_the_previously_selected_commit() {
    let id = JobId::new(7);
    let (state, merge) = merge_on_screen(id, 1);
    assert_eq!(
        state
            .history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge),
        "the merge is selected before the restart"
    );

    let next_id = JobId::new(8);
    let (restarted, _) = walk_started(&state, next_id);

    assert_eq!(
        restarted.history.as_ref().map(|history| history.reanchor),
        Some(Some(merge)),
        "the new walk is told to land back on the commit that was selected"
    );
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
    let older = Oid::from_bytes(&[8; Oid::SHA1_BYTES]).expect("an oid");
    let name = graft_core::ref_name::RefName::new("refs/stash").expect("valid");
    let badges = crate::refs::badges(
        &[(name, stash)],
        &graft_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        &[(stash, 0), (older, 1)],
        |_, _| Err("no upstreams here".to_owned()),
    )
    .expect("badges");
    assert_eq!(badges.stash(), Some(stash));
    state.refs = Some(Arc::new(badges));

    // Both entries, not only the newest (m3-working-tree-undo-and-stash/06).
    let (hidden, commands) = update(&state, key(KeyCode::Char('H')));
    assert_eq!(
        asked_view(&commands).inclusion.hidden_stashes,
        [stash, older].into()
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

/// A left click on the overview strip's column (`overview-strip`,
/// m2-details-and-diffs/20) selects the clicked cell's bucket's first row,
/// same as `History::select_at` — not the ordinary graph click, which the
/// strip's column now shadows.
#[test]
fn a_click_on_the_strip_selects_the_buckets_first_row() {
    let id = JobId::new(23);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    state.strip_shown = true;
    state.history.as_mut().expect("a walk").rows = 2;

    // Bucket math (`overview_strip::bucket_range`), height 5 over 2 rows:
    // cell 2's range is `1..2` — the merge row — landing on screen row 2
    // (`HEADER_ROWS` + 1).
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 79,
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
        Some(merge_id),
        "the strip's click landed on the merge row, not the ordinary graph click"
    );
}

/// The strip hides no click from the wheel: `wheel` only special-cases the
/// detail pane's own regions (`detail_hit`), so a notch over the strip's
/// column falls through to the same viewport scroll as anywhere else on the
/// graph (`overview-strip`, m2-details-and-diffs/20).
#[test]
fn the_wheel_over_the_strip_still_scrolls_the_viewport() {
    let id = JobId::new(24);
    let (mut state, _) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    state.strip_shown = true;
    state.history.as_mut().expect("a walk").rows = 2;

    let scroll_down = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 79,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (down, _) = update(&state, scroll_down);
    assert_eq!(down.history.as_ref().map(|history| history.top), Some(1));
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

/// The state a focus report means something in: a repository is open.
fn opened_at(now_seconds: i64) -> AppState {
    AppState {
        repo_path: PathBuf::from("/code/api"),
        repository: Some(facts("/code/api", "main")),
        harness: Harness {
            clock: crate::session::Clock::Fixed(now_seconds),
            ..Harness::default()
        },
        ..AppState::default()
    }
}

#[test]
fn coming_back_to_the_window_re_reads_the_working_tree_and_nothing_else() {
    // Design note change 7: what another window did to the repository is
    // what the user expects to see on coming back. Not the history walk —
    // restarting it would move the cursor under their hands.
    // The launch journal read rides along on the first `update` of an open
    // repository, whatever the event; what matters is the working-tree read
    // and that no history walk starts.
    let (next, commands) = update(&opened_at(10), core(Event::FocusGained));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..) | Command::ShowRows(..))),
        "{commands:?}"
    );
    assert_eq!(next.last_focus_refresh, Some(10_000));
}

#[test]
fn a_second_focus_report_within_half_a_second_is_ignored() {
    // Terminals report focus in pairs, and tmux reports it per pane switch.
    let mut state = opened_at(10);
    state.last_focus_refresh = Some(10_000 - 100);
    let (next, commands) = update(&state, core(Event::FocusGained));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert_eq!(next.last_focus_refresh, Some(10_000 - 100), "untouched");

    state.last_focus_refresh = Some(10_000 - 600);
    let (_, commands) = update(&state, core(Event::FocusGained));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "600 ms later: {commands:?}"
    );
}

#[test]
fn a_focus_report_before_the_repository_opened_asks_for_nothing() {
    let before = AppState::default();
    let (next, commands) = update(&before, core(Event::FocusGained));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next, before);
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
            // The journal is re-read too: another Graft on the same worktree
            // can have appended to it (m3-working-tree-undo-and-stash/06).
            Command::ReadJournal(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 1)),
            Command::OpenRepository,
            // The walk runs under an id `update` minted here, so a second
            // `Ctrl+R` before its `Started` lands can still name it
            // (m5-history-and-ghost-preview/05).
            Command::ReadHistory(
                JobId::new(crate::update::detail::LOCAL_JOB_BIT | 2),
                Order::Topological,
            ),
            Command::ReadRefs(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 3)),
            // The remote facts, beside the refs (m4-remotes/05).
            Command::ReadRemotes(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 4)),
            Command::ReadWorkingTree(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 5)),
        ]
    );
    // Asking for work is all it does — apart from remembering which job the
    // journal read is, so a stale answer can be dropped by id. Everything
    // else is the jobs' to change, once they come back.
    let expected = AppState {
        journal: crate::update::journal::JournalState {
            entries: crate::update::detail::Loading::Pending(JobId::new(
                crate::update::detail::LOCAL_JOB_BIT | 1,
            )),
            reading: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 1)),
            ..state.journal.clone()
        },
        remotes: crate::update::detail::Loading::Pending(JobId::new(
            crate::update::detail::LOCAL_JOB_BIT | 4,
        )),
        remote: crate::update::remote::RemoteState {
            reading: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 4)),
            ..crate::update::remote::RemoteState::default()
        },
        // The walk is named at once, not when its `Started` event lands.
        awaiting: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 2)),
        // The working-tree read is named too: the status waits on it.
        status_read: Some(JobId::new(crate::update::detail::LOCAL_JOB_BIT | 5)),
        local_jobs: next.local_jobs,
        ..state.clone()
    };
    assert_eq!(next, expected);
}

/// `Ctrl+R`, and the job id the walk it asked for will run under.
fn refresh(state: &AppState) -> (AppState, Vec<Command>, JobId) {
    let (next, commands) = update(
        state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );
    let walk = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadHistory(id, _) => Some(*id),
            _ => None,
        })
        .expect("a refresh asks for a history walk");
    (next, commands, walk)
}

/// The `Ctrl+R` race (ADR-0037 §6, m5-history-and-ghost-preview/05).
///
/// Two refreshes with nothing fed back in between — no `Started`, no window,
/// nothing. The second one still has to cancel the walk the first one asked
/// for, which it can only do if that walk was named when it was asked for.
/// Left uncancelled, the walk parks on a worker for the rest of the run.
///
/// Deterministic by construction: this is `update` alone, so the ordering
/// under test is the one written here, not one a scheduler happened to pick.
#[test]
fn a_refresh_before_the_previous_walk_started_still_cancels_it() {
    let state = AppState {
        repo_path: PathBuf::from("/code/api"),
        ..AppState::default()
    };

    let (after_first, _, first) = refresh(&state);
    let (after_second, second_commands, second) = refresh(&after_first);

    assert_eq!(
        after_first.awaiting,
        Some(first),
        "the walk is named the moment it is asked for, not when it reports"
    );
    assert_ne!(first, second, "each refresh asks for its own walk");
    assert!(
        second_commands.contains(&Command::CancelJob(first)),
        "the second refresh must cancel the first refresh's walk: {second_commands:?}"
    );
    assert_eq!(after_second.awaiting, Some(second));
}

/// A walk that has been superseded still announces itself — the announcement
/// was already in flight when it was cancelled. Taking it would blank the
/// rows the live walk has already put on screen.
#[test]
fn stale_started_is_dropped() {
    let state = AppState {
        repo_path: PathBuf::from("/code/api"),
        ..AppState::default()
    };
    let (after_first, _, first) = refresh(&state);
    let (mut after_second, _, second) = refresh(&after_first);
    // The live walk has answered: the screen holds rows.
    let mut live = History::started();
    live.rows = 40;
    live.phase = Phase::Complete;
    after_second.history = Some(live.clone());

    let (next, commands) = update(&after_second, AppEvent::Started(first, Task::ReadHistory));

    assert_eq!(
        next.history,
        Some(live),
        "a superseded walk's `Started` must leave the rows on screen alone"
    );
    assert_eq!(next.awaiting, Some(second), "the live walk keeps the name");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ShowRows(id, _) if *id == first)),
        "no rows are asked of a walk that was cancelled: {commands:?}"
    );
}

#[test]
fn a_bare_r_asks_for_nothing() {
    // `r` is rebase on the history screen (DESIGN.md §2.3), not refresh.
    let (_, commands) = update(&AppState::default(), key(KeyCode::Char('r')));

    assert!(commands.is_empty());
}

/// Every field of [`AppState`] written out, so a field added to the state
/// stops this file compiling until [`the_same_event_always_gives_the_same_answer`]
/// has been given a value for it — a new field with hidden state is exactly
/// what that test is about.
#[allow(
    clippy::too_many_lines,
    reason = "one line per `AppState` field is the point: it grows with the state"
)]
fn every_field_spelled_out() -> AppState {
    AppState {
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
        operating: None,
        screen: Screen::Graph,
        lanes_expanded: false,
        focus: false,
        hover: None,
        detail: DetailState::default(),
        local_jobs: LocalJobs::default(),
        renderer_override: None,
        settled_tier: None,
        icons: graft_ui::graph_screen::Icons::Plain,
        max_lanes: None,
        search_prompt: None,
        search: None,
        on_working_tree: false,
        compare_base: None,
        context_menu: None,
        radar: crate::update::radar::RadarState::default(),
        radar_column_seen: std::cell::Cell::new(false),
        map: crate::update::map::MapState::default(),
        journal: crate::update::journal::JournalState::default(),
        stash: crate::update::stash::StashState::default(),
        remotes: detail::Loading::Idle,
        remote: crate::update::remote::RemoteState::default(),
        last_focus_refresh: None,
        strip_shown: false,
        confirm: None,
        credential: None,
        name_prompt: None,
        drag: None,
        commit_editor: None,
        commit_draft: None,
        in_progress: None,
        mergetool: None,
        banner_focus: false,
        ended_by_answer: false,
        live_entries: Vec::new(),
        stopped: None,
        land_on_head: false,
        head_landing: crate::update::HeadLanding::default(),
        live_landing_owed: false,
        status_read: None,
        head_message: None,
        commit_template: None,
        external_editor: None,
        suspend_on_ctrl_z: false,
        preview: None,
        resolver: None,
    }
}

#[test]
fn the_same_event_always_gives_the_same_answer() {
    // Purity's observable half: no clock, no counter, no hidden state, so
    // repeating a call repeats the answer.
    let state = every_field_spelled_out();

    let first = update(&state, key(KeyCode::Char('q')));
    let second = update(&state, key(KeyCode::Char('q')));

    assert_eq!(first, second);
    assert_eq!(state.quit, None, "the input state was mutated");
}

/// A repository that opened clears a read failure and nothing else.
///
/// The refresh a failed push asks for lands here, so wiping the status would
/// take `push rejected: …` off the screen before it could be read — and the
/// next `P`'s lease box is offered off the back of that rejection. `Busy` is
/// a running write's line and `Done` a finished one's; neither belongs to the
/// repository (m4-remotes/05 review, M2).
#[test]
fn only_a_read_failure_is_cleared_when_the_repository_opens() {
    let opened = |status: Status| {
        let before = AppState {
            status,
            ..AppState::default()
        };
        update(
            &before,
            AppEvent::Produced(
                JobId::new(1),
                Product::Repository(Ok(facts("/code/api", "main"))),
            ),
        )
        .0
        .status
    };

    let sticky = Status::sticky("push rejected: origin/main has moved · p pull first".to_owned());
    assert_eq!(opened(sticky.clone()), sticky, "a write's failure stays");
    assert_eq!(
        opened(Status::Failed("could not read the refs".to_owned())),
        Status::Ready,
        "a read failure is superseded"
    );
    let busy = Status::Busy("fetch all remotes · esc cancels".to_owned());
    assert_eq!(opened(busy.clone()), busy, "a running write's line stays");
    let done = Status::Done("pushed main → origin/main".to_owned());
    assert_eq!(opened(done.clone()), done, "a finished write's line stays");
    assert_eq!(opened(Status::Ready), Status::Ready);
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
    // Two commands: the launch reads of the undo journal and the remote
    // facts, which cannot run before the repository is open
    // (m3-working-tree-undo-and-stash/06, m4-remotes/05).
    assert!(
        matches!(
            commands.as_slice(),
            [Command::ReadJournal(_), Command::ReadRemotes(_)]
        ),
        "{commands:?}"
    );
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
    let (next, commands) = walk_started(&AppState::default(), id);

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
    let (state, _) = walk_started(&state, new);
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
///
/// Walks the directory tree rather than a hand-kept file list, so a new
/// `update/**/*.rs` file is included automatically instead of silently
/// skipped — a file inside a submodule directory (`update/detail/*.rs`) is
/// exactly as much `update`'s code as one directly under `update/`, and must
/// be scanned too (m2-details-and-diffs review M8: `detail.rs` and `mod.rs`
/// split into subdirectories after this test was written).
fn update_sources(dir: &std::path::Path) -> Vec<String> {
    let mut sources = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("update/ exists") {
            let path = entry.expect("readable dir entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
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
            // file's tail, the same way a single-file module used to strip
            // its `#[cfg(test)]`. Every other file is pure end to end and is
            // kept whole — stripping used to happen only after joining every
            // file, which threw away everything past `mod.rs`'s own marker.
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
    }
    sources
}

#[test]
fn update_cannot_reach_the_world() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/update");
    let sources = update_sources(std::path::Path::new(dir));
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

fn diff_of(path: &str) -> (DetailArc<DiffView>, DetailArc<graft_git::diff::FileDiff>) {
    let view = DetailArc::new(DiffView {
        path: sanitize_line(path),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        untracked: false,
    });
    let raw = DetailArc::new(graft_git::diff::FileDiff {
        entry: graft_git::diff::FileEntry {
            path: path.as_bytes().to_vec(),
            old_path: None,
            status: graft_git::diff::FileStatus::Modified,
            added: Some(0),
            removed: Some(0),
        },
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    });
    (view, raw)
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

    // The evil-merge probe follows the selection, not the pane (M2 review
    // M3): row 0 here is an ordinary commit, so opening the pane on it asks
    // for no probe, and closing the pane cancels nothing.
    assert!(
        !commands.iter().any(|c| matches!(c, Command::EvilProbe(..))),
        "an ordinary commit is not probed: {commands:?}"
    );

    let (again, commands) = update(&opened, key(KeyCode::Escape));
    assert!(!again.detail.shown);
    assert!(commands.is_empty(), "{commands:?}");
}

/// M2 review M3: the probe follows the selection, so the merge the cursor
/// rests on is probed with the pane closed — that is what puts the `⚠` on
/// the graph row rather than only inside the pane.
#[test]
fn the_selected_merge_is_probed_with_the_pane_closed() {
    let (state, merge) = merge_on_screen(JobId::new(41), 1);
    assert!(!state.detail.shown, "the pane starts closed");
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    assert!(!next.detail.shown, "folding does not open the pane");
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::EvilProbe(_, oid) if *oid == merge)),
        "the selected merge is probed: {commands:?}"
    );
    assert_eq!(next.detail.evil_at, Some(merge));
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
    state.working_tree = Some(std::sync::Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![
            graft_git::status::StatusEntry {
                path: "staged.txt".into(),
                original_path: None,
                staged: Some(graft_git::status::Change::Modified),
                unstaged: None,
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: false,
            },
            graft_git::status::StatusEntry {
                path: "unstaged.txt".into(),
                original_path: None,
                staged: None,
                unstaged: Some(graft_git::status::Change::Modified),
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: false,
            },
            graft_git::status::StatusEntry {
                path: "untracked.txt".into(),
                original_path: None,
                staged: None,
                unstaged: None,
                untracked: true,
                unmerged: false,
                unmerged_how: None,
                submodule: false,
            },
        ],
    }));
    let badges = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(head_id),
        &std::collections::HashMap::new(),
        &[],
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

/// m3-working-tree-undo-and-stash/04: the row is pinned, so `k` still
/// reaches it when `HEAD` is nowhere near the top. Before this it vanished
/// the moment the user scrolled — the one row they can always act on, gone
/// because they scrolled (`KNOWN_GAPS.md`, m2/06b).
#[test]
fn k_reaches_the_working_tree_row_even_with_head_scrolled_off() {
    let mut state = dirty_on_screen(JobId::new(40));
    // `HEAD` on a commit that is not the topmost visible row.
    let elsewhere = graft_core::oid::Oid::from_bytes(&[0x5a; graft_core::oid::Oid::SHA1_BYTES])
        .expect("20 bytes");
    let badges = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(elsewhere),
        &std::collections::HashMap::new(),
        &[],
        |_, _| unreachable!("no branches on this fixture to count"),
    )
    .expect("no refs to fail reading");
    state.refs = Some(Arc::new(badges));
    assert!(
        state.uncommitted_row_shown(),
        "the row draws: the tree is dirty"
    );
    assert!(
        !state.uncommitted_row_attached(),
        "but it is not on HEAD's lane"
    );

    let (next, _) = update(&state, key(KeyCode::Char('k')));
    assert!(next.on_working_tree);
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
            Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
                status: std::sync::Arc::new(graft_git::status::WorkingTreeStatus::default()),
                in_progress: None,
                mergetool: None,
            }))),
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
    // The current selection is already "followed", and its evil-merge probe
    // already answered (m2-details-and-diffs/14): no `follow_selection`
    // fetch or probe fires behind the click and confuses the command
    // assertions below.
    state.detail.spec = state.selected_spec();
    if let Some(crate::update::detail::DetailSpec::Commit(oid)) = state.detail.spec {
        state.detail.evil_at = Some(oid);
        state.detail.evil = Loading::Ready(
            graft_ops::mistake_radar::probes::EvilVerdict::NotCheckable("not a merge".into()),
        );
    }
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

/// m2-details-and-diffs/23: a harness prelude waits for the first window and
/// the refs answer, then feeds one key per settled state: its `Enter` opens
/// the pane on row 0, not on nothing, and the `j` after it waits for the
/// file list that `Enter` asked for.
#[test]
fn the_prelude_waits_for_the_window_and_the_refs_and_then_opens_row_0() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![
        KeyPress::plain(KeyCode::Enter),
        KeyPress::plain(KeyCode::Char('j')),
    ];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    assert!(!state.detail.shown, "no window yet: nothing to open on");
    assert_eq!(state.harness.prelude.len(), 2, "both keys still waiting");

    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    assert!(
        !state.detail.shown,
        "the window is here but the working-tree status is not: still waiting"
    );
    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert!(state.detail.shown, "Enter was fed once the window landed");
    assert!(state.detail.spec.is_some(), "the pane is on a real row");
    assert!(
        matches!(state.detail.files, detail::Loading::Pending(_)),
        "Enter asked for row 0's files"
    );
    assert_eq!(state.harness.prelude.len(), 1, "j waits for the file list");
}

/// A clean working-tree answer, the way the status job reports one.
fn working_tree_read(id: JobId) -> AppEvent {
    AppEvent::Produced(
        id,
        Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
            status: std::sync::Arc::new(graft_git::status::WorkingTreeStatus::default()),
            in_progress: None,
            mergetool: None,
        }))),
    )
}

/// m3-working-tree-undo-and-stash/07: a prelude key waits for a write in
/// flight. `u` queued behind `Space` must undo the stage, not race it — the
/// acceptance captures press exactly such sequences.
#[test]
fn the_prelude_holds_a_key_while_a_write_is_in_flight() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Char('J'))];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    state.operating = Some(JobId::new(77));

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "a write is in flight: J waits"
    );
    assert!(!state.journal.shown);

    let mut state = state;
    state.operating = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "the write landed: J was fed"
    );
    assert!(state.journal.shown, "and it opened the journal browser");
}

/// m5-history-and-ghost-preview/11: a preview whose plan is still being
/// worked out is a job an earlier key started. The acceptance captures
/// press `m → ⏎` — `→` and `⏎` on a box with no plan yet would close it
/// and the pty would record a graph with no box, which proves nothing.
#[test]
fn the_prelude_holds_a_key_while_a_preview_plan_is_pending() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Right)];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let planning = JobId::new(78);
    state.preview = Some(super::preview::PreviewState {
        subject: super::preview::PreviewSubject::RevertTo(graft_core::journal::EntryId::new(
            0, [0; 10],
        )),
        resident: Vec::new(),
        slice_top: 0,
        job: planning,
        plan: Loading::Pending(planning),
        rows: Vec::new(),
        view_start: 0,
        selected_row: None,
        ghost_index: std::collections::HashMap::new(),
        clipped: 0,
        undrawable: None,
        cursor: 0,
        return_to_journal: false,
    });

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "the plan is in flight: → waits"
    );

    let mut state = state;
    state.preview = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "the box is gone: → was fed"
    );
}

/// m6-conflicts/07 F6: the resolver's opening read (`job`) and a `B` base
/// load (`base_job`) are jobs an earlier key started. A take fed before the
/// read lands is refused (`resolver/take.rs`), so the acceptance captures
/// recorded refusals instead of takes.
#[test]
fn the_prelude_holds_a_key_while_the_resolver_is_reading() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Char('o'))];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let reading = JobId::new(79);
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("conflicted.rs"),
        opened_from: resolver::OpenedFrom::Pane,
        file: None,
        whole_file: None,
        how: None,
        cursor_block: None,
        top: 0,
        base_shown: false,
        job: Some(reading),
        reading: true,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
    });

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "the opening read is in flight: o waits"
    );

    let mut state = state;
    let open = state.resolver.as_mut().expect("still open");
    open.job = None;
    open.base_job = Some(JobId::new(80));
    let (state, _) = update(&state, sized(80, 12));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "a base load is in flight: o waits"
    );

    let mut state = state;
    state.resolver.as_mut().expect("still open").base_job = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "nothing in flight: o was fed"
    );
}

/// m4-remotes/06: the one exception to the rule above. A credential prompt
/// is a text field waiting for a key, and the write that asked for it *is*
/// still in flight — a prelude that waited for `operating` to clear would
/// deadlock, and the acceptance walk could never answer a password.
#[test]
fn prelude_proceeds_into_credential_prompt() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![
        KeyPress::plain(KeyCode::Char('p')),
        KeyPress::plain(KeyCode::Enter),
    ];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let push = JobId::new(77);
    state.operating = Some(push);

    let (state, commands) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(state.harness.prelude.len(), 2, "a write is in flight");

    // git asks; the prompt opens and the prelude starts feeding again.
    let (state, commands_after) = update(
        &state,
        AppEvent::Progress(
            push,
            Progress::CredentialNeeded {
                prompt: "Password for 'http://u@h':".to_owned(),
                pulse_seed: false,
            },
        ),
    );

    assert!(
        state.harness.prelude.is_empty(),
        "both keys were fed into the prompt"
    );
    assert!(state.credential.is_none(), "Enter closed it");
    let answered: Vec<_> = commands
        .iter()
        .chain(&commands_after)
        .filter(|command| matches!(command, Command::AnswerCredential(..)))
        .collect();
    assert_eq!(
        answered,
        vec![&Command::AnswerCredential(
            push,
            crate::update::credential::Answer::new(zeroize::Zeroizing::new(b"p".to_vec()))
        )],
        "the `p` landed in the field, not on the pull key, and Enter sent it"
    );
}

/// Critical 1 of the m3/06 review: a global binding must never fire while a
/// text field has the keyboard. Typing `update` as a stash message launched
/// an undo at the `u` — a journaled write nobody asked for, from inside a
/// field the same milestone had just added.
///
/// One test per text-entry surface in the app, each driven through
/// [`update`] so the whole key path runs, because the defect was in the
/// *order* of that path and a handler called directly cannot see it.
mod a_text_field_keeps_its_own_letters {
    use super::*;

    /// `u`, `U` and `Ctrl+Z`, in that order.
    fn undo_keys() -> [AppEvent; 3] {
        [
            key(KeyCode::Char('u')),
            key(KeyCode::Char('U')),
            AppEvent::Core(Event::Key(KeyPress {
                code: KeyCode::Char('z'),
                modifiers: Modifiers::CONTROL,
            })),
        ]
    }

    /// Feeds the three keys and fails if any of them asked for an undo or a
    /// redo, or put the app into a write.
    fn press_undo_keys(state: &AppState) -> AppState {
        let mut state = state.clone();
        for event in undo_keys() {
            let (next, commands) = update(&state, event);
            assert!(
                !commands
                    .iter()
                    .any(|command| matches!(command, Command::Undo(..))),
                "a text field's key asked for an undo: {commands:?}"
            );
            assert!(next.operating.is_none(), "a write started from a letter");
            state = next;
        }
        state
    }

    #[test]
    fn typing_u_into_the_stash_message_writes_letters_not_an_undo() {
        let state = AppState::default();
        let (state, _) = update(&state, key(KeyCode::Char('S')));
        let (state, _) = update(&state, key(KeyCode::Char('s')));
        assert!(state.stash.prompt.is_some(), "`s` opened the save prompt");

        let state = press_undo_keys(&state);

        let prompt = state.stash.prompt.as_ref().expect("the prompt is open");
        assert_eq!(
            prompt.message.text(),
            "uU",
            "`u` and `U` are letters here; `Ctrl+Z` is not text and is swallowed"
        );
    }

    #[test]
    fn typing_u_into_the_commit_subject_writes_letters_not_an_undo() {
        let state = AppState {
            commit_editor: Some(commit_editor::CommitEditor::new()),
            ..AppState::default()
        };
        let state = press_undo_keys(&state);

        let editor = state.commit_editor.as_ref().expect("the editor is open");
        assert_eq!(editor.subject.text(), "uU");
    }

    #[test]
    fn typing_u_into_the_commit_body_writes_letters_not_an_undo() {
        let mut editor = commit_editor::CommitEditor::new();
        editor.focus = graft_ui::commit_editor::Field::Body;
        let state = AppState {
            commit_editor: Some(editor),
            ..AppState::default()
        };
        let state = press_undo_keys(&state);

        let editor = state.commit_editor.as_ref().expect("the editor is open");
        assert_eq!(editor.body.text(), "uU");
        assert_eq!(editor.subject.text(), "", "the body had focus, not this");
    }

    #[test]
    fn typing_u_into_the_search_prompt_writes_letters_not_an_undo() {
        // `/` only opens over a walk that exists.
        let (state, _) = merge_on_screen(JobId::new(1), 0);
        let (state, _) = update(&state, key(KeyCode::Char('/')));
        assert!(state.search_prompt.is_some(), "`/` opened the prompt");

        let state = press_undo_keys(&state);

        assert_eq!(
            state.search_prompt.as_deref(),
            Some("uU"),
            "the query takes the letters"
        );
    }

    /// The predicate itself, so a text field added later is caught by name
    /// rather than by whichever handler happens to run first.
    #[test]
    fn every_text_field_the_app_has_is_named_by_text_field_open() {
        assert!(!AppState::default().text_field_open(), "none open");

        let searching = AppState {
            search_prompt: Some(String::new()),
            ..AppState::default()
        };
        assert!(searching.text_field_open(), "the search prompt");

        let committing = AppState {
            commit_editor: Some(commit_editor::CommitEditor::new()),
            ..AppState::default()
        };
        assert!(committing.text_field_open(), "the commit editor");

        let mut stashing = AppState::default();
        stashing.stash.prompt = Some(stash::StashPrompt::default());
        assert!(stashing.text_field_open(), "the stash save prompt");
    }
}

/// Major 5 of the m3/06 review: every key handler copies `local_jobs` out of
/// the state, hands a closure over the copy to a callee, and writes the copy
/// back — so a callee that mints from `state.local_jobs` has its ids thrown
/// away, and the next mint reissues an id still in flight. Two callees were
/// fixed by threading `next_id` into them; the pattern that made it possible
/// is still at fourteen call sites, and the existing regression test only
/// names those two callees.
///
/// This is the guard for the class: over a scripted run, no local job id is
/// ever handed out twice. It reads the ids out of each `Command`'s `Debug`
/// rather than matching thirty variants by hand — deliberately, because a
/// match would have to be extended for every new command and would go quietly
/// stale, which is the same failure this test exists to prevent.
#[test]
fn no_local_job_id_is_ever_minted_twice() {
    use std::collections::HashMap;

    let events = [
        key(KeyCode::Char('J')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char(' ')),
        key(KeyCode::Char('J')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char(' ')),
        key(KeyCode::Char('s')),
        key(KeyCode::Escape),
        key(KeyCode::Char('S')),
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
        key(KeyCode::Char('u')),
        key(KeyCode::Char('U')),
        core(Event::FocusGained),
    ];

    let mut state = AppState {
        repository: Some(facts("/code/api", "main")),
        ..AppState::default()
    };
    // Which press first handed out each id, so a clash names both.
    let mut seen: HashMap<u64, String> = HashMap::new();
    for (press, event) in events.into_iter().enumerate() {
        let (next, commands) = update(&state, event);
        for command in &commands {
            // `CancelJob` names a job that already exists; it is the one
            // command that legitimately repeats an id.
            if matches!(command, Command::CancelJob(_)) {
                continue;
            }
            let rendered = format!("{command:?}");
            for id in local_job_ids(&rendered) {
                let what = format!("press {press}: {rendered}");
                if let Some(first) = seen.insert(id, what.clone()) {
                    panic!("job id {id} handed out twice — first {first}, then {what}");
                }
            }
        }
        state = next;
    }
}

/// Every `JobId(n)` in a `Debug` rendering that carries the local-job bit.
fn local_job_ids(rendered: &str) -> Vec<u64> {
    let mut found = Vec::new();
    let mut rest = rendered;
    while let Some(at) = rest.find("JobId(") {
        rest = &rest[at + "JobId(".len()..];
        let end = rest.find(')').unwrap_or(rest.len());
        if let Ok(id) = rest[..end].parse::<u64>()
            && id & crate::update::detail::LOCAL_JOB_BIT != 0
        {
            found.push(id);
        }
        rest = &rest[end..];
    }
    found
}

/// `Product::Remotes` under any id but the read in flight is dropped: a
/// slow answer from before a refresh must not replace a newer one
/// (m4-remotes/05).
#[test]
fn stale_remotes_dropped() {
    let facts = |branch: &str| {
        Arc::new(crate::jobs::remote::RemoteFacts {
            remotes: Vec::new(),
            head_branch: Some(branch.to_owned()),
            head_upstream: None,
        })
    };
    let mut state = AppState::default();
    let (old_id, fresh) = (JobId::new(7), JobId::new(8));
    state.remote.reading = Some(fresh);
    state.remotes = detail::Loading::Pending(fresh);
    let (next, _) = update(
        &state,
        AppEvent::Produced(old_id, Product::Remotes(Ok(facts("refs/heads/old")))),
    );
    assert_eq!(
        next.remotes,
        detail::Loading::Pending(fresh),
        "the stale answer is dropped"
    );
    let (next, _) = update(
        &next,
        AppEvent::Produced(fresh, Product::Remotes(Ok(facts("refs/heads/main")))),
    );
    assert_eq!(
        next.remotes,
        detail::Loading::Ready(facts("refs/heads/main"))
    );
    assert_eq!(next.remote.reading, None);
}

/// **Every key the bar draws, in any context, at any width, is answered by
/// the handler that has focus in that state** (m5/31). The invariant three
/// of this milestone's defects fell through, in one sweep.
///
/// It grew out of `every_hinted_key_is_bound`, whose narrower question —
/// is this `(key, verb)` in `keys::bound_actions`? — is still asked in the
/// same loop, because the table is what ties a key to the **verb** the hint
/// names: `M` is bound, but to the Map view, so a hint reading
/// `M mergetool` is as dead as one on an unbound key (m5-03 review,
/// Critical 2). Its earlier version carved out `m`/`r`/`b`/`t`/`s`/`a` and
/// compared keys only; there are no exemptions now, because a key the
/// design names but no group has built is `HintState::Planned` and is never
/// drawn.
///
/// What that table cannot say is **where** a binding is live, and `⏎ open`
/// went straight through the hole: `Enter` is bound, and with the detail
/// pane open on its diff it was consumed and did nothing while the bar
/// still advertised it (m5 review, Q2). So every hint is also **pressed**,
/// into a state that really is the screen its bar is drawn on.
///
/// **"Answered", without a second table of effects.** Each key is pressed
/// twice over: once as itself, once as [`INERT`] — a key nothing binds.
/// Whatever the state does on any event at all (the evil-merge probe
/// re-asking for rows, a job taking an id) happens under both, so a key is
/// answered exactly when what comes back differs from the inert press.
/// Nothing here states what a key *should* do, so there is no second table
/// to drift out of step with the first.
///
/// **Enumerated, not sampled.** The contexts come from
/// `graph_screen::every_context`, the one list every sweep now reads, and
/// [`state_for`] matches it exhaustively — a context added to the enum
/// cannot compile until it has been given a state here.
/// [`every_swept_state_is_the_context_it_claims`] then makes the app's own
/// classifier confirm each state, so a fixture cannot quietly be a
/// different screen from the one whose bar is being checked.
///
/// **A hint naming two keys** (`j/k`, `<>`) is answered when *either* acts:
/// `k` on the top row has nowhere to go, and a boundary is not a dead key.
///
/// **At the width the bar is drawn for** (m5/33, review M6). The fixture is
/// built at each of [`SWEPT_WIDTHS`], not at 160 for all six, so the
/// handlers see the layout the bar belongs to — the sub-100-column `Tab`
/// layout is where tickets 29 and 31's defects lived, and the sweep was
/// pressing its bars into a wide side-by-side state. All six widths settle.
/// The `Divider` the bar is asked for comes from the state through
/// `view::rows::divider_shown` rather than from the loop, for the same
/// reason: a bar the app never draws is not a bar to check. Doing this
/// found one dead key straight away — `<> resize` under 100 columns, where
/// the pane is a tab strip with no divider to drag (fixed in
/// `graft_ui::graph_screen::GraphView::divider_shown`).
#[test]
fn every_key_the_bar_draws_is_answered() {
    use graft_ui::graph_screen::{Divider, every_context};

    let bound = super::keys::bound_actions();
    // Collected rather than asserted one at a time: when the bar and the
    // handlers disagree they tend to disagree in several places at once,
    // and a run that names one of them hides the shape of it.
    let mut dead: Vec<String> = Vec::new();
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in SWEPT_WIDTHS {
                dead.extend(dead_keys_on(&context, divider, columns, bound));
            }
        }
    }
    assert!(
        dead.is_empty(),
        "the bar draws {} key(s) the handler with focus does nothing with:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}

/// **The bar the Detail tab draws with an overlay behind it answers every
/// key on it** (m5/42, the review's Q1 as m5/36 confirmed it).
///
/// The sweep above now covers this state too, through
/// `HintContext::PaneOverOverlay` — this test is the same claim made where
/// the defect was, so a failure names the overlay and the focus rather than
/// a context enum. Before the fix it reported, for each of the three
/// overlays and both focuses, a bar of the graph's verbs with every one of
/// them dead: `b branch`, `r rebase`, `t tag`, `c commit`, `y sha`,
/// `m merge`, `q quit`, `F2 status`, `F3 renderer`, `x compare`,
/// `+ lanes`, `. more` — the overlay's `shown_key` claims the keyboard
/// before `graph_key` ever runs.
#[test]
fn the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows() {
    use graft_ui::graph_screen::{Divider, hints_for};

    let mut dead: Vec<String> = Vec::new();
    for overlay in EVERY_OVERLAY {
        for opens in [true, false] {
            let state = settled(overlay_over(commit_pane(JobId::new(701), opens), overlay));
            // The bar the frame really draws over this state, at the width
            // where the Detail tab owns the screen (`view::frame`'s `Tab`
            // layout, the only layout that draws a bar under an overlay).
            let state = AppState {
                size: Size {
                    columns: 80,
                    rows: 48,
                },
                ..state
            };
            assert_eq!(
                crate::update::detail::tab_hint_rows(&state),
                1,
                "{overlay:?} (opens {opens}): the Detail tab reserves its hint row"
            );
            let context = crate::view::rows::pane_hint_context(&state);
            let idle = nothing_happened(&state);
            for hint in hints_for(&context, Divider::Hidden, 80) {
                if !answered(&state, &idle, hint.key) {
                    dead.push(format!(
                        "{overlay:?} (opens {opens}) draws `{} {}`",
                        hint.key, hint.verb
                    ));
                }
            }
        }
    }
    assert!(
        dead.is_empty(),
        "the bar behind an overlay draws {} key(s) nothing answers:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}

/// One cell of [`every_key_the_bar_draws_is_answered`]'s sweep: this
/// context, at this width, with the pane open or not — every hint its bar
/// draws that the handler with focus does nothing with.
fn dead_keys_on(
    context: &HintContext,
    divider: Divider,
    columns: u16,
    bound: &[(&str, &str)],
) -> Vec<String> {
    states_for(context, divider, columns)
        .into_iter()
        .flat_map(|state| dead_keys_in(&settled(state), context, divider, columns, bound))
        .collect()
}

/// [`dead_keys_on`] against one of the screens that context can be the bar
/// for — every hint the bar draws that this screen's focused handler does
/// nothing with.
fn dead_keys_in(
    state: &AppState,
    context: &HintContext,
    divider: Divider,
    columns: u16,
    bound: &[(&str, &str)],
) -> Vec<String> {
    use graft_ui::graph_screen::hints_for;

    let state = state.clone();
    // Both bars this state can be under: the one the enum names, and the one
    // `view::rows` actually asks for when handed the state (they differ only
    // in row decoration, but a classifier that reads the state wrongly is
    // exactly how a live key and a drawn key come apart — so the keys it
    // picks are pressed too).
    let mut drawn = vec![context.clone()];
    if classified_form(context).is_some() {
        let banner = crate::update::banner::banner_view(&state);
        let asked = crate::view::rows::footer_context(&state, &[], banner.as_ref());
        if asked != *context {
            drawn.push(asked);
        }
    }
    let idle = nothing_happened(&state);
    // Any key at all ends a drag: `update::drag::drag_key` takes `state.drag`
    // before it reads the key. So while a badge is in the air the inert key
    // moves the state too and no press can be told from another; the sweep
    // says which states it is blind in rather than reading every key there as
    // answered by accident.
    let blind = idle.0 != state;
    // The sweep's own baseline must read as *not* an answer, or every key
    // here would read as one. `nothing_happened` makes that true today by
    // asserting the two inert presses agree outright; pinned again here, at
    // the point that depends on it, so relaxing the baseline cannot quietly
    // turn the sweep green (m5/33, review M2).
    assert!(
        blind
            || !did_something(
                &idle,
                &outcome(&state, Press::Key(KeyPress::plain(ALSO_INERT)))
            ),
        "{context:?} ({divider:?}) at {columns} columns: a key nothing binds reads as an \
         answer, so every key drawn on this bar would"
    );
    // A live confirm box is blind the same way: every key it has no use
    // for cancels it, the inert one included. Its three hints are pinned
    // one by one where the box is (`confirm::tests`: `←`/`→`
    // `left_on_cancel_stays_on_cancel`, `right_moves_onto_the_acting_choice_and_clamps`;
    // `⏎` `enter_on_confirm_dispatches_the_operation`; `esc`
    // `escape_closes_without_dispatching`).
    assert!(
        !blind || state.drag.is_some() || state.confirm.is_some(),
        "{context:?} ({divider:?}) at {columns} columns: a key nothing binds moved the \
         state, and not by dropping a dragged badge — every key here now reads as answered \
         whether it is or not"
    );
    // The divider the **app** would draw this bar with, not the fixture's
    // knob: under 100 columns the pane is a tab strip and
    // `view::rows::divider_shown` says there is nothing to drag, so asking
    // for the `Shown` bar here would be asking for a bar the app never draws
    // (m5/33, review M6).
    let drawn_divider = if crate::view::divider_shown(&state) {
        Divider::Shown
    } else {
        Divider::Hidden
    };

    let mut dead = Vec::new();
    let mut pressed: Vec<(&str, &str)> = Vec::new();
    for bar in &drawn {
        for hint in hints_for(bar, drawn_divider, columns) {
            assert!(
                bound.contains(&(hint.key, hint.verb)),
                "{bar:?} at {columns} columns hints `{} {}`, which nothing binds",
                hint.key,
                hint.verb
            );
            if pressed.contains(&(hint.key, hint.verb)) {
                continue;
            }
            pressed.push((hint.key, hint.verb));
            if !blind && !answered(&state, &idle, hint.key) {
                // Both names: the screen the state is, and the bar the key
                // came off — they differ exactly when the classifier reads
                // the state as another screen, and then which one is wrong is
                // the whole question.
                dead.push(format!(
                    "{context:?} ({divider:?}) at {columns} columns, bar of {bar:?}, \
                     draws `{} {}`",
                    hint.key, hint.verb
                ));
            }
        }
    }
    dead
}

/// The states [`every_key_the_bar_draws_is_answered`] presses keys into are
/// the screens they claim to be — **the app's own classifier says so**, not
/// the fixture. Without this the sweep could press every key into the wrong
/// state and stay green, which is the same fault it exists to catch.
///
/// With no drawn rows to read badges or a marker off, `view::rows`
/// answers `PlainRow` where the bar's context says `BranchRow` or `Marked`:
/// the difference is decoration on the row, and the keys are the same. That
/// is the only gap left: since m5/42 the three overlays are classified like
/// everything else, so a fixture with the journal, the stash or the Map
/// view open is checked against the bar the app would really draw over it.
#[test]
fn every_swept_state_is_the_context_it_claims() {
    use graft_ui::graph_screen::{Divider, every_context};

    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let Some(want) = classified_form(&context) else {
                continue;
            };
            for state in states_for(&context, divider, 160) {
                let banner = crate::update::banner::banner_view(&state);
                assert_eq!(
                    crate::view::rows::footer_context(&state, &[], banner.as_ref()),
                    want,
                    "a fixture for {context:?} ({divider:?}) is not that screen"
                );
            }
        }
    }
}

/// What `view::rows::footer_context` answers for [`state_for`]'s state,
/// with no drawn rows to read a badge or a marker from, or `None` where the
/// graph's footer never classifies the context at all.
fn classified_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::BranchRow { .. } | HintContext::Marked(_) => Some(HintContext::PlainRow),
        HintContext::PaneFocused { opens, .. } => Some(HintContext::PaneFocused {
            row: Box::new(HintContext::PlainRow),
            opens: *opens,
        }),
        // The resolver is an overlay now, and `view::rows::overlay_context`
        // answers exactly this form for it. `base`/`restart` are both
        // `true` (m6-conflicts/05): `B`/`d` act the moment they are
        // pressed, so this is the one combination the real app ever
        // produces — the other three have no state to classify and are
        // left unswept (their two hints are `Planned`, which `drawable`
        // filters out, so nothing dead can hide there).
        HintContext::Resolver {
            base: true,
            restart: true,
            whole_file: false,
            roles,
        } => Some(HintContext::Resolver {
            base: true,
            restart: true,
            whole_file: false,
            roles: *roles,
        }),
        // A whole-file card (m6-05 review, Major 1): the one other form.
        HintContext::Resolver {
            base: false,
            restart: false,
            whole_file: true,
            roles,
        } => Some(HintContext::Resolver {
            base: false,
            restart: false,
            whole_file: true,
            roles: *roles,
        }),
        // `Preview`'s box draws its own footer: `hint_context` is not the
        // code that decides it. The three `Resolver` forms above this arm
        // never reach the app.
        HintContext::Preview | HintContext::Resolver { .. } => None,
        // The wrapper classifies as its row does.
        HintContext::OperationLive(row) => {
            classified_form(row).map(|row| HintContext::OperationLive(Box::new(row)))
        }
        // The three overlays and the pane with one behind it **are**
        // classified, since m5/42: `view::rows` reads
        // `AppState::graph_overlay` and answers the overlay's own list with
        // the graph focused, or `PaneOverOverlay` with the detail pane
        // focused. Before that the overlays were `None` here — and that
        // hole is exactly why a sweep carrying this invariant passed a bar
        // of nine dead keys (the ticket's own finding).
        HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::WorkingTreeRow
        | HintContext::PlainRow
        | HintContext::Banner { .. }
        | HintContext::ChangesPane { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential => Some(context.clone()),
    }
}

/// The screen widths [`every_key_the_bar_draws_is_answered`] sweeps: the
/// two layout breakpoints (100 and 160 columns, DESIGN.md §1) with a width
/// either side of each. The state is built at the width whose bar is being
/// checked, so the handlers see the layout the bar was drawn for
/// (m5/33, review M6).
const SWEPT_WIDTHS: [u16; 6] = [40, 80, 100, 120, 160, 200];

/// A key nothing in the app binds — the baseline every press is measured
/// against.
const INERT: KeyCode = KeyCode::Function(12);
/// A second one, so the baseline can be shown to *be* a baseline rather
/// than an action of its own.
const ALSO_INERT: KeyCode = KeyCode::Function(11);

/// The state once the app has stopped reacting to being looked at.
///
/// A freshly built pane asks for what it is missing on the first event it
/// sees (`ListFiles` for a pane whose files are `Idle`), which moves the
/// state under a key that binds nothing and would leave the whole screen
/// reading as answered. Pressing an unbound key until nothing moves settles
/// that; whatever moves afterwards is the key's doing.
///
/// A drag is not settled — every key ends one (see the sweep's `blind`),
/// so settling would put the badge down and throw the context away.
fn settled(mut state: AppState) -> AppState {
    // A confirm box is the same: every key it has no use for cancels it
    // (`confirm::confirm_key`'s catch-all), so settling would shut it.
    if state.drag.is_some() || state.confirm.is_some() {
        return state;
    }
    for _ in 0..4 {
        let (next, _) = update(&state, key(INERT));
        if next == state {
            return state;
        }
        state = next;
    }
    panic!("the state never stopped moving under a key that binds nothing");
}

/// What this state looks like after a key **nothing binds** — the shape of
/// nothing happening here.
///
/// It has to be a real press, not an empty outcome: `update` does ambient
/// work on every event whatever the key (the evil-merge probe re-asks for
/// its rows, a job takes an id), so an absolute comparison would read every
/// key as answered.
fn nothing_happened(state: &AppState) -> (AppState, Vec<Command>) {
    let idle = update(state, key(INERT));
    assert_eq!(
        update(state, key(ALSO_INERT)),
        idle,
        "the baseline must be a key that does nothing, not one that does something"
    );
    idle
}

/// Does the key the bar spells `name` do anything at all in this state,
/// beyond what a key nothing binds already does?
///
/// This used to be `outcome(..) != *idle`, under a comment claiming that
/// comparing against the inert press is what catches a key **consumed by
/// the focused handler that then does nothing**. It is the inverse (m5/33,
/// review M2). Such a key returns early, so the ambient work never runs and
/// its outcome differs from the baseline by having *less* in it — and a
/// plain `!=` reads any difference as an answer, so the one defect the
/// sweep exists for was the one it waved through.
///
/// The real guarantee is [`did_something`]: **added**, not merely
/// different. Nothing here assumes the baseline is empty, because it is
/// not — `update` does ambient work on every event whatever the key (the
/// evil-merge probe re-asks for its rows), and the inert press comes back
/// carrying that command like any other press.
fn answered(state: &AppState, idle: &(AppState, Vec<Command>), name: &str) -> bool {
    presses_for(name)
        .into_iter()
        .any(|press| did_something(idle, &outcome(state, press)))
}

/// Did this press **add** anything to what a key nothing binds already
/// does: move the state, or produce a command the baseline did not?
///
/// Losing the baseline's ambient work is not an answer, so a press whose
/// commands are a subset of the baseline's and whose state is the
/// baseline's counts as nothing happening.
fn did_something(idle: &(AppState, Vec<Command>), outcome: &(AppState, Vec<Command>)) -> bool {
    outcome.0 != idle.0 || outcome.1.iter().any(|command| !idle.1.contains(command))
}

/// What a hint tells the hand to do. Not every entry on the bar is a key:
/// `release drop on a row` is the mouse button coming up, which is how a
/// badge drag ends (`update::drag`). The sweep **performs** it rather than
/// exempting it — a rule that needs no list of words to skip, and one that
/// would notice if the release stopped ending the drag.
#[derive(Clone, Copy, Debug)]
enum Press {
    /// A key went down.
    Key(KeyPress),
    /// The left button came up.
    Release,
}

/// The presses `name` stands for. A hint naming two keys stands for both.
///
/// Every spelling the hint tables use is here; anything else stops the
/// sweep rather than being quietly passed over, because a bar entry nobody
/// can perform is the defect this test is for.
fn presses_for(name: &str) -> Vec<Press> {
    let plain = |code| Press::Key(KeyPress::plain(code));
    match name {
        "release" => vec![Press::Release],
        "⏎" => vec![plain(KeyCode::Enter)],
        "esc" => vec![plain(KeyCode::Escape)],
        "space" => vec![plain(KeyCode::Char(' '))],
        "⇥" => vec![plain(KeyCode::Tab)],
        "⌃A" => vec![Press::Key(KeyPress::control(KeyCode::Char('a')))],
        "⌃R" => vec![Press::Key(KeyPress::control(KeyCode::Char('r')))],
        "F2" => vec![plain(KeyCode::Function(2))],
        "F3" => vec![plain(KeyCode::Function(3))],
        "j/k" => vec![plain(KeyCode::Char('j')), plain(KeyCode::Char('k'))],
        "g/G" => vec![plain(KeyCode::Char('g')), plain(KeyCode::Char('G'))],
        "PgUp/PgDn" => vec![plain(KeyCode::PageUp), plain(KeyCode::PageDown)],
        "<>" => vec![plain(KeyCode::Char('<')), plain(KeyCode::Char('>'))],
        "←/→" => vec![plain(KeyCode::Left), plain(KeyCode::Right)],
        one => {
            let mut letters = one.chars();
            match (letters.next(), letters.next()) {
                (Some(letter), None) => vec![plain(KeyCode::Char(letter))],
                _ => panic!(
                    "the bar draws `{one}`, which this sweep cannot perform — teach \
                     `presses_for` what it is, or it is a bar entry nobody can act on"
                ),
            }
        }
    }
}

/// `press` put through the whole of `update`, exactly as the event loop
/// would deliver it.
fn outcome(state: &AppState, press: Press) -> (AppState, Vec<Command>) {
    let event = match press {
        Press::Key(key) => AppEvent::Core(Event::Key(key)),
        Press::Release => AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        })),
    };
    update(state, event)
}

/// **The screen each context's bar is drawn on.** Exhaustive over
/// `HintContext`, so a context added to the enum has to be given a state
/// here before this crate compiles again — which is what keeps the sweep
/// from quietly covering one context fewer than the bar can draw.
///
/// `divider` is not decoration: `Divider::Shown` means a pane is open on
/// this frame, and `<> resize` is drawn because of it. A state built
/// without the pane would fail its own hint.
fn states_for(context: &HintContext, divider: Divider, columns: u16) -> Vec<AppState> {
    screen_for(context)
        .into_iter()
        .map(|mut state| {
            // The width the bar being checked was drawn for (m5/33, review
            // M6). It used to be 160 for every fixture while the sweep drew
            // bars at six widths, so every narrow bar was pressed into a
            // wide, side-by-side state — and the `Tab` layout under 100
            // columns is exactly where tickets 29 and 31's defects lived.
            state.size = Size { columns, rows: 48 };
            one_stash(&mut state);
            if divider == Divider::Shown && !state.detail.shown {
                // On a commit, even where the cursor is on the working-tree
                // row: a pane open on the working tree *is* the changes
                // pane, which is its own context. This is the moment after
                // the cursor moved onto the row and before the pane has
                // followed it.
                let spec = head_spec(&state)
                    .or_else(|| state.selected_spec())
                    .expect("a row to open the pane on");
                show_pane(&mut state, spec, DetailFocus::Graph);
            }
            state
        })
        // With the pane open and the graph focused, `Tab` goes to the pane,
        // not the banner, so the bar never leads with `⇥ banner` there
        // (`banner::one_tab_to_banner`): that screen is not this context.
        .filter(|state| {
            !matches!(context, HintContext::OperationLive(_))
                || !state.detail.shown
                || state.detail.focus == DetailFocus::Diff
        })
        .collect()
}

/// The half of [`states_for`] that is one arm per context — kept apart from
/// the width and the pane, which every arm shares.
///
/// **A list, not a state** (m5/42). Most contexts have one screen behind
/// them, but a context can be the bar for several: the detail pane with an
/// overlay behind it draws one bar over three different overlays, and the
/// three do not swallow the same keys — the Map view claims `p`, `d` and
/// `s`, which the stash list answers. A sweep that picked one of them would
/// be as blind to the other two as the whole sweep was to this state.
fn screen_for(context: &HintContext) -> Vec<AppState> {
    let walk = JobId::new(701);
    match context {
        // The cursor on the uncommitted-changes row. With a pane open it is
        // open on a *commit*: a pane open on the working tree is the
        // changes pane, which is the next context along.
        HintContext::WorkingTreeRow => {
            let mut state = dirty_on_screen(walk);
            state.on_working_tree = true;
            state.history.as_mut().expect("a walk").cursor = MERGE_ROW;
            vec![state]
        }
        // One row of history under the cursor. The badge and the Mistake
        // Radar marker that tell `BranchRow`, `Marked` and `PlainRow` apart
        // are painted on the row, not held in the state, and no key turns
        // on them.
        HintContext::BranchRow { .. }
        | HintContext::PlainRow
        | HintContext::Marked(_)
        | HintContext::Preview => vec![merge_on_screen(walk, MERGE_ROW).0],
        // The resolver open on a two-block conflict: every live key in
        // `resolver_list` has something to do in it.
        HintContext::Resolver {
            whole_file: false,
            roles,
            ..
        } => vec![in_roles(resolver_on_screen(walk), *roles)],
        HintContext::Resolver {
            whole_file: true,
            roles,
            ..
        } => vec![in_roles(resolver_card_on_screen(walk), *roles)],
        // The same row, with the pane open on it and its diff focused —
        // where `⏎` has nothing left to open.
        HintContext::PaneFocused { opens, .. } => vec![commit_pane(walk, *opens)],
        HintContext::Banner {
            skippable,
            conflicted,
        } => vec![stopped_operation(walk, *skippable, *conflicted)],
        HintContext::ChangesPane { unmerged } => vec![changes_pane_open(walk, *unmerged)],
        // The row's own screen with a merge standing and the banner handed
        // back (m6-conflicts/12): `⇥` leads, and every other key on the
        // row's bar must still answer with an operation live.
        HintContext::OperationLive(row) => screen_for(row)
            .into_iter()
            .map(|mut state| {
                state.in_progress = Some(graft_git::state::InProgressDetail {
                    kind: graft_git::state::InProgress::Merge,
                    step: None,
                    head_name: None,
                    onto: None,
                    theirs: Ok(None),
                });
                state.banner_focus = false;
                state
            })
            .collect(),
        HintContext::Journal => vec![overlay_over(
            merge_on_screen(walk, MERGE_ROW).0,
            Shown::Journal,
        )],
        HintContext::Stash => vec![overlay_over(
            merge_on_screen(walk, MERGE_ROW).0,
            Shown::Stash,
        )],
        HintContext::Map => vec![overlay_over(merge_on_screen(walk, MERGE_ROW).0, Shown::Map)],
        // **The state ticket 31's sweep could not reach** (m5/42): the pane
        // has the keyboard and an overlay, not the graph, stands behind it.
        // One per overlay, because the three swallow different keys.
        HintContext::PaneOverOverlay { opens } => EVERY_OVERLAY
            .into_iter()
            .map(|overlay| overlay_over(commit_pane(walk, *opens), overlay))
            .collect(),
        HintContext::Dragging => vec![badge_in_the_air(walk)],
        // A live box over the graph (rulings 07b, Q1): its bar is its own
        // keys, whatever stands behind it.
        HintContext::ConfirmBox | HintContext::NamePrompt { .. } | HintContext::Credential => {
            vec![box_over_the_graph(walk, context)]
        }
    }
}

/// [`screen_for`]'s live boxes, each over the graph with a merge under the
/// cursor.
fn box_over_the_graph(walk: JobId, context: &HintContext) -> AppState {
    match context {
        HintContext::ConfirmBox => {
            let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
            state.confirm = Some(super::confirm::PendingConfirm {
                view: graft_ui::confirm::ConfirmView::new(
                    graft_term::sanitize_line("Discard changes?"),
                    vec![graft_term::sanitize_line("3 files lose their edits.")],
                    graft_term::sanitize_line("git restore ."),
                ),
                then: super::confirm::ConfirmThen::Undo { redo: false },
            });
            state
        }
        HintContext::NamePrompt { toggle } => {
            let (mut state, at) = merge_on_screen(walk, MERGE_ROW);
            let kind = if *toggle {
                super::name_prompt::NameKind::Branch { at, checkout: true }
            } else {
                super::name_prompt::NameKind::Tag { at }
            };
            state.name_prompt = Some(super::name_prompt::NamePrompt {
                kind,
                field: graft_ui::text_input::TextField::new("feature"),
            });
            state
        }
        HintContext::Credential => {
            let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
            let job = JobId::new(702);
            state.operating = Some(job);
            state.credential = Some(super::credential::CredentialPrompt {
                title: graft_term::sanitize_line("Credentials — example.com"),
                prompt: graft_term::sanitize_line("Username for 'https://example.com':"),
                field: super::credential::PromptField::Open(graft_ui::text_input::TextField::new(
                    "me",
                )),
                job,
            });
            state
        }
        other => unreachable!("{other:?} is not a box"),
    }
}

/// The three overlays that stand where the graph stands and take its keys
/// (`AppState::graph_overlay`, minus the commit editor — which claims every
/// key before the panes and draws no hint bar, `view::rows::overlay_context`).
#[derive(Clone, Copy, Debug)]
enum Shown {
    Map,
    Journal,
    Stash,
}

/// Every overlay a bar can be drawn behind.
const EVERY_OVERLAY: [Shown; 3] = [Shown::Map, Shown::Journal, Shown::Stash];

/// `state` with `overlay` open over it, its rows loaded — so the keys that
/// move a selection have somewhere to move to and the sweep is not reading
/// an empty list as a dead key.
fn overlay_over(mut state: AppState, overlay: Shown) -> AppState {
    match overlay {
        Shown::Map => {
            state.map.shown = true;
            state.map.data = Loading::Ready(Arc::new(two_branch_map()));
        }
        Shown::Journal => {
            state.journal.shown = true;
            state.journal.entries = Loading::Ready(Arc::new(crate::jobs::undo::JournalRead {
                entries: vec![journal_entry()],
                unreadable: Vec::new(),
                live: Vec::new(),
            }));
        }
        Shown::Stash => {
            state.stash.shown = true;
            state.stash.entries = Loading::Ready(Arc::new(vec![stash_entry()]));
        }
    }
    state
}

/// Two branches in the Map view: one row to sit on and one to move to, so
/// `j/k move` has somewhere to go.
fn two_branch_map() -> crate::map::MapData {
    let branches = (0..2)
        .map(|i| crate::map::MapBranch {
            name: graft_core::ref_name::RefName::new(format!("refs/heads/b{i}").into_bytes())
                .expect("a valid ref name"),
            identity: graft_graph::identity::BranchIdentity::of_name(format!("b{i}").as_bytes()),
            tip: Oid::from_bytes(&[u8::try_from(i + 1).expect("a small row"); Oid::SHA1_BYTES])
                .expect("20 bytes is an oid"),
            tip_date: 0,
            counts: vec![0; 10],
            is_current: i == 0,
            is_default: i == 0,
            merged_into: None,
            marks: Vec::new(),
            complete: true,
        })
        .collect();
    crate::map::MapData {
        branches,
        more: 0,
        from: 0,
        to: 1,
        buckets: 10,
        complete: true,
    }
}

/// The detail pane open on the cursor's commit, with a file list and a long
/// diff in it, focused where `opens` says: `⏎` opens the diff from the
/// files list, and has nothing left to open once the diff has it
/// (`detail::keys::enter_opens`).
fn commit_pane(walk: JobId, opens: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    let spec = state.selected_spec().expect("the cursor is on a commit");
    let focus = if opens {
        DetailFocus::Files
    } else {
        DetailFocus::Diff
    };
    show_pane(&mut state, spec, focus);
    let (view, raw) = pane_files();
    state.detail.files = Loading::Ready(view);
    state.detail.list = Some(raw);
    let (diff, diff_raw) = long_diff("a.rs");
    state.detail.diff = Loading::Ready(diff);
    state.detail.diff_raw = Some(diff_raw);
    state
}

/// The row [`merge_on_screen`] puts a **merge** on, which is the row a
/// graph-row context's keys are swept against: `space fold` has a merge to
/// fold there, the way `H hide stash` has a stash to hide and `y sha` has a
/// commit to copy. A key that does nothing under one row's *data* — `space`
/// off a merge, `k` at the top — is a boundary, not a dead key, and this
/// sweep is not the place that would tell the difference.
const MERGE_ROW: usize = 1;

/// One stash in the refs, so `H hide stash` has something to hide: with
/// none, `ViewChange::Stashes` returns `None` by design (`update::keys`).
fn one_stash(state: &mut AppState) {
    let stash = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let name = graft_core::ref_name::RefName::new("refs/stash").expect("a valid ref name");
    let badges = crate::refs::badges(
        &[(name, stash)],
        &graft_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        &[(stash, 0)],
        |_, _| Err("no upstreams on this fixture".to_owned()),
    )
    .expect("badges");
    state.refs = Some(Arc::new(badges));
}

/// The pane open on `spec`, with a two-file list in it so the keys that act
/// on a file have one under the cursor.
fn show_pane(state: &mut AppState, spec: DetailSpec, focus: DetailFocus) {
    state.detail.shown = true;
    state.detail.spec = Some(spec);
    state.detail.focus = focus;
    let (files, list) = two_files();
    state.detail.files = Loading::Ready(files);
    state.detail.list = Some(list);
}

/// `DetailSpec` for the commit at row 0, for a state whose cursor is on the
/// working-tree row and so has no commit selected.
fn head_spec(state: &AppState) -> Option<DetailSpec> {
    let row = state
        .history
        .as_ref()
        .and_then(History::selected)?
        .commit
        .id;
    Some(DetailSpec::Commit(row))
}

/// An operation stopped with its banner focused: a rebase when `s skip`
/// applies, a merge when it does not, with a conflicted file when the
/// banner's `M`/`e` are drawn.
fn stopped_operation(walk: JobId, skippable: bool, conflicted: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, 0);
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: if skippable {
            graft_git::state::InProgress::Rebase
        } else {
            graft_git::state::InProgress::Merge
        },
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    state.banner_focus = true;
    if conflicted {
        state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
            entries: vec![status_entry("conflicted.rs", true)],
        }));
    }
    state
}

/// The changes pane open and focused on its file list, over a working tree
/// whose files are the ones the list draws — `unmerged` makes the file
/// under the cursor a conflicted one, which is what swaps the pane's write
/// keys for the conflict keys.
fn changes_pane_open(walk: JobId, unmerged: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, 0);
    state.on_working_tree = true;
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("a.rs", unmerged),
            status_entry("b.rs", unmerged),
        ],
    }));
    show_pane(&mut state, DetailSpec::WorkingTree, DetailFocus::Files);
    let (view, raw) = changes_pane_files();
    state.detail.files = Loading::Ready(view);
    state.detail.list = Some(raw);
    state
}

/// The changes pane's file list: an `UNSTAGED` heading and the two files
/// under it, the cursor on the first. `space stage` reads the **section**
/// the cursor's row belongs to (`operate::stage_selected`), so a flat list
/// with no heading is a pane whose write keys cannot fire.
fn changes_pane_files() -> (DetailArc<FileListView>, DetailArc<FileList>) {
    let (view, list) = two_files();
    let mut view = (*view).clone();
    view.rows.insert(
        0,
        FileRow {
            depth: 0,
            name: sanitize_line("UNSTAGED"),
            kind: FileRowKind::Section {
                section: graft_ui::detail::WorkingTreeSection::Unstaged,
                count: 2,
            },
            entry: None,
        },
    );
    for row in view.rows.iter_mut().skip(1) {
        row.depth = 1;
    }
    view.cursor = 1;
    (DetailArc::new(view), list)
}

/// A commit's file list with a directory row under the cursor, which is
/// what `space` folds while the files region has focus
/// (`detail::keys::files_key`).
fn pane_files() -> (DetailArc<FileListView>, DetailArc<FileList>) {
    let (view, list) = two_files();
    let mut view = (*view).clone();
    view.rows.insert(
        0,
        FileRow {
            depth: 0,
            name: sanitize_line("src"),
            kind: FileRowKind::Dir { collapsed: false },
            entry: None,
        },
    );
    for row in view.rows.iter_mut().skip(1) {
        row.depth = 1;
    }
    view.cursor = 0;
    (DetailArc::new(view), list)
}

/// A diff long enough to scroll: `j`/`k` in the diff region clamp to the
/// content they have, so a pane with nothing loaded answers them with
/// nothing, which is a fixture that cannot see what it is looking for.
fn long_diff(path: &str) -> (DetailArc<DiffView>, DetailArc<graft_git::diff::FileDiff>) {
    use graft_ui::detail::{HunkView, LineKindView, LineView};

    let (view, raw) = diff_of(path);
    let mut view = (*view).clone();
    let line = |number: u32| LineView {
        kind: LineKindView::Context,
        old_no: Some(number),
        new_no: Some(number),
        text: sanitize_line("a line of context"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    };
    let mut lines = vec![LineView {
        kind: LineKindView::HunkHeading,
        old_no: None,
        new_no: None,
        text: sanitize_line("@@ -1,200 +1,200 @@"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }];
    lines.extend((1..=200).map(line));
    view.hunks = vec![HunkView {
        heading: sanitize_line("-1,200 +1,200"),
        lines,
    }];
    (DetailArc::new(view), raw)
}

/// One `git status` entry for `path`, modified in the tree, conflicted when
/// `unmerged`.
fn status_entry(path: &str, unmerged: bool) -> graft_git::status::StatusEntry {
    graft_git::status::StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: Some(graft_git::status::Change::Modified),
        untracked: false,
        unmerged,
        unmerged_how: None,
        submodule: false,
    }
}

/// The resolver open on a real two-block conflict, the state every one of
/// `resolver_list`'s live keys has to act in: a cursor on the first block so
/// `o`/`t`/`b` write, a second open block so `n` moves, a mergetool
/// configured so `M` hands over instead of refusing, and a working tree that
/// lists the file as unmerged so the title and `⏎` have a file to name.
pub(super) fn resolver_on_screen(walk: JobId) -> AppState {
    const CONFLICTED: &[u8] = b"one\n\
        <<<<<<< HEAD\nours a\n=======\ntheirs a\n>>>>>>> topic\n\
        two\n\
        <<<<<<< HEAD\nours b\n=======\ntheirs b\n>>>>>>> topic\n\
        three\n";
    let file = graft_git::conflict::parse(CONFLICTED).expect("two open blocks");
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![status_entry("conflicted.rs", true)],
    }));
    state.mergetool = Some("vimdiff".to_string());
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("conflicted.rs"),
        opened_from: resolver::OpenedFrom::Pane,
        file: Some(Arc::new(file)),
        whole_file: None,
        how: None,
        cursor_block: Some(0),
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
    });
    state
}

/// The resolver open on a whole-file card (`deleted by them`), the state
/// every key a card's bar draws has to act in (m6-05 review, Major 1): a
/// second conflicted file after it so `n` has somewhere to cross to, and a
/// mergetool so `M` hands over.
pub(super) fn resolver_card_on_screen(walk: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("gone.txt", true),
            status_entry("later.rs", true),
        ],
    }));
    state.mergetool = Some("vimdiff".to_string());
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("gone.txt"),
        opened_from: resolver::OpenedFrom::Pane,
        file: None,
        whole_file: Some(resolver::WholeFileShape {
            how: graft_git::status::UnmergedHow::DeletedByThem,
            binary: false,
            on_disk: Some(crate::jobs::conflict::OnDisk {
                size: 11,
                head: Arc::from(vec![b"kept by us".to_vec()]),
                cut: false,
            }),
        }),
        how: Some(graft_git::status::UnmergedHow::DeletedByThem),
        cursor_block: None,
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
    });
    state
}

/// A branch badge picked up from row 1 and held over row 0, which is where
/// the gesture the bar is hinting actually is.
fn badge_in_the_air(walk: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(walk, 1);
    let at = state
        .history
        .as_ref()
        .and_then(History::selected)
        .expect("the cursor's row has arrived")
        .commit
        .id;
    state.drag = Some(crate::update::drag::DragState {
        source: crate::update::drag::DragSource {
            name: "refs/heads/feature".to_owned(),
            short: "feature".to_owned(),
            at,
            current: false,
        },
        from_row: 1,
        over: Some(0),
    });
    state
}

/// One journal entry, so `⏎ revert` has something to undo.
fn journal_entry() -> graft_core::journal::JournalEntry {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };

    JournalEntry {
        id: EntryId::new(1, [0; 10]),
        at_unix_ms: 1,
        kind: OperationKind::Commit,
        description: "commit \"x\"".to_owned(),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: Vec::new(),
            index: true,
            worktree: false,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        backup_ref: Some("refs/graft/backup/main/x".to_owned()),
        pins: Vec::new(),
        commands: vec!["git commit".to_owned()],
        reverses: None,
    }
}

/// One stash, so the overlay's keys have an entry under the cursor.
fn stash_entry() -> graft_git::stash::StashEntry {
    graft_git::stash::StashEntry {
        index: 0,
        commit: Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
        message: "WIP on main".to_owned(),
        at: Timestamp {
            seconds: 1_700_000_000,
            offset_seconds: 0,
        },
        paths: vec![PathBuf::from("a.rs")],
    }
}

/// The other half: a hint the table still calls planned must really be
/// unbound. Binding `m` to merge without flipping its hint to live fails
/// here — which is how the footer learns about a verb the moment it exists
/// (`hint-bar-redesign` scope item 6; m5-03 review, Major 6).
///
/// `bound_actions` says nothing about *where* a binding is live, so a verb
/// another context already hints as live is out of this check's reach: the
/// banner's `e edit` (edit the conflicted file) is planned while the changes
/// pane's `e edit` (open `$EDITOR` at the cursor) is bound, and the pair
/// alone cannot tell them apart. Every planned verb nothing hints as live —
/// `b branch`, `t tag`, `M mergetool`, `space resolved`, the banner's
/// `c continue`/`s skip`/`a abort` — is checked. `m merge` and `r rebase`
/// were on that list until m5/24: group 08 bound them and the hint tables
/// call them live, so they are in `bound_actions` now and this test's Live
/// half is what holds them.
#[test]
fn every_planned_hint_is_still_unbound() {
    use graft_ui::graph_screen::{HintState, every_hint};

    let bound = super::keys::bound_actions();
    let live: Vec<(&str, &str)> = every_hint()
        .iter()
        .filter(|hint| hint.state == HintState::Live)
        .map(|hint| (hint.key, hint.verb))
        .collect();
    for hint in every_hint() {
        let pair = (hint.key, hint.verb);
        match hint.state {
            HintState::Planned => {
                if live.contains(&pair) {
                    continue;
                }
                assert!(
                    !bound.contains(&pair),
                    "`{} {}` is bound now: flip it to `hint(..)` in graph_screen::hints",
                    hint.key,
                    hint.verb
                );
            }
            HintState::Live => assert!(
                bound.contains(&pair),
                "`{} {}` is hinted as live but nothing binds it",
                hint.key,
                hint.verb
            ),
        }
    }
}

/// Bound `(key, verb)` pairs the hint bar draws **nowhere**, each with the
/// reason it is not a defect — the named exemption list
/// [`every_bound_action_is_hinted_live_or_is_named_here`] walks against
/// (m5/33, review M4).
///
/// Five of the six are `docs/design/02-keymap.md` §2.2 globals: keys live in
/// every pane, which the bar does not list because the bar lists the *focused
/// context's* keys. The sixth is not a decided exemption and says so.
const NEVER_HINTED: &[(&str, &str, &str)] = &[
    (
        "g/G",
        "top/bottom",
        "keymap §2.2: global movement, live in every pane",
    ),
    (
        "PgUp/PgDn",
        "page",
        "keymap §2.2: global movement, live in every pane",
    ),
    ("⌃R", "refresh", "keymap §2.2: global"),
    (
        "U",
        "redo",
        "keymap §2.2: global, and §2.1's Shift pair of the `u undo` the bar does draw",
    ),
    (
        "esc",
        "dismiss",
        "keymap §2.2: `Esc` backs out everywhere. The bar draws it where it is the \
         context's own key (`esc close`, `esc cancel`); clearing a stale status line \
         is not a context action",
    ),
    (
        "a",
        "amend",
        "**open question, not a decision** (m5/33): keymap §2.3 binds `a` on the graph \
         pane and §2.1 says the bar always shows what is active, but no hint table has \
         ever listed it — the bar draws `c commit` alone. Filed in ticket 33; here so \
         the walk below is honest rather than blocked",
    ),
];

/// **The converse direction** (m5/33, review M4).
/// [`every_planned_hint_is_still_unbound`] walks the hint tables and asks
/// whether each entry is bound; it cannot see a key that is bound and is in
/// **no table at all**, and [`super::keys::bound_actions`] is hand-written,
/// so that gap is real rather than theoretical.
///
/// This walks the other way: every `(key, verb)` the app binds is drawn
/// `HintState::Live` in some context, or is named in [`NEVER_HINTED`] with
/// its reason. The exemption list is walked too — a row that stopped being
/// bound, or that is drawn now, fails here rather than sitting stale.
#[test]
fn every_bound_action_is_hinted_live_or_is_named_here() {
    use graft_ui::graph_screen::{HintState, every_hint};

    let bound = super::keys::bound_actions();
    // `every_hint` is the union of every context's own list plus the tail
    // and the doorway hints `whole_bar` adds, so "live in here" is "live on
    // some bar".
    let live: Vec<(&str, &str)> = every_hint()
        .iter()
        .filter(|hint| hint.state == HintState::Live)
        .map(|hint| (hint.key, hint.verb))
        .collect();
    let exempt: Vec<(&str, &str)> = NEVER_HINTED.iter().map(|(k, v, _)| (*k, *v)).collect();

    let unhinted: Vec<String> = bound
        .iter()
        .filter(|pair| !live.contains(pair) && !exempt.contains(pair))
        .map(|(key, verb)| format!("`{key} {verb}`"))
        .collect();
    assert!(
        unhinted.is_empty(),
        "bound, and drawn on no bar in any context: {}. Either hint it, or add it to \
         NEVER_HINTED with the reason it is never drawn",
        unhinted.join(", ")
    );

    for (key, verb, why) in NEVER_HINTED {
        assert!(
            bound.contains(&(*key, *verb)),
            "`{key} {verb}` is on the never-hinted list ({why}) but nothing binds it \
             any more — take the row off"
        );
        assert!(
            !live.contains(&(*key, *verb)),
            "`{key} {verb}` is drawn live now, so the never-hinted list is wrong \
             about it ({why}) — take the row off"
        );
    }
}

// The vertical axis of ticket 19's principle: count from what you were
// given, not from the screen (review M1).

/// **The class test for the vertical axis.** `update::geometry` exists to
/// mirror the view for hit-testing, and its `Tab` arm did not know the
/// operation banner takes a row above the strip (and a second off the
/// bottom while focused). So below 100 columns with a merge stopped, every
/// click landed one row above the row the user saw, and two while the
/// banner had the keyboard.
///
/// Swept, not sampled, and against the rects the **drawing path** computes
/// (`view::frame::detail_rects` is what `split_for_detail` splits from),
/// not against a second copy of the arithmetic — a mirror checked against
/// a copy of itself agrees by construction. Widths through both
/// breakpoints; heights down to one where the banner's two rows are most
/// of the frame; both tabs; focused and not; banner live and not.
/// The third reader of the tab layout's row budget: `AppState::graph_rows`,
/// which is how many history rows `j`/`k`, `PgDn` and `G` believe are on
/// screen. The `[Graph] Detail` strip is a row the graph never gets, and
/// neither was a term here before review M1 - so the cursor could be moved
/// onto a row the graph had not drawn.
///
/// The strip is all it costs *on the graph tab*: the banner's word rides the
/// graph header there (see `banner::tab_banner_rows`), which is the other
/// half of M2. The number itself is checked against the drawn rects by
/// `the_geometry_mirror_matches_the_rects_the_frame_hands_out`; this test is
/// about who spends it.
#[test]
fn the_tab_strip_comes_out_of_the_graphs_row_budget() {
    use graft_ui::detail::DetailFocus;

    let mut tabbed = AppState {
        size: crate::update::state::Size {
            columns: 80,
            rows: 24,
        },
        ..AppState::default()
    };
    tabbed.detail.shown = true;
    tabbed.detail.focus = DetailFocus::Graph;
    let mut wide = tabbed.clone();
    wide.size.columns = 160;

    assert_eq!(
        crate::update::banner::tab_rows_above_graph(&wide),
        0,
        "beside the graph there is no strip and no tab banner"
    );
    assert_eq!(
        crate::update::banner::tab_rows_above_graph(&tabbed),
        1,
        "the `[Graph] Detail` strip"
    );
    assert_eq!(
        tabbed.graph_rows() + 1,
        wide.graph_rows(),
        "the strip is a row the graph does not get"
    );

    // A live operation, banner focused, graph tab still the one shown: the
    // strip is still the whole cost, because the graph header carries the
    // kind word and its keys ride the footer.
    let mut live = tabbed.clone();
    live.banner_focus = true;
    live.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    assert_eq!(
        live.graph_rows(),
        tabbed.graph_rows(),
        "the banner costs the graph tab nothing: it is drawn in the header"
    );
}

/// **The regression this ticket most likely causes.** Below 100 columns
/// with the pane open on the **Graph** tab, a click must still land on the
/// row the user is looking at: the `[Graph] Detail` strip pushes the rows
/// down by one, and reserving the pane's hint bar
/// (`update::detail::tab_hint_rows`, m5-history-and-ghost-preview/29) must
/// not move them again — the bar is the Detail tab's row, not the graph's,
/// so `rows_above_graph` is unchanged and the click keeps landing where it
/// did (ticket 25's fix of exactly this shape, review M1).
///
/// **Mutation:** add one to `rows_above_graph`'s return and this goes red
/// at the assert below — which is what makes it a test of the offset and
/// not just of the click path.
#[test]
fn a_click_below_one_hundred_columns_still_lands_on_the_row_it_named() {
    use graft_ui::detail::DetailFocus;

    let id = JobId::new(20);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 12,
    };
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Graph;
    assert!(
        matches!(
            crate::update::detail::effective_layout(
                state.detail.focus,
                &state.detail.pane_size,
                (state.size.columns, state.size.rows),
            ),
            graft_ui::detail_pane::DetailLayout::Tab { shown: false }
        ),
        "80 columns with the graph tab up is the layout this is about"
    );
    assert_eq!(
        crate::update::detail::tab_hint_rows(&state),
        0,
        "the Graph tab keeps the graph's own footer; the pane reserves nothing"
    );

    // The strip's row, then the first graph row: the second commit sits one
    // row below that.
    let strip = 1;
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + strip + 1,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);

    assert_eq!(
        next.history.as_ref().map(|history| history.cursor),
        Some(1),
        "the row the pointer was on"
    );
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id)
    );
}

/// One size, one focus, one banner: the drawn rects and the mirror's.
fn the_mirror_agrees_at(
    width: u16,
    height: u16,
    live: bool,
    focus: graft_ui::detail::DetailFocus,
    banner_focus: bool,
) {
    use ratatui::layout::Rect;

    let bounds = |rect: Rect| geometry::Bounds {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    };
    let mut state = AppState {
        size: crate::update::state::Size {
            columns: width,
            rows: height,
        },
        banner_focus,
        ..AppState::default()
    };
    state.detail.shown = true;
    state.detail.focus = focus;
    if live {
        state.in_progress = Some(graft_git::state::InProgressDetail {
            kind: graft_git::state::InProgress::Merge,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        });
    }
    let layout =
        crate::update::detail::effective_layout(focus, &state.detail.pane_size, (width, height));
    let area = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let (graph, pane) = crate::view::frame::detail_rects(&state, layout, area);
    let mirrored = geometry::detail_split(
        layout,
        (width, height),
        crate::update::banner::tab_banner_rows(&state),
        crate::update::detail::tab_hint_rows(&state),
    );
    let at = format!(
        "{width}x{height}, live={live}, focus={focus:?}, \
         banner_focus={banner_focus}, layout={layout:?}"
    );
    // The pane rect exactly: it is what `detail_hit` tests a click against.
    assert_eq!(mirrored.1, pane.map(bounds), "pane rect at {at}");
    // The graph rect on the **vertical** axis exactly. Its width is
    // deliberately one wider here in the `Right` layout: m5/19 routes the
    // gutter column to the region it came out of rather than to nothing, so
    // the mirror keeps a cell the drawing path does not paint
    // (`detail_pane/layout.rs`'s `gutter`). That is the horizontal axis and
    // has its own test (`graft-ui`'s `pane_boundaries.rs`).
    assert_eq!(
        mirrored.0.map(|rect| (rect.y, rect.height)),
        graph.map(|rect| (rect.y, rect.height)),
        "graph rows at {at}"
    );
    assert_eq!(
        mirrored.0.is_some(),
        graph.is_some(),
        "graph drawn-or-not at {at}"
    );
}

#[test]
fn the_geometry_mirror_matches_the_rects_the_frame_hands_out() {
    use graft_ui::detail::DetailFocus;

    for width in 20u16..=240 {
        for height in [3u16, 8, 24, 40, 48] {
            for live in [false, true] {
                for focus in [DetailFocus::Graph, DetailFocus::Files, DetailFocus::Diff] {
                    for banner_focus in [false, true] {
                        the_mirror_agrees_at(width, height, live, focus, banner_focus);
                    }
                }
            }
        }
    }
}

/// `state` in the operation whose sides go by `roles`: stash pop is the
/// resolver open with no operation at all (`update::resolver::side_names`).
fn in_roles(mut state: AppState, roles: graft_ui::conflict_resolver::SideRoles) -> AppState {
    use graft_git::state::InProgress;
    use graft_ui::conflict_resolver::SideRoles;
    let kind = match roles {
        SideRoles::Merge => Some(InProgress::Merge),
        SideRoles::Rebase => Some(InProgress::Rebase),
        SideRoles::CherryPick => Some(InProgress::CherryPick),
        SideRoles::Revert => Some(InProgress::Revert),
        SideRoles::StashPop => None,
    };
    state.in_progress = kind.map(|kind| crate::update::resolver::fixtures::operation(kind, None));
    state
}
