//! The empty state's clone host (m8/18b): its pool's quit, and a clone's
//! notice carried to the repository it opens. Real git, loopback remotes.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_fixtures::remote::{HttpRemote, HungRemote, LocalRemote};
use mergewright_ui::keymap::Keymap;
use signal_hook::iterator::Signals;

use super::super::exit_wait::Ending;
use super::super::startup::arrive;
use super::{Carried, Outcome, OutsideJobs, Place, carry_out};
use crate::config::Loaded;
use crate::empty_state::{CloneRun, CloneState, OutsideState, OutsideStep, Prompt, outside_edge};
use crate::event_loop::{Exit, Wake};
use crate::first_launch::FirstLaunch;
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
        first_launch: None,
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
        &FirstLaunch::default(),
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
    let quit = carry_out(
        step,
        &mut state,
        &mut jobs,
        &config,
        &FirstLaunch::default(),
    );
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
        &FirstLaunch::default(),
    );
    assert!(hung.wait_for_connection(Duration::from_secs(30)));
    let Some(Prompt::Clone(clone)) = &state.prompt else {
        panic!("no clone prompt");
    };
    let CloneRun::Running { job, .. } = clone.run else {
        panic!("not running");
    };

    assert!(
        carry_out(
            OutsideStep::StopClone(job),
            &mut state,
            &mut jobs,
            &config,
            &FirstLaunch::default(),
        )
        .is_none()
    );
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
    Carried::default()
}

/// A clone that landed with a notice (its askpass socket not cleaned up):
/// the empty state opens the repository with the notice, the next place's
/// first frame says it, and the place after — any other `Exit::Open` —
/// arrives with none.
#[test]
fn a_clone_notice_reaches_the_next_place_once() {
    let scratch = tempfile::tempdir().unwrap();
    let cloned = mergewright_ops::init::init_here(scratch.path()).expect("git init");
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
        &FirstLaunch::default(),
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
    let repo: PathBuf = mergewright_ops::init::init_here(scratch.path()).expect("git init");
    let mut state = outside(scratch.path());
    let (sender, _wakes) = channel();
    let mut jobs = OutsideJobs::new(sender);

    let opened = carry_out(
        OutsideStep::Open(repo),
        &mut state,
        &mut jobs,
        &Loaded::default(),
        &FirstLaunch::default(),
    );

    assert!(
        matches!(opened, Some(Outcome::Opened { notice: None, .. })),
        "an open is not a clone"
    );
}

/// The first-launch line ended in one place stays gone in the next
/// (m8/18e, review M1): `drive`'s switch is ended by whichever loop carries
/// the palette's request, and every later `Arrival` reads it. From a
/// repository (the palette, then `Exit::Open`) and from the empty state
/// into a repository. A tour arrival is built by the same `repo_arrival`.
#[test]
fn a_line_ended_in_one_place_stays_gone_in_the_next() {
    let scratch = tempfile::tempdir().unwrap();
    let repo = mergewright_ops::init::init_here(scratch.path()).expect("git init");
    ended_in_a_repository(&repo);
    ended_in_the_empty_state(scratch.path());
}

/// A repository: the palette opens, its loop carries the request, and
/// `Exit::Open` goes on to `repo` without the line.
fn ended_in_a_repository(repo: &Path) {
    let open = AppEvent::Core(mergewright_core::event::Event::Key(KeyPress::plain(
        KeyCode::Char('?'),
    )));
    let config_dir = tempfile::tempdir().unwrap();
    let mut process = carried();
    process.first_launch = FirstLaunch::shown(config_dir.path().to_path_buf());
    let mut graph = AppState::default();
    arrive(&mut graph, process.repo_arrival(None));
    assert_eq!(
        graph.first_launch.as_deref().map(PathBuf::as_path),
        Some(config_dir.path())
    );
    let (opened, commands) = crate::update::update(&graph, open);
    assert_eq!(opened.first_launch, None);
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(Vec::<u8>::new())).unwrap();
    let outside_world = super::super::outside::Outside {
        first_launch: Some(&process.first_launch),
        ..Default::default()
    };
    let (after, _) = super::super::running::carry_out(
        &mut terminal,
        &crate::session::for_tests::session(),
        &NoJobs,
        opened,
        outside_world,
        commands,
    )
    .unwrap();
    assert_eq!(after.status, Status::Ready, "written cleanly");
    assert_eq!(
        crate::first_launch::test_read(config_dir.path()).as_deref(),
        Some("done\n")
    );
    let next = super::moved(
        Exit::Open(crate::event_loop::Open::at(repo.to_path_buf())),
        &mut None,
        &mut process,
        &Loaded::default(),
    );
    assert!(matches!(next, Ok(Place::Repo { .. })), "the open goes on");
    assert_eq!(process.repo_arrival(None).first_launch, None);
}

/// The empty state in `folder`: the palette opens, its loop carries the
/// step, and the next repository arrives without the line.
fn ended_in_the_empty_state(folder: &Path) {
    let config_dir = tempfile::tempdir().unwrap();
    let mut process = carried();
    process.first_launch = FirstLaunch::shown(config_dir.path().to_path_buf());
    let mut state = OutsideState {
        first_launch: process.first_launch.line(),
        ..outside(folder)
    };
    let step = outside_edge(
        &mut state,
        &Keymap::default(),
        KeyPress::plain(KeyCode::Char('?')),
    );
    assert_eq!(
        step,
        OutsideStep::FirstLaunchDone(config_dir.path().to_path_buf())
    );
    let (sender, _wakes) = channel();
    let mut jobs = OutsideJobs::new(sender);
    let stayed = carry_out(
        step,
        &mut state,
        &mut jobs,
        &Loaded::default(),
        &process.first_launch,
    );
    assert!(stayed.is_none(), "the palette stays up");
    assert_eq!(state.problem, None, "written cleanly");
    assert_eq!(
        crate::first_launch::test_read(config_dir.path()).as_deref(),
        Some("done\n")
    );
    assert_eq!(process.repo_arrival(None).first_launch, None);
}

/// The repository loop's commands here are its own (the line); a job would
/// be a wrong turn.
struct NoJobs;

impl crate::jobs::Dispatch for NoJobs {
    fn dispatch(&self, command: crate::update::Command) {
        panic!("dispatched to jobs: {command:?}");
    }
}

/// Mergewright's own binary, which git runs as the askpass helper: beside this
/// test binary's `deps/` (as `jobs::clone::tests::mergewright_binary`; a unit test
/// has no `CARGO_BIN_EXE_mwr`).
fn mergewright_binary() -> PathBuf {
    let test_binary = std::env::current_exe().unwrap();
    let profile = test_binary
        .parent()
        .and_then(Path::parent)
        .expect("a test binary lives in <target>/<profile>/deps");
    let mwr = profile.join(crate::cli::COMMAND);
    assert!(
        mwr.is_file(),
        "{} is not built: run the workspace's tests (cargo nextest run), which build it",
        mwr.display()
    );
    mwr
}

/// `esc` in the clone's password box stops the clone (rulings R6: "git
/// asks → `esc` in it stops the clone"): `clone stopped · nothing kept`,
/// never `clone failed`, no folder, no box. The order inside `carry_out`
/// (stop, then refuse) is what holds this (review M2).
///
/// Runs in a child of this test binary with the system and global git
/// config off, so his keychain helper neither answers nor is asked (as
/// `jobs::clone::tests::asks_through_the_pool_and_clones`).
#[test]
fn esc_in_the_clone_password_box_stops_the_clone() {
    let empty_config = tempfile::NamedTempFile::new().unwrap();
    let child = "event_loop::places::tests::child_esc_in_the_clone_password_box";
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            child,
            "--include-ignored",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty_config.path())
        .env("MWR_ESC_CLONE_CHILD", "1")
        .output()
        .expect("the child test runs");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the child failed:\n{said}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // A filter that matched nothing would also exit 0.
    assert!(
        said.contains(&format!("test {child} ... ok")),
        "the child test did not run:\n{said}"
    );
}

/// `esc` in the box stops the clone first, then refuses git's ask: the
/// other order lets git die of the refusal before the stop lands, which
/// reads as a wrong password (review M2). On a real pool a swap only loses
/// a race (git's run checks its stop on every poll, `mergewright_git::run`), so
/// the order itself is held here.
#[test]
fn esc_in_the_box_stops_before_it_refuses() {
    #[derive(Default)]
    struct Calls(std::cell::RefCell<Vec<&'static str>>);
    impl super::AnswersAsks for Calls {
        fn cancel(&self, _: mergewright_core::job::JobId) {
            self.0.borrow_mut().push("stop");
        }
        fn answer_credential(
            &self,
            _: mergewright_core::job::JobId,
            answer: Option<mergewright_git::run::askpass::Secret>,
        ) -> bool {
            self.0
                .borrow_mut()
                .push(if answer.is_some() { "answer" } else { "refuse" });
            true
        }
    }
    let job = mergewright_core::job::JobId::new(7);

    let esc = Calls::default();
    super::answer_ask(&esc, job, None);
    assert_eq!(*esc.0.borrow(), ["stop", "refuse"]);

    let typed = Calls::default();
    super::answer_ask(
        &typed,
        job,
        Some(crate::update::credential::Answer::new(
            zeroize::Zeroizing::new(b"sesame".to_vec()),
        )),
    );
    assert_eq!(
        *typed.0.borrow(),
        ["answer"],
        "a typed answer stops nothing"
    );
}

/// The child half of the test above; does nothing unless it started it.
#[test]
#[ignore = "run only as the child of esc_in_the_clone_password_box_stops_the_clone"]
fn child_esc_in_the_clone_password_box() {
    if std::env::var_os("MWR_ESC_CLONE_CHILD").is_none() {
        return;
    }
    let remote = LocalRemote::new(mergewright_fixtures::Shape::Linear).unwrap();
    let http = HttpRemote::serve(remote.bare(), "tester", "sesame").unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let (mut state, mut jobs, wakes) = clone_started_with_askpass(scratch.path(), &http.url());
    let (config, line) = (Loaded::default(), FirstLaunch::default());

    // git asks: the box opens.
    let deadline = Instant::now() + Duration::from_secs(60);
    carry_wakes_until(&mut state, &mut jobs, &wakes, deadline, |state| {
        state.credential.is_some()
    });

    // `esc` in the box.
    let Some(Prompt::Clone(clone)) = &state.prompt else {
        panic!("no clone prompt: {:?}", state.prompt);
    };
    let CloneRun::Running { job, .. } = clone.run else {
        panic!("not running: {:?}", clone.run);
    };
    let step = outside_edge(
        &mut state,
        &Keymap::default(),
        KeyPress::plain(KeyCode::Escape),
    );
    assert_eq!(step, OutsideStep::Credential(job, None));
    assert!(carry_out(step, &mut state, &mut jobs, &config, &line).is_none());

    // Its answer.
    carry_wakes_until(
        &mut state,
        &mut jobs,
        &wakes,
        deadline,
        |state| matches!(&state.prompt, Some(Prompt::Clone(clone)) if matches!(clone.run, CloneRun::Ended { .. })),
    );
    let Some(Prompt::Clone(CloneState {
        run: CloneRun::Ended { words },
        ..
    })) = &state.prompt
    else {
        panic!("not ended: {:?}", state.prompt);
    };
    assert!(!words.contains("clone failed"), "{words}");
    assert_eq!(words, "clone stopped \u{b7} nothing kept");
    assert!(!into.exists(), "the stopped clone left its folder");
    assert_eq!(state.credential, None);
}

/// The empty state in `folder` with a clone of `url` into `folder/x`
/// started by `⏎` on the clone prompt, on the pool `OutsideJobs::started`
/// would make but with Mergewright's binary as git's askpass.
fn clone_started_with_askpass(
    folder: &Path,
    url: &str,
) -> (OutsideState, OutsideJobs, Receiver<Wake>) {
    let mut state = outside(folder);
    let (sender, wakes) = channel();
    let mut jobs = OutsideJobs::new(sender.clone());
    let store = Arc::new(crate::repos::RepoStore::new(folder.to_path_buf(), None));
    jobs.pool = Some(
        crate::jobs::JobRunner::start(sender, 1, store)
            .unwrap()
            .with_askpass_helper(mergewright_binary()),
    );
    state.prompt = Some(Prompt::Clone(CloneState {
        url: mergewright_ui::text_input::TextField::new(url),
        run: CloneRun::Typing,
    }));
    let started = carry_out(
        OutsideStep::StartClone(CloneRequest {
            url: url.to_owned(),
            into: folder.join("x"),
        }),
        &mut state,
        &mut jobs,
        &Loaded::default(),
        &FirstLaunch::default(),
    );
    assert!(started.is_none(), "starting a clone stays on the screen");
    (state, jobs, wakes)
}

/// Carry each wake as the empty state's loop does — `outside_job`, then
/// `carry_out` — until `done`, waiting no later than `deadline`.
fn carry_wakes_until(
    state: &mut OutsideState,
    jobs: &mut OutsideJobs,
    wakes: &Receiver<Wake>,
    deadline: Instant,
    done: impl Fn(&OutsideState) -> bool,
) {
    while !done(state) {
        let wake = wakes
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("the clone's next event in time");
        let step = crate::empty_state::outside_job(state, &wake);
        let config = Loaded::default();
        assert!(carry_out(step, state, jobs, &config, &FirstLaunch::default()).is_none());
    }
}

#[test]
fn practice_at_reads_whole_seconds_and_refuses_the_rest() {
    use std::ffi::OsStr;

    use mergewright_ops::tour::PracticeAt;

    use super::practice_at;

    assert_eq!(
        practice_at(None, 42),
        Ok(PracticeAt::Now { now_unix_secs: 42 })
    );
    assert_eq!(
        practice_at(Some(OsStr::new("1700000000")), 42),
        Ok(PracticeAt::Fixed {
            first_unix_secs: 1_700_000_000
        })
    );
    for bad in ["x", "-1", "1.5", ""] {
        assert_eq!(
            practice_at(Some(OsStr::new(bad)), 42),
            Err(format!(
                "MWR_PRACTICE_EPOCH must be whole seconds since 1970, got {bad}"
            )),
            "{bad:?}"
        );
    }
}
