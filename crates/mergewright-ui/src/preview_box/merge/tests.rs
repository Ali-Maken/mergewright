//! The merge box (ease-pass 10): its lines, its footer, and that it draws
//! no buttons.

use mergewright_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::MergeParts;
use crate::keymap::Keymap;
use crate::preview_box::{PreviewBox, PreviewBoxWidget, preview_rows};
use crate::test_support::terminal as test_capabilities;
use crate::theme::{Theme, ThemeKind};

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(mergewright_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn clashing() -> MergeParts {
    MergeParts {
        from: sanitize_line("sara/fix-toc"),
        into: sanitize_line("wallet-release"),
        incoming: 3,
        incoming_capped: false,
        who: vec![sanitize_line("Sara"), sanitize_line("Omar")],
        who_more: 0,
        changed: 4,
        changed_capped: false,
        decide: vec![(sanitize_line("report.md"), Some(2))],
        not_checkable: None,
        withheld: None,
        edits: Vec::new(),
        fast_forward: false,
    }
}

fn drawn(parts: MergeParts, width: u16) -> Vec<String> {
    let view =
        PreviewBox::new(sanitize_line("Merge sara/fix-toc into wallet-release")).with_merge(parts);
    let height = preview_rows(&view, width);
    let frame = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(frame);
    (&PreviewBoxWidget {
        view: &view,
        theme: &theme(),
        keys: Keymap::shipped(),
    })
        .render(frame, &mut buf);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

fn has(rows: &[String], text: &str) -> bool {
    rows.iter().any(|row| row.contains(text))
}

#[test]
fn a_clashing_merge_says_what_comes_in_what_git_combines_and_what_he_decides() {
    let rows = drawn(clashing(), 100);
    assert!(rows[0].contains("Merge sara/fix-toc into wallet-release"));
    for line in [
        "│ Nothing has changed yet.",
        "│ Comes in      3 commits from sara/fix-toc (Sara, Omar)",
        "│ Changes       4 files",
        "│ Git combines  3 files on its own",
        "│ You decide    1 file clashes: report.md (2 places)",
        "│ After y you go straight to the first clash.",
    ] {
        assert!(has(&rows, line), "{line:?} in {rows:#?}");
    }
    let footer = rows.last().expect("a footer");
    assert!(
        footer.contains("╰ y merge  esc cancel  ⏎ see the 3 commits ─"),
        "{footer}"
    );
    assert!(!has(&rows, "Cancel"), "no buttons: {rows:#?}");
    assert!(!has(&rows, "m merge"), "no opening-key alias");
}

#[test]
fn a_clean_merge_says_nothing_to_decide_and_that_it_commits() {
    let parts = MergeParts {
        decide: Vec::new(),
        ..clashing()
    };
    let rows = drawn(parts, 100);
    assert!(has(&rows, "│ You decide    nothing: no file clashes"));
    assert!(has(&rows, "│ After y the merge is committed."));
    assert!(has(&rows, "│ Git combines  4 files on its own"));
}

/// His edits set aside around the merge (ADR-0045 §4) keep their lines,
/// under `You decide` and above `After y`; a not-checked forecast keeps
/// them too.
#[test]
fn his_set_aside_edits_are_said_under_the_counts() {
    let edits = vec![
        sanitize_line("your edits: 1 file set aside, put back after"),
        sanitize_line("your edits may clash when put back: notes.md"),
    ];
    let rows = drawn(
        MergeParts {
            edits: edits.clone(),
            ..clashing()
        },
        100,
    );
    let at = |text: &str| rows.iter().position(|row| row.contains(text));
    let decide = at("│ You decide").expect("You decide");
    let aside = at("│ your edits: 1 file set aside, put back after").expect("set aside");
    let clash = at("│ your edits may clash when put back: notes.md").expect("put back");
    let after = at("│ After y").expect("After y");
    assert!(
        decide < aside && aside < clash && clash < after,
        "{rows:#?}"
    );
    let rows = drawn(
        MergeParts {
            edits,
            not_checkable: Some(sanitize_line("conflicts: not checked (old git)")),
            ..clashing()
        },
        100,
    );
    assert!(has(&rows, "│ your edits: 1 file set aside"), "{rows:#?}");
}

#[test]
fn counts_are_singular_capped_and_more_authors_are_counted() {
    let parts = MergeParts {
        incoming: 1,
        who: vec![sanitize_line("Sara")],
        who_more: 2,
        changed: 50,
        changed_capped: true,
        decide: vec![
            (sanitize_line("a.rs"), Some(1)),
            (sanitize_line("b.bin"), None),
        ],
        ..clashing()
    };
    let rows = drawn(parts, 120);
    assert!(has(
        &rows,
        "Comes in      1 commit from sara/fix-toc (Sara, +2)"
    ));
    assert!(has(&rows, "Changes       50+ files"));
    assert!(has(&rows, "Git combines  48+ files on its own"));
    assert!(has(
        &rows,
        "You decide    2 files clash: a.rs (1 place), b.bin"
    ));
    assert!(has(&rows, "⏎ see the 1 commit"));
}

#[test]
fn unrelated_withholds_y_and_says_why_in_one_line() {
    let parts = MergeParts {
        withheld: Some(sanitize_line(
            "Can't merge: sara/fix-toc and wallet-release share no history.",
        )),
        ..clashing()
    };
    let rows = drawn(parts, 100);
    assert!(has(
        &rows,
        "│ Can't merge: sara/fix-toc and wallet-release share no history."
    ));
    assert!(has(&rows, "Comes in      3 commits"));
    for gone in ["Changes", "You decide", "After y", "y merge"] {
        assert!(!has(&rows, gone), "{gone:?} gone: {rows:#?}");
    }
    let footer = rows.last().expect("a footer");
    assert!(
        footer.contains("╰ esc cancel  ⏎ see the 3 commits ─"),
        "{footer}"
    );
}

#[test]
fn not_checkable_keeps_y_and_says_not_checked() {
    let parts = MergeParts {
        not_checkable: Some(sanitize_line("conflicts: not checked (git 2.38+ needed)")),
        ..clashing()
    };
    let rows = drawn(parts, 100);
    assert!(has(&rows, "│ conflicts: not checked (git 2.38+ needed)"));
    assert!(!has(&rows, "You decide"));
    assert!(has(&rows, "y merge  esc cancel"));
}

#[test]
fn a_narrow_footer_drops_the_list_key_first() {
    let rows = drawn(clashing(), 40);
    let footer = rows.last().expect("a footer");
    assert!(footer.contains("y merge  esc cancel"), "{footer}");
    assert!(!footer.contains("see the"), "{footer}");
}

/// The box's text rows between the borders, each cut of its frame and
/// trailing blanks: what he reads, top to bottom.
fn texts(rows: &[String]) -> Vec<String> {
    rows.iter()
        .map(|row| {
            row.trim_start_matches(['│', '╭', '─', ' '])
                .trim_end_matches(['│', '╮', '─', ' '])
                .to_owned()
        })
        .collect()
}

fn fast_forward() -> MergeParts {
    MergeParts {
        decide: Vec::new(),
        changed: 5,
        who: vec![sanitize_line("Sara")],
        fast_forward: true,
        ..clashing()
    }
}

#[test]
fn a_fast_forward_says_nothing_is_combined_and_into_moves_up() {
    let rows = drawn(fast_forward(), 100);
    let texts = texts(&rows);
    assert_eq!(
        texts[..texts.len() - 1],
        [
            "Merge sara/fix-toc into wallet-release",
            "Nothing has changed yet.",
            "",
            "Comes in      3 commits from sara/fix-toc (Sara)",
            "Changes       5 files",
            "Git combines  nothing: no merge commit is made",
            "You decide    nothing: no file clashes",
            "",
            "After y wallet-release moves up to sara/fix-toc.",
        ],
        "{rows:#?}"
    );
    let footer = rows.last().expect("a footer");
    assert!(
        footer.contains("╰ y merge  esc cancel  ⏎ see the 3 commits ─"),
        "{footer}"
    );
}

/// A fast-forward cannot clash, so its words win over `not checked`;
/// with no check there is no count of files to give, so `Changes` goes.
#[test]
fn a_fast_forward_with_no_check_still_says_it_only_moves_up() {
    let parts = MergeParts {
        changed: 0,
        not_checkable: Some(sanitize_line("conflicts: not checked (git 2.38+ needed)")),
        ..fast_forward()
    };
    let rows = drawn(parts, 100);
    for line in [
        "│ Git combines  nothing: no merge commit is made",
        "│ You decide    nothing: no file clashes",
        "│ After y wallet-release moves up to sara/fix-toc.",
    ] {
        assert!(has(&rows, line), "{line:?} in {rows:#?}");
    }
    for gone in ["not checked", "Changes"] {
        assert!(!has(&rows, gone), "{gone:?} gone: {rows:#?}");
    }
    assert!(has(&rows, "y merge  esc cancel"));
}
