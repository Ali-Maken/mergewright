//! ease-pass 26: a failed setting write passes words only; the bar's `⊗` is
//! the renderer's (`mergewright_ui::failure`).

use super::graph_mode_kept;
use crate::config::MergedBranches;
use crate::update::{AppState, Status};

/// Replaces ticket 25's `the_bar_says_kept_or_why_not_with_its_mark_drawn`
/// check that the status text carried `✗`: now the text is bare words.
#[test]
fn a_failed_write_passes_words_only() {
    let mut state = AppState::default();
    graph_mode_kept(
        &mut state,
        MergedBranches::Shown,
        Err("can't write /x/config.toml: denied".to_owned()),
    );
    let Status::Failed(text) = &state.status else {
        panic!("a failed write is Status::Failed: {:?}", state.status);
    };
    assert!(text.starts_with("Graph:"), "{text}");
    assert!(!text.contains('✗') && !text.contains('⊗'), "{text}");
    let drawn = crate::view::tests::draw(&state, 120, 12);
    let bar = drawn
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .expect("a row")
        .trim_matches('"')
        .trim();
    assert!(
        bar.starts_with("⊗ Graph: every branch for this run · not kept: can't write"),
        "{bar}"
    );
    assert_eq!(drawn.matches('⊗').count(), 1, "{drawn}");
    assert!(!drawn.contains('✗'), "{drawn}");
}

/// The failed setting bar at 187x50, Unicode then ASCII tier: the capture
/// `docs/shots/ease/26-failure-mark.txt` (ease-pass 26). Bottom rows only.
#[test]
fn capture_ease_26_failure_mark() {
    let bar = |ascii: bool| {
        let mut state = AppState {
            history: Some(crate::view::tests::history_of(5, 0)),
            size: crate::update::Size {
                columns: 187,
                rows: 50,
            },
            ..AppState::default()
        };
        if ascii {
            state.renderer_override = Some(mergewright_core::capabilities::RenderingTier::Ascii);
        }
        graph_mode_kept(
            &mut state,
            MergedBranches::Shown,
            Err("can't write /x/config.toml: denied".to_owned()),
        );
        let drawn = crate::view::tests::draw(&state, 187, 50);
        let rows: Vec<&str> = drawn.lines().collect();
        rows[rows.len().saturating_sub(3)..].join("\n")
    };
    let (unicode, ascii) = (bar(false), bar(true));
    assert!(
        unicode.contains("⊗ Graph: every branch for this run · not kept: can't write"),
        "{unicode}"
    );
    assert!(
        ascii.contains("(x) Graph: every branch for this run · not kept: can't write")
            && !ascii.contains('⊗'),
        "{ascii}"
    );
    let capture = format!(
        "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_ease_26_failure_mark in crates/mergewright/src/update/palette/setting/tests.rs
# (asserted). Last three rows of the frame, 187x50. Colour (the mark in red)
# is unverified: a capture cannot show it.
#
=== Unicode tier. Right: the bottom row starts `⊗ Graph: every branch for this run · not kept: <why>`, then ` · esc dismiss` ===
{unicode}

=== ASCII tier. Right: the same row starts `(x) Graph: ...`, no `⊗` ===
{ascii}
"
    );
    mergewright_fixtures::capture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/ease/26-failure-mark.txt"
        ),
        capture,
    );
}
