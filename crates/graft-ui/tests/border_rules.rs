//! No bordered surface puts text against its own rule.
//!
//! The four surfaces that draw a label into a border are checked as a class,
//! not one at a time. They disagreed: `confirm` and `preview_box` went
//! through the shared `frame_box::border_row`, which puts a space on both
//! sides of the label, while `credential_prompt` kept a private copy that
//! put one only on the left. The result read as
//! `╭ Credentials — 127.0.0.1:8443─────╮`, where `8443─────` looks like part
//! of the host, and `╰ ⏎ send  ⌃U clear  esc cancel the push────╯` did the
//! same to the footer.
//!
//! So the property is stated over the drawn cells rather than over any one
//! helper: a rule cell (`─`) never touches a cell holding text. Space on
//! either side, another rule cell, or a corner — anything else is the fault.
//! A fifth surface that grows its own `border_row` fails here the day it is
//! drawn.

mod support;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
use graft_ui::confirm::{Confirm, ConfirmView};
use graft_ui::credential_prompt::{CredentialPrompt, CredentialView};
use graft_ui::name_prompt::{NamePrompt, NamePromptView};
use graft_ui::preview_box::{PreviewBox, PreviewBoxWidget};
use graft_ui::text_input::FieldView;
use graft_ui::theme::{Theme, ThemeKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use support::theme;

/// The glyph a border's rule is drawn with.
const RULE: &str = "\u{2500}";

/// Cells that may sit beside a rule: more rule, the four rounded corners,
/// and blank.
fn may_touch_a_rule(cell: &str) -> bool {
    matches!(
        cell,
        RULE | "\u{256d}" | "\u{256e}" | "\u{256f}" | "\u{2570}" | " " | ""
    )
}

/// The first place `buf` runs a rule straight into text, in words.
fn rule_fault(buf: &Buffer, area: Rect) -> Option<String> {
    for y in area.top()..area.bottom() {
        let row: Vec<String> = (area.left()..area.right())
            .map(|x| buf[(x, y)].symbol().to_owned())
            .collect();
        for (index, cell) in row.iter().enumerate() {
            if cell != RULE {
                continue;
            }
            let left = index.checked_sub(1).and_then(|i| row.get(i));
            let right = row.get(index + 1);
            for (side, neighbour) in [("left", left), ("right", right)] {
                if let Some(neighbour) = neighbour
                    && !may_touch_a_rule(neighbour)
                {
                    return Some(format!(
                        "row {y} column {} — the rule touches {neighbour:?} on its {side}: {}",
                        area.left() + u16::try_from(index).unwrap_or(u16::MAX),
                        row.concat()
                    ));
                }
            }
        }
    }
    None
}

/// Render `widget` at a few widths and report the first fault.
fn check(name: &str, theme: &Theme, draw: impl Fn(Rect, &mut Buffer, &Theme)) {
    for width in [40u16, 56, 72, 80, 120] {
        let area = Rect::new(0, 0, width, 8);
        let mut buf = Buffer::empty(area);
        draw(area, &mut buf, theme);
        assert!(
            rule_fault(&buf, area).is_none(),
            "{name} at {width} columns: {}",
            rule_fault(&buf, area).unwrap_or_default()
        );
    }
}

/// An empty field, the one shape every prompt here draws.
fn blank_field(label: &'static str) -> FieldView<'static> {
    FieldView {
        label,
        text: sanitize_line(""),
        cursor: None,
        guide_at: None,
        counter: None,
        focused: true,
        secret: None,
    }
}

#[test]
fn the_confirm_box_and_the_credential_prompt_keep_their_rules_off_their_text() {
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    let confirm = ConfirmView::new(
        sanitize_line("Push main → origin/main"),
        vec![sanitize_line(
            "1 commit · can't be undone from here — origin keeps what you send",
        )],
        sanitize_line("git push --progress origin refs/heads/main:refs/heads/main"),
    )
    .with_labels("Cancel", "Push");
    check("the confirm box", &theme, |area, buf, theme| {
        (&Confirm {
            view: &confirm,
            theme,
        })
            .render(area, buf);
    });

    // A host and port in the title and key hints in the bottom rule:
    // exactly the two places the fault was found.
    let view = CredentialView {
        title: sanitize_line("Credentials — 127.0.0.1:8443"),
        prompt: sanitize_line("Password for 'http://user@127.0.0.1:8443':"),
        field: blank_field("Password"),
        note: sanitize_line("typed here only · never stored by Graft"),
        footer: sanitize_line("⏎ send  ⌃U clear  esc cancel the push"),
    };
    check("the credential prompt", &theme, |area, buf, theme| {
        (&CredentialPrompt {
            view: &view,
            theme,
            ascii: false,
        })
            .render(area, buf);
    });
}

#[test]
fn the_ghost_preview_and_the_name_prompt_keep_their_rules_off_their_text() {
    let theme = theme(ThemeKind::Dark, ColorDepth::TrueColor);

    let mut preview = PreviewBox::new(sanitize_line("Rebase feature onto main — 3 commits"));
    preview.lines = vec![sanitize_line("pick a1b2c3d a change")];
    preview.commands = vec![sanitize_line("git rebase --onto main feature")];
    check("the Ghost Preview box", &theme, |area, buf, theme| {
        (&PreviewBoxWidget {
            view: &preview,
            theme,
        })
            .render(area, buf);
    });

    let name = NamePromptView {
        title: sanitize_line("New branch from a1b2c3d — 127.0.0.1:8443"),
        field: blank_field("Name"),
        option: Some((sanitize_line("check it out"), true)),
        refusal: None,
    };
    check("the name prompt", &theme, |area, buf, theme| {
        (&NamePrompt { view: &name, theme }).render(area, buf);
    });
}
