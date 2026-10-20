//! C1–C15: the commit mock's behaviours on the mock-shaped repository with
//! its working tree (three edited files and a new `.env`), one test per id.

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use super::fixture::{self, SECRET_VALUE, mock, with_edits};
use super::screen::{bar, buffer, drawn, line_after, line_with, open, shown, top, typed};
use crate::app_harness::{App, git};

/// The mock repository with its edits, opened at 120 and at rest.
fn edited() -> (tempfile::TempDir, App, AppState) {
    let scratch = mock();
    with_edits(scratch.path());
    let (app, state) = open(scratch.path(), 120);
    (scratch, app, state)
}

/// `c`, then wait for the line counts and the first file's diff.
pub fn opened(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the commit screen");
    settled(app, state)
}

/// The commit screen with its counts and diff read.
pub fn settled(app: &App, state: AppState) -> AppState {
    app.pump(state, "the counts and the diff", |state| {
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.counts.is_some() && editor.diff.view.is_some())
            && state.commit_template.is_none()
    })
}

/// The commit landed, the graph re-read, the cursor on it, at rest.
pub fn landed(app: &App, state: AppState) -> AppState {
    let state = app.settle_write(state, "the commit");
    let state = app.pump(state, "the graph on the new commit", |state| {
        super::screen::cursor_on_head(state) && state.last_commit.is_some()
    });
    crate::captures::at_rest(app, state)
}

/// `c`, `subject`, `⏎`: his commit, landed.
pub fn committed(app: &App, state: &AppState, subject: &str) -> AppState {
    let state = opened(app, state);
    let state = typed(app, &state, subject);
    landed(app, app.key(&state, KeyCode::Enter))
}

/// In the file list, every going-in file left out: the cursor follows a
/// moved file into `Staying out`, so each `space` starts from the top
/// (as `commit_screen/shots.rs:130-134` does).
pub fn leave_all_out(app: &App, mut files: AppState) -> AppState {
    for _ in 0..3 {
        for _ in 0..4 {
            files = app.key(&files, KeyCode::Char('k'));
        }
        files = settled(app, app.key(&files, KeyCode::Char(' ')));
    }
    files
}

fn committed_files(repo: &std::path::Path) -> Vec<String> {
    let mut files: Vec<String> = git(repo, &["show", "--name-only", "--format=", "HEAD"])
        .lines()
        .map(str::to_owned)
        .collect();
    files.sort();
    files
}

/// C1: `c` puts the cursor in the message with its placeholder; the header
/// reads `committing to wallet-release`.
#[test]
fn c1_c_opens_on_the_message_with_its_placeholder() {
    let (_scratch, app, state) = edited();
    let frame = drawn(&opened(&app, &state));
    assert!(
        top(&frame)
            .trim_end()
            .ends_with("committing to wallet-release"),
        "{:?}",
        top(&frame)
    );
    assert!(line_with(&frame, "What did you change? (one line)").contains("▸What did you change?"));
    app.close();
}

/// C2: the subject draws bold with its `n/72` counter; past 72 the counter
/// says so.
#[test]
fn c2_subject_bold_counter_and_warning_past_72() {
    let (_scratch, app, state) = edited();
    let state = typed(&app, &opened(&app, &state), "fix toc");
    let frame = drawn(&state);
    let subject = line_with(&frame, "fix toc");
    assert!(subject.trim_end().ends_with("7/72 │"), "{subject:?}");
    let y = u16::try_from(super::screen::row_of(&frame, "fix toc")).unwrap_or(0);
    let x = u16::try_from(subject.chars().position(|c| c == 'f').unwrap_or(0)).unwrap_or(0);
    let cells = buffer(&state);
    assert!(
        cells[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::BOLD),
        "bold subject"
    );
    let long = typed(&app, &state, &"x".repeat(70));
    let frame = drawn(&long);
    let counter = line_with(&frame, "77/72");
    assert!(counter.contains("77/72"), "{}", shown(&frame));
    app.close();
}

/// C3: the body's placeholder.
#[test]
fn c3_body_placeholder() {
    let (_scratch, app, state) = edited();
    let frame = drawn(&opened(&app, &state));
    assert!(
        line_with(&frame, "More detail, if it needs it (optional)").starts_with("│  More detail")
    );
    app.close();
}

/// C4: the summary line, and its words when nothing goes in.
#[test]
fn c4_summary_line() {
    let (_scratch, app, state) = edited();
    let state = opened(&app, &state);
    let frame = drawn(&state);
    assert!(
        frame.iter().any(|line| line.trim()
            == "3 files go in, +4 −0, as a new commit on wallet-release. 1 stays out."),
        "{}",
        shown(&frame)
    );
    let files = app.key(&app.key(&state, KeyCode::Tab), KeyCode::Tab);
    let files = leave_all_out(&app, files);
    let frame = drawn(&files);
    assert!(
        frame
            .iter()
            .any(|line| line.trim() == "Nothing goes in. Put a file in with space."),
        "{}",
        shown(&frame)
    );
    app.close();
}

/// C5: `Going in (n)` with `✓`, `Staying out (n)` with `·`; an empty group
/// says `nothing yet` / `nothing`.
#[test]
fn c5_two_groups_and_their_marks() {
    let (_scratch, app, state) = edited();
    let state = opened(&app, &state);
    let frame = drawn(&state);
    assert!(
        line_with(&frame, "Going in (3)")
            .trim_start()
            .starts_with("Going in (3)")
    );
    assert!(line_with(&frame, "M src/a.rs").contains("✓ M src/a.rs"));
    assert!(
        line_with(&frame, "Staying out (1)")
            .trim_start()
            .starts_with("Staying out (1)")
    );
    let files = app.key(&app.key(&state, KeyCode::Tab), KeyCode::Tab);
    let out = settled(&app, app.key(&files, KeyCode::Char(' ')));
    assert!(line_with(&drawn(&out), "M README.md").contains("· M README.md"));
    let frame = drawn(&leave_all_out(&app, files));
    assert!(
        line_after(&frame, "Going in (0)").contains("nothing yet"),
        "{}",
        shown(&frame)
    );
    app.close();
}

/// C6: nothing staged → every change goes in; something staged → only it.
#[test]
fn c6_staged_decides_what_goes_in() {
    let scratch = mock();
    with_edits(scratch.path());
    git(scratch.path(), &["add", "src/a.rs"]);
    let (app, state) = open(scratch.path(), 120);
    let frame = drawn(&opened(&app, &state));
    assert!(
        frame.iter().any(|line| line.contains("Going in (1)")),
        "{}",
        shown(&frame)
    );
    assert!(frame.iter().any(|line| line.contains("Staying out (3)")));
    app.close();
}

/// C7: a new secrets-looking file stays out, `!` and `looks like a secrets
/// file`; its diff opens with `! left out: … space puts it in.` and its
/// value is never drawn.
#[test]
fn c7_secrets_file_stays_out_and_says_why() {
    let (_scratch, app, state) = edited();
    let mut state = opened(&app, &state);
    for _ in 0..3 {
        state = settled(&app, app.key(&state, KeyCode::Down));
    }
    let frame = drawn(&state);
    assert!(line_with(&frame, ".env").contains("! A .env  looks like a secrets file"));
    let note = line_with(&frame, "! left out:");
    assert!(note.contains("space puts it in."), "{note:?}");
    assert!(
        !shown(&frame).contains(SECRET_VALUE),
        "the value is never drawn"
    );
    app.close();
}

/// C8: the diff shows the changed file's `+` lines.
#[test]
fn c8_diff_lines() {
    let (_scratch, app, state) = edited();
    let frame = drawn(&opened(&app, &state));
    assert!(
        line_with(&frame, "+ more").contains("2 + more"),
        "{}",
        shown(&frame)
    );
    app.close();
}

/// C9: `⏎` commits only with a message and a file; else the bar says which.
#[test]
fn c9_enter_refuses_until_message_and_file() {
    let (scratch, app, state) = edited();
    let head = super::screen::head_subject(scratch.path());
    let state = opened(&app, &state);
    let refused = app.key(&state, KeyCode::Enter);
    assert!(bar(&drawn(&refused)).contains("⏎ type a message first"));
    assert_eq!(super::screen::head_subject(scratch.path()), head);
    let typed_one = typed(&app, &refused, "fix toc");
    let files = app.key(&app.key(&typed_one, KeyCode::Tab), KeyCode::Tab);
    let files = leave_all_out(&app, files);
    let message = app.key(&files, KeyCode::Escape);
    let refused = app.key(&message, KeyCode::Enter);
    assert!(
        bar(&drawn(&refused)).contains("⏎ put a file in first"),
        "{}",
        bar(&drawn(&refused))
    );
    assert_eq!(super::screen::head_subject(scratch.path()), head);
    app.close();
}

/// The file cursor's row: the line holding `›` or `▸` in the files list.
fn file_cursor(frame: &[String]) -> String {
    frame
        .iter()
        .find(|line| line.starts_with(" ›") || line.starts_with(" ▸"))
        .cloned()
        .unwrap_or_default()
}

/// C10: `↑ ↓` in the message move the file cursor; focus stays.
#[test]
fn c10_arrows_in_the_message_move_the_file_cursor() {
    let (_scratch, app, state) = edited();
    let state = opened(&app, &state);
    assert!(file_cursor(&drawn(&state)).contains("README.md"));
    let down = settled(&app, app.key(&state, KeyCode::Down));
    let frame = drawn(&down);
    assert!(
        file_cursor(&frame).contains("docs/toc.md"),
        "{}",
        shown(&frame)
    );
    let typed_one = typed(&app, &down, "x");
    assert!(
        line_with(&drawn(&typed_one), "1/72").contains("▸x"),
        "focus stays in the message"
    );
    app.close();
}

/// C11: `⇥` message → body → files; `esc` in the body → message; in the
/// files `j k` move, `space` leaves one out, `esc` → message.
#[test]
fn c11_tab_cycles_and_esc_returns_to_the_message() {
    let (_scratch, app, state) = edited();
    let state = typed(&app, &opened(&app, &state), "fix toc");
    let body = app.key(&state, KeyCode::Tab);
    assert!(line_with(&drawn(&body), "More detail").contains("▸More detail"));
    let back = app.key(&body, KeyCode::Escape);
    assert!(line_with(&drawn(&back), "fix toc").contains("▸fix toc"));
    let files = app.key(&body, KeyCode::Tab);
    assert!(file_cursor(&drawn(&files)).starts_with(" ▸"));
    let moved = settled(&app, app.key(&files, KeyCode::Char('j')));
    assert!(file_cursor(&drawn(&moved)).contains("docs/toc.md"));
    let out = settled(&app, app.key(&moved, KeyCode::Char(' ')));
    assert!(line_with(&drawn(&out), "docs/toc.md").contains("· M docs/toc.md"));
    let back = app.key(&out, KeyCode::Escape);
    assert!(line_with(&drawn(&back), "fix toc").contains("▸fix toc"));
    app.close();
}

/// C12: the three bars in the mock's words.
#[test]
fn c12_the_three_bars() {
    let (_scratch, app, state) = edited();
    let state = typed(&app, &opened(&app, &state), "fix toc");
    assert_eq!(
        bar(&drawn(&state)).trim(),
        "⏎ commit 3 files  ⇥ more detail  ↑/↓ see a file's changes  esc cancel, your message is kept"
    );
    let body = app.key(&state, KeyCode::Tab);
    assert_eq!(
        bar(&drawn(&body)).trim(),
        "esc back to the message line  ⇥ files"
    );
    let files = app.key(&body, KeyCode::Tab);
    assert_eq!(
        bar(&drawn(&files)).trim(),
        "space leave it out  j/k move  ⏎ commit  esc back to the message"
    );
    app.close();
}

/// C13: the landing: the done line, the bar, the header's `1 commit to
/// push`.
#[test]
fn c13_landing_line_bar_and_header() {
    let (scratch, app, state) = edited();
    let state = committed(&app, &state, "fix toc");
    let frame = drawn(&state);
    assert_eq!(
        super::screen::status(&frame).trim(),
        "✓ Committed 3 files to wallet-release.  u undo · P push"
    );
    assert!(
        bar(&frame)
            .trim_start()
            .starts_with("u undo the commit, your edits come back · P push · c commit again"),
        "{:?}",
        bar(&frame)
    );
    assert!(
        top(&frame).contains("1 commit to push"),
        "{:?}",
        top(&frame)
    );
    assert_eq!(
        committed_files(scratch.path()),
        ["README.md", "docs/toc.md", "src/a.rs"]
    );
    app.close();
}

/// C14: `u` after the commit gives the edits back uncommitted; `c` again
/// opens on the old message.
#[test]
fn c14_u_then_c_after_the_commit() {
    let (scratch, app, state) = edited();
    let state = committed(&app, &state, "fix toc");
    let undone = app.settle_write(app.key(&state, KeyCode::Char('u')), "the undo");
    assert_eq!(
        super::screen::head_subject(scratch.path()),
        fixture::RELEASE_REPORT
    );
    let status = git(scratch.path(), &["status", "--porcelain"]);
    assert_eq!(status.lines().count(), 4, "{status}");
    let again = opened(&app, &crate::captures::at_rest(&app, undone));
    assert!(line_with(&drawn(&again), "fix toc").contains("▸fix toc"));
    app.close();
}

/// C15: `esc` in the message closes the screen; the message comes back with
/// the next `c`.
#[test]
fn c15_esc_keeps_the_message_for_the_next_c() {
    let (_scratch, app, state) = edited();
    let state = typed(&app, &opened(&app, &state), "half a thought");
    let closed = app.key(&state, KeyCode::Escape);
    assert!(closed.commit_editor.is_none());
    let again = opened(&app, &closed);
    assert!(line_with(&drawn(&again), "half a thought").contains("▸half a thought"));
    app.close();
}
