//! The confirm box and the credential prompt photographed **over real
//! content** (m5-history-and-ghost-preview/26, retaken by 28).
//!
//! The question these captures were built to answer was the user's, open
//! since ticket 13: a bordered box has an explicit edge, so whatever
//! survives beside it reads as background — *is that surviving content
//! aligned and legible, or fragmentary?* He answered it from the 80x24 pair
//! and the answer is **ADR-0040**: a non-clearing box covers whole columns
//! or it clears. So these are no longer pictures of an open question. They
//! are pictures of the decided behaviour, and the writer asserts it before
//! writing: the confirm box snaps onto column boundaries and keeps the
//! graph beside it, the credential prompt clears the band it stands on.
//!
//! Every frame is drawn from a real `AppState` over a real repository and a
//! real `file://` remote, at a fixed clock, with the scratch path shortened
//! to `/code/api` before the frame is laid out.
#![allow(
    clippy::expect_used,
    reason = "a capture that cannot be drawn has nothing to show; the panic names why"
)]

use std::path::{Path, PathBuf};

use mergewright::history::Phase;
use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright::update::detail::Loading;
use mergewright_core::event::KeyCode;
use mergewright_core::journal::OperationKind;
use mergewright_fixtures::Shape;
use mergewright_fixtures::remote::LocalRemote;

use crate::app_harness::App;
use crate::captures::{check_shot, frame_at};

/// The instant every frame's relative dates are drawn against. The corpus's
/// own "now", so a capture taken today and one taken next year are the same
/// bytes (ticket 23).
const NOW: Clock = Clock::Fixed(mergewright_fixtures::CORPUS_NOW);

/// The path the header is laid out against, in place of the ~50-character
/// scratch directory the fixture really lives in. Replaced **before** the
/// frame is drawn, so the width the header spends is the width a real
/// checkout spends (the banner captures' lesson, m5/16).
const HOME: &str = "/code/api";

/// The host git's question names. Pinned: a real HTTP fixture is served on
/// a port the OS picks, and a capture whose title changed every run could
/// not be diffed (requirement 3 of the ticket).
const ASK: &str = "Password for 'http://user@127.0.0.1:8443':";

/// What is typed into the field. Seven characters, so the eight fixed mask
/// glyphs ADR-0038 §2 draws cannot be read as a length.
const TYPED: &str = "hunter2";

/// A work clone one commit **ahead** of its remote, without making a
/// commit: the bare's branch and the clone's tracking ref are both rewound
/// one, so the tip `P` offers to push is a corpus commit whose oid, author
/// and date are the corpus's. Committing here instead would stamp the new
/// commit with the wall clock and give the capture a different sha every
/// run.
fn one_commit_to_push() -> LocalRemote {
    let remote =
        LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote");
    let branch = remote.default_branch().to_owned();
    let work = remote.work().to_owned();
    let bare = remote.bare().to_owned();
    // Mergewright's own git keeps the user's config: a global hooks path or a
    // missing identity must not reach this clone.
    for args in [
        ["config", "core.hooksPath", ".git/hooks"],
        ["config", "user.name", "Graft Test"],
        ["config", "user.email", "test@example.invalid"],
    ] {
        run(&remote, &work, &args);
    }
    let head = format!("refs/heads/{branch}");
    let tracking = format!("refs/remotes/origin/{branch}");
    run(&remote, &bare, &["update-ref", &head, &format!("{head}^")]);
    run(
        &remote,
        &work,
        &["update-ref", &tracking, &format!("{tracking}^")],
    );
    remote
}

fn run(remote: &LocalRemote, dir: &Path, args: &[&str]) {
    remote
        .git(dir, args)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
}

/// Everything `P` decides from has landed, and the walk behind it has
/// stopped moving: rows, refs, the working tree, the remote facts, a
/// **complete** walk and a finished promotion.
///
/// The last two are the capture's, not `P`'s. A walk still streaming draws
/// `N rows so far  date order, sorting\u{2026}` in the header, and `N` is
/// however far the background sort happened to be when the frame was taken
/// \u{2014} which is a different number every run (ticket 26, requirement 3).
///
/// And the radar's answer: the tip row's `ⓘ` is its mark, from a job of its
/// own, so a frame taken before it lands lacks it (`KNOWN_GAPS` M7/04b: half
/// the runs under load).
fn ready(app: &App, state: AppState) -> AppState {
    let state = app.pump(
        state,
        "the settled walk, refs, working tree and remote facts",
        |state| {
            state.working_tree.is_some()
                && state.refs.is_some()
                // The footer's commit-graph line and the graph's height
                // wait on `git --version` (`update/state.rs` `commit_graph_line`).
                && state.system_git.is_some()
                && matches!(state.remotes, Loading::Ready(_))
                && state.remote.reading.is_none()
                && state.history.as_ref().is_some_and(|history| {
                    history.phase == Phase::Complete
                        && history.promotion.is_none()
                        && history
                            .window
                            .as_ref()
                            .is_some_and(|window| !window.rows.is_empty())
                })
        },
    );
    crate::captures::radar_answered(app, state)
}

/// The journal read that carries the push, so `u` decides from it — and the
/// refreshed tracking tip the lease is taken against.
fn journal_has_the_push(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the journal with the push", |state| {
        matches!(&state.journal.entries, Loading::Ready(read)
            if read.entries.iter().any(|entry| entry.kind == OperationKind::Push))
            && state.journal.reading.is_none()
    });
    ready(app, state)
}

/// The rows `view::view` draws for `state` at this size, with the scratch
/// path replaced by [`HOME`] before the layout is decided.
fn shot(app: &App, state: &AppState, columns: u16, rows: u16, scratch: &Path) -> Vec<String> {
    let mut state = state.clone();
    state.repo_path = PathBuf::from(HOME);
    if let Some(facts) = state.repository.as_mut() {
        facts.path = PathBuf::from(HOME);
    }
    let sized = app.resize(&state, columns, rows);
    let home = scratch.display().to_string();
    frame_at(&sized, columns, rows, Some(NOW))
        .into_iter()
        .map(|line| line.replace(&home, HOME))
        .collect()
}

/// How much of the frame is not the box: the rows that still carry graph
/// text outside the box's own columns. Both boxes are judged by this one
/// number, in opposite directions (ADR-0040) — the confirm box must leave
/// rows standing beside it, the credential prompt must leave none.
fn rows_with_content_beside_the_box(drawn: &[String]) -> usize {
    drawn
        .iter()
        .map(|line| without_pane_rule(line))
        .filter(|line| {
            let Some(edge) = line.find(['\u{256d}', '\u{2502}', '\u{2570}']) else {
                return false;
            };
            !line[..edge].trim().is_empty() && line.contains('\u{2502}')
        })
        .count()
}

/// The widest strip of graph the box left showing, and the row's tail on
/// the other side of it, **read back out of the frame** rather than written
/// by hand. The tail is trimmed: a run of blank cells is not content that
/// survived.
///
/// This is the whole of what the user is judging, so the caption states it —
/// and three captions in this milestone went stale because a caption stated
/// something the code no longer did. A line derived from the picture cannot.
fn what_survives(drawn: &[String]) -> (String, String) {
    const LEFT_EDGES: [char; 3] = ['\u{256d}', '\u{2502}', '\u{2570}'];
    const RIGHT_EDGES: [char; 3] = ['\u{256e}', '\u{2502}', '\u{256f}'];
    let mut widest = (String::new(), String::new());
    for line in drawn {
        let cells: Vec<char> = without_pane_rule(line).chars().collect();
        let (Some(first), Some(last)) = (
            cells.iter().position(|cell| LEFT_EDGES.contains(cell)),
            cells.iter().rposition(|cell| RIGHT_EDGES.contains(cell)),
        ) else {
            continue;
        };
        let left: String = cells[..first].iter().collect();
        if left.trim_end().chars().count() > widest.0.chars().count() {
            let right: String = cells[last + 1..].iter().collect();
            widest = (left.trim_end().to_owned(), right.trim().to_owned());
        }
    }
    widest
}

/// `line` (a full-width frame row) with the branch pane's rule blanked
/// (ease-pass 09): it stands on every body row, and read as a box's edge
/// it would make every row a box row.
fn without_pane_rule(line: &str) -> String {
    let mut cells: Vec<char> = line.chars().collect();
    let columns = u16::try_from(cells.len()).unwrap_or(u16::MAX);
    if let Some(width) = mergewright_ui::branch_pane::pane_width(columns) {
        let rule = usize::from(columns - width - 2);
        if cells.get(rule) == Some(&'\u{2502}') {
            cells[rule] = ' ';
        }
    }
    cells.into_iter().collect()
}

/// Draws one labelled section into `out`.
fn section(out: &mut String, label: &str, drawn: &[String]) {
    out.push_str(&format!("== {label} ==\n\n"));
    for line in drawn {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
}

/// Writes `docs/shots/m5-ui-confirm-over-graph-*.txt` and
/// `docs/shots/m5-ui-credential-over-graph-*.txt`: the two boxes, at 80x24
/// and 160x48, each standing on a 200-commit graph. Run by hand to
/// regenerate:
/// `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it two_boxes_over_content`.
#[test]
fn capture_two_boxes_over_real_content_for_docs_shots() {
    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let (push_box, credential_box) = boxes_at(label, columns, rows);
        let mut out = confirm_header(label, columns, rows, &what_survives(&push_box));
        section(&mut out, "`P`: the push confirm, on the graph", &push_box);
        check_shot(&format!("m5-ui-confirm-over-graph-{label}.txt"), &out);

        let mut out = credential_header(label, columns, rows);
        section(
            &mut out,
            "git's password question, over the push it belongs to",
            &credential_box,
        );
        check_shot(&format!("m5-ui-credential-over-graph-{label}.txt"), &out);
    }
}

/// The two frames at one size, each checked against what the capture is for
/// before it is written: real history behind the box, ADR-0040's outcome
/// beside it, and no plaintext anywhere.
fn boxes_at(label: &str, columns: u16, rows: u16) -> (Vec<String>, Vec<String>) {
    let remote = one_commit_to_push();
    let work = remote.work().to_owned();
    let (app, state) = App::open(&work);
    // Sized before the keys, not after: the layout, the graph's row budget
    // and the hint bar's room all come off `state.size`, so a state sized
    // for one screen photographed at another is a picture of neither.
    let state = app.resize(&state, columns, rows);
    let state = ready(&app, state);

    // 1. `P`: the push confirm, over the graph it is about.
    let asked = app.key(&state, KeyCode::Char('P'));
    assert!(
        asked.confirm.is_some(),
        "P opened no box; status {:?}",
        asked.status
    );
    let push_box = shot(&app, &asked, columns, rows, &work);

    // 2. The same key answered: the push runs, and git's question arrives
    // over it. Opened through `credential_needed`, the same entry point
    // `Progress::CredentialNeeded` uses, on the real job the confirm
    // started — the prompt is the running push's, not a hand-built state.
    let pushing = app.key(&app.key(&asked, KeyCode::Right), KeyCode::Enter);
    let job = pushing
        .operating
        .expect("confirming the push starts the write");
    let mut asking = pushing.clone();
    mergewright::update::credential::credential_needed(&mut asking, job, ASK, false);
    assert!(asking.credential.is_some(), "the prompt did not open");
    assert!(
        mergewright::update::credential::credential_paste(&mut asking, TYPED),
        "the prompt is open and takes a paste"
    );
    let credential_box = shot(&app, &asking, columns, rows, &work);

    // 3. The push lands, so the capture is of a repository the app really
    // wrote to and not of a box over a pending intention.
    let state = app.settle_write(pushing, "the push");
    let state = journal_has_the_push(&app, state);

    check(label, &push_box, &credential_box);

    drop(state);
    drop(asking);
    app.close();
    (push_box, credential_box)
}

/// What the capture is for, asserted before it is written: real history
/// behind each box, ADR-0040's opposite outcomes beside them, no plaintext
/// anywhere, and the masked field at its fixed width (ADR-0038 §2).
fn check(label: &str, push_box: &[String], credential_box: &[String]) {
    for (what, drawn) in [
        ("the push confirm", push_box),
        ("the credential prompt", credential_box),
    ] {
        let body = drawn.join("\n");
        assert!(
            body.contains("Commit 19"),
            "{what} at {label} has no history behind it:\n{body}"
        );
        assert!(
            !body.contains(TYPED),
            "the typed password reached a capture: {what} at {label}"
        );
    }
    // ADR-0040, the confirm box: it does not clear, because the rows beside
    // it are the graph the user is about to act on.
    assert!(
        rows_with_content_beside_the_box(push_box) >= 3,
        "the push confirm at {label} has nothing surviving beside it, and \
         ADR-0040 says it keeps the graph:\n{}",
        push_box.join("\n")
    );
    // ADR-0040, the credential prompt: it clears its band, full width, at
    // every width — and the band only, so the footer below it stands.
    assert_eq!(
        rows_with_content_beside_the_box(credential_box),
        0,
        "the credential prompt at {label} left content in the band it \
         clears (ADR-0040):\n{}",
        credential_box.join("\n")
    );
    assert!(
        credential_box
            .last()
            .is_some_and(|footer| !footer.trim().is_empty()),
        "the credential prompt at {label} cleared the footer too; ADR-0040 \
         clears the band, not the screen"
    );
    assert_eq!(
        credential_box
            .join("\n")
            .matches(['\u{2022}', '\u{25e6}'])
            .count(),
        mergewright_ui::text_input::SECRET_GLYPHS,
        "the masked field is a fixed {} glyphs (ADR-0038 §2) at {label}",
        mergewright_ui::text_input::SECRET_GLYPHS
    );
}

/// The caption for the confirm captures. What the user is judging, in his
/// own words, and what the capture cannot show.
fn confirm_header(label: &str, columns: u16, rows: u16, survives: &(String, String)) -> String {
    let (left, right) = survives;
    format!(
        "# m5-ui-confirm-over-graph-{label}.txt: {columns}x{rows}, drawn by\n\
         # `view::view` from a real AppState \u{2014} the Linear corpus cloned into a\n\
         # `file://` bare, the clone left one commit ahead of it, the walk let\n\
         # settle, then `P` pressed\n\
         # (crates/mergewright/tests/it/two_boxes_over_content.rs,\n\
         # capture_two_boxes_over_real_content_for_docs_shots, asserted).\n\
         # Not a pty capture: the terminal is ratatui's TestBackend, so this is\n\
         # a character grid with no colour attributes in it \u{2014} CLAUDE.md rule 5,\n\
         # colour ships unverified. The scratch repository's temporary path is\n\
         # set to `{HOME}` and the clock to the corpus's own `now` **before**\n\
         # the frame is drawn, so the header is laid out against the width a\n\
         # real checkout takes and the ages never move.\n\
         #\n\
         # **What right looks like here** (ADR-0040): the box does **not**\n\
         # clear its band \u{2014} the rows beside it are the graph you are about to\n\
         # act on, and seeing them is M5's promise. Instead every column that\n\
         # survives beside it survives **whole**: its left edge sits on a\n\
         # boundary between two row columns and its right edge one cell short\n\
         # of one, on that column's **gutter**, so there is a blank column\n\
         # between the border and the SHA rather than `\u{256e}60af0eb` reading as one\n\
         # token (ADR-0040's 2026-09-21 amendment). No clipped subject, no\n\
         # four-character tail of a SHA. The box's content is never shrunk to\n\
         # reach a position; if none fitted, it would clear instead \u{2014} though\n\
         # the gutter costs the box a cell, and at 80 columns that is the cell\n\
         # its longest line used as right-hand padding. Judge that by eye.\n\
         #\n\
         # The snap is computed where the screen is laid out\n\
         # (`mergewright::view::frame`, from `GraphScreen::column_boundaries`), not\n\
         # inside the widget \u{2014} ADR-0040 clause 3, so the box stays a box and\n\
         # knows nothing about the graph.\n\
         #\n\
         # What is behind it: 200 commits of ordinary history, the tip one\n\
         # commit ahead of `origin/main`, with the graph filling the rows\n\
         # above, below and beside the frame.\n\
         #\n\
         # What survives beside it, read back out of the frame below rather\n\
         # than written here by hand \u{2014} on the widest such row, `{left}` is\n\
         # left of the border and `{right}` is right of it.\n\
         #\n\
         # The 3-choice box `u` opens once the work is on a remote (ADR-0038\n\
         # \u{a7}4) stands on this same corpus and is in\n\
         # docs/shots/m5-ui-undo-after-push-80x24.txt. It is not reproduced\n\
         # here: reaching it needs a commit Mergewright makes **during** the run,\n\
         # and git stamps that commit from the wall clock, so its oid \u{2014} which\n\
         # the box and the header both draw \u{2014} would move every run and this\n\
         # file could not be diffed.\n\
         \n"
    )
}

/// The caption for the credential captures.
fn credential_header(label: &str, columns: u16, rows: u16) -> String {
    format!(
        "# m5-ui-credential-over-graph-{label}.txt: {columns}x{rows}, drawn by\n\
         # `view::view` from a real AppState \u{2014} the same repository and the\n\
         # same running push as m5-ui-confirm-over-graph-{label}.txt, with\n\
         # git's password question opened on the job the confirm started\n\
         # (`update::credential::credential_needed`, the entry point\n\
         # `Progress::CredentialNeeded` uses) and a seven-character password\n\
         # typed into it\n\
         # (crates/mergewright/tests/it/two_boxes_over_content.rs,\n\
         # capture_two_boxes_over_real_content_for_docs_shots, asserted).\n\
         # The host in the question is pinned to `127.0.0.1:8443`: a real HTTP\n\
         # fixture is served on a port the OS picks, and a capture whose title\n\
         # changed every run could not be diffed. Not a pty capture \u{2014} no\n\
         # colour attributes, CLAUDE.md rule 5.\n\
         #\n\
         # **What right looks like here** (ADR-0040): the opposite outcome,\n\
         # from the same rule. The prompt **clears the band it stands on**,\n\
         # full width, at every width. Two reasons, either sufficient: at 80\n\
         # its right border landed inside the short-SHA column and left four-\n\
         # character tails standing against it, reading as identifiers that\n\
         # are not; and nothing behind a modal password question serves the\n\
         # task \u{2014} the prompt's own border says which push is waiting\n\
         # (`esc cancel the push`); the bottom row is its keys (m6-conflicts/69).\n\
         #\n\
         # It clears **the band**, not the screen: the graph above it and the\n\
         # footer below it stay. So there is nothing beside the prompt in the\n\
         # rows it occupies, and that is the picture, not an omission \u{2014} the\n\
         # writer asserts it before writing this file, and a change of heart\n\
         # fails there rather than going stale here.\n\
         #\n\
         # The border's rule no longer runs into the title or the key hints:\n\
         # `Credentials \u{2014} 127.0.0.1:8443\u{2500}\u{2500}\u{2500}` read as part of the host. The\n\
         # prompt's private copy of `frame_box::border_row` is gone and the\n\
         # four bordered surfaces share one implementation (ADR-0040).\n\
         #\n\
         # ADR-0038 \u{a7}2 is unchanged and holds here: the field draws a fixed\n\
         # 8 mask glyphs whatever was typed, never the typed length, and the\n\
         # plaintext is in no row of this file (the writer asserts both).\n\
         \n"
    )
}
