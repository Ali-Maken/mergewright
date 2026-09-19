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
    let frame = Rect::new(0, 0, width, height);
    let area = credential_area(frame);
    let mut buf = Buffer::empty(frame);
    (&CredentialPrompt { view, theme }).render(area, &mut buf);
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

fn password_prompt() -> CredentialView<'static> {
    CredentialView {
        title: sanitize_line("Credentials — 127.0.0.1:8443"),
        prompt: sanitize_line("Password for 'http://user@127.0.0.1:8443':"),
        field: FieldView {
            label: "",
            text: sanitize_line("hunter2"),
            cursor: Some(7),
            guide_at: None,
            counter: None,
            focused: true,
            masked: true,
        },
        note: sanitize_line("typed here only · never stored by Graft"),
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
            masked: false,
        },
        note: sanitize_line("typed here only · never stored by Graft"),
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
    assert!(
        box_lines[2].contains("••••••••") || box_lines[2].contains("•••••••"),
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
    assert!(
        content[0].contains("too small"),
        "expected the too-small message: {content:?}"
    );
    assert!(!content[0].contains("hunter2"), "{content:?}");
}

/// Not an assertion — prints the masked and unmasked prompts at both
/// breakpoints plus the too-small fallback, for a `docs/shots/` capture.
/// `cargo test -p graft-ui credential_prompt::tests::capture_for_docs_shots
/// -- --ignored --nocapture`, by hand, when the rendering changes.
#[test]
#[ignore = "run by hand to regenerate docs/shots/m4-credential-prompt-and-hints.txt"]
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
        (&CredentialPrompt { view: &view, theme: &theme }).render(area, &mut buf);
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
