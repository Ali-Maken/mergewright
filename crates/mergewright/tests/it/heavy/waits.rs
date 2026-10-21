//! Bounded waits on the history job's updates (split out of `heavy`).

use super::*;

/// Reads updates until the [`Window`](history::Window) answering `request`
/// arrives, dropping progress reports (and any other request's window, were
/// one still in flight) along the way.
///
/// Not `window.done`: that means the *whole* walk has finished, not that
/// this request was answered — a request can be fully answered (every row
/// it asked for laid out) while other commits the walk hasn't reached yet
/// still leave `done` false.
///
/// Bounded by [`wait_for`]: [`DEADLINE`](crate::deadline::DEADLINE) for the
/// whole wait. The caller holds the request sender, so the stream does not
/// end on its own; a window that never comes fails here, naming the request.
pub(super) fn until_window_for(
    told: &mpsc::Receiver<HistoryUpdate>,
    request: u64,
) -> history::Window {
    wait_for(
        told,
        crate::deadline::DEADLINE,
        &format!("request {request}'s window"),
        |update| match update {
            HistoryUpdate::Window(window) if window.request == request => Some(window),
            HistoryUpdate::Window(_)
            | HistoryUpdate::Progress { .. }
            | HistoryUpdate::LaidOut { .. }
            | HistoryUpdate::FoldedHits { .. }
            | HistoryUpdate::Started(_) => None,
        },
    )
}

/// Reads updates until a [`Window`](history::Window) answering `request`
/// with `promoted == true` arrives. Since 12c Part C a promotion's first
/// answer is provisional; the promoted one comes from a later idle step.
///
/// Bounded by [`wait_for`] as [`until_window_for`] is: one
/// [`DEADLINE`](crate::deadline::DEADLINE) for the whole wait, so
/// unpromoted windows that keep coming still end it. The failure counts
/// every update passed over, unpromoted windows among them.
pub(super) fn until_promoted_window_for(
    told: &mpsc::Receiver<HistoryUpdate>,
    request: u64,
) -> history::Window {
    wait_for(
        told,
        crate::deadline::DEADLINE,
        &format!("request {request}'s promoted window"),
        |update| match update {
            HistoryUpdate::Window(window) if window.request == request && window.promoted => {
                Some(window)
            }
            HistoryUpdate::Window(_)
            | HistoryUpdate::Progress { .. }
            | HistoryUpdate::LaidOut { .. }
            | HistoryUpdate::FoldedHits { .. }
            | HistoryUpdate::Started(_) => None,
        },
    )
}

/// Waits on `told` until `pick` returns `Some`, at most `bound` in total.
/// Panics naming `what` and how many messages it passed over when the bound
/// runs out or the stream ends first.
#[allow(
    clippy::panic,
    reason = "test-only helper in an integration test binary; a wait that never ends must fail the test loudly"
)]
fn wait_for<T, R>(
    told: &mpsc::Receiver<T>,
    bound: Duration,
    what: &str,
    mut pick: impl FnMut(T) -> Option<R>,
) -> R {
    // One clock for the whole wait, not per message: messages that keep
    // coming without the answer must still end it.
    let deadline = Instant::now() + bound;
    let mut passed = 0_usize;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match told.recv_timeout(left) {
            Ok(message) => {
                if let Some(found) = pick(message) {
                    return found;
                }
                passed += 1;
            }
            Err(RecvTimeoutError::Timeout) => panic!(
                "waiting for {what}: nothing within {bound:?}; passed over {passed} message(s)"
            ),
            Err(RecvTimeoutError::Disconnected) => panic!(
                "waiting for {what}: the stream ended first; passed over {passed} message(s)"
            ),
        }
    }
}

/// 20n D (gap M7/12f-hang): a wait whose answer never comes, on a stream
/// that never ends (the sender is kept alive, as the heavy test keeps
/// `asks`), fails within its bound instead of hanging the tier.
#[test]
fn a_wait_with_no_answer_fails_instead_of_hanging() {
    let (send, told) = mpsc::channel::<u32>();
    for message in [1, 2, 3] {
        send.send(message).expect("the receiver is alive");
    }
    let (done, finished) = mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wait_for(&told, Duration::from_secs(1), "the answer 99", |message| {
                (message == 99).then_some(message)
            })
        }));
        let words = match outcome {
            Ok(found) => format!("found {found}"),
            Err(panic) => panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|words| (*words).to_owned())
                })
                .unwrap_or_else(|| "a panic with no words".to_owned()),
        };
        done.send(words).expect("the test is waiting");
    });
    let words = finished
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or_else(|why| panic!("hung: `wait_for` did not return within 30 s ({why})"));
    waiter
        .join()
        .expect("the waiter reports rather than panics");
    drop(send);
    assert!(
        words.contains("the answer 99") && words.contains("passed over 3"),
        "the failure names the wait and the messages passed over: {words}"
    );
}
