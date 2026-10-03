//! Fixtures shared by the crate's tests.
//!
//! [`Capabilities`] has no `Default` on purpose — an unprobed terminal is not
//! a neutral one — so every test that needs a [`Theme`] would otherwise repeat
//! a sixteen-field literal, and a new field would mean editing all of them.

use std::time::Duration;

use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
    Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::row::Row;
use graft_graph::seeds::{BranchSeed, Seeds};

/// Every non-blank cell of `buffer` drawn on `theme`'s `selection` fill has
/// a foreground at 4.5:1 or better against it (ADR-0058 §3). Truecolor
/// themes only. Returns how many cells it checked, so a caller can assert
/// that the cursor row was on screen at all.
pub(crate) fn assert_legible_on_selection(
    buffer: &ratatui::buffer::Buffer,
    theme: &crate::theme::Theme,
) -> usize {
    use crate::theme::{Token, TokenColor, contrast};
    use graft_core::capabilities::Rgb;
    use ratatui::style::Color;

    let TokenColor::Rgb(selection) = theme.token(Token::Selection) else {
        panic!("a truecolor theme gives selection an Rgb");
    };
    let fill = Color::Rgb(selection.red, selection.green, selection.blue);
    let mut checked = 0;
    for y in buffer.area.top()..buffer.area.bottom() {
        for x in buffer.area.left()..buffer.area.right() {
            let cell = &buffer[(x, y)];
            if cell.bg != fill || cell.symbol().trim().is_empty() {
                continue;
            }
            let fg = match cell.fg {
                Color::Rgb(red, green, blue) => Rgb { red, green, blue },
                other => panic!("({x},{y}) {:?}: fg {other:?} on the fill", cell.symbol()),
            };
            let ratio = contrast(fg, selection);
            assert!(
                ratio >= 4.5,
                "({x},{y}) {:?} is {ratio:.2}:1 on selection",
                cell.symbol()
            );
            checked += 1;
        }
    }
    checked
}

/// A terminal with the given colour depth and `NO_COLOR` setting. Every other
/// field is irrelevant to the tests that use this and is filled with something
/// inert.
pub(crate) fn terminal(color_depth: ColorDepth, no_color: bool) -> Capabilities {
    Capabilities {
        tier: RenderingTier::Unicode,
        pixel_protocol: None,
        kitty_graphics: Supported::No,
        kitty_deflate: false,
        answers_dsr: false,
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::No,
        cell_size: CellSize {
            width: 8,
            height: 16,
            source: CellSizeSource::Assumed,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Assumed,
        },
        color_depth,
        no_color,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
}

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

fn commit(id: u8, parents: &[u8], seconds: i64, subject: &str) -> Commit {
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    };
    Commit {
        id: oid(id),
        parents: parents.iter().map(|&byte| oid(byte)).collect(),
        author: who.clone(),
        committer: who,
        message: subject.as_bytes().to_vec(),
    }
}

/// `main` with `topics` topic branches merged one after another, laid out:
/// `topics` merge rows at the top — the one at row `k` opens column `k + 1`
/// — then the topic commits, then the root. Ids: root 1, topic `k` is
/// `1 + k`, its merge is `1 + topics + k`.
pub(crate) fn wide_history(topics: u8) -> Vec<Row> {
    let merge = |k: u8| 1 + topics + k;
    let mut commits = Vec::new();
    for k in (1..=topics).rev() {
        let below = if k == 1 { 1 } else { merge(k - 1) };
        let subject = format!("Merge branch 'topic-{k}'");
        commits.push(commit(
            merge(k),
            &[below, 1 + k],
            i64::from(100 - k),
            &subject,
        ));
    }
    for k in (1..=topics).rev() {
        let subject = format!("topic-{k}: work");
        commits.push(commit(1 + k, &[1], i64::from(50 - k), &subject));
    }
    commits.push(commit(1, &[], 0, "root"));
    layout(
        &commits,
        Seeds {
            default_branch: Some(BranchSeed {
                tip: oid(merge(topics)),
                identity: BranchIdentity::of_name(b"main"),
            }),
            ..Seeds::default()
        },
    )
}
