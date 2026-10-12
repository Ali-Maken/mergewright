//! The credential prompt's acceptance: structure at both breakpoints, the
//! too-small fallback, and that nothing ever draws outside its own `Rect`
//! (m4-remotes ticket 03).

use graft_term::sanitize_line;
use proptest::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

/// Draw `view` at `width × height` and return its rows as plain strings —
/// the character grid a capture would show.
fn rows(view: &CredentialView<'_>, theme: &Theme, width: u16, height: u16) -> Vec<String> {
    rows_at_tier(view, theme, width, height, false)
}

/// Like [`rows`], but choosing the ASCII tier's mask glyph (`*`) when
/// `ascii` is set — the path a real terminal at that tier draws.
fn rows_at_tier(
    view: &CredentialView<'_>,
    theme: &Theme,
    width: u16,
    height: u16,
    ascii: bool,
) -> Vec<String> {
    let frame = Rect::new(0, 0, width, height);
    let area = credential_area(frame);
    let mut buf = Buffer::empty(frame);
    (&CredentialPrompt { view, theme, ascii }).render(area, &mut buf);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// `pulse` is `false`: only `eight_glyphs_for_any_length` below cares about
/// the pulse flipping, and it builds its own views.
fn password_prompt() -> CredentialView<'static> {
    CredentialView {
        title: sanitize_line("Credentials — 127.0.0.1:8443"),
        prompt: sanitize_line("Password for 'http://user@127.0.0.1:8443':"),
        field: FieldView {
            label: "",
            // No real text to pass: `graft::view::credential_view` never
            // builds one for a secret field (ADR-0038 §2).
            text: sanitize_line(""),
            cursor: None,
            guide_at: None,
            counter: None,
            focused: true,
            secret: Some(false),
        },
        note: sanitize_line("typed here only · never stored by Mergewright"),
        footer: sanitize_line("⏎ send  ⌃U clear  esc cancel the push"),
    }
}

fn username_prompt() -> CredentialView<'static> {
    CredentialView {
        title: sanitize_line("Credentials — github.com"),
        prompt: sanitize_line("Username for 'https://github.com':"),
        field: FieldView {
            label: "",
            text: sanitize_line("alice"),
            cursor: Some(5),
            guide_at: None,
            counter: None,
            focused: true,
            secret: None,
        },
        note: sanitize_line("typed here only · never stored by Mergewright"),
        footer: sanitize_line("⏎ send  ⌃U clear  esc cancel the push"),
    }
}

#[test]
fn masked_80x24() {
    let theme = theme();
    let view = password_prompt();
    let lines = rows(&view, &theme, 80, 24);
    let box_lines: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(box_lines.len(), 5, "{lines:#?}");
    assert!(box_lines[0].contains("Credentials"), "{box_lines:?}");
    assert!(box_lines[1].contains("Password for"), "{box_lines:?}");
    assert!(box_lines[2].contains("▏"), "{box_lines:?}");
    // A fixed 8 glyphs, whatever the real length — this view carries no
    // text at all, so any count other than 8 would mean the row is
    // computing its own idea of the length rather than the constant
    // (ADR-0038 §2; `masked_prompt_draws_only_glyphs` in `text_input`
    // covers the widget directly).
    assert_eq!(
        box_lines[2].matches('•').count(),
        crate::text_input::SECRET_GLYPHS,
        "{box_lines:?}"
    );
    assert!(!box_lines[2].contains("hunter2"), "{box_lines:?}");
    assert!(box_lines[3].contains("typed here only"), "{box_lines:?}");
    assert!(box_lines[4].contains("send"), "{box_lines:?}");
}

#[test]
fn username_160x48() {
    let theme = theme();
    let view = username_prompt();
    let lines = rows(&view, &theme, 160, 48);
    let box_lines: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(box_lines.len(), 5, "{lines:#?}");
    assert!(box_lines[0].contains("Credentials"), "{box_lines:?}");
    assert!(box_lines[1].contains("Username for"), "{box_lines:?}");
    assert!(box_lines[2].contains("alice"), "{box_lines:?}");
    assert!(box_lines[3].contains("typed here only"), "{box_lines:?}");
    assert!(box_lines[4].contains("send"), "{box_lines:?}");
}

#[test]
fn too_small_draws_one_line() {
    let theme = theme();
    let view = password_prompt();
    let lines = rows(&view, &theme, 30, 5);
    let content: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(content.len(), 1, "{lines:#?}");
    // The whole message, not a `fit`-cut prefix (Major 1): `TOO_SMALL` is
    // kept short enough to fit even at the narrowest width this branch
    // fires at, so a cut here would mean the message grew past its budget.
    assert_eq!(content[0].trim(), TOO_SMALL, "message was cut: {content:?}");
    assert!(!content[0].contains("hunter2"), "{content:?}");
}

#[test]
fn too_small_message_is_centred() {
    let theme = theme();
    let view = password_prompt();
    let width = 30;
    let lines = rows(&view, &theme, width, 5);
    let line = lines
        .iter()
        .find(|l| l.contains(TOO_SMALL))
        .unwrap_or_else(|| panic!("no line has the message: {lines:#?}"));
    let leading_spaces = line.len() - line.trim_start().len();
    let expected = usize::from(width) - TOO_SMALL.len();
    assert_eq!(
        leading_spaces,
        expected / 2,
        "expected the message centred in {width} columns: {line:?}"
    );
}

#[test]
fn masked_field_at_the_ascii_tier_draws_stars() {
    let theme = theme();
    let view = password_prompt();
    let lines = rows_at_tier(&view, &theme, 80, 24, true);
    let field_row = lines
        .iter()
        .find(|l| l.contains('▏'))
        .unwrap_or_else(|| panic!("no field row: {lines:#?}"));
    // A fixed 8 glyphs, whatever the real length (ADR-0038 §2).
    assert_eq!(
        field_row.matches('*').count(),
        crate::text_input::SECRET_GLYPHS,
        "expected {} stars at the ASCII tier: {field_row:?}",
        crate::text_input::SECRET_GLYPHS
    );
    assert!(!field_row.contains('•'), "{field_row:?}");
    assert!(!field_row.contains("hunter2"), "{field_row:?}");
}

/// Not an assertion — prints the masked and unmasked prompts at both
/// breakpoints plus the too-small fallback, for a `docs/shots/` capture.
/// `MWR_UPDATE_SHOTS=1 cargo test -p graft-ui credential_prompt::tests::capture_for_docs_shots`, by hand, when the rendering changes.
#[test]
fn capture_for_docs_shots() {
    let theme = theme();
    for (label, view, w, h) in [
        ("masked password, 80x24", password_prompt(), 80, 24),
        ("plaintext username, 160x48", username_prompt(), 160, 48),
        ("too small, 30x5", password_prompt(), 30, 5),
    ] {
        println!("=== {label} ===");
        for line in rows(&view, &theme, w, h) {
            if !line.trim().is_empty() {
                println!("{line:?}");
            }
        }
    }
}

proptest! {
    /// Nothing drawn outside `credential_area` at any size — the box, and
    /// the too-small fallback alike.
    #[test]
    fn stays_inside_its_rect(width in 1u16..200, height in 1u16..60) {
        let theme = theme();
        let view = password_prompt();
        let frame = Rect::new(0, 0, width, height);
        let area = credential_area(frame);
        prop_assert!(area.right() <= frame.right());
        prop_assert!(area.bottom() <= frame.bottom());

        let mut buf = Buffer::empty(frame);
        (&CredentialPrompt { view: &view, theme: &theme, ascii: false }).render(area, &mut buf);
        for y in 0..height {
            for x in 0..width {
                let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
                if !inside {
                    prop_assert_eq!(
                        buf[(x, y)].symbol(),
                        " ",
                        "drew outside the rect at ({}, {}) for {}x{}",
                        x,
                        y,
                        width,
                        height
                    );
                }
            }
        }
    }
}
