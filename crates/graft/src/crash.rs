//! The panic hook (PROMPT.md §6.3): restore the terminal *first*, then leave a
//! crash report on disk, then say where it went — in that order, because a
//! backtrace printed into a raw-mode alternate screen is a backtrace nobody
//! can read or copy.
//!
//! The hook itself must never panic. Everything below therefore degrades
//! instead of failing: an unwritable report directory prints the report to
//! stderr, and an unreadable clock produces a zero timestamp.

use std::any::Any;
use std::backtrace::Backtrace;
use std::cell::{Cell, RefCell};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use graft_core::on_disk::Generation;

use crate::cli::COMMAND;
use crate::terminal::TerminalRestorer;

/// Install the panic hook for the rest of the process.
///
/// The `restorer` is shared with the live [`crate::terminal::TerminalSession`],
/// so restoring here and the guard's later `Drop` are the same idempotent
/// restore: the sequences go out once.
pub fn install(restorer: TerminalRestorer) {
    // Rust's default hook is deliberately not chained: it would print the same
    // panic message and backtrace into stderr that the report already holds,
    // burying the one line that matters — where the report is.
    panic::set_hook(Box::new(move |panic_info| {
        let timestamp = Timestamp::now();
        let report = report(
            message(panic_info.payload()),
            &location(panic_info),
            &Backtrace::force_capture().to_string(),
            &timestamp,
        );

        if being_caught() {
            // A caller is catching this panic and will report it itself — the
            // job runner, whose worker must not take the application down with
            // it. Graft is still drawing, so restoring the terminal here would
            // wipe a live screen and printing here would corrupt the frame.
            // The report is still written, and where it went is left for the
            // catcher to put in front of the user.
            remember_report(save(&report, &timestamp));
            return;
        }

        let restore_error = restorer.restore().err();
        let saved = save(&report, &timestamp);
        announce(&mut io::stderr(), &report, saved, restore_error);
    }));
}

thread_local! {
    /// True while this thread is inside [`catching`].
    static BEING_CAUGHT: Cell<bool> = const { Cell::new(false) };
    /// Where the hook put the report for the panic [`catching`] is about to
    /// return. Written by the hook, taken by `catching`.
    static CAUGHT_REPORT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A panic that was caught rather than allowed to end the process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panicked {
    /// What the panic said.
    pub message: String,
    /// Where the crash report went, or why it could not be written. `None`
    /// when no hook is installed — a library user, or a test.
    pub report: Option<String>,
}

/// Run `work`, catching a panic instead of letting it end the process, with
/// Graft's crash handling stood down for the duration.
///
/// This is what a job runner worker wraps a job in: a panicking job is a bug,
/// but it is one job's bug, and the application it was started from is still
/// on screen and still responsive. The caller gets the panic message and the
/// crash report's location, and owes the user both — a caught panic that goes
/// unreported is a bug that ships (CLAUDE.md).
///
/// `work` is asserted to be unwind-safe: a job owns everything it touches,
/// and what it shares with the runner (the cancellation flag, the channel the
/// result goes down) is atomic or poison-checked, so a half-finished job
/// leaves nothing behind for the next one to trip over.
pub fn catching<R>(work: impl FnOnce() -> R) -> Result<R, Panicked> {
    let outer = BEING_CAUGHT.with(|caught| caught.replace(true));
    let outcome = panic::catch_unwind(AssertUnwindSafe(work));
    BEING_CAUGHT.with(|caught| caught.set(outer));

    outcome.map_err(|payload| Panicked {
        message: message(payload.as_ref()).to_owned(),
        report: CAUGHT_REPORT.with(|report| report.borrow_mut().take()),
    })
}

/// Whether the panic now unwinding this thread is one [`catching`] will hand
/// back to its caller.
///
/// A thread whose locals are already being destroyed cannot answer, and the
/// safe answer there is "no": full crash handling, rather than a panic that
/// nobody reports.
fn being_caught() -> bool {
    BEING_CAUGHT.try_with(Cell::get).unwrap_or(false)
}

/// Leave the report's whereabouts for [`catching`] to pick up.
fn remember_report(saved: io::Result<PathBuf>) {
    let told = match saved {
        Ok(path) => path.display().to_string(),
        // Losing the report must not also lose the fact that it was lost.
        Err(error) => format!("the crash report could not be written: {error}"),
    };
    // The failure is deliberately not propagated: `try_with` fails only while
    // this thread's locals are being destroyed, and by then there is no
    // `catching` frame left to read the note. Nothing is lost, and a panic hook
    // must never panic.
    CAUGHT_REPORT
        .try_with(|report| *report.borrow_mut() = Some(told))
        .unwrap_or(());
}

/// Write the report where `install`'s hook would, and hand back its path.
fn save(report: &str, timestamp: &Timestamp) -> io::Result<PathBuf> {
    match report_dir() {
        Some(dir) => write_report(&dir, timestamp, report),
        None => Err(io::Error::other(
            "neither $XDG_STATE_HOME nor $HOME is set, so there is nowhere to write a crash report",
        )),
    }
}

/// Tell the user what happened in as few lines as possible: where the report
/// is, or — if it could not be written — the report itself, since the
/// alternative is losing it.
fn announce(
    out: &mut impl Write,
    report: &str,
    saved: io::Result<PathBuf>,
    restore_error: Option<io::Error>,
) {
    let mut lines = String::new();
    if let Some(error) = restore_error {
        lines.push_str(&format!(
            "{COMMAND}: could not restore the terminal: {error}\n"
        ));
    }
    match saved {
        Ok(path) => lines.push_str(&format!("{COMMAND} crashed. Report: {}\n", path.display())),
        Err(error) => lines.push_str(&format!(
            "{COMMAND} crashed, and the report could not be written ({error}):\n\n{report}\n"
        )),
    }
    // Ignoring this write is deliberate: stderr is the last channel there is,
    // and the hook must not panic on the way out.
    let _ = out.write_all(lines.as_bytes());
}

/// `${XDG_STATE_HOME:-~/.local/state}/tributary`.
fn report_dir() -> Option<PathBuf> {
    let state_home = non_empty("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| non_empty("HOME").map(|home| Path::new(&home).join(".local").join("state")))?;
    Some(report_dir_under(&state_home))
}

/// The reports' folder inside a state home: the app's folder in force, one
/// source with the config folder (`Generation::CURRENT`). Reports written under
/// the old name before the rename are left where they are.
fn report_dir_under(state_home: &Path) -> PathBuf {
    state_home.join(Generation::CURRENT.dir())
}

fn non_empty(variable: &str) -> Option<String> {
    env::var(variable).ok().filter(|value| !value.is_empty())
}

/// `<dir>/crash-<timestamp>.log`, creating the directory if it is missing.
///
/// The timestamp only resolves to the second, and two threads can panic inside
/// the same second, so the file is created exclusively and a suffix is added
/// until the name is free: a second crash report must never quietly overwrite
/// the first. If every name is taken the error is returned, and the caller
/// prints the whole report to stderr rather than losing it.
fn write_report(dir: &Path, timestamp: &Timestamp, report: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let stem = timestamp.for_filename();
    let mut last = None;
    for attempt in 0..MAX_REPORTS_PER_SECOND {
        let path = match attempt {
            0 => dir.join(format!("crash-{stem}.log")),
            n => dir.join(format!("crash-{stem}-{n}.log")),
        };
        match fs::File::create_new(&path) {
            Ok(mut file) => {
                file.write_all(report.as_bytes())?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("no crash report filename was free")))
}

/// Enough to tell a burst of panicking threads apart; past this the reports are
/// all the same second and stderr is the better place for the rest.
const MAX_REPORTS_PER_SECOND: u32 = 64;

/// The report itself: what panicked, where, and how it got there.
fn report(message: &str, location: &str, backtrace: &str, timestamp: &Timestamp) -> String {
    format!(
        "{COMMAND} {version} panicked\n\
         time:     {time}\n\
         location: {location}\n\
         message:  {message}\n\
         \n\
         backtrace:\n{backtrace}",
        version = env!("CARGO_PKG_VERSION"),
        time = timestamp.for_report(),
    )
}

/// The panic message. Every `panic!` produces a payload that is either a
/// `&str` (a literal) or a `String` (a formatted message); anything else came
/// from `panic_any` and can only be described.
fn message(payload: &(dyn Any + Send)) -> &str {
    if let Some(literal) = payload.downcast_ref::<&str>() {
        literal
    } else if let Some(formatted) = payload.downcast_ref::<String>() {
        formatted
    } else {
        "<non-string panic payload>"
    }
}

fn location(panic_info: &PanicHookInfo<'_>) -> String {
    match panic_info.location() {
        Some(location) => location.to_string(),
        None => "<unknown>".to_owned(),
    }
}

/// A wall-clock instant, formatted without pulling in a date library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Timestamp {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl Timestamp {
    /// UTC — a crash report is read next to a log or a shell history, and a
    /// clock that went backwards should not be able to stop a report being
    /// written, so a pre-epoch clock reads as the epoch itself.
    fn now() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since_epoch| since_epoch.as_secs());
        Self::from_unix_seconds(seconds)
    }

    fn from_unix_seconds(seconds: u64) -> Self {
        // Both conversions are total: whole days since the epoch are at most
        // `u64::MAX / 86_400`, which is four orders of magnitude inside `i64`,
        // and a second of the day is below 86_400. The fallbacks are
        // unreachable, and are clamps rather than failures because a strange
        // clock must not stop a crash report being written.
        let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
        let second_of_day = u32::try_from(seconds % 86_400).unwrap_or(0);
        let (year, month, day) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            hour: second_of_day / 3_600,
            minute: (second_of_day / 60) % 60,
            second: second_of_day % 60,
        }
    }

    fn for_report(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// No colons: they are legal in a POSIX filename but awkward everywhere a
    /// path gets pasted.
    fn for_filename(self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Days since the Unix epoch to a calendar date, by Howard Hinnant's
/// `civil_from_days` (public domain). Shifting the year to start in March puts
/// the leap day last, which is what makes the month arithmetic branch-free.
// The algorithm's own arithmetic keeps the day in 1..=31 and the month in
// 1..=12 before either is narrowed, so neither cast can truncate or lose a
// sign. Rewriting them as fallible conversions would add branches that can
// never be taken to a piece of arithmetic whose whole point is that it has
// none.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "day and month are already bounded by the algorithm"
)]
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Re-base onto 0000-03-01, the start of a 400-year era.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A private directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "graft-crash-test-{name}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    /// The acceptance case: a real panic payload, caught, becomes a file that
    /// holds the message, the location and the backtrace.
    #[test]
    fn a_report_file_holds_the_message_location_and_backtrace() {
        let panicked = panic::catch_unwind(AssertUnwindSafe(|| panic!("the graph lane vanished")));
        let payload = panicked.unwrap_err();
        let timestamp = Timestamp::from_unix_seconds(1_789_216_800);

        let report = report(
            message(payload.as_ref()),
            "crates/graft/src/graph.rs:42:13",
            &Backtrace::force_capture().to_string(),
            &timestamp,
        );
        let dir = TempDir::new("report");
        let path = write_report(&dir.0, &timestamp, &report).unwrap();
        let written = fs::read_to_string(&path).unwrap();

        assert_eq!(path, dir.0.join("crash-20260912-124000.log"));
        assert!(written.contains("the graph lane vanished"), "{written}");
        assert!(
            written.contains("crates/graft/src/graph.rs:42:13"),
            "{written}"
        );
        assert!(written.contains("2026-09-12T12:40:00Z"), "{written}");
        assert!(written.contains("backtrace:\n"), "{written}");
        // `force_capture` ignores RUST_BACKTRACE, so there are always frames.
        assert!(written.contains("graft"), "{written}");
    }

    #[test]
    fn reports_go_to_the_tributary_folder_and_name_the_command() {
        assert!(report_dir_under(Path::new("/state")).ends_with("tributary"));
        let first = report("m", "l", "b", &Timestamp::from_unix_seconds(0));
        let expected = format!("trib {} panicked\n", env!("CARGO_PKG_VERSION"));
        assert!(first.starts_with(&expected), "{first}");
    }

    #[test]
    fn a_formatted_panic_message_survives() {
        let panicked = panic::catch_unwind(|| panic!("{} lanes, expected {}", 3, 4));
        let payload = panicked.unwrap_err();

        assert_eq!(message(payload.as_ref()), "3 lanes, expected 4");
    }

    #[test]
    fn a_payload_that_is_not_a_string_is_described_not_dropped() {
        let panicked = panic::catch_unwind(|| panic::panic_any(7_u8));
        let payload = panicked.unwrap_err();

        assert_eq!(message(payload.as_ref()), "<non-string panic payload>");
    }

    #[test]
    fn the_report_directory_is_created_if_it_is_missing() {
        let dir = TempDir::new("nested");
        let nested = dir.0.join("state").join("graft");
        let timestamp = Timestamp::from_unix_seconds(0);

        let path = write_report(&nested, &timestamp, "report").unwrap();

        assert_eq!(path, nested.join("crash-19700101-000000.log"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "report");
    }

    /// Two panics inside the same second must leave two reports, not one.
    #[test]
    fn a_second_report_in_the_same_second_does_not_overwrite_the_first() {
        let dir = TempDir::new("collision");
        let timestamp = Timestamp::from_unix_seconds(1_789_216_800);

        let first = write_report(&dir.0, &timestamp, "first").unwrap();
        let second = write_report(&dir.0, &timestamp, "second").unwrap();

        assert_eq!(first, dir.0.join("crash-20260912-124000.log"));
        assert_eq!(second, dir.0.join("crash-20260912-124000-1.log"));
        assert_eq!(fs::read_to_string(&first).unwrap(), "first");
        assert_eq!(fs::read_to_string(&second).unwrap(), "second");
    }

    #[test]
    fn the_announcement_names_the_report_path() {
        let mut out = Vec::new();

        announce(
            &mut out,
            "report",
            Ok(PathBuf::from("/state/crash.log")),
            None,
        );

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "trib crashed. Report: /state/crash.log\n"
        );
    }

    /// Losing the file must not lose the report, and a terminal that could not
    /// be restored is worth saying out loud.
    #[test]
    fn an_unwritable_report_is_printed_instead_and_a_restore_failure_is_named() {
        let mut out = Vec::new();

        announce(
            &mut out,
            "the whole report",
            Err(io::Error::other("read-only file system")),
            Some(io::Error::other("stdout is closed")),
        );

        let printed = String::from_utf8(out).unwrap();
        assert!(printed.contains("stdout is closed"), "{printed}");
        assert!(printed.contains("read-only file system"), "{printed}");
        assert!(printed.contains("the whole report"), "{printed}");
    }

    /// Why a job can panic without the screen going away: inside `catching`,
    /// the thread is marked, and the hook uses that mark to stand down instead
    /// of restoring the terminal out from under a running application.
    ///
    /// The mark is per thread and nests, so a job that catches a panic of its
    /// own leaves the worker's own catching frame exactly as it found it.
    #[test]
    fn a_thread_inside_catching_tells_the_hook_to_stand_down() {
        assert!(
            !being_caught(),
            "an ordinary thread is not catching anything"
        );

        let seen = catching(|| {
            let outer = being_caught();
            let inner = catching(being_caught).expect("this closure does not panic");
            let after_inner = being_caught();
            (outer, inner, after_inner)
        })
        .expect("this closure does not panic");

        assert_eq!(seen, (true, true, true));
        assert!(!being_caught(), "the mark is cleared on the way out");
    }

    /// And a panic that nobody is catching leaves the mark alone, so the hook
    /// does the full job: restore, report, announce.
    #[test]
    fn a_panic_that_escapes_catching_clears_the_mark_behind_it() {
        let panicked = catching(|| panic!("lane 3 vanished"));

        assert_eq!(
            panicked.expect_err("the closure panics").message,
            "lane 3 vanished"
        );
        assert!(!being_caught(), "a panic must not leave the mark set");
    }

    #[test]
    fn timestamps_cover_the_epoch_leap_years_and_century_rules() {
        let formatted = |seconds| Timestamp::from_unix_seconds(seconds).for_report();

        assert_eq!(formatted(0), "1970-01-01T00:00:00Z");
        assert_eq!(formatted(1), "1970-01-01T00:00:01Z");
        assert_eq!(formatted(68_255_999), "1972-02-29T23:59:59Z");
        // 2000 is a leap year (divisible by 400); 1900 and 2100 are not.
        assert_eq!(formatted(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(formatted(4_107_542_400), "2100-03-01T00:00:00Z");
        assert_eq!(formatted(1_789_216_800), "2026-09-12T12:40:00Z");
    }
}
