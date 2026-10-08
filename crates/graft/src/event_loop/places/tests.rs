//! The empty state's clone host (m8/18b): its pool's quit, and a clone's
//! notice carried to the repository it opens. Real git, loopback remotes.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use graft_core::event::{KeyCode, KeyPress};
use graft_fixtures::remote::HungRemote;
use graft_ui::keymap::Keymap;
use signal_hook::iterator::Signals;

use super::super::exit_wait::Ending;
use super::super::startup::arrive;
use super::{Carried, Outcome, OutsideJobs, Place, carry_out};
use crate::config::Loaded;
use crate::empty_state::{CloneRun, OutsideState, OutsideStep, Prompt, outside_edge};
use crate::event_loop::{Exit, Wake};
use crate::jobs::SHUTDOWN_GRACE;
use crate::jobs::clone::{CloneAnswer, CloneRequest};
use crate::update::{AppEvent, AppState, Product, Status};

/// The empty state in `folder`, nothing open.
fn outside(folder: &Path) -> OutsideState {
    OutsideState {
        folder: folder.to_path_buf(),
        missing: false,
        home: None,
        recents: Vec::new(),
        cursor: 0,
        prompt: None,
        problem: None,
        palette: None,
        config_problems: Vec::new(),
        first_launch: crate::first_launch::FirstLaunch::default(),
        credential: None,
    }
}

/// A clone's answer off the loop's channel, skipping its progress.
fn clone_answer(wakes: &Receiver<Wake>, within: Duration) -> CloneAnswer {
    let deadline = Instant::now() + within;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match wakes.recv_timeout(left) {
            Ok(Wake::Event(AppEvent::Produced(_, Product::Cloned(answer)))) => return answer,
            Ok(_) => {}
            Err(why) => panic!("no clone answer within {within:?}: {why}"),
        }
    }
}

/// `Ctrl+C` while a clone runs: the empty state ends with `Exit::Quit` and
/// hands its pool on; the pool's stop — what `wind_down` runs once the
/// terminal is back — cancels the clone within `SHUTDOWN_GRACE`, with no
/// waiting line (a clone is stoppable, rulings R4), and git's cleanup
/// leaves no folder.
#[test]
fn quitting_mid_clone_stops_it() {
    let hung = HungRemote::listen().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let mut state = outside(scratch.path());
    let (sender, wakes) = channel();
    let mut jobs = OutsideJobs::new(sender);
    let config = Loaded::default();

    let started = carry_out(
        OutsideStep::StartClone(CloneRequest {
            url: hung.url(),
            into: into.clone(),
        }),
        &mut state,
        &mut jobs,
        &config,
    );
    assert!(started.is_none(), "starting a clone stays on the screen");
    assert!(
        matches!(&state.prompt, Some(Prompt::Clone(clone)) if matches!(clone.run, CloneRun::Running { .. })),
        "{:?}",
        state.prompt
    );
    assert!(
        hung.wait_for_connection(Duration::from_secs(30)),
        "git never reached the hung remote"
    );

    let step = outside_edge(
        &mut state,
        &Keymap::default(),
        KeyPress::control(KeyCode::Char('c')),
    );
    let quit = carry_out(step, &mut state, &mut jobs, &config);
    assert!(
        matches!(quit, Some(Outcome::Exit(Exit::Quit))),
        "Ctrl+C quits mid-clone"
    );

    stop_as_wind_down_does(jobs);
    assert_eq!(
        clone_answer(&wakes, Duration::from_secs(5)),
        CloneAnswer::Ended {
            words: "clone stopped \u{b7} nothing kept".to_owned()
        }
    );
    assert!(!into.exists(), "the stopped clone left its folder");
}

/// The empty state's pool handed on and stopped, as `wind_down` stops it
/// once the terminal is back: within `SHUTDOWN_GRACE`, every job noticing
/// its stop, and no waiting line (a clone is stoppable, rulings R4).
fn stop_as_wind_down_does(jobs: OutsideJobs) {
    let signals = super::signals::unwatched().expect("a self-pipe");
    let resizes = Signals::new::<[i32; 0], i32>([])
        .expect("a self-pipe")
        .handle();
    let pool = jobs
        .into_pool(signals, resizes, "u".to_owned())
        .expect("the clone started a pool, handed on to stop");
    let mut stderr = Vec::new();
    let stopping = Instant::now();
    let shutdown = pool
        .stop_to(&mut stderr, Ending::Exited, None)
        .expect("the pool stops");

    assert!(
        stopping.elapsed() <= SHUTDOWN_GRACE,
        "stopped in {:?}",
        stopping.elapsed()
    );
    assert_eq!(shutdown.stalled, 0, "the clone noticed its stop");
    assert!(shutdown.forced.is_empty(), "nothing unstoppable ran");
    assert!(
        stderr.is_empty(),
        "no waiting line for a stoppable clone: {}",
        String::from_utf8_lossy(&stderr)
    );
}

/// `StopClone` (18c's `esc` while running) cancels the job on the pool; its
/// answer comes back through the loop's channel and lands on the prompt.
#[test]
fn stopping_the_clone_ends_it_on_the_prompt() {
    let hung = HungRemote::listen().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let mut state = outside(scratch.path());
    let (sender, wakes) = channel();
    let mut jobs = OutsideJobs::new(sender);
    let config = Loaded::default();
    carry_out(
        OutsideStep::StartClone(CloneRequest {
            url: hung.url(),
            into: into.clone(),
        }),
        &mut state,
        &mut jobs,
        &config,
    );
    assert!(hung.wait_for_connection(Duration::from_secs(30)));
    let Some(Prompt::Clone(clone)) = &state.prompt else {
        panic!("no clone prompt");
    };
    let CloneRun::Running { job, .. } = clone.run else {
        panic!("not running");
    };

    assert!(carry_out(OutsideStep::StopClone(job), &mut state, &mut jobs, &config).is_none());
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let wake = wakes
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("the clone answers its stop");
        if crate::empty_state::outside_job(&mut state, &wake) != OutsideStep::Stay
            || !matches!(
                &state.prompt,
                Some(Prompt::Clone(clone)) if matches!(clone.run, CloneRun::Running { .. })
            )
        {
            break;
        }
    }

    assert!(
        matches!(&state.prompt, Some(Prompt::Clone(clone)) if clone.run == CloneRun::Ended { words: "clone stopped \u{b7} nothing kept".to_owned() }),
        "{:?}",
        state.prompt
    );
    assert!(!into.exists(), "the stopped clone left its folder");
}

/// A process's carry with nothing in it yet.
fn carried() -> Carried {
    Carried {
        keymap: Arc::new(Keymap::default()),
        problems: Vec::new(),
        first_launch: crate::first_launch::FirstLaunch::default(),
        warning: None,
        notice: None,
    }
}

/// A clone that landed with a notice (its askpass socket not cleaned up):
/// the empty state opens the repository with the notice, the next place's
/// first frame says it, and the place after — any other `Exit::Open` —
/// arrives with none.
#[test]
fn a_clone_notice_reaches_the_next_place_once() {
    let scratch = tempfile::tempdir().unwrap();
    let cloned = graft_ops::init::init_here(scratch.path()).expect("git init");
    let notice = "cloned \u{b7} the credential prompt's socket could not be cleaned up: gone";
    let mut state = outside(scratch.path());
    let (sender, _wakes) = channel();
    let mut jobs = OutsideJobs::new(sender);

    let opened = carry_out(
        OutsideStep::Cloned {
            path: cloned.clone(),
            notice: Some(notice.to_owned()),
        },
        &mut state,
        &mut jobs,
        &Loaded::default(),
    );
    let Some(Outcome::Opened {
        place: Place::Repo { path, .. },
        notice: handed,
    }) = opened
    else {
        panic!("the clone was not opened");
    };
    assert_eq!(path, cloned);
    assert!(jobs.pool.is_none(), "landing a clone starts no pool");

    // As `drive` takes it on, then builds the next two arrivals.
    let mut carried = carried();
    carried.notice = handed;
    let first = carried.repo_arrival(None);
    let second = carried.repo_arrival(None);
    assert_eq!(first.notice.as_deref(), Some(notice));
    assert_eq!(second.notice, None, "said once");

    let mut landed = AppState::default();
    arrive(&mut landed, first);
    assert_eq!(landed.status, Status::sticky(notice));
    let frame = crate::view::tests::draw(&landed, 160, 12);
    assert!(
        frame.contains("the credential prompt's socket could not be cleaned up"),
        "{frame}"
    );

    let mut next = AppState::default();
    arrive(&mut next, second);
    assert_eq!(next.status, Status::Ready, "no notice on a later place");
}

/// An open or init from the empty state hands on no notice.
#[test]
fn an_ordinary_open_hands_on_no_notice() {
    let scratch = tempfile::tempdir().unwrap();
    let repo: PathBuf = graft_ops::init::init_here(scratch.path()).expect("git init");
    let mut state = outside(scratch.path());
    let (sender, _wakes) = channel();
    let mut jobs = OutsideJobs::new(sender);

    let opened = carry_out(
        OutsideStep::Open(repo),
        &mut state,
        &mut jobs,
        &Loaded::default(),
    );

    assert!(
        matches!(opened, Some(Outcome::Opened { notice: None, .. })),
        "an open is not a clone"
    );
}
