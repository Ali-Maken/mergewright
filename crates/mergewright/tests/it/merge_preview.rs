//! The merge preview (ease-pass 10), through the real app over real git:
//! what comes in, what git combines, what he decides; `⏎` lists the
//! incoming commits; unrelated histories withhold `y`.
//!
//! Fixture: `wallet-release` checked out, `sara/fix-toc` forked off it
//! with three commits (Sara, Omar, Sara). In the clashing shape Omar's
//! commit and `wallet-release`'s own both change `report.md` in the same
//! two places. Regenerate the captures on purpose:
//! `MWR_UPDATE_SHOTS=1 cargo nextest run -E 'test(capture_the_merge_preview)'`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright::update::detail::Loading;
use mergewright::update::preview::Showing;
use mergewright_core::event::KeyCode;

use crate::app_harness::{App, git, pin_identity};
use crate::preview::{FORK_EPOCH, dated_commit, frame_at, opened};
use mergewright_core::on_disk::DIR;

/// The clock: an hour past the fixture's start, so every age is a number.
const NOW: i64 = FORK_EPOCH + 60 * 60;

const REPORT: &str = "# Report\n\n## Contents\n1. Summary\n2. Figures\n\nalpha\nbeta\ngamma\ndelta\nepsilon\n\n## Figures\nsee the appendix\n";
const REPORT_OURS: &str = "# Report\n\n## Contents\n1. The summary\n2. Figures\n\nalpha\nbeta\ngamma\ndelta\nepsilon\n\n## Figures\nsee appendix A\n";
const REPORT_THEIRS: &str = "# Report\n\n## Contents\n1. [Summary](#summary)\n2. Figures\n\nalpha\nbeta\ngamma\ndelta\nepsilon\n\n## Figures\nsee [the appendix](#appendix)\n";

/// A commit by `author`, at `minute` past [`FORK_EPOCH`].
fn authored(repo: &Path, name: &str, body: &str, subject: &str, author: &str, minute: i64) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
    git(repo, &["add", name]);
    let when = format!("@{} +0000", FORK_EPOCH + minute * 60);
    let by = format!(
        "{author} <{}@example.com>",
        author.to_lowercase().replace(' ', ".")
    );
    mergewright_fixtures::git::run_at(repo, &["commit", "-m", subject, "--author", &by], &when)
        .unwrap_or_else(|why| panic!("committing {subject:?}: {why}"));
}

/// `wallet-release` checked out; `sara/fix-toc` three commits ahead of
/// where they forked. `clash`: Omar's commit clashes with `wallet-release`
/// in `report.md`, two places; else it adds `notes.md`.
pub fn fix_toc(clash: bool) -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let path = scratch.path();
    pin_identity(path);
    git(path, &["switch", "-q", "-c", "wallet-release"]);
    dated_commit(path, "report.md", REPORT, "Add the report", 1);
    git(path, &["branch", "sara/fix-toc"]);
    dated_commit(path, "report.md", REPORT_OURS, "Number the report", 2);
    git(path, &["switch", "-q", "sara/fix-toc"]);
    authored(
        path,
        "toc.md",
        "toc\n",
        "Add a table of contents",
        "Sara Ahmed",
        3,
    );
    if clash {
        authored(
            path,
            "report.md",
            REPORT_THEIRS,
            "Link the contents",
            "Omar Khan",
            4,
        );
    } else {
        authored(path, "notes.md", "notes\n", "Add notes", "Omar Khan", 4);
    }
    authored(
        path,
        "links.md",
        "links\n",
        "Check every link",
        "Sara Ahmed",
        5,
    );
    git(path, &["switch", "-q", "wallet-release"]);
    scratch
}

/// [`fix_toc`], clashing, plus `stray`: one root commit sharing no history.
fn with_a_stray() -> mergewright_fixtures::Scratch {
    let scratch = fix_toc(true);
    let path = scratch.path();
    git(path, &["switch", "-q", "--orphan", "stray"]);
    dated_commit(path, "stray.txt", "stray\n", "Start over", 6);
    git(path, &["switch", "-qf", "wallet-release"]);
    scratch
}

fn open(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open_with_clock(repo, Clock::Fixed(NOW));
    let state = app.resize(&state, 120, 40);
    let state = opened(&app, state);
    (app, state)
}

/// Does the cursor's row carry branch `name`?
fn on_branch(state: &AppState, name: &str) -> bool {
    let (Some(refs), Some(row)) = (
        state.refs.as_deref(),
        state
            .history
            .as_ref()
            .and_then(mergewright::history::History::selected),
    ) else {
        return false;
    };
    refs.on(row.commit.id).iter().any(|label| {
        matches!(label, mergewright::refs::RefLabel::Branch { name: it, .. } if it == name.as_bytes())
    })
}

/// `j`, then `k`, until the cursor is on `name`'s row.
fn cursor_on(app: &App, state: AppState, name: &str) -> AppState {
    let mut state = state;
    for key in [KeyCode::Char('j'), KeyCode::Char('k')] {
        for _ in 0..20 {
            if on_branch(&state, name) {
                return state;
            }
            state = app.key(&state, key);
        }
    }
    assert!(on_branch(&state, name), "never found {name}'s row");
    state
}

/// The plan landed.
fn planned(app: &App, state: AppState) -> AppState {
    app.pump(state, "the merge plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !matches!(preview.plan, Loading::Idle | Loading::Pending(_)))
    })
}

/// `m` on `name`'s row, the plan landed.
fn merge_preview(app: &App, state: AppState, name: &str) -> AppState {
    let state = cursor_on(app, state, name);
    planned(app, app.key(&state, KeyCode::Char('m')))
}

fn screen(state: &AppState) -> Vec<String> {
    frame_at(state, 120, 40, Some(Clock::Fixed(NOW)))
}

/// `^\d+[mhdwy]$`: the graph's own age format.
fn graph_age(age: &str) -> bool {
    let (digits, unit) = age.split_at(age.len().saturating_sub(1));
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) && "mhdwy".contains(unit)
}

fn journal_len(repo: &Path) -> usize {
    let path = repo.join(format!(".git/{DIR}/undo-journal.jsonl"));
    if !path.exists() {
        return 0;
    }
    mergewright_ops::journal::Journal::open(&path)
        .expect("a readable journal")
        .entries()
        .len()
}

fn head(repo: &Path) -> String {
    git(repo, &["rev-parse", "HEAD"]).trim().to_owned()
}

#[test]
fn m_on_a_clashing_branch_says_what_comes_in_combines_and_clashes() {
    let scratch = fix_toc(true);
    let (app, state) = open(scratch.path());
    let state = merge_preview(&app, state, "sara/fix-toc");
    let rows = screen(&state);
    let text = rows.join("\n");
    for line in [
        "Merge sara/fix-toc into wallet-release",
        "│ Nothing has changed yet.",
        "│ Comes in      3 commits from sara/fix-toc (Sara, Omar)",
        // The fixture: toc.md, report.md, links.md change; report.md clashes.
        "│ Changes       3 files",
        "│ Git combines  2 files on its own",
        "│ You decide    1 file clashes: report.md (2 places)",
        "│ After y you go straight to the first clash.",
        "╰ y merge  esc cancel  ⏎ see the 3 commits ─",
    ] {
        assert!(text.contains(line), "{line:?} in\n{text}");
    }
    assert!(!text.contains("[ Cancel ]"), "no buttons:\n{text}");
    assert!(!text.contains("m merge  "), "no `m merge` alias:\n{text}");
    app.close();
}

#[test]
fn a_clean_merge_says_nothing_to_decide_and_m_y_lands_on_the_merge() {
    let scratch = fix_toc(false);
    let repo = scratch.path();
    let (app, state) = open(repo);
    let state = merge_preview(&app, state, "sara/fix-toc");
    let text = screen(&state).join("\n");
    assert!(
        text.contains("│ You decide    nothing: no file clashes"),
        "{text}"
    );
    assert!(text.contains("│ After y the merge is committed."), "{text}");
    // Two keys: `m`, `y` (journey `merge-clean`).
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the merge");
    assert!(state.land_on_head, "the Done owes a landing on HEAD");
    let state = app.pump(state, "the landing", |state| !state.land_on_head);
    assert!(state.preview.is_none(), "the box closed");
    git(repo, &["rev-parse", "--verify", "HEAD^2"]);
    let at = state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .map(|row| row.commit.id.to_string());
    assert_eq!(at, Some(head(repo)), "the cursor is on the merge commit");
    app.close();
}

#[test]
fn unrelated_histories_withhold_y_from_the_box_and_the_list() {
    let scratch = with_a_stray();
    let repo = scratch.path();
    let before = (head(repo), journal_len(repo));
    let (app, state) = open(repo);
    let state = merge_preview(&app, state, "stray");
    let text = screen(&state).join("\n");
    assert!(
        text.contains("│ Can't merge: stray and wallet-release share no history."),
        "{text}"
    );
    for gone in ["Changes ", "You decide", "After y"] {
        assert!(!text.contains(gone), "{gone:?} gone:\n{text}");
    }
    assert!(
        text.contains("╰ esc cancel  ⏎ see the 1 commit ─"),
        "{text}"
    );
    for key in ['y', 'm'] {
        let pressed = app.key(&state, KeyCode::Char(key));
        assert!(pressed.preview.is_some(), "`{key}` leaves the box open");
        assert!(pressed.operating.is_none(), "`{key}` runs nothing");
    }
    let list = app.key(&state, KeyCode::Enter);
    let text = screen(&list).join("\n");
    assert!(text.contains("╰ esc back to the preview ─"), "{text}");
    let pressed = app.key(&list, KeyCode::Char('y'));
    assert!(pressed.preview.is_some() && pressed.operating.is_none());
    assert_eq!((head(repo), journal_len(repo)), before, "nothing written");
    app.close();
}

#[test]
fn ghost_rows_stay_drawn_above_the_merge_box() {
    let scratch = fix_toc(true);
    let (app, state) = open(scratch.path());
    let state = merge_preview(&app, state, "sara/fix-toc");
    let preview = state.preview.as_ref().expect("a preview");
    assert_eq!(preview.ghost_index.len(), 1, "the merge commit's ghost");
    let rows = screen(&state);
    let label = rows.iter().position(|row| row.contains("after merge"));
    let boxed = rows
        .iter()
        .position(|row| row.contains("Merge sara/fix-toc into"));
    assert!(
        matches!((label, boxed), (Some(l), Some(b)) if l < b),
        "{}",
        rows.join("\n")
    );
    app.close();
}

#[test]
fn enter_lists_the_incoming_commits_and_esc_comes_back() {
    let scratch = fix_toc(true);
    let repo = scratch.path();
    let before = (head(repo), journal_len(repo));
    let (app, state) = open(repo);
    let state = merge_preview(&app, state, "sara/fix-toc");
    let list = app.key(&state, KeyCode::Enter);
    let Some(Showing::Incoming(view)) = list.preview.as_ref().map(|it| &it.showing) else {
        panic!("⏎ shows the list");
    };
    let subjects: Vec<&str> = view.rows.iter().map(|row| row.subject.as_str()).collect();
    assert_eq!(
        subjects,
        [
            "Check every link",
            "Link the contents",
            "Add a table of contents"
        ]
    );
    let who: Vec<&str> = view.rows.iter().map(|row| row.who.as_str()).collect();
    assert_eq!(who, ["Sara", "Omar", "Sara"]);
    for row in &view.rows {
        assert!(graph_age(&row.age), "{:?} is a graph age", row.age);
    }
    let text = screen(&list).join("\n");
    assert!(
        text.contains("The 3 commits that come in from sara/fix-toc"),
        "{text}"
    );
    assert!(
        text.contains("╰ esc back to the preview  y merge ─"),
        "{text}"
    );
    let moved = app.key(&list, KeyCode::Char('j'));
    let Some(Showing::Incoming(view)) = moved.preview.as_ref().map(|it| &it.showing) else {
        panic!("still the list");
    };
    assert_eq!(view.cursor, 1, "`j` moves down one");
    let back = app.key(&moved, KeyCode::Escape);
    assert_eq!(
        back.preview.as_ref().map(|it| &it.showing),
        Some(&Showing::Box)
    );
    let gone = app.key(&back, KeyCode::Escape);
    assert!(gone.preview.is_none(), "esc from the box closes it");
    assert_eq!((head(repo), journal_len(repo)), before, "nothing written");
    // `y` from the list merges: it stops on the clash.
    let merged = app.key(&list, KeyCode::Char('y'));
    assert!(
        merged.operating.is_some(),
        "`y` from the list runs the merge"
    );
    let merged = app.pump(merged, "the merge", |state| state.operating.is_none());
    assert!(merged.preview.is_none(), "the list closed with the box");
    assert_ne!(journal_len(repo), before.1, "the merge ran and journaled");
    app.close();
}

#[test]
fn a_rebase_preview_keeps_its_buttons_and_enter() {
    let scratch = crate::preview::forked();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let mut state = state;
    for _ in 0..20 {
        if crate::preview::on_main(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = planned(&app, app.key(&state, KeyCode::Char('r')));
    let view = mergewright::update::preview::preview_box(&state).expect("a box");
    let labels: Vec<String> = view.choices().iter().map(ToString::to_string).collect();
    assert_eq!(labels, ["Cancel", "Confirm"], "buttons drawn");
    assert!(view.merge.is_none());
    let closed = app.key(&state, KeyCode::Enter);
    assert!(
        closed.preview.is_none(),
        "⏎ presses the cursor's choice: Cancel"
    );
    let ran = app.settle_write(app.key(&state, KeyCode::Char('y')), "the rebase");
    assert!(ran.preview.is_none(), "`y` still confirms the rebase");
    app.close();
}

#[test]
fn an_untracked_file_in_the_way_closes_the_box_with_the_refusal() {
    let scratch = fix_toc(true);
    let repo = scratch.path();
    std::fs::write(repo.join("toc.md"), "mine\n").expect("writable");
    let before = (head(repo), journal_len(repo));
    let (app, state) = open(repo);
    let state = cursor_on(&app, state, "sara/fix-toc");
    let state = app.pump(
        app.key(&state, KeyCode::Char('m')),
        "the refusal",
        |state| state.preview.is_none(),
    );
    let text = screen(&state).join("\n");
    assert!(
        text.contains("toc.md"),
        "the status names the path:\n{text}"
    );
    assert_eq!((head(repo), journal_len(repo)), before, "nothing written");
    app.close();
}

pub(crate) mod fast_forward;
mod openers;
mod shots;
