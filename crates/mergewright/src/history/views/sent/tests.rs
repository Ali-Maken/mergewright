//! Sending the last window again after a repair laid its rows out anew
//! (ease-pass 06d): the stale path of [`Views::refresh_sent`].

use std::sync::Arc;

use mergewright_fixtures::Shape;
use mergewright_git::history::{CommitTable, Order, Walk};
use mergewright_git::repo::RepoHandle;
use mergewright_graph::Seeds;
use mergewright_graph::layout::StreamConfig;

use crate::history::source::WalkSource;
use crate::history::state::{HistoryUpdate, RowRequest, View, Wanted, Window};
use crate::jobs::Delivery;

use super::super::Views;

const SMALL: StreamConfig = StreamConfig {
    chunk_rows: 8,
    checkpoint_every: 16,
    max_checkpoints: 4,
    resident_chunks: 2,
    buffer_rows: 4,
};

fn never() -> bool {
    false
}

fn views(handle: &RepoHandle) -> Views<'_, WalkSource<'_>> {
    let open = move |_: Option<&Arc<[mergewright_core::commit_handle::CommitHandle]>>| {
        let table = Arc::new(CommitTable::open(handle).expect("the table opens"));
        Ok(WalkSource::new(
            handle,
            Walk::all_refs(handle, Order::Topological, table)?,
        ))
    };
    Views::new(Seeds::default(), SMALL, Box::new(open))
}

/// Answers rows `0..4` of the full view; the window `views` then holds.
fn sent(views: &mut Views<'_, WalkSource<'_>>) -> Window {
    let request = RowRequest::new(7, Wanted::Rows(0..4), View::default());
    let mut window = None;
    views
        .answer(request, &mut || None, &never, &mut |update| {
            if let HistoryUpdate::Window(sent) = update {
                window = Some(sent);
            }
            Delivery::Received
        })
        .expect("answers");
    window.expect("a window was sent")
}

fn refreshed(views: &mut Views<'_, WalkSource<'_>>, invalidated: Option<usize>) -> Vec<Window> {
    let mut windows = Vec::new();
    let delivery = views
        .refresh_sent(invalidated, &mut |update| {
            if let HistoryUpdate::Window(window) = update {
                windows.push(window);
            }
            Delivery::Received
        })
        .expect("reads");
    assert_eq!(delivery, Delivery::Received);
    windows
}

#[test]
fn a_repair_inside_the_sent_window_sends_it_again_under_its_request() {
    let dir = mergewright_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let mut views = views(&handle);
    let first = sent(&mut views);

    let again = refreshed(&mut views, Some(2));

    assert_eq!(again.len(), 1, "sent again once");
    assert_eq!(again[0].request, 7, "under the same request id");
    assert_eq!(again[0].first, first.first);
    assert_eq!(again[0].rows, first.rows, "its rows read anew");
    // Sent: a second step with nothing new sends nothing.
    assert!(refreshed(&mut views, None).is_empty());
}

#[test]
fn a_repair_below_the_sent_window_sends_nothing() {
    let dir = mergewright_fixtures::borrow(Shape::Linear);
    let handle = RepoHandle::open(&dir).expect("opens");
    let mut views = views(&handle);
    let first = sent(&mut views);

    assert!(refreshed(&mut views, Some(first.end())).is_empty());
}
