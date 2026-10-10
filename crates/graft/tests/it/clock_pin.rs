//! `TRIB_NOW` pins the binary's drawn clock (m8-polish-and-ship/19d-b):
//! the ages on screen, for the whole run. It never reaches the journal: the
//! binary stamps every entry from the wall clock (19e §R1), so a stray pin
//! in his shell writes no false history. A value that is not whole seconds
//! refuses to launch.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::Path;

use crate::pty::{Isolated, Pty, Screen};

const SIZE: (u16, u16) = (120, 30);

/// 2024-01-01 and 2025-01-01, UTC: a year apart. Against the fixtures'
/// `FIXED_DATE` (2021-01-01) the label rounds them to `4y` and `5y`.
const EARLIER: i64 = 1_704_067_200;
const A_YEAR_LATER: i64 = 1_735_689_600;

/// Launch on `repo` with `TRIB_NOW` = `now`.
fn launch(isolated: &Isolated, repo: &Path, now: &str) -> Pty {
    Pty::spawn_with_env(
        isolated,
        repo,
        repo.parent().expect("a temp repo has a parent"),
        SIZE,
        &[("TRIB_NOW", now.into())],
    )
}

/// Whether any word on screen is exactly `age` (`3y` but not `13y`).
fn draws_age(screen: &Screen, age: &str) -> bool {
    screen.lines().iter().any(|line| {
        line.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == age)
    })
}

fn age_drawn_at(now: i64, age: &str) {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let isolated = Isolated::new();
    let mut pty = launch(&isolated, scratch.path(), &now.to_string());
    let screen = pty.wait_for(&format!("a commit row aged {age}"), |s| draws_age(s, age));
    for other in ["3y", "4y", "5y", "6y"].into_iter().filter(|o| *o != age) {
        assert!(!draws_age(&screen, other), "{other}: {}", screen.text());
    }
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// The same repository, two pins a year apart: two different ages on the
/// same commit row.
#[test]
fn graft_now_pins_the_drawn_age() {
    age_drawn_at(EARLIER, "4y");
    age_drawn_at(A_YEAR_LATER, "5y");
}

/// The old name is not read (m8-polish-and-ship/27b): a value `TRIB_NOW`
/// would refuse, set under the pre-rename name, launches normally. The name
/// is spelt in two halves so `rename_names::no_old_env_name` stays exact.
#[test]
fn the_old_now_name_is_not_read() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let isolated = Isolated::new();
    let repo = scratch.path();
    let mut pty = Pty::spawn_with_env(
        &isolated,
        repo,
        repo.parent().expect("a temp repo has a parent"),
        SIZE,
        &[(concat!("GRAFT", "_NOW"), "1.5".into())],
    );
    pty.wait_for("the app drew", |s| !s.text().trim().is_empty());
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// Not whole seconds: exit 2, the words on stderr, and the terminal never
/// taken (no alternate screen).
#[test]
fn a_bad_graft_now_refuses_to_launch() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let isolated = Isolated::new();
    let pty = launch(&isolated, scratch.path(), "1.5");
    let (status, stderr, output) = pty.wait_exit();
    assert_eq!(status.code(), Some(2), "stderr: {stderr}");
    assert_eq!(
        stderr,
        "trib: TRIB_NOW must be whole seconds since 1970, got 1.5\n"
    );
    assert_eq!(
        crate::pty::count(&output, b"\x1b[?1049h"),
        0,
        "no screen: {:?}",
        String::from_utf8_lossy(&output)
    );
}

/// One round on the graph: `k` to the changes row, `⏎`, `space` stages the
/// top file of `left` unstaged, `Esc`, then `c`, `message`, `⏎` commits it
/// (as the tour's tasks 1-2 do). Back on the graph with the commit on top.
fn stage_and_commit(pty: &mut Pty, left: usize, message: &str) {
    pty.send(b"k");
    pty.wait_for("the cursor on the changes row", |s| {
        s.lines()
            .iter()
            .any(|l| l.contains('\u{25b8}') && l.contains("Uncommitted changes"))
    });
    pty.send(b"\r");
    let unstaged = format!("UNSTAGED ({left})");
    pty.wait_for("the changes pane", |s| {
        s.has("space stage") && s.has(&unstaged)
    });
    pty.send(b" ");
    pty.wait_for("one staged", |s| s.has("STAGED (1)") && !s.has(&unstaged));
    pty.send(b"\x1b");
    pty.wait_for("the graph's own keys", |s| {
        s.has("\u{23ce} changes \u{b7} c commit")
    });
    pty.send(b"c");
    pty.wait_for("the commit editor", |s| s.has("\u{23ce} commit"));
    pty.send(message.as_bytes());
    pty.send(b"\r");
    let tip = format!("main] {message}");
    pty.wait_for(&format!("{message} the tip of main"), |s| s.has(&tip));
}

/// This process's wall clock in milliseconds: the bounds a run's journal
/// stamps must fall within.
fn wall_ms() -> i64 {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the test machine's clock reads after 1970");
    i64::try_from(since.as_millis()).expect("milliseconds fit an i64")
}

/// Every entry stamped by the wall clock while the run lasted, none at the
/// pin: `TRIB_NOW` drew the ages and wrote nothing (19e §R1).
fn stamped_by_the_wall(entries: &[graft_core::journal::JournalEntry], run: (i64, i64), pin: i64) {
    let (from, to) = run;
    for entry in entries {
        assert!(
            (from..=to).contains(&entry.at_unix_ms),
            "{} not within the run's wall clock {from}..={to}: {entry:?}",
            entry.at_unix_ms
        );
        assert_ne!(
            entry.at_unix_ms,
            pin * 1_000,
            "stamped at the pin: {entry:?}"
        );
    }
}

/// m8/19e §R1: under the default pin, one in-app commit, then quit. Every
/// journal entry the run wrote is stamped from the wall clock between the
/// launch and the exit; none at `PINNED_NOW`.
#[test]
fn graft_now_never_stamps_the_journal() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    // The app commits: under the isolated `HOME` only the repo can name
    // who (a Linux host's implicit identity is refused; m8/30).
    crate::app_harness::pin_identity(repo);
    let isolated = Isolated::new();
    let handle = graft_git::repo::RepoHandle::open(repo).expect("repo opens");
    let path = graft_ops::journal::journal_path(&handle).expect("journal path");
    assert!(
        !path.exists(),
        "a fresh copy has no journal: every entry is new"
    );
    std::fs::write(repo.join("README.md"), "edited\n").expect("the scratch repo is writable");

    let from = wall_ms();
    let mut pty = launch(&isolated, repo, &graft_fixtures::PINNED_NOW.to_string());
    pty.wait_for("the graph", |s| s.has("Uncommitted changes"));
    stage_and_commit(&mut pty, 1, "Made under the pin");
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    let to = wall_ms();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");

    let entries = journal(repo);
    // Stage, commit.
    assert_eq!(entries.len(), 2, "{entries:?}");
    stamped_by_the_wall(&entries, (from, to), graft_fixtures::PINNED_NOW);
}

/// Two commits under a pinned drawn clock, and `u` takes the newest: it
/// undoes the second, not the first. The pin draws the ages only; the
/// entries are stamped from the wall clock (19e §R1).
#[test]
fn u_undoes_the_newest_under_a_pinned_drawn_clock() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    crate::app_harness::pin_identity(repo);
    let isolated = Isolated::new();
    // Both files before launch: the app reads them at start.
    std::fs::write(repo.join("README.md"), "edited\n").expect("the scratch repo is writable");
    std::fs::write(repo.join("notes.txt"), "edited\n").expect("the scratch repo is writable");
    let from = wall_ms();
    let mut pty = launch(&isolated, repo, &EARLIER.to_string());
    pty.wait_for("the graph", |s| {
        draws_age(s, "4y") && s.has("Uncommitted changes")
    });

    for (left, message) in [(2, "First pinned"), (1, "Second pinned")] {
        stage_and_commit(&mut pty, left, message);
    }

    let entries = journal(repo);
    // Stage, commit, stage, commit: four entries, each on the wall clock.
    assert_eq!(entries.len(), 4, "{entries:?}");
    stamped_by_the_wall(&entries, (from, wall_ms()), EARLIER);

    pty.send(b"u");
    pty.wait_for("the second commit undone", |s| {
        s.has("main] First pinned") && !s.has("main] Second pinned")
    });
    let head = git_out(repo, &["log", "-1", "--format=%s"]);
    assert_eq!(head.trim(), "First pinned");

    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

fn journal(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    let handle = graft_git::repo::RepoHandle::open(repo).expect("repo opens");
    let path = graft_ops::journal::journal_path(&handle).expect("journal path");
    let bytes = std::fs::read(&path).expect("the commits wrote a journal");
    graft_ops::journal::parse(&bytes)
        .expect("journal parses")
        .entries
}

fn git_out(repo: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).expect("utf-8")
}

/// m8/19d-b2: `App::open_with_clock` pins the frame, not only the journal:
/// one commit row, two clocks a year apart, two different ages.
#[test]
fn open_with_clock_pins_the_drawn_frame() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    for (now, age) in [(EARLIER, "4y"), (A_YEAR_LATER, "5y")] {
        let (app, state) = crate::app_harness::App::open_with_clock(
            scratch.path(),
            graft::session::Clock::Fixed(now),
        );
        let state = app.pump(state, "a commit row with its age", |state| {
            crate::captures::frame(state, 120, 40)
                .iter()
                .any(|line| line.contains("main]"))
        });
        let state = crate::captures::settled_frame(&app, state);
        let frame = crate::captures::frame(&state, 120, 40);
        let drawn = |age: &str| {
            frame.iter().any(|line| {
                line.split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|word| word == age)
            })
        };
        assert!(drawn(age), "{age} at {now}:\n{}", frame.join("\n"));
        for other in ["3y", "4y", "5y", "6y"].into_iter().filter(|o| *o != age) {
            assert!(!drawn(other), "{other} at {now}:\n{}", frame.join("\n"));
        }
        app.close();
    }
}

/// m8/19d-b2, ticket 19d §R5: with no date env, the pty harness dates an
/// in-app commit an hour before the default pin, so its row reads `60m`
/// (the age label's words for 3600 s; the ticket guessed `1h`), never
/// `future`.
#[test]
fn an_in_app_commit_under_the_default_pin_draws_an_age() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    crate::app_harness::pin_identity(repo);
    let isolated = Isolated::new();
    std::fs::write(repo.join("README.md"), "edited\n").expect("the scratch repo is writable");
    let mut pty = Pty::spawn(
        &isolated,
        repo,
        repo.parent().expect("a temp repo has a parent"),
        SIZE,
    );
    pty.wait_for("the graph", |s| s.has("Uncommitted changes"));
    pty.send(b"k");
    pty.wait_for("the cursor on the changes row", |s| {
        s.lines()
            .iter()
            .any(|l| l.contains('\u{25b8}') && l.contains("Uncommitted changes"))
    });
    pty.send(b"\r");
    pty.wait_for("the changes pane", |s| {
        s.has("space stage") && s.has("UNSTAGED (1)")
    });
    pty.send(b" ");
    pty.wait_for("one staged", |s| {
        s.has("STAGED (1)") && !s.has("UNSTAGED (1)")
    });
    pty.send(b"\x1b");
    pty.wait_for("the graph's own keys", |s| {
        s.has("\u{23ce} changes \u{b7} c commit")
    });
    pty.send(b"c");
    pty.wait_for("the commit editor", |s| s.has("\u{23ce} commit"));
    pty.send(b"Made in the app");
    pty.send(b"\r");
    let screen = pty.wait_for("the new commit's row", |s| s.has("main] Made in the app"));
    let row = screen
        .lines()
        .iter()
        .find(|l| l.contains("main] Made in the app"))
        .cloned()
        .expect("the row was waited for");
    assert!(
        row.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == "60m"),
        "{row}\n{}",
        screen.text()
    );
    assert!(!screen.has("future"), "{}", screen.text());
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// What a capture producer must not say: a wall clock, or a `Harness` /
/// left on its wall-clock default (m8/19d-b2, ticket 19d §R4).
fn wall_clock_lines(source: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        // A path in front (`..crate::update::Harness::default()`) is the
        // same call.
        let code = line
            .trim_start()
            .replace("crate::update::", "")
            .replace("graft::update::", "");
        if code.starts_with("//") {
            continue;
        }
        let bare_default =
            code.contains("Harness::default()") && !code.contains("..Harness::default()");
        // `..Harness::default()` is fine only when the literal it closes
        // names `clock:` itself: look back to the `Harness {` that opens it.
        let unpinned_update = code.contains("..Harness::default()")
            && !lines[..index]
                .iter()
                .rev()
                .take_while(|above| !above.contains("Harness {"))
                .any(|above| above.trim_start().starts_with("clock:"));
        if code.contains("Clock::Wall") || bare_default || unpinned_update {
            found.push((index + 1, (*line).to_owned()));
        }
    }
    found
}

/// Whether a source file writes a capture under `docs/shots/`.
fn writes_a_capture(source: &str) -> bool {
    [
        "check_shot(",
        "Captures::",
        "graft_fixtures::check(",
        "graft_fixtures::check_bytes(",
        "graft_fixtures::capture(",
        "graft_fixtures::capture_bytes(",
        "docs/shots/",
    ]
    .iter()
    .any(|call| source.contains(call))
}

fn rust_files(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source dir reads") {
        let path = entry.expect("a dir entry reads").path();
        if path.is_dir() {
            rust_files(&path, into);
        } else if path.extension().is_some_and(|e| e == "rs") {
            into.push(path);
        }
    }
}

/// The scan bites on each shape it names, and passes a pinned one.
#[test]
fn the_wall_clock_scan_names_each_unpinned_shape() {
    for bad in [
        "let c = Clock::Wall;",
        "harness: Harness::default(),",
        "Harness {\n    prelude,\n    ..Harness::default()\n}",
        "harness: crate::update::Harness::default(),",
    ] {
        assert!(!wall_clock_lines(bad).is_empty(), "{bad}");
    }
    let pinned = "Harness {\n    clock: Clock::Fixed(graft_fixtures::PINNED_NOW),\n    ..Harness::default()\n}\n// Clock::Wall in a comment";
    assert!(wall_clock_lines(pinned).is_empty());
}

/// Every source file that writes a capture draws on a pinned clock, so no
/// capture rolls with the calendar (m8/19d-b2, ticket 19d §R4).
#[test]
fn no_capture_producer_runs_on_the_wall_clock() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["tests", "examples", "src"] {
        rust_files(&root.join(dir), &mut files);
    }
    let this = root.join("tests/it/clock_pin.rs");
    let mut offenders = Vec::new();
    for file in files.iter().filter(|f| **f != this) {
        let source = std::fs::read_to_string(file).expect("a source file reads");
        if !writes_a_capture(&source) {
            continue;
        }
        for (line, text) in wall_clock_lines(&source) {
            let shown = file.strip_prefix(root).unwrap_or(file).display();
            offenders.push(format!("{shown}:{line}: {}", text.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "{} capture producer line(s) on the wall clock (pin with graft_fixtures::PINNED_NOW):\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
