use graft_core::capabilities::ColorDepth;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::layout::layout;
use graft_graph::row::Row;
use graft_graph::seeds::Seeds;
use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::{terminal, wide_history};
use crate::theme::ThemeKind;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

/// A linear history of `count` commits, newest first.
fn linear(count: u8) -> Vec<Row> {
    let commits: Vec<Commit> = (1..=count)
        .rev()
        .map(|id| {
            let who = Identity {
                name: b"Test".to_vec(),
                email: b"test@example.invalid".to_vec(),
                when: Timestamp {
                    seconds: i64::from(id),
                    offset_seconds: 0,
                },
            };
            Commit {
                id: oid(id),
                parents: if id == 1 { vec![] } else { vec![oid(id - 1)] },
                author: who.clone(),
                committer: who,
                message: format!("Commit {id}").into_bytes(),
            }
        })
        .collect();
    layout(&commits, Seeds::default())
}

fn view(rows: &[Row], walk: WalkView, cursor: usize) -> GraphView<'_> {
    GraphView {
        location: Some(sanitize_line("/code/api")),
        walk,
        problem: None,
        rows: rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mut line = GraphLine::plain(row);
                if index == cursor {
                    line.paint = RowPaint::Highlighted;
                }
                GraphEntry {
                    badges: Vec::new(),
                    line,
                    subject: sanitize_line(&String::from_utf8_lossy(row.commit.subject())),
                    author: AuthorChip {
                        initials: sanitize_line(&initials(&String::from_utf8_lossy(
                            &row.commit.author.name,
                        ))),
                        identity: BranchIdentity::of_name(&row.commit.author.email),
                    },
                    when: "2h".to_owned(),
                    sha: short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH)),
                }
            })
            .collect(),
        icons: Icons::Plain,
        compaction: Compaction::at(8),
        filters: Filters::default(),
    }
}

fn draw(view: &GraphView<'_>, columns: u16, rows: u16) -> Terminal<TestBackend> {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(view, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    terminal
}

fn text(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn rows_sit_between_the_header_and_the_key_hints() {
    let rows = linear(3);
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 3 }, 0),
        40,
        6,
    ));

    assert!(screen[0].contains("/code/api"), "{screen:#?}");
    assert!(screen[0].contains("3 commits"), "{screen:#?}");
    assert!(screen[1].contains("Commit 3"), "{screen:#?}");
    assert!(screen[3].contains("Commit 1"), "{screen:#?}");
    assert!(screen[5].contains("q quit"), "{screen:#?}");
    assert_eq!(usize::from(CHROME_ROWS), 2);
}

#[test]
fn the_cursor_row_is_raised_and_the_others_are_not() {
    let rows = linear(3);
    let terminal = draw(&view(&rows, WalkView::Complete { rows: 3 }, 1), 40, 6);
    let buffer = terminal.backend().buffer();
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps());
    let raised = color(theme.token(Token::SurfaceRaised));

    assert_eq!(buffer[(30, 2)].bg, raised);
    assert_ne!(buffer[(30, 1)].bg, raised);
    assert!(buffer[(10, 2)].modifier.contains(Modifier::BOLD));
}

fn terminal_caps() -> graft_core::capabilities::Capabilities {
    terminal(ColorDepth::TrueColor, false)
}

#[test]
fn at_palette16_the_cursor_row_gets_a_rule_instead_of_a_background() {
    let rows = linear(3);
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::Palette16, false));
    let mut terminal = Terminal::new(TestBackend::new(40, 6)).expect("test backend opens");
    let view = view(&rows, WalkView::Complete { rows: 3 }, 1);
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(&view, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();

    // No usable background step at this depth: the row's fill is left
    // untouched, same as an unhighlighted row, not `surface-raised`.
    assert_eq!(buffer[(30, 2)].bg, buffer[(30, 1)].bg, "{buffer:#?}");
    // The marker slot carries the highlight instead, in the accent colour.
    let accent = color(theme.token(Token::Accent));
    let has_rule = (0..buffer.area.width)
        .any(|x| buffer[(x, 2)].symbol() == "▏" && buffer[(x, 2)].fg == accent);
    assert!(
        has_rule,
        "expected a Palette16 rule on the cursor row: {buffer:#?}"
    );
    assert!(!buffer[(10, 2)].modifier.is_empty()); // still bold
}

#[test]
fn truecolor_snapshot_is_unchanged_by_the_palette16_rule() {
    let rows = linear(3);
    let terminal = draw(&view(&rows, WalkView::Complete { rows: 3 }, 1), 40, 6);
    let buffer = terminal.backend().buffer();
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps());
    let raised = color(theme.token(Token::SurfaceRaised));

    assert_eq!(buffer[(30, 2)].bg, raised);
    assert!(!(0..buffer.area.width).any(|x| buffer[(x, 2)].symbol() == "▏"));
}

#[test]
fn a_jump_in_progress_says_how_far_it_has_got_and_how_to_back_out() {
    let rows = linear(2);
    let screen = text(&draw(
        &view(&rows, WalkView::Jumping { laid_out: 4500 }, 0),
        60,
        5,
    ));

    assert!(screen[0].contains("laying out… 4500 rows"), "{screen:#?}");
    assert!(screen[0].contains("Esc cancels"), "{screen:#?}");
}

#[test]
fn a_walk_that_is_fine_never_reads_as_an_error() {
    for walk in [
        WalkView::Opening,
        WalkView::Reading { laid_out: 0 },
        WalkView::Reading { laid_out: 1000 },
        WalkView::Complete { rows: 0 },
        WalkView::Cancelled { laid_out: 500 },
    ] {
        let (words, token) = walk_text(&walk);
        assert!(
            !words.contains("fail") && !words.contains("error"),
            "{words}"
        );
        assert_ne!(token, Token::Warning, "{walk:?}");
    }
    let (words, token) = walk_text(&WalkView::Failed(sanitize_line("bad object")));
    assert!(words.contains("bad object"));
    assert_eq!(token, Token::Warning);
}

#[test]
fn a_problem_takes_the_footers_place() {
    let rows = linear(2);
    let mut view = view(&rows, WalkView::Complete { rows: 2 }, 0);
    view.problem = Some(sanitize_line("job 3 failed: not a git repository"));
    let screen = text(&draw(&view, 60, 5));

    assert!(screen[4].contains("not a git repository"), "{screen:#?}");
    assert!(!screen[4].contains("q quit"), "{screen:#?}");
}

#[test]
fn more_rows_than_fit_are_cut_at_the_footer() {
    let rows = linear(10);
    let screen = text(&draw(
        &view(&rows, WalkView::Reading { laid_out: 10 }, 0),
        40,
        5,
    ));

    assert!(screen[3].contains("Commit 8"), "{screen:#?}");
    assert!(screen[4].contains("q quit"), "{screen:#?}");
}

#[test]
fn the_marker_slot_is_the_same_width_empty_or_filled() {
    let rows = linear(2);
    let empty = view(&rows, WalkView::Complete { rows: 2 }, 0);
    let empty_screen = text(&draw(&empty, 120, 5));

    let mut marked = rows.clone();
    marked[0].marker = Some(graft_core::mistake_radar::MistakeRadarKind::EvilMerge);
    let filled = view(&marked, WalkView::Complete { rows: 2 }, 0);
    let filled_screen = text(&draw(&filled, 120, 5));

    // Whether or not row 0 carries a marker, the subject starts at the same
    // column: the marker glyph and its neighbour's blank both take
    // `MARKER_WIDTH` cells, so later columns never shift.
    let subject_at = |line: &str| line.find("Commit").expect("the subject is drawn");
    assert_eq!(subject_at(&empty_screen[1]), subject_at(&filled_screen[1]));
    assert_ne!(empty_screen[1], filled_screen[1]);
}

#[test]
fn the_author_chip_drops_below_the_breakpoint_and_returns_above_it() {
    let rows = linear(2);
    let view = view(&rows, WalkView::Complete { rows: 2 }, 0);

    let narrow = text(&draw(&view, AUTHOR_BREAKPOINT - 1, 5));
    let wide = text(&draw(&view, AUTHOR_BREAKPOINT, 5));

    assert!(!narrow[1].contains("TE"), "{narrow:#?}");
    assert!(wide[1].contains("TE"), "{wide:#?}");
    // The date and SHA survive at both widths — only the author chip drops.
    assert!(narrow[1].contains("2h"), "{narrow:#?}");
    assert!(wide[1].contains("2h"), "{wide:#?}");
}

#[test]
fn tiny_and_empty_terminals_draw_without_panicking() {
    let rows = linear(3);
    let view = view(&rows, WalkView::Complete { rows: 3 }, 0);
    for (columns, height) in [(0, 0), (1, 1), (2, 2), (80, 1), (3, 24)] {
        let terminal = draw(&view, columns, height);
        assert_eq!(terminal.backend().buffer().area.height, height);
    }
}

fn branch(name: &str, current: bool, tracking: Tracking) -> Badge {
    Badge {
        name: sanitize_line(name),
        kind: BadgeKind::Branch {
            identity: BranchIdentity::of_name(name.as_bytes()),
            current,
            tracking,
        },
    }
}

/// One of each kind of badge, the way `app` hands them over.
fn every_kind() -> Vec<Badge> {
    vec![
        branch(
            "main",
            true,
            Tracking::InSync {
                remote: sanitize_line("origin"),
            },
        ),
        branch(
            "feature/auth",
            false,
            Tracking::Apart {
                ahead: 2,
                behind: 1,
            },
        ),
        Badge {
            name: sanitize_line("origin/feature/auth"),
            kind: BadgeKind::Remote {
                identity: BranchIdentity::of_name(b"origin/feature/auth"),
            },
        },
        Badge {
            name: sanitize_line("v2.3.0"),
            kind: BadgeKind::Tag,
        },
        Badge {
            name: sanitize_line("HEAD"),
            kind: BadgeKind::Head,
        },
        Badge {
            name: sanitize_line("stash"),
            kind: BadgeKind::Stash,
        },
    ]
}

#[test]
fn badges_read_as_designed_in_plain_unicode() {
    let texts: Vec<String> = every_kind()
        .iter()
        .map(|badge| badge.text(Icons::Plain))
        .collect();
    assert_eq!(
        texts,
        [
            "[◆ main ⇄ origin]",
            "[feature/auth ↑2 ↓1]",
            "[origin/feature/auth]",
            "[tag: v2.3.0]",
            "[HEAD]",
            "[stash]",
        ]
    );
    let ahead_only = branch(
        "a",
        false,
        Tracking::Apart {
            ahead: 3,
            behind: 0,
        },
    );
    let behind_only = branch(
        "b",
        false,
        Tracking::Apart {
            ahead: 0,
            behind: 4,
        },
    );
    assert_eq!(ahead_only.text(Icons::Plain), "[a ↑3]");
    assert_eq!(behind_only.text(Icons::Plain), "[b ↓4]");
}

/// Rows 0 and 1 of a two-commit history, with every kind of badge on row 0.
fn badged(rows: &[Row], icons: Icons) -> GraphView<'_> {
    let mut view = view(rows, WalkView::Complete { rows: 2 }, 1);
    view.rows[0].badges = every_kind();
    view.rows[1].badges = vec![branch("topic", false, Tracking::Untracked)];
    view.icons = icons;
    view
}

#[test]
fn badges_are_cut_before_the_subject_disappears_at_80_columns() {
    let rows = linear(2);
    let screen = text(&draw(&badged(&rows, Icons::Plain), 80, 5));

    assert!(screen[1].contains("[◆ main ⇄ origin]"), "{screen:#?}");
    assert!(
        screen[1].contains('…'),
        "the pile of badges is cut: {screen:#?}"
    );
    assert!(
        screen[1].contains("Commit 2"),
        "the subject survives: {screen:#?}"
    );
    assert!(screen[2].contains("[topic] Commit 1"), "{screen:#?}");
}

#[test]
fn ui_icons_changes_how_badges_render() {
    let rows = linear(2);
    let plain = text(&draw(&badged(&rows, Icons::Plain), 120, 5)).join("\n");
    let nerd = text(&draw(&badged(&rows, Icons::Nerd), 120, 5)).join("\n");

    assert_ne!(plain, nerd);
    insta::assert_snapshot!("badges-icons-plain", plain);
    insta::assert_snapshot!("badges-icons-nerd", nerd);
}

#[test]
fn with_no_color_the_cursor_row_keeps_its_rule_and_dimmed_rows_stay_faint() {
    let rows = linear(3);
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, true));
    let mut view = view(&rows, WalkView::Complete { rows: 3 }, 1);
    view.rows[2].line.paint = RowPaint::Dimmed;
    let mut terminal = Terminal::new(TestBackend::new(40, 6)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(&view, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();

    // No colour at all, so no background step and no accent: the rule's
    // shape is the highlight, and weight/faintness carry the paint.
    let has_rule = (0..buffer.area.width).any(|x| buffer[(x, 2)].symbol() == "▏");
    assert!(has_rule, "expected a rule on the cursor row: {buffer:#?}");
    assert!(buffer[(10, 2)].modifier.contains(Modifier::BOLD));
    assert!(buffer[(10, 3)].modifier.contains(Modifier::DIM));
}

/// The first `count` cells of a screen line.
fn cells(line: &str, count: usize) -> String {
    line.chars().take(count).collect()
}

/// The twelve merge rows of a twelve-topic history: thirteen columns open,
/// more than `graph.max_lanes` allows.
fn wide_merges() -> Vec<Row> {
    let mut rows = wide_history(12);
    rows.truncate(12);
    rows
}

#[test]
fn more_lanes_than_the_limit_fold_behind_a_marker_and_leave_the_subject_readable() {
    let rows = wide_merges();
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 12 }, 0),
        80,
        24,
    ));

    // Seven lanes plus `+6`: 17 cells, the subject right after.
    assert_eq!(cells(&screen[1], 17), "◉─────────────+6 ", "{screen:#?}");
    assert!(screen[1].contains("Merge branch 'topic-12'"), "{screen:#?}");
    assert!(screen[12].contains("Merge branch 'topic-1'"), "{screen:#?}");
    assert!(screen[23].contains("+ expand lanes"), "{screen:#?}");
}

#[test]
fn expanded_every_lane_draws_in_its_real_column_from_the_same_rows() {
    let rows = wide_merges();
    let mut expanded = view(&rows, WalkView::Complete { rows: 12 }, 0);
    expanded.compaction.expanded = true;
    let screen = text(&draw(&expanded, 80, 24));

    assert_eq!(
        cells(&screen[1], 26),
        "◉─╮                       ",
        "{screen:#?}"
    );
    assert_eq!(
        cells(&screen[12], 26),
        "◉─│─│─│─│─│─│─│─│─│─│─│─╮ ",
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains("+6")),
        "{screen:#?}"
    );
    assert!(screen[23].contains("+ collapse lanes"), "{screen:#?}");
}

#[test]
fn within_the_limit_there_is_nothing_to_expand_and_no_hint_says_so() {
    let rows = linear(3);
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 3 }, 0),
        80,
        6,
    ));
    assert!(!screen[5].contains("lanes"), "{screen:#?}");
}

/// The header says what the graph leaves out, and says nothing when it
/// leaves nothing out.
#[test]
fn filters_are_named_in_the_header_only_when_something_is_left_out() {
    use super::Filters;
    assert_eq!(Filters::default().text(), "");
    let all = Filters {
        first_parent: true,
        folded: 2,
        stash_hidden: true,
        focused: true,
    };
    assert_eq!(
        all.text(),
        "first-parent · 2 merges folded · stash hidden · focus (F/Esc off)"
    );
    let one = Filters {
        folded: 1,
        ..Filters::default()
    };
    assert_eq!(one.text(), "1 merge folded");
}
