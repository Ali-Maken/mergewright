//! A commit carrying thousands of refs (m8-polish-and-ship/12c): the row's
//! badge work is bounded by the screen's width, it draws and hit-tests as
//! its first badges do, and the drawing order is the one `refs::badges` set.

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::HeadState;
use mergewright_ui::graph_screen::{Badge, badge_spans, badges_that_fit, row_at};

use super::{drawn_at, history_of, oid};
use crate::refs::RefBadges;
use crate::session::for_tests::session;
use crate::update::{AppState, Size};
use crate::view::entries::badge_view;
use crate::view::{badge_index_at, graph_view};

/// The newest commit of [`history_of`]`(5, _)`, which every ref here sits on.
const PILED: u8 = 5;

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

fn read(refs: &[(RefName, Oid)], head: &HeadState, stashes: &[(Oid, usize)]) -> RefBadges {
    crate::refs::badges(
        refs,
        head,
        &std::collections::HashMap::new(),
        stashes,
        |_, _| {
            Ok(mergewright_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("plain data always builds")
}

/// `count` local branches `team/<i>` on one commit, `team/0` checked out:
/// the perf fixture's pile in miniature.
fn pile(count: usize) -> RefBadges {
    let refs: Vec<(RefName, Oid)> = (0..count)
        .map(|i| (name(&format!("refs/heads/team/{i}")), oid(PILED)))
        .collect();
    read(&refs, &HeadState::Branch(name("refs/heads/team/0")), &[])
}

fn state_with(refs: RefBadges, columns: u16) -> AppState {
    let mut state = AppState {
        history: Some(history_of(PILED, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        size: Size { columns, rows: 12 },
        ..AppState::default()
    };
    // The whole width to the graph, so the row is as wide as the screen.
    state.detail.shown = false;
    state
}

/// The badges the frame's own builder made for the piled commit's row.
fn built(state: &AppState) -> Vec<Badge> {
    graph_view(state, state.harness.clock.now_seconds())
        .rows
        .into_iter()
        .find(|entry| entry.line.row.commit.id == oid(PILED))
        .expect("the piled commit has a row")
        .badges
}

/// The guard: one `badge_view` per badge built (`view::entries::row_entry`),
/// and everything after it — `badges_width`, `badge_spans`, the draw, the
/// hit-test — walks that list. At 120 columns a pile of 1 000 and one of
/// 16 000 cost the same; before 12c the row built all 16 000.
#[test]
fn row_badge_work_is_bounded_by_width() {
    let fit = badges_that_fit(120);
    assert!(
        fit < 1_000,
        "the bound is the width's, not the pile's: {fit}"
    );
    assert_eq!(built(&state_with(pile(10), 120)).len(), 10);
    assert_eq!(built(&state_with(pile(1_000), 120)).len(), fit);
    assert_eq!(built(&state_with(pile(16_000), 120)).len(), fit);
    // And the width is the screen's: a wider screen may build more.
    assert_eq!(
        built(&state_with(pile(16_000), 200)).len(),
        badges_that_fit(200)
    );
}

/// A click on the last badge a 16 000-ref row draws picks that badge — the
/// one at the same place in `RefBadges::on` — and a click just past it
/// picks none.
#[test]
fn many_refs_row_hit_test_matches_draw() {
    let state = state_with(pile(16_000), 120);
    let now = state.harness.clock.now_seconds();
    // Read off the drawn cells, not asked of `mergewright_ui`.
    let (first_x, y) = drawn_at(&session(), &state, 120, 12, "team/0");

    let view = graph_view(&state, now);
    let pane =
        crate::view::frame::primary_pane_area(&state, crate::view::frame::screen_area(&state))
            .expect("the graph has a pane");
    let (entry, columns) = row_at(&view, pane, y).expect("the piled row is drawn there");
    let spans = badge_spans(entry, &columns, view.icons, view.keys);
    let (last, index) = spans.last().cloned().expect("the row draws badges");
    assert!(spans.len() > 1, "more than one badge fits at 120 columns");

    // The frame really drew that badge there, read cell by cell.
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 12))
        .expect("TestBackend reports its size");
    terminal
        .draw(|frame| crate::view::view(&session(), &state, frame))
        .expect("TestBackend cannot fail to draw");
    let drawn: String = last
        .clone()
        .map(|x| terminal.backend().buffer()[(x, y)].symbol().to_owned())
        .collect();
    let crate::refs::RefLabel::Branch { name: on, .. } =
        &state.refs.as_deref().expect("refs on hand").on(oid(PILED))[index]
    else {
        panic!("every ref in the pile is a branch");
    };
    let on = String::from_utf8_lossy(on);
    // `[team/12]`, or `[team/1…` where the edge cut it.
    let shown = drawn
        .trim_start_matches('[')
        .trim_end_matches(['…', ']', ' ']);
    assert!(
        !shown.is_empty() && on.starts_with(shown),
        "cells {last:?} read {drawn:?}, but badge {index} is {on:?}"
    );
    assert!(first_x <= last.start);

    for column in last.clone() {
        assert_eq!(
            badge_index_at(&state, column, y, now),
            Some(index),
            "column {column} is inside the last drawn badge"
        );
    }
    assert_eq!(
        badge_index_at(&state, last.end, y, now),
        None,
        "column {} is past the last drawn badge",
        last.end
    );
}

/// A branch-and-HEAD row of every kind: `main`, `feature`, a remote, a tag,
/// then `tags` more tags, all on the piled commit.
fn mixed(tags: usize) -> Vec<(RefName, Oid)> {
    let mut refs = vec![
        (name("refs/heads/main"), oid(PILED)),
        (name("refs/heads/feature"), oid(PILED)),
        (name("refs/remotes/origin/feature"), oid(PILED)),
        (name("refs/tags/v1"), oid(PILED)),
    ];
    refs.extend((0..tags).map(|i| (name(&format!("refs/tags/t{i}")), oid(PILED))));
    refs
}

/// The piled row's badges as drawn text.
fn texts(state: &AppState) -> Vec<String> {
    built(state)
        .iter()
        .map(|badge| badge.text(state.icons, mergewright_ui::keymap::Keymap::shipped()))
        .collect()
}

/// HEAD, the current branch, a local branch, a remote, a tag and a stash on
/// one commit draw in `refs::badges`' order — and with 16 000 tags piled on
/// after them the row is that same order, cut at the width.
#[test]
fn badge_order_unchanged() {
    let stash = [(oid(PILED), 0)];
    let on_main = HeadState::Branch(name("refs/heads/main"));
    let detached = HeadState::Detached(oid(PILED));
    assert_eq!(
        texts(&state_with(read(&mixed(0), &on_main, &stash), 120)),
        [
            "[◆ main]",
            "[feature]",
            "[origin/feature]",
            "[tag: v1]",
            "[stash@{0}]"
        ]
    );
    assert_eq!(
        texts(&state_with(read(&mixed(0), &detached, &stash), 120)),
        [
            "[HEAD]",
            "[main]",
            "[feature]",
            "[origin/feature]",
            "[tag: v1]",
            "[stash@{0}]"
        ]
    );

    for head in [on_main, detached] {
        let state = state_with(read(&mixed(16_000), &head, &stash), 120);
        let all: Vec<Badge> = state
            .refs
            .as_deref()
            .expect("refs on hand")
            .on(oid(PILED))
            .iter()
            .map(badge_view)
            .collect();
        assert_eq!(built(&state), all[..badges_that_fit(120)]);
    }
}
