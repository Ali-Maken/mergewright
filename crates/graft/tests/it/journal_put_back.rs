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
use graft_core::on_disk::CURRENT_DIR;

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
        !repo
            .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"))
            .exists()
            || graft_ops::journal::Journal::open(
                &repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"))
            )
            .expect("the journal is readable")
            .entries()
            .is_empty(),
        "and nothing was journaled"
    );
}

/// V8 (24-b, ruling 2026-09-29): `u` pressed in J with the cursor on a `✗` row undoes
/// the journal's next entry and lands as `⏎` on the `← u` row does — J
/// closed.
#[test]
fn u_in_j_undoes_the_next_entry_from_any_row() {
    let (scratch, lost, left) = lost_by_the_shell();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = journal_with_every_stage(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the put-back's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Idle | graft::update::detail::Loading::Pending(_)
            )
        })
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    assert_eq!(main_tip(repo), lost, "main has the lost commit again");

    // The shell loses a second commit: a `✗` row newer than the put-back.
    std::fs::write(repo.join("cache.txt"), "ttl: 5\n").expect("the scratch repo is writable");
    git(repo, &["add", "cache.txt"]);
    git(repo, &["commit", "-q", "-m", "Try redis cache"]);
    let second = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(repo, &["reset", "-q", "--hard", "HEAD~1"]);

    let state = journal_with_every_stage(&app, state);
    // m7/30o Minor 13: the cursor really is on a `✗` row when `u` is
    // pressed. The put-back's `●` row is newer than the shell's reset, so
    // no preselect moves it there (the review's point: before this the
    // test pressed `u` on the `← u` row itself); `n` does.
    let state = app.key(&state, KeyCode::Char('n'));
    let frame = crate::m7_walk::journal::drawn(&state, 187, 50);
    assert!(
        crate::m7_walk::journal::selected_row(&frame).contains("\u{2717} lost"),
        "the selected row is a \u{2717} lost row:\n{}",
        crate::m7_walk::journal::text(&frame)
    );
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo from J");
    assert_eq!(main_tip(repo), left, "u took the put-back back");
    assert_ne!(main_tip(repo), second, "u never put the ✗ row back");
    assert!(
        !state.journal.shown,
        "u in J lands as ⏎ on the ← u row: J closed"
    );
    drop(state);
    app.close();
}

/// `⏎` on J's preselected lost row, and its preview's plan landed.
fn put_back_box(app: &App, state: graft::update::AppState) -> graft::update::AppState {
    let state = journal_with_every_stage(app, state);
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the put-back's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, graft::update::detail::Loading::Ready(_)))
    })
}

/// m7/30o Minor 7: `Esc` while a put-back runs says what that put-back
/// does — it runs to the end, and `u` undoes it after — not a rebase's
/// banner words (`HISTORY_CANNOT_STOP`).
#[test]
fn a_put_back_runs_with_its_own_esc_words() {
    let (scratch, _lost, _left) = lost_by_the_shell();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = put_back_box(&app, state);
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "the put-back runs");
    let running = app.key(&state, KeyCode::Escape);
    assert!(
        matches!(&running.status, graft::update::Status::Busy(words)
            if words.ends_with("runs to the end · u undoes it after")),
        "{:?}",
        running.status
    );
    let state = app.settle_write(running, "the put-back");
    drop(state);
    app.close();
}

/// m7/30o Minor 7: a put-back refused at launch (a write began while the
/// box was up) leaves J up, the refusal on J's status row — J closes only
/// once the launch is `Ok`.
#[test]
fn a_put_back_refused_at_launch_stays_on_j() {
    let (scratch, _lost, left) = lost_by_the_shell();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let mut state = put_back_box(&app, state);
    state.operating = Some(graft_core::job::JobId::new(9_999));
    let state = app.key(&state, KeyCode::Char('y'));
    let frame = crate::m7_walk::journal::drawn(&state, 187, 50);
    let body = crate::m7_walk::journal::text(&frame);
    assert!(state.journal.shown, "J still up:\n{body}");
    let legend = frame
        .iter()
        .rposition(|row| row.starts_with("● done in Mergewright"))
        .unwrap_or_else(|| panic!("J's legend:\n{body}"));
    assert!(
        frame[legend - 1].contains("wait for the write to finish"),
        "the refusal on J's status row:\n{body}"
    );
    drop(state);
    app.close();
    assert_eq!(main_tip(repo), left, "nothing ran");
}

/// `⏎ y` on J's preselected lost row, the put-back settled and the graph
/// re-read with the lost commit on it.
fn put_back_through_j(app: &App, state: graft::update::AppState) -> graft::update::AppState {
    let state = journal_with_every_stage(app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the put-back's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Idle | graft::update::detail::Loading::Pending(_)
            )
        })
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    let state = app.pump(state, "the graph with the commit back on it", |state| {
        crate::captures::frame(state, 160, 40)
            .iter()
            .any(|row| row.contains("Add rate limiter") && !row.contains('\u{2713}'))
    });
    crate::captures::settled_frame(app, state)
}

/// After a put-back, `J` again: the put-back is a `●` row of Graft's own
/// (`u` can take it back), and the commit it put back is no longer lost —
/// no `✗` row.
#[test]
fn j_again_shows_a_graft_row_and_no_lost_row() {
    let (scratch, lost, _left) = lost_by_the_shell();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = put_back_through_j(&app, state);
    assert_eq!(main_tip(repo), lost);

    let state = journal_with_every_stage(&app, state);
    let frame = crate::captures::frame(&state, 187, 50);
    let body = frame.join("\n");
    assert!(
        frame
            .iter()
            .any(|row| row.contains("\u{25cf}  Mergewright") && row.contains("put main back to")),
        "the put-back's own \u{25cf} row:\n{body}"
    );
    assert!(!body.contains("\u{2717} lost"), "no \u{2717} row:\n{body}");
    drop(state);
    app.close();
}

/// Where a put-back lands: the graph, the commit tagged `↺ put back`, the
/// commit the branch left tagged `○ main was here`, and the `✓` line.
#[test]
fn put_back_and_was_here_tags_draw() {
    let (scratch, _lost, _left) = lost_by_the_shell();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = put_back_through_j(&app, state);
    assert!(!state.journal.shown, "y closed J");
    let frame = crate::captures::frame(&state, 160, 40);
    let body = frame.join("\n");
    let tagged = |needle: &str, subject: &str| {
        frame
            .iter()
            .any(|row| row.contains(needle) && row.contains(subject))
    };
    assert!(
        tagged("\u{21ba} put back", "Add rate limiter"),
        "the put-back commit's tag:\n{body}"
    );
    assert!(
        tagged("\u{25cb} main was here", "Correct the note"),
        "where main was:\n{body}"
    );
    assert!(
        body.contains(
            "put back \u{2713} \u{b7} main has \"Add rate limiter\" again \u{b7} u undoes"
        ),
        "the \u{2713} line:\n{body}"
    );
    drop(state);
    app.close();
}

/// What he staged at `src/a.rs` (graft-ops' test 1's `S`), differing from
/// `HEAD`'s `pub fn a() {}`.
const STAGED: &[u8] = b"pub fn a() -> u32 {\n    3\n}\n";
/// What is on disk after staging (`W`), differing from `S`.
const WORKING: &[u8] = b"pub fn a() -> u32 {\n    4\n}\n";

/// Journey 15's precondition without walking 13 and 14 (their put-backs
/// need the walk's pinned-clock child): `main` has D again and `spike` is
/// back at E, by the shell, stamped between the fixture's fetch and the
/// app's clock. With no lost commit above it, `n` from J's first row lands
/// on the lost edit, as in walk 15.
fn journeys_13_and_14_by_the_shell(
    repo: &std::path::Path,
    commits: &graft_fixtures::lost_work::Commits,
) {
    let at = format!("@{} +0000", graft_fixtures::lost_work::END + 30);
    for args in [
        vec!["merge", "-q", "--ff-only", commits.d.as_str()],
        vec!["branch", "spike", commits.e.as_str()],
    ] {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(&args)
            .env("GIT_COMMITTER_DATE", &at)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// After `⏎`: the box says the staged copy stays. Writes
/// `put-back-staged-preview-187x50.txt`.
fn check_staged_box(state: &graft::update::AppState) {
    use crate::m7_walk::journal::{drawn, text};
    let body = text(&drawn(state, 187, 50));
    assert!(
        body.contains("Preview: put back your edit to src/a.rs"),
        "the box's title:\n{body}"
    );
    assert!(
        body.contains(
            "your files: src/a.rs comes back · what's in src/a.rs now is replaced · \
             src/a.rs stays staged as you had it"
        ),
        "the box says the staged copy stays:\n{body}"
    );
    assert!(body.contains("u undoes it"), "the box's last line:\n{body}");
    crate::captures::check_shot("m7/put-back-staged-preview-187x50.txt", &body);
}

/// After `y`: J closed, both sections, the cursor on the `UNSTAGED` row with
/// the lost edit's diff. Writes `put-back-staged-landed-187x50.txt`.
fn check_staged_landing(state: &graft::update::AppState) {
    use crate::m7_walk::journal::{drawn, text};
    let landed = drawn(state, 187, 50);
    let body = text(&landed);
    assert!(
        !landed.iter().any(|row| row.contains("JOURNAL ·")),
        "J is closed:\n{body}"
    );
    assert!(
        body.contains("STAGED (1)") && body.contains("UNSTAGED (1)"),
        "the pane holds both sections:\n{body}"
    );
    // The lost edit's `2` added (its diff against the index); `S`'s `3`
    // added is the STAGED row's diff against HEAD, not shown.
    assert!(
        landed.iter().any(|row| row.contains("+     2")),
        "the diff is the lost edit's:\n{body}"
    );
    assert!(
        !landed.iter().any(|row| row.contains("+     3")),
        "not the staged copy's diff:\n{body}"
    );
    assert_eq!(
        state.detail.selected_section(),
        Some(graft_ui::detail::WorkingTreeSection::Unstaged),
        "the cursor is on the UNSTAGED row:\n{body}"
    );
    let bottom = landed.last().map_or("", String::as_str);
    assert!(
        bottom.contains("your edit to src/a.rs is back")
            && bottom.contains('✓')
            && bottom.contains("u undoes"),
        "the bottom row:\n{body}"
    );
    crate::captures::check_shot("m7/put-back-staged-landed-187x50.txt", &body);
}

/// m7/30p R2: journey 15 over a staged file. The put-back writes the
/// working tree only, so his staged copy stays; the box says so before `y`,
/// and `y` lands on the file's `UNSTAGED` row, whose diff is the lost edit.
/// Also `docs/shots/m7/put-back-staged-{preview,landed}-187x50.txt`.
#[test]
fn a_staged_path_put_back_lands_on_its_unstaged_row() {
    use crate::m7_walk::blame::Route;
    use crate::m7_walk::journal::{drawn, lost_work, opened, selected_row, text};
    use graft_fixtures::lost_work::EDITED;

    let fixture = lost_work();
    let repo = fixture.work();
    journeys_13_and_14_by_the_shell(repo, fixture.commits());
    std::fs::write(repo.join(EDITED), STAGED).expect("S is written");
    git(repo, &["add", "--", EDITED]);
    std::fs::write(repo.join(EDITED), WORKING).expect("W is written");
    let staged = || git(repo, &["show", &format!(":{EDITED}")]);
    assert_eq!(staged().as_bytes(), STAGED);
    let (app, state) = opened(repo);

    let mut route = Route::new("edit-in-undo-backup");
    let state = route.press(&app, &state, "J", KeyCode::Char('J'));
    let state = app.pump(state, "J's stages 2 and 3", |state| {
        !state.journal.finding_lost()
    });
    let state = route.press(&app, &state, "n", KeyCode::Char('n'));
    assert!(
        selected_row(&drawn(&state, 187, 50)).contains("your edit to src/a.rs"),
        "n lands on the lost edit's row:\n{}",
        text(&drawn(&state, 187, 50))
    );
    let state = route.press(&app, &state, "⏎", KeyCode::Enter);
    let state = app.pump(state, "what the row puts back", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, graft::update::detail::Loading::Ready(_)))
    });
    check_staged_box(&state);

    let state = route.press(&app, &state, "y", KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    let state = crate::captures::settled_frame(&app, state);
    route.within_count();

    check_staged_landing(&state);
    assert_eq!(
        std::fs::read(repo.join(EDITED)).expect("src/a.rs reads"),
        graft_fixtures::lost_work::LOST_EDIT
    );
    assert_eq!(staged().as_bytes(), STAGED, "the index still holds S");

    // Uncounted, after the journey: `u` brings back W over the same index.
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo of the put-back");
    assert_eq!(
        std::fs::read(repo.join(EDITED)).expect("src/a.rs reads"),
        WORKING,
        "u brings back W"
    );
    assert_eq!(staged().as_bytes(), STAGED, "the index still holds S");
    drop(state);
    app.close();
}
