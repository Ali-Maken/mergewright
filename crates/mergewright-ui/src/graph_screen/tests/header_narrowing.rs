//! The header's tail as the row narrows, m8/15b, and the long path at 80
//! columns, m8/17 (moved out of `graph_screen/tests.rs`, ease-pass/12).

use super::*;

/// m8-polish-and-ship/17 item 4: at 80 columns a 120-character path is cut
/// from the left and the order notice still shows whole, right-aligned.
#[test]
fn a_long_path_never_cuts_the_order_notice_at_80_columns() {
    let rows = linear(2);
    let path = format!("~/{}/api", "deeply-nested-folder/".repeat(6));
    assert!(path.chars().count() >= 120, "{}", path.chars().count());
    for (notice, words) in [
        (OrderNotice::Sorting, "date order, sorting\u{2026}"),
        (OrderNotice::Provisional, "order stays provisional (date)"),
    ] {
        let mut shown = view(&rows, WalkView::Complete { rows: 2 }, 0);
        shown.location = Some(at(&path));
        shown.order = Some(notice);
        let header = text(&draw(&shown, 80, 6))[0].clone();
        assert!(header.trim_end().ends_with(words), "{header:?}");
        assert!(
            header.contains(" api "),
            "the repo name survives: {header:?}"
        );
        assert!(header.contains('\u{2026}'), "the path is cut: {header:?}");
        assert_eq!(header.width(), 80);
    }
}

/// m8/15b (15 A2): the header's tail drops whole items as the row narrows —
/// `gfx` first, then the walk, then the filters, then the search — and
/// draws each it keeps whole. With an operation live the banner outlasts
/// the search.
#[test]
fn header_narrowing_drops_gfx_walk_filters_search_banner_in_that_order() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 0);
    screen.graphics = Some(GraphicsStatus::Ok);
    screen.filters = Filters {
        first_parent: true,
        ..Filters::default()
    };
    let search = "/root  2 matches (1/2)  n/N next/prev  esc close";
    screen.search_tail = Some(super::SearchTail {
        text: sanitize_line(search),
        failed: false,
    });
    // Drop order, first to go first; each item's whole words.
    let items = [
        ("gfx", "gfx ok"),
        ("walk", "3 commits"),
        ("filters", "first-parent"),
        ("search", search),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for columns in (8u16..=140).rev() {
        let header = text(&draw(&screen, columns, 4))[0].clone();
        let kept: Vec<bool> = items
            .iter()
            .map(|(_, words)| header.contains(words))
            .collect();
        // Whole or not at all: no item's start is drawn without its end.
        for (name, words) in &items {
            let start: String = words.chars().take(4).collect();
            assert!(
                header.contains(words) || !header.contains(&start),
                "{columns}: {name} cut: {header:?}"
            );
        }
        // A later item in the drop order is never gone while an earlier
        // one is still drawn.
        for later in 1..items.len() {
            assert!(
                kept[later] || !kept[..later].iter().any(|kept| *kept),
                "{columns}: {} dropped before {:?}: {header:?}",
                items[later].0,
                items[..later]
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
            );
        }
        seen.insert(kept.iter().filter(|kept| **kept).count());
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4],
        "every step of the narrowing was reached"
    );
}

/// m8/15b (15 A2), the last step of the drop order: with an operation live
/// the banner outlasts the search.
#[test]
fn header_narrowing_drops_the_search_before_the_banner() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 0);
    let search = "/root  2 matches (1/2)  n/N next/prev  esc close";
    screen.search_tail = Some(super::SearchTail {
        text: sanitize_line(search),
        failed: false,
    });
    screen.banner_op = Some(rebase_banner(Some((3, 7)), None));
    let wide = text(&draw(&screen, 160, 4))[0].clone();
    assert!(
        wide.contains(search) && wide.contains("REBASING"),
        "{wide:?}"
    );
    let mut search_gone_at = None;
    for columns in (30u16..=160).rev() {
        let header = text(&draw(&screen, columns, 4))[0].clone();
        assert!(
            header.contains("REBAS"),
            "{columns}: banner kept: {header:?}"
        );
        if !header.contains(search) {
            search_gone_at.get_or_insert(columns);
            assert!(!header.contains("/root"), "{columns}: {header:?}");
        }
    }
    assert!(
        search_gone_at.is_some(),
        "the search gave way to the banner"
    );
}
