//! Fixtures shared by the crate's tests.
//!
//! [`Capabilities`] has no `Default` on purpose — an unprobed terminal is not
//! a neutral one — so every test that needs a [`Theme`] would otherwise repeat
//! a sixteen-field literal, and a new field would mean editing all of them.

use std::time::Duration;

use mergewright_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Supported,
    Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::layout::layout;
use mergewright_graph::row::Row;
use mergewright_graph::seeds::{BranchSeed, Seeds};

/// Every non-blank cell of `buffer` drawn on `theme`'s `selection` fill has
/// a foreground at 4.5:1 or better against it (ADR-0058 §3). Truecolor
/// themes only. Returns how many cells it checked, so a caller can assert
/// that the cursor row was on screen at all.
pub(crate) fn assert_legible_on_selection(
    buffer: &ratatui::buffer::Buffer,
    theme: &crate::theme::Theme,
) -> usize {
    use crate::theme::{Token, TokenColor, contrast};
    use mergewright_core::capabilities::Rgb;
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
            default_branch: Some(BranchSeed::new(
                oid(merge(topics)),
                BranchIdentity::of_name(b"main"),
            )),
            ..Seeds::default()
        },
    )
}

/// His keymap with `ids` set to `none`.
pub(crate) fn unbound(ids: &[&str]) -> crate::keymap::Keymap {
    crate::keymap::Keymap::with_overrides(
        &ids.iter()
            .map(|id| ((*id).to_owned(), vec!["none".to_owned()]))
            .collect::<Vec<_>>(),
        crate::keymap::KeySwitches::default(),
    )
    .0
}

/// **One check for the journal, stash and Map footers** (m8/11e F2, E1).
/// `bottom(width, keys)` draws the place at `width` under `keys` and hands
/// back its footer row, trailing blanks trimmed. `clauses` are its full key
/// clauses under the shipped keymap, `notes` its notes as drawn
/// (`" · +3 more"`), in order.
///
/// - 48, 80, 120, 200: the keys end with `? keys`; with `help` unbound,
///   `⌃P keys`; with `palette` unbound too, no doorway and no stray
///   separator.
/// - 20..=200: every piece drawn is whole — one of the clauses, the
///   doorway, or its bare `?` — and once `? keys` is drawn, every wider
///   width draws it too.
pub(crate) fn assert_place_footer(
    what: &str,
    clauses: &[&str],
    notes: &[&str],
    bottom: impl Fn(u16, &crate::keymap::Keymap) -> String,
) {
    assert_footer_doorway(what, notes, &bottom);
    assert_footer_whole(what, clauses, notes, &bottom);
}

/// A footer row's key pieces: `notes` stripped off its end, the rest split
/// at its two-space joins.
fn footer_pieces(what: &str, line: &str, notes: &[&str]) -> Vec<String> {
    let mut keys_part = line.to_owned();
    for note in notes.iter().rev() {
        if let Some(rest) = keys_part.strip_suffix(note) {
            keys_part = rest.to_owned();
        }
    }
    assert!(
        !keys_part.contains(" · "),
        "{what}: a note cut or out of order: {line:?}"
    );
    if keys_part.is_empty() {
        return Vec::new();
    }
    keys_part.split("  ").map(str::to_owned).collect()
}

/// The doorway half of [`assert_place_footer`]: 48, 80, 120, 200 cells.
pub(crate) fn assert_footer_doorway(
    what: &str,
    notes: &[&str],
    bottom: &impl Fn(u16, &crate::keymap::Keymap) -> String,
) {
    let shipped = crate::keymap::Keymap::shipped();
    let pieces = |line: &str| footer_pieces(what, line, notes);
    let no_help = unbound(&["app.help"]);
    let neither = unbound(&["app.help", "app.palette"]);
    for width in [48u16, 80, 120, 200] {
        let line = bottom(width, shipped);
        assert!(
            pieces(&line).last().is_some_and(|last| last == "? keys"),
            "{what} at {width}: the keys do not end with `? keys`: {line:?}"
        );
        let line = bottom(width, &no_help);
        assert!(
            pieces(&line).last().is_some_and(|last| last == "⌃P keys"),
            "{what} at {width}, `help` unbound: {line:?}"
        );
        let line = bottom(width, &neither);
        let drawn = pieces(&line);
        assert!(
            !line.contains("keys") && !line.starts_with(' ') && !line.starts_with('·'),
            "{what} at {width}, both unbound: {line:?}"
        );
        assert!(
            drawn.iter().all(|piece| !piece.is_empty()),
            "{what} at {width}, both unbound: a stray separator: {line:?}"
        );
    }
}

/// The whole-clause half of [`assert_place_footer`]: 20..=200 cells.
pub(crate) fn assert_footer_whole(
    what: &str,
    clauses: &[&str],
    notes: &[&str],
    bottom: &impl Fn(u16, &crate::keymap::Keymap) -> String,
) {
    let shipped = crate::keymap::Keymap::shipped();
    let pieces = |line: &str| footer_pieces(what, line, notes);
    let mut drawn_before = false;
    for width in 20..=200u16 {
        let line = bottom(width, shipped);
        let drawn = pieces(&line);
        for piece in &drawn {
            assert!(
                clauses.contains(&piece.as_str()) || piece == "? keys" || piece == "?",
                "{what} at {width}: `{piece}` is a fragment: {line:?}"
            );
        }
        let has = drawn.last().is_some_and(|last| last == "? keys");
        assert!(
            has || !drawn_before,
            "{what}: `? keys` drawn below {width} but not at it: {line:?}"
        );
        drawn_before |= has;
    }
}
