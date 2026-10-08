use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::sanitize;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(ColorDepth::TrueColor, false),
    )
}

fn row(name: &str, message: &str, age: &str, files: usize) -> StashRowView {
    StashRowView {
        name: sanitize(name),
        message: sanitize(message),
        age: sanitize(age),
        files,
    }
}

fn view<'a>(rows: Vec<StashRowView>, status: StashStatus) -> StashView<'a> {
    StashView {
        rows,
        selected: 0,
        preview: None,
        prompt: None,
        status,
        notice: None,
        notice_is_failure: false,
        box_open: false,
    }
}

fn grid(view: &StashView<'_>, width: u16, height: u16) -> Vec<String> {
    let theme = theme();
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("a test backend opens");
    terminal
        .draw(|frame| {
            frame.render_widget(
                StashScreen {
                    keys: Keymap::shipped(),
                    view,
                    theme: &theme,
                },
                frame.area(),
            );
        })
        .expect("drawing into a test backend");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// ADR-0058 §3: the cursor row's text clears 4.5:1 on `selection` in
/// dark B.
#[test]
fn text_on_the_cursor_row_is_legible() {
    let theme = theme();
    let shown = view(
        vec![
            row("stash@{0}", "WIP on main: tidy", "2h", 3),
            row("stash@{1}", "spike", "3d", 1),
        ],
        StashStatus::Ready,
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 12)).expect("test backend");
    terminal
        .draw(|frame| {
            frame.render_widget(
                StashScreen {
                    keys: Keymap::shipped(),
                    view: &shown,
                    theme: &theme,
                },
                frame.area(),
            );
        })
        .expect("draws");
    let checked =
        crate::test_support::assert_legible_on_selection(terminal.backend().buffer(), &theme);
    assert!(checked > 10, "the cursor row is drawn ({checked})");
}

#[test]
fn the_header_counts_the_entries_and_the_rows_read_as_the_design_says() {
    let view = view(
        vec![
            row("stash@{0}", "\"token refresh\"", "2h ago", 3),
            row("stash@{1}", "\"quick fix\"", "1d ago", 1),
        ],
        StashStatus::Ready,
    );
    let lines = grid(&view, 80, 10);
    assert_eq!(lines[0], "STASHES (2)");
    assert!(lines[1].contains("stash@{0}"), "{:?}", lines[1]);
    assert!(lines[1].contains("\"token refresh\""), "{:?}", lines[1]);
    assert!(lines[1].contains("2h ago"), "{:?}", lines[1]);
    assert!(lines[1].contains("3 files"), "{:?}", lines[1]);
    assert!(lines[2].contains("1 file"), "{:?}", lines[2]);
    assert!(!lines[2].contains("1 files"), "{:?}", lines[2]);
}

#[test]
fn an_empty_list_says_what_makes_one() {
    let lines = grid(&view(Vec::new(), StashStatus::Empty), 80, 10);
    assert!(
        lines
            .iter()
            .any(|line| line.contains(&empty(Keymap::shipped()))),
        "{lines:?}"
    );
}

#[test]
fn a_failed_read_says_why() {
    let lines = grid(
        &view(Vec::new(), StashStatus::Failed(sanitize("reflog is gone"))),
        80,
        10,
    );
    assert!(
        lines.iter().any(|line| line.contains("reflog is gone")),
        "{lines:?}"
    );
}

#[test]
fn the_prompt_marks_the_chosen_scope_in_text_not_colour() {
    let field = FieldView {
        label: "message",
        text: sanitize("wip"),
        cursor: Some(3),
        guide_at: None,
        counter: None,
        focused: true,
        secret: None,
    };
    let mut screen = view(Vec::new(), StashStatus::Empty);
    screen.prompt = Some(StashPromptView {
        title: sanitize("Stash the working tree"),
        message: field,
        staged_only: true,
        refusal: None,
    });
    let lines = grid(&screen, 80, 10);
    let options = lines
        .iter()
        .find(|line| line.contains("staged only"))
        .expect("the options row draws");
    assert!(options.contains("( ) all incl. untracked"), "{options:?}");
    assert!(options.contains("(•) staged only"), "{options:?}");
}

#[test]
fn a_refused_save_says_so_under_the_options() {
    let field = FieldView {
        label: "message",
        text: sanitize(""),
        cursor: Some(0),
        guide_at: None,
        counter: None,
        focused: true,
        secret: None,
    };
    let mut screen = view(Vec::new(), StashStatus::Empty);
    screen.prompt = Some(StashPromptView {
        title: sanitize("Stash the working tree"),
        message: field,
        staged_only: false,
        refusal: Some(sanitize("nothing to stash")),
    });
    let lines = grid(&screen, 80, 10);
    assert!(
        lines.iter().any(|line| line.contains("nothing to stash")),
        "{lines:?}"
    );
}

#[test]
fn a_notice_draws_above_the_footer() {
    let mut screen = view(
        vec![row("stash@{0}", "\"a\"", "2h ago", 1)],
        StashStatus::Ready,
    );
    screen.notice = Some(sanitize("stash touches files you changed: a.rs"));
    screen.notice_is_failure = true;
    let lines = grid(&screen, 80, 10);
    assert!(
        lines[8].contains("stash touches files you changed"),
        "{lines:?}"
    );
    assert!(lines[9].contains("apply"), "{lines:?}");
}

/// Review 86, M2: a box over the list holds every key, so the list's
/// own bar is not drawn under it — the box's footer is the bar, as on
/// the graph (`HintContext::Preview`).
#[test]
fn a_box_over_the_list_leaves_the_bar_row_blank() {
    let mut screen = view(
        vec![row("stash@{0}", "\"a\"", "2h ago", 1)],
        StashStatus::Ready,
    );
    screen.box_open = true;
    let lines = grid(&screen, 80, 10);
    assert_eq!(lines[9], "", "{lines:?}");
    assert!(
        lines.iter().all(|line| !line.contains("p pop")),
        "{lines:?}"
    );
}

#[test]
fn it_draws_nothing_outside_a_tiny_area() {
    for height in [0_u16, 1, 2, 3] {
        let view = view(
            vec![row("stash@{0}", "\"a\"", "2h ago", 1)],
            StashStatus::Ready,
        );
        let lines = grid(&view, 40, height.max(1));
        assert_eq!(lines.len(), usize::from(height.max(1)));
    }
}

/// Spell sweep (c), ticket m8/10c: the list's footer and empty state
/// name his chords; the save prompt, a text field, keeps the shipped
/// ones whatever he set.
#[test]
fn the_stash_list_spells_his_keys_and_its_prompt_the_shipped_ones() {
    use crate::keymap::sweep::{assert_shipped, assert_spelled, swept};
    let (keys, _) = swept();
    let theme = theme();
    let ready = view(vec![row("stash@{0}", "wip", "2m", 1)], StashStatus::Ready);
    let footer = |view: &StashView<'_>| {
        StashScreen {
            keys: &keys,
            view,
            theme: &theme,
        }
        .footer(400)
    };
    assert_spelled(
        &footer(&ready),
        &keys,
        &[
            &["stash.apply"],
            &["stash.pop"],
            &["stash.save"],
            &["stash.drop"],
            &["stash.preview"],
            &["app.back"],
        ],
    );
    let mut empty_list = view(Vec::new(), StashStatus::Empty);
    assert_spelled(
        &footer(&empty_list),
        &keys,
        &[&["stash.save"], &["app.back"]],
    );
    assert_spelled(&empty(&keys), &keys, &[&["stash.save"]]);
    empty_list.prompt = Some(StashPromptView {
        title: sanitize("Stash the working tree"),
        message: FieldView {
            label: "message",
            text: sanitize("wip"),
            cursor: Some(3),
            guide_at: None,
            counter: None,
            focused: true,
            secret: None,
        },
        staged_only: true,
        refusal: None,
    });
    assert_shipped(
        &footer(&empty_list),
        &[&["app.open"], &["app.next_pane"], &["app.back"]],
    );
}

/// The bottom row of the stash list at `width` under `keys`.
fn footer_row(view: &StashView<'_>, width: u16, keys: &Keymap) -> String {
    let theme = theme();
    let mut terminal = Terminal::new(TestBackend::new(width, 12)).expect("a test backend opens");
    terminal
        .draw(|frame| {
            frame.render_widget(
                StashScreen {
                    keys,
                    view,
                    theme: &theme,
                },
                frame.area(),
            );
        })
        .expect("drawing into a test backend");
    let buffer = terminal.backend().buffer();
    (0..width)
        .map(|x| buffer[(x, 11)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// Every status the list draws its own keys in (m8/11e E3), with the
/// full clauses each draws under the shipped keymap.
fn footer_cases<'a>() -> Vec<(&'static str, StashView<'a>, Vec<&'static str>)> {
    let rows = || vec![row("stash@{0}", "WIP on main: tidy", "2h", 3)];
    let short = vec!["s save", "esc close"];
    vec![
        (
            "rows",
            view(rows(), StashStatus::Ready),
            vec![
                "⏎ apply",
                "p pop",
                "s save",
                "d drop",
                "b branch",
                "space files",
                "esc close",
            ],
        ),
        (
            "loading",
            view(Vec::new(), StashStatus::Loading),
            short.clone(),
        ),
        ("empty", view(Vec::new(), StashStatus::Empty), short.clone()),
        (
            "failed",
            view(Vec::new(), StashStatus::Failed(sanitize("no stash"))),
            short,
        ),
    ]
}

/// m8/11e F2: the list's own footer is the bar he sees; it ends with
/// the doorway, in every status that draws its own keys.
#[test]
fn footer_ends_with_the_keys_doorway() {
    for (what, view, _) in &footer_cases() {
        crate::test_support::assert_footer_doorway(what, &[], &|width, keys| {
            footer_row(view, width, keys)
        });
    }
}

/// m8/11e E1: at every width the footer draws whole clauses or none —
/// never a cut one — and the doorway whole, bare, or not at all.
#[test]
fn footer_never_draws_a_fragment() {
    for (what, view, clauses) in &footer_cases() {
        crate::test_support::assert_footer_whole(what, clauses, &[], &|width, keys| {
            footer_row(view, width, keys)
        });
    }
}

/// m8/11e G1: `b` makes a branch from the stash under the cursor, so it
/// is drawn only when one is selected (Ready), between `drop` and
/// `files` (§2.9's order); never at Loading, Empty or Failed.
#[test]
fn the_branch_key_is_drawn_only_with_a_stash_selected() {
    let keys = Keymap::shipped();
    for (what, view, _) in &footer_cases() {
        for width in [120, 200] {
            let row = footer_row(view, width, keys);
            if *what == "rows" {
                let drop = row.find("d drop").expect("drop is drawn");
                let branch = row
                    .find("b branch")
                    .unwrap_or_else(|| panic!("{what} at {width}: no `b branch` in `{row}`"));
                let files = row.find("space files").expect("files is drawn");
                assert!(
                    drop < branch && branch < files,
                    "{what} at {width}: `b branch` not between drop and files: `{row}`"
                );
            } else {
                assert!(
                    !row.contains("b branch"),
                    "{what} at {width}: nothing is selected, yet `{row}`"
                );
            }
        }
    }
}
