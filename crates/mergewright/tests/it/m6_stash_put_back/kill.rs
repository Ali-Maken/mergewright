//! m6-conflicts/75: a dirty checkout from the `.` menu, killed before each
//! of ADR-0045 §4's six steps, then launched again. The edits are in the
//! tree or in `git stash list` — never in neither — and when they are in the
//! list, the relaunch line says `stash@{n}`.
//!
//! **How the kill is made.** The app has no stop hook a test can reach (its
//! job's `Cancel` is private to `mergewright::jobs`), and a graceful stop is not a
//! kill: it closes the entry `Aborted` and runs the put-back. So the app
//! goes through `. ⏎ y` for real, `y`'s `Command::Operate` is taken off the
//! event before any job runs it, and the test runs that very operation the
//! way the app's job does (`jobs/operate.rs` `in_context`: the repository,
//! the system git, the journal, one `Context`) — with a `Stop` that
//! **panics** the moment the step before the target has left its mark on
//! disk. The panic unwinds out of `apply` with nothing closed and nothing
//! put back: what a `kill -9` leaves. A git the dead operation had started
//! runs on to its own end, as it would after a real kill; the relaunch waits
//! for it, so what it reads does not depend on a race.
//!
//! **Where the kill lands.** `Stop` is polled while a git runs, so the kill
//! comes at the first poll after the previous step's mark: that step's own
//! git finishing, or the next step's git already started (which then runs on).
//! Run on 2026-09-26, "before step 3" came with `stash store` done — the
//! edit in the tree *and* in `stash@{0}`. Every case the test meets holds the
//! promise; the one it cannot pin (`S` objects only, not listed) is mergewright-ops'
//! graceful-stop test's (`a_stop_before_each_step_loses_nothing`).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command as Shell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mergewright::update::{AppEvent, AppState, Command, update};
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_git::version::system_git;
use mergewright_ops::apply::{Context, apply};
use mergewright_ops::journal::{Journal, journal_path};
use mergewright_ops::operation::Operation;

use super::{cursor_to, main_and_fix, planned, rev};
use crate::app_harness::{App, git};
use crate::conflict_interim::launched;
use crate::preview::opened;

const EDIT: &str = "my edit\n";

/// ADR-0045 §4's steps, by what each is.
const STEPS: [&str; 6] = [
    "stash create",
    "begin (the Pending line)",
    "stash store",
    "reset --hard",
    "the checkout",
    "the put-back",
];

/// `main_and_fix` with `base.txt` edited and staged — the file both
/// branches hold at `base`, so the checkout carries it.
fn dirty() -> mergewright_fixtures::Scratch {
    let scratch = main_and_fix();
    std::fs::write(scratch.path().join("base.txt"), EDIT).expect("the scratch repo is writable");
    git(scratch.path(), &["add", "--", "base.txt"]);
    scratch
}

/// `. ⏎ y` on `fix`'s row, through `update`; hands back the operation `y`
/// launched, which no job has run.
fn dot_enter_y(path: &Path) -> Operation {
    let (app, state) = App::open(path);
    let state = cursor_to(&app, opened(&app, state), &rev(path, "fix"));
    let state = app.key(&state, KeyCode::Char('.'));
    let state = app.key(&state, KeyCode::Enter);
    let state = planned(&app, state);
    let (after, commands) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('y')))),
    );
    let op = commands
        .into_iter()
        .find_map(|command| match command {
            Command::Operate(_, op, ..) => Some(*op),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`y` launched no operation: {:?}", after.status));
    // The process dies with the job it was about to start: nothing the app
    // holds outlives it.
    drop(after);
    drop(state);
    app.close();
    op
}

/// A `Stop` that panics once `landed` says the step before the target has
/// left its mark — the kill.
struct KillWhen {
    landed: Box<dyn Fn() -> bool + Sync>,
    fired: AtomicBool,
}

impl Stop for KillWhen {
    fn should_stop(&self) -> bool {
        if (self.landed)() {
            self.fired.store(true, Ordering::Relaxed);
            panic!("killed");
        }
        false
    }
}

fn loose_objects(git_dir: &Path) -> usize {
    let Ok(fans) = std::fs::read_dir(git_dir.join("objects")) else {
        return 0;
    };
    fans.flatten()
        .filter(|fan| fan.file_name().len() == 2)
        .map(|fan| std::fs::read_dir(fan.path()).map_or(0, Iterator::count))
        .sum()
}

/// Killed just before step `step` (1–6): keyed to what step `step - 1`
/// leaves on disk, as mergewright-ops' `a_stop_before_each_step_loses_nothing`
/// keys its graceful stops.
fn kill_before(path: &Path, journal: PathBuf, step: usize) -> KillWhen {
    let root = path.to_path_buf();
    let git_dir = root.join(".git");
    let objects = loose_objects(&git_dir);
    let start_head = std::fs::read(git_dir.join("HEAD")).expect("HEAD reads");
    let landed: Box<dyn Fn() -> bool + Sync> = match step {
        1 => Box::new(|| true),
        // `stash create` has written its objects.
        2 => Box::new(move || loose_objects(&git_dir) > objects),
        // `begin` has written the Pending line.
        3 => Box::new(move || std::fs::read(&journal).is_ok_and(|bytes| !bytes.is_empty())),
        // `stash store` has moved `refs/stash`.
        4 => Box::new(move || git_dir.join("refs/stash").exists()),
        // `reset --hard` has put `base.txt` back.
        5 => Box::new(move || {
            std::fs::read(root.join("base.txt")).is_ok_and(|bytes| bytes == b"base\n")
        }),
        // The checkout has moved `HEAD`.
        _ => Box::new(move || std::fs::read(git_dir.join("HEAD")).ok() != Some(start_head.clone())),
    };
    KillWhen {
        landed,
        fired: AtomicBool::new(false),
    }
}

/// Runs `op` the way the app's job does, and kills it before `step`.
fn run_and_kill(path: &Path, op: &Operation, step: usize) {
    let handle = RepoHandle::open(path).expect("the repo opens");
    let journal_file = journal_path(&handle).expect("the journal path");
    let kill = kill_before(path, journal_file.clone(), step);
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_millis(),
    )
    .expect("milliseconds fit an i64");
    let git_version = system_git().expect("git is on PATH").version();
    let ran = catch_unwind(AssertUnwindSafe(|| {
        let mut journal = Journal::open(&journal_file).expect("the journal opens");
        let mut ctx = Context::open(&handle, git_version, &mut journal, now, [7; 10], &kill)
            .expect("the context opens");
        apply(&mut ctx, op).map(|_| ())
    }));
    assert!(
        ran.is_err() && kill.fired.load(Ordering::Relaxed),
        "the kill before step {step} ({}) never fired: {ran:?}",
        STEPS[step - 1]
    );
    wait_for_the_orphans();
}

/// A git the killed operation started is still running on its own, as after
/// a real `kill -9`. Waits for it to end, so the relaunch reads one state and
/// not a race. A poll on the condition, bounded: nothing here is a fix for a
/// race in the app.
///
/// Running, not merely a child: the unwind that stands in for the kill drops
/// git's `Child` unwaited, and an exited git stays this process's zombie
/// until it ends — a real kill takes the parent with it. Linux's `pgrep -P`
/// listed that zombie, so the wait ran out with no git running (seen in the
/// Debian container: `Z [git] <defunct>`, m8-polish-and-ship/30); macOS's
/// did not.
fn wait_for_the_orphans() {
    for _ in 0..3_000 {
        if running_children().is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "a git the killed operation started still runs after 30 s: {:?}",
        running_children()
    );
}

/// This process's children that have not exited (`ps` state not `Z`), as
/// `pid state command` lines, the `ps` asked leaving itself out.
fn running_children() -> Vec<String> {
    let me = std::process::id().to_string();
    let ps = Shell::new("ps")
        .args(["-A", "-o", "ppid=,pid=,stat=,comm="])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("ps runs");
    let asking = ps.id().to_string();
    let listed = ps.wait_with_output().expect("ps answers");
    assert!(listed.status.success(), "ps failed: {listed:?}");
    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (parent, pid, state) = (fields.next()?, fields.next()?, fields.next()?);
            (parent == me && pid != asking && !state.starts_with('Z'))
                .then(|| line.trim().to_owned())
        })
        .collect()
}

/// The stash entry, by position, whose `base.txt` holds the edit.
fn edit_in_the_list(path: &Path) -> Option<usize> {
    git(path, &["stash", "list", "--format=%H"])
        .lines()
        .position(|commit| git(path, &["show", &format!("{commit}:base.txt")]) == EDIT)
}

/// ADR-0045 §5, §7 at the app: killed before each of steps 1–6 and launched
/// again, the edit is in the tree or in the stash list, and the relaunch
/// line names `stash@{n}` whenever it is in the list, saying whether the
/// edits are back or only there (30c).
#[test]
fn edits_survive_a_kill_at_every_step() {
    for step in 1..=STEPS.len() {
        let what = format!("killed before step {step} ({})", STEPS[step - 1]);
        let scratch = dirty();
        let path = scratch.path();
        let op = dot_enter_y(path);
        run_and_kill(path, &op, step);

        let (app, state) = App::open(path);
        let state = launched(&app, state);
        let state = app.pump(state, "the refs the relaunch reads", |state: &AppState| {
            state.refs.is_some()
        });

        let in_tree =
            std::fs::read_to_string(path.join("base.txt")).is_ok_and(|bytes| bytes == EDIT);
        let listed = edit_in_the_list(path);
        assert!(
            in_tree || listed.is_some(),
            "{what}: the edit is in neither the tree nor the stash list"
        );
        let notice = mergewright::update::undo::interrupted_notice(&state);
        if let Some(at) = listed {
            // This launch closed the entry, so recovery read where the edits
            // are (30c): back with a copy listed, or only in the list.
            let clause = if in_tree {
                format!("your edits are back \u{b7} a copy stays in stash@{{{at}}}")
            } else {
                format!("your edits are in stash@{{{at}}} \u{b7} S p puts them back")
            };
            assert!(
                notice
                    .as_deref()
                    .is_some_and(|words| words.contains(&clause)),
                "{what}: the edit is in stash@{{{at}}}, the relaunch line says {notice:?}"
            );
        }
        eprintln!("{what}: tree {in_tree}, list {listed:?}, relaunch line {notice:?}");
        drop(state);
        app.close();
    }
}
