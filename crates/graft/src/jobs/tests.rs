//! What the pool has to get right, driven with fake jobs.
//!
//! Nothing here sleeps to make a race come out the right way. Where a test
//! needs two threads to reach a point in a particular order, it says so with a
//! channel: a job that has started sends, and the test blocks until it has.
//! The only durations in this file are failure bounds — a `recv_timeout` that
//! turns a hang into a named assertion failure rather than a test suite that
//! never returns.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use super::*;

/// How long a test waits for a result before deciding the runner is broken.
/// Generous, because it is never reached when the code works.
const PATIENCE: Duration = Duration::from_secs(5);

/// This crate's own source tree: a real repository, and the one the tests
/// that dispatch a repository command read.
fn this_repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A runner and the events it reports, with no event loop in sight.
fn runner(workers: usize) -> (JobRunner<Sender<AppEvent>>, Receiver<AppEvent>) {
    let (runner, events, _store) = runner_at(&this_repository(), workers);
    (runner, events)
}

/// The same, over a named repository, handing back the store so a test can
/// ask how many times it was opened.
fn runner_at(
    path: &Path,
    workers: usize,
) -> (
    JobRunner<Sender<AppEvent>>,
    Receiver<AppEvent>,
    Arc<RepoStore>,
) {
    let (sender, receiver) = mpsc::channel();
    let store = Arc::new(RepoStore::new(path.to_path_buf(), None));
    let runner = JobRunner::start(sender, workers, Arc::clone(&store))
        .expect("a test machine can start a thread");
    (runner, receiver, store)
}

/// The next thing the runner reported, whatever kind it is.
fn next_report(events: &Receiver<AppEvent>) -> AppEvent {
    events
        .recv_timeout(PATIENCE)
        .expect("a job must always report what became of it")
}

/// The next report, which these tests expect to be a plain lifecycle event.
/// A job that brought a value back is a different shape and a test that wants
/// one says so by calling [`next_report`].
fn next_event(events: &Receiver<AppEvent>) -> Event {
    match next_report(events) {
        AppEvent::Core(event) => event,
        other => panic!("expected a lifecycle event, got {other:?}"),
    }
}

/// The announcement a dispatched command makes before its job can finish, and
/// the id it named. It is what lets the application know which job it is
/// waiting for while the job is still running.
fn next_started(events: &Receiver<AppEvent>, expected: &Task) -> JobId {
    match next_report(events) {
        AppEvent::Started(id, task) if &task == expected => id,
        other => panic!("expected {expected:?} to be announced, got {other:?}"),
    }
}

#[test]
fn a_finished_job_comes_back_as_an_event() {
    let (runner, events) = runner(1);

    let id = runner.submit(|_cancel| Outcome::Done);

    assert_eq!(next_event(&events), Event::JobFinished(id));
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn a_job_that_fails_says_why() {
    let (runner, events) = runner(1);

    let id = runner.submit(|_cancel| Outcome::Failed("no such repository".to_owned()));

    assert_eq!(
        next_event(&events),
        Event::JobFailed(id, JobFailure::Failed("no such repository".to_owned()))
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn every_id_is_its_own() {
    let (runner, _events) = runner(1);

    let first = runner.submit(|_cancel| Outcome::Done);
    let second = runner.submit(|_cancel| Outcome::Done);

    assert_ne!(first, second);
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn six_jobs_on_a_pool_of_two_all_finish() {
    let (runner, events) = runner(2);
    let ran = Arc::new(AtomicUsize::new(0));

    let mut submitted = Vec::new();
    for _ in 0..6 {
        let ran = Arc::clone(&ran);
        submitted.push(runner.submit(move |_cancel| {
            ran.fetch_add(1, Ordering::Relaxed);
            Outcome::Done
        }));
    }

    let mut finished: Vec<JobId> = (0..6)
        .map(|_| match next_event(&events) {
            Event::JobFinished(id) => id,
            other => panic!("expected a finished job, got {other:?}"),
        })
        .collect();
    finished.sort_unstable();

    assert_eq!(finished, submitted);
    assert_eq!(ran.load(Ordering::Relaxed), 6);
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// A pool of two is two jobs at once, not two jobs one after the other. Both
/// jobs report that they have started before either is allowed to finish, which
/// is only possible if they are on different threads.
#[test]
fn a_pool_of_two_runs_two_jobs_at_once() {
    let (runner, events) = runner(2);
    let (started, both_started) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let released = Arc::new(Mutex::new(released));

    for _ in 0..2 {
        let started = started.clone();
        let released = Arc::clone(&released);
        runner.submit(move |_cancel| {
            started.send(()).expect("the test is still listening");
            released
                .lock()
                .expect("no job panics while holding this")
                .recv()
                .expect("the test releases both jobs");
            Outcome::Done
        });
    }

    both_started
        .recv_timeout(PATIENCE)
        .expect("the first job must start");
    both_started
        .recv_timeout(PATIENCE)
        .expect("the second job must start while the first is still running");

    release.send(()).expect("a job is waiting");
    release.send(()).expect("a job is waiting");
    assert!(matches!(next_event(&events), Event::JobFinished(_)));
    assert!(matches!(next_event(&events), Event::JobFinished(_)));
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn a_running_job_stops_when_it_is_cancelled() {
    let (runner, events) = runner(1);
    let (started, has_started) = mpsc::channel();

    let id = runner.submit(move |cancel| {
        started.send(()).expect("the test is still listening");
        // A long job, standing in for a rev-walk: it checks its token between
        // steps. The deadline is a failure bound — if cancellation never
        // arrives the test fails with a message instead of hanging.
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            if cancel.is_cancelled() {
                return Outcome::Cancelled;
            }
            thread::yield_now();
        }
        Outcome::Failed("the job was never cancelled".to_owned())
    });

    has_started
        .recv_timeout(PATIENCE)
        .expect("the job must start");
    runner.cancel(id);

    assert_eq!(next_event(&events), Event::JobCancelled(id));
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// The cheapest cancellation there is: a job that was still queued is dropped
/// without ever running.
#[test]
fn a_job_cancelled_before_it_starts_never_runs() {
    let (runner, events) = runner(1);
    let (started, has_started) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let ran = Arc::new(AtomicBool::new(false));

    let blocker = runner.submit(move |_cancel| {
        started.send(()).expect("the test is still listening");
        released.recv().expect("the test releases this job");
        Outcome::Done
    });
    let queued = {
        let ran = Arc::clone(&ran);
        runner.submit(move |_cancel| {
            ran.store(true, Ordering::Release);
            Outcome::Done
        })
    };

    // The pool has one worker and it is inside the first job, so the second is
    // certainly still on the queue.
    has_started
        .recv_timeout(PATIENCE)
        .expect("the first job must start");
    runner.cancel(queued);
    release.send(()).expect("a job is waiting");

    assert_eq!(next_event(&events), Event::JobFinished(blocker));
    assert_eq!(next_event(&events), Event::JobCancelled(queued));
    assert!(
        !ran.load(Ordering::Acquire),
        "a cancelled job must not run at all"
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// A job that panics is a bug in one job. The user is told, and the pool keeps
/// working — the whole point of catching it at the worker boundary.
///
/// The panic message goes to stderr through whatever hook is installed, so this
/// test is noisy by design; `crash::install` is what makes it quiet in the real
/// application.
#[test]
fn a_panicking_job_is_reported_and_the_pool_survives() {
    let (runner, events) = runner(1);

    let panicking = runner.submit(|_cancel| panic!("lane 3 vanished"));
    let after = runner.submit(|_cancel| Outcome::Done);

    match next_event(&events) {
        Event::JobFailed(id, JobFailure::Panicked { message, report }) => {
            assert_eq!(id, panicking);
            assert_eq!(message, "lane 3 vanished");
            // No crash hook is installed in a test, so there is no report to
            // name — and saying "none" is not the same as saying nothing.
            assert_eq!(report, None);
        }
        other => panic!("expected a panic failure, got {other:?}"),
    }
    assert_eq!(next_event(&events), Event::JobFinished(after));
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn a_panic_that_is_not_a_string_is_still_reported() {
    let (runner, events) = runner(1);

    let id = runner.submit(|_cancel| std::panic::panic_any(7_u8));

    match next_event(&events) {
        Event::JobFailed(failed, JobFailure::Panicked { message, .. }) => {
            assert_eq!(failed, id);
            assert_eq!(message, "<non-string panic payload>");
        }
        other => panic!("expected a panic failure, got {other:?}"),
    }
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// Quitting must not wait for a job that will not stop. The job is cancelled,
/// the wait is bounded, and the runner says how many it gave up on rather than
/// hanging on to a terminal that has to go back to the user.
#[test]
fn shutdown_gives_up_on_a_job_that_ignores_its_token() {
    let (runner, events) = runner(1);
    let (started, has_started) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();

    let blocker = runner.submit(move |_cancel| {
        started.send(()).expect("the test is still listening");
        released.recv().expect("the test releases this job");
        Outcome::Done
    });
    let ran = Arc::new(AtomicBool::new(false));
    let queued = {
        let ran = Arc::clone(&ran);
        runner.submit(move |_cancel| {
            ran.store(true, Ordering::Release);
            Outcome::Done
        })
    };
    has_started
        .recv_timeout(PATIENCE)
        .expect("the first job must start");

    // A short grace: the job cannot possibly finish, because only this test can
    // release it.
    let shutdown = runner.shutdown(Duration::from_millis(20));

    assert_eq!(shutdown, Shutdown::Stalled { workers: 1 });
    // The abandoned worker is still a worker. Once released it ends its job and
    // says so — as a cancellation, because by then the application had asked it
    // to stop, whatever the work itself thought it returned. The job queued
    // behind it is dropped rather than run.
    release.send(()).expect("the job is waiting");
    assert_eq!(next_event(&events), Event::JobCancelled(blocker));
    assert_eq!(next_event(&events), Event::JobCancelled(queued));
    assert!(
        !ran.load(Ordering::Acquire),
        "shutdown must not start a job that was still queued"
    );
}

#[test]
fn shutting_down_an_idle_pool_is_clean() {
    let (runner, _events) = runner(2);

    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// A result whose screen has gone has nowhere to go, and that is a normal end
/// to a run rather than a failure to report.
#[test]
fn a_result_with_nobody_listening_is_not_an_error() {
    let (sender, receiver) = mpsc::channel::<AppEvent>();
    drop(receiver);

    let delivery = sender.deliver(AppEvent::Core(Event::JobFinished(JobId::new(1))));

    assert_eq!(delivery, Delivery::NobodyListening);
}

#[test]
fn a_pool_whose_events_go_nowhere_still_shuts_down() {
    let (sender, receiver) = mpsc::channel::<AppEvent>();
    let store = Arc::new(RepoStore::new(this_repository(), None));
    let runner = JobRunner::start(sender, 2, store).expect("a test machine can start a thread");
    drop(receiver);

    runner.submit(|_cancel| Outcome::Done);

    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn the_pool_is_never_empty_and_never_larger_than_the_cap() {
    let (empty, _events) = runner(0);
    let (huge, _events) = runner(99);

    assert_eq!(empty.worker_count(), 1);
    assert_eq!(huge.worker_count(), MAX_WORKERS);
    assert_eq!(empty.shutdown(PATIENCE), Shutdown::Clean);
    assert_eq!(huge.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn the_default_pool_fits_a_laptop() {
    let (sender, _events) = mpsc::channel::<AppEvent>();
    let store = Arc::new(RepoStore::new(this_repository(), None));
    let runner =
        JobRunner::with_default_size(sender, store).expect("a test machine can start a thread");

    assert!((1..=MAX_WORKERS).contains(&runner.worker_count()));
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// The command that proves the pipeline: a real repository opens and brings its
/// facts back, and a directory that is not one comes back as an answer — a
/// `Product` carrying the reason — rather than as a broken job.
#[test]
fn opening_a_repository_reports_what_it_found() {
    let (runner, events) = runner(1);

    runner.dispatch(Command::OpenRepository);

    next_started(&events, &Task::OpenRepository);
    match next_report(&events) {
        AppEvent::Produced(_, Product::Repository(Ok(facts))) => {
            assert!(!facts.bare, "this repository has a working tree");
        }
        other => panic!("expected an open repository, got {other:?}"),
    }
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn a_path_that_is_not_a_repository_comes_back_as_an_answer() {
    let not_a_repository = tempfile::tempdir().expect("a temporary directory");
    let (runner, events, _store) = runner_at(not_a_repository.path(), 1);

    runner.dispatch(Command::OpenRepository);

    next_started(&events, &Task::OpenRepository);
    match next_report(&events) {
        AppEvent::Produced(_, Product::Repository(Err(message))) => {
            assert!(!message.is_empty(), "a refusal must say why");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// The point of [`RepoStore`]: two jobs, one open. Discovering and opening a
/// repository is the expensive part, and M0 paid it once per job.
#[test]
fn two_jobs_share_one_open_repository() {
    let (runner, events, store) = runner_at(&this_repository(), 1);

    runner.dispatch(Command::OpenRepository);
    next_started(&events, &Task::OpenRepository);
    next_report(&events);

    runner.dispatch(Command::ReadHistory(Order::CommitDate));
    next_started(&events, &Task::ReadHistory);
    next_report(&events);

    assert_eq!(
        store.opens(),
        1,
        "the second job must reuse the handle the first one opened"
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// The other startup command. Whether git is there or not is an answer, and
/// either way the job succeeds and the application is told.
#[test]
fn checking_the_system_git_reports_either_way() {
    let (runner, events) = runner(1);

    runner.dispatch(Command::CheckSystemGit);

    next_started(&events, &Task::CheckSystemGit);
    match next_report(&events) {
        AppEvent::Produced(_, Product::SystemGit(check)) => match check {
            GitCheck::Ran(_) => {}
            GitCheck::Unavailable(why) => {
                assert!(!why.is_empty(), "a missing git must say why");
            }
        },
        other => panic!("expected a git check, got {other:?}"),
    }
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn a_cancel_command_reaches_the_job_it_names() {
    let (runner, events) = runner(1);
    let (started, has_started) = mpsc::channel();

    let id = runner.submit(move |cancel| {
        started.send(()).expect("the test is still listening");
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            if cancel.is_cancelled() {
                return Outcome::Cancelled;
            }
            thread::yield_now();
        }
        Outcome::Failed("the job was never cancelled".to_owned())
    });
    has_started
        .recv_timeout(PATIENCE)
        .expect("the job must start");

    runner.dispatch(Command::CancelJob(id));

    assert_eq!(next_event(&events), Event::JobCancelled(id));
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// Cancelling something that is already over is not an error and not an event:
/// its result is already on its way.
#[test]
fn cancelling_a_job_that_is_over_changes_nothing() {
    let (runner, events) = runner(1);

    let id = runner.submit(|_cancel| Outcome::Done);
    assert_eq!(next_event(&events), Event::JobFinished(id));

    runner.cancel(id);
    runner.cancel(JobId::new(9_999));

    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
    assert!(
        events.recv_timeout(Duration::from_millis(20)).is_err(),
        "a job that is over must not report twice"
    );
}

/// The history job says what the walk is, answers a request for rows with
/// those rows, and stays for the next request until it is cancelled — which
/// ends it once idle, giving the worker back.
#[test]
fn a_history_job_answers_row_requests_until_cancelled() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Linear);
    let (runner, events, _store) = runner_at(&dir, 1);

    runner.dispatch(Command::ReadHistory(Order::Topological));
    let id = next_started(&events, &Task::ReadHistory);
    let asked = history::RowRequest {
        id: 1,
        wanted: history::Wanted::Rows(190..usize::MAX),
        view: history::View::default(),
        promote: None,
    };
    runner.dispatch(Command::ShowRows(id, asked));

    let mut started = false;
    let window = loop {
        match next_report(&events) {
            AppEvent::Progress(from, Progress::History(update)) => {
                assert_eq!(from, id);
                match update {
                    history::HistoryUpdate::Started(_) => started = true,
                    history::HistoryUpdate::Progress { request, .. } => assert_eq!(request, 1),
                    history::HistoryUpdate::Window(window) => break window,
                }
            }
            other => panic!("the job is still answering, but it said {other:?}"),
        }
    };
    assert!(started, "the walk says what it is first");
    // The linear shape is 200 commits: rows 190.. are its last ten.
    assert_eq!(
        (window.request, window.first, window.rows.len()),
        (1, 190, 10)
    );
    assert_eq!((window.laid_out, window.done), (200, true));

    runner.dispatch(Command::CancelJob(id));
    assert_eq!(
        next_report(&events),
        AppEvent::Core(Event::JobCancelled(id))
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// m1-the-graph/54, closed by m2-details-and-diffs/21: a walk started after
/// `Ctrl+R` rebuilt the id table for a freshly-written commit-graph is on
/// the graph path at once — no more provisional order, no promotion job.
///
/// This is the end-to-end half of the ticket's "Update/loop test": the pure
/// reanchor mechanism (`history_started` setting `History::reanchor`) is
/// covered separately in `update::tests`, since driving real git through
/// `event_loop`'s scripted-`Wakes` harness has no precedent — that harness
/// only ever drives a fake `Dispatch`, never a real `JobRunner`.
#[test]
fn refresh_after_a_commit_graph_write_puts_the_next_walk_on_the_graph_path() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Linear);
    let (runner, events, _store) = runner_at(scratch.path(), 1);

    // Before: no commit-graph, so a topological order is provisional.
    runner.dispatch(Command::ReadHistory(Order::Topological));
    let id = next_started(&events, &Task::ReadHistory);
    let before = loop {
        match next_report(&events) {
            AppEvent::Progress(from, Progress::History(history::HistoryUpdate::Started(facts))) => {
                assert_eq!(from, id);
                break facts;
            }
            AppEvent::Progress(_, Progress::History(_)) => {}
            other => panic!("expected the walk to start, got {other:?}"),
        }
    };
    assert!(before.is_provisional(), "no commit-graph yet");
    runner.dispatch(Command::CancelJob(id));
    assert_eq!(
        next_report(&events),
        AppEvent::Core(Event::JobCancelled(id))
    );

    graft_fixtures::git::run(scratch.path(), &["commit-graph", "write", "--reachable"])
        .expect("git can write a commit-graph for this fixture");

    // `Ctrl+R`'s command: the synchronous refresh rebinds the table before
    // the next `ReadHistory` is dispatched, same as `keys::refresh_key`.
    runner.dispatch(Command::OpenRepository);
    next_started(&events, &Task::OpenRepository);
    next_report(&events);

    runner.dispatch(Command::ReadHistory(Order::Topological));
    let id = next_started(&events, &Task::ReadHistory);
    let after = loop {
        match next_report(&events) {
            AppEvent::Progress(from, Progress::History(history::HistoryUpdate::Started(facts))) => {
                assert_eq!(from, id);
                break facts;
            }
            AppEvent::Progress(_, Progress::History(_)) => {}
            other => panic!("expected the walk to start, got {other:?}"),
        }
    };
    assert!(
        !after.is_provisional(),
        "the freshly-written commit-graph is used at once, no promotion needed"
    );

    runner.dispatch(Command::CancelJob(id));
    assert_eq!(
        next_report(&events),
        AppEvent::Core(Event::JobCancelled(id))
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

#[test]
fn ahead_behind_is_counted_on_a_worker_never_the_loops_thread() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::RefStates);
    let (runner, events, _store) = runner_at(&dir, 1);

    runner.dispatch(Command::ReadRefs);

    next_started(&events, &Task::ReadRefs);
    match next_report(&events) {
        AppEvent::Produced(_, Product::Refs(Ok(refs))) => {
            assert!(refs.head().is_some(), "HEAD is on `main`, which has a tip");
        }
        other => panic!("expected the refs, got {other:?}"),
    }
    let walked_on = runner.ahead_behind.walked_on();
    assert!(
        !walked_on.is_empty(),
        "ref-states has branches with upstreams"
    );
    let here = std::thread::current().id();
    assert!(
        walked_on.iter().all(|&thread| thread != here),
        "{walked_on:?}"
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}

/// What the job `id` brought back, past its own start announcement: the
/// detail pane's jobs run under ids `update` chose, so they are told apart
/// by id, not by task.
fn product_of(events: &Receiver<AppEvent>, id: JobId) -> Product {
    loop {
        match next_report(events) {
            AppEvent::Started(started, _) if started == id => {}
            AppEvent::Produced(produced, product) if produced == id => return product,
            other => panic!("expected job {id:?} to answer, got {other:?}"),
        }
    }
}

/// A writable copy of the feature-branches fixture, `main`'s tip, and how
/// many files that tip changes against its first parent (git's own count).
fn feature_branches_main() -> (graft_fixtures::Scratch, graft_core::oid::Oid, usize) {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
    let git = |args: &[&str]| {
        let out = graft_fixtures::git::run(scratch.path(), args).expect("git runs on the fixture");
        String::from_utf8(out).expect("git prints utf-8 here")
    };
    let oid = graft_core::oid::Oid::from_hex(git(&["rev-parse", "main"]).trim())
        .expect("main names a commit");
    let files = git(&["diff", "--name-only", "main^1", "main"])
        .lines()
        .count();
    assert!(files > 0, "the fixture's main tip must change a file");
    (scratch, oid, files)
}

/// The detail pane's three reads, end to end on the pool, over a writable
/// copy of a corpus fixture: the file list, one file's diff, and the refs
/// that contain the commit.
#[test]
fn the_detail_reads_come_back_as_products() {
    use crate::update::detail::{DetailSpec, DiffKey, LocalJobs};

    let (scratch, oid, expected_files) = feature_branches_main();
    let (runner, events, _store) = runner_at(scratch.path(), 1);
    let mut ids = LocalJobs::default();
    let spec = DetailSpec::Commit(oid);
    let options = graft_git::diff::DiffOptions::default();

    let list_id = ids.next_id();
    runner.dispatch(Command::ListFiles(list_id, spec.diff_spec(), options));
    let Product::Files(Ok((view, list))) = product_of(&events, list_id) else {
        panic!("the file list must read");
    };
    assert_eq!(view.files as usize, expected_files);
    assert_eq!(list.files.len(), expected_files);

    let diff_id = ids.next_id();
    let key = DiffKey {
        spec,
        entry: 0,
        options,
        max_lines: 5_000,
    };
    runner.dispatch(Command::ReadFileDiff(diff_id, key, list.files[0].clone()));
    let Product::FileDiff(got, Ok(_)) = product_of(&events, diff_id) else {
        panic!("the first file's diff must read");
    };
    assert_eq!(got, key);

    let contained_id = ids.next_id();
    runner.dispatch(Command::ContainedIn(contained_id, oid));
    let Product::ContainedIn(got, Ok(contained)) = product_of(&events, contained_id) else {
        panic!("the containing refs must read");
    };
    assert_eq!(got, oid);
    assert!(
        contained.branches.iter().any(|b| b.as_slice() == b"main"),
        "main contains its own tip, got {contained:?}"
    );
    assert_eq!(runner.shutdown(PATIENCE), Shutdown::Clean);
}
