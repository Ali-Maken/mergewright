//! The move run in a child of this test binary with a `git` shim first on
//! its `PATH`, the parent's environment never changed (the pattern of
//! `merge_set_aside`'s spawn count and `set_aside_kill`). The shim logs every
//! argv, and on the first `update-ref --stdin` — the create transaction —
//! runs it and then either kills the child or moves one of the refs it just
//! made, so the read-back sees something else.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use graft_git::repo::RepoHandle;
use graft_ops::rename::settle_names;

use crate::support::TempRepo;

const SHIM: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$RENAME_LOG"
case " $* " in
*" update-ref --stdin "*)
    if [ -n "$RENAME_AFTER" ] && [ ! -e "$RENAME_LOG.hit" ]; then
        : > "$RENAME_LOG.hit"
        "$RENAME_REAL_GIT" "$@" || exit $?
        case "$RENAME_AFTER" in
        kill)
            kill -9 "$PPID"
            exit 1
            ;;
        move)
            "$RENAME_REAL_GIT" -C "$RENAME_CHILD_REPO" update-ref "$RENAME_MOVE_REF" "$RENAME_MOVE_TO"
            exit $?
            ;;
        esac
    fi
    ;;
esac
exec "$RENAME_REAL_GIT" "$@"
"#;

/// Written into the log by the child between two settles.
const MARK: &str = "-- the next open --";

/// What the shim does after the create transaction.
pub(super) enum After<'a> {
    /// Nothing: log only.
    Nothing,
    /// SIGKILL the child, as a crash would.
    Kill,
    /// Point this ref at this value.
    Move(&'a str, &'a str),
}

/// The real git, found on this process's `PATH` as `Command` would.
fn real_git() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git is on PATH")
}

/// Run [`child_settles`] on `repo`, settling it `settles` times. Each
/// settle's outcome, `Debug`-printed, one per line — none for a settle the
/// child did not live through — and the log of every git argv it ran.
fn run_child(repo: &TempRepo, after: &After<'_>, settles: usize) -> (Vec<String>, String) {
    let bin = repo.path().join(".git/rename-shim");
    std::fs::create_dir_all(&bin).expect("the shim dir is made");
    let script = bin.join("git");
    std::fs::write(&script, SHIM).expect("the shim writes");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the shim is executable");
    let log = bin.join("log");
    let out = bin.join("outcomes");
    let path = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH is set"),
    )))
    .expect("PATH joins");
    let mut child = std::process::Command::new(std::env::current_exe().expect("the test binary"));
    child
        .args([
            "--exact",
            "rename::shim::child_settles",
            "--ignored",
            "--test-threads=1",
        ])
        .env("PATH", path)
        .env("RENAME_REAL_GIT", real_git())
        .env("RENAME_LOG", &log)
        .env("RENAME_OUT", &out)
        .env("RENAME_SETTLES", settles.to_string())
        .env("RENAME_CHILD_REPO", repo.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    match after {
        After::Nothing => {}
        After::Kill => {
            child.env("RENAME_AFTER", "kill");
        }
        After::Move(name, oid) => {
            child
                .env("RENAME_AFTER", "move")
                .env("RENAME_MOVE_REF", name)
                .env("RENAME_MOVE_TO", oid);
        }
    }
    let status = child.status().expect("the child runs");
    let killed = matches!(after, After::Kill);
    assert_eq!(status.success(), !killed, "the child: {status:?}");
    let outcomes = match std::fs::read_to_string(&out) {
        Ok(text) => text.lines().map(str::to_owned).collect(),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(why) => panic!("the outcomes read: {why}"),
    };
    let logged = std::fs::read_to_string(&log).expect("the log reads");
    std::fs::remove_dir_all(&bin).expect("the shim goes");
    (outcomes, logged)
}

/// Settle `repo` in a child, `after` done by the shim on the create.
pub(super) fn settle_in_child(repo: &TempRepo, after: &After<'_>, settles: usize) -> Vec<String> {
    run_child(repo, after, settles).0
}

/// Settle twice in one child: both outcomes, and the git subcommands the
/// second settle spawned.
pub(super) fn settle_twice_counting(repo: &TempRepo) -> (Vec<String>, Vec<String>) {
    let (outcomes, logged) = run_child(repo, &After::Nothing, 2);
    let (first, second) = logged
        .split_once(MARK)
        .expect("the child marked the second open");
    assert!(!first.trim().is_empty(), "the first open spawned git");
    let spawned = second
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    (outcomes, spawned)
}

/// The child's half: settle the repository `RENAME_SETTLES` times, writing
/// each outcome as soon as it is known.
#[test]
#[ignore = "run by `run_child` in a child process behind the git shim"]
fn child_settles() {
    let var = |name: &str| std::env::var_os(name).unwrap_or_else(|| panic!("{name} is set"));
    let repo = PathBuf::from(var("RENAME_CHILD_REPO"));
    let out = PathBuf::from(var("RENAME_OUT"));
    let log = PathBuf::from(var("RENAME_LOG"));
    let settles: usize = var("RENAME_SETTLES")
        .to_str()
        .and_then(|text| text.parse().ok())
        .expect("a count");
    let mut outcomes = String::new();
    for n in 0..settles {
        if n > 0 {
            append(&log, &format!("{MARK}\n"));
        }
        let handle = RepoHandle::open(&repo).expect("the repository opens");
        outcomes.push_str(&format!("{:?}\n", settle_names(&handle).outcome));
        std::fs::write(&out, &outcomes).expect("the outcomes write");
    }
}

fn append(path: &Path, text: &str) {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .expect("the log appends");
}
