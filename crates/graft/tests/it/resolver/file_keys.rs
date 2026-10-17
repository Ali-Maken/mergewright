//! `O`/`T` settle a whole file and move on (m6-conflicts/17), against a
//! real stopped merge: `gone.txt` (a whole-file card) and `notes.txt` (two
//! blocks). A merge stages a marker file at the review's `y`, not here
//! (ease-pass/05b); a whole-file card's choice still stages.

use super::file_list::gone_and_notes_merge;
use super::*;
use crate::app_harness::git;

/// The resolver on `notes.txt` in [`gone_and_notes_merge`].
fn on_notes() -> (Scratch, App, AppState) {
    let scratch = gone_and_notes_merge();
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = resolver_on(&app, state, "notes.txt");
    (scratch, app, state)
}

/// Waits for the resolver to land on `path`, read, with no write out.
fn landed_on(app: &App, state: AppState, path: &str, card: bool) -> AppState {
    app.pump(state, "the next file", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new(path)
                    && open.job.is_none()
                    && open.set_job.is_none()
                    && if card {
                        open.whole_file.is_some()
                    } else {
                        open.file.is_some()
                    }
            })
    })
}

fn notice(state: &AppState) -> Option<String> {
    open(state).notice.as_ref().map(|(words, _)| words.clone())
}

fn per_block_theirs(repo: &Path) -> Vec<u8> {
    let bytes = std::fs::read(repo.join("notes.txt")).expect("the conflicted file");
    let mut file = graft_git::conflict::parse(&bytes).expect("it parses");
    file.take_all(graft_git::conflict::Take::Theirs);
    file.render()
}

fn shift(code: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(code))
}

#[test]
fn shift_t_takes_theirs_and_moves_on_staging_nothing_in_a_merge() {
    let (scratch, app, state) = on_notes();
    let repo = scratch.path().to_owned();
    let want = per_block_theirs(&repo);
    let state = app.press(&state, shift('T'));
    let state = landed_on(&app, state, "gone.txt", true);
    assert_eq!(
        std::fs::read(repo.join("notes.txt")).expect("it reads"),
        want
    );
    let unmerged = git(&repo, &["ls-files", "-u", "--", "notes.txt"]);
    assert!(!unmerged.trim().is_empty(), "notes.txt was staged");
    let notes = super::file_list::rows(&state)
        .into_iter()
        .find(|(path, _)| path == "notes.txt");
    assert_eq!(
        notes,
        Some((
            "notes.txt".to_owned(),
            graft::update::resolver::RowState::AllTaken
        )),
        "solved, waiting for the review"
    );
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn shift_o_on_a_whole_file_card_keeps_yours_and_moves_on() {
    let scratch = gone_and_notes_merge();
    let repo = scratch.path().to_owned();
    let (app, state) = App::open(&repo);
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = resolver_on_whole_file(&app, state, "gone.txt");
    let state = app.press(&state, shift('O'));
    let state = landed_on(&app, state, "notes.txt", false);
    let head = git(&repo, &["show", "HEAD:gone.txt"]);
    assert_eq!(on_disk(&repo, "gone.txt"), head, "HEAD's blob");
    let unmerged = git(&repo, &["ls-files", "-u", "--", "gone.txt"]);
    assert!(unmerged.trim().is_empty(), "still unmerged: {unmerged}");
    let staged = git(&repo, &["ls-files", "--stage", "--", "gone.txt"]);
    assert!(staged.contains(" 0\tgone.txt"), "staged: {staged}");
    drop(state);
    app.close();
    drop(scratch);
}

/// `t` on each block, both landed.
fn every_block_taken(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('t'));
    let state = settled(app, state);
    let state = app.key(&state, KeyCode::Char('t'));
    let state = settled(app, state);
    assert_eq!(open(&state).file.as_ref().expect("a document").open(), 0);
    state
}

/// In a merge, `T` with nothing left to take writes nothing and stages
/// nothing (ease-pass/05b): it goes on, as `⏎` does.
#[test]
fn shift_t_with_every_block_taken_only_moves_on() {
    let (scratch, app, state) = on_notes();
    let state = every_block_taken(&app, &state);
    let (_, commands) = press_for_commands(&state, shift('T'));
    let stages = commands
        .iter()
        .filter(|command| {
            matches!(command, Command::Operate(_, op, ..) if matches!(**op, Operation::Stage(_)))
        })
        .count();
    let takes = commands
        .iter()
        .filter(|command| matches!(command, Command::TakeSide(..)))
        .count();
    assert_eq!((stages, takes), (0, 0), "{commands:?}");
    let by_t = landed_on(&app, app.press(&state, shift('T')), "gone.txt", true);
    app.close();
    drop(scratch);

    // The same steps with `⏎` in place of `T`.
    let (scratch, app, state) = on_notes();
    let state = every_block_taken(&app, &state);
    let by_enter = landed_on(&app, app.key(&state, KeyCode::Enter), "gone.txt", true);
    assert_eq!(open(&by_t).path, open(&by_enter).path);
    assert_eq!(notice(&by_t), notice(&by_enter));
    assert_eq!(by_t.status, by_enter.status);
    app.close();
    drop(scratch);
}

/// `T` on `notes.txt`, landed on `gone.txt`: the captures the user judges
/// by eye (m6-conflicts/17). Colour unverified (rule 5).
#[test]
fn file_keys_capture() {
    let (scratch, app, state) = on_notes();
    // (a) Before `T`, on `notes.txt`: the bar offers both file keys.
    let before = crate::captures::frame(&state, 120, 40);
    let bar = before.last().expect("a bar").clone();
    assert!(bar.contains("O/T") && bar.contains("j/k"), "{bar}");
    crate::captures::check_shot(
        "m6/fix-file-keys-before-120x40.txt",
        &format!("{}\n", before.join("\n")),
    );
    let before = crate::captures::frame(&state, 200, 50);
    let bar = before.last().expect("a bar").clone();
    for hint in ["O/T file", "⏎ done", "j/k file"] {
        assert!(bar.contains(hint), "{hint}: {bar}");
    }
    crate::captures::check_shot(
        "m6/fix-file-keys-before-200x50.txt",
        &format!("{}\n", before.join("\n")),
    );

    // (b) After `T`: landed on the `gone.txt` card, `notes.txt` solved and
    // not done — a merge stages it at the review's `y` (ease-pass/05b).
    let state = app.press(&state, shift('T'));
    let state = landed_on(&app, state, "gone.txt", true);
    // `PgUp` changes nothing on a card but clears any status line, so the
    // bottom row is the card's bar (review m6-amend-17 C1).
    let state = app.key(&state, KeyCode::PageUp);

    // At 120 the files are a strip; at 200 a side list, wide enough for
    // every name beside the widest state (m6-conflicts/42).
    for (columns, rows, shot, done_row) in [
        (120, 40, "m6/fix-file-keys-120x40.txt", "notes.txt · done ✓"),
        (200, 50, "m6/fix-file-keys-200x50.txt", "notes.txt  done ✓"),
    ] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        assert!(!body.contains(done_row), "strip: {body}");
        assert!(body.contains("notes.txt"), "strip: {body}");
        assert!(
            frame
                .iter()
                .flat_map(|line| line.split('│'))
                .any(|cell| cell.trim_start().starts_with("▸ gone.txt · modified")),
            "identity row: {body}"
        );
        let bar = frame.last().expect("a bar");
        assert!(
            !bar.contains("O/T") && !bar.contains("j/k"),
            "{columns}: {bar}"
        );
        assert!(
            bar.contains("o ") && bar.contains("esc"),
            "{columns}: {bar}"
        );
        crate::captures::check_shot(shot, &format!("{body}\n"));
    }
    drop(state);
    app.close();
    drop(scratch);
}
