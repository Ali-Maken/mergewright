//! The finish summary against a real merge (m6-conflicts/19, the app half
//! built by m6-conflicts/34): `c` on the resolved merge's banner, the
//! summary read from the repository, and `⏎` committing through the real
//! `git merge --continue`. What landed is read back from git, never from
//! the status line.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppEvent, AppState, Command, Product, Status, update};
use graft_core::event::{KeyCode, KeyPress};

use crate::app_harness::{App, git, git_as_graft, press_for_commands};
use crate::conflict_interim::{
    mark_resolved, open_changes_pane, stopped_on_the_conflict, tab_back_to_the_banner,
};

fn write(repo: &Path, name: &str, body: &str) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
}

/// `feature` and `main` both rewrote `shared.rs` and `notes.md`, so merging
/// `feature` stops on those two; `feature` also changed one line of
/// `README.md`, which git merges on its own.
fn about_to_merge() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    // The merge commit is the app's `git merge --continue`: the fixture's
    // own name, so no machine's identity decides whether it can commit.
    git(repo, &["config", "user.name", "Graft Fixture"]);
    git(repo, &["config", "user.email", "fixture@example.invalid"]);
    write(repo, "shared.rs", "fn main() {}\n");
    write(repo, "notes.md", "the plan\n");
    write(
        repo,
        "README.md",
        "# Project\n\nline one\nline two\nline three\n",
    );
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", "the shared files"]);
    git(repo, &["switch", "--quiet", "-c", "feature"]);
    write(repo, "shared.rs", "fn main() { theirs() }\n");
    write(repo, "notes.md", "their plan\n");
    write(
        repo,
        "README.md",
        "# Project\n\nline one\nline TWO\nline three\n",
    );
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", "the feature's take"]);
    git(repo, &["switch", "--quiet", "main"]);
    write(repo, "shared.rs", "fn main() { ours() }\n");
    write(repo, "notes.md", "our plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", "main's take"]);
    scratch
}

/// The merge started with `m`, both files resolved and marked, back on the
/// banner, `c` pressed and the summary read.
fn summary_open(app: &App, state: AppState, repo: &Path) -> AppState {
    let state = stopped_on_the_conflict(app, state);
    write(repo, "shared.rs", "fn main() { both() }\n");
    write(repo, "notes.md", "the agreed plan\n");
    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(app, state);
    let state = mark_resolved(app, state, "notes.md");
    let state = mark_resolved(app, state, "shared.rs");
    let state = tab_back_to_the_banner(app, state);
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(
        state.finish.is_some(),
        "`c` on the merge opens the summary: {:?}",
        state.status
    );
    assert!(state.operating.is_none(), "and writes nothing");
    app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// `⏎` (or whatever `key` is) and the `--continue` it runs, to its answer.
fn committed(app: &App, state: &AppState, key: KeyPress) -> AppState {
    let state = app.press(state, key);
    assert!(
        state.operating.is_some(),
        "the key commits: {:?}",
        state.finish.as_ref().map(|open| &open.notice)
    );
    app.pump(state, "the continue", |state| state.operating.is_none())
}

/// The last commit's whole message, as git stored it.
fn last_message(repo: &Path) -> String {
    git(repo, &["log", "-1", "--format=%B"])
}

fn typed(app: &App, state: AppState, text: &str) -> AppState {
    text.chars().fold(state, |state, character| {
        app.key(&state, KeyCode::Char(character))
    })
}

#[test]
fn an_edited_subject_commits_with_that_message() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let state = app.key(&state, KeyCode::Tab);
    let subject_len = state.finish.as_ref().map_or(0, |open| open.subject.len());
    let state = (0..subject_len).fold(state, |state, _| app.key(&state, KeyCode::Backspace));
    let state = typed(&app, state, "Merge feature: the agreed plan");
    let state = committed(&app, &state, KeyPress::plain(KeyCode::Enter));

    assert_eq!(
        last_message(repo).lines().next(),
        Some("Merge feature: the agreed plan")
    );
    assert!(
        !repo.join(".git/MERGE_HEAD").exists(),
        "git finished the merge"
    );
    drop(state);
    app.close();
}

#[test]
fn a_hash_line_typed_in_the_body_lands() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let state = app.key(&state, KeyCode::Tab);
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(&app, state, "#123 fixes login");
    let state = committed(&app, &state, KeyPress::control(KeyCode::Enter));

    let message = last_message(repo);
    assert!(
        message.lines().any(|line| line == "#123 fixes login"),
        "the typed line is in the commit:\n{message}"
    );
    drop(state);
    app.close();
}

/// What `Handover::edit_message` does (`event_loop/outside.rs`) minus the
/// terminal suspend, which needs a real terminal: the message written to
/// the file the app uses, the fixture's `$EDITOR` **run as a process**
/// through the app's own `editor_command`, the file read back.
///
/// The editor is the one git names with the environment's editor variables
/// taken out — git ranks `GIT_EDITOR` first, so a developer's own would
/// otherwise replace the fixture's (as `m5_walk::gestures::fixture_editor`).
fn run_the_fixture_editor(repo: &Path, message: &str) -> String {
    let asked = std::process::Command::new("git")
        .args(["var", "GIT_EDITOR"])
        .current_dir(repo.join(".git"))
        .env_remove("GIT_EDITOR")
        .env_remove("VISUAL")
        .env_remove("EDITOR")
        .output()
        .expect("git var runs");
    assert!(asked.status.success(), "git names an editor");
    let editor = String::from_utf8_lossy(&asked.stdout).trim().to_owned();
    let file = repo.join(".git/graft/COMMIT_EDITMSG");
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("writable");
    std::fs::write(&file, message).expect("writable");
    let status =
        graft::event_loop::outside::editor_command(&editor, &file, None, Some(repo)).status();
    assert!(
        status.as_ref().is_ok_and(std::process::ExitStatus::success),
        "{editor}: {status:?}"
    );
    std::fs::read_to_string(&file).expect("the editor left the file")
}

/// `Ctrl+E` hands the message to `$EDITOR`: a real process, the fixture's
/// `core.editor`, a script that appends `#123 fixes login`. The harness'
/// job pool cannot run a handover — that is the loop's, never a job's — so
/// the test takes the command `update` issued, runs the editor the way the
/// loop's `Handover::edit_message` does, and feeds back what it wrote. Only
/// the terminal suspend around the process is not exercised.
#[test]
fn a_hash_line_from_the_editor_lands() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = about_to_merge();
    let repo = scratch.path();
    let script = repo.join(".git/fixture-editor.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\nfor f in \"$@\"; do :; done\nprintf '\\n#123 fixes login\\n' >> \"$f\"\n",
    )
    .expect("writable");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    git(
        repo,
        &[
            "config",
            "--local",
            "core.editor",
            script.to_str().expect("a UTF-8 scratch path"),
        ],
    );
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let (state, commands) = press_for_commands(&state, KeyPress::control(KeyCode::Char('e')));
    let Some(Command::EditCommitMessage { id, message }) = commands
        .iter()
        .find(|command| matches!(command, Command::EditCommitMessage { .. }))
    else {
        panic!("`Ctrl+E` hands the message over: {commands:?}");
    };
    let edited = run_the_fixture_editor(repo, message);
    assert!(
        edited.lines().any(|line| line == "#123 fixes login"),
        "the editor ran: {edited:?}"
    );
    let (state, _) = update(
        &state,
        AppEvent::Produced(*id, Product::ExternalEditor(Ok(edited))),
    );
    let body = state
        .finish
        .as_ref()
        .map(|open| open.body.text())
        .expect("the summary is still open");
    assert!(
        body.lines().any(|line| line == "#123 fixes login"),
        "the editor's line is in the body: {body:?}"
    );

    let state = committed(&app, &state, KeyPress::control(KeyCode::Enter));
    let message = last_message(repo);
    assert!(
        message.lines().any(|line| line == "#123 fixes login"),
        "the editor's line is in the commit:\n{message}"
    );
    drop(state);
    app.close();
}

#[test]
fn the_summary_shows_no_git_comment_lines() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let merge_msg = std::fs::read_to_string(repo.join(".git/MERGE_MSG")).expect("git wrote it");
    assert!(
        merge_msg.lines().any(|line| line.starts_with('#')),
        "the fixture's MERGE_MSG carries git's comment block:\n{merge_msg}"
    );
    let open = state.finish.as_ref().expect("open");
    assert_eq!(
        open.subject.text(),
        merge_msg.lines().next().expect("a first line"),
        "the subject is git's first line"
    );
    let drawn = crate::captures::frame(&state, 120, 40);
    let message_lines: Vec<&String> = drawn
        .iter()
        .filter(|line| line.trim_start().starts_with('#'))
        .collect();
    assert!(
        message_lines.is_empty(),
        "no drawn line starts with `#`:\n{}",
        drawn.join("\n")
    );
    drop(state);
    app.close();
}

#[test]
fn a_hook_refusal_keeps_the_summary_open() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = about_to_merge();
    let repo = scratch.path();
    let hook = repo.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'commit-msg: a ticket number is required' >&2\nexit 1\n",
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(&app, state, " (kept)");
    let subject = state
        .finish
        .as_ref()
        .map(|open| open.subject.text().to_owned())
        .expect("open");

    let state = committed(&app, &state, KeyPress::plain(KeyCode::Enter));
    let open = state
        .finish
        .as_ref()
        .unwrap_or_else(|| panic!("the summary stays open: {:?}", state.status));
    let (words, problem) = open.notice.clone().expect("the refusal is on the summary");
    assert!(problem, "a refusal reads as a problem");
    assert!(
        words.contains("a ticket number is required"),
        "the hook's own words: {words:?}"
    );
    assert_eq!(open.subject.text(), subject, "the message is kept");
    assert_eq!(open.committing, None);
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "nothing was committed"
    );
    drop(state);
    app.close();
}

#[test]
fn done_lands_on_the_merge_commit() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = summary_open(&app, state, repo);

    let state = committed(&app, &state, KeyPress::plain(KeyCode::Enter));
    let state = app.pump(state, "the landing the finish owes", |state| {
        state.in_progress.is_none() && !state.land_on_head
    });
    assert!(state.finish.is_none(), "the summary closed");
    assert!(state.graph_has_focus(), "the graph has the keyboard");
    let head = git(repo, &["rev-parse", "HEAD"]);
    let selected = state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string());
    assert_eq!(
        selected.as_deref(),
        Some(head.trim()),
        "the cursor is on HEAD"
    );
    let parents = git(repo, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "a merge commit: itself and two parents: {parents}"
    );
    assert_eq!(
        state.status,
        Status::Done("merged feature into main".to_owned()),
        "drawn as `merged feature into main ✓`"
    );
    drop(state);
    app.close();
}

// ------------------------------------------------ what the rows say you chose

/// `gone_notes_and_readme_merge` stopped, the app open on it.
fn stopped_on_gone_and_notes() -> (graft_fixtures::Scratch, App, AppState) {
    let scratch = crate::resolver::gone_notes_and_readme_merge();
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    (scratch, app, state)
}

/// The resolver, landed on `path` after the file before it was done, with
/// nothing in flight.
fn advanced_to(app: &App, state: AppState, path: &str) -> AppState {
    app.pump(state, "the next file", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new(path)
                    && open.job.is_none()
                    && (open.file.is_some() || open.whole_file.is_some())
            })
    })
}

/// `o` on `gone.txt`'s card, the last file: the resolver closes to the
/// banner, and the status the stage asked for has landed.
fn gone_kept_last(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('o'));
    app.pump(state, "the last file done", |state| {
        state.operating.is_none() && state.resolver.is_none() && state.status_read.is_none()
    })
}

/// `c` on the banner and the summary read.
fn c_on_the_banner(app: &App, state: AppState) -> AppState {
    let state = crate::resolver::on_the_banner(app, state);
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(
        state.finish.is_some(),
        "`c` opens the summary: {:?}",
        state.status
    );
    app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// The drawn line naming `path` under `RESOLVED BY YOU`, at 120×40.
fn row_for(state: &AppState, path: &str) -> String {
    let drawn = crate::captures::frame(state, 120, 40);
    let from = drawn
        .iter()
        .position(|line| line.contains("RESOLVED BY YOU"))
        .unwrap_or_else(|| panic!("no RESOLVED BY YOU section:\n{}", drawn.join("\n")));
    drawn
        .iter()
        .skip(from)
        .find(|line| line.contains(path))
        .cloned()
        .unwrap_or_else(|| panic!("no row for {path}:\n{}", drawn.join("\n")))
}

/// `T` on `notes.txt`, then `o` on `gone.txt`, the last file: both resolved
/// in the app, back on the banner.
fn both_resolved_in_the_app(app: &App, state: AppState) -> AppState {
    let state = crate::resolver::resolver_on(app, state, "notes.txt");
    let state = app.press(&state, KeyPress::plain(KeyCode::Char('T')));
    let state = advanced_to(app, state, "gone.txt");
    gone_kept_last(app, &state)
}

#[test]
fn the_summary_words_what_you_chose() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let repo = scratch.path().to_owned();
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('t')));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = advanced_to(&app, state, "gone.txt");
    let state = gone_kept_last(&app, &state);
    let state = c_on_the_banner(&app, state);

    let notes = row_for(&state, "notes.txt");
    assert!(
        notes.contains("theirs · feature in 2 of 2 conflicts"),
        "{notes}"
    );
    let gone = row_for(&state, "gone.txt");
    assert!(gone.contains("kept · yours · main"), "{gone}");
    drop(state);
    app.close();

    // A relaunch remembers no choices: the rows say what the result is.
    let (app, state) = App::open(&repo);
    let state = app.pump(state, "the resolved merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    assert!(state.choices.is_empty(), "nothing carried across");
    let state = c_on_the_banner(&app, state);
    let notes = row_for(&state, "notes.txt");
    assert!(notes.contains("same as theirs · feature"), "{notes}");
    let gone = row_for(&state, "gone.txt");
    assert!(gone.contains("same as yours · main"), "{gone}");
    drop(state);
    app.close();
}

#[test]
fn shift_t_records_its_choices() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let state = both_resolved_in_the_app(&app, state);
    let state = c_on_the_banner(&app, state);

    let notes = row_for(&state, "notes.txt");
    assert!(
        notes.contains("theirs · feature in 2 of 2 conflicts"),
        "{notes}"
    );
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn a_start_over_forgets_the_choices() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let state = app.key(&state, KeyCode::Char('t'));
    let state = crate::resolver::settled(&app, state);
    assert!(
        state.choices.contains_key(Path::new("notes.txt")),
        "the take was recorded: {:?}",
        state.choices
    );

    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some(), "`d` asks first");
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the restart", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none() && open.file.is_some())
    });
    assert_eq!(
        crate::resolver::open(&state)
            .file
            .as_ref()
            .map(|file| file.open()),
        Some(2),
        "the file is conflicted afresh"
    );
    assert!(
        !state.choices.contains_key(Path::new("notes.txt")),
        "no choices for a file started over: {:?}",
        state.choices
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// The app's own merge of [`about_to_merge`], landed in the resolver, and
/// `T` on each file until the last closes to the banner: `choices` holds
/// both. A merge begun in the shell is not the app's to commit or abort.
fn every_file_theirs(app: &App, state: AppState) -> AppState {
    let mut state = crate::conflict_interim::landed_in_the_resolver(app, state);
    let mut guard = 0;
    while let Some(was) = state.resolver.as_ref().map(|open| open.path.clone()) {
        assert!(guard < 4, "`T` never closed the resolver");
        guard += 1;
        state = app.press(&state, KeyPress::plain(KeyCode::Char('T')));
        state = app.pump(state, "the file done and the next", |state| {
            state.operating.is_none()
                && state.resolver.as_ref().is_none_or(|open| {
                    open.path != was
                        && open.job.is_none()
                        && (open.file.is_some() || open.whole_file.is_some())
                })
        });
    }
    app.pump(state, "the status the last stage asked for", |state| {
        state.status_read.is_none()
    })
}

/// [`every_file_theirs`], then the summary opened, a subject typed, `Esc`:
/// the draft is kept.
fn choices_and_a_draft(app: &App, state: AppState) -> AppState {
    let state = every_file_theirs(app, state);
    let state = c_on_the_banner(app, state);
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(app, state, " (draft)");
    let state = app.key(&state, KeyCode::Escape);
    assert_eq!(state.choices.len(), 2, "{:?}", state.choices);
    assert!(state.finish_draft.is_some(), "the draft is kept");
    state
}

#[test]
fn choices_and_draft_clear_when_the_merge_ends() {
    // Done: the merge commits.
    let scratch = about_to_merge();
    let (app, state) = App::open(scratch.path());
    let state = choices_and_a_draft(&app, state);
    let state = c_on_the_banner(&app, state);
    let state = committed(&app, &state, KeyPress::plain(KeyCode::Enter));
    assert!(
        !scratch.path().join(".git/MERGE_HEAD").exists(),
        "the merge committed: {:?}",
        state.finish.as_ref().map(|open| &open.notice)
    );
    assert!(state.choices.is_empty(), "after Done: {:?}", state.choices);
    assert_eq!(state.finish_draft, None, "after Done");
    drop(state);
    app.close();
    drop(scratch);

    // Abort: the merge is thrown away.
    let scratch = about_to_merge();
    let (app, state) = App::open(scratch.path());
    let state = choices_and_a_draft(&app, state);
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(state.confirm.is_some(), "`a` asks first");
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the abort", |state| state.operating.is_none());
    assert!(
        !scratch.path().join(".git/MERGE_HEAD").exists(),
        "the merge is over"
    );
    assert!(
        state.choices.is_empty(),
        "after an abort: {:?}",
        state.choices
    );
    assert_eq!(state.finish_draft, None, "after an abort");
    drop(state);
    app.close();
    drop(scratch);
}

fn edited(state: &AppState, path: &str) -> bool {
    state
        .choices
        .get(Path::new(path))
        .is_some_and(|choices| choices.edited)
}

fn handed_back(state: &AppState, result: Result<(), String>) -> AppState {
    let (state, _rereads) = update(
        state,
        AppEvent::Produced(graft_core::job::JobId::new(0), Product::HandedOver(result)),
    );
    state
}

/// The terminal handovers that mark a file `edited in your editor`: a
/// clean return from `$EDITOR` (the changes pane's `e`, the resolver's `e`)
/// or the mergetool. The harness cannot hand the terminal over, so each
/// command is answered the way the loop answers it (`event_loop::running`).
#[test]
fn a_clean_handover_marks_the_file_edited() {
    let (scratch, app, state) = stopped_on_gone_and_notes();

    // The changes pane's `e` on `notes.txt`: a failed handover marks nothing.
    let state = crate::resolver::on_the_row(&app, state, "notes.txt");
    let (state, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflictEditor { .. })),
        "{commands:?}"
    );
    let state = handed_back(&state, Err("vi exited 1, message unchanged".to_owned()));
    assert!(!edited(&state, "notes.txt"), "a failed handover");
    assert_eq!(state.conflict_editing, None, "the path is spent");
    let (state, _) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    let state = handed_back(&state, Ok(()));
    assert!(edited(&state, "notes.txt"), "{:?}", state.choices);

    // The mergetool on `gone.txt`.
    let (state, _rereads) = update(
        &state,
        AppEvent::Produced(
            graft_core::job::JobId::new(0),
            Product::Mergetool {
                path: "gone.txt".into(),
                result: Ok(()),
            },
        ),
    );
    assert!(edited(&state, "gone.txt"), "{:?}", state.choices);
    drop(state);
    app.close();

    // The resolver's own `e`, in a fresh session.
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let (state, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenExternalEditor { .. })),
        "{commands:?}"
    );
    let state = handed_back(&state, Ok(()));
    assert!(edited(&state, "notes.txt"), "{:?}", state.choices);
    drop(state);
    app.close();
    drop(scratch);
}

/// The finish summary the user judges by eye (m6-conflicts/19, built by
/// 35): `notes.txt` taken theirs in both blocks (`Ctrl+T ⏎`), `gone.txt`
/// kept yours (`o`), `README.md` combined by git. Colour unverified
/// (rule 5); the look is the user's (rule 6).
#[test]
fn summary_captures() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let repo = scratch.path().to_owned();
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('t')));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = advanced_to(&app, state, "gone.txt");
    let state = gone_kept_last(&app, &state);
    let state = c_on_the_banner(&app, state);

    closed_captures(&state, &repo);
    let state = expanded_capture(&app, &state);
    drop(state);
    app.close();
    drop(scratch);
}

/// The three widths, nothing expanded: [`summary_captures`]' first half.
fn closed_captures(state: &AppState, repo: &Path) {
    let merge_msg = std::fs::read_to_string(repo.join(".git/MERGE_MSG")).expect("git wrote it");
    let first = merge_msg.lines().next().expect("a first line").to_owned();
    for (columns, rows) in [(120, 40), (160, 48), (200, 50)] {
        let frame = crate::captures::frame(state, columns, rows);
        let body = frame.join("\n");
        assert!(
            body.contains("Finish · merging feature → main · nothing is written until ⏎"),
            "{columns}: {body}"
        );
        assert!(body.contains("RESOLVED BY YOU · 2"), "{columns}: {body}");
        assert!(
            body.contains("COMBINED BY GIT ON ITS OWN · 1"),
            "{columns}: {body}"
        );
        let readme = frame
            .iter()
            .find(|line| line.contains("README.md"))
            .unwrap_or_else(|| panic!("{columns}: no README.md row: {body}"));
        assert!(
            readme.contains("▸ README.md")
                && readme.contains("modified")
                && readme.contains("+1 −1"),
            "{columns}: {readme}"
        );
        assert!(
            body.contains(&first),
            "{columns}: git's first line {first:?}"
        );
        let bar = frame.last().expect("a bar");
        assert!(bar.trim_start().starts_with("⏎ commit"), "{columns}: {bar}");
        crate::captures::check_shot(
            &format!("m6/fix-finish-summary-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
}

/// `space` on the cursor's row: [`summary_captures`]' second half.
fn expanded_capture(app: &App, state: &AppState) -> AppState {
    // `space` on `README.md`, the cursor's row: its diff under it.
    let state = app.key(state, KeyCode::Char(' '));
    let state = app.pump(state, "the README.md diff", |state| {
        state
            .finish
            .as_ref()
            .is_some_and(|open| open.diff_job.is_none() && open.expanded.is_some())
    });
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(!body.contains("load all"), "no `L load all`: {body}");
    let at = frame
        .iter()
        .position(|line| line.contains("▸ README.md"))
        .expect("the README.md row");
    // The first row under it is the hunk's heading row, blank here (git
    // gives this hunk no function context); the diff lines follow it.
    let under: Vec<&String> = frame
        .iter()
        .skip(at + 1)
        .take_while(|line| !line.contains("MESSAGE"))
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert!(
        !under.is_empty() && under.iter().all(|line| line.starts_with("    ")),
        "the diff sits under the row, indented: {body}"
    );
    assert!(
        under.iter().any(|line| line.contains("- line two"))
            && under.iter().any(|line| line.contains("+ line TWO")),
        "the change itself: {body}"
    );
    crate::captures::check_shot(
        "m6/fix-finish-summary-expanded-120x40.txt",
        &format!("{body}\n"),
    );
    state
}
