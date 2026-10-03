//! The empty state as drawn (mock-ups §1.15), at the two sizes the ticket
//! names. Colour is unverified (CLAUDE.md rule 5) except the cursor row's
//! legibility, which is measured.

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::{EmptyPrompt, EmptyState, EmptyStateView, RecentRow, bar_keys, recent_age};
use crate::theme::{Theme, ThemeKind};

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &crate::test_support::terminal(ColorDepth::TrueColor, false),
    )
}

fn row(name: &str, path: &str, age: &str) -> RecentRow {
    RecentRow {
        name: sanitize_line(name),
        path: sanitize_line(path),
        age: sanitize_line(age),
    }
}

fn with_recents() -> EmptyStateView {
    EmptyStateView {
        folder: sanitize_line("~/code"),
        missing: false,
        recent: vec![
            row("payments-service", "~/code/payments-service", "yesterday"),
            row("web-dashboard", "~/code/web-dashboard", "3 days ago"),
            row("dotfiles", "~/dotfiles", "last week"),
        ],
        cursor: 0,
        prompt: None,
        problem: None,
    }
}

fn no_recents() -> EmptyStateView {
    EmptyStateView {
        recent: Vec::new(),
        ..with_recents()
    }
}

fn draw(view: &EmptyStateView, columns: u16, rows: u16) -> (Buffer, Vec<String>) {
    let area = Rect::new(0, 0, columns, rows);
    let mut buffer = Buffer::empty(area);
    let theme = theme();
    (&EmptyState {
        view,
        theme: &theme,
        ascii: false,
    })
        .render(area, &mut buffer);
    let lines = (0..rows)
        .map(|y| {
            (0..columns)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    (buffer, lines)
}

#[test]
fn rows_are_as_the_mockup_at_both_sizes() {
    for (columns, rows) in [(80, 24), (120, 36)] {
        let (_, lines) = draw(&with_recents(), columns, rows);
        let text = lines.join("\n");
        assert_eq!(lines[0], " graft · no repository here", "{text}");
        assert_eq!(lines[1], " ~/code", "{text}");
        assert_eq!(lines[2], "", "{text}");
        assert_eq!(lines[3], "  o  Open…     i  Init", "{text}");
        assert_eq!(lines[5], "  RECENT", "{text}");
        assert_eq!(
            lines[6], "  ▸ payments-service    ~/code/payments-service     yesterday",
            "{text}"
        );
        assert_eq!(
            lines[7], "    web-dashboard       ~/code/web-dashboard        3 days ago",
            "{text}"
        );
        assert_eq!(
            lines[8], "    dotfiles            ~/dotfiles                  last week",
            "{text}"
        );
        let rows = usize::from(rows);
        assert!(lines[rows - 2].chars().all(|c| c == '─'), "{text}");
        assert_eq!(
            lines[rows - 1],
            " o open… · i init · ⏎ open the selected · q quit",
            "{text}"
        );
        assert!(
            !text.contains("Clone") && !text.contains("tour"),
            "c/t drawn: {text}"
        );
    }
}

#[test]
fn an_empty_list_draws_no_recent_block_and_no_enter_hint() {
    let (_, lines) = draw(&no_recents(), 80, 24);
    let text = lines.join("\n");
    assert!(!text.contains("RECENT"), "{text}");
    assert_eq!(lines[23], " o open… · i init · q quit", "{text}");
}

#[test]
fn a_missing_folder_says_so_and_offers_no_init() {
    let view = EmptyStateView {
        folder: sanitize_line("~/code/newthing"),
        missing: true,
        ..no_recents()
    };
    let (_, lines) = draw(&view, 80, 24);
    let text = lines.join("\n");
    assert_eq!(lines[1], " ~/code/newthing · no such folder", "{text}");
    assert_eq!(lines[3], "  o  Open…", "{text}");
    assert_eq!(lines[23], " o open… · q quit", "{text}");
    assert!(!text.contains("init") && !text.contains("Init"), "{text}");
}

#[test]
fn the_open_prompt_sits_under_the_list_with_its_own_keys() {
    let view = EmptyStateView {
        prompt: Some(EmptyPrompt::Open {
            text: sanitize_line("~/code/"),
            cursor: 7,
        }),
        ..with_recents()
    };
    let (_, lines) = draw(&view, 80, 24);
    let text = lines.join("\n");
    assert!(lines[10].starts_with("  ╭ open ─"), "{text}");
    assert_eq!(
        lines[11].trim_end_matches(['│', ' ']),
        "  │  open › ~/code/▏",
        "{text}"
    );
    assert!(lines[13].starts_with("  ╰─"), "{text}");
    assert_eq!(lines[23], " ⏎ open · ⇥ complete · esc cancel", "{text}");
}

#[test]
fn the_init_confirm_names_the_folder_and_offers_y() {
    let view = EmptyStateView {
        prompt: Some(EmptyPrompt::ConfirmInit),
        ..no_recents()
    };
    let (_, lines) = draw(&view, 120, 36);
    let text = lines.join("\n");
    assert!(lines[5].starts_with("  ╭ init ─"), "{text}");
    assert!(
        lines[6].contains("│  Make ~/code a Git repository?"),
        "{text}"
    );
    assert_eq!(lines[35], " y init · esc keep it", "{text}");
}

/// A missing folder too long for the row loses its start, never
/// `· no such folder`: the reason is what the row is for.
#[test]
fn a_long_missing_folder_keeps_no_such_folder() {
    let view = EmptyStateView {
        folder: sanitize_line("/srv/builds/nightly/workspace-0042/sandboxes/projects/newthing"),
        missing: true,
        ..no_recents()
    };
    let (_, lines) = draw(&view, 60, 24);
    let text = lines.join("\n");
    assert!(lines[1].starts_with(" …"), "{text}");
    assert!(lines[1].ends_with("/newthing · no such folder"), "{text}");
}

/// A folder too long for the box loses its start, not the question: what
/// `y` answers stays readable, and the folder's own name (its end) stays.
#[test]
fn a_long_folder_in_the_init_confirm_keeps_the_question() {
    let view = EmptyStateView {
        folder: sanitize_line("/srv/builds/nightly/workspace-0042/sandboxes/projects/long/code"),
        prompt: Some(EmptyPrompt::ConfirmInit),
        ..no_recents()
    };
    let (_, lines) = draw(&view, 80, 24);
    let text = lines.join("\n");
    let question = lines[6].trim_end_matches(['│', ' ']);
    assert!(question.starts_with("  │  Make …"), "{text}");
    assert!(question.ends_with("/code a Git repository?"), "{text}");
}

#[test]
fn a_problem_replaces_the_bar_row() {
    let view = EmptyStateView {
        problem: Some(sanitize_line("open: /tmp/x is not inside a git repository")),
        ..with_recents()
    };
    let (_, lines) = draw(&view, 80, 24);
    assert_eq!(lines[23], " open: /tmp/x is not inside a git repository");
}

/// The cursor row is painted exactly as the graph's: the `selection` fill,
/// every glyph on it at 4.5:1 or better (ADR-0058 §3).
#[test]
fn the_cursor_row_is_on_the_selection_fill_and_legible() {
    let view = EmptyStateView {
        cursor: 1,
        ..with_recents()
    };
    let (buffer, _) = draw(&view, 80, 24);
    let checked = crate::test_support::assert_legible_on_selection(&buffer, &theme());
    assert!(checked > 20, "the cursor row was checked ({checked} cells)");
    let fill = crate::paint::color(theme().token(crate::theme::Token::Selection));
    assert_eq!(buffer[(4, 7)].bg, fill, "the second row is the cursor row");
    assert_ne!(buffer[(4, 6)].bg, fill, "the first is not");
    assert_eq!(buffer[(2, 7)].symbol(), "▸");
}

#[test]
fn bar_keys_follow_the_form() {
    let keys = |view: &EmptyStateView| -> Vec<&str> {
        bar_keys(view).into_iter().map(|(key, _)| key).collect()
    };
    assert_eq!(keys(&with_recents()), ["o", "i", "⏎", "q"]);
    assert_eq!(keys(&no_recents()), ["o", "i", "q"]);
}

#[test]
fn ages_read_as_the_mockup() {
    let day = 86_400;
    let now = 1_800_000_000;
    assert_eq!(recent_age(now - 60, now), "today");
    assert_eq!(recent_age(now - day, now), "yesterday");
    assert_eq!(recent_age(now - 3 * day, now), "3 days ago");
    assert_eq!(recent_age(now - 8 * day, now), "last week");
    assert_eq!(recent_age(now - 15 * day, now), "2 weeks ago");
    assert_eq!(recent_age(now - 40 * day, now), "last month");
    assert_eq!(recent_age(now - 100 * day, now), "3 months ago");
    assert_eq!(recent_age(now - 400 * day, now), "last year");
    assert_eq!(recent_age(now - 800 * day, now), "2 years ago");
    assert_eq!(recent_age(now + day, now), "today");
}
