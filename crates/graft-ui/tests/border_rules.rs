//! No bordered surface puts text against a border glyph — on any side,
//! inside or out (m5-history-and-ghost-preview/30's invariant, replacing
//! three point fixes that each created the next).
//!
//! Ticket 28 found the first instance and stated the property only over the
//! horizontal rule: `confirm` and `preview_box` went through the shared
//! `frame_box::border_row`, which puts a space on both sides of a title,
//! while `credential_prompt` kept a private copy that put one only on the
//! left. The result read as `╭ Credentials — 127.0.0.1:8443─────╮`, where
//! `8443─────` looks like part of the host.
//!
//! Ticket 29 found a second instance the rule-only property could not see at
//! all — a box's own right *border*, run into the graph column beside it
//! (`╮60af0eb`) — and fixed it with a gutter in the snap, in a different
//! file (`graft::view::frame`), tested a different way
//! (`graft-ui/tests/pane_boundaries.rs`, which knows about the graph's
//! columns and so can check "beside the box", something this file's widgets
//! alone cannot).
//!
//! Ticket 30 (this one) found a third instance neither caught: the box's own
//! **vertical** border (`│`) run into its own longest content line
//! (`…you send│`), because nothing here checked a `│` cell's neighbours at
//! all, only a `─` cell's. So the property widens to every border glyph a
//! box draws with — `─│╭╮╯╰` — checked against its left and right neighbour
//! wherever it appears, not only in the top and bottom rows. That is now one
//! test class instead of three: it would have caught all three instances
//! above, and it catches the fourth the day some fifth surface's content row
//! stops reserving its own padding.
//!
//! **Scope, stated plainly:** this file checks a border glyph against the
//! cell immediately to its own left or right, within the widget's own
//! rectangle — "inside" the invariant's "inside or out". The "out" half —
//! whether a box's border touches *another widget's* content drawn beside
//! it — needs the other widget on screen to check against, which nothing
//! here draws; that half is `pane_boundaries.rs`'s
//! `no_box_drawn_over_the_graph_touches_the_content_beside_it` and
//! `graft::view::tests`'
//! `the_confirm_box_leaves_a_gutter_before_a_wider_detail_pane` (this
//! ticket's Q1 case, the detail pane beside the box).
#![allow(
    clippy::expect_used,
    reason = "a test row that cannot even be measured has nothing to check; the panic names why"
)]

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

/// Every glyph a box draws its own frame with: the horizontal rule, the
/// vertical border, and the four rounded corners.
fn is_border_glyph(cell: &str) -> bool {
    matches!(
        cell,
        "\u{2500}" | "\u{2502}" | "\u{256d}" | "\u{256e}" | "\u{256f}" | "\u{2570}"
    )
}

/// Cells that may sit beside a border glyph: another border glyph, or
/// blank. Anything else is text, and text is the fault.
fn may_touch_a_border(cell: &str) -> bool {
    is_border_glyph(cell) || cell == " " || cell.is_empty()
}

/// The first place `buf` runs a border glyph straight into text, in words —
/// checked on both sides of **every** border glyph, not only the rule: a
/// `│` with a non-space neighbour is the same fault turned ninety degrees
/// (instance three, `…you send│`).
fn border_fault(buf: &Buffer, area: Rect) -> Option<String> {
    for y in area.top()..area.bottom() {
        let row: Vec<String> = (area.left()..area.right())
            .map(|x| buf[(x, y)].symbol().to_owned())
            .collect();
        for (index, cell) in row.iter().enumerate() {
            if !is_border_glyph(cell) {
                continue;
            }
            let left = index.checked_sub(1).and_then(|i| row.get(i));
            let right = row.get(index + 1);
            for (side, neighbour) in [("left", left), ("right", right)] {
                if let Some(neighbour) = neighbour
                    && !may_touch_a_border(neighbour)
                {
                    return Some(format!(
                        "row {y} column {} ({cell:?}) — it touches {neighbour:?} on its {side}: \
                         {}",
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
            border_fault(&buf, area).is_none(),
            "{name} at {width} columns: {}",
            border_fault(&buf, area).unwrap_or_default()
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
        note: sanitize_line("typed here only · never stored by Mergewright"),
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
            keys: graft_ui::keymap::Keymap::shipped(),
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

/// One row, hand-built to hold exactly the fault each ticket found, so the
/// **detector** itself is pinned against all three by name — not only
/// against whichever widget happens to reproduce them today.
fn one_row(row: &str) -> (Buffer, Rect) {
    let width = u16::try_from(row.chars().count()).expect("a test row is narrower than u16::MAX");
    let area = Rect::new(0, 0, width, 1);
    let mut buf = Buffer::empty(area);
    buf.set_string(0, 0, row, ratatui::style::Style::default());
    (buf, area)
}

/// Instance 1 (ticket 28): the title ran into its own rule —
/// `╭ Credentials — 127.0.0.1:8443─────╮`. Ticket 28 already fixed the
/// widget (`border_row` puts a space on both sides of a label everywhere
/// now), so this cannot be reproduced by drawing a widget any more; it pins
/// the **detector** against the fault directly, so a regression anywhere
/// still gets caught the day it draws this shape again.
#[test]
fn instance_one_a_title_run_into_its_own_rule_is_still_caught() {
    let (buf, area) = one_row("╭ Credentials — 127.0.0.1:8443─────╮");
    assert!(
        border_fault(&buf, area).is_some(),
        "the detector must flag a rule run straight into a title"
    );
}

/// Instance 2 (ticket 29): the box's border ran into the graph column
/// beside it — `╮60af0eb`. Fixed at the layout site
/// (`graft_ui::graph_screen::snap_to_columns`'s gutter step), a different
/// file this crate's widgets know nothing about, so it is pinned the same
/// way as instance 1: against the detector directly. The live, drawn
/// version of this instance — the box's border against the graph's own
/// content, not a hand-built row — is
/// `pane_boundaries.rs`'s `no_box_drawn_over_the_graph_touches_the_content_beside_it`,
/// which sweeps the real `snap_to_columns` and names the mutation that
/// reproduces it (dropping the `- 1` gutter step).
#[test]
fn instance_two_a_border_run_into_the_graph_beside_it_is_still_caught() {
    let (buf, area) = one_row("╮60af0eb");
    assert!(
        border_fault(&buf, area).is_some(),
        "the detector must flag a border run straight into adjoining content"
    );
}

/// Instance 3 (this ticket): the confirm box's own longest line ran into
/// its own right border — `…you send│`. Unlike instances 1 and 2, this one
/// **is** still reproducible by drawing the real widget: reverting
/// `frame_box::content_row` to clamp `" {text}"` to the full inner width
/// (no reserved right-hand padding) reproduces it exactly, failing
/// `the_confirm_box_and_the_credential_prompt_keep_their_rules_off_their_text`
/// at 40 columns naming
/// `"│ git push --progress origin refs/head…│"` — verified by hand,
/// restored before this commit. Pinned here the same way as the other two,
/// so the detector's own coverage of this exact shape does not depend on
/// which widget happens to draw it.
#[test]
fn instance_three_a_box_s_own_longest_line_run_into_its_own_border_is_still_caught() {
    let (buf, area) = one_row("│ 1 commit …you send│");
    assert!(
        border_fault(&buf, area).is_some(),
        "the detector must flag a content row's own text run into its own border"
    );
}
