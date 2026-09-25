//! The `.`/right-click context menu from a real repository, through the real
//! app (m2-details-and-diffs/12, m5-history-and-ghost-preview/12, 13, 18).
//!
//! Split out of `it::preview`, which had grown past the 400-line rule with
//! two unrelated subjects in it (PROMPT.md §6.2; M5 groups 12/13 review,
//! M2). The fixture and the frame helpers stay in `it::preview` — this is
//! the same repository, looked at through the menu.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::preview::{FORK_NOW, forked, frame, frame_at, on_main, opened, rows_drawn};

// ---------------------------------------------------------------------------
// The `.` menu's history verbs (piece 6).
//
// `update::context_menu::verbs`' own tests settle what each verb *means* over
// scripted refs. Nothing there opens a repository, so nothing there can prove
// that `.` on a real branch row actually lists them, that the widget draws
// them, or that choosing one reaches `graft_ops::preview::plan`. That is what
// these do.
// ---------------------------------------------------------------------------

/// `.` on `main`'s row, with the menu's plan on hand.
/// Presses `j` until the cursor sits on `main`'s row — where the history
/// verbs have something to name.
pub(crate) fn on_mains_row(app: &App, state: AppState) -> AppState {
    let mut state = opened(app, state);
    let mut guard = 0;
    loop {
        assert!(
            guard < 20,
            "never found main's row: {:?}",
            rows_drawn(&state)
        );
        if on_main(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
        guard += 1;
    }
    state
}

fn menu_on_mains_row(app: &App, state: AppState) -> AppState {
    let state = on_mains_row(app, state);
    let state = app.key(&state, KeyCode::Char('.'));
    assert!(
        state.context_menu.is_some(),
        "`.` opens the menu: {:?}",
        state.status
    );
    state
}

/// The labels the menu lists, in order.
fn menu_labels(state: &AppState) -> Vec<String> {
    state
        .context_menu
        .as_ref()
        .expect("a menu is open")
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str().to_owned())
        .collect()
}

/// The menu is **opaque**: on every row it draws, everything right of the
/// label is blank, out to the end of the line.
///
/// The widget pads each of its own rows, so the popup's own columns were
/// never the problem — the popup is only as wide as its longest label, and
/// the graph rows it floats over kept drawing their date and sha columns
/// beside each verb. In a character grid that reads as if the verb were
/// bound to that sha (`Copy SHA (y)  …  now  6d3bd01`). The user found it by
/// eye in `docs/shots/m5-ui-context-menu-verbs-80x24.txt`; this is the
/// assertion the capture could not make (m5-history-and-ghost-preview/12).
///
/// `frame` trims each line's trailing blanks, so an opaque row is exactly
/// `" {label}"` and a bleeding one is longer. Both capture widths, because
/// the bleed is wider and louder at 160 columns than at 80.
#[test]
fn the_context_menu_is_opaque_on_every_row_it_draws() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);
    let labels = menu_labels(&state);
    assert!(labels.len() > 8, "the fixture's menu is long: {labels:?}");

    for (columns, rows) in [(80_u16, 24_u16), (160, 48)] {
        let drawn = frame(&state, columns, rows);
        for label in &labels {
            let wanted = format!(" {label}");
            let line = drawn
                .iter()
                .find(|line| line.starts_with(&wanted))
                .unwrap_or_else(|| {
                    panic!(
                        "{wanted:?} is drawn at {columns}x{rows}:\n{}",
                        drawn.join("\n")
                    )
                });
            assert_eq!(
                line,
                &wanted,
                "at {columns}x{rows} the menu row {label:?} carries what was \
                 underneath it to the right of its label:\n{}",
                drawn.join("\n")
            );
        }
    }
    app.close();
}

/// The band the menu clears runs to the edge of the **graph** pane and
/// stops there: with the detail pane open, every column of the detail pane
/// is exactly what it was before the menu opened.
///
/// `primary_pane_area` (`view::frame`) is what decides which half of the
/// split the band runs to, and nothing exercised it: the opacity test above
/// runs with the detail pane closed at both sizes, and so does the golden
/// snapshot. Swapping `split_area(…).0` for `.1` — clearing *across* the
/// detail pane — passed the whole suite (M5 groups 12/13 review, M3).
#[test]
fn the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = on_mains_row(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the detail pane's file list", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });

    // 160x48 lays the detail pane out to the right (`DetailLayout::Right`),
    // which is the split this is about.
    let width = 160_usize;
    let closed = frame(&state, 160, 48);
    let split = char_index_of(&closed[0], "COMMIT ")
        .unwrap_or_else(|| panic!("the detail pane draws its header:\n{}", closed.join("\n")));
    assert!(
        closed
            .iter()
            .filter_map(|line| char_index_of(line, "parents "))
            .chain(
                closed
                    .iter()
                    .filter_map(|line| char_index_of(line, "A main-two.txt"))
            )
            .all(|at| at == split),
        "the detail pane's own rows all start at its left edge, column {split}:\n{}",
        closed.join("\n")
    );

    let menu = app.key(&state, KeyCode::Char('.'));
    assert!(menu.context_menu.is_some(), "`.` opens the menu");
    let open = frame(&menu, 160, 48);

    for label in menu_labels(&menu) {
        let wanted = format!(" {label}");
        let row = open
            .iter()
            .position(|line| line.starts_with(&wanted))
            .unwrap_or_else(|| panic!("{wanted:?} is drawn:\n{}", open.join("\n")));
        let drawn = cells(&open[row], width);
        let before = cells(&closed[row], width);
        let left: String = drawn[..split].iter().collect();
        assert_eq!(
            left.trim_end(),
            wanted,
            "the menu row {label:?} carries what was under it out to the split:\n{}",
            open.join("\n")
        );
        assert_eq!(
            &drawn[split..],
            &before[split..],
            "the menu's band cleared into the detail pane on row {row} ({label:?}):\n{}",
            open.join("\n")
        );
    }
    app.close();
}

/// The column `needle` starts at, counting cells rather than bytes: these
/// frames are full of box drawing and `…`.
fn char_index_of(line: &str, needle: &str) -> Option<usize> {
    line.find(needle).map(|byte| line[..byte].chars().count())
}

/// One frame line as `width` cells. [`frame`] trims trailing blanks, so the
/// line has to be padded back out before two of them can be compared
/// column by column.
fn cells(line: &str, width: usize) -> Vec<char> {
    let mut cells: Vec<char> = line.chars().collect();
    cells.resize(width, ' ');
    cells
}

/// A trailing `(m)` on a menu label promises that `m` is live **right now**.
/// So pressing it with the menu open closes the menu and runs the verb, in
/// one keystroke, landing on exactly the picture `m` draws when no menu was
/// open (`docs/design/02-keymap.md` §2.1: the `.` menu claims `j`/`k`/`Enter`
/// /`Esc` and nothing else).
///
/// Compared frame-to-frame rather than field-to-field: "as if the menu had
/// not been open" is a claim about what the user ends up looking at.
#[test]
fn a_verb_key_the_menu_advertises_closes_it_and_runs_the_verb() {
    let scratch = forked();
    let planned = |state: &AppState| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    };

    // Each branch gets its own app over the same repository, and each is left
    // to settle before its frame is taken. `m` only opens a preview — nothing
    // is written — so the second app opens on the same commits as the first.
    //
    // They used to fork one `AppState` and share one job pool, which cannot
    // work: a finished job's event goes to whichever branch is pumping, so the
    // walk's completion landed on one of them and the other kept saying `date
    // order, sorting…` forever. That was invisible until
    // m5-history-and-ghost-preview/40 shortened the repository path in the
    // header — the temp path used to fill all 80 columns and push the status
    // off the end of both frames.
    let (app, state) = App::open(scratch.path());
    let row = on_mains_row(&app, state);
    let menu = app.key(&row, KeyCode::Char('.'));
    assert!(
        menu.context_menu.is_some(),
        "`.` opens the menu: {:?}",
        menu.status
    );
    let labels = menu_labels(&menu);
    assert!(
        labels.iter().any(|label| label.ends_with("(m)")),
        "the menu advertises `m`: {labels:?}"
    );
    let through_menu = app.key(&menu, KeyCode::Char('m'));
    assert!(
        through_menu.context_menu.is_none(),
        "`m` closes the menu on its way through"
    );
    assert!(
        through_menu.preview.is_some(),
        "`m` ran the merge it advertised instead of being swallowed: {:?}",
        through_menu.status
    );
    let through_menu = app.pump(through_menu, "the merge plan", planned);
    let through_menu = sort_landed(&app, through_menu);
    let through_menu = frame(&through_menu, 80, 24);
    app.close();

    let (app, state) = App::open(scratch.path());
    let row = on_mains_row(&app, state);
    let direct = app.key(&row, KeyCode::Char('m'));
    let direct = app.pump(direct, "the merge plan", planned);
    let direct = sort_landed(&app, direct);
    assert_eq!(
        through_menu,
        frame(&direct, 80, 24),
        "`.` then `m` must land where a bare `m` lands"
    );
    app.close();
}

/// `m` is not alone: every letter the menu advertises has to be live, not
/// just the one the defect was found with. Each of these closes the menu and
/// then does **its own** job — named here per key, not as "the screen
/// changed".
///
/// It used to assert only that the frame differed from the one `Esc` leaves.
/// That is "something happened", and for `u` on this fixture the something
/// was a line on the status bar: a future generic "key not bound" status
/// would have kept it green (M5 groups 12/13 review, m3).
///
/// `f`/`p`/`P` are left out on purpose: pressing them really starts a
/// network job, which is not a fast-tier test. `y` is left out because this
/// harness cannot run it — `CopyToClipboard` is terminal output the event
/// loop performs, not a job the test runner can answer. That all four are
/// handed on is pinned by
/// `graft::update::context_menu::tests::context_menu_key_closes_and_hands_a_verb_key_on`,
/// and that the menu's own items run the same code `f`/`p`/`P` run is pinned
/// by `graft::update::context_menu::tests::the_menu_runs_what_the_dropped_keys_run`.
#[test]
fn every_verb_the_menu_advertises_is_live_while_it_is_open() {
    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('m'),
        KeyCode::Char('r'),
        KeyCode::Char('u'),
    ] {
        let scratch = forked();
        let tip = main_tip(scratch.path());
        let (app, state) = App::open(scratch.path());
        let menu = menu_on_mains_row(&app, state);

        let merely_closed = app.key(&menu, KeyCode::Escape);
        let pressed = app.key(&menu, code);
        assert!(
            pressed.context_menu.is_none(),
            "{code:?} closes the menu on its way through"
        );
        assert_ne!(
            frame(&pressed, 80, 24),
            frame(&merely_closed, 80, 24),
            "{code:?} was swallowed: the menu closed and nothing else happened"
        );
        // The effect this key, and no other, is supposed to have.
        the_effect_of(code, &pressed, &tip);
        app.close();
    }
}

/// What each letter the menu advertises is supposed to *do*, named per key.
/// The cursor is on `main`'s row, whose commit is `tip`.
fn the_effect_of(code: KeyCode, pressed: &AppState, tip: &str) {
    match code {
        KeyCode::Char('x') => {
            let base = pressed
                .compare_base
                .as_ref()
                .unwrap_or_else(|| panic!("`x` pinned no compare base: {:?}", pressed.status));
            assert_eq!(base.oid.to_string(), tip, "`x` marked the cursor's commit");
        }
        KeyCode::Char('m') => assert!(
            matches!(
                planned(pressed, "m"),
                graft_ops::operation::Operation::Merge(_)
            ),
            "`m` planned a merge, not {:?}",
            planned(pressed, "m")
        ),
        KeyCode::Char('r') => assert!(
            matches!(
                planned(pressed, "r"),
                graft_ops::operation::Operation::Rebase(_)
            ),
            "`r` planned a rebase, not {:?}",
            planned(pressed, "r")
        ),
        KeyCode::Char('u') => {
            assert!(
                pressed.operating.is_some(),
                "`u` started an undo: {:?}",
                pressed.status
            );
            let said = format!("{:?}", pressed.status).to_lowercase();
            assert!(
                said.contains("undo"),
                "`u` says on the status line what it is doing: {said}"
            );
        }
        other => panic!("no effect named for {other:?}"),
    }
}

/// The operation the preview `key` opened would run.
fn planned<'a>(state: &'a AppState, key: &str) -> &'a graft_ops::operation::Operation {
    state
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .unwrap_or_else(|| panic!("`{key}` opened no preview: {:?}", state.status))
}

/// `Esc` is one of the four keys the menu does claim: it closes the menu and
/// stops there, running nothing (`docs/design/02-keymap.md` §2.1).
#[test]
fn esc_closes_the_menu_and_runs_nothing() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let menu = menu_on_mains_row(&app, state);
    let closed = app.key(&menu, KeyCode::Escape);
    assert!(closed.context_menu.is_none(), "`Esc` closes the menu");
    assert!(
        closed.preview.is_none(),
        "`Esc` is not a verb: {:?}",
        closed.status
    );
    app.close();
}

/// `.` on a real branch row lists the history verbs, named against the real
/// branches, and the widget really draws them.
#[test]
fn the_context_menu_lists_its_history_verbs_and_draws_them() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);

    let labels = menu_labels(&state);
    for wanted in [
        "Checkout main",
        "Merge main into feature (m)",
        "Rebase feature onto main (r)",
        "Reset feature to here, mixed",
        "Reset feature to here, hard",
        "Delete branch main",
    ] {
        assert!(
            labels.iter().any(|label| label == wanted),
            "{wanted:?} missing from {labels:?}"
        );
    }

    // Drawn, not merely listed: the rows come off the real buffer.
    let text = frame(&state, 100, 30).join("\n");
    assert!(text.contains("Merge main into feature (m)"), "{text}");
    assert!(text.contains("Rebase feature onto main (r)"), "{text}");
    app.close();
}

/// Choosing a verb opens the real Ghost Preview of exactly that operation,
/// through `graft_ops::preview::plan`, and writes nothing.
#[test]
fn choosing_a_menu_verb_previews_that_operation_and_writes_nothing() {
    let scratch = forked();
    let before = git(
        scratch.path(),
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    );
    let (app, state) = App::open(scratch.path());
    let state = menu_on_mains_row(&app, state);

    let wanted = "Rebase feature onto main (r)";
    let at = menu_labels(&state)
        .iter()
        .position(|label| label == wanted)
        .unwrap_or_else(|| panic!("{wanted:?} missing from {:?}", menu_labels(&state)));
    let expected = {
        let open = state.context_menu.as_ref().expect("a menu is open");
        let graft_ui::context_menu::ContextAction::History(index) = open.menu.items[at].action
        else {
            panic!("{wanted:?} is a history verb")
        };
        open.verbs[index].op.clone()
    };

    let mut state = state;
    for _ in 0..at {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = app.key(&state, KeyCode::Enter);

    assert!(state.context_menu.is_none(), "the menu closed");
    let preview = state.preview.as_ref().expect("a preview opened");
    assert_eq!(
        preview.subject,
        graft::update::preview::PreviewSubject::Operation {
            op: Box::new(expected),
            opened_by: ".",
        },
        "the operation previewed is the one the label named"
    );

    // The plan comes back from the real `graft_ops::preview::plan`, with the
    // two commits it would replay in it.
    let state = app.pump(state, "the menu verb's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let text = frame(&state, 100, 30).join("\n");
    assert!(text.contains("Preview:"), "{text}");
    assert_eq!(
        git(
            scratch.path(),
            &["for-each-ref", "--format=%(refname) %(objectname)"]
        ),
        before,
        "planning is a read: nothing moved"
    );
    app.close();
}

/// The capture under `docs/shots/`, from this same real path, **asserted**
/// against what is committed there — the **user** judges the menu by eye
/// (CLAUDE.md rule 6), and this keeps what he judged from changing behind
/// his back.
///
/// It can assert now because the frame is reproducible: [`forked`] stamps a
/// fixed date on every commit it makes and [`FORK_NOW`] fixes the instant
/// the ages are drawn against. Before that it was not, and a snapshot of a
/// frame that wobbles is worse than none
/// (m5-history-and-ghost-preview/13).
///
/// To regenerate after a deliberate change: `GRAFT_UPDATE_SHOTS=1 cargo test
/// -p graft --test it context_menu::capture_context_menu_for_docs_shots`, then
/// look at the diff before committing it. That run is **red** — see
/// [`graft_fixtures::capture`].
#[test]
fn capture_context_menu_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    // Settled before it is photographed; `captures::frame_at` draws it at
    // the fixed path the caption claims.
    let state = settled(&app, opened(&app, state));
    let state = menu_on_mains_row(&app, state);

    let mut drifted = Vec::new();
    for (label, columns, rows) in [("80x24", 80u16, 24u16), ("160x48", 160, 48)] {
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(FORK_NOW)));
        let mut out = context_menu_caption(label, columns, rows);
        out.push('\n');
        for line in &drawn {
            out.push_str(line);
            out.push('\n');
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(format!("m5-ui-context-menu-verbs-{label}.txt"));
        if let Err(why) = graft_fixtures::check(&path, &out) {
            drifted.push(why);
        }
    }
    app.close();
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// Pumped until the topological sort has landed and the header stops
/// saying `date order, sorting…`.
///
/// A "what right looks like" frame has to be a settled one: both
/// context-menu captures were taken mid-sort, so the picture the user
/// judged showed a transient (M5 groups 12/13 review, m2).
/// Pumped until the header stops saying `date order, sorting…`, and no
/// further.
///
/// [`settled`] waits for the Mistake Radar as well, which is right for a
/// capture but wrong here: the radar's context never arrives while a preview
/// is open, so that wait times out. A frame that only has to agree about the
/// sort waits for the sort.
///
/// The width is 160 on purpose, like [`settled`]'s: the status sits at the
/// right of the header and an 80-column frame can truncate it away, which
/// would make this return before the sort had landed.
fn sort_landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the topological sort", |state| {
        !frame(state, 160, 48)[0].contains("sorting")
    })
}

pub(crate) fn settled(app: &App, state: AppState) -> AppState {
    app.pump(state, "the walk and the radar to settle", |state| {
        // The radar too: its context arrives on its own job, and the marker
        // column it fills is part of the picture. Waiting only for the sort
        // left the `\u{24d8}` on `feature`'s tip a race — present or absent
        // depending on how loaded the machine was.
        matches!(
            state.radar.context,
            graft::update::detail::Loading::Ready(_)
        ) && !frame(state, 160, 48)[0].contains("sorting")
            // The footer's commit-graph line waits on `git --version`.
            && state.system_git.is_some()
    })
}

/// The "what right looks like" block the M5 contact sheet puts this capture
/// under (ADR-0037).
///
/// Every line here is a claim the user checks by eye, so every line has to be
/// true of **this** width. The block was byte-identical in both files until
/// m5-history-and-ghost-preview/12, which left the 160-column capture
/// claiming to be the 80-column one.
fn context_menu_caption(label: &str, columns: u16, rows: u16) -> String {
    let claims = context_menu_claims(columns);
    format!(
        "# m5-ui-context-menu-verbs-{label}.txt: {columns}x{rows}, drawn by `view::view`\n\
             # from a real AppState — a real repository (`main` three commits, `feature`\n\
             # forked off the first with two of its own, `feature` checked out), opened\n\
             # over the real job pool, cursor moved to `main`'s row and `.` pressed\n\
             # through `update`\n\
             # (crates/graft/tests/it/context_menu.rs, capture_context_menu_for_docs_shots).\n\
             # This file is a golden snapshot: that test asserts the frame still\n\
             # matches it, so drift fails the suite. Regenerate on purpose with\n\
             # GRAFT_UPDATE_SHOTS=1. Not a pty capture: the terminal is ratatui's\n\
             # TestBackend, so this is a character grid with no colour attributes in\n\
             # it — CLAUDE.md rule 5, colour ships unverified. The fixture's commits\n\
             # carry fixed dates and the ages are drawn against a fixed clock, so the\n\
             # frame is the same on every run and does not age. The repository is\n\
             # moved to `/code/api` **before** the frame is drawn, so the header is\n\
             # fitted to the path shown here rather than to a temporary one, and the\n\
             # walk's sort and the radar have landed \u{2014} a settled screen, not a transient.\n\
             #\n\
             {claims}"
    )
}

/// The claims the user checks by eye, the half of the caption that is about
/// the picture rather than about where it came from. Split out of
/// [`context_menu_caption`], which had grown past the line rule.
fn context_menu_claims(columns: u16) -> String {
    let width_line = if columns <= 80 {
        format!(
            "#  - At {columns} columns \u{2014} the narrow end \u{2014} the menu still fits\n\
             #    inside the graph pane rather than running off it.\n"
        )
    } else {
        format!(
            "#  - At {columns} columns the graph's date and sha columns sit far to the\n\
             #    right of the menu, and they stop dead at the menu's own rows: the\n\
             #    popup clears the whole band it stands on, so no verb has a stray\n\
             #    sha beside it.\n"
        )
    };
    format!(
        "# What right looks like:\n\
             #  - The menu is anchored under `main`'s row and is wide enough for its\n\
             #    longest label, with the cursor on the first item.\n\
             #  - `Checkout main` then `Cherry-pick <sha7>` come first (m6-conflicts/71),\n\
             #    then `Explain` if the row is marked, then the row's own actions\n\
             #    (`Copy SHA (y)`, `Compare with this (x)`, `Open details (\u{23ce})`),\n\
             #    then the rest of the history verbs, then the ambient\n\
             #    ones (`Fetch (f)` \u{2026} `Undo (u)`). The menu lists every verb valid for\n\
             #    this row at every width \u{2014} it does not filter against the hint bar,\n\
             #    because a verb you can only discover at 160 columns is the failure\n\
             #    ADR-0039 exists to prevent.\n\
             #  - The menu is opaque. On each of its rows, everything right of the\n\
             #    label is blank out to the edge of the pane: no date or sha from the\n\
             #    commit row underneath survives beside a verb, where it would read as\n\
             #    if the verb were bound to that sha.\n\
             #  - The history verbs name the real branches, not placeholders: `Checkout\n\
             #    main`, `Merge main into feature (m)`, `Rebase feature onto main (r)`,\n\
             #    `Cherry-pick <sha7>`, `Revert <sha7>`, `Reset feature to here,\n\
             #    mixed`/`, hard`, `Delete branch main`. `feature` is the branch\n\
             #    checked out, `main` is the row under the cursor.\n\
             #  - No verb names the branch you are standing on where git would refuse\n\
             #    it: there is no `Checkout feature` and no `Delete branch feature`.\n\
             #  - A trailing `(key)` means one thing everywhere in the menu: that\n\
             #    letter is live right now, so next time you can skip the menu. Only\n\
             #    `Merge` and `Rebase` carry one, because only `m` and `r` exist. A\n\
             #    qualifier that is not a key never wears that shape \u{2014} hence `Reset\n\
             #    \u{2026}, mixed`, not `(mixed)` (`docs/design/02-keymap.md` \u{a7}2.1).\n\
             #  - Choosing any of them opens a picture, not a write (PROMPT.md \u{a7}1.7).\n\
             #  - The marker column has settled: `feature`'s tip carries the radar's\n\
             #    muted `\u{24d8}`. Earlier captures were taken before the radar\n\
             #    answered and the column was blank \u{2014} that was the transient, not\n\
             #    this. What the finding says is not part of this picture.\n\
             {width_line}\
             #\n"
    )
}

// ---------------------------------------------------------------------------
// Right-click: the menu's row and the verb's row are the same row
// (m5-history-and-ghost-preview/18 F1).

/// The subject of `main`'s tip in [`forked`] — the row these tests
/// right-click, which is never the row the cursor starts on.
const MAIN_TIP_SUBJECT: &str = "main moves again";

/// `main`'s tip as `git` itself reports it. Every assertion below measures
/// the verb against this, and the menu's labels against the branch name —
/// never the menu against the verb, which is what the defect would have
/// kept agreeing with.
fn main_tip(repo: &Path) -> String {
    git(repo, &["rev-parse", "main"]).trim().to_owned()
}

/// The screen row `subject` is drawn on, read out of the frame the user
/// sees rather than from the geometry the code under test uses.
fn screen_row_of(state: &AppState, subject: &str) -> u16 {
    let drawn = frame(
        state,
        crate::app_harness::SIZE.columns,
        crate::app_harness::SIZE.rows,
    );
    let hits: Vec<usize> = drawn
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(subject))
        .map(|(y, _)| y)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{subject:?} is drawn on exactly one row; frame: {drawn:?}"
    );
    u16::try_from(hits[0]).expect("a screen row fits in u16")
}

/// Opened, cursor left where it lands (`feature`'s tip), then a right-click
/// on `main`'s row — a row the cursor is **not** on.
fn right_click_on_mains_row(app: &App, state: AppState) -> AppState {
    let state = opened(app, state);
    assert!(
        !on_main(&state),
        "the cursor has to start away from main's row or this proves nothing"
    );
    let row = screen_row_of(&state, MAIN_TIP_SUBJECT);
    let menu = app.right_click(&state, 2, row);
    assert!(
        menu.context_menu.is_some(),
        "a right-click on a commit row opens its menu: {:?}",
        menu.status
    );
    menu
}

/// A right-click is a pick: it moves the cursor onto the row it opens the
/// menu on, so "a verb acts on the cursor" and "the menu names this row"
/// stay the same sentence (`docs/design/02-keymap.md` §2.1).
///
/// Before this, the menu opened on the clicked row and left the cursor
/// alone, and group 13's fall-through then ran the letter against the
/// cursor: the label named one commit and the verb moved another
/// (m5-history-and-ghost-preview/18 F1, review `96edfd7` M1).
#[test]
fn a_right_click_moves_the_cursor_to_the_row_it_opens_the_menu_on() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    assert!(
        on_main(&menu),
        "the cursor followed the right-click onto main's row"
    );
    assert!(
        !menu.on_working_tree,
        "a commit row is not the working tree"
    );
    app.close();
}

/// The menu's labels and the verb it hands on both name the row that was
/// clicked — each checked against `git`, not against the other.
///
/// `m`: the label says `Merge main into feature`, and the merge that opens
/// carries `main`'s tip as its source. With the cursor left on `feature`'s
/// tip this used to refuse with `already on it` and draw nothing.
#[test]
fn m_after_a_right_click_merges_the_branch_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let labels = menu_labels(&menu);
    assert!(
        labels
            .iter()
            .any(|label| label == "Merge main into feature (m)"),
        "the menu names main as what it would merge: {labels:?}"
    );

    let pressed = app.key(&menu, KeyCode::Char('m'));
    let pressed = app.pump(pressed, "the merge plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let op = pressed
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .unwrap_or_else(|| panic!("`m` opened no preview: {:?}", pressed.status));
    match op {
        graft_ops::operation::Operation::Merge(params) => assert_eq!(
            params.source_tip.to_string(),
            tip,
            "the merge's source is the row the menu named"
        ),
        other => panic!("`m` planned something else: {other:?}"),
    }
    app.close();
}

/// `r`: the label says `Rebase feature onto main`, and the rebase that
/// opens carries `main`'s tip as what it replays onto.
#[test]
fn r_after_a_right_click_rebases_onto_the_branch_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let labels = menu_labels(&menu);
    assert!(
        labels
            .iter()
            .any(|label| label == "Rebase feature onto main (r)"),
        "the menu names main as what it would rebase onto: {labels:?}"
    );

    let pressed = app.key(&menu, KeyCode::Char('r'));
    let pressed = app.pump(pressed, "the rebase plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let op = pressed
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .unwrap_or_else(|| panic!("`r` opened no preview: {:?}", pressed.status));
    match op {
        graft_ops::operation::Operation::Rebase(params) => assert_eq!(
            params.onto_tip.to_string(),
            tip,
            "the rebase replays onto the row the menu named"
        ),
        other => panic!("`r` planned something else: {other:?}"),
    }
    app.close();
}

/// `x`: the compare base it pins is the clicked commit, not the cursor's.
#[test]
fn x_after_a_right_click_marks_the_commit_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let pressed = app.key(&menu, KeyCode::Char('x'));
    let base = pressed
        .compare_base
        .as_ref()
        .unwrap_or_else(|| panic!("`x` pinned nothing: {:?}", pressed.status));
    assert_eq!(
        base.oid.to_string(),
        tip,
        "`x` marked the row the menu named"
    );
    app.close();
}

/// `y` is the silent one: nothing is drawn and nothing is confirmed, so a
/// wrong sha leaves the app looking correct and comes back hours later in
/// a shell command the user pasted it into. It gets its own test for that
/// reason (m5-history-and-ghost-preview/18 F1).
///
/// The commands are taken rather than dispatched: `CopyToClipboard` is
/// terminal output the event loop performs, and the job runner refuses it.
#[test]
fn y_after_a_right_click_copies_the_sha_the_menu_named() {
    let scratch = forked();
    let tip = main_tip(scratch.path());
    let (app, state) = App::open(scratch.path());
    let menu = right_click_on_mains_row(&app, state);
    let labels = menu_labels(&menu);
    assert!(
        labels.iter().any(|label| label == "Copy SHA (y)"),
        "the menu advertises `y`: {labels:?}"
    );

    let (pressed, commands) = crate::app_harness::press_for_commands(
        &menu,
        graft_core::event::KeyPress::plain(KeyCode::Char('y')),
    );
    assert_eq!(
        commands,
        vec![graft::update::Command::CopyToClipboard(tip.clone())],
        "`y` copied the sha of the row the menu named"
    );
    match &pressed.status {
        graft::update::Status::Copied { short } => assert!(
            tip.starts_with(short.as_str()),
            "the footer names the sha that was copied: {short} of {tip}"
        ),
        other => panic!("`y` said nothing: {other:?}"),
    }
    app.close();
}
