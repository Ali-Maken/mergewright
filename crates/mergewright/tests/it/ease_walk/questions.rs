//! Q1–Q9: one assertion per answered question (`docs/design-notes/
//! ease-pass.md` "Questions for the user", all answered (a)), on the
//! mock-shaped repository. Words and shapes only; colour is the buffer's
//! attribute at most (rule 5).

use std::path::Path;

use mergewright::update::detail::DetailSpec;
use mergewright::update::preview::Showing;
use mergewright_core::event::{KeyCode, KeyPress};

use super::commit::{committed, opened as commit_opened, settled};
use super::fixture::{self, mock, with_edits};
use super::flow::{opened, previewed};
use super::merge_finish::first_key;
use super::screen::{
    bar, buffer, coloured_cells, cursor_on, drawn, line_with, open, shown, top, typed,
};
use crate::app_harness::git;

/// Q1 (a), with the user's answer of 2026-10-20: `C` on a teammate's row
/// opens the branch list on his branch; `⏎` switches to it. 2 keys.
#[test]
fn q1_c_opens_the_list_on_his_branch_and_enter_switches() {
    let (scratch, app, state) = opened();
    let state = cursor_on(&app, state, fixture::INTEGRATION);
    let state = app.pump(app.key(&state, KeyCode::Char('C')), "the list", |state| {
        state.picker.as_ref().is_some_and(|picker| {
            matches!(picker.list, mergewright::update::detail::Loading::Ready(_))
        })
    });
    let on = state
        .picker
        .as_ref()
        .and_then(|picker| picker.selected_row())
        .map(|row| String::from_utf8_lossy(row.name.as_bytes()).into_owned());
    assert_eq!(on.as_deref(), Some("refs/heads/integration"));
    let state = app.key(&state, KeyCode::Enter);
    let repo = scratch.path().to_owned();
    let _switched = app.pump(state, "the switch", move |state| {
        state.picker.is_none() && head_branch(&repo) == "refs/heads/integration"
    });
    assert_eq!(head_branch(scratch.path()), "refs/heads/integration");
    app.close();
}

fn head_branch(repo: &Path) -> String {
    git(repo, &["symbolic-ref", "HEAD"]).trim().to_owned()
}

/// Q2 (a): the teammate's bar says `x compare`; `x` on one row and `x` on
/// another compares the two.
#[test]
fn q2_x_compare_on_the_bar_and_x_x_compares() {
    let (_scratch, app, state) = opened();
    let state = cursor_on(&app, state, fixture::INTEGRATION);
    assert!(bar(&drawn(&state)).contains(" · x compare · "));
    let base = selected_oid(&state);
    let state = app.key(&state, KeyCode::Char('x'));
    let state = cursor_on(&app, state, fixture::PAY_307);
    let head = selected_oid(&state);
    let state = app.key(&state, KeyCode::Char('x'));
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare { base, head }),
        "the two rows compared"
    );
    app.close();
}

fn selected_oid(state: &mergewright::update::AppState) -> mergewright_core::oid::Oid {
    let Some(row) = state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
    else {
        panic!("a cursor row");
    };
    row.commit.id
}

/// Q3 (a): `#` takes colour off the whole frame, glyphs unchanged; it stays
/// off across the commit screen opening and closing; `#` again restores it.
#[test]
fn q3_hash_colour_off_until_pressed_again() {
    let scratch = mock();
    with_edits(scratch.path());
    let (app, state) = open(scratch.path(), 120);
    let before = drawn(&state);
    assert!(coloured_cells(&buffer(&state)) > 0);
    let off = app.key(&state, KeyCode::Char('#'));
    assert_eq!(coloured_cells(&buffer(&off)), 0);
    assert_eq!(drawn(&off), before, "glyphs unchanged");
    let commit = commit_opened(&app, &off);
    assert_eq!(
        coloured_cells(&buffer(&commit)),
        0,
        "off on the commit screen"
    );
    let closed = app.key(&commit, KeyCode::Escape);
    assert!(closed.commit_editor.is_none());
    assert_eq!(coloured_cells(&buffer(&closed)), 0, "still off after it");
    let on = app.key(&closed, KeyCode::Char('#'));
    assert!(
        coloured_cells(&buffer(&on)) > 0,
        "`#` again restores colour"
    );
    app.close();
}

/// Q4 (a): the lag measurement is recorded with `SUMMARY` lines for the
/// base, roles, and roles-plus-tall-row builds. Not judged here: the
/// numbers go to the user.
#[test]
fn q4_pixel_lag_summary_recorded() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/perf/ease-pixel.txt");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    for bin in ["bin=base ", "bin=roles ", "bin=tall "] {
        assert!(
            text.lines()
                .any(|line| line.starts_with("SUMMARY ") && line.contains(bin)),
            "no SUMMARY line for {bin}"
        );
    }
}

/// Q5 (a): the would-be merge's ghost rows stay drawn above the box.
#[test]
fn q5_ghost_rows_above_the_preview_box() {
    let (_scratch, app, state) = opened();
    let state = previewed(&app, state, fixture::INTEGRATION);
    let frame = drawn(&state);
    let ghost = super::screen::row_of(&frame, "after merge");
    let boxed = super::screen::row_of(&frame, "╭ Merge integration into wallet-release");
    assert!(ghost < boxed, "{}", shown(&frame));
    line_with(&frame, "Merge branch 'integration' in");
    app.close();
}

/// Q6 (a): no buttons on the merge preview; `⏎` lists the incoming commits.
#[test]
fn q6_no_buttons_and_enter_lists() {
    let (_scratch, app, state) = opened();
    let state = previewed(&app, state, fixture::INTEGRATION);
    let text = shown(&drawn(&state));
    assert!(
        !text.contains("[ Cancel ]") && !text.contains("[ Confirm ]"),
        "{text}"
    );
    let list = app.key(&state, KeyCode::Enter);
    assert!(matches!(
        list.preview.as_ref().map(|preview| &preview.showing),
        Some(Showing::Incoming(_))
    ));
    app.close();
}

/// Q7 (a): `esc` on the clash screen → the graph, banner focused, its first
/// key `⏎ back to the clashes`; `⏎` → the first open clash. On the merge;
/// the rebase's leg is `journeys::pause_and_resume_on_the_rebase`.
#[test]
fn q7_esc_pauses_on_the_banner_and_enter_resumes() {
    let (_scratch, app, state) = super::flow::stopped("integration");
    let paused = app.key(&state, KeyCode::Escape);
    assert!(paused.banner_focus && paused.resolver.is_none());
    assert_eq!(first_key(bar(&drawn(&paused))), "⏎ back to the clashes");
    let back = app.pump(app.key(&paused, KeyCode::Enter), "the clash", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some() && open.set_job.is_none())
    });
    crate::clash_stops::on_the_first_clash(&back);
    app.close();
}

/// Q8 (a): no option row on the commit screen; `⌃S` signs off; `⌃P` lists
/// amend, skip hooks and allow empty; skip hooks shows `· skipping hooks` in
/// the header.
#[test]
fn q8_options_off_screen_ctrl_s_and_ctrl_p() {
    let scratch = mock();
    with_edits(scratch.path());
    let (app, state) = open(scratch.path(), 120);
    let state = commit_opened(&app, &state);
    let text = shown(&drawn(&state));
    for word in ["[ ]", "[x]", "Sign off", "Skip hooks", "Allow empty"] {
        assert!(!text.contains(word), "no option row: {word:?}\n{text}");
    }
    let signed = app.press(&state, KeyPress::control(KeyCode::Char('s')));
    assert!(
        signed
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.sign_off)
    );
    let palette = app.press(&signed, KeyPress::control(KeyCode::Char('p')));
    let listed = shown(&drawn(&palette));
    for word in ["Amend", "Skip the commit hooks", "Allow an empty commit"] {
        assert!(listed.contains(word), "{word:?} listed\n{listed}");
    }
    let chosen = app.key(
        &typed(&app, &palette, "Skip the commit hooks"),
        KeyCode::Enter,
    );
    let chosen = settled(&app, chosen);
    assert!(
        chosen
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.no_verify)
    );
    let header = drawn(&chosen);
    assert!(
        top(&header).contains("· skipping hooks"),
        "{:?}",
        top(&header)
    );
    app.close();
}

/// Q9 (a): `u` after a commit → the graph with the edits uncommitted; the
/// next `c` opens with the old message.
#[test]
fn q9_u_after_commit_keeps_the_message() {
    let scratch = mock();
    with_edits(scratch.path());
    let repo = scratch.path();
    let (app, state) = open(repo, 120);
    let before = git(repo, &["rev-parse", "HEAD"]);
    let state = committed(&app, &state, "fix toc");
    let undone = app.settle_write(app.key(&state, KeyCode::Char('u')), "the undo");
    assert_eq!(git(repo, &["rev-parse", "HEAD"]), before, "HEAD is back");
    assert!(undone.commit_editor.is_none(), "the graph");
    let status = git(repo, &["status", "--porcelain"]);
    assert!(status.contains(" M docs/toc.md"), "uncommitted: {status}");
    let undone = app.pump(undone, "the working tree re-read", |state| {
        state
            .working_tree
            .as_deref()
            .is_some_and(|tree| tree.entries.len() == 4)
    });
    let reopened = commit_opened(&app, &undone);
    let subject = reopened
        .commit_editor
        .as_ref()
        .map(|editor| editor.subject.text().to_owned());
    assert_eq!(subject.as_deref(), Some("fix toc"));
    app.close();
}
