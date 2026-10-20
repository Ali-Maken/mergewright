//! Q10–Q17: one assertion per answered question, continued from
//! [`super::questions`]. Words and shapes only (rule 5).

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;
use mergewright_git::state::InProgress;

use super::fixture::{self, mock};
use super::flow::{opened, pick, stopped};
use super::screen::{
    bar, cursor_on, drawn, key_at_rest, line_with, node_column, open, shown, sized, top,
};
use crate::app_harness::{App, git};

/// `ali/tidy` checked out, `r` on `integration`'s row, `y`: stopped on the
/// rebase's clash in `report.md`.
fn rebase_stopped() -> (tempfile::TempDir, App, AppState) {
    let scratch = mock();
    git(scratch.path(), &["switch", "--quiet", "ali/tidy"]);
    let (app, state) = open(scratch.path(), 120);
    let state = crate::clash_stops::stop_by(&app, state, b"integration", 'r', InProgress::Rebase);
    (scratch, app, state)
}

/// Q10 (a): an opened merge's commits draw on their own lane, right of the
/// merge's, curving into it.
#[test]
fn q10_opened_merge_rows_on_their_own_lane() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let state = cursor_on(&app, state, fixture::PR_MERGE);
    let state = key_at_rest(&app, &state, ' ');
    let frame = drawn(&state);
    let merge = node_column(&frame, fixture::PR_MERGE);
    for subject in fixture::PR_COMMITS {
        assert!(
            node_column(&frame, subject) > merge,
            "{subject} on its own lane\n{}",
            shown(&frame)
        );
    }
    let joins: String = line_with(&frame, fixture::PR_MERGE)
        .chars()
        .skip(merge + 1)
        .take(2)
        .collect();
    assert!(
        joins.contains(['╮', '┐', '╗']),
        "joins the merge: {joins:?}"
    );
    app.close();
}

/// Q11 (a): the top line keeps today's frame and ends `lanes drawn: N of M`
/// when lanes are dropped; the bar ends with today's `. actions · ? keys`.
#[test]
fn q11_todays_frame_with_the_mocks_words() {
    let (_scratch, app, state) = opened();
    let frame = drawn(&state);
    assert!(
        top(&frame).starts_with(" ◆ api  ~/code/api · wallet-release "),
        "{:?}",
        top(&frame)
    );
    assert!(bar(&frame).trim_end().ends_with(". actions · ? keys"));
    let short = key_at_rest(&app, &sized(&app, &state, 120, 10), 'G');
    let frame = drawn(&short);
    let line = top(&frame).trim_end();
    assert!(
        line.starts_with(" ◆ api  ~/code/api · wallet-release "),
        "{line:?}"
    );
    assert!(line.ends_with("lanes drawn: 5 of 7"), "{line:?}");
    app.close();
}

/// Q12 (a): a rebase's clash uses the marks and the advance; `⏎` stages the
/// file; `c` goes on.
#[test]
fn q12_rebase_clash_marks_enter_stages_c_goes_on() {
    let (scratch, app, state) = rebase_stopped();
    let repo = scratch.path();
    let state = pick(&app, &state, 't');
    let frame = drawn(&state);
    line_with(&frame, "▶  5 Report line 5");
    line_with(&frame, "✓ Every clash in report.md is solved.");
    assert!(!git(repo, &["ls-files", "-u"]).is_empty(), "not staged yet");
    let staged = app.settle_write(app.key(&state, KeyCode::Enter), "the stage");
    assert!(git(repo, &["ls-files", "-u"]).is_empty(), "`⏎` staged it");
    let staged = app.pump(staged, "the banner after the stage", |state| {
        state
            .resolver
            .as_ref()
            .is_none_or(|open| open.job.is_none())
    });
    let state = if staged.banner_focus {
        staged
    } else {
        app.key(&staged, KeyCode::Escape)
    };
    // `c` is refused while the files are still being re-read.
    let state = app.pump(state, "the files re-read", |state| {
        state.status_read.is_none()
    });
    let gone_on = app.key(&state, KeyCode::Char('c'));
    let gone_on = app.pump(gone_on, "the rebase going on", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    assert!(gone_on.in_progress.is_none(), "the rebase finished");
    assert_eq!(super::screen::head_subject(repo), fixture::TIDY);
    assert!(!repo.join(".git/rebase-merge").exists());
    app.close();
}

/// Q13 (a): a PR merge's folded row reads `▸ N commits from PR <n>`.
#[test]
fn q13_pr_merge_folds_as_commits_from_pr() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let frame = drawn(&state);
    assert!(
        line_with(&frame, fixture::PR_MERGE).contains("▸ 2 commits from PR 16021"),
        "{}",
        shown(&frame)
    );
    app.close();
}

/// Q14 (a): the top line names HEAD's branch once — no `on <branch>` — and
/// adds only `lanes drawn: …` / `… merged branches folded` to today's words.
#[test]
fn q14_top_line_names_the_branch_once() {
    let (_scratch, app, state) = opened();
    for frame in [
        drawn(&state),
        drawn(&key_at_rest(&app, &sized(&app, &state, 120, 10), 'G')),
    ] {
        let line = top(&frame);
        assert_eq!(line.matches("wallet-release").count(), 1, "{line:?}");
        assert!(!line.contains("on wallet-release"), "{line:?}");
        let (_, right) = line
            .split_once("~/code/api · wallet-release")
            .unwrap_or_else(|| panic!("today's frame: {line:?}"));
        for words in right.split("  ").map(str::trim).filter(|it| !it.is_empty()) {
            assert!(
                words.ends_with(" commits")
                    || words.starts_with("lanes drawn: ")
                    || words.ends_with(" merged branches folded"),
                "{words:?} on {line:?}"
            );
        }
    }
    app.close();
}

/// Q15 (a): sara's pane on `team` says built on `develop`, never `ali/x` or
/// `wallet-release` — the walk is `team::g12_sibling_never_built_on`.
#[test]
fn q15_sibling_is_never_built_on() {
    super::team::g12_sibling_never_built_on();
}

/// Q16 (a): `t` on a clash already kept as yours replaces the pick — `▶ took
/// theirs`, no `already taken` — and `u` brings `◀ kept yours` back.
#[test]
fn q16_retake_replaces_and_u_brings_the_earlier_back() {
    let (_scratch, app, state) = stopped("integration");
    let state = pick(&app, &state, 'o');
    line_with(&drawn(&state), "kept yours");
    let state = pick(&app, &state, 't');
    let frame = drawn(&state);
    line_with(&frame, "▶  5 Report line 5, as Bob put it  took theirs");
    assert!(
        !shown(&frame).contains("already taken"),
        "{}",
        shown(&frame)
    );
    let state = pick(&app, &state, 'u');
    line_with(
        &drawn(&state),
        "◀  5 Report line 5, for the release  kept yours",
    );
    app.close();
}

/// Q17 (a): the rebase that clashes lands on its first open clash, 0 keys
/// after the confirm; a stop with no clash (an `edit` stop) lands on the
/// graph, not the clash screen.
#[test]
fn q17_rebase_clash_lands_on_the_clash_and_an_edit_stop_on_the_graph() {
    let (_scratch, app, state) = rebase_stopped();
    crate::clash_stops::on_the_first_clash(&state);
    app.close();

    let dir = crate::rewrite_stop::stopped_on_a1();
    let (app, state) = crate::rewrite_stop::relaunched(dir.path());
    assert!(state.stopped.is_some(), "an edit stop");
    assert!(state.resolver.is_none(), "not the clash screen");
    assert!(state.history.is_some(), "the graph");
    app.close();
}
