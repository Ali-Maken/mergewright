//! A submodule row says what git says (m7-power-tools/13): pointer moved,
//! changed inside, untracked inside, not set up — and the diff pane on it
//! lists the commits the move pulled in. Real `git`, real submodules from a
//! local path (no network), the real job pool.
//!
//! The oracle is git itself: every id and subject an assertion expects is
//! read from the submodule's own repository with `git rev-parse`/`git log`,
//! never from the code under test.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};
use std::process::Command as Shell;

use mergewright::update::AppState;
use mergewright::update::detail::Loading;
use mergewright_core::event::KeyCode;
use mergewright_ui::detail::{
    FileRowKind, FileStatusView, SubmoduleCommitView, SubmoduleLogView, SubmoduleView,
    WorkingTreeSection,
};

use crate::app_harness::{App, changes_pane, cursor_to, git};

/// Where the submodule sits in the superproject.
const SUB: &str = "libs/parser";

/// The superproject (a scratch copy of the trivial shape) with `SUB` at
/// the source's first commit, and the source repository itself.
struct Super {
    scratch: mergewright_fixtures::Scratch,
    _source: tempfile::TempDir,
    /// The source's three commits, oldest first, full ids.
    ids: [String; 3],
}

impl Super {
    fn root(&self) -> &Path {
        self.scratch.path()
    }

    fn sub(&self) -> PathBuf {
        self.root().join(SUB)
    }
}

/// `git` with a fixed identity and a fixed date, so every id this file
/// makes — and so every capture — is the same on every run.
fn dated(repo: &Path, args: &[&str]) {
    let at = format!("{} +0000", mergewright_fixtures::CORPUS_NOW - 86_400);
    let out = Shell::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Mergewright Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .env("GIT_AUTHOR_DATE", &at)
        .env("GIT_COMMITTER_DATE", &at)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn short(id: &str) -> String {
    id[..7].to_owned()
}

/// The fixture: a source repository with commits `parser 1..3`, added to
/// the superproject at [`SUB`], its pointer committed at `parser 1`.
fn with_a_submodule() -> Super {
    let source = tempfile::tempdir().expect("a temp dir");
    let src = source.path();
    git(src, &["init", "-q", "-b", "main"]);
    for n in 1..=3 {
        std::fs::write(src.join("f.txt"), format!("v{n}\n")).expect("writable");
        git(src, &["add", "f.txt"]);
        dated(src, &["commit", "-q", "-m", &format!("parser {n}")]);
    }
    let ids = [2, 1, 0].map(|back| {
        git(src, &["rev-parse", &format!("HEAD~{back}")])
            .trim()
            .to_owned()
    });

    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    git(
        repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            &src.to_string_lossy(),
            SUB,
        ],
    );
    // The temp path would make `.gitmodules` — and every id after it —
    // differ per run; the URL is never read again.
    git(
        repo,
        &[
            "config",
            "-f",
            ".gitmodules",
            &format!("submodule.{SUB}.url"),
            "../parser.git",
        ],
    );
    git(repo, &["add", ".gitmodules"]);
    git(&repo.join(SUB), &["checkout", "-q", &ids[0]]);
    git(repo, &["add", SUB]);
    dated(repo, &["commit", "-q", "-m", "add parser"]);
    // No commit-graph offer: it would take a key (keymap §2.1).
    git(repo, &["commit-graph", "write", "--reachable"]);
    Super {
        scratch,
        _source: source,
        ids,
    }
}

/// The changes pane's rows as `(section, name, submodule facts)` for each
/// submodule row.
fn submodule_rows(state: &AppState) -> Vec<(WorkingTreeSection, String, SubmoduleView)> {
    let Loading::Ready(files) = &state.detail.files else {
        panic!("the file list has not arrived: {:?}", state.detail.files);
    };
    let mut section = None;
    let mut found = Vec::new();
    for row in &files.rows {
        match &row.kind {
            FileRowKind::Section { section: s, .. } => section = Some(*s),
            FileRowKind::File {
                status: FileStatusView::Submodule(view),
                ..
            } => found.push((
                section.expect("a file row is under a section"),
                row.name.as_str().to_owned(),
                (**view).clone(),
            )),
            _ => {}
        }
    }
    found
}

/// The one submodule row, its words as the pane draws them.
fn the_row(state: &AppState) -> (WorkingTreeSection, String) {
    let rows = submodule_rows(state);
    assert_eq!(rows.len(), 1, "one submodule row: {rows:?}");
    let (section, name, view) = rows.into_iter().next().expect("one row");
    assert_eq!(name, SUB, "the row names the whole path");
    (section, view.row_words(true))
}

fn open(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = changes_pane(&app, state);
    (app, state)
}

/// A frame of the app as the user sees it, clock fixed.
fn frame(state: &AppState) -> String {
    let rows = crate::captures::frame_at(
        state,
        120,
        40,
        Some(mergewright::session::Clock::Fixed(mergewright_fixtures::CORPUS_NOW)),
    );
    format!("{}\n", rows.join("\n"))
}

/// Sublime Merge #1084: a change that is **only** a submodule's pointer is
/// still a change, and says so — with both ids.
#[test]
fn a_submodule_only_change_shows_pointer_moved() {
    let fixture = with_a_submodule();
    git(&fixture.sub(), &["checkout", "-q", &fixture.ids[2]]);

    let (app, state) = open(fixture.root());

    assert_eq!(
        the_row(&state),
        (
            WorkingTreeSection::Unstaged,
            format!(
                "submodule \u{b7} pointer moved {} \u{2192} {}",
                short(&fixture.ids[0]),
                short(&fixture.ids[2])
            )
        )
    );
    app.close();
}

/// lazygit #3641: moved **and** dirty is both facts, not one hiding the
/// other. Also writes `docs/shots/m7/submodule-row-120x40.txt`. Right: the
/// UNSTAGED row `▣ libs/parser  submodule ·…` (cut at the list's 30
/// columns), beside it the diff pane's `+` rows.
#[test]
fn a_moved_and_dirty_submodule_shows_both() {
    let fixture = with_a_submodule();
    git(&fixture.sub(), &["checkout", "-q", &fixture.ids[2]]);
    std::fs::write(fixture.sub().join("f.txt"), "edited\n").expect("writable");

    let (app, state) = open(fixture.root());

    assert_eq!(
        the_row(&state),
        (
            WorkingTreeSection::Unstaged,
            format!(
                "submodule \u{b7} pointer moved {} \u{2192} {} \u{b7} changed inside",
                short(&fixture.ids[0]),
                short(&fixture.ids[2])
            )
        )
    );
    let state = crate::captures::settled_frame(&app, state);
    let shot = frame(&state);
    // At 120 columns the pane sits below the graph and its file list is 30
    // columns wide: the row keeps every fact as a mark, right-aligned
    // (m7-power-tools/30i); the diff pane's line 2 has the words.
    assert_row_marks(&shot, "\u{2192} \u{b1}");
    // The row carries only marks (asserted just above), so the words are
    // the diff pane's line 2 — drawn beside the row, on the same screen line.
    assert!(
        shot.contains("changed inside"),
        "line 2 of the diff pane: {shot}"
    );
    crate::captures::check_shot("m7/submodule-row-120x40.txt", &shot);
    // At 187 the pane sits right with room for the full-words rung.
    let wide = crate::captures::frame_at(
        &state,
        187,
        40,
        Some(mergewright::session::Clock::Fixed(mergewright_fixtures::CORPUS_NOW)),
    );
    let wide = format!("{}\n", wide.join("\n"));
    assert!(
        wide.contains(&format!(
            "\u{25a3} {SUB}  submodule \u{b7} pointer moved {} \u{2192} {} \u{b7} changed inside",
            short(&fixture.ids[0]),
            short(&fixture.ids[2])
        )),
        "{wide}"
    );
    crate::captures::check_shot("m7/submodule-row-187x40.txt", &wide);
    app.close();
}

/// Untracked files inside are their own fact. git's worktree diff says
/// nothing about them (measured: `diff-files` is silent on `S..U`), so the
/// row comes from the status alone.
#[test]
fn untracked_inside_is_its_own_fact() {
    let fixture = with_a_submodule();
    std::fs::write(fixture.sub().join("scratch.txt"), "x\n").expect("writable");

    let (app, state) = open(fixture.root());

    assert_eq!(
        the_row(&state),
        (
            WorkingTreeSection::Unstaged,
            "submodule \u{b7} untracked inside".to_owned()
        )
    );
    let state = crate::captures::settled_frame(&app, state);
    assert_row_marks(&frame(&state), "?");
    app.close();
}

/// The submodule row in a 120x40 frame ends in exactly `marks`, right after
/// the path's gap (m7-power-tools/30i).
fn assert_row_marks(shot: &str, marks: &str) {
    let row = shot
        .lines()
        .find(|line| line.contains(&format!("\u{25a3} {SUB}")))
        .unwrap_or_else(|| panic!("a submodule row: {shot}"));
    let after = row
        .split(&format!("\u{25a3} {SUB}"))
        .nth(1)
        .expect("text after the path");
    let tail: String = after
        .trim_start()
        .chars()
        .take(marks.chars().count() + 1)
        .collect();
    assert_eq!(tail.trim_end(), marks, "{row:?}");
    assert!(!row.contains("submodule"), "{row:?}");
}

/// A submodule git says nothing about draws no row, even with the pane
/// open on another change.
#[test]
fn a_clean_submodule_shows_no_row() {
    let fixture = with_a_submodule();
    std::fs::write(fixture.root().join("note.txt"), "x\n").expect("writable");

    let (app, state) = open(fixture.root());

    assert_eq!(submodule_rows(&state), Vec::new());
    assert!(
        crate::app_harness::rows(&state).contains(&"note.txt".to_owned()),
        "the pane is open on the other change"
    );
    app.close();
}

/// A clone that never ran `submodule update --init`: the row says not set
/// up, and its diff pane says the commits aren't here.
#[test]
fn a_submodule_not_set_up_says_so() {
    let fixture = with_a_submodule();
    let clones = tempfile::tempdir().expect("a temp dir");
    let clone = clones.path().join("clone");
    git(
        clones.path(),
        &["clone", "-q", &fixture.root().to_string_lossy(), "clone"],
    );
    assert!(!clone.join(SUB).join(".git").exists(), "not set up");
    git(&clone, &["commit-graph", "write", "--reachable"]);
    std::fs::write(clone.join("note.txt"), "x\n").expect("writable");

    let (app, state) = open(&clone);

    assert_eq!(
        the_row(&state),
        (
            WorkingTreeSection::Unstaged,
            "submodule \u{b7} not set up".to_owned()
        )
    );
    let state = on_the_submodule(&app, state);
    assert_eq!(diff_submodule(&state).log, Some(SubmoduleLogView::NotHere));
    app.close();
}

/// The files cursor on the submodule row, its diff arrived.
fn on_the_submodule(app: &App, state: AppState) -> AppState {
    let state = cursor_to(app, state, SUB);
    app.pump(
        state,
        "the submodule's diff",
        |state| matches!(&state.detail.diff, Loading::Ready(diff) if diff.path.as_str() == SUB),
    )
}

/// The diff pane's submodule facts.
fn diff_submodule(state: &AppState) -> SubmoduleView {
    let Loading::Ready(diff) = &state.detail.diff else {
        panic!("no diff: {:?}", state.detail.diff);
    };
    match &diff.status {
        FileStatusView::Submodule(view) => (**view).clone(),
        other => panic!("not a submodule diff: {other:?}"),
    }
}

/// The commits git's own `log --left-right from...to` in the submodule
/// lists, as the diff pane should: `>` pulled in first, then `<` rewound.
fn gits_own_log(sub: &Path, from: &str, to: &str) -> Vec<SubmoduleCommitView> {
    let out = git(
        sub,
        &[
            "log",
            "--left-right",
            "--first-parent",
            "--format=%m %h %s",
            "--abbrev=7",
            &format!("{from}...{to}"),
        ],
    );
    let rows: Vec<(bool, SubmoduleCommitView)> = out
        .lines()
        .map(|line| {
            let mut parts = line.splitn(3, ' ');
            let side = parts.next().expect("a side");
            let id = parts.next().expect("an id");
            let subject = parts.next().expect("a subject");
            let pulled_in = side == ">";
            (
                pulled_in,
                SubmoduleCommitView {
                    pulled_in,
                    id: mergewright_term::sanitize::sanitize_line(id),
                    subject: mergewright_term::sanitize::sanitize_line(subject),
                },
            )
        })
        .collect();
    let (pulled, rewound): (Vec<_>, Vec<_>) = rows.into_iter().partition(|(pulled, _)| *pulled);
    pulled
        .into_iter()
        .chain(rewound)
        .map(|(_, row)| row)
        .collect()
}

/// The diff pane on a submodule moved sideways lists the commits the move
/// pulled in (`+`) and rewound (`−`), as git's own log in the submodule
/// lists them. Also writes `docs/shots/m7/submodule-diff-120x40.txt`.
/// Right: the header `libs/parser · submodule · <a> → <b>`, then
/// `+ <sha> parser 3`, `+ <sha> parser 2`, `− <sha> side 1`.
#[test]
fn the_diff_pane_lists_the_commits_a_move_pulled_in() {
    let fixture = with_a_submodule();
    // The pointer on a side commit off `parser 1`; the worktree on
    // `parser 3`: two commits pulled in, one rewound.
    std::fs::write(fixture.sub().join("side.txt"), "side\n").expect("writable");
    git(&fixture.sub(), &["add", "side.txt"]);
    dated(&fixture.sub(), &["commit", "-q", "-m", "side 1"]);
    let side = git(&fixture.sub(), &["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    git(fixture.root(), &["add", SUB]);
    dated(
        fixture.root(),
        &["commit", "-q", "-m", "pin the side commit"],
    );
    git(fixture.root(), &["commit-graph", "write", "--reachable"]);
    git(&fixture.sub(), &["checkout", "-q", &fixture.ids[2]]);
    let (app, state) = open(fixture.root());

    let state = on_the_submodule(&app, state);

    let oracle = gits_own_log(&fixture.sub(), &side, &fixture.ids[2]);
    assert_eq!(
        oracle
            .iter()
            .map(|row| (row.pulled_in, row.subject.as_str()))
            .collect::<Vec<_>>(),
        vec![(true, "parser 3"), (true, "parser 2"), (false, "side 1")],
        "the fixture moved as meant"
    );
    assert_eq!(
        diff_submodule(&state).log,
        Some(SubmoduleLogView::Commits(oracle))
    );
    let state = crate::captures::settled_frame(&app, state);
    let shot = frame(&state);
    assert!(
        shot.contains(&format!(
            "{SUB} \u{b7} submodule \u{b7} {} \u{2192} {}",
            short(&side),
            short(&fixture.ids[2])
        )),
        "{shot}"
    );
    assert!(
        shot.contains(&format!("+ {} parser 3", short(&fixture.ids[2]))),
        "{shot}"
    );
    assert!(
        shot.contains(&format!("\u{2212} {} side 1", short(&side))),
        "{shot}"
    );
    crate::captures::check_shot("m7/submodule-diff-120x40.txt", &shot);
    app.close();
}

/// `space` on a moved submodule's row stages the pointer — what `git add`
/// on the path does — and the row moves to STAGED with the same move.
#[test]
fn space_on_a_submodule_stages_the_pointer() {
    let fixture = with_a_submodule();
    git(&fixture.sub(), &["checkout", "-q", &fixture.ids[2]]);
    let (app, state) = open(fixture.root());
    let state = cursor_to(&app, state, SUB);

    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.settle_write(state, "staging the submodule");

    let staged = git(fixture.root(), &["ls-files", "--stage", "--", SUB]);
    assert!(
        staged.starts_with(&format!("160000 {} ", fixture.ids[2])),
        "the index holds the new pointer: {staged}"
    );
    let state = app.pump(state, "the pane re-read", |state| {
        matches!(&state.detail.files, Loading::Ready(_))
            && submodule_rows(state)
                .first()
                .is_some_and(|(section, ..)| *section == WorkingTreeSection::Staged)
    });
    assert_eq!(
        the_row(&state),
        (
            WorkingTreeSection::Staged,
            format!(
                "submodule \u{b7} pointer moved {} \u{2192} {}",
                short(&fixture.ids[0]),
                short(&fixture.ids[2])
            )
        )
    );
    app.close();
}
