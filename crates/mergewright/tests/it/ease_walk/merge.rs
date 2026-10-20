//! M1–M10: the merge mock's preview and clash screen on the mock-shaped
//! repository, one test per id. Words and shapes only (rule 5).

use mergewright::update::preview::Showing;
use mergewright_core::event::KeyCode;

use super::fixture;
use super::flow::{at_clash, opened, pick, previewed, stop_after_y, stopped};
use super::screen::{bar, cursor_on, drawn, line_after, line_with, shown, top};
use crate::app_harness::git;

/// M1: on a teammate's row the bar leads `m merge <b> into yours`.
#[test]
fn m1_bar_offers_merge_into_yours() {
    let (_scratch, app, state) = opened();
    let state = cursor_on(&app, state, fixture::INTEGRATION);
    let frame = drawn(&state);
    assert!(
        bar(&frame)
            .trim_start()
            .starts_with("m merge integration into yours · "),
        "{:?}",
        bar(&frame)
    );
    app.close();
}

/// M2: the preview's words, and its footer's three keys in the mock's
/// order (`y merge`, `esc cancel`, `⏎ see the N commits`).
#[test]
fn m2_preview_words_and_footer() {
    let (_scratch, app, state) = opened();
    let state = previewed(&app, state, fixture::INTEGRATION);
    let frame = drawn(&state);
    for words in [
        "╭ Merge integration into wallet-release ─",
        "│ Nothing has changed yet.",
        "│ Comes in      1 commit from integration (Bob)",
        "│ Changes       1 file",
        "│ You decide    1 file clashes: report.md (1 place)",
        "│ After y you go straight to the first clash.",
    ] {
        line_with(&frame, words);
    }
    let footer = line_with(&frame, "╰ y merge");
    let (y, esc, enter) = (
        footer.find("y merge"),
        footer.find("esc cancel"),
        footer.find("⏎ see the 1 commit"),
    );
    assert!(
        matches!((y, esc, enter), (Some(y), Some(e), Some(n)) if y < e && e < n),
        "{footer:?}"
    );
    app.close();
}

/// M3: `⏎` lists the incoming commits; `esc` comes back to the box; `y`
/// from the list merges.
#[test]
fn m3_enter_lists_incoming_esc_back_y_merges() {
    let (scratch, app, state) = opened();
    let state = previewed(&app, state, fixture::INTEGRATION);
    let list = app.key(&state, KeyCode::Enter);
    let frame = drawn(&list);
    line_with(&frame, "│ The 1 commit that comes in from integration");
    assert!(line_with(&frame, fixture::INTEGRATION).contains("Bob"));
    line_with(&frame, "╰ esc back to the preview  y merge");
    let back = app.key(&list, KeyCode::Escape);
    assert_eq!(
        back.preview.as_ref().map(|preview| &preview.showing),
        Some(&Showing::Box),
        "esc: back to the box"
    );
    let stopped = stop_after_y(&app, &list);
    assert!(stopped.preview.is_none(), "the list closed with the box");
    assert!(
        !super::flow::unmerged(scratch.path()).is_empty(),
        "the merge ran and stopped"
    );
    app.close();
}

/// M4: `esc` in the preview cancels: the box closes, nothing written.
#[test]
fn m4_esc_cancels() {
    let (scratch, app, state) = opened();
    let before = git(scratch.path(), &["rev-parse", "HEAD"]);
    let state = previewed(&app, state, fixture::INTEGRATION);
    let gone = app.key(&state, KeyCode::Escape);
    assert!(gone.preview.is_none(), "the box closed");
    assert!(gone.operating.is_none(), "nothing ran");
    assert_eq!(git(scratch.path(), &["rev-parse", "HEAD"]), before);
    assert!(!scratch.path().join(".git/MERGE_HEAD").exists());
    app.close();
}

/// M5: the stop lands on the first clash, 0 keys; the header reads
/// `merging integration into wallet-release`.
#[test]
fn m5_stop_lands_on_the_first_clash_with_its_header() {
    let (_scratch, app, state) = stopped("integration");
    crate::clash_stops::on_the_first_clash(&state);
    let frame = drawn(&state);
    assert_eq!(
        top(&frame).trim(),
        "merging integration into wallet-release"
    );
    app.close();
}

/// M6: the list column: `Clashing files`, `?`/`✓` with `N open`/`done`,
/// `✓ Review the merge  y` once all are done, `Combined by git (N)`.
#[test]
fn m6_list_column() {
    let (_scratch, app, state) = stopped("pay-307");
    let frame = drawn(&state);
    assert!(line_with(&frame, "Clashing files").starts_with("Clashing files"));
    assert!(line_with(&frame, "notes.md   2 open").starts_with("▏? notes.md   2 open"));
    assert!(line_with(&frame, "Combined by git (1)").starts_with("Combined by git (1)"));
    assert!(line_with(&frame, "checks.md").starts_with("✓ checks.md"));
    assert!(
        !frame.iter().any(|line| line.contains("Review the merge")),
        "no review row while clashes are open\n{}",
        shown(&frame)
    );
    app.close();

    let (_scratch, app, state) = stopped("integration");
    let state = pick(&app, &state, 'o');
    let frame = drawn(&state);
    assert!(line_with(&frame, "report.md  done").contains("✓ report.md  done"));
    assert!(line_with(&frame, "Review the merge").starts_with("✓ Review the merge  y"));
    app.close();
}

/// M7: the file's title, its numbered plain lines, the open clash's two
/// sides headed `? yours · <into>` | `theirs · <from>` and unnumbered, the
/// cursor's clash outlined.
#[test]
fn m7_file_title_numbered_lines_and_the_open_clash() {
    let (_scratch, app, state) = stopped("integration");
    let frame = drawn(&state);
    assert!(line_with(&frame, "│▸ report.md").contains("│▸ report.md · 1 conflict · 1 open"));
    assert!(line_with(&frame, "Release report").ends_with("│    1 Release report"));
    let heads = line_with(&frame, "? yours · wallet-release");
    assert!(heads.contains("│theirs · integration"), "{heads:?}");
    let ours = line_with(&frame, "Report line 5, for the release");
    assert!(
        !ours.contains(" 5 Report line 5"),
        "the clash's lines are unnumbered: {ours:?}"
    );
    let above = frame
        .iter()
        .position(|line| line.contains("? yours · wallet-release"))
        .and_then(|at| at.checked_sub(1))
        .map_or("", |at| frame[at].as_str());
    assert!(above.contains("│┌"), "outlined: {above:?}");
    assert!(
        line_after(&frame, "Report line 5, for the release").contains("│└"),
        "outlined below"
    );
    app.close();
}

/// M8: a taken clash shrinks to the taken lines, marked `◀ ▶ ◆` with
/// `kept yours` / `took theirs` / `kept both`.
#[test]
fn m8_taken_block_shrinks_and_says_which() {
    let (_scratch, app, state) = stopped("pay-307");
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 't');
    let frame = drawn(&state);
    let yours = line_with(&frame, "kept yours");
    assert!(
        yours.contains("◀  1 Release first note  kept yours"),
        "{yours:?}"
    );
    let theirs = line_with(&frame, "took theirs");
    assert!(
        theirs.contains("▶ 12 Omar's last note  took theirs"),
        "{theirs:?}"
    );
    assert!(
        !frame.iter().any(|line| line.contains("? yours")),
        "both blocks shrank\n{}",
        shown(&frame)
    );
    let state = pick(&app, &state, 'n');
    let state = pick(&app, &state, 'b');
    let frame = drawn(&state);
    assert!(line_with(&frame, "1 Release report").contains("◆  1 Release report"));
    assert!(line_with(&frame, "kept both").contains("2 Omar's report title  kept both"));
    app.close();
}

/// M9: a pick advances to the file's next open clash; a solved file stays
/// on screen with `✓ Every clash in <f> is solved.` and the next hint.
#[test]
fn m9_advance_within_the_file_and_the_solved_line() {
    let (_scratch, app, state) = stopped("pay-307");
    assert_eq!(at_clash(&state), ("notes.md".to_owned(), Some(0)));
    let state = pick(&app, &state, 'o');
    assert_eq!(at_clash(&state), ("notes.md".to_owned(), Some(1)));
    let state = pick(&app, &state, 'o');
    assert_eq!(at_clash(&state).0, "notes.md", "the solved file stays");
    let frame = drawn(&state);
    line_with(
        &frame,
        "│   ✓ Every clash in notes.md is solved.  n next file",
    );
    app.close();

    let (_scratch, app, state) = stopped("integration");
    let frame = drawn(&pick(&app, &state, 'o'));
    line_with(
        &frame,
        "│   ✓ Every clash in report.md is solved.  y review the merge",
    );
    app.close();
}

/// M10: from a solved file, `n` goes to the first open clash in the merge,
/// in the next file.
#[test]
fn m10_n_goes_to_the_first_open_clash() {
    let (_scratch, app, state) = stopped("pay-307");
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 'n');
    assert_eq!(at_clash(&state), ("report.md".to_owned(), Some(0)));
    let frame = drawn(&state);
    assert!(line_with(&frame, "report.md  1 open").contains("? report.md  1 open"));
    app.close();
}
