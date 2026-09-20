//! `f`, `p`, `P` and `Esc` over the real job pool, real `git` and real
//! remotes (m4-remotes/05).
//!
//! The milestone's promise is *you never leave Graft to fetch, pull or push —
//! not for a rejection or a hang*. Each test presses the keys through
//! `update`, runs the jobs on the real pool against a `file://` bare
//! (`LocalRemote`) or a remote that never answers (`HungRemote`), and reads
//! the outcome back from the repositories with `git` — never from
//! `AppState` for a repository fact.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;
use std::time::{Duration, Instant};

use graft::update::detail::Loading;
use graft::update::{AppState, Status};
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_fixtures::Shape;
use graft_fixtures::fifo::Fifo;
use graft_fixtures::remote::{HungRemote, LocalRemote};

use crate::app_harness::{App, git};

fn local() -> LocalRemote {
    let remote =
        LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote");
    // Graft's own git keeps the user's config; a global `core.hooksPath`
    // would hide the hook `esc_cancels_local_write` installs.
    run(
        &remote,
        remote.work(),
        &["config", "core.hooksPath", ".git/hooks"],
    );
    // Graft's own commit needs an identity, and its write takes no `-c`.
    run(
        &remote,
        remote.work(),
        &["config", "user.name", "Graft Test"],
    );
    run(
        &remote,
        remote.work(),
        &["config", "user.email", "test@example.invalid"],
    );
    remote
}

/// Hermetic `git` in `dir` (the fixture's own runner), trimmed stdout.
fn run(remote: &LocalRemote, dir: &Path, args: &[&str]) -> String {
    remote
        .git(dir, args)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"))
}

/// `chmod +x`, without spawning one: `PermissionsExt` is the plain way
/// (m4-remotes/05 review, Minor 10).
fn executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .expect("the hook was just written")
        .permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    std::fs::set_permissions(path, permissions).expect("the hook can be made executable");
}

fn rev(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name]).trim().to_owned()
}

fn main_ref(remote: &LocalRemote) -> String {
    format!("refs/heads/{}", remote.default_branch())
}

fn tracking(remote: &LocalRemote) -> String {
    format!("refs/remotes/origin/{}", remote.default_branch())
}

fn journal(repo: &Path) -> Vec<JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the writes made is readable")
        .entries()
        .to_vec()
}

fn plain(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

/// Everything `f`/`p`/`P` decide from has landed: rows, refs, the working
/// tree, and the remote facts with no re-read in flight.
fn ready(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows, refs and remote facts", |state| {
        state.working_tree.is_some()
            && state.refs.is_some()
            && matches!(state.remotes, Loading::Ready(_))
            && state.remote.reading.is_none()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// Runs the pending write to its answer, whatever it was.
fn finish(app: &App, state: AppState, what: &str) -> AppState {
    assert!(
        state.operating.is_some(),
        "{what}: queued; status {:?}",
        state.status
    );
    app.pump(state, what, |state| state.operating.is_none())
}

/// The box `P` opened: `→` off `Cancel`, `Enter`.
fn confirm(app: &App, state: &AppState) -> AppState {
    assert!(
        state.confirm.is_some(),
        "P opened no box; status {:?}",
        state.status
    );
    let state = app.key(state, KeyCode::Right);
    app.key(&state, KeyCode::Enter)
}

/// `f` after a teammate pushed: the tracking ref moves to their tip, the
/// badge reads one behind, the line says `fetched`, the journal has it.
#[test]
fn fetch_key_moves_tracking_ref() {
    let remote = local();
    let work = remote.work();
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let before = rev(work, &tracking(&remote));
    assert_ne!(before, theirs);

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('f'));
    let state = app.settle_write(state, "the fetch");
    assert_eq!(state.status, Status::Done("fetched".to_owned()));
    assert_eq!(
        rev(work, &tracking(&remote)),
        theirs,
        "the tracking ref moved"
    );

    // The badge comes from the refs product the fetch's refresh asked for.
    let branch = remote.default_branch().as_bytes().to_vec();
    let state = app.pump(state, "the refreshed badge", |state| {
        state.refs.as_deref().is_some_and(|refs| {
            refs.branches().any(|(_, name, current, upstream)| {
                current
                    && name == branch.as_slice()
                    && matches!(upstream, graft::refs::Upstream::Apart(counts) if counts.ahead == 0 && counts.behind == 1)
            })
        })
    });
    drop(state);

    let fetches: Vec<_> = journal(work)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Fetch)
        .collect();
    assert_eq!(fetches.len(), 1, "{fetches:?}");
    assert_eq!(fetches[0].status, EntryStatus::Committed);
    app.close();
}

/// `p` fast-forwards `main` to the teammate's tip, bytes and all; `u` puts
/// it back and leaves the tracking ref where the fetch put it; `U` redoes.
#[test]
fn pull_then_undo_redo() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    let old = rev(work, &main);
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let changed = git(remote.bare(), &["diff", "--name-only", &old, &theirs]);
    let file = changed
        .lines()
        .next()
        .expect("their commit changed a file")
        .to_owned();
    let their_bytes = git(remote.bare(), &["show", &format!("{theirs}:{file}")]);
    let old_bytes = std::fs::read(work.join(&file)).ok();

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('p'));
    let state = app.settle_write(state, "the pull");
    assert_eq!(
        state.status,
        Status::Done(format!("pulled origin/{}", remote.default_branch()))
    );
    assert_eq!(rev(work, &main), theirs, "main fast-forwarded");
    assert_eq!(rev(work, &tracking(&remote)), theirs);
    assert_eq!(
        std::fs::read_to_string(work.join(&file)).expect("the pulled file is there"),
        their_bytes
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    assert_eq!(rev(work, &main), old, "u put main back");
    assert_eq!(
        std::fs::read(work.join(&file)).ok(),
        old_bytes,
        "and the bytes"
    );
    assert_eq!(
        rev(work, &tracking(&remote)),
        theirs,
        "the tracking ref is not undo's"
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "the redo");
    drop(state);
    assert_eq!(rev(work, &main), theirs, "U pulled it again");
    app.close();
}

/// Stage a change before the app opens, so its first status read has it.
fn stage(repo: &Path) {
    std::fs::write(repo.join("pushed.txt"), "pushed\n").expect("the work clone is writable");
    git(repo, &["add", "pushed.txt"]);
}

/// Commit what is staged through the commit editor: `c`, a subject, `Tab`
/// to `[ Commit ]`, `Enter`. The commit is journaled, so `u` can take it
/// back.
fn commit_by_key(app: &App, state: &AppState, subject: &str) -> AppState {
    let mut state = app.key(state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    for character in subject.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    for _ in 0..4 {
        if state.commit_editor.as_ref().map(|editor| editor.focus)
            == Some(graft_ui::commit_editor::Field::CommitButton)
        {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    app.key(&state, KeyCode::Enter)
}

/// `P`, `Enter`: the bare's `main` is the local one. `u` then undoes the
/// *commit*, locally — the push is not undoable — and says `origin/main
/// still has it`; the bare does not move. `U` puts the commit back.
#[test]
fn push_then_undo_is_local_only() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    let before = rev(work, &main);
    stage(work);

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = commit_by_key(&app, &state, "to push");
    let state = app.settle_write(state, "the commit");
    let committed = rev(work, &main);
    assert_ne!(committed, before);
    let state = ready(&app, state);

    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    let state = app.settle_write(state, "the push");
    let branch = remote.default_branch();
    assert_eq!(
        state.status,
        Status::Done(format!("pushed {branch} → origin/{branch}"))
    );
    assert_eq!(rev(remote.bare(), &main), committed, "the bare has it");

    // `u`'s wording reads the journal the refresh re-read.
    let state = app.pump(state, "the journal with the push", |state| {
        matches!(&state.journal.entries, Loading::Ready(read)
            if read.entries.iter().any(|entry| entry.kind == OperationKind::Push))
            && state.journal.reading.is_none()
    });
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    let Status::Done(line) = &state.status else {
        unreachable!("settle_write insists on Done");
    };
    assert!(
        line.ends_with(&format!("origin/{branch} still has it")),
        "{line}"
    );
    assert_eq!(rev(work, &main), before, "u undid the commit locally");
    assert_eq!(
        rev(remote.bare(), &main),
        committed,
        "the bare is untouched"
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "the redo");
    drop(state);
    assert_eq!(rev(work, &main), committed, "U put the commit back");
    app.close();
}

/// The words a non-fast-forward rejection leaves.
fn rejected_status(branch: &str) -> Status {
    Status::Sticky {
        reason: format!("push rejected: origin/{branch} has moved"),
        keys: Some("p pull first · P force with lease".to_owned()),
    }
}

/// An amended, already-pushed commit: `P` is rejected in words; the next
/// `P` is the lease box, and it lands.
#[test]
fn rejected_then_lease_push() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    run(&remote, work, &["commit", "--amend", "-q", "-m", "amended"]);
    let amended = rev(work, &main);

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let branch = remote.default_branch();

    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    let state = finish(&app, state, "the rejected push");
    assert_eq!(state.status, rejected_status(branch));
    assert_ne!(rev(remote.bare(), &main), amended);

    let state = ready(&app, state);
    let state = app.press(&state, plain('P'));
    let title = state
        .confirm
        .as_ref()
        .map(|pending| pending.view.title.as_str().to_owned());
    assert_eq!(
        title,
        Some(format!("Force-push {branch} → origin/{branch}?"))
    );
    let state = confirm(&app, &state);
    let state = app.settle_write(state, "the lease push");
    drop(state);
    assert_eq!(rev(remote.bare(), &main), amended, "the lease push landed");
    app.close();
}

/// A teammate pushed and nobody fetched: `P` is rejected, the lease push
/// that follows is refused as stale, and the bare keeps their commit.
#[test]
fn stale_lease_refused() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    remote.commit_locally("ours").expect("a local commit");

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let branch = remote.default_branch();

    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    let state = finish(&app, state, "the rejected push");
    assert_eq!(state.status, rejected_status(branch));

    let state = ready(&app, state);
    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    let state = finish(&app, state, "the stale lease push");
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("a refused lease is a sticky failure: {:?}", state.status);
    };
    assert!(reason.contains("stale lease"), "{reason}");
    assert_eq!(
        rev(remote.bare(), &main),
        theirs,
        "the bare keeps their commit"
    );
    app.close();
}

/// Every `*.lock` under `.git`.
fn locks(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(locks(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}

/// Wait for the hung remote to have git's connection, so `Esc` stops a
/// fetch that is really waiting on the network.
///
/// The fixture's own event: its listener raises a `Condvar` as it accepts.
/// The 30 s is a deadline for a connection that never comes, not a poll
/// interval.
fn wait_for_connection(hung: &HungRemote) {
    assert!(
        hung.wait_for_connection(Duration::from_secs(30)),
        "git never connected to the hung remote"
    );
}

/// `Esc` during `f` against a remote that never answers: the write clears
/// within 5 s, the journal says `cancelled`, no lock is left behind, and
/// the fetch made exactly one connection.
#[test]
fn esc_cancels_hung_fetch() {
    let remote = local();
    let work = remote.work();
    let hung = HungRemote::listen().expect("the hung remote listens");
    run(&remote, work, &["remote", "set-url", "origin", &hung.url()]);

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('f'));
    assert!(state.operating.is_some());
    wait_for_connection(&hung);

    let pressed = Instant::now();
    let state = app.key(&state, KeyCode::Escape);
    assert_eq!(
        state.status,
        Status::Busy("fetch all remotes · cancelling".to_owned())
    );
    let state = finish(&app, state, "the cancelled fetch");
    assert!(
        pressed.elapsed() < Duration::from_secs(5),
        "{:?}",
        pressed.elapsed()
    );
    assert_eq!(
        state.status,
        Status::Done("fetch all remotes cancelled".to_owned())
    );

    let last = journal(work).pop().expect("the fetch was journaled");
    assert_eq!(last.kind, OperationKind::Fetch);
    assert!(
        matches!(&last.status, EntryStatus::Aborted { reason } if reason.contains("cancelled")),
        "{:?}",
        last.status
    );
    assert_eq!(locks(&work.join(".git")), Vec::<std::path::PathBuf>::new());
    assert_eq!(hung.connections(), 1);
    app.close();
}

/// `Esc` during a local write — a commit whose pre-commit hook sleeps —
/// stops it the same way (M3 left every write unstoppable).
#[test]
fn esc_cancels_local_write() {
    let remote = local();
    let work = remote.work();
    let hook = work.join(".git/hooks/pre-commit");
    // The hook announces itself down a FIFO the test is already blocked on
    // reading, so "the hook is inside `git commit`" is an event and not a
    // sleep-and-check loop (CLAUDE.md rule 2).
    let started = Fifo::create().expect("the fifo is made");
    std::fs::write(
        &hook,
        format!(
            "#!/bin/sh\necho started > '{}'\nsleep 5\n",
            started.path().display()
        ),
    )
    .expect("the hooks dir is writable");
    run(&remote, work, &["update-index", "--refresh"]);
    executable(&hook);
    let main = main_ref(&remote);
    let before = rev(work, &main);
    stage(work);

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = commit_by_key(&app, &state, "slow");
    assert!(state.operating.is_some(), "the commit is running");
    // Esc before the job starts would stop nothing that runs; wait for the
    // hook to be inside `git commit`. The 30 s is the deadline for a hook
    // that never runs, not a poll interval.
    assert!(
        started.wait(Duration::from_secs(30)),
        "the pre-commit hook never ran"
    );

    let pressed = Instant::now();
    let state = app.key(&state, KeyCode::Escape);
    let state = finish(&app, state, "the cancelled commit");
    assert!(
        pressed.elapsed() < Duration::from_secs(5),
        "{:?}",
        pressed.elapsed()
    );
    assert_eq!(
        state.status,
        Status::Done("commit \"slow\" cancelled".to_owned())
    );
    assert_eq!(rev(work, &main), before, "no commit landed");

    let last = journal(work).pop().expect("the commit was journaled");
    assert_eq!(last.kind, OperationKind::Commit);
    assert!(
        matches!(last.status, EntryStatus::Aborted { .. }),
        "{:?}",
        last.status
    );
    assert_eq!(locks(&work.join(".git")), Vec::<std::path::PathBuf>::new());
    app.close();
}

/// A fetch with something to bring shows git's own progress on the status
/// line at least once, between `f` and `fetched`.
#[test]
fn progress_reaches_the_status_line() {
    let remote = local();
    let work = remote.work();
    for n in 0..3 {
        remote
            .commit_on_remote(&format!("theirs {n}"))
            .expect("a teammate's push");
    }

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('f'));
    let seen = std::cell::RefCell::new(vec![state.status.clone()]);
    let state = app.pump(state, "the fetch", |state| {
        let mut seen = seen.borrow_mut();
        if seen.last() != Some(&state.status) {
            seen.push(state.status.clone());
        }
        state.operating.is_none()
    });
    let seen = seen.into_inner();
    let progressed = seen.iter().any(|status| {
        matches!(status, Status::Busy(line)
            if line.starts_with("fetch all remotes · ")
                && line.ends_with(" · esc cancels")
                && line != "fetch all remotes · esc cancels")
    });
    assert!(progressed, "no progress line among {seen:?}");
    assert_eq!(state.status, Status::Done("fetched".to_owned()));
    app.close();
}
