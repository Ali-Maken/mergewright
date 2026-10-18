//! `update` tests: doorway. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The place state `place_footers::place_footer_states` names `what`.
fn place_state(what: &str) -> AppState {
    place_footers::place_footer_states()
        .into_iter()
        .find(|(name, _)| name == what)
        .map_or_else(|| panic!("no place state named {what}"), |(_, state)| state)
}

/// One doorway capture: file name, what built its state, the state, its
/// size, and what right looks like there.
type DoorwayShot = (&'static str, &'static str, AppState, u16, u16, &'static str);

/// **The keys doorway on the places** (m8/11d, m8/11e): captures under
/// `docs/shots/m8/`, each drawn by `view::view` from an `AppState` —
/// the journal, stash and Map footers from `place_footers`, the blame bar
/// from the state `states_for` builds for its context. The bar row is the
/// user's to judge (rule 6); colour unverified (rule 5). The marked row's is
/// `tests/it/hint_bar.rs`: a radar marker lives on the row, not the state.
#[test]
fn capture_the_keys_doorway_for_docs_shots() {
    let mut drifted = Vec::new();
    for (name, context, state, columns, rows, what) in doorway_shots() {
        let state = settled(AppState {
            size: Size { columns, rows },
            ..state
        });
        // `TestBackend` quotes each row; the capture is the bare grid.
        let frame = crate::view::tests::draw_pinned(&state, columns, rows)
            .lines()
            .map(|row| {
                row.strip_prefix('"')
                    .and_then(|row| row.strip_suffix('"'))
                    .unwrap_or(row)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let out = format!(
            "# m8/{name}: {columns}x{rows}, drawn by `view::view` from the AppState\n\
             # `update::tests` builds for: {context}\n\
             # (crates/mergewright/src/update/tests/doorway.rs, capture_the_keys_doorway_for_docs_shots).\n\
             # Not a pty capture: a character grid, no colour (CLAUDE.md rule 5 — colour\n\
             # unverified).\n#\n# Here {what}.\n\n{frame}\n"
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/m8")
            .join(name);
        if let Err(why) = mergewright_fixtures::check(&path, &out) {
            drifted.push(why);
        }
    }
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// The doorway captures: the journal, stash and Map footers (m8/11e) and
/// the blame bar (m8/11d).
fn doorway_shots() -> Vec<DoorwayShot> {
    let blame_lines = HintContext::Blame {
        finding: false,
        ignore_file: false,
        lines: true,
    };
    let blame = states_for(&blame_lines, Divider::Hidden, 80)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no state for {blame_lines:?}"));
    vec![
        (
            "doorway-journal-120x36.txt",
            "journal row 0",
            place_state("journal row 0"),
            120,
            36,
            "the journal (J) is open with the graph's keyboard, the detail pane shut. \
             Right: its footer ends `J/esc close  ? keys` — the doorway last, after the \
             journal's own keys (m8/11e F2)",
        ),
        (
            "doorway-stash-120x36.txt",
            "stash list",
            place_state("stash list"),
            120,
            36,
            "the stash list (S) is open with one stash selected. Right: its footer reads \
             `⏎ apply  p pop  s save  d drop  b branch  space files  S/esc close  ? keys` — \
             `b branch` between drop and files, the doorway last (m8/11e F2, G1)",
        ),
        (
            "doorway-stash-60x20.txt",
            "stash list",
            place_state("stash list"),
            60,
            20,
            "the stash list at 60 columns. Right: the footer sheds whole clauses from \
             the middle and keeps `? keys` last — never a cut word (m8/11e E1)",
        ),
        (
            "doorway-map-120x36.txt",
            "map, +3 more, partial",
            place_state("map, +3 more, partial"),
            120,
            36,
            "the Map (M) with three branches more than it shows and a row cut short. \
             Right: the footer reads keys, then `? keys`, then ` · +3 more · partial` \
             (m8/11e E2)",
        ),
        (
            "doorway-blame-80x24.txt",
            "blame, lines read",
            blame,
            80,
            24,
            "a blame place with its lines read, 80 columns. Right: the bar reads \
             `⏎ that commit  ^ before it  L history of these lines  ? keys  /  space  w  esc` — \
             `? keys` after the words, leading the bare keys (ADR-0039, m8/11d R1)",
        ),
    ]
}

/// **A text field inside a place draws no keys doorway** (m8/11d, the
/// check before R1): journal find and blame find while typing, and the
/// stash save prompt, are where `?` types (`palette::open_key` refuses
/// under `text_field_open`), so no bar on that frame may offer `? keys` —
/// with the detail pane shut and shown, at the widths the app draws at.
#[test]
fn a_text_field_inside_a_place_draws_no_keys_doorway() {
    use mergewright_ui::graph_screen::every_context;

    // Every blame context with its lines read, as at base (11e-rulings-2 G5:
    // setup only); the journal and stash footers are no hint contexts since
    // m8/11e, so their states come from `place_footers`.
    let blame: Vec<AppState> = every_context()
        .into_iter()
        .filter(|context| matches!(context, HintContext::Blame { lines: true, .. }))
        .flat_map(|context| states_for(&context, Divider::Hidden, 120))
        .collect();
    let places: Vec<(&str, KeyCode, Vec<AppState>)> = vec![
        (
            "Journal",
            KeyCode::Char('/'),
            vec![place_state("journal row 0")],
        ),
        ("Stash", KeyCode::Char('s'), vec![place_state("stash list")]),
        ("Blame", KeyCode::Char('/'), blame),
    ];
    let mut drawn: Vec<String> = Vec::new();
    let mut reached: Vec<String> = Vec::new();
    for (context, opener, states) in places {
        for state in states {
            for (columns, rows) in [(120, 36), (80, 24), (60, 20)] {
                for shown in [false, true] {
                    let mut state = settled(AppState {
                        size: Size { columns, rows },
                        ..state.clone()
                    });
                    state.detail.shown = shown;
                    let (after, _) = crate::update::update(&state, key(opener));
                    if !after.text_field_open() {
                        continue;
                    }
                    reached.push(context.to_owned());
                    let frame = crate::view::tests::draw(&after, columns, rows);
                    if frame.contains("? keys") || frame.contains("⌃P keys") {
                        drawn.push(format!("{context} at {columns}x{rows}, pane {shown}"));
                    }
                }
            }
        }
    }
    reached.dedup();
    for wanted in ["Journal", "Stash", "Blame"] {
        assert!(
            reached.iter().any(|context| context.starts_with(wanted)),
            "{wanted}'s text field never opened: {reached:?}"
        );
    }
    assert!(
        drawn.is_empty(),
        "a doorway where `?` types:\n  {}",
        drawn.join("\n  ")
    );
}

/// A harness on the pinned clock (m8/19d-b2): this file writes captures,
/// so nothing in it draws against the wall clock
/// (`clock_pin::no_capture_producer_runs_on_the_wall_clock`).
pub(super) fn pinned_harness() -> Harness {
    Harness {
        clock: crate::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW),
        ..Harness::default()
    }
}
