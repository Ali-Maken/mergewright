//! An untake that did not write, and the keys refused while one is out
//! (review ease-pass-05 M3 failure and minor 1; ticket ease-pass/05r).

use std::path::Path;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode};
use graft_core::job::{JobFailure, JobId};

use super::advance::{landed, on, sent, three};
use super::{press, sticky};
use crate::jobs::conflict::UntakeFailure;
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

const FILE: &str = "three.txt";

/// The untake ids in `commands`.
fn untakes(commands: &[Command]) -> Vec<JobId> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::UntakeSide(id, _) => Some(*id),
            _ => None,
        })
        .collect()
}

/// `three.txt` with one pick landed (`o` on block 0), and `u`'s untake out.
fn undoing() -> (AppState, JobId) {
    // `z.txt` below it, so `j` has somewhere to go.
    let paths = [FILE, "z.txt"];
    let (taking, commands) = press(&on(FILE, three(), &paths, &paths), KeyCode::Char('o'));
    let (picked, _) = landed(&taking, &sent(&commands)[0]);
    assert_eq!(picked.picks.in_file(Path::new(FILE)), 1, "one pick");
    let (undoing, commands) = press(&picked, KeyCode::Char('u'));
    let ids = untakes(&commands);
    assert_eq!(ids.len(), 1, "one untake out: {commands:?}");
    assert_eq!(undoing.picks.undoing(), Some(ids[0]));
    assert!(undoing.picks.is_empty(), "the pick is off the stack");
    (undoing, ids[0])
}

fn answered(state: &AppState, id: JobId, result: Result<(), UntakeFailure>) -> AppState {
    let result = result.map(|()| {
        let mut file = super::advance::three();
        // What the untake leaves: block 0 open again — the fresh document.
        file.fingerprint = graft_git::conflict::blob_id(&file.render());
        Arc::new(file)
    });
    update(state, AppEvent::Produced(id, Product::Untaken(result))).0
}

/// `UntakeFailure::Failed`: nothing written, the pick still applies — back
/// on top, the untake no longer out, the error said on a line that stays
/// (Sticky, as the stale words are), and the next `u` tries it again.
#[test]
fn a_failed_untake_puts_its_pick_back_and_says_why() {
    let (state, id) = undoing();
    let after = answered(&state, id, Err(UntakeFailure::Failed("disk full".into())));
    assert_eq!(after.picks.undoing(), None);
    assert_eq!(after.picks.in_file(Path::new(FILE)), 1, "back on top");
    assert_eq!(
        sticky(&after),
        ("could not undo the pick in three.txt: disk full", None)
    );
    let (_, commands) = press(&after, KeyCode::Char('u'));
    assert_eq!(untakes(&commands).len(), 1, "the next `u` retries it");
}

/// A job that broke or was cancelled: the untake will never answer, so its
/// pick goes back on top and nothing waits on it.
#[test]
fn an_untake_that_never_answers_puts_its_pick_back() {
    let ends: [fn(JobId) -> Event; 2] = [Event::JobCancelled, |id| {
        Event::JobFailed(id, JobFailure::Failed("panicked".into()))
    }];
    for event in ends {
        let (state, id) = undoing();
        let (after, _) = update(&state, AppEvent::Core(event(id)));
        assert_eq!(after.picks.undoing(), None);
        assert_eq!(after.picks.in_file(Path::new(FILE)), 1, "back on top");
        let (_, commands) = press(&after, KeyCode::Char('u'));
        assert_eq!(untakes(&commands).len(), 1, "the next `u` tries it");
    }
}

/// The untake wrote: the pick is gone for good.
#[test]
fn a_written_untake_leaves_its_pick_off_the_stack() {
    let (state, id) = undoing();
    let after = answered(&state, id, Ok(()));
    assert_eq!(after.picks.undoing(), None);
    assert!(after.picks.is_empty(), "not back on top");
    assert!(
        matches!(after.status, Status::Done { .. }),
        "{:?}",
        after.status
    );
}

/// Minor 1: while the untake runs, a key that would write, leave the file
/// or close the resolver says the untake is what it waits on — not `still
/// writing the last take`, which no take is.
#[test]
fn keys_refused_while_an_untake_runs_say_so() {
    let (state, _) = undoing();
    for code in [KeyCode::Char('o'), KeyCode::Char('j'), KeyCode::Escape] {
        let (after, commands) = press(&state, code);
        assert_eq!(
            sticky(&after),
            (
                "still undoing the last pick \u{b7} try again in a moment",
                None
            ),
            "{code:?}"
        );
        assert!(sent(&commands).is_empty(), "{code:?}: nothing sent");
        assert!(after.resolver.is_some(), "{code:?}: still open");
    }
}
