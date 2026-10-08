//! The empty state as drawn (mock-ups §1.15), at the two sizes the ticket
//! names. Colour is unverified (CLAUDE.md rule 5) except the cursor row's
//! legibility, which is measured.

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::{
    CloneShown, CloneView, EmptyPrompt, EmptyState, EmptyStateView, RecentRow, bar_keys,
    first_launch_words, recent_age,
};
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
        first_launch: false,
        credential: false,
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
        keys: crate::keymap::Keymap::shipped(),
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
        assert_eq!(lines[3], "  o  Open…     i  Init     c  Clone…", "{text}");
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
            " o open… · i init · c clone… · ⏎ open the selected · q quit · ? keys",
            "{text}"
        );
        assert!(!text.contains("tour"), "t drawn: {text}");
    }
}

#[test]
fn an_empty_list_draws_no_recent_block_and_no_enter_hint() {
    let (_, lines) = draw(&no_recents(), 80, 24);
    let text = lines.join("\n");
    assert!(!text.contains("RECENT"), "{text}");
    assert_eq!(
        lines[23], " o open… · i init · c clone… · q quit · ? keys",
        "{text}"
    );
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
    assert_eq!(lines[23], " o open… · q quit · ? keys", "{text}");
    assert!(!text.contains("init") && !text.contains("Init"), "{text}");
    assert!(!text.contains("clone") && !text.contains("Clone"), "{text}");
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
    let keys = |view: &EmptyStateView| -> Vec<String> {
        bar_keys(view, crate::keymap::Keymap::shipped())
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    };
    assert_eq!(keys(&with_recents()), ["o", "i", "c", "⏎", "q", "?"]);
    assert_eq!(keys(&no_recents()), ["o", "i", "c", "q", "?"]);
}

fn clone_view(url: &str, into: &str, shown: CloneShown) -> EmptyStateView {
    EmptyStateView {
        prompt: Some(EmptyPrompt::Clone(CloneView {
            url: sanitize_line(url),
            cursor: url.chars().count(),
            into: sanitize_line(into),
            shown,
        })),
        ..with_recents()
    }
}

fn bar_line(view: &EmptyStateView) -> String {
    bar_keys(view, crate::keymap::Keymap::shipped())
        .into_iter()
        .map(|(key, label)| format!("{key} {label}"))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Rulings R6: the clone prompt in the open prompt's slot, `→ <into>` muted
/// under the URL, and each sub-state's bar.
#[test]
fn the_clone_prompt_draws_each_state_and_its_bar() {
    let typing = clone_view("file:///srv/x.git", "~/code/x", CloneShown::Typing);
    let (_, lines) = draw(&typing, 80, 24);
    let text = lines.join("\n");
    assert!(lines[10].starts_with("  ╭ clone ─"), "{text}");
    assert_eq!(
        lines[11].trim_end_matches(['│', ' ']),
        "  │  clone › file:///srv/x.git▏",
        "{text}"
    );
    assert_eq!(
        lines[12].trim_end_matches(['│', ' ']),
        "  │  → ~/code/x",
        "{text}"
    );
    assert!(lines[13].starts_with("  ╰─"), "{text}");
    assert_eq!(lines[23], " ⏎ clone · esc cancel", "{text}");
    assert_eq!(bar_line(&typing), "⏎ clone · esc cancel");

    let connecting = clone_view(
        "file:///srv/x.git",
        "~/code/x",
        CloneShown::Running { progress: None },
    );
    let (_, lines) = draw(&connecting, 80, 24);
    assert!(lines[12].contains("→ ~/code/x · connecting…"), "{lines:#?}");
    assert!(!lines[11].contains('▏'), "read-only: no cursor");
    assert_eq!(bar_line(&connecting), "esc cancels");
    let running = clone_view(
        "file:///srv/x.git",
        "~/code/x",
        CloneShown::Running {
            progress: Some(sanitize_line("Receiving objects 40%")),
        },
    );
    let (_, lines) = draw(&running, 80, 24);
    assert!(
        lines[12].contains("→ ~/code/x · Receiving objects 40%"),
        "{lines:#?}"
    );

    let ended = clone_view(
        "file:///srv/x.git",
        "~/code/x",
        CloneShown::Ended {
            reason: sanitize_line("clone failed: repository not found"),
        },
    );
    let (_, lines) = draw(&ended, 80, 24);
    assert!(
        lines[13].contains("│  clone failed: repository not found"),
        "{lines:#?}"
    );
    assert!(lines[14].starts_with("  ╰─"), "{lines:#?}");
    assert_eq!(bar_line(&ended), "⏎ retry · esc cancel");
}

/// Widths 60, 80, 120 (and a short screen): the prompt's rows never reach
/// the rule, and a long `→` path loses its start at a folder boundary,
/// keeping its tail.
#[test]
fn clone_prompt_fits_every_width() {
    let into = "~/work/clients/acme/platform/services/payments/billing-gateway-v2";
    for shown in [
        CloneShown::Typing,
        CloneShown::Running {
            progress: Some(sanitize_line("Receiving objects  40% (400/1000)")),
        },
        CloneShown::Ended {
            reason: sanitize_line("clone failed: could not resolve host"),
        },
    ] {
        for (columns, rows) in [(60, 24), (80, 24), (120, 36), (60, 12)] {
            let view = EmptyStateView {
                recent: (0..9)
                    .map(|i| row(&format!("r{i}"), "~/r", "today"))
                    .collect(),
                ..clone_view(
                    "https://example.com/acme/billing-gateway-v2.git",
                    into,
                    shown.clone(),
                )
            };
            let (_, lines) = draw(&view, columns, rows);
            let text = lines.join("\n");
            let rule = usize::from(rows) - 2;
            assert!(
                lines[rule].chars().all(|c| c == '─'),
                "{columns}x{rows}: {text}"
            );
            let bottom = lines
                .iter()
                .rposition(|line| line.starts_with("  ╰"))
                .unwrap_or_else(|| panic!("{columns}x{rows}: no box bottom\n{text}"));
            assert!(bottom < rule, "{columns}x{rows}: box over the rule\n{text}");
            let arrow = lines
                .iter()
                .find(|line| line.contains('→'))
                .unwrap_or_else(|| panic!("{columns}x{rows}: no → row\n{text}"));
            assert!(
                arrow.contains("billing-gateway-v2"),
                "{columns}x{rows}: {arrow:?}"
            );
            // Whole when it fits; else cut at the start, at a `/`.
            let whole = arrow.contains(into);
            assert!(
                whole || (arrow.contains("→ …/") && !arrow.contains("~/work")),
                "{columns}x{rows}: {arrow:?}"
            );
            if columns == 60 {
                assert!(!whole, "60 columns cannot hold it whole: {arrow:?}");
            }
        }
    }
}

/// A URL longer than the field scrolls: its end (what he just typed or
/// pasted) and the cursor after it stay in view, the start behind `…`.
#[test]
fn a_long_url_keeps_its_end_and_the_cursor_in_view() {
    let url = "file:///var/folders/z4/tqdvc9557mv3f1pqcnw0/T/graft-remote-o3U7fu/remote.git";
    let (_, lines) = draw(
        &clone_view(url, "~/code/remote", CloneShown::Typing),
        60,
        24,
    );
    let field = lines[11].trim_end_matches(['│', ' ']);
    assert!(field.starts_with("  │  clone › …"), "{field:?}");
    assert!(field.ends_with("/remote.git▏"), "{field:?}");
}

/// While git's question is open, the bar is the question's (`⏎ send · esc
/// cancel`), whatever the prompt under it.
#[test]
fn the_credential_bar_replaces_the_clone_bar() {
    let view = EmptyStateView {
        credential: true,
        ..clone_view(
            "http://h/x.git",
            "~/code/x",
            CloneShown::Running { progress: None },
        )
    };
    assert_eq!(bar_line(&view), "⏎ send · esc cancel");
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

/// Spell sweep (c), ticket m8/10c: every form's bar names his chords,
/// except the open prompt's — a text field, so its keys stay shipped.
#[test]
fn the_bar_spells_his_keys_and_the_open_prompt_the_shipped_ones() {
    use crate::keymap::sweep::{assert_shipped, assert_spelled, swept};
    let (keys, _) = swept();
    let line = |view: &EmptyStateView| -> String {
        bar_keys(view, &keys)
            .into_iter()
            .map(|(key, label)| format!("{key} {label}"))
            .collect::<Vec<_>>()
            .join("  ")
    };
    assert_spelled(
        &line(&with_recents()),
        &keys,
        &[
            &["empty_state.open"],
            &["empty_state.init"],
            &["empty_state.clone"],
            &["app.open"],
            &["app.quit"],
        ],
    );
    let confirm = EmptyStateView {
        prompt: Some(EmptyPrompt::ConfirmInit),
        ..with_recents()
    };
    assert_spelled(&line(&confirm), &keys, &[&["confirm.yes"], &["app.back"]]);
    let open = EmptyStateView {
        prompt: Some(EmptyPrompt::Open {
            text: sanitize_line("~/code"),
            cursor: 6,
        }),
        ..with_recents()
    };
    assert_shipped(
        &line(&open),
        &[&["app.open"], &["app.next_pane"], &["app.back"]],
    );
}

/// The first-launch line (m8/18d, R7): muted, on the row above the rule,
/// when nothing else is there; not drawn when the list reaches that row.
#[test]
fn first_launch_sits_above_the_rule_when_free() {
    let view = EmptyStateView {
        first_launch: true,
        ..with_recents()
    };
    let (_, lines) = draw(&view, 120, 36);
    assert_eq!(
        lines[33], " first time? ? keys here · ⌃P anything",
        "{lines:#?}"
    );
    assert!(lines[34].starts_with('─'));

    // Nine rows at 16 high: the list runs to the row above the rule.
    let full = EmptyStateView {
        recent: (0..9)
            .map(|i| row(&format!("r{i}"), "~/r", "today"))
            .collect(),
        first_launch: true,
        ..with_recents()
    };
    let (_, lines) = draw(&full, 120, 16);
    assert!(lines[13].contains("r7"), "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.contains("first time?")),
        "{lines:#?}"
    );

    let (_, lines) = draw(&with_recents(), 120, 36);
    assert!(!lines.iter().any(|line| line.contains("first time?")));
}

/// At every width a clause is drawn whole or not at all (ADR-0039), and
/// an unbound key drops its clause.
#[test]
fn first_launch_never_draws_a_fragment() {
    let keys = crate::keymap::Keymap::shipped();
    let first = "first time? ? keys here";
    let whole = "first time? ? keys here · ⌃P anything";
    for width in 20..=200 {
        let words = first_launch_words(keys, width);
        let expected = if width >= crate::frame_box::cells(whole) {
            Some(whole)
        } else if width >= crate::frame_box::cells(first) {
            Some(first)
        } else {
            None
        };
        assert_eq!(words.as_deref(), expected, "{width}");
    }
}
