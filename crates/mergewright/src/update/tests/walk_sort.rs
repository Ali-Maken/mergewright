//! `update` tests: walk sort. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

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
        AppEvent::Produced(old, Product::TopologicalOrder(sorted_order())),
    );
    assert!(commands.is_empty(), "{commands:?}");
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.promotion, Some(Promotion::Sorting));

    let (state, commands) = update(
        &state,
        AppEvent::Produced(new, Product::TopologicalOrder(sorted_order())),
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
        AppEvent::Produced(sort, Product::TopologicalOrder(sorted_order())),
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
