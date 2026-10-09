use std::io;

use graft_core::mistake_radar::MistakeRadarKind;
use signal_hook::consts::SIGHUP;

use super::super::Exit;
use super::{excused_by_hangup, finish_to, report_stalled_jobs_to, seed_config};
use crate::config::{Radar, RadarSetting};
use crate::jobs::Shutdown;
use crate::update::AppState;

/// A writer whose every write fails, standing in for a closed stderr.
struct FailingWriter;

impl io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("stderr is gone"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("stderr is gone"))
    }
}

#[test]
fn report_stalled_jobs_propagates_a_failing_writer() {
    let shutdown = Shutdown {
        stalled: 2,
        forced: Vec::new(),
    };

    let result = report_stalled_jobs_to(&mut FailingWriter, &shutdown);

    assert!(result.is_err(), "a failing writer must not be swallowed");
}

#[test]
fn report_stalled_jobs_clean_shutdown_never_writes() {
    // `FailingWriter` would fail the call if `Clean` reached the write at
    // all, so this also proves the early return is taken.
    let result = report_stalled_jobs_to(&mut FailingWriter, &Shutdown::CLEAN);

    assert!(result.is_ok());
}

/// `KNOWN_GAPS` m2-details-and-diffs/13b: `[radar]` config was never
/// threaded into `AppState.radar.policy`, so it had no effect. A
/// non-default setting must change what the policy reports.
#[test]
fn a_non_default_radar_setting_reaches_app_state_policy() {
    let mut default_state = AppState::default();
    seed_config(&mut default_state, &crate::config::Config::default());
    assert_eq!(
        default_state.radar.policy,
        graft_ops::mistake_radar::Policy::default(),
        "an all-default [radar] section must leave the default policy in effect"
    );

    let mut state = AppState::default();
    let radar = Radar {
        foxtrot: RadarSetting::Off,
        ..Radar::default()
    };
    seed_config(
        &mut state,
        &crate::config::Config {
            radar,
            ..crate::config::Config::default()
        },
    );

    assert_eq!(
        state.radar.policy.severity(MistakeRadarKind::FoxtrotMerge),
        None,
        "[radar] foxtrot = off must turn the finding off on AppState.radar.policy"
    );
    assert_ne!(
        state.radar.policy, default_state.radar.policy,
        "a non-default [radar] setting must change what the policy reports"
    );
}

/// The same mistake the radar setting made, one section along: a config
/// key nothing threads into `AppState` is a setting that does nothing.
#[test]
fn keys_suspend_on_ctrl_z_reaches_app_state() {
    let mut state = AppState::default();
    seed_config(&mut state, &crate::config::Config::default());
    assert!(
        !state.suspend_on_ctrl_z,
        "ADR-0002: Ctrl+Z is undo unless the user says otherwise"
    );

    let mut state = AppState::default();
    seed_config(
        &mut state,
        &crate::config::Config {
            keys: crate::config::Keys {
                suspend_on_ctrl_z: true,
                remaps: Vec::new(),
            },
            ..crate::config::Config::default()
        },
    );
    assert!(state.suspend_on_ctrl_z);
}

/// m8-polish-and-ship/17 item 5: `ui.overview = true` starts the strip shown.
#[test]
fn ui_overview_seeds_strip_shown() {
    let mut state = AppState::default();
    seed_config(&mut state, &crate::config::Config::default());
    assert!(!state.strip_shown, "off by default");

    let mut state = AppState::default();
    let mut config = crate::config::Config::default();
    config.ui.overview = true;
    seed_config(&mut state, &config);
    assert!(state.strip_shown);
}

#[test]
fn finish_with_failing_writer_still_returns_the_loop_error() {
    let loop_error = io::Error::other("loop failed");
    let restore_error = io::Error::other("restore failed");

    // Both the run and the restore failed, and even the stderr note about
    // the restore failure fails to write: the loop's error must still be
    // the one returned, per the comment in `finish_to`.
    let result = finish_to(&mut FailingWriter, Err(loop_error), Err(restore_error));

    let err = result.expect_err("both inputs were errors");
    assert_eq!(err.to_string(), "loop failed");
}

/// m8/13g: after a hang-up the restore fails with `EIO` (the terminal
/// is gone); the exit is still the `SIGHUP`, so `main` re-raises it.
#[test]
fn finish_after_a_hangup_dies_of_it_whatever_the_restore_said() {
    let refused = io::Error::from_raw_os_error(5);

    let result = finish_to(&mut FailingWriter, Ok(Exit::Signal(SIGHUP)), Err(refused));

    assert_eq!(result.expect("a hang-up exits"), Exit::Signal(SIGHUP));
}

/// m8/21b-4: a hang-up excuses only the dead terminal's own failure.
/// A restore refused for any other reason is still the exit's error.
#[test]
fn finish_after_a_hangup_still_reports_a_restore_the_terminal_did_not_refuse() {
    let refused = io::Error::from(io::ErrorKind::PermissionDenied);

    let result = finish_to(&mut FailingWriter, Ok(Exit::Signal(SIGHUP)), Err(refused));

    let err = result.expect_err("not the dead terminal's failure");
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn a_hangup_excuses_only_the_dead_terminals_failure() {
    let eio = || io::Error::from_raw_os_error(5);
    let cases = [
        (Exit::Signal(SIGHUP), eio(), true),
        (
            Exit::Signal(SIGHUP),
            io::Error::from(io::ErrorKind::UnexpectedEof),
            true,
        ),
        (
            Exit::Signal(SIGHUP),
            io::Error::from(io::ErrorKind::PermissionDenied),
            false,
        ),
        (Exit::Quit, eio(), false),
    ];
    for (exit, error, excused) in cases {
        assert_eq!(
            excused_by_hangup(&exit, &error),
            excused,
            "{exit:?} with {error:?}"
        );
    }
}

#[test]
fn finish_reports_restore_failure_when_loop_succeeded() {
    let restore_error = io::Error::other("restore failed");

    let result = finish_to(&mut io::sink(), Ok(Exit::Quit), Err(restore_error));

    let err = result.expect_err("restore failed");
    assert_eq!(err.to_string(), "restore failed");
}
