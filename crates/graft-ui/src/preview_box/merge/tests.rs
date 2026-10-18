//! The merge box (ease-pass 10): its lines, its footer, and that it draws
//! no buttons.

use graft_term::sanitize_line;
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
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
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
