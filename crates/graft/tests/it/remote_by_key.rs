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
use graft::update::{AppState, RefsRead, Status};
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_fixtures::Shape;
use graft_fixtures::fifo::Fifo;
use graft_fixtures::remote::{HungRemote, LocalRemote};

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame, without_invented_id};

pub(crate) fn local() -> LocalRemote {
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
pub(crate) fn run(remote: &LocalRemote, dir: &Path, args: &[&str]) -> String {
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

pub(crate) fn rev(repo: &Path, name: &str) -> String {
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

pub(crate) fn plain(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

/// Everything `f`/`p`/`P` decide from has landed: rows, refs, the working
/// tree, and the remote facts with no re-read in flight.
pub(crate) fn ready(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows, refs and remote facts", is_ready)
}

/// [`ready`]'s test, for a caller that waits on more than it does.
pub(crate) fn is_ready(state: &AppState) -> bool {
    state.working_tree.is_some()
        && state.refs.is_some()
        // The refs the last read asked for: after a write's refresh `P`
        // refuses in words until they land (m6-conflicts/49).
        && !matches!(state.refs_read, RefsRead::Pending(_))
        && matches!(state.remotes, Loading::Ready(_))
        && state.remote.reading.is_none()
        // What a lease would take off the remote: `P` says `still reading`
        // until it lands (m7-power-tools/16b).
        && !matches!(
            state.remote.last_rejected_push.as_ref().and_then(|rejected| rejected.leaves.as_ref()),
            Some((_, _, graft::update::remote::LeavesRead::Reading(_)))
        )
        && state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
}

/// Runs the pending write to its answer, whatever it was.
pub(crate) fn finish(app: &App, state: AppState, what: &str) -> AppState {
    assert!(
        state.operating.is_some(),
        "{what}: queued; status {:?}",
        state.status
    );
    app.pump(state, what, |state| state.operating.is_none())
}

/// The box `P` opened: `→` off `Cancel`, `Enter`.
pub(crate) fn confirm(app: &App, state: &AppState) -> AppState {
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
    assert_eq!(state.status, Status::done("fetched"));
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
        Status::done(format!("pulled origin/{}", remote.default_branch()))
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
        Status::done(format!("pushed {branch} → origin/{branch}"))
    );
    assert_eq!(rev(remote.bare(), &main), committed, "the bare has it");

    // `u`'s wording reads the journal the refresh re-read.
    let state = app.pump(state, "the journal with the push", |state| {
        matches!(&state.journal.entries, Loading::Ready(read)
            if read.entries.iter().any(|entry| entry.kind == OperationKind::Push))
            && state.journal.reading.is_none()
    });
    let state = app.key(&state, KeyCode::Char('u'));
    // From M5, `u` on work a push already sent explains itself before it
    // runs (ADR-0038 §4, m5-history-and-ghost-preview/15). `confirm` moves
    // one right — onto `[ Undo locally ]` — and presses Enter.
    assert_eq!(
        state
            .confirm
            .as_ref()
            .map(|pending| pending.view.title.as_str()),
        Some("Undo after a push"),
        "u after a push asked nothing; status {:?}",
        state.status
    );
    let state = confirm(&app, &state);
    let state = app.settle_write(state, "the undo");
    let Status::Done { what: line, .. } = &state.status else {
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

/// **Piece 7, ADR-0038 §4.** `u` on work a push already sent stops and
/// explains itself — once. `[ Undo locally ]` runs the ordinary undo, the
/// bare does not move, and the same push does not ask again: `U` puts the
/// commit back and the next `u` takes it away with no box in between.
#[test]
fn u_after_push_asks_once_then_undoes_locally() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    let before = rev(work, &main);
    stage(work);

    let (app, state) = App::open(work);
    let state = pushed_one_commit(&app, state);
    let committed = rev(work, &main);
    assert_eq!(rev(remote.bare(), &main), committed, "the bare has it");

    // First `u`: the box, and nothing running.
    let state = app.key(&state, KeyCode::Char('u'));
    let view = &state
        .confirm
        .as_ref()
        .unwrap_or_else(|| panic!("u asked nothing; status {:?}", state.status))
        .view;
    assert_eq!(view.title.as_str(), "Undo after a push");
    let choices: Vec<&str> = view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(choices, ["Cancel", "Undo locally", "Re-push old tip"]);
    assert!(state.operating.is_none(), "the box started a write");
    assert_eq!(rev(work, &main), committed, "nothing ran yet");

    // `[ Undo locally ]` is one right of `Cancel`.
    let state = confirm(&app, &state);
    let state = app.settle_write(state, "the undo");
    assert_eq!(rev(work, &main), before, "u undid the commit locally");
    assert_eq!(
        rev(remote.bare(), &main),
        committed,
        "the bare is untouched"
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "the redo");
    assert_eq!(rev(work, &main), committed, "U put the commit back");

    // Asked once: the same push explains itself no further.
    let state = app.key(&state, KeyCode::Char('u'));
    assert!(
        state.confirm.is_none(),
        "the same push asked twice: {:?}",
        state.confirm.as_ref().map(|p| p.view.title.as_str())
    );
    let state = app.settle_write(state, "the second undo");
    drop(state);
    assert_eq!(rev(work, &main), before, "and it undid, with no box");
    app.close();
}

/// **Piece 7, ADR-0038 §4, all three conditions.** `[ Re-push old tip ]`
/// opens its own confirm — the lease box — and only that box's `Enter`
/// moves the remote. It moves it *back*, which nothing but a
/// `--force-with-lease` push can do: without the lease `graft_ops` refuses
/// the source commit outright (`Refusal::RepushNeedsLease`) and the bare
/// stays where the push left it. The local branch keeps its commits.
#[test]
fn u_after_push_repush_old_tip_moves_the_bare_back() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    let before = rev(work, &main);
    stage(work);

    let (app, state) = App::open(work);
    let state = pushed_one_commit(&app, state);
    let committed = rev(work, &main);
    assert_eq!(rev(remote.bare(), &main), committed, "the bare has it");

    // `u`, then `→ →` onto `[ Re-push old tip ]`.
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let pending = state
        .confirm
        .as_ref()
        .unwrap_or_else(|| panic!("no second confirm; status {:?}", state.status));
    let short: String = before.chars().take(7).collect();
    assert_eq!(
        pending.view.title.as_str(),
        format!(
            "Re-push old tip {short} → origin/{}?",
            remote.default_branch()
        )
    );
    assert!(
        pending
            .view
            .command
            .as_str()
            .contains("--force-with-lease="),
        "{:?}",
        pending.view.command
    );
    assert!(state.operating.is_none(), "the lease box started a write");
    assert_eq!(
        rev(remote.bare(), &main),
        committed,
        "the remote moved before its own confirm"
    );

    let state = confirm(&app, &state);
    let state = app.settle_write(state, "the re-push");
    drop(state);
    assert_eq!(
        rev(remote.bare(), &main),
        before,
        "the re-push did not move origin back"
    );
    assert_eq!(
        rev(work, &main),
        committed,
        "the re-push touched the local branch"
    );
    app.close();
}

/// **Gap row 557 (m6-conflicts/06).** The re-push's lease comes from the
/// push's own journal record — the tip that push left on the remote — never
/// from whatever `state.remotes` read last.
///
/// A teammate pushes after ours and the tracking ref is fetched, so
/// `state.remotes` now holds *their* tip. A lease taken from it would match
/// the remote and the re-push would silently throw their commit away. Taken
/// from the entry, it names the tip we pushed, the remote is no longer
/// there, and git refuses in words. The bare keeps their commit.
#[test]
fn repush_lease_comes_from_the_entry() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    stage(work);

    let (app, state) = App::open(work);
    let state = pushed_one_commit(&app, state);
    let ours = rev(work, &main);
    let push = journal(work)
        .into_iter()
        .rfind(|entry| entry.kind == OperationKind::Push)
        .expect("the push is journaled");
    let recorded = push
        .refs_after
        .get(&tracking(&remote))
        .copied()
        .flatten()
        .expect("the push entry records the tracking tip it left");
    assert_eq!(recorded.to_string(), ours, "the entry names our pushed tip");

    let (state, theirs) = their_push_fetched(&app, &state, &remote);

    // `u`, `→ →` onto `[ Re-push old tip ]`, `Enter`: the lease box.
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let pending = state
        .confirm
        .as_ref()
        .unwrap_or_else(|| panic!("no lease box; status {:?}", state.status));
    let graft::update::confirm::ConfirmThen::Operate(op) = &pending.then else {
        panic!("the lease box runs a push: {:?}", pending.then);
    };
    let graft_ops::operation::Operation::Push(params) = &**op else {
        panic!("the lease box runs a push: {:?}", pending.then);
    };
    let expected = params.lease.as_ref().map(|lease| lease.expected);
    assert_eq!(
        expected,
        Some(recorded),
        "the lease names the entry's tip, not the fetched {theirs}"
    );

    let state = confirm(&app, &state);
    let state = finish(&app, state, "the re-push");
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("a refused lease is a sticky failure: {:?}", state.status);
    };
    // `graft_ops` checks the lease against the fetched tracking ref before
    // git is asked, and says so; here "fetched since" is the true reason.
    assert!(
        reason.starts_with(&format!(
            "force-push refused: origin/{} is not where the box said",
            remote.default_branch()
        )),
        "{reason}"
    );
    drop(state);
    assert_eq!(
        rev(remote.bare(), &main),
        theirs,
        "the bare keeps their commit"
    );
    app.close();
}

/// A teammate pushes and this clone fetches it outside the app, then
/// `Ctrl+R`: `state.remotes` now holds *their* tip — what the old lease was
/// built from. Returns their tip, 40 hex.
fn their_push_fetched(app: &App, state: &AppState, remote: &LocalRemote) -> (AppState, String) {
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    run(remote, remote.work(), &["fetch", "-q", "origin"]);
    let state = app.press(state, KeyPress::control(KeyCode::Char('r')));
    let state = app.pump(state, "the re-read tracking tip", |state| {
        matches!(&state.remotes, Loading::Ready(facts)
            if facts.head_upstream.as_ref().and_then(|up| up.tracking_tip)
                .is_some_and(|tip| tip.to_string() == theirs))
            && state.remote.reading.is_none()
            && matches!(state.journal.entries, Loading::Ready(_))
            && state.journal.reading.is_none()
    });
    (state, theirs)
}

/// A commit made by `c` and sent by `P`, with the journal and the refreshed
/// tracking tip landed: the state `u` finds after a push.
fn pushed_one_commit(app: &App, state: AppState) -> AppState {
    let state = ready(app, state);
    let state = commit_by_key(app, &state, "to push");
    let state = app.settle_write(state, "the commit");
    let state = ready(app, state);
    let state = app.press(&state, plain('P'));
    let state = confirm(app, &state);
    let state = app.settle_write(state, "the push");
    journal_has_the_push(app, state)
}

/// The journal read that carries the push, so `u` decides from it.
fn journal_has_the_push(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the journal with the push", |state| {
        matches!(&state.journal.entries, Loading::Ready(read)
            if read.entries.iter().any(|entry| entry.kind == OperationKind::Push))
            && state.journal.reading.is_none()
    });
    // And the refreshed tracking tip the lease is taken against.
    ready(app, state)
}

/// The words a non-fast-forward rejection leaves.
fn rejected_status(branch: &str) -> Status {
    Status::Sticky {
        reason: format!("push rejected: origin/{branch} has commits you don't"),
        keys: Some("p pull, then m or r · P force with lease".to_owned()),
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

/// **m6-06, `KNOWN_GAPS` 550, clause (iii).** The push box is opened on
/// `main`; `HEAD` then moves to `feature` outside Graft, with no re-read —
/// the stale-facts state of the `Ctrl+R` race, made deterministic. `Enter`
/// launches the box's push unchanged (the app holds no check of its own);
/// `graft_ops::apply` re-reads `HEAD` and refuses it with
/// `Refusal::BranchChanged` before the `Pending` line. So: the bare's
/// `main` does not move, the journal has no push, and the refusal's words
/// reach the status line.
#[test]
fn a_push_after_head_moved_is_refused_in_words() {
    let remote = local();
    let work = remote.work();
    let main = main_ref(&remote);
    // One commit ahead, so a push that went through would move the bare.
    let ours = remote.commit_locally("ours").expect("a local commit");
    let bare_before = rev(remote.bare(), &main);
    assert_ne!(bare_before, ours, "the push would have something to send");

    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('P'));
    assert!(state.confirm.is_some(), "P opened the push box");

    // Outside Graft, no re-read pumped: the box still holds `main`.
    run(&remote, work, &["switch", "-q", "-c", "feature"]);
    assert_eq!(
        git(work, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/feature"
    );

    let state = confirm(&app, &state);
    let state = finish(&app, state, "the refused push");
    // A refusal is sticky, like the stale lease's: it stays until read.
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "HEAD moved to another branch since it was read — look again".to_owned(),
            keys: None,
        }
    );
    assert_eq!(
        rev(remote.bare(), &main),
        bare_before,
        "the bare's main is unmoved"
    );
    let pushes: Vec<_> = journal(work)
        .into_iter()
        .filter(|entry| entry.kind == OperationKind::Push)
        .collect();
    assert!(
        pushes.is_empty(),
        "no Pending line for the push: {pushes:?}"
    );
    drop(state);
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
    assert_eq!(state.status, Status::done("fetch all remotes cancelled"));

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
    assert_eq!(state.status, Status::done("commit \"slow\" cancelled"));
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
    assert_eq!(state.status, Status::done("fetched"));
    app.close();
}

/// Writes the undo-after-push capture into `docs/shots/`.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it remote_by_key::capture_undo_after_push_for_docs_shots`.
#[test]
fn capture_undo_after_push_for_docs_shots() {
    let remote = local();
    let work = remote.work();
    stage(work);

    let (app, state) = App::open(work);
    let state = pushed_one_commit(&app, state);
    let state = crate::captures::at_rest(&app, state);

    // The one thing in this capture the run invents: `to push` is committed
    // moments before the frame is drawn, so its id is new every run. The
    // frame keeps its shape and the id is zeroed out of the file
    // (m5-history-and-ghost-preview/40) — the capture's caption says so.
    let pushed = run(&remote, work, &["rev-parse", "HEAD"]).trim().to_owned();

    // `u`: the box that explains the push.
    let asked = app.key(&state, KeyCode::Char('u'));
    assert!(asked.confirm.is_some(), "u asked nothing to capture");
    let first = frame(&asked, 80, 24);

    // Right, Right, Enter: `[ Re-push old tip ]`, which opens its own box
    // before anything reaches the remote.
    let moved = app.key(&asked, KeyCode::Right);
    let moved = app.key(&moved, KeyCode::Right);
    let repush = app.key(&moved, KeyCode::Enter);
    assert!(
        repush.confirm.is_some(),
        "re-push ran without its own confirm"
    );
    let second = frame(&repush, 80, 24);

    let body = format!(
        "{}\n== `u` after a push: the one box with three real answers ==\n\n{}\n\n\
         == `[ Re-push old tip ]`: its own second box, before the remote moves ==\n\n{}\n",
        undo_after_push_header(),
        first.join("\n"),
        second.join("\n"),
    );
    check_shot(
        "m5-ui-undo-after-push-80x24.txt",
        &without_invented_id(&body, &pushed),
    );

    drop(repush);
    drop(state);
    app.close();
}

/// The caption. What right looks like, in the user's terms, and what the
/// capture cannot show.
fn undo_after_push_header() -> String {
    "# m5-ui-undo-after-push-80x24.txt: 80x24, drawn by `view::view` from a\n\
     # real AppState \u{2014} a scratch repository pushed to a `file://` bare,\n\
     # then `u` (crates/graft/tests/it/remote_by_key.rs,\n\
     # capture_undo_after_push_for_docs_shots, asserted). Not a pty\n\
     # capture: the terminal is ratatui's TestBackend, so this is a\n\
     # character grid with no colour attributes in it \u{2014} CLAUDE.md rule 5,\n\
     # colour ships unverified.\n\
     #\n\
     # Why: `u` undoes your local branch. A push has already sent that work\n\
     # somewhere others can pull. Undoing silently would leave the two\n\
     # disagreeing with no word about it (ADR-0038 \u{a7}4).\n\
     #\n\
     # What right looks like:\n\
     #  - **The first box names the remote side** and says in words that\n\
     #    undo is local only, and that re-pushing the old tip rewrites a\n\
     #    branch others may have pulled. Three answers, `Cancel` first.\n\
     #  - **Re-push asks a second time.** Choosing `[ Re-push old tip ]`\n\
     #    does not push: it opens the ordinary push confirm, showing the\n\
     #    exact command. Two deliberate answers before a remote branch is\n\
     #    rewritten.\n\
     #  - **The command line says `--force-with-lease`**, never a bare\n\
     #    `--force`. At 80 columns the box is narrower than the full\n\
     #    command, so the tail is clipped \u{2014} the same clipping the M4 lease\n\
     #    box already takes (noted in the ticket; the fix is in graft-ops).\n\
     #  - Your local branch keeps every commit it has: re-push moves the\n\
     #    remote back, not you.\n\
     #\n\
     # The `to push` commit is made by this test moments before the frame is\n\
     # drawn, so its id is new on every run. It is zeroed to `0000000` in\n\
     # this file, at the width it was drawn at, so the capture can be\n\
     # compared to the code. Nothing else here is normalised.\n"
        .to_owned()
}

/// The caption of `fix-plain-box-after-rebase-187x50.txt`: what right looks
/// like, and what the capture cannot show.
const PLAIN_BOX_AFTER_REBASE_HEADER: &str = "\
# TestBackend capture (not a pty), 187x50, no colour recorded. Fixture:
# capture_fix_plain_box_after_rebase in crates/graft/tests/it/remote_by_key.rs
# (asserted). A teammate pushed; `P` `→` `⏎` was rejected; `p` named the
# divergence; `r` `r` rebased onto origin/main; then `P` (m6-conflicts/49).
# The rebased commit is drawn moments before the frame, so its id is new
# every run: it is zeroed to `0000000` here. Nothing else is normalised.
#
# Judge by eye:
# - The box is titled `Push main → origin/main`, not `Force-push`.
# - Its line starts with the ahead count (`1 commit`).
# - The command row has no `--force-with-lease`.
# - The action reads `Push`, not `Force push`.
# - Colour is unverified (CLAUDE.md rule 5).
#
";

/// The caption of `fix-box-takes-y-<size>.txt` (m6-conflicts/50).
fn box_takes_y_header(size: &str) -> String {
    format!(
        "\
# TestBackend capture (not a pty), {size}, no colour recorded. Fixture:
# capture_fix_box_takes_y in crates/graft/tests/it/remote_by_key.rs (asserted).
# A teammate pushed, one local commit. Frame 1: `P`, the plain push box. Then
# `y` pushes (rejected: origin moved). Frame 2: `P` again, the force (lease)
# box, then `y` pressed: it is the frame before `y`, byte for byte.
#
# What right looks like:
# - Frame 1's bottom row (the hint bar) reads
#   `y push  ←/→ move  ⏎ choose  esc cancel`, whole; the box's bottom
#   border is a plain `╰…╯`; `[ Cancel ]` under the cursor.
# - Frame 2: the box is titled `Force-push …` and is still up after `y`
#   (asserted: same frame, nothing sent, origin unmoved). No `y push`
#   anywhere: `y` never confirms the force push (ADR-0044 §3). The bottom
#   row is the box's own bar, `←/→ move  ⏎ choose  esc cancel`: an open
#   box owns the row, and the rejection's sticky line waits under it until
#   `esc` (m6-conflicts/69).
# - Colour is unverified (CLAUDE.md rule 5).
#
"
    )
}

/// The caption of `fix-box-keeps-on-y-120x40.txt`, retaken by m6-conflicts/50:
/// ticket 47's claim now stands on the box `y` never takes.
const BOX_KEEPS_ON_Y_HEADER: &str = "\
# m6-conflicts/47, retaken by m6-conflicts/50: the force (lease) push box open,
# `y` pressed (120x40). TestBackend capture (not a pty); colour unverified.
# Fixture: capture_fix_box_takes_y in crates/graft/tests/it/remote_by_key.rs.
# The plain push box takes `y` since ticket 50, so the box that keeps on `y`
# is the lease box: a force push is never one habitual key away.
# What right looks like: the frame after `y` is the frame before it, byte for
# byte (asserted): the box titled `Force-push main → origin/main` stays,
# `[ Cancel ]` under the cursor, nothing sent, no `y push` anywhere. The
# bottom row is the box's own bar, `←/→ move  ⏎ choose  esc cancel`: the
# rejection's sticky line waits under the box (m6-conflicts/69).

";

/// Regenerates `docs/shots/m6/fix-box-takes-y-120x40.txt`, `-80x24.txt` and
/// `fix-box-keeps-on-y-120x40.txt` (m6-conflicts/50): `y` takes the plain
/// push box from `Cancel` against a real remote, and never the lease box.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it remote_by_key::capture_fix_box_takes_y`.
#[test]
fn capture_fix_box_takes_y() {
    let remote = local();
    let work = remote.work();
    remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    remote.commit_locally("ours").expect("a local commit");
    let branch = remote.default_branch();
    let main = main_ref(&remote);
    let theirs = rev(remote.bare(), &main);

    let (app, state) = App::open_with_clock(
        work,
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let (plain_box, plain_frames) = plain_box_takes_y(&app, state, branch);

    // `y` from `Cancel` pushes: the write starts, and origin refuses it.
    let pushing = app.press(&plain_box, plain('y'));
    assert!(pushing.confirm.is_none(), "`y` left the plain box open");
    let state = finish(&app, pushing, "the push `y` sent");
    assert_eq!(state.status, rejected_status(branch));

    let lease_frames = lease_box_keeps_on_y(&app, state);
    assert_eq!(rev(remote.bare(), &main), theirs, "origin moved");

    for ((size, plain_drawn), lease_drawn) in plain_frames.iter().zip(&lease_frames) {
        check_shot(
            &format!("m6/fix-box-takes-y-{size}.txt"),
            &format!(
                "{}=== frame 1: `P`, the plain push box ===\n{}\n\n\
                 === frame 2: `y` (rejected), `P`, the lease box, then `y` ===\n{}\n",
                box_takes_y_header(size),
                plain_drawn.join("\n"),
                lease_drawn.join("\n"),
            ),
        );
    }
    check_shot(
        "m6/fix-box-keeps-on-y-120x40.txt",
        &format!(
            "{BOX_KEEPS_ON_Y_HEADER}=== after `y` (identical to before it) ===\n{}\n",
            lease_frames[0].join("\n")
        ),
    );
    app.close();
}

/// The two sizes [`capture_fix_box_takes_y`] draws at.
const BOX_SIZES: [(&str, u16, u16); 2] = [("120x40", 120, 40), ("80x24", 80, 24)];

/// `P` on a ready app: the plain push box, cursor on `Cancel`, and its
/// frames at [`BOX_SIZES`], each bar leading with `y push`.
fn plain_box_takes_y(
    app: &App,
    state: AppState,
    branch: &str,
) -> (AppState, Vec<(&'static str, Vec<String>)>) {
    let state = ready(app, state);
    let state = crate::captures::at_rest(app, state);
    let plain_box = app.press(&state, plain('P'));
    let pending = plain_box.confirm.as_ref().expect("P opened a box");
    assert_eq!(pending.view.selected(), 0, "the box opens on Cancel");
    assert_eq!(
        pending.view.title.as_str(),
        format!("Push {branch} → origin/{branch}")
    );
    let frames: Vec<(&str, Vec<String>)> = BOX_SIZES
        .into_iter()
        .map(|(size, columns, rows)| (size, frame(&plain_box, columns, rows)))
        .collect();
    for (size, drawn) in &frames {
        assert_eq!(
            drawn.last().map(|row| row.trim()),
            Some("y push  ←/→ move  ⏎ choose  esc cancel"),
            "{size}"
        );
    }
    (plain_box, frames)
}

/// After a rejection, `P` opens the lease box; `y` on it changes nothing
/// and sends nothing. Its frames at [`BOX_SIZES`], before `y` (asserted
/// equal to after).
fn lease_box_keeps_on_y(app: &App, state: AppState) -> Vec<Vec<String>> {
    let state = ready(app, state);
    let state = crate::captures::at_rest(app, state);
    let lease_box = app.press(&state, plain('P'));
    let pending = lease_box.confirm.as_ref().expect("P opened the lease box");
    assert!(
        pending.view.title.as_str().starts_with("Force-push "),
        "{}",
        pending.view.title.as_str()
    );
    let after_y = app.press(&lease_box, plain('y'));
    assert_eq!(
        after_y.confirm, lease_box.confirm,
        "`y` changed the lease box"
    );
    assert_eq!(after_y.status, lease_box.status, "`y` changed the status");
    assert!(after_y.operating.is_none(), "`y` started a write");
    BOX_SIZES
        .into_iter()
        .map(|(size, columns, rows)| {
            let before = frame(&lease_box, columns, rows);
            let after = frame(&after_y, columns, rows);
            assert_eq!(
                after, before,
                "{size}: the frame after `y` is not the one before"
            );
            // The box owns the bottom row: its own keys, not the
            // rejection's sticky line waiting under it (m6-conflicts/69).
            // No `y` hint anywhere on screen.
            let bottom = after.last().map(|row| row.trim().to_owned());
            assert_eq!(
                bottom.as_deref(),
                Some("←/→ move  ⏎ choose  esc cancel"),
                "{size}"
            );
            assert!(
                !after.iter().any(|row| row.contains("y push")),
                "{size}: the lease box offers `y`"
            );
            before
        })
        .collect()
}

/// `P` `→` `⏎` rejected, then `p` refused as diverged: the state `r` answers.
pub(crate) fn rejected_then_diverged(app: &App, state: AppState, branch: &str) -> AppState {
    let state = ready(app, state);
    let state = app.press(&state, plain('P'));
    let state = confirm(app, &state);
    let state = finish(app, state, "the rejected push");
    assert_eq!(state.status, rejected_status(branch));

    let state = ready(app, state);
    let state = app.press(&state, plain('p'));
    let state = finish(app, state, "the refused pull");
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("a diverged pull is a sticky line: {:?}", state.status);
    };
    assert!(reason.contains("have diverged"), "{reason}");
    state
}

/// Regenerates `docs/shots/m6/fix-plain-box-after-rebase-187x50.txt`
/// (m6-conflicts/49): a teammate pushed, `P` was rejected, `p` named the
/// divergence, `r r` rebased onto their work — now `P` opens the plain box.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it remote_by_key::capture_fix_plain_box_after_rebase`.
#[test]
fn capture_fix_plain_box_after_rebase() {
    let remote = local();
    let work = remote.work();
    remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    remote.commit_locally("ours").expect("a local commit");
    let branch = remote.default_branch();

    let (app, state) = App::open(work);
    let state = rejected_then_diverged(&app, state, branch);

    let state = ready(&app, state);
    let state = app.press(&state, plain('r'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !matches!(preview.plan, Loading::Pending(_) | Loading::Idle))
    });
    let state = app.press(&state, plain('r'));
    let state = app.settle_write(state, "the rebase");
    let state = ready(&app, state);
    let state = crate::captures::at_rest(&app, state);
    let rebased = rev(work, "HEAD");

    let state = app.press(&state, plain('P'));
    let pending = state.confirm.as_ref().expect("P opened a box");
    assert_eq!(
        pending.view.title.as_str(),
        format!("Push {branch} → origin/{branch}")
    );
    assert!(
        !pending.view.command.as_str().contains("--force"),
        "{}",
        pending.view.command.as_str()
    );
    assert_eq!(
        pending
            .view
            .choices
            .get(1)
            .map(graft_term::SanitizedText::as_str),
        Some("Push")
    );

    let drawn = frame(&state, 187, 50).join("\n");
    check_shot(
        "m6/fix-plain-box-after-rebase-187x50.txt",
        &format!(
            "{PLAIN_BOX_AFTER_REBASE_HEADER}{}\n",
            without_invented_id(&drawn, &rebased)
        ),
    );
    drop(state);
    app.close();
}
