use std::collections::BTreeSet;

use graft_core::capabilities::ColorDepth;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::mistake_radar::Severity;
use graft_core::oid::Oid;
use graft_graph::layout::layout;
use graft_graph::row::Row;
use graft_graph::seeds::Seeds;
use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

use super::footer::{HINTS, commit_graph_segments, grouped, key_hints};
use super::layout::walk_text;
use super::*;
use crate::graph_column::{GraphColumn, RowPaint};
use crate::paint::color;
use crate::test_support::{terminal, wide_history};
use crate::theme::{Theme, ThemeKind, Token};

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
                    compare_marked: false,
                    marker: None,
                }
            })
            .collect(),
        icons: Icons::Plain,
        compaction: Compaction::at(8),
        filters: Filters::default(),
        reveal: None,
        order: None,
        ascii: false,
        pixel: false,
        graphics: None,
        notice: None,
        search: None,
        compare_footer: None,
        commit_graph: None,
        banner: Vec::new(),
        detail_shown: false,
        changes_pane: false,
        marker_column: false,
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
fn a_banner_sits_under_the_header_and_takes_rows_from_the_body() {
    let rows = linear(3);
    let mut view = view(&rows, WalkView::Complete { rows: 3 }, 0);
    view.banner = vec![
        sanitize_line("gallery 1/23 · trivial · ] next"),
        sanitize_line("shows: three commits"),
    ];
    let screen = text(&draw(&view, 40, 6));

    assert!(screen[0].contains("/code/api"), "{screen:#?}");
    assert!(screen[1].contains("gallery 1/23 · trivial"), "{screen:#?}");
    assert!(screen[2].contains("shows: three commits"), "{screen:#?}");
    assert!(screen[3].contains("Commit 3"), "{screen:#?}");
    assert!(screen[4].contains("Commit 2"), "{screen:#?}");
    assert!(
        !screen[4].contains("Commit 1"),
        "two rows left for the body"
    );
    assert!(
        screen[5].contains("q quit"),
        "the key hints keep the footer"
    );

    // The pixel tier's image rectangle moves down with the rows.
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let area = GraphScreen::new(&view, &theme).graph_area(Rect::new(0, 0, 40, 6));
    assert_eq!((area.y, area.height), (3, 2));
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

    // Since M2 the marker is painted, not laid out (`Row::marker` stays
    // unset): `GraphEntry::marker`/`GraphView::marker_column` carry it.
    let mut filled = view(&rows, WalkView::Complete { rows: 2 }, 0);
    filled.rows[0].marker = Some(MarkerView {
        severity: Severity::Warning,
    });
    filled.marker_column = true;
    let filled_screen = text(&draw(&filled, 120, 5));

    // Whether or not row 0 carries a marker, the subject starts at the same
    // column: the marker glyph and its neighbour's blank both take
    // `MARKER_WIDTH` cells, so later columns never shift.
    // A char index, not `str::find`'s byte offset: `⚠` is multi-byte, so a
    // byte offset would report a shift that isn't there in cells.
    let subject_at = |line: &str| {
        let byte = line.find("Commit").expect("the subject is drawn");
        line[..byte].chars().count()
    };
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
fn more_lanes_than_the_limit_fold_behind_a_rail_and_leave_the_subject_readable() {
    let rows = wide_merges();
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 12 }, 0),
        80,
        24,
    ));

    // Seven lanes plus the rail: 16 cells, the subject right after. The
    // count is the header's, never a row's.
    assert_eq!(cells(&screen[1], 16), "◉▾            ⋮ ", "{screen:#?}");
    assert!(screen[0].trim_end().ends_with("+6 lanes"), "{screen:#?}");
    assert!(
        !screen[1..23].iter().any(|line| line.contains("+6")),
        "{screen:#?}"
    );
    assert!(screen[1].contains("Merge branch 'topic-12'"), "{screen:#?}");
    assert!(screen[12].contains("Merge branch 'topic-1'"), "{screen:#?}");
    assert!(screen[23].contains("+ expand lanes"), "{screen:#?}");
}

/// ADR-0025: a thread twenty columns wide is a priority, not a veto — the
/// configured cap still holds. Twenty-two rows (twenty merges plus two
/// topics), the cursor on the topmost merge, its whole thread kept: the
/// column still stops at `max_lanes = 8`, the rail is the last column drawn
/// on every row, and the header's count is exact.
#[test]
fn a_thread_of_twenty_columns_still_fits_the_configured_cap() {
    let mut rows = wide_history(20);
    rows.truncate(22);
    let opened_on = |k: usize| rows[k].outgoing[1].0;
    let selected = opened_on(0);
    let thread: BTreeSet<usize> = (0..20).map(opened_on).collect();

    let mut screen_view = view(&rows, WalkView::Complete { rows: 22 }, 0);
    screen_view.compaction = Compaction {
        max_lanes: 8,
        expanded: false,
        keep: Keep {
            selected: Some(selected),
            reserved: BTreeSet::new(),
            preferred: thread,
        },
    };
    let fold = screen_view.compaction.plan(rows.iter());
    let width = usize::from(GraphColumn::width_through(&fold));
    assert!(width <= 40, "graph column {width} cells wide");

    let open = fold.kept().len() + fold.folded().len();
    let screen = text(&draw(&screen_view, 80, 24));
    assert!(
        screen[0]
            .trim_end()
            .ends_with(&format!("+{} lanes", open - 7)),
        "{screen:#?}"
    );

    // The rail is the drawn column right after every kept lane: nothing but
    // the marker and padding sits between the graph and the subject.
    let rail = fold.marker().expect("the thread does not fit unfolded");
    assert_eq!(rail, fold.kept().len());
    for line in &screen[1..23] {
        assert_eq!(
            cells(line, width).chars().count(),
            width,
            "row shorter than the column it was given: {line:?}"
        );
    }
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
        !screen
            .iter()
            .any(|line| line.contains("+6") || line.contains('⋮')),
        "expanded: no count and no rail: {screen:#?}"
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
    assert!(
        !screen[0].contains("lanes"),
        "nothing folds, no count: {screen:#?}"
    );
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

/// A linear history like [`linear`], but the newest row's subject is
/// `subject` verbatim (sanitised) instead of `"Commit N"` — for forcing (or
/// not forcing) the `…` truncation the footer reveal keys off.
fn linear_with_subject(count: u8, subject: &str) -> Vec<Row> {
    let mut rows = linear(count);
    rows[0].commit.message = subject.as_bytes().to_vec();
    rows
}

const LONG_SUBJECT: &str =
    "A very long subject line that is much wider than the row and needs the ellipsis";

/// ADR-0023: no popup. A row whose subject needed `…` reveals it in full in
/// the footer, in place of the ordinary hints, while `reveal` names it.
#[test]
fn a_reveal_replaces_the_hints_with_the_full_subject() {
    let rows = linear_with_subject(3, LONG_SUBJECT);
    let mut open = view(&rows, WalkView::Complete { rows: 3 }, 1);
    open.reveal = Some(0);

    let screen = text(&draw(&open, 90, 6));
    let footer = screen.last().expect("a footer row");
    assert!(footer.contains(LONG_SUBJECT), "{screen:#?}");
    assert!(!footer.contains('…'), "{screen:#?}");
    assert!(!footer.contains("move"), "{screen:#?}");
}

/// A short subject never needed the `…` in the first place: naming it in
/// `reveal` has nothing to add, so the ordinary hints stay (ADR-0023 — a row
/// that already reads in full is not "cut with `…`").
#[test]
fn revealing_a_row_that_was_never_truncated_leaves_the_hints_alone() {
    let rows = linear(3);
    let mut open = view(&rows, WalkView::Complete { rows: 3 }, 1);
    open.reveal = Some(0);

    // 90, not 60: since m4-remotes ticket 03 the remote-write hints sit
    // ahead of `j/k move` (design note "New UI"), and 60 columns is too
    // narrow to keep both — this test is about the reveal, not the width.
    let screen = text(&draw(&open, 90, 12));
    let footer = screen.last().expect("a footer row");
    assert!(footer.contains("move"), "{screen:#?}");
    assert!(!footer.contains("Commit 3"), "{screen:#?}");
}

/// `reveal: None` is the ordinary state: hints, no popup, nothing hidden.
#[test]
fn no_reveal_shows_the_ordinary_hints() {
    let rows = linear(3);
    let open = view(&rows, WalkView::Complete { rows: 3 }, 1);
    assert_eq!(open.reveal, None);

    // 90, not 60 — see the comment on the reveal test above.
    let screen = text(&draw(&open, 90, 12));
    let footer = screen.last().expect("a footer row");
    assert!(footer.contains("move"), "{screen:#?}");
}

/// Escape sequences in a subject render inert in the reveal, the same as
/// everywhere else repository text crosses into the view (17's fixture).
#[test]
fn escape_sequences_render_inert_in_the_reveal() {
    let rows = linear(3);
    let mut open = view(&rows, WalkView::Complete { rows: 3 }, 1);
    open.rows[0].subject = sanitize_line("Subject\x1b[31mred");
    open.reveal = Some(0);

    let screen = text(&draw(&open, 60, 12));
    let footer = screen.last().expect("a footer row");
    assert!(!footer.contains('\x1b'), "{screen:#?}");
}

#[test]
fn the_pixel_tier_leaves_the_graph_column_blank_at_the_same_width() {
    let rows = linear(3);
    let text_tier = view(&rows, WalkView::Complete { rows: 3 }, 1);
    let mut pixel_tier = text_tier.clone();
    pixel_tier.pixel = true;
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let area = Rect::new(0, 0, 40, 6);
    let graph = GraphScreen::new(&text_tier, &theme).graph_area(area);

    let drawn = text(&draw(&text_tier, 40, 6));
    let blank = text(&draw(&pixel_tier, 40, 6));

    assert_eq!(graph, Rect::new(0, 1, graph.width, 4), "every body row");
    assert!(graph.width > 0);
    assert_eq!(
        GraphScreen::new(&pixel_tier, &theme).graph_area(area),
        graph
    );
    let columns = usize::from(graph.width);
    let split = |line: &str| -> (String, String) {
        (
            line.chars().take(columns).collect(),
            line.chars().skip(columns).collect(),
        )
    };
    for y in 1..4 {
        let (drawn_graph, drawn_text) = split(&drawn[y]);
        let (blank_graph, blank_text) = split(&blank[y]);
        assert_ne!(drawn_graph.trim(), "", "{drawn:#?}");
        assert_eq!(blank_graph.trim(), "", "{blank:#?}");
        // Everything right of the column is where it was.
        assert_eq!(drawn_text, blank_text, "{blank:#?}");
    }
}

#[test]
fn the_graphics_status_is_named_by_its_state() {
    let rows = linear(1);
    let mut screen = view(&rows, WalkView::Complete { rows: 1 }, 0);
    assert!(!text(&draw(&screen, 80, 4))[0].contains("gfx"));

    screen.graphics = Some(GraphicsStatus::Ok);
    assert!(text(&draw(&screen, 80, 4))[0].contains("gfx ok"));

    screen.graphics = Some(GraphicsStatus::Failed(sanitize_line("ENOENT:gone")));
    let header = text(&draw(&screen, 80, 4))[0].clone();
    assert!(header.contains("gfx: ENOENT:gone"), "{header}");
}

/// The footer row alone at `columns`, drawn over a short linear history.
fn footer_at(columns: u16) -> String {
    let rows = linear(3);
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 3 }, 0),
        columns,
        6,
    ));
    screen[5].trim_end().to_owned()
}

#[test]
fn the_footer_lists_every_graph_key_dropping_whole_hints_when_narrow() {
    // Widened from 200 when the remote hints (`f`/`p`/`P`) were added
    // (m4-remotes ticket 03): 19 hints no longer fit in 200 columns.
    let wide = footer_at(230);
    for hint in HINTS {
        assert!(wide.contains(hint), "{hint} missing: {wide}");
    }
    for columns in [80, 120, 200] {
        let footer = footer_at(columns);
        assert!(footer.contains("F3 renderer"), "{columns}: {footer}");
        assert!(footer.chars().count() <= usize::from(columns), "{footer}");
        // A narrower footer is the wide one cut at a hint boundary.
        assert!(wide.starts_with(&footer), "{columns}: {footer}");
        assert!(
            wide.len() == footer.len() || wide[footer.len()..].starts_with("  "),
            "{columns} cut a hint in half: {footer}"
        );
        insta::assert_snapshot!(format!("footer-hints-{columns}"), footer);
    }
}

/// Design note "New UI": `f fetch  p pull  P push` at 160 and at 120
/// columns — the widths the note says the drop-from-the-right rule keeps
/// them at. Grouped with `u undo` in [`LEADING_HINTS`] rather than at the
/// mockup's position after `S stash`: placed there instead, only `P push`
/// survives 160 and none survive 120 — see the comment on [`HINTS`].
#[test]
fn remote_hints_at_wide_and_medium() {
    for columns in [120, 160] {
        let footer = footer_at(columns);
        assert!(footer.contains("f fetch"), "{columns}: {footer}");
        assert!(footer.contains("p pull"), "{columns}: {footer}");
        assert!(footer.contains("P push"), "{columns}: {footer}");
        let undo = footer.find("u undo").expect("u undo present");
        let fetch = footer.find("f fetch").expect("f fetch present");
        let search = footer.find("/ search").expect("/ search present");
        assert!(undo < fetch && fetch < search, "{columns}: {footer}");
    }
}

/// m3-working-tree-undo-and-stash/04: the write keys are hinted on the
/// changes pane and nowhere else, so the footer never offers a key that does
/// nothing where the cursor is.
#[test]
fn the_write_hints_appear_only_on_the_changes_pane() {
    let rows = linear(3);
    let mut view = view(&rows, WalkView::Complete { rows: 3 }, 0);

    let quiet = text(&draw(&view, 200, 8));
    let quiet_footer = quiet.last().expect("a footer row").clone();
    assert!(
        !quiet_footer.contains("space stage") && !quiet_footer.contains("d discard"),
        "{quiet_footer}"
    );

    view.detail_shown = true;
    view.changes_pane = true;
    let open = text(&draw(&view, 200, 8));
    let open_footer = open.last().expect("a footer row").clone();
    assert!(
        open_footer.contains("space stage") && open_footer.contains("d discard"),
        "{open_footer}"
    );
    // Ahead of the movement keys, so a narrow terminal keeps them.
    let stage = open_footer.find("space stage").expect("the hint is there");
    let movement = open_footer.find("j/k move").expect("the movement hint");
    assert!(stage < movement, "{open_footer}");
}

#[test]
fn the_expand_hint_keeps_its_place_after_the_renderer() {
    let line = key_hints(&["+ expand lanes"], 200);
    assert!(
        line.starts_with(" q quit  F2 status  F3 renderer  u undo  + expand lanes  f fetch"),
        "{line}"
    );
}

// m1-the-graph/44: the lane cap follows the terminal's width (ADR-0022
// decision 3, ADR-0025 decision 3).

#[test]
fn lanes_that_fit_matches_graph_widths_half_screen_bound() {
    // Half of 8 columns is 4 cells, one lane at 2 cells each — but the rail
    // always fits alongside lane 0, so the floor is 2, not 1.
    assert_eq!(lanes_that_fit(8), 2);
    // Half of 80 columns is 40 cells, 20 lanes at 2 cells each.
    assert_eq!(lanes_that_fit(80), 20);
}

#[test]
fn max_lanes_for_follows_the_width_breakpoints_when_unconfigured() {
    assert_eq!(max_lanes_for(79, None), 8);
    assert_eq!(max_lanes_for(100, None), 10);
    assert_eq!(max_lanes_for(159, None), 10);
    assert_eq!(max_lanes_for(160, None), 12);
    assert_eq!(max_lanes_for(200, Some(5)), 5);
}

#[test]
fn max_lanes_for_clamps_a_configured_value_to_what_the_screen_holds() {
    assert_eq!(max_lanes_for(80, Some(30)), lanes_that_fit(80));
    assert_eq!(max_lanes_for(80, Some(30)), 20);
}

/// A history wide enough that its open columns exceed even the screen's own
/// bound at 80 columns (more than `lanes_that_fit(80) == 20`).
fn very_wide_merges() -> Vec<Row> {
    let mut rows = wide_history(30);
    rows.truncate(30);
    rows
}

#[test]
fn a_resize_from_80_to_160_columns_changes_the_folds_lane_count() {
    let rows = very_wide_merges();
    let mut view = view(&rows, WalkView::Complete { rows: rows.len() }, 0);
    view.compaction = Compaction::at(max_lanes_for(80, None));
    let narrow = text(&draw(&view, 80, 24));
    let narrow_header = narrow[0].clone();

    view.compaction = Compaction::at(max_lanes_for(160, None));
    let wide = text(&draw(&view, 160, 40));
    let wide_header = wide[0].clone();

    // 30 topics open 31 columns; 8 lanes at 80 columns and 10 at 160 both
    // fold most of them, but the wider screen folds fewer.
    assert!(narrow_header.contains("lanes"), "{narrow_header}");
    assert!(wide_header.contains("lanes"), "{wide_header}");
    assert_ne!(narrow_header, wide_header);
}

#[test]
fn a_configured_cap_larger_than_the_screen_never_cuts_the_graph_column() {
    let rows = very_wide_merges();
    let mut view = view(&rows, WalkView::Complete { rows: rows.len() }, 0);
    let cap = max_lanes_for(80, Some(30));
    assert_eq!(cap, 20, "clamped to what 80 columns can hold");
    view.compaction = Compaction::at(cap);
    let screen = text(&draw(&view, 80, 24));

    // 20 lanes (rail included) at 2 cells each is 40 cells — exactly the
    // half-screen bound `graph_width` enforces, so the column is not cut.
    assert!(screen[1].contains('⋮'), "no rail drawn: {screen:#?}");
    let rail_at = screen[1]
        .chars()
        .position(|c| c == '⋮')
        .expect("rail is drawn");
    // The rail is the last drawn column: column (cap - 1), 2 cells wide.
    assert_eq!(rail_at, (cap - 1) * 2, "{screen:#?}");
    assert!(rail_at <= 40, "cut past the half-screen bound: {screen:#?}");
}

#[test]
fn every_commit_graph_wording_fits_under_its_breakpoint() {
    let lines = [
        CommitGraphLine::Offered,
        CommitGraphLine::NoGitToBuild,
        CommitGraphLine::Writing,
        CommitGraphLine::Written { commits: 1_000_000 },
    ];
    // Each wording holds from its breakpoint up; the narrowest from 80.
    for (width, room) in [(120, 120), (100, 100), (80, 80)] {
        for line in lines {
            let text: String = commit_graph_segments(line, width)
                .into_iter()
                .map(|(text, _)| text)
                .collect();
            assert!(
                UnicodeWidthStr::width(text.as_str()) <= room,
                "{line:?} at {width}: {} cols: {text:?}",
                UnicodeWidthStr::width(text.as_str())
            );
        }
    }
    let written: String =
        commit_graph_segments(CommitGraphLine::Written { commits: 1_000_000 }, 120)
            .into_iter()
            .map(|(text, _)| text)
            .collect();
    assert_eq!(
        written,
        " commit-graph written: 1,000,000 commits · the next open sorts at once · Ctrl+R to use it"
    );
    assert_eq!(grouped(200), "200");
    assert_eq!(grouped(1_000), "1,000");
}

#[test]
fn the_commit_graph_line_sits_above_the_hints_and_takes_a_graph_row() {
    let rows = linear(30);
    let mut shown = view(&rows, WalkView::Complete { rows: 30 }, 0);
    let without = text(&draw(&shown, 80, 10));
    shown.commit_graph = Some(CommitGraphLine::Offered);
    let with = text(&draw(&shown, 80, 10));

    assert_eq!(with[9], without[9], "the hints keep the bottom row");
    assert!(
        with[8].starts_with(" no commit-graph file · B build one · X never here"),
        "{:?}",
        with[8]
    );
    assert_eq!(with[7], without[7], "the rows above are unchanged");
    assert_ne!(with[8], without[8], "one graph row gave way");
}

/// `. explain` (`m2-details-and-diffs/17`) is the cursor row's own hint: it
/// shows only when the row painted [`RowPaint::Highlighted`] carries a
/// marker, never for a marker elsewhere or no marker at all.
#[test]
fn the_explain_hint_shows_only_on_a_marked_cursor_row() {
    let rows = linear(2);

    let plain = view(&rows, WalkView::Complete { rows: 2 }, 0);
    let plain_screen = text(&draw(&plain, 80, 5));
    assert!(
        !plain_screen[4].contains(". explain"),
        "{:?}",
        plain_screen[4]
    );

    // The marker on the *other* row (index 1, off the cursor at index 0)
    // must not turn the hint on.
    let mut marked_elsewhere = view(&rows, WalkView::Complete { rows: 2 }, 0);
    marked_elsewhere.rows[1].marker = Some(MarkerView {
        severity: Severity::Warning,
    });
    let elsewhere_screen = text(&draw(&marked_elsewhere, 80, 5));
    assert!(
        !elsewhere_screen[4].contains(". explain"),
        "{:?}",
        elsewhere_screen[4]
    );

    let mut marked_cursor = view(&rows, WalkView::Complete { rows: 2 }, 0);
    marked_cursor.rows[0].marker = Some(MarkerView {
        severity: Severity::Warning,
    });
    let cursor_screen = text(&draw(&marked_cursor, 80, 5));
    assert!(
        cursor_screen[4].contains(". explain"),
        "{:?}",
        cursor_screen[4]
    );
}
