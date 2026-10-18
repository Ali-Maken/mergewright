//! m7-power-tools/36: `B` in the detail pane is blame while the
//! commit-graph offer is up, and two of 15's daily-wording fixes walked in
//! the real app (single-`Close` boxes, the fresh commit editor). Real `git`,
//! the real job pool, the real `view::view`. Colour unverified (rule 5).

use mergewright::update::preview::{PreviewState, preview_box};
use mergewright::update::{AppState, Command, CommitGraphOffer};
use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_ui::detail::DetailFocus;

use crate::app_harness::{App, git, press_for_commands};
use crate::m6_stash_put_back::{cursor_to, rev};
use crate::m7_walk::blame::{LINE, cursor_line, drawn, on_line_22};

/// The offer as a slow sort would have raised it. The fixture's sort takes
/// milliseconds, under `SLOW_SORT`, so since m7-power-tools/15 no real open
/// raises it; the state is the one `commit_graph::sort_landed` sets.
fn with_the_offer_up(mut state: AppState) -> AppState {
    state.commit_graph = CommitGraphOffer::Offered;
    assert!(
        state.commit_graph_line().is_some(),
        "the offer row is drawn: {:?}",
        state.system_git
    );
    state
}

/// The session fixture with no commit-graph file, so the offer can stand.
fn session_without_a_graph() -> mergewright_fixtures::Scratch {
    mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Session)
}

/// M3: `⇥` to the diff, `B` → a blame place, never the commit-graph write.
#[test]
fn b_in_the_diff_pane_blames_while_the_offer_is_up() {
    let scratch = session_without_a_graph();
    let (app, state) = on_line_22(scratch.path(), 187, 44);
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    let state = with_the_offer_up(state);
    // Before `B`: the offer row stands, its keys gone with the pane focused.
    let before = drawn(&state, 187, 44).join("\n");
    assert!(before.contains("no commit-graph file"), "{before}");
    assert!(
        !before.contains("B build one") && !before.contains("X never here"),
        "{before}"
    );

    let (pressed, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('B')));
    assert!(
        !commands.contains(&Command::WriteCommitGraph),
        "{commands:?}"
    );
    assert_eq!(pressed.commit_graph, CommitGraphOffer::Offered);

    let state = crate::m7_walk::blame::blame(&app, &state);
    assert_eq!(cursor_line(&state).final_line, LINE, "`B` is blame");
    assert_eq!(state.commit_graph, CommitGraphOffer::Offered);

    let frame = drawn(&state, 187, 44);
    let text = frame.join("\n");
    assert!(!text.contains("commit-graph written"), "{text}");
    assert!(
        !text.contains("B build one") && !text.contains("X never here"),
        "{text}"
    );
    crate::captures::check_shot(
        "m7/diff-pane-b-with-offer-187x44.txt",
        &format!(
            "{OFFER_HEADER}\n== 1. the diff focused, cursor on line 22, the offer up ==\n\n{before}\n\n             == 2. `B` ==\n\n{text}\n"
        ),
    );
    app.close();
}

const OFFER_HEADER: &str = "\
# m7/diff-pane-b-with-offer-187x44.txt: 187x44, drawn by `view::view` from a
# real AppState (crates/mergewright/tests/it/offer_and_blame.rs,
# b_in_the_diff_pane_blames_while_the_offer_is_up, asserted). Not a pty
# capture: no colour (CLAUDE.md rule 5, colour unverified).
#
# The session fixture, no commit-graph file, the offer up (set as a slow
# sort would). `⏎` on main, the file, `⏎` to the diff, cursor on line 22,
# then `B`.
#
# What right looks like:
#  - Frame 1: the offer row under the graph reads `no commit-graph file:
#    history sorts on every open` with no `B`/`X`: the pane has the
#    keyboard.
#  - Frame 2: the blame place is open on src/auth/session.rs (the title
#    names it), the cursor on line 22 with its who/when trailer. The place
#    takes the screen, offer row included.
#  - No `commit-graph written` anywhere: `B` blamed, it did not build.
";

/// A repository where `m` on `old` has nothing to do: `old` is `main~1`.
fn merge_with_nothing_to_do() -> (mergewright_fixtures::Scratch, App, AppState) {
    let scratch = session_without_a_graph();
    git(scratch.path(), &["branch", "old", "main~1"]);
    let old = rev(scratch.path(), "old");
    let (app, state) = App::open_with_clock(
        scratch.path(),
        mergewright::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW),
    );
    let state = app.resize(&state, 120, 40);
    let state = crate::preview::opened(&app, state);
    let state = cursor_to(&app, state, &old);
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the merge's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, mergewright::update::detail::Loading::Ready(_)))
    });
    (scratch, app, state)
}

/// The bottom border of the box on screen: the row holding `[ Close ]`'s
/// box's `╰`, below the choices.
fn box_footer(frame: &[String]) -> String {
    let close = frame
        .iter()
        .position(|row| row.contains("[ Close ]") || row.contains("Close"))
        .unwrap_or_else(|| panic!("no Close: {frame:#?}"));
    frame[close..]
        .iter()
        .find(|row| row.contains('╰'))
        .cloned()
        .unwrap_or_else(|| panic!("no bottom border: {frame:#?}"))
}

/// :860: every box with one `Close` — nothing to do, refused, preview
/// failed, undrawable — has the footer `esc back`, not `←/→ choose  ⏎ act`.
/// One real preview, re-pointed at each caller's state.
#[test]
fn a_single_close_box_offers_only_esc() {
    let (_scratch, app, state) = merge_with_nothing_to_do();
    let preview = state.preview.clone().expect("a preview");
    let callers: [(&str, PreviewState); 4] = [
        ("nothing to do", preview.clone()),
        (
            "refused",
            PreviewState {
                plan: mergewright::update::detail::Loading::Failed("main has moved".to_owned()),
                refused: true,
                ..preview.clone()
            },
        ),
        (
            "preview failed",
            PreviewState {
                plan: mergewright::update::detail::Loading::Failed("git said no".to_owned()),
                refused: false,
                ..preview.clone()
            },
        ),
        (
            "undrawable",
            PreviewState {
                undrawable: Some(
                    mergewright_core::oid::Oid::from_hex("0707070707070707070707070707070707070707")
                        .expect("an object id"),
                ),
                ..preview.clone()
            },
        ),
    ];
    for (caller, preview) in callers {
        let mut shown = state.clone();
        shown.preview = Some(preview);
        let view = preview_box(&shown).expect("a box");
        assert_eq!(view.choices().len(), 1, "{caller}: {view:?}");
        let frame = crate::captures::frame_at(&shown, 120, 40, None);
        let footer = box_footer(&frame);
        assert!(footer.contains("esc back"), "{caller}: {footer}");
        assert!(!footer.contains("choose"), "{caller}: {footer}");
        if caller == "refused" {
            crate::captures::check_shot(
                "m7/refused-box-single-close-120x40.txt",
                &format!("{REFUSED_HEADER}\n{}\n", frame.join("\n")),
            );
        }
    }
    app.close();
}

const REFUSED_HEADER: &str = "\
# m7/refused-box-single-close-120x40.txt: 120x40, drawn by `view::view` from
# a real AppState (crates/mergewright/tests/it/offer_and_blame.rs,
# a_single_close_box_offers_only_esc, asserted). Not a pty capture: no
# colour (CLAUDE.md rule 5, colour unverified).
#
# `m` on a branch already in main, its preview then refused (the refusal's
# words, `main has moved`, set on the real preview: a refusal from git
# cannot be timed in a test).
#
# What right looks like:
#  - The box: the refusal's line, one `[ Close ]`.
#  - Its bottom border reads only `esc back` (review 86's words): no
#    `←/→ choose  ⏎ act  esc cancel` over a single button.
";

/// :776: the editor opened on staged work, nothing typed — no `subject
/// empty` anywhere.
#[test]
fn capture_commit_editor_fresh() {
    let scratch = session_without_a_graph();
    std::fs::write(scratch.path().join("notes.md"), "a note\n")
        .unwrap_or_else(|why| panic!("write notes.md: {why}"));
    git(scratch.path(), &["add", "notes.md"]);
    let (app, state) = App::open_with_clock(
        scratch.path(),
        mergewright::session::Clock::Fixed(mergewright_fixtures::CORPUS_NOW),
    );
    let state = app.resize(&state, 120, 40);
    let state = crate::preview::opened(&app, state);
    let state = app.pump(state, "the working tree", |state| {
        state.working_tree.is_some()
    });
    // The screen with its template asked, its counts and the first file's
    // changes in: a frame that still says `reading…` is a race.
    let state = crate::commit_screen::opened(&app, &state);
    let frame = crate::captures::frame_at(
        &state,
        120,
        40,
        Some(mergewright::session::Clock::Fixed(mergewright_fixtures::CORPUS_NOW)),
    );
    let text = frame.join("\n");
    assert!(!text.contains("subject empty"), "{text}");
    crate::captures::check_shot(
        "m7/commit-editor-fresh-120x40.txt",
        &format!("{EDITOR_HEADER}\n{text}\n"),
    );
    app.close();
}

const EDITOR_HEADER: &str = "\
# m7/commit-editor-fresh-120x40.txt: 120x40, drawn by `view::view` from a
# real AppState (crates/mergewright/tests/it/offer_and_blame.rs,
# capture_commit_editor_fresh, asserted). Not a pty capture: no colour
# (CLAUDE.md rule 5, colour unverified).
#
# The session fixture with notes.md staged; `c`.
#
# What right looks like:
#  - The commit screen (ease-pass/08), its subject empty, the cursor in it;
#    `notes.md` under `Going in`, its changes on the right.
#  - No `subject empty` anywhere (m7-power-tools/15); the bar's `⏎` slot
#    says `type a message first` instead, as the commit mock does.
";
