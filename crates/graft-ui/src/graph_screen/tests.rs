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

use super::footer::{commit_graph_segments, grouped, segments_width};
use super::hints::{HintContext, TabWord};
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
pub(super) fn linear(count: u8) -> Vec<Row> {
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

pub(super) fn view(rows: &[Row], walk: WalkView, cursor: usize) -> GraphView<'_> {
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
        stopped: None,
        commit_graph: None,
        banner: Vec::new(),
        divider_shown: false,
        hint_context: HintContext::PlainRow,
        marker_column: false,
        banner_op: None,
    }
}

pub(super) fn draw(view: &GraphView<'_>, columns: u16, rows: u16) -> Terminal<TestBackend> {
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(view, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    terminal
}

pub(super) fn text(terminal: &Terminal<TestBackend>) -> Vec<String> {
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
    assert!(screen[5].contains("⏎ open"), "{screen:#?}");
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
        screen[5].contains("⏎ open"),
        "the key hints keep the footer"
    );

    // The pixel tier's image rectangle moves down with the rows.
    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let area = GraphScreen::new(&view, &theme).graph_area(Rect::new(0, 0, 40, 6));
    assert_eq!((area.y, area.height), (3, 2));
}

#[test]
fn the_cursor_row_is_selected_and_the_others_are_not() {
    let rows = linear(3);
    let terminal = draw(&view(&rows, WalkView::Complete { rows: 3 }, 1), 40, 6);
    let buffer = terminal.backend().buffer();
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps());
    let selection = color(theme.token(Token::Selection));

    assert_eq!(buffer[(30, 2)].bg, selection);
    assert_ne!(buffer[(30, 1)].bg, selection);
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
    // untouched, same as an unhighlighted row, not `selection`.
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

/// m5-history-and-ghost-preview/35. **This test used to assert the
/// opposite** — that the `▏` rule appears *only* at Palette16, and that
/// `TrueColor` is marked by the raised background alone. That was the
/// behaviour the user could not see: the background step is `#14161B` →
/// `#232730` in the dark theme, and *"really and I mean really hard to
/// notice which row I'm on"* is what it came to in his terminal.
///
/// The rule is drawn at every depth now, so both marks are on the cursor
/// row at `TrueColor`: the fill for terminals that show it well, and the
/// shape for the reader — and for the black-and-white screenshot, and for
/// the theme whose surfaces sit close together.
#[test]
fn at_truecolor_the_cursor_row_gets_the_rule_as_well_as_the_background() {
    let rows = linear(3);
    let terminal = draw(&view(&rows, WalkView::Complete { rows: 3 }, 1), 40, 6);
    let buffer = terminal.backend().buffer();
    let theme = Theme::new(ThemeKind::Dark, &terminal_caps());
    let selection = color(theme.token(Token::Selection));
    let accent = color(theme.token(Token::Accent));

    assert_eq!(buffer[(30, 2)].bg, selection, "the fill is still painted");
    let rule = (0..buffer.area.width)
        .find(|x| buffer[(*x, 2)].symbol() == "▏")
        .expect("the cursor row carries the rule at TrueColor too");
    assert_eq!(
        buffer[(rule, 2)].fg,
        accent,
        "the rule is the accent colour"
    );
    assert!(
        buffer[(rule, 2)].modifier.contains(Modifier::BOLD),
        "and bold, so it reads against the lane glyphs beside it"
    );
    assert!(
        !(0..buffer.area.width).any(|x| buffer[(x, 1)].symbol() == "▏"),
        "no other row carries one"
    );
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

/// A stopped walk says so, and names the key that starts a new one when the
/// header has room: the widest of three wordings that fits after the
/// location (m6-conflicts/85 A2). A 46-column location and the brand leave
/// 65 columns for the words at 120, 25 at 80 and 15 at 70.
#[test]
fn the_stopped_header_keeps_the_widest_wording_that_fits() {
    let rows = linear(2);
    let location = format!("/{}", "r".repeat(45));
    let header = |columns: u16| {
        let mut stopped = view(&rows, WalkView::Cancelled { laid_out: 500 }, 0);
        stopped.location = Some(sanitize_line(&location));
        text(&draw(&stopped, columns, 5))[0].clone()
    };

    let wide = header(120);
    assert!(
        wide.contains("stopped at 500 rows \u{b7} \u{2303}R reloads"),
        "{wide}"
    );
    let narrow = header(80);
    assert!(narrow.contains("stopped at 500 rows"), "{narrow}");
    assert!(
        !narrow.contains("reloads") && !narrow.contains('…'),
        "{narrow}"
    );
    let narrower = header(70);
    assert!(narrower.trim_end().ends_with("stopped"), "{narrower}");
    assert!(!narrower.contains("stopped at"), "{narrower}");
}

/// A walk resting between requests reads as rows so far; one working on an
/// ask reads as reading. The two never share words, so the header can say
/// when `Esc` has something to stop (m6-conflicts/85 A1).
#[test]
fn a_paused_walk_and_a_reading_one_read_apart() {
    let (paused, _) = walk_text(&WalkView::Paused { laid_out: 500 });
    let (reading, _) = walk_text(&WalkView::Reading { laid_out: 500 });
    assert_eq!(paused, "500 rows so far");
    assert_eq!(reading, "reading\u{2026} 500 rows");
}

#[test]
fn a_walk_that_is_fine_never_reads_as_an_error() {
    for walk in [
        WalkView::Opening,
        WalkView::Reading { laid_out: 0 },
        WalkView::Reading { laid_out: 1000 },
        WalkView::Paused { laid_out: 1000 },
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
    view.problem = Some(ProblemLine {
        reason: sanitize_line("job 3 failed: not a git repository"),
        keys: None,
    });
    let screen = text(&draw(&view, 60, 5));

    assert!(screen[4].contains("not a git repository"), "{screen:#?}");
    assert!(!screen[4].contains("⏎ open"), "{screen:#?}");
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
    assert!(screen[4].contains("⏎ open"), "{screen:#?}");
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

/// m5-history-and-ghost-preview/35. The marker slot holds **both** marks
/// when a row has both: the cursor's rule in its first cell, the Mistake
/// Radar's glyph in its second.
///
/// Found by the capture this ticket took, not by reasoning: the first cut
/// of 35 kept the radar's precedence, and on `docs/shots/m5-ui-dragging-*`
/// the row carrying `ⓘ` lost its rule entirely — back to the raised
/// background alone, which is the one signal the user had already told us
/// he could not see. A fix for "you cannot find the cursor" that switches
/// itself off on some rows is not a fix.
#[test]
fn a_row_with_a_marker_still_shows_the_cursor_rule() {
    let rows = linear(2);
    let mut filled = view(&rows, WalkView::Complete { rows: 2 }, 0);
    filled.rows[0].marker = Some(MarkerView {
        severity: Severity::Warning,
    });
    filled.marker_column = true;
    let terminal = draw(&filled, 120, 5);
    let buffer = terminal.backend().buffer();

    let rule = (0..buffer.area.width)
        .find(|x| buffer[(*x, 1)].symbol() == "▏")
        .expect("the cursor row keeps its rule beside a marker");
    assert_eq!(
        buffer[(rule + 1, 1)].symbol(),
        "⚠",
        "and the marker takes the cell next to it"
    );

    // The columns after it have not moved: the slot is `MARKER_WIDTH` cells
    // whatever it holds.
    let empty = view(&rows, WalkView::Complete { rows: 2 }, 0);
    let plain = text(&draw(&empty, 120, 5));
    let both = text(&terminal);
    let subject_at = |line: &str| {
        let byte = line.find("Commit").expect("the subject is drawn");
        line[..byte].chars().count()
    };
    assert_eq!(subject_at(&plain[1]), subject_at(&both[1]));
}

/// m5-history-and-ghost-preview/40. A marker glyph never touches the subject.
///
/// 35 gave the cursor's rule a cell of the two the marker slot already had,
/// and the cell it took was the gap: every row carrying a radar marker drew
/// `▏⚠Merge branch 'feature'`, the warning welded to the words it warns
/// about, in ten committed captures. The slot is three cells now — rule,
/// glyph, gap — and this is the test that says so, at every width the
/// screen is drawn at, for both severities and with and without the cursor.
#[test]
fn a_marker_glyph_never_touches_the_subject() {
    let rows = linear(2);
    for severity in [Severity::Warning, Severity::Info] {
        for cursor in [0usize, 1] {
            for width in [80u16, 100, 120, 160] {
                let mut filled = view(&rows, WalkView::Complete { rows: 2 }, cursor);
                filled.rows[0].marker = Some(MarkerView { severity });
                filled.marker_column = true;
                let terminal = draw(&filled, width, 5);
                let buffer = terminal.backend().buffer();

                let glyph = (0..buffer.area.width)
                    .find(|x| matches!(buffer[(*x, 1)].symbol(), "⚠" | "ⓘ"))
                    .unwrap_or_else(|| {
                        panic!("the marker is drawn at {width}: {:#?}", text(&terminal))
                    });
                assert_eq!(
                    buffer[(glyph + 1, 1)].symbol(),
                    " ",
                    "{severity:?}, cursor on row {cursor}, {width} columns: the cell \
                     after the marker is the gap, not the subject: {:#?}",
                    text(&terminal)
                );
            }
        }
    }
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
    // The lane-fold hint (`+ expand lanes`) is not in the design note's
    // hint-bar table for any context (`hint-bar-redesign`) and is dropped
    // from the footer with this ticket — `+` still works, but is no longer
    // hinted (flagged as a gap in the ticket's NOTES, not fixed here: the
    // note's table is followed verbatim per the ticket's own instruction).
    assert!(screen[23].contains("⏎ open"), "{screen:#?}");
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
    // See `more_lanes_than_the_limit_fold_behind_a_rail_and_leave_the_subject_readable`:
    // the lane-fold hint is no longer drawn (`hint-bar-redesign`).
    assert!(screen[23].contains("⏎ open"), "{screen:#?}");
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

    let screen = text(&draw(&open, 60, 12));
    let footer = screen.last().expect("a footer row");
    assert!(footer.contains("⏎ open"), "{screen:#?}");
    assert!(!footer.contains("Commit 3"), "{screen:#?}");
}

/// `reveal: None` is the ordinary state: hints, no popup, nothing hidden.
#[test]
fn no_reveal_shows_the_ordinary_hints() {
    let rows = linear(3);
    let open = view(&rows, WalkView::Complete { rows: 3 }, 1);
    assert_eq!(open.reveal, None);

    let screen = text(&draw(&open, 60, 12));
    let footer = screen.last().expect("a footer row");
    assert!(footer.contains("⏎ open"), "{screen:#?}");
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

/// The footer's hint list widens with the frame, not just in the unit tests
/// over `hints_for` directly: every extra cell of row buys hints, and none
/// of the rows overflows (`view()`'s default context).
#[test]
fn the_footer_hint_list_widens_with_the_frame() {
    let at = |columns| footer_at(columns).split("  ").count();
    assert!(at(80) >= 4, "{}", footer_at(80));
    assert!(
        at(120) > at(80),
        "80: {} / 120: {}",
        footer_at(80),
        footer_at(120)
    );
    assert!(at(160) >= at(120), "{}", footer_at(160));
    // `q` is reached at 160; whether it keeps its verb depends on how much
    // the context has of its own to say (m5-history-and-ghost-preview/10
    // made `b branch` and `t tag` live on every row list).
    assert!(footer_at(160).contains(" q "), "{}", footer_at(160));
    for columns in [80u16, 120, 160] {
        assert!(
            footer_at(columns).chars().count() <= usize::from(columns),
            "the bar overflowed {columns}: {}",
            footer_at(columns)
        );
    }
}

/// Acceptance (g): the footer at 80x24 for a branch row leads with the
/// note's table order — the note's own `m merge` and `r rebase` are not
/// among them until a group binds them, so the first three are the first
/// three the branch row actually answers (m5-03 review, Critical 2;
/// `hint-bar-redesign`) — and fills the rest of the row, `. more` leading
/// the bare keys that follow (variant B, m5/28).
///
/// Re-measured for the shrink ladder (m5/24, ADR-0039's third amendment):
/// the three most relevant keep whole verbs and the row's remaining cells
/// now go on more *keys* rather than on four more verbs — eight hints became
/// seventeen, and the ones past the leading three are on their bare-key
/// rung. `b` and `t` joined them in m5-history-and-ghost-preview/10, and
/// `j/k` fell off the end of the row to make room. Which of those two rows is better is the user's call by eye
/// (`docs/shots/m5-hints-ladder.txt`, CLAUDE.md rule 6); what this test pins
/// is that the leading verbs survive at 80 columns.
#[test]
fn footer_eighty_branch_row() {
    let rows = linear(3);
    let mut view = view(&rows, WalkView::Complete { rows: 3 }, 0);
    view.hint_context = HintContext::BranchRow { current: false };
    let screen = text(&draw(&view, 80, 24));
    let footer = screen.last().expect("a footer row").trim_end().to_owned();
    assert_eq!(
        footer,
        " m merge  r rebase  ⏎ open  . more  b  t  P  c  p  f  u  S  /  q  F2  F3  x  +"
    );
}

/// The changes pane's own hints replace the row context's, and drop the
/// write keys for the conflict set on an unmerged file — minus `M mergetool`
/// and `space resolved`, which M6 still has to build (m5-03 review, Critical
/// 2). `<> resize` follows the pane, not the context
/// (m3-working-tree-undo-and-stash/04, m5-history-and-ghost-preview/04).
#[test]
fn the_changes_pane_hints_replace_the_row_context() {
    let rows = linear(3);
    let mut view = view(&rows, WalkView::Complete { rows: 3 }, 0);
    view.hint_context = HintContext::ChangesPane {
        unmerged: false,
        live: None,
        tab: TabWord::Graph,
    };
    view.divider_shown = true;
    let footer = text(&draw(&view, 200, 8))
        .last()
        .expect("a footer row")
        .trim_end()
        .to_owned();
    assert!(footer.contains("space stage"), "{footer}");
    assert!(footer.contains("<> resize"), "{footer}");
    assert!(!footer.contains("m merge"), "{footer}");

    view.hint_context = HintContext::ChangesPane {
        unmerged: true,
        live: None,
        tab: TabWord::Graph,
    };
    view.divider_shown = false;
    let unmerged_footer = text(&draw(&view, 200, 8))
        .last()
        .expect("a footer row")
        .trim_end()
        .to_owned();
    assert_eq!(
        unmerged_footer,
        " ⏎ resolve  M mergetool  e edit  space resolved  c commit  ⇥ graph  . more"
    );
    assert!(
        !unmerged_footer.contains("space stage"),
        "{unmerged_footer}"
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

/// Every wording the commit-graph line has, so the sweep below covers each
/// variant's whole set.
const COMMIT_GRAPH_LINES: [CommitGraphLine; 4] = [
    CommitGraphLine::Offered,
    CommitGraphLine::NoGitToBuild,
    CommitGraphLine::Writing,
    CommitGraphLine::Written { commits: 1_000_000 },
];

/// A footer line never says less because it was given more room, swept over
/// every width rather than sampled at the breakpoints.
///
/// The defect (m5/19, the user's own, by eye): at 120 columns the offer read
/// `no commit-graph file: history sorts on every open · B build one (git
/// commit-graph write --reachable) · X never here`, and at 160 — forty more
/// columns of terminal — `no commit-graph file · B build one · X never
/// here`. The wording was picked by a width *tier* calibrated against the
/// screen, while the width handed in is the graph area's, which loses 42% of
/// the screen to the detail pane at 160. A tier cannot be fixed by moving
/// its numbers; picking the widest wording that fits removes the tier.
///
/// Swept, not sampled: the fault lived at one breakpoint and not the other,
/// which is exactly what checking 120 and 160 by hand would agree with.
#[test]
fn no_footer_line_gets_terser_as_the_room_for_it_grows() {
    for line in COMMIT_GRAPH_LINES {
        let mut narrower = 0usize;
        for width in 0..=240usize {
            let drawn = segments_width(&commit_graph_segments(line, width));
            assert!(
                drawn >= narrower,
                "{line:?}: {width} columns draws {drawn} cells, {} draws {narrower}",
                width - 1
            );
            narrower = drawn;
        }
    }
}

/// The wording drawn is the widest one that fits, so no room is left on the
/// table — and it only overflows when nothing fits at all.
#[test]
fn the_commit_graph_line_spends_the_room_it_is_given() {
    for line in COMMIT_GRAPH_LINES {
        // The widest the line ever gets: from there up, that is what draws.
        let widest = segments_width(&commit_graph_segments(line, usize::MAX));
        for width in 0..=240usize {
            let drawn = segments_width(&commit_graph_segments(line, width));
            if drawn > width {
                // Overflowing is only allowed when the narrowest wording
                // itself does not fit; `commit_graph_line` then cuts it.
                assert_eq!(
                    drawn,
                    segments_width(&commit_graph_segments(line, 0)),
                    "{line:?} at {width}: overflowed with a wording that was not the narrowest"
                );
            }
            if width >= widest {
                assert_eq!(drawn, widest, "{line:?} at {width}: room going spare");
            }
        }
    }
}

/// The hint bar never drops a hint because the bar got wider, swept over
/// every width in every context.
#[test]
fn no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows() {
    for context in every_hint_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let mut narrower = 0usize;
            for columns in 0..=240u16 {
                let drawn = hints_for(&context, divider, columns).len();
                assert!(
                    drawn >= narrower,
                    "{context:?} with {divider:?}: {columns} columns draws {drawn} hints, \
                     {} draws {narrower}",
                    columns - 1
                );
                narrower = drawn;
            }
        }
    }
}

/// ADR-0044 §4: only the resolver's way out (`d start over`, `esc close`)
/// keeps its words ahead of the other hints' words; every other context's ladder is
/// the plain one.
#[test]
fn only_the_resolver_marks_words() {
    for context in every_hint_context() {
        let marked: Vec<_> = hints_for(&context, Divider::Hidden, 240)
            .into_iter()
            .filter(|hint| hint.keeps_words)
            .collect();
        if matches!(context, HintContext::Resolver { .. }) {
            assert!(
                marked.iter().all(|hint| matches!(hint.key, "d" | "esc")),
                "{context:?}: {marked:?}"
            );
            assert!(
                marked.iter().any(|hint| hint.key == "esc"),
                "{context:?}: `esc close` keeps its words"
            );
        } else {
            assert!(marked.is_empty(), "{context:?}: {marked:?}");
        }
    }
}

/// Every context the hint bar has a table for, `Marked` included.
fn every_hint_context() -> Vec<HintContext> {
    let plain = vec![
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
        HintContext::Banner {
            skippable: true,
            conflicted: true,
            merge: false,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: false,
            merge: false,
        },
        HintContext::Preview,
        HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
        },
        HintContext::ChangesPane {
            unmerged: true,
            live: None,
            tab: TabWord::Graph,
        },
        HintContext::Journal,
        HintContext::Stash,
    ];
    let marked = plain
        .iter()
        .cloned()
        .map(|context| HintContext::Marked(Box::new(context)));
    plain.iter().cloned().chain(marked).collect()
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

/// `. explain` (`m2-details-and-diffs/17`) is the cursor row's own hint: the
/// footer shows it exactly when `hint_context` is [`HintContext::Marked`] —
/// decided by the caller (`graft::view::rows`, from whether the cursor row
/// carries a Mistake Radar marker), never derived from `GraphEntry::marker`
/// here (that field only paints the marker glyph, `hint-bar-redesign`).
#[test]
fn the_explain_hint_follows_hint_context_not_the_row_marker() {
    let rows = linear(2);

    let mut plain = view(&rows, WalkView::Complete { rows: 2 }, 0);
    // A marker on a row (even the cursor's) must not turn the hint on by
    // itself: only `hint_context` does.
    plain.rows[0].marker = Some(MarkerView {
        severity: Severity::Warning,
    });
    let plain_screen = text(&draw(&plain, 80, 5));
    assert!(
        !plain_screen[4].contains(". explain"),
        "{:?}",
        plain_screen[4]
    );

    let mut marked = view(&rows, WalkView::Complete { rows: 2 }, 0);
    marked.hint_context = HintContext::Marked(Box::new(HintContext::PlainRow));
    let marked_screen = text(&draw(&marked, 80, 5));
    assert!(
        marked_screen[4].contains(". explain"),
        "{:?}",
        marked_screen[4]
    );
}

// --- Operation banner (m5-history-and-ghost-preview/04, ADR-0038 §3) ---

fn rebase_banner(step: Option<(u32, u32)>, conflict: Option<&str>) -> BannerView {
    BannerView {
        kind: sanitize_line("REBASING"),
        step,
        conflict: conflict.map(sanitize_line),
        focused: false,
        skippable: true,
    }
}

/// The banner names the **state**, and the conflicted file is not on it at
/// any width: the changes pane lists every conflicted file under
/// `CONFLICTS (n)`, and a 24-row screen that says the same fact twice has
/// spent a row on nothing (m5/20 item 1).
#[test]
fn the_banner_names_the_state_and_never_the_files() {
    let banner = rebase_banner(Some((3, 7)), Some("src/auth/token.rs"));
    let text = banner_line(&banner);
    assert_eq!(text, "REBASING step 3 of 7");
    assert!(!text.contains("token.rs"), "{text:?}");
    assert!(!text.contains("conflict"), "{text:?}");
    assert!(!text.contains("(+"), "{text:?}");
}

/// No `⚠`. A stopped merge is a state to finish, not a failure
/// (CLAUDE.md: a status field must never read as an error when nothing is
/// wrong). `⚠` keeps its force for the Mistake Radar's real findings
/// (m5/20 item 2).
#[test]
fn the_banner_carries_no_warning_glyph() {
    for banner in [
        rebase_banner(Some((3, 7)), Some("src/auth/token.rs")),
        rebase_banner(None, None),
    ] {
        let text = banner_line(&banner);
        assert!(!text.contains('⚠'), "{text:?}");
    }
}

/// One wording. `banner_line` takes no width, so there is no tier left to
/// spell the same fact two ways — `conflict in notes.md` at 120 against
/// `conflict notes.md` at 80 and 160, which is what the user read off three
/// captures of one repository in one state (m5/20 item 3). The drawn
/// version of this, across the tab layout too, is
/// `graft::it::conflict_interim::the_banner_reads_the_same_at_every_width`.
#[test]
fn the_banner_is_worded_the_same_whatever_the_header_it_lands_in() {
    let rows = linear(2);
    let banner = rebase_banner(Some((3, 7)), Some("src/auth/token.rs"));
    let mut with_banner = view(&rows, WalkView::Complete { rows: 2 }, 0);
    with_banner.banner_op = Some(banner.clone());
    let whole = banner_line(&banner);
    for columns in [100u16, 120, 160, 200] {
        let header = text(&draw(&with_banner, columns, 6))[0].clone();
        assert!(
            header.contains(&whole),
            "at {columns} columns: {header:?} does not hold {whole:?}"
        );
    }
}

/// No step at all (a plain merge): the state on its own.
#[test]
fn banner_line_with_no_step_names_only_the_operation() {
    let banner = BannerView {
        kind: sanitize_line("MERGING"),
        step: None,
        conflict: Some(sanitize_line("src/x.rs")),
        focused: false,
        skippable: false,
    };
    assert_eq!(banner_line(&banner), "MERGING");
}

/// A step, and nothing after it.
#[test]
fn banner_line_with_a_step_counts_it() {
    assert_eq!(
        banner_line(&rebase_banner(Some((3, 7)), None)),
        "REBASING step 3 of 7"
    );
}

/// The banner replaces the header's walk/filters/order/graphics slot
/// entirely, and reverses when focused.
#[test]
fn the_banner_replaces_the_header_slot_and_reverses_when_focused() {
    let rows = linear(2);
    let mut unfocused = view(&rows, WalkView::Complete { rows: 2 }, 0);
    unfocused.banner_op = Some(rebase_banner(Some((3, 7)), Some("src/auth.rs")));
    let screen = text(&draw(&unfocused, 80, 6));
    assert!(screen[0].contains("REBASING"), "{:?}", screen[0]);
    assert!(!screen[0].contains("commits"), "{:?}", screen[0]);

    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let mut focused = unfocused.clone();
    focused.banner_op.as_mut().unwrap().focused = true;
    let mut terminal = Terminal::new(TestBackend::new(80, 6)).expect("test backend opens");
    terminal
        .draw(|frame| frame.render_widget(GraphScreen::new(&focused, &theme), frame.area()))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();
    let reversed = (0..buffer.area.width).any(|x| {
        buffer[(x, 0)]
            .style()
            .add_modifier
            .contains(Modifier::REVERSED)
    });
    assert!(reversed, "expected the focused banner slot to reverse");
}

/// The banner's room comes out of the header **before** the location's.
///
/// The location used to take what it liked and hand the banner the
/// remainder, so a long repository path — every capture's scratch repo, and
/// plenty of real checkouts — cut the header to `⚠ REBASING step 3 of 7 — …`
/// and the file git stopped on was gone: the urgent half cut for the half
/// that does not change (m5/16, review C1, the `160x48` capture on `main`).
/// A path with an `…` in it still says where you are, so the path is what
/// pays.
#[test]
fn a_long_location_is_cut_before_the_banner_is() {
    let rows = linear(2);
    let mut long = view(&rows, WalkView::Complete { rows: 2 }, 0);
    long.location = Some(sanitize_line(
        "/media/archive/work/customers/acme/services/billing-api",
    ));
    let banner = rebase_banner(Some((3, 7)), Some("src/auth/token.rs"));
    long.banner_op = Some(banner.clone());

    for columns in [80u16, 100, 120, 160] {
        let header = text(&draw(&long, columns, 6))[0].clone();
        let whole = banner_line(&banner);
        assert!(
            header.contains(&whole),
            "at {columns} columns the banner was cut instead of the path: \
             {header:?} does not hold {whole:?}"
        );
    }

    // And at the width where something has to give, the `…` is on the
    // path. (From 100 columns the whole header fits and nothing is cut —
    // it used to have to give up to 120, because the banner named the
    // conflicted file as well as the state; m5/20 item 1.)
    let header = text(&draw(&long, 80, 6))[0].clone();
    assert!(
        header.contains('\u{2026}'),
        "at 80 columns the path was not the thing cut: {header:?}"
    );
}

/// The operation banner costs **no** row under the header, at any width,
/// and reserves no slot for anything either: the row right under the header
/// is the first graph row, banner or no banner.
///
/// It used to draw `[ Continue ] [ Skip ] [ Abort ]` there from 160 columns
/// — mouse targets nothing answers a click on, a row of graph paid for
/// three verbs the hint bar already teaches on keys that work at every
/// width (m5/16, review C1).
#[test]
fn the_operation_banner_costs_no_row_under_the_header() {
    let rows = linear(2);
    let plain = view(&rows, WalkView::Complete { rows: 2 }, 0);
    let mut with_banner = plain.clone();
    with_banner.banner_op = Some(rebase_banner(Some((3, 7)), Some("src/auth.rs")));

    for columns in [80u16, 120, 160, 200] {
        let banner = text(&draw(&with_banner, columns, 10));
        let none = text(&draw(&plain, columns, 10));
        assert!(
            !banner[1].contains("Continue"),
            "at {columns} columns: {:?}",
            banner[1]
        );
        assert_eq!(
            banner[1], none[1],
            "at {columns} columns the banner took a graph row from under the header"
        );
    }
}

// --- CONFLICTS section (m5-history-and-ghost-preview/04) ---

/// `WorkingTreeSection::Unmerged`'s heading word is `CONFLICTS` — the
/// section drawn first, and the row text (words, not a letter and counts),
/// are asserted in `detail_pane::tests::conflicts_section_draws_first_with_words_not_a_letter`
/// and `detail_pane::files::tests::an_unmerged_row_shows_how_as_words_not_a_letter_and_counts`.
#[test]
fn unmerged_sections_heading_word_is_conflicts() {
    use crate::detail::WorkingTreeSection;
    assert_eq!(WorkingTreeSection::Unmerged.title(), "CONFLICTS");
}

/// Not an assertion — dumps the banner at three widths, so the header's
/// operation banner can be judged by eye. The widths are three sizes of
/// terminal, **not** three tiers: `banner_line` takes no width and there is
/// one wording (m5/20 item 3). Run by hand to regenerate: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft-ui graph_screen::tests::capture_for_docs_shots`.
///
/// The long location is deliberate: it is what pushed the banner off the
/// header before the room was reserved for it first (m5/16, review C1).
#[test]
// The body is one capture header and four `draw` calls, not logic — the same
// exemption `capture_hint_bar_for_docs_shots` below carries for the same
// reason. The caption grew when it stopped describing behaviour the code no
// longer has (m5/24).
#[allow(clippy::too_many_lines)]
fn capture_for_docs_shots() {
    let mut shots = graft_fixtures::Captures::new();
    let rows = linear(2);
    let mut with_banner = view(&rows, WalkView::Complete { rows: 2 }, 0);
    with_banner.banner_op = Some(rebase_banner(Some((3, 7)), Some("src/auth/token.rs")));

    let mut out = String::new();
    out.push_str(
        "# docs/shots/m5-ui-banner.txt — operation banner (rebase/merge/conflict)\n\
         #\n\
         # What right looks like: the header's informational tail is\n\
         # replaced, while an operation is in progress, by the state and\n\
         # nothing else — `REBASING step 3 of 7`.\n\
         #\n\
         # Three things it does not say (m5/20, all three the user's):\n\
         # no `⚠` — a stopped merge is a state to finish, not a failure;\n\
         # no conflicted filename and no `(+N)` — the changes pane already\n\
         # lists every conflicted file under `CONFLICTS (n)`, and a 24-row\n\
         # screen that says the same fact twice has spent a row on nothing;\n\
         # and no width tiers — `banner_line` takes no width, so the same\n\
         # repository in the same state reads the same in all three blocks\n\
         # below. They are three sizes of terminal, not three wordings.\n\
         #\n\
         # It costs no row: the line under the header is the first graph row\n\
         # at every width. There is no `[ Continue ] [ Skip ] [ Abort ]` row\n\
         # any more — nothing answered a click on it and it cost a graph row\n\
         # to teach three keys the hint bar already teaches (m5/16).\n\
         #\n\
         # The last block is the point: a long repository path does not cut\n\
         # the banner. The banner's room comes out of the header first, so\n\
         # the *path* is what an `…` would land on — and at 120 columns,\n\
         # against a banner this short, it no longer has to: the whole path\n\
         # and the whole banner are both there.\n\
         \n",
    );
    let mut long_path = with_banner.clone();
    long_path.location = Some(sanitize_line(
        "/media/archive/work/customers/acme/services/billing-api",
    ));
    for (label, w, view) in [
        ("80x10", 80u16, &with_banner),
        ("120x10", 120, &with_banner),
        ("160x10", 160, &with_banner),
        (
            "a long path does not cut the banner, 120x10",
            120,
            &long_path,
        ),
    ] {
        out.push_str(&format!("=== {label} ===\n"));
        for line in text(&draw(view, w, 10)) {
            if !line.trim().is_empty() {
                out.push_str(line.trim_end());
                out.push('\n');
            }
        }
        out.push('\n');
    }
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m5-ui-banner.txt"
        ),
        out,
    );
    shots.finish();
}

// --- Hint bar captures (`hint-bar-redesign`, ADR-0037 §4) ---

/// Not an assertion — dumps the footer's hint bar for a marked row at the
/// three breakpoints the width rule turns on (80, 120, 160 cols), so it can
/// be judged by eye against the design note's mockup.
///
/// Only the marked context: a marker comes from the Mistake Radar, which
/// needs a corpus built to trip it, so this one is a hand-built fixture
/// drawn straight into a `TestBackend`. The other three contexts the note
/// mocks up — a branch row, a plain row, the Uncommitted-changes row, and
/// the changes pane besides — are captured from a real repository through
/// the real app instead (`graft::it::hint_bar::capture_hint_bar_for_docs_shots`),
/// because a capture of a view model somebody set by hand cannot show
/// whether the app picks that view model (m5-03 review, Major 5).
///
/// Run by hand to regenerate: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft-ui graph_screen::tests::capture_hint_bar_for_docs_shots`.
#[test]
#[allow(clippy::too_many_lines)] // one hand-built fixture plus its capture header, not logic
fn capture_hint_bar_for_docs_shots() {
    let mut shots = graft_fixtures::Captures::new();
    // A corpus shape with a branch, a marked merge and a dirty tree
    // (scope item 7): three ordinary commits, then a merge carrying a
    // Mistake Radar marker, then the row a local branch badge sits on.
    let mut rows = linear(5);
    rows[1].commit.parents = vec![rows[2].commit.id, rows[3].commit.id];
    let rows = layout(
        &rows
            .iter()
            .map(|row| row.commit.clone())
            .collect::<Vec<_>>(),
        Seeds::default(),
    );

    let branch_badge = Badge {
        name: sanitize_line("feature/hints"),
        kind: BadgeKind::Branch {
            identity: BranchIdentity::of_name(b"feature/hints"),
            current: false,
            tracking: Tracking::Untracked,
        },
    };

    let scenarios: [(&str, HintContext, usize, bool); 1] = [(
        "marked",
        HintContext::Marked(Box::new(HintContext::PlainRow)),
        1,
        false,
    )];

    for (name, context, cursor, badge) in scenarios {
        let mut scenario = view(&rows, WalkView::Complete { rows: 5 }, cursor);
        scenario.hint_context = context;
        if badge {
            scenario.rows[cursor].badges.push(branch_badge.clone());
        }
        if name == "marked" {
            scenario.rows[cursor].marker = Some(MarkerView {
                severity: Severity::Warning,
            });
        }
        for (label, columns, term_rows) in [
            ("80x24", 80u16, 24u16),
            ("120x40", 120, 40),
            ("160x48", 160, 48),
        ] {
            let mut out = format!(
                "# m5-hints-{name}-{label}.txt: {columns}x{term_rows}, the GraphScreen widget\n\
                 # drawn into ratatui's TestBackend from a view model built by hand —\n\
                 # HintContext::{context:?} on a five-row corpus. Not a pty capture and\n\
                 # not the app: no colour is recorded (CLAUDE.md rule 5, colour ships\n\
                 # unverified), and the context is set here rather than chosen by\n\
                 # `view::rows::hint_context`. The contexts a real repository can reach\n\
                 # are captured through the whole app instead — m5-hints-branch-*,\n\
                 # -plain-*, -uncommitted-*, -changes-*. Fixture:\n\
                 # capture_hint_bar_for_docs_shots in\n\
                 # crates/graft-ui/src/graph_screen/tests.rs (asserted;\n\
                 # regenerate on purpose).\n\
                 #\n\
                 # Judge by eye: the footer's last row reads the marked row's hints for\n\
                 # this width — `. explain` leads (the marker's own key), then the row's\n\
                 # keys, filled from the most relevant end: each hint takes the best rung\n\
                 # that still leaves room for the hints ahead of it — full, short, then\n\
                 # bare key — taken per hint, never as a step the whole list takes\n\
                 # together, and a hint that will not fit even at its bare key is\n\
                 # dropped (ADR-0039's third amendment, 2026-09-21). The three most\n\
                 # relevant never fall below their floor rung, which is their short rung\n\
                 # where the user's table names one and their full verb where it does\n\
                 # not, and the bar is never all hidden — never a key that does nothing.\n\
                 \n",
                context = scenario.hint_context,
            );
            for line in text(&draw(&scenario, columns, term_rows)) {
                out.push_str(line.trim_end());
                out.push('\n');
            }
            shots.check(
                format!(
                    concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../docs/shots/m5-hints-{}-{}.txt"
                    ),
                    name, label
                ),
                out,
            );
        }
    }
    shots.finish();
}

/// The shrink ladder, for the user's eye (m5/24 and m5/28, ADR-0039;
/// CLAUDE.md rule 6 — nobody here ticks "looks right").
///
/// Writes `docs/shots/m5-hints-ladder.txt`: the same six contexts at 80,
/// 120 and 160 cells of bar, as built. The A/B comparison that used to
/// follow them is gone — the variants were the question, the user answered
/// it (variant B, the doorway leading the bare keys), and as-built is now B,
/// so a comparison would be comparing the build against itself.
///
/// Run by hand to regenerate: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft-ui graph_screen::tests::capture_the_ladder_for_docs_shots`.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "six contexts, three widths and the header each block explains itself with, not logic"
)]
fn capture_the_ladder_for_docs_shots() {
    use super::hints::{Divider, hints_for, hints_text};

    let mut shots = graft_fixtures::Captures::new();

    let cases: [(&str, HintContext, Divider); 6] = [
        (
            "the cursor is on the uncommitted-changes row",
            HintContext::WorkingTreeRow,
            Divider::Hidden,
        ),
        (
            "the cursor is on the tip, which carries the current branch's badge",
            HintContext::BranchRow { current: true },
            Divider::Hidden,
        ),
        (
            "the cursor is on another branch's tip",
            HintContext::BranchRow { current: false },
            Divider::Hidden,
        ),
        (
            "the cursor is on an ordinary commit row",
            HintContext::PlainRow,
            Divider::Hidden,
        ),
        (
            "the changes pane has focus, the divider draggable",
            HintContext::ChangesPane {
                unmerged: false,
                live: None,
                tab: TabWord::Graph,
            },
            Divider::Shown,
        ),
        (
            "the undo journal browser has focus",
            HintContext::Journal,
            Divider::Hidden,
        ),
    ];
    let widths = [80u16, 120, 160];

    // A ruler exactly `columns` cells wide, so the room the bar was given is
    // visible rather than asserted.
    let ruler = |columns: u16| {
        let label = format!(" {columns} cells of bar ");
        let rule = usize::from(columns).saturating_sub(label.width() + 2);
        let left = rule / 2;
        format!("[{}{label}{}]", "·".repeat(left), "·".repeat(rule - left))
    };

    let mut out = String::new();
    out.push_str(
        "# docs/shots/m5-hints-ladder.txt — the hint bar's shrink ladder\n\
         # (ADR-0039, m5/24 and m5/28)\n\
         #\n\
         # What right looks like: the bar fills from the most relevant end. The\n\
         # three most relevant hints keep their whole verbs; every hint after them\n\
         # is drawn at its bare key until the whole context is on the bar, and only\n\
         # then do the tail's keys start gaining verbs, left to right. A hint that\n\
         # will not fit even at its bare key is dropped, and `. more` — the doorway\n\
         # — is never dropped and **sits at the head of the bare keys**, which is\n\
         # the end of the row when there are none.\n\
         #\n\
         # The question these rows were captured to answer is answered: a tail of\n\
         # naked letters after three labelled hints read as *leftover*, and the\n\
         # user chose variant B, which is what is built here. Every labelled item\n\
         # is now contiguous and the last of them announces the menu, so everything\n\
         # after `. more` reads as keys the menu will spell out — which is what\n\
         # they are. That resolves `space` and `j/k` in the tail too: the region is\n\
         # labelled even though its members are not. It cost no cells and no glyph.\n\
         # The A/B comparison that used to sit at the bottom of this file is gone\n\
         # with the question.\n\
         #\n\
         # The bottom of the ladder, which these widths are all far above:\n\
         # floor → `. more` → `.` → nothing. The bar never draws a fragment of\n\
         # itself at any width. The narrowest bar anything reachable actually has\n\
         # is 48 cells and the tightest context's floor fits in 38, so nothing\n\
         # below is near it (ADR-0039).\n\
         #\n\
         # How this was made: `hints_for(context, divider, columns)` rendered by\n\
         # `hints_text`, which is exactly the two calls `graph_screen::footer` makes\n\
         # (`footer.rs`, the `(None, None, None, None)` arm). It is the bar's own\n\
         # row and nothing around it — the whole screen at these widths is in\n\
         # `m5-hints-<context>-<size>.txt` beside this file. `columns` is the room\n\
         # the *bar* has, not the screen's width: with a detail pane beside the\n\
         # graph a 160-column terminal hands the bar about 92 cells (ADR-0039's\n\
         # second amendment). No colour here — TestBackend records a character\n\
         # grid, so the muted grey the footer draws in is unverified\n\
         # (CLAUDE.md rule 5).\n\
         #\n\
         # The short rung is visible where the room runs out mid-hint: `F3 rend`\n\
         # below. Every short form is a prefix of the full verb, the user's own\n\
         # word, restated 2026-09-21 (m5/27) — a verb it does not name (`stash`,\n\
         # `merge`, `rebase`, `compare`, `commit` among them) has no short rung at\n\
         # all and keeps its whole word at every width.\n\
         \n",
    );

    for columns in widths {
        out.push_str(&format!("=== as built — {columns} cells of bar ===\n\n"));
        for (what, context, divider) in &cases {
            out.push_str(&format!("--- {what}\n"));
            out.push_str(&ruler(columns));
            out.push('\n');
            out.push_str(&hints_text(&hints_for(context, *divider, columns)));
            out.push_str("\n\n");
        }
    }

    out.push_str(
        "=== `rev` — revert's only home, the undo journal browser ===\n\
         #\n\
         # `revert` keeps its short form under the m5/27 rule — `rev` is a clean\n\
         # prefix, not a squeeze or a substitution. But `rev` is also a git word\n\
         # meaning *revision* (`rev-parse`, `rev-list`), and that clash is the\n\
         # user's to judge, not the builder's — this section exists only to put it\n\
         # in front of him.\n\
         #\n\
         # It cannot be shown at 80, 120 or 160 cells: the journal browser's whole\n\
         # list is `⏎ revert  esc close`, two hints, and that fits whole (plus\n\
         # `. more`) from about 28 cells of bar — every width in the three-width\n\
         # matrix above is past that, so the journal row there always reads\n\
         # `⏎ revert  esc close  . more` in full, never `rev`. Below is the\n\
         # narrowest room `rev` actually appears in, computed the same way as\n\
         # every row above: `hints_for`, nothing hand-set.\n\
         \n",
    );
    for columns in [16u16, 20, 24, 27] {
        let hints = hints_for(&HintContext::Journal, Divider::Hidden, columns);
        out.push_str(&format!(
            "--- the undo journal browser has focus, {columns} cells\n"
        ));
        out.push_str(&ruler(columns));
        out.push('\n');
        out.push_str(&hints_text(&hints));
        out.push_str("\n\n");
    }

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/m5-hints-ladder.txt");
    shots.check(&path, &out);
    shots.finish();
}

/// The hit-test and the renderer are the same code, and this is the test
/// that says so: draw a row, then read every cell of it back and check that
/// each one belongs to the badge [`badge_spans`] names.
///
/// `graft`'s mouse hit-test asks `badge_spans` which badge a column is on
/// (`graft::view::hit::badge_index_at`). If that ever stopped agreeing with
/// what was drawn, a drag would pick up the badge next to the one under the
/// pointer — silently, because both are branches and both look plausible.
/// ADR-0040's lesson, one layer down.
#[test]
fn badge_spans_match_the_drawn_cells() {
    let rows = linear(3);
    let badges = three_badges();
    // Wide enough for all three, and narrow enough that the last is clipped
    // — the clipped case is exactly where a mirrored packing would drift.
    for width in [40_u16, 60, 100] {
        for icons in [Icons::Plain, Icons::Nerd] {
            let mut screen = view(&rows, WalkView::Complete { rows: 3 }, 0);
            screen.icons = icons;
            screen.rows[0].badges.clone_from(&badges);
            check_one_row(&screen, width, icons);
        }
    }
}

/// A branch with an ahead/behind chip, a remote and a tag: three badges of
/// three different widths, so the packing has something to get wrong.
fn three_badges() -> Vec<Badge> {
    vec![
        Badge {
            name: sanitize_line("main"),
            kind: BadgeKind::Branch {
                identity: BranchIdentity::of_name(b"main"),
                current: true,
                tracking: Tracking::Apart {
                    ahead: 2,
                    behind: 1,
                },
            },
        },
        Badge {
            name: sanitize_line("origin/main"),
            kind: BadgeKind::Remote {
                identity: BranchIdentity::of_name(b"origin/main"),
            },
        },
        Badge {
            name: sanitize_line("v2.3.0"),
            kind: BadgeKind::Tag,
        },
    ]
}

/// Draw `screen` at `width` and check every span against the cells drawn.
fn check_one_row(screen: &GraphView<'_>, width: u16, icons: Icons) {
    {
        {
            let terminal = draw(screen, width, 12);
            let drawn = text(&terminal);
            let area = Rect {
                x: 0,
                y: 0,
                width,
                height: 12,
            };
            let (entry, columns) =
                row_at(screen, area, 1).expect("the first commit row is drawn at y = 1");
            let line = &drawn[1];
            let cells: Vec<&str> = line.split("").filter(|cell| !cell.is_empty()).collect();

            let spans = badge_spans(entry, &columns, icons);
            assert!(!spans.is_empty(), "{width} columns, {icons:?}: no badges");
            for (span, index) in &spans {
                let drawn_here: String = cells[usize::from(span.start)..usize::from(span.end)]
                    .concat()
                    .trim_end()
                    .to_owned();
                let expected = entry.badges[*index].text(icons);
                // A badge cut off at the pane's edge is drawn elided
                // (`draw_cell`'s `fit`), so the last one on a narrow row
                // reads as a prefix plus `…`. The span is still the badge's
                // cells, which is what this is checking.
                let drawn_here = drawn_here.trim_end_matches('…');
                assert!(
                    expected.starts_with(drawn_here),
                    "{width} columns, {icons:?}: cells {span:?} read {drawn_here:?}, \
                     but badge {index} is {expected:?}"
                );
                assert!(
                    !drawn_here.is_empty(),
                    "{width} columns, {icons:?}: badge {index} claims {span:?} and drew nothing"
                );
            }
            // Non-overlapping and in order, which is what makes "the column
            // is in this span" a single answer.
            for pair in spans.windows(2) {
                assert!(
                    pair[0].0.end <= pair[1].0.start,
                    "{width} columns, {icons:?}: {:?} overlaps {:?}",
                    pair[0].0,
                    pair[1].0
                );
            }
        }
    }
}

/// A row with no badges claims no cells — so a click on a bare subject
/// picks nothing up.
#[test]
fn a_row_without_badges_has_no_spans() {
    let rows = linear(3);
    let screen = view(&rows, WalkView::Complete { rows: 3 }, 0);
    let area = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 12,
    };
    let (entry, columns) = row_at(&screen, area, 1).expect("a commit row");
    assert!(badge_spans(entry, &columns, Icons::Plain).is_empty());
}

/// Off the commit rows there is no row at all: the header, the footer and
/// anything past the end of the history answer `None`.
#[test]
fn row_at_is_none_off_the_body() {
    let rows = linear(2);
    let screen = view(&rows, WalkView::Complete { rows: 2 }, 0);
    let area = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 12,
    };
    assert!(row_at(&screen, area, 0).is_none(), "the header");
    assert!(row_at(&screen, area, 1).is_some(), "the first commit");
    assert!(row_at(&screen, area, 2).is_some(), "the second");
    assert!(row_at(&screen, area, 3).is_none(), "past the history");
    assert!(row_at(&screen, area, 11).is_none(), "the footer");
}

// ---------------------------------------------------------------------------
// `every_context` cannot quietly miss a shape (m5-history-and-ghost-preview/33,
// review M3)
// ---------------------------------------------------------------------------

/// Shapes [`coverage_slot`] can answer with — one per variant and per
/// boolean field combination (54 → 63 in m6-conflicts/50: the confirm box
/// once per `ConfirmYes`).
const COVERAGE_SLOTS: usize = 79;

/// The slot `context` takes in [`every_context_reaches_every_shape`]'s tally.
///
/// **Exhaustive on the enum and on every boolean field**, which is the whole
/// point: a variant added to `HintContext`, or a field combination added to
/// one, stops this crate compiling until it is given a slot here — and then
/// the test below fails until `every_context` actually produces one.
///
/// `Marked` and `PaneFocused` are tallied by the wrapper alone. What they
/// wrap is not a shape of its own: `every_context` builds them by wrapping
/// the plain list, so a plain shape that reaches the list reaches its
/// wrapped forms too.
fn coverage_slot(context: &HintContext) -> usize {
    match context {
        HintContext::WorkingTreeRow => 0,
        HintContext::BranchRow { current: false } => 1,
        HintContext::BranchRow { current: true } => 2,
        HintContext::PlainRow => 3,
        // Both fields are plain `bool`s, so the arm stays exhaustive.
        // `merge` is `!skippable` in every form a live operation makes
        // (m6-conflicts/70), so it takes no slot of its own.
        HintContext::Banner {
            skippable,
            conflicted,
            merge: _,
        } => 4 + 2 * usize::from(*skippable) + usize::from(*conflicted),
        HintContext::Preview => 8,
        HintContext::ChangesPane { .. } => changes_pane_slot(context),
        HintContext::Journal => 11,
        HintContext::Stash => 12,
        HintContext::Dragging => 13,
        HintContext::Marked(_) => 14,
        HintContext::PaneFocused {
            opens: false,
            blamable: false,
            ..
        } => 15,
        HintContext::PaneFocused {
            opens: true,
            blamable: false,
            ..
        } => 16,
        // m7-power-tools/20: `B blame` on the pane (73; the diff focused —
        // `catalogue::pane_focused_form` says why not the file list), the
        // blame place's four flag pairs (74–77), the way back to it (78).
        HintContext::PaneFocused { blamable: true, .. } => 73,
        HintContext::Blame {
            finding,
            ignore_file,
        } => 74 + 2 * usize::from(*finding) + usize::from(*ignore_file),
        HintContext::BackToBlame(_) => 78,
        HintContext::Map => 17,
        HintContext::PaneOverOverlay { opens: false } => 18,
        HintContext::PaneOverOverlay { opens: true } => 19,
        HintContext::Resolver { .. } => resolver_slot(context),
        HintContext::ConfirmBox { yes } => confirm_slot(*yes),
        HintContext::NamePrompt { toggle: false } => 26,
        HintContext::NamePrompt { toggle: true } => 27,
        HintContext::Credential => 28,
        // m6-conflicts/52: each `c` × one `⇥` from the banner or not. The
        // form every live bar had before 52 keeps its slot, 29; the other
        // five are 49–53.
        HintContext::OperationLive {
            c, tab_to_banner, ..
        } => match 2 * live_c_slot(*c) + usize::from(!*tab_to_banner) {
            0 => 29,
            other => 48 + other,
        },
        // Only the files focus reads `expandable`.
        HintContext::Finish {
            focus: crate::finish_summary::SummaryFocus::Files,
            expandable,
        } => 32 + usize::from(*expandable),
        HintContext::Finish {
            focus: crate::finish_summary::SummaryFocus::Subject,
            ..
        } => 34,
        HintContext::Finish {
            focus: crate::finish_summary::SummaryFocus::Body,
            ..
        } => 35,
    }
}

/// [`coverage_slot`] for the changes pane (m6-conflicts/70 split it out).
fn changes_pane_slot(context: &HintContext) -> usize {
    match context {
        // Nothing live: `⇥ graph` from the diff keeps 9–10; `⇥ diff` from
        // the file list is 63–64 (m6-conflicts/70). `⇥ banner` with nothing
        // live is not a screen, and shares 9–10.
        HintContext::ChangesPane {
            unmerged,
            live: None,
            tab: TabWord::Graph | TabWord::Banner,
        } => 9 + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: None,
            tab: TabWord::Diff,
        } => 63 + usize::from(*unmerged),
        // m6-conflicts/52: each `c` an operation can make, over each
        // `unmerged`: from the file list, `⇥ diff` (43–48); from the diff
        // with a banner, `⇥ banner` (65–70); a bisect's `⇥ graph` (71–72,
        // which `c finish`/`c continue`, never bannerless, share).
        HintContext::ChangesPane {
            unmerged,
            live: Some(c),
            tab: TabWord::Diff,
        } => 43 + 2 * live_c_slot(*c) + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: Some(c),
            tab: TabWord::Banner,
        } => 65 + 2 * live_c_slot(*c) + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: Some(_),
            tab: TabWord::Graph,
        } => 71 + usize::from(*unmerged),
        _ => unreachable!("only called on a `ChangesPane`"),
    }
}

/// [`coverage_slot`] for the confirm box: 25 with no `y`, then one per
/// acting button `y` takes, 54–62 (m6-conflicts/50).
fn confirm_slot(yes: Option<super::ConfirmYes>) -> usize {
    yes.map_or(25, |yes| {
        54 + super::ConfirmYes::ALL
            .iter()
            .position(|each| *each == yes)
            .expect("ALL names every ConfirmYes")
    })
}

/// A [`LiveC`](super::LiveC)'s place in [`coverage_slot`]'s live ranges.
fn live_c_slot(c: super::LiveC) -> usize {
    match c {
        super::LiveC::Finish => 0,
        super::LiveC::Continue => 1,
        super::LiveC::Refuses => 2,
    }
}

/// [`coverage_slot`] for the resolver: done (30–31), document (20–23) or
/// card (24), and the same three shapes with `c` offered
/// (m6-conflicts/31): 36–37, 38–41, 42.
fn resolver_slot(context: &HintContext) -> usize {
    let HintContext::Resolver {
        base,
        restart,
        whole_file,
        done,
        finishable,
        ..
    } = *context
    else {
        panic!("not a resolver context: {context:?}");
    };
    match (done, whole_file, finishable) {
        (true, _, false) => 30 + usize::from(!restart),
        (true, _, true) => 36 + usize::from(!restart),
        (false, false, false) => 20 + usize::from(base) + 2 * usize::from(restart),
        (false, false, true) => 38 + usize::from(base) + 2 * usize::from(restart),
        (false, true, false) => 24,
        (false, true, true) => 42,
    }
}

/// `every_context`'s list is written by hand — Rust cannot enumerate a
/// type's values, and `Marked`/`PaneFocused` are recursive, so there is no
/// finite set to derive without choosing a depth by hand anyway. What the
/// compiler *can* do is refuse: [`coverage_slot`] matches exhaustively, so a
/// new shape cannot compile until it is named, and this test then fails
/// until the list actually produces one.
///
/// So adding a variant is still a human's edit. It is no longer a human's
/// **omission** (m5/33, review M3 — the gap row and the ticket notes used to
/// claim the compiler forced the list itself, which it does not).
#[test]
fn every_context_reaches_every_shape() {
    let mut seen = [false; COVERAGE_SLOTS];
    for context in every_context() {
        seen[coverage_slot(&context)] = true;
    }

    let missed: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter(|(_, hit)| !**hit)
        .map(|(slot, _)| slot)
        .collect();
    assert!(
        missed.is_empty(),
        "`every_context` never produces slot(s) {missed:?} of `coverage_slot` — a shape \
         the sweeps that read it will never see"
    );
}
