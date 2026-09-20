//! Name prompt: with and without the checkout option, and with a refusal.

use graft_term::sanitize_line;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn field() -> FieldView<'static> {
    FieldView {
        label: "",
        text: sanitize_line("feature/auth"),
        cursor: Some(12),
        guide_at: None,
        counter: None,
        focused: true,
        secret: None,
    }
}

fn rows(view: &NamePromptView<'_>, theme: &Theme, width: u16, height: u16) -> Vec<String> {
    let frame = Rect::new(0, 0, width, height);
    let area = name_prompt_area(frame);
    let mut buf = Buffer::empty(frame);
    (&NamePrompt { view, theme }).render(area, &mut buf);
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

#[test]
fn draws_the_title_and_typed_name() {
    let theme = theme();
    let view = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: None,
        refusal: None,
    };
    let drawn = rows(&view, &theme, 80, 24);
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("New branch at a1b2c3d"))
    );
    assert!(drawn.iter().any(|line| line.contains("feature/auth")));
}

#[test]
fn the_checkout_option_draws_when_offered_and_shows_its_state() {
    let theme = theme();
    let mut view = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: Some((sanitize_line("check out after creating"), true)),
        refusal: None,
    };
    let on = rows(&view, &theme, 80, 24);
    assert!(on.iter().any(|line| line.contains("(•) check out")));

    view.option = Some((sanitize_line("check out after creating"), false));
    let off = rows(&view, &theme, 80, 24);
    assert!(off.iter().any(|line| line.contains("( ) check out")));
}

#[test]
fn a_refusal_draws_under_the_field() {
    let theme = theme();
    let view = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: None,
        refusal: Some(sanitize_line("exists")),
    };
    let drawn = rows(&view, &theme, 80, 24);
    assert!(drawn.iter().any(|line| line.contains("exists")));
}

#[test]
fn too_small_shows_the_fallback_message() {
    let theme = theme();
    let view = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: None,
        refusal: None,
    };
    let drawn = rows(&view, &theme, 30, 6);
    let non_empty: Vec<&String> = drawn.iter().filter(|line| !line.is_empty()).collect();
    assert_eq!(non_empty.len(), 1, "{drawn:#?}");
    assert!(non_empty[0].contains("too small for the prompt"));
}

/// Every widget in this ticket must draw nothing outside its own `Rect`.
#[test]
fn name_prompt_stays_inside_its_rect() {
    let theme = theme();
    let view = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: Some((sanitize_line("check out after creating"), true)),
        refusal: Some(sanitize_line("bad name: cannot start with -")),
    };
    let outer = Rect::new(0, 0, 80, 24);
    let inner = Rect::new(5, 5, 56, 6);
    let mut buf = Buffer::empty(outer);
    (&NamePrompt {
        view: &view,
        theme: &theme,
    })
        .render(inner, &mut buf);
    for y in 0..outer.height {
        for x in 0..outer.width {
            let inside = x >= inner.x
                && x < inner.x + inner.width
                && y >= inner.y
                && y < inner.y + inner.height;
            if !inside {
                assert_eq!(
                    buf[(x, y)].symbol(),
                    " ",
                    "drew outside its rect at ({x}, {y})"
                );
            }
        }
    }
}

/// Not an assertion — dumps the plain prompt, with the checkout option, and
/// with a refusal, at 80×24 and 160×48. Run by hand to regenerate:
/// `cargo test -p graft-ui name_prompt::tests::capture_for_docs_shots --
/// --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-ui-name-prompt.txt; run by hand to regenerate"]
fn capture_for_docs_shots() {
    let theme = theme();
    let mut out = String::new();
    out.push_str(
        "# docs/shots/m5-ui-name-prompt.txt — one-field name/create prompt\n\
         #\n\
         # What right looks like: title, typed name field, an optional\n\
         # (•)/( ) checkout toggle, a refusal line under the field when the\n\
         # name is rejected, and the footer \"⏎ create  ⇥ toggle  esc cancel\".\n\
         # Below 40x7 it draws one centred \"too small\" line and nothing else.\n\
         \n",
    );
    let plain = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: None,
        refusal: None,
    };
    let with_option = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: Some((sanitize_line("check out after creating"), true)),
        refusal: None,
    };
    let refused = NamePromptView {
        title: sanitize_line("New branch at a1b2c3d"),
        field: field(),
        option: Some((sanitize_line("check out after creating"), false)),
        refusal: Some(sanitize_line("bad name: cannot start with -")),
    };
    for (label, view, w, h) in [
        ("plain, 80x24", &plain, 80, 24),
        ("with checkout option, 160x48", &with_option, 160, 48),
        ("refused, 80x24", &refused, 80, 24),
    ] {
        out.push_str(&format!("=== {label} ===\n"));
        for line in rows(view, &theme, w, h) {
            if !line.trim().is_empty() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push('\n');
    }
    std::fs::write(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m5-ui-name-prompt.txt"
        ),
        out,
    )
    .expect("docs/shots is writable");
}
