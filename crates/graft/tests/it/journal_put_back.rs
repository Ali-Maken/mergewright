//! J's three jobs over the real job pool and real `git`
//! (m7-power-tools/24-a): the shell's `reset --hard` loses a commit, `J`
//! finds it, `⏎ y` puts `main` back, and `u` takes the put-back back.
//!
//! What `recover_for` decides is graft-ops' (its own tests); what the
//! screen draws is 24-b's. These check the part in between that only a
//! running app has: each stage's job is asked, answered and routed, and the
//! put-back really moves the branch — read out of git, not out of
//! `AppState`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft_core::event::KeyCode;

use crate::app_harness::{App, git};

/// Two commits on `main`, then the shell's `reset --hard HEAD~1`: the
/// second commit is on no branch any more. Returns the scratch repo, the
/// lost commit and the tip `main` was left on.
fn lost_by_the_shell() -> (graft_fixtures::Scratch, String, String) {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("limits.txt"), "rate: 10\n").expect("the scratch repo is writable");
    git(repo, &["add", "limits.txt"]);
    git(repo, &["commit", "-q", "-m", "Add rate limiter"]);
    let lost = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(repo, &["reset", "-q", "--hard", "HEAD~1"]);
    let left = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    assert_ne!(lost, left);
    (scratch, lost, left)
}

fn main_tip(repo: &std::path::Path) -> String {
    git(repo, &["rev-parse", "refs/heads/main"])
        .trim()
        .to_owned()
}

/// `J`, then every stage answered by its real job.
fn journal_with_every_stage(app: &App, state: graft::update::AppState) -> graft::update::AppState {
    let state = app.pump(state, "the launch read", |state| {
        state.journal.reading.is_none()
    });
    let state = app.key(&state, KeyCode::Char('J'));
    assert!(state.journal.shown);
    assert!(
        state.journal.finding_lost(),
        "`J` asked for stage 2 at once"
    );
    app.pump(state, "stages 2 and 3", |state| {
        !state.journal.finding_lost()
    })
}

/// The whole path, and `u` after it: J's lost row is preselected (a shell
/// step newer than any Graft row), `⏎` asks what it puts back, `y` runs it,
/// and `u` undoes it like any other operation.
#[test]
fn u_undoes_the_put_back() {
    let (scratch, lost, left) = lost_by_the_shell();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = journal_with_every_stage(&app, state);

    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "what the row puts back", |state| {
        state.preview.is_some() || state.status != graft::update::Status::default()
    });
    assert!(
        state.preview.is_some(),
        "⏎ on the lost row opened the put-back's preview: {:?}",
        state.status
    );
    assert!(state.journal.shown, "J stays under the preview");
    let state = app.pump(state, "the put-back's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Idle | graft::update::detail::Loading::Pending(_)
            )
        })
    });
    assert!(
        matches!(
            state.preview.as_ref().map(|preview| &preview.plan),
            Some(graft::update::detail::Loading::Ready(_))
        ),
        "the plan landed: {:?}",
        state.preview.as_ref().map(|preview| &preview.plan)
    );
    assert_eq!(main_tip(repo), left, "a preview moves nothing");

    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    assert_eq!(main_tip(repo), lost, "main has the lost commit again");

    // `y` closed J with the box (24-a2's `confirm_preview`): `u` is
    // pressed on the graph, where he landed.
    assert!(!state.journal.shown, "y closed J");
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo of the put-back");
    drop(state);
    app.close();
    assert_eq!(main_tip(repo), left, "u took the put-back back");
}

/// `u` never acts on what the shell did: it takes back Graft's own newest
/// operation, never a `$` or `✗` row under the cursor. With only the
/// shell's steps listed there is nothing for it to take back, and the
/// reset stands.
///
/// `u` is global (`update/keys.rs`, the `is_undo_key` arm before the
/// overlays), so in J it is not swallowed: it asks the journal, the same
/// as from the graph (ticket 24 NOTES).
#[test]
fn u_never_acts_on_outside_rows() {
    let (scratch, _lost, left) = lost_by_the_shell();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = journal_with_every_stage(&app, state);
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the undo's answer", |state| {
        state.operating.is_none()
    });
    assert!(
        matches!(&state.status, graft::update::Status::Done { what, .. } if what == "nothing to undo"),
        "nothing was undone: {:?}",
        state.status
    );
    drop(state);
    app.close();
    assert_eq!(main_tip(repo), left, "the shell's reset stands");
    assert!(
        !repo.join(".git/graft/undo-journal.jsonl").exists()
            || graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
                .expect("the journal is readable")
                .entries()
                .is_empty(),
        "and nothing was journaled"
    );
}
