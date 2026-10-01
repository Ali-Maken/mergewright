//! The promotion as the screen takes it (m7-power-tools/12 Part C): the
//! order handed to the walk is applied when the first window in it lands,
//! not when `sorted` sends the request.

use graft_core::job::JobId;
use graft_git::history::Order;
use graft_ui::graph_screen::OrderNotice;

use crate::history::{HistoryUpdate, Promotion, Window};
use crate::update::tests::{rows_for, sorted_order, sorting};
use crate::update::{AppEvent, Command, Product, Progress, update};

/// The walk's answer to `request` over `rows`, in the order it names.
fn answered(walk: JobId, request: u64, promoted: bool) -> AppEvent {
    let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) =
        rows_for(walk, request, 0..3, 40)
    else {
        unreachable!("`rows_for` answers with a window");
    };
    AppEvent::Progress(
        walk,
        Progress::History(HistoryUpdate::Window(Window { promoted, ..window })),
    )
}

/// While the promoted layout catches up the walk answers from the
/// provisional one: those rows land as rows, and the header still says the
/// order is being sorted. The first promoted window clears it.
#[test]
fn a_provisional_window_after_sorted_keeps_the_order_notice() {
    let (walk, sort) = (JobId::new(1), JobId::new(2));
    let (state, commands) = update(
        &sorting(walk, sort),
        AppEvent::Produced(sort, Product::TopologicalOrder(sorted_order())),
    );
    let [Command::ShowRows(to, request)] = commands.as_slice() else {
        panic!("the order goes to the walk: {commands:?}");
    };
    assert_eq!(*to, walk);
    assert!(request.promote.is_some());

    let (state, _) = update(&state, answered(walk, request.id, false));
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(
        history.window.as_ref().map(|window| window.rows.len()),
        Some(3)
    );
    assert!(matches!(
        history.promotion,
        Some(Promotion::Swapping { .. })
    ));
    assert_eq!(
        crate::view::order_notice(history),
        Some(OrderNotice::Sorting)
    );
    let facts = history.walk.as_ref().expect("facts");
    assert_eq!(facts.order, Order::CommitDate);

    let (state, _) = update(&state, answered(walk, request.id, true));
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.promotion, None);
    assert_eq!(crate::view::order_notice(history), None);
    let facts = history.walk.as_ref().expect("facts");
    assert_eq!(facts.order, Order::Topological);
}
