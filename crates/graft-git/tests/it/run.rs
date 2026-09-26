//! The typed git runner and its first reader, `contained_in` (m2/01), against
//! corpus fixtures and the real system `git`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::contained::{Contained, contained_in};
use graft_git::repo::{HeadState, RefTarget, RepoHandle};
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

fn rev_parse(repo: &RepoHandle, revision: &str) -> Oid {
    let output = GitInvocation::new(repo)
        .args([GitArg::Flag("rev-parse"), GitArg::Word(revision.to_owned())])
        .run(&())
        .unwrap_or_else(|why| panic!("rev-parse {revision}: {why}"));
    let hex = String::from_utf8_lossy(&output.stdout);
    Oid::from_hex(hex.trim()).unwrap_or_else(|why| panic!("{hex:?} is not an id: {why}"))
}

/// Says stop from its second poll on: `run`'s own poll before it spawns
/// says go, the first poll after the spawn — microseconds later, before git
/// has finished starting — says stop.
struct SecondPoll(AtomicUsize);

impl Stop for SecondPoll {
    fn should_stop(&self) -> bool {
        self.0.fetch_add(1, Ordering::Relaxed) >= 1
    }
}

fn log_everything(repo: &RepoHandle) -> GitInvocation {
    GitInvocation::new(repo).args([
        GitArg::Flag("log"),
        GitArg::Flag("-p"),
        GitArg::Flag("--all"),
        GitArg::Flag("--no-color"),
    ])
}

#[test]
fn rev_parse_head_agrees_with_the_repo_handle() {
    let repo = open(&graft_fixtures::borrow(Shape::Linear));
    let output = GitInvocation::new(&repo)
        .arg(GitArg::Flag("rev-parse"))
        .arg(GitArg::Flag("HEAD"))
        .run(&())
        .expect("rev-parse runs");
    let printed = Oid::from_hex(String::from_utf8(output.stdout).expect("hex").trim()).expect("id");
    let HeadState::Branch(branch) = repo.head().expect("head reads") else {
        panic!("Linear is on a branch");
    };
    let tip = repo
        .references()
        .expect("refs read")
        .into_iter()
        .find(|entry| entry.name == branch)
        .expect("the branch exists");
    assert_eq!(tip.target, RefTarget::Object(printed));
    assert!(!output.truncated);
}

#[test]
fn a_failing_command_reports_status_first_stderr_line_and_itself() {
    let repo = open(&graft_fixtures::borrow(Shape::Linear));
    let error = GitInvocation::new(&repo)
        .args([
            GitArg::Flag("rev-parse"),
            GitArg::Flag("--verify"),
            GitArg::Word("nonesuch".to_owned()),
        ])
        .run(&())
        .expect_err("nonesuch is no revision");
    let RunError::Exited {
        status,
        ref stderr_first_line,
        ref command,
    } = error
    else {
        panic!("expected Exited, got {error:?}");
    };
    assert_eq!(status, 128);
    assert!(
        stderr_first_line.starts_with("fatal: "),
        "{stderr_first_line}"
    );
    assert!(command.contains("rev-parse --verify nonesuch"), "{command}");
    assert!(!error.to_string().contains('\n'));
}

#[test]
fn a_stop_ends_a_long_log() {
    let repo = open(&graft_fixtures::borrow(Shape::Linear));
    let stop = SecondPoll(AtomicUsize::new(0));
    let result = log_everything(&repo).run(&stop);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    // That the child is reaped is checked in `run`'s unit tests, which can
    // see the child.
}

#[test]
fn an_atomic_bool_already_set_stops_before_any_output() {
    let repo = open(&graft_fixtures::borrow(Shape::Linear));
    let result = log_everything(&repo).run(&AtomicBool::new(true));
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
}

/// Answers `false` for its first `quiet` polls; every later poll waits
/// `LATE` first, then says stop. The wait gives a short git time to finish
/// (or to do its write) before the answer lands (m7-power-tools/27).
struct StopsLate {
    polls: AtomicUsize,
    quiet: usize,
}

const LATE: std::time::Duration = std::time::Duration::from_millis(500);

impl Stop for StopsLate {
    fn should_stop(&self) -> bool {
        if self.polls.fetch_add(1, Ordering::Relaxed) < self.quiet {
            return false;
        }
        std::thread::sleep(LATE);
        true
    }
}

fn update_ref_made() -> [GitArg; 3] {
    [
        GitArg::Flag("update-ref"),
        GitArg::Ref(b"refs/heads/made".to_vec()),
        GitArg::Flag("HEAD"),
    ]
}

fn has_made(dir: &Path) -> bool {
    std::process::Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["show-ref", "--verify", "--quiet", "refs/heads/made"])
        .status()
        .unwrap_or_else(|why| panic!("git show-ref: {why}"))
        .success()
}

/// Race 1a: a stop that lands after git has already exited is not a stop —
/// the command ran to the end, and a write it made stands. Reporting
/// `Stopped` here tells the caller a done write did not happen.
#[test]
fn a_stop_after_git_has_exited_reports_the_exit() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    let repo = open(scratch.path());
    let stop = StopsLate {
        polls: AtomicUsize::new(0),
        quiet: 1,
    };
    let result = GitInvocation::new(&repo).args(update_ref_made()).run(&stop);
    assert!(has_made(scratch.path()), "git did the write");
    assert!(result.is_ok(), "{result:?}");

    let stop = StopsLate {
        polls: AtomicUsize::new(0),
        quiet: 1,
    };
    let raw = GitInvocation::new(&repo)
        .args([GitArg::Flag("rev-parse"), GitArg::Flag("HEAD")])
        .run_raw(&stop)
        .unwrap_or_else(|why| panic!("run_raw finishes too: {why}"));
    assert_eq!(raw.code, Some(0));
    assert_eq!(
        String::from_utf8_lossy(&raw.output.stdout).trim(),
        rev_parse(&repo, "HEAD").to_string()
    );
}

/// Race 1b: a stop already asked before `run` is called spawns nothing, so a
/// write command never gets to write.
#[test]
fn a_stop_asked_before_run_spawns_nothing() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
    let repo = open(scratch.path());
    let stop = StopsLate {
        polls: AtomicUsize::new(0),
        quiet: 0,
    };
    let result = GitInvocation::new(&repo).args(update_ref_made()).run(&stop);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(!has_made(scratch.path()), "git never ran");

    let stop = StopsLate {
        polls: AtomicUsize::new(0),
        quiet: 0,
    };
    let result = GitInvocation::new(&repo)
        .args(update_ref_made())
        .run_raw(&stop);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(!has_made(scratch.path()), "git never ran");
}

#[test]
fn output_past_the_cap_is_cut_and_marked_truncated() {
    let repo = open(&graft_fixtures::borrow(Shape::Linear));
    let output = log_everything(&repo)
        .output_cap(1024)
        .run(&())
        .expect("a capped read is still Ok");
    assert!(output.truncated);
    assert!(
        output.stdout.len() < 1024 + 65_536,
        "{}",
        output.stdout.len()
    );
    assert!(output.stdout.starts_with(b"commit "));
}

#[test]
fn stdin_reaches_git() {
    let repo = open(&graft_fixtures::borrow(Shape::Linear));
    let output = GitInvocation::new(&repo)
        .args([GitArg::Flag("hash-object"), GitArg::Flag("--stdin")])
        .stdin(b"hello\n".to_vec())
        .run(&())
        .expect("hash-object runs");
    // `printf 'hello\n' | git hash-object --stdin`
    assert_eq!(output.stdout, b"ce013625030ba8dba906f756967f9e9ca394464a\n");
}

#[test]
fn a_non_utf8_path_reaches_git_byte_for_byte() {
    let repo = open(&graft_fixtures::borrow(Shape::NonUtf8Path));
    // The Latin-1 path is only in the `latin1` branch's tree (never checked
    // out), so `ls-tree` echoes it rather than `ls-files`.
    let output = GitInvocation::new(&repo)
        .args([
            GitArg::Flag("ls-tree"),
            GitArg::Flag("-z"),
            GitArg::Flag("--name-only"),
            GitArg::Ref(b"refs/heads/latin1".to_vec()),
            GitArg::Path(b"caf\xe9.txt".to_vec()),
        ])
        .run(&())
        .expect("ls-tree runs");
    assert_eq!(output.stdout, b"caf\xe9.txt\0");
}

#[test]
fn the_tip_of_main_is_in_main_and_its_remote_and_in_no_tag() {
    let repo = open(&graft_fixtures::borrow(Shape::RefStates));
    let contained = contained_in(&repo, rev_parse(&repo, "refs/heads/main"), &()).expect("runs");
    assert_eq!(contained.branches, vec![b"main".to_vec()]);
    assert_eq!(contained.remotes, vec![b"origin/main".to_vec()]);
    assert!(contained.tags.is_empty());
    assert_eq!(contained.more, 0);
}

#[test]
fn a_commit_only_a_tag_reaches_lists_that_tag() {
    let repo = open(&graft_fixtures::borrow(Shape::RefStates));
    let contained = contained_in(&repo, rev_parse(&repo, "refs/tags/archived"), &()).expect("runs");
    assert_eq!(
        contained,
        Contained {
            branches: vec![],
            remotes: vec![],
            tags: vec![b"archived".to_vec()],
            more: 0,
        }
    );
}

#[test]
fn an_unborn_head_does_not_break_contained_in() {
    let repo = open(&graft_fixtures::borrow(Shape::UnbornBranch));
    let root = rev_parse(&repo, "refs/heads/other~1");
    let contained = contained_in(&repo, root, &()).expect("runs");
    assert_eq!(
        contained.branches,
        vec![b"main".to_vec(), b"other".to_vec()]
    );
    assert!(contained.remotes.is_empty() && contained.tags.is_empty());
}
