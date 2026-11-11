//! The row's columns as the main mock draws them (ease-pass 07, G5–G8):
//! the branch column's widths and cell, the labels after the message, who
//! and age — and that a drag's source cells are the ones drawn.

use mergewright_graph::LaneRole;

use super::*;

/// The row drawn at screen row 1 of a `width`-wide screen: its text, and
/// the columns it was drawn to.
fn first_row(screen: &GraphView<'_>, width: u16) -> (String, Columns) {
    let terminal = draw(screen, width, 8);
    let line = text(&terminal)[1].clone();
    let area = Rect::new(0, 0, width, 8);
    let (_, columns) = row_at(screen, area, 1).expect("a commit row at y = 1");
    (line, columns)
}

fn cell(name: &str, role: LaneRole, you: bool) -> BranchCell {
    BranchCell {
        name: sanitize_line(name),
        role,
        you,
        dim: false,
    }
}

/// The cells `[from, from + width)` of `line`, as text.
fn cells_of(line: &str, from: u16, width: u16) -> String {
    line.chars()
        .skip(usize::from(from))
        .take(usize::from(width))
        .collect()
}

#[test]
fn the_branch_column_is_20_14_12_wide_at_120_110_90() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(cell(
        "feature/a-very-long-branch-name",
        LaneRole::This,
        false,
    ));
    for (width, expected) in [(120_u16, 20_u16), (110, 14), (90, 12)] {
        let (line, columns) = first_row(&screen, width);
        assert_eq!(columns.branch_width, expected, "{width} columns: {line:?}");
        let drawn = cells_of(&line, columns.branch_x, columns.branch_width);
        assert!(
            drawn.starts_with("feature/") && drawn.ends_with('…'),
            "{width} columns: the name is cut to the column: {drawn:?}"
        );
        // The message starts one gap after the column.
        assert_eq!(columns.subject_x, columns.branch_x + expected + 1);
        assert!(
            cells_of(&line, columns.subject_x, 8) == "Commit 3",
            "{width}: {line:?}"
        );
    }
}

#[test]
fn heads_branch_reads_you_and_takes_its_roles_colour_bold_unless_other() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(cell("wallet-release", LaneRole::This, true));
    screen.rows[1].branch = Some(cell("ops/ci", LaneRole::Other, false));
    let terminal = draw(&screen, 120, 8);
    let buffer = terminal.backend().buffer();
    let drawn = text(&terminal);
    let area = Rect::new(0, 0, 120, 8);
    let (_, columns) = row_at(&screen, area, 1).expect("a commit row");
    let x = columns.branch_x;
    assert!(
        cells_of(&drawn[1], x, 20).starts_with("wallet-release (you)"),
        "{:?}",
        drawn[1]
    );
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps());
    assert_eq!(buffer[(x, 1)].fg, color(theme.token(Token::RoleThis)));
    assert!(buffer[(x, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(x, 2)].fg, color(theme.token(Token::RoleOther)));
    assert!(!buffer[(x, 2)].modifier.contains(Modifier::BOLD));
}

/// A name too long for the column is cut, never ` (you)`: whose branch it
/// is matters more than the name's tail.
#[test]
fn a_long_heads_branch_keeps_you_and_cuts_its_name() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(cell("feature/receipts-list", LaneRole::This, true));
    let terminal = draw(&screen, 120, 8);
    let drawn = text(&terminal);
    let (_, columns) = row_at(&screen, Rect::new(0, 0, 120, 8), 1).expect("a commit row");
    assert_eq!(
        cells_of(&drawn[1], columns.branch_x, 20),
        "feature/recei\u{2026} (you)"
    );
    // A short name is not padded out to the cut.
    screen.rows[0].branch = Some(cell("main", LaneRole::This, true));
    let drawn = text(&draw(&screen, 120, 8));
    assert!(
        cells_of(&drawn[1], columns.branch_x, 20).starts_with("main (you) "),
        "{:?}",
        drawn[1]
    );
}

fn terminal_caps() -> mergewright_core::capabilities::Capabilities {
    terminal(ColorDepth::TrueColor, false)
}

#[test]
fn an_opened_merges_row_names_its_branch_dim() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(BranchCell {
        dim: true,
        ..cell("fix-toc", LaneRole::Other, false)
    });
    let terminal = draw(&screen, 120, 8);
    let area = Rect::new(0, 0, 120, 8);
    let (_, columns) = row_at(&screen, area, 1).expect("a commit row");
    let cell = &terminal.backend().buffer()[(columns.branch_x, 1)];
    assert_eq!(cell.symbol(), "f");
    assert!(cell.modifier.contains(Modifier::DIM));
}

fn tracked(name: &str, ahead: usize) -> Badge {
    branch(
        name,
        false,
        if ahead == 0 {
            Tracking::InSync {
                remote: sanitize_line("origin"),
            }
        } else {
            Tracking::Apart { ahead, behind: 0 }
        },
    )
}

#[test]
fn labels_follow_the_message_muted_and_say_where_each_branch_is() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    // Row 0: HEAD's branch is the column's own name, 1 ahead; a second local
    // branch 2 ahead; a tag. Row 1: an in-sync branch, a remote on its own,
    // and a branch with no upstream.
    screen.rows[0].branch = Some(cell("main", LaneRole::This, true));
    screen.rows[0].badges = vec![
        Badge {
            name: sanitize_line("v1.2"),
            kind: BadgeKind::Tag,
        },
        tracked("wallet-release", 2),
        tracked("main", 1),
    ];
    screen.rows[1].badges = vec![
        tracked("in-sync", 0),
        Badge {
            name: sanitize_line("origin/x"),
            kind: BadgeKind::Remote {
                identity: BranchIdentity::of_name(b"origin/x"),
            },
        },
        branch("local-only", false, Tracking::Untracked),
    ];
    let drawn = text(&draw(&screen, 160, 8));
    assert!(
        drawn[1].contains("Commit 3  ↑1 to push  tag v1.2  wallet-release is here  ↑2 to push"),
        "{:?}",
        drawn[1]
    );
    assert!(
        drawn[2].contains(
            "Commit 2  in-sync is here  origin/x is here  local-only is here  no upstream set"
        ),
        "{:?}",
        drawn[2]
    );
    for line in &drawn[1..3] {
        assert!(!line.contains('['), "no badge brackets: {line:?}");
        assert!(!line.contains("in-sync is here  ↑"), "{line:?}");
    }
}

#[test]
fn a_branch_level_with_its_upstream_in_the_column_says_nothing() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(cell("main", LaneRole::This, true));
    screen.rows[0].badges = vec![tracked("main", 0)];
    let drawn = text(&draw(&screen, 120, 8));
    assert!(!drawn[1].contains("is here"), "{:?}", drawn[1]);
    assert!(!drawn[1].contains("to push"), "{:?}", drawn[1]);
}

#[test]
fn who_and_age_are_right_aligned_and_the_users_own_rows_read_you_bold() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].who = Some(WhoCell {
        name: sanitize_line("you"),
        own: true,
    });
    screen.rows[1].who = Some(WhoCell {
        name: sanitize_line("Sara"),
        own: false,
    });
    screen.rows[1].when = "10mo".to_owned();
    let terminal = draw(&screen, 120, 8);
    let drawn = text(&terminal);
    let area = Rect::new(0, 0, 120, 8);
    let (_, columns) = row_at(&screen, area, 1).expect("a commit row");
    assert_eq!(cells_of(&drawn[1], columns.who_x, 6), "   you");
    assert_eq!(cells_of(&drawn[1], columns.when_x, 4), "  2h");
    assert_eq!(cells_of(&drawn[2], columns.who_x, 6), "  Sara");
    assert_eq!(cells_of(&drawn[2], columns.when_x, 4), "10mo");
    let buffer = terminal.backend().buffer();
    assert!(
        buffer[(columns.who_x + 3, 1)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        !buffer[(columns.who_x + 2, 2)]
            .modifier
            .contains(Modifier::BOLD)
    );
    // Nothing on the row is a SHA any more.
    for line in &drawn[1..4] {
        assert!(
            !line
                .split_whitespace()
                .any(|word| word.len() == 7 && word.chars().all(|c| c.is_ascii_hexdigit())),
            "{line:?}"
        );
    }
}

#[test]
fn a_drag_on_the_branch_columns_name_picks_that_branch() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(cell("main", LaneRole::This, true));
    screen.rows[0].badges = vec![tracked("other", 0), tracked("main", 0)];
    let area = Rect::new(0, 0, 120, 8);
    let (entry, columns) = row_at(&screen, area, 1).expect("a commit row");
    let spans = ref_spans(entry, &columns, Icons::Plain, Keymap::shipped(), false);
    let on_name = spans
        .iter()
        .find(|(span, _)| span.contains(&columns.branch_x))
        .expect("the branch column's name is a drag source");
    assert_eq!(on_name.1, 1, "it names `main`'s badge");
    let drawn = text(&draw(&screen, 120, 8));
    let label = spans
        .iter()
        .find(|(_, index)| *index == 0)
        .expect("`other is here` is a drag source");
    assert_eq!(
        cells_of(&drawn[1], label.0.start, label.0.end - label.0.start),
        "other is here"
    );
}

#[test]
fn colours_off_keeps_the_branch_column_bold() {
    let rows = linear(3);
    let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 2);
    screen.rows[0].branch = Some(cell("main", LaneRole::This, true));
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps()).colourless();
    let mut terminal = Terminal::new(TestBackend::new(120, 8)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(&screen, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    let area = Rect::new(0, 0, 120, 8);
    let (_, columns) = row_at(&screen, area, 1).expect("a commit row");
    let cell = &terminal.backend().buffer()[(columns.branch_x, 1)];
    assert!(cell.modifier.contains(Modifier::BOLD));
}
