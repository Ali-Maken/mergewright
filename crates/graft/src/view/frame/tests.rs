//! `view::frame`'s own tests. Moved out of an inline `mod tests` that had
//! grown the file past the 400-line rule (PROMPT.md §6.2; M5 groups
//! 12/13 review, M2), the same shape `update::context_menu::verbs` uses.

use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::text_input::TextField;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::boxes::{draw_confirm, draw_credential, overlay_band};
use super::*;
use crate::session::for_tests::session;
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::credential::{CredentialPrompt, PromptField};

const SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 80,
    height: 24,
};

fn menu_rect() -> Rect {
    Rect {
        x: 0,
        y: 4,
        width: 33,
        height: 15,
    }
}

/// With no detail pane the band runs to the right edge of the screen, so
/// nothing the graph drew survives beside the popup
/// (m5-history-and-ghost-preview/12).
#[test]
fn the_band_runs_to_the_end_of_the_whole_area_when_there_is_no_split() {
    let band = overlay_band(SCREEN, menu_rect(), Some(SCREEN));
    assert_eq!(band.x, 0);
    assert_eq!(band.width, 80, "out to the screen's edge");
    assert_eq!(band.y, 4);
    assert_eq!(band.height, 15, "only the popup's own rows");
}

/// With the detail pane open the band stops at the graph's edge: clearing
/// past it would erase the pane the user is reading.
#[test]
fn the_band_stops_at_the_pane_edge_so_the_detail_pane_survives() {
    let graph = Rect {
        x: 0,
        y: 0,
        width: 48,
        height: 24,
    };
    let band = overlay_band(SCREEN, menu_rect(), Some(graph));
    assert_eq!(band.width, 48, "the graph's width, not the screen's");
    assert_eq!(
        band.x + band.width,
        48,
        "the detail pane's first column is untouched"
    );
}

/// A pane that somehow ends before the popup does still gets cleared
/// under the whole popup — the band is never narrower than what it is
/// there to cover.
#[test]
fn the_band_is_never_narrower_than_the_popup_itself() {
    let narrow = Rect {
        x: 0,
        y: 0,
        width: 10,
        height: 24,
    };
    assert_eq!(overlay_band(SCREEN, menu_rect(), Some(narrow)).width, 33);
    assert_eq!(overlay_band(SCREEN, menu_rect(), None).width, 33);
}

/// One glyph that appears nowhere in any overlay, so a cell still
/// holding it is a cell the overlay never wrote.
const DIRT: char = '#';

/// Fills every cell with [`DIRT`], lets `draw` put an overlay on top,
/// and returns what each row of the terminal ended up holding. This is
/// the graph rows underneath, stood in for by something unmistakable.
fn over_a_dirty_screen(
    columns: u16,
    rows: u16,
    draw: impl FnOnce(&mut Frame<'_>, Rect),
) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| {
            let area = frame.area();
            let dirt: String = std::iter::repeat_n(DIRT, usize::from(area.width)).collect();
            for y in area.top()..area.bottom() {
                frame
                    .buffer_mut()
                    .set_string(area.x, y, &dirt, Style::default());
            }
            draw(frame, area);
        })
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// Asserts that nothing the overlay was handed still shows what was
/// under it.
fn nothing_shows_through(drawn: &[String], rect: Rect, what: &str) {
    for y in rect.top()..rect.bottom() {
        let row: String = drawn[usize::from(y)]
            .chars()
            .skip(usize::from(rect.x))
            .take(usize::from(rect.width))
            .collect();
        assert!(
            !row.contains(DIRT),
            "{what}: row {y} of the overlay's own rectangle still shows \
             what was underneath it — {row:?}\n{}",
            drawn.join("\n")
        );
    }
}

fn a_confirm() -> PendingConfirm {
    PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line("Discard changes?"),
            vec![sanitize_line("3 files lose their edits.")],
            sanitize_line("git restore ."),
        ),
        then: ConfirmThen::Undo { redo: false },
    }
}

fn a_credential() -> CredentialPrompt {
    CredentialPrompt {
        title: sanitize_line("Credentials — example.com"),
        prompt: sanitize_line("Username for 'https://example.com':"),
        field: PromptField::Open(TextField::new("")),
        job: JobId::new(1),
    }
}

/// The confirm box owns the rectangle the caller tints for it — at every
/// size, its degraded sizes included. Below two usable rows the widget
/// draws **nothing**, and without the clear the tint simply recolours
/// the graph row that was there (m5-history-and-ghost-preview/13).
#[test]
fn the_confirm_box_never_shows_what_was_under_it() {
    let state = AppState {
        confirm: Some(a_confirm()),
        ..AppState::default()
    };
    let session = session();
    let view = &state.confirm.as_ref().expect("a box is open").view;
    for (columns, rows) in [(80_u16, 24_u16), (64, 9), (40, 4), (20, 3), (80, 2), (3, 2)] {
        let area = Rect {
            x: 0,
            y: 0,
            width: columns,
            height: rows,
        };
        let drawn = over_a_dirty_screen(columns, rows, |frame, area| {
            draw_confirm(&state, frame, area, &session.theme, &[]);
        });
        let rect = graft_ui::confirm::confirm_area(area, view).intersection(area);
        nothing_shows_through(&drawn, rect, &format!("confirm at {columns}x{rows}"));
    }
}

/// The same for git's credential question. Below seven rows the widget
/// draws one *centred* warning and returns, so without the clear the
/// tinted row keeps the graph at both ends of it.
#[test]
fn the_credential_prompt_never_shows_what_was_under_it() {
    let state = AppState {
        credential: Some(a_credential()),
        ..AppState::default()
    };
    let session = session();
    for (columns, rows) in [
        (80_u16, 24_u16),
        (80, 6),
        (60, 4),
        (39, 8),
        (30, 2),
        (80, 1),
    ] {
        let area = Rect {
            x: 0,
            y: 0,
            width: columns,
            height: rows,
        };
        let drawn = over_a_dirty_screen(columns, rows, |frame, area| {
            draw_credential(&state, frame, area, &session);
        });
        let rect = graft_ui::credential_prompt::credential_area(area).intersection(area);
        nothing_shows_through(&drawn, rect, &format!("credential at {columns}x{rows}"));
    }
}

// ─────────── the modal boxes are drawn wherever they are live ───────────

/// The whole frame as `view` draws it, one string per row.
fn whole_frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| view(&session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// A resolver open on a two-block conflict, as `d` finds it.
fn an_open_resolver() -> crate::update::resolver::ResolverState {
    const CONFLICTED: &[u8] = b"one\n\
        <<<<<<< HEAD\nours a\n=======\ntheirs a\n>>>>>>> topic\n\
        two\n\
        <<<<<<< HEAD\nours b\n=======\ntheirs b\n>>>>>>> topic\n";
    crate::update::resolver::ResolverState {
        path: std::path::PathBuf::from("conflicted.rs"),
        opened_from: crate::update::resolver::OpenedFrom::Pane,
        file: Some(std::sync::Arc::new(
            graft_git::conflict::parse(CONFLICTED).expect("two open blocks"),
        )),
        whole_file: None,
        how: None,
        cursor_block: Some(0),
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
    }
}

/// A state with `overlay` standing where the graph would be. The match is
/// exhaustive on purpose: a new `GraphOverlay` stops this file compiling
/// until the sweep below stands a confirm box on it too.
fn standing_on(overlay: Option<GraphOverlay>) -> AppState {
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        ..AppState::default()
    };
    match overlay {
        None => {}
        Some(GraphOverlay::Resolver) => state.resolver = Some(an_open_resolver()),
        Some(GraphOverlay::CommitEditor) => {
            state.commit_editor = Some(crate::update::commit_editor::CommitEditor::new());
        }
        Some(GraphOverlay::Map) => state.map.shown = true,
        Some(GraphOverlay::Journal) => state.journal.shown = true,
        Some(GraphOverlay::Stash) => state.stash.shown = true,
    }
    assert_eq!(state.graph_overlay(), overlay, "the state stands on it");
    state
}

/// **A live confirm box is on screen, whatever it stands on** (m6-07 F10).
///
/// `confirm_key` is the first modal claim in `update::keys::key_event`, so
/// a box in `state.confirm` takes the next key over every overlay and both
/// screens. The resolver's `d` set one that `draw_resolver` never drew: the
/// user pressed `d`, saw nothing change, and the next `→ ⏎` started the
/// file over blind. So the sweep is every overlay `AppState::graph_overlay`
/// can answer, the bare graph, and the status screen — each with the
/// detail pane shut and open, at 120x40 and 80x24 — not the one case the
/// finding named.
#[test]
fn a_live_confirm_box_is_drawn_over_every_overlay() {
    let overlays = [
        None,
        Some(GraphOverlay::Resolver),
        Some(GraphOverlay::CommitEditor),
        Some(GraphOverlay::Map),
        Some(GraphOverlay::Journal),
        Some(GraphOverlay::Stash),
    ];
    let screens = overlays
        .into_iter()
        .map(|overlay| (format!("{overlay:?}"), standing_on(overlay)))
        .chain(std::iter::once((
            "the status screen".to_owned(),
            AppState {
                screen: Screen::Status,
                ..AppState::default()
            },
        )));
    let mut missing = Vec::new();
    for (name, base) in screens {
        for detail in [false, true] {
            for (columns, rows) in [(120_u16, 40_u16), (80, 24)] {
                let mut state = base.clone();
                state.confirm = Some(a_confirm());
                state.detail.shown = detail;
                state.detail.focus = graft_ui::detail::DetailFocus::Files;
                state.size = crate::update::Size { columns, rows };
                let drawn = whole_frame(&state, columns, rows);
                // The box stands over the resolver's document, not over a
                // frame the resolver declined to draw.
                if name.contains("Resolver") {
                    assert!(
                        drawn.iter().any(|row| row.contains("conflicted.rs")),
                        "the resolver is under the box:\n{}",
                        drawn.join("\n")
                    );
                }
                if !drawn.iter().any(|row| row.contains("Discard changes?")) {
                    missing.push(format!(
                        "{name}, detail {detail}, {columns}x{rows}:\n{}",
                        drawn.join("\n")
                    ));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "a live confirm box is not drawn on {} frame(s):\n\n{}",
        missing.len(),
        missing.join("\n\n")
    );
}

// ─────────── the status line on the sub-100-column Detail tab ───────────

/// The changes pane on the Detail tab at 80x24, mid-rebase with one path
/// conflicted, the banner focused or not.
fn detail_tab_mid_rebase(banner_focus: bool) -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        size: crate::update::Size {
            columns: 80,
            rows: 24,
        },
        in_progress: Some(graft_git::state::InProgressDetail {
            kind: graft_git::state::InProgress::Rebase,
            step: Some((1, 2)),
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        banner_focus,
        working_tree: Some(std::sync::Arc::new(graft_git::status::WorkingTreeStatus {
            entries: vec![graft_git::status::StatusEntry {
                path: "a.txt".into(),
                original_path: None,
                staged: None,
                unstaged: None,
                untracked: false,
                unmerged: true,
                unmerged_how: None,
                submodule: false,
            }],
        })),
        detail: crate::update::detail::DetailState {
            shown: true,
            focus: graft_ui::detail::DetailFocus::Files,
            ..crate::update::detail::DetailState::default()
        },
        ..AppState::default()
    }
}

/// **Below 100 columns the Detail tab still says what happened** (m6-07
/// F9). The graph screen's footer is where the status line lives, and on
/// the Detail tab the graph screen is not drawn: the bottom row is the
/// pane's hints, or the focused banner's. The relaunch's `rebase still in
/// progress from a previous run` and the resolver's `all conflicts resolved
/// · c continue` were both on `state.status` and on no cell of the pty's
/// 80x24 frames. Every status kind that line carries — a Sticky, a failure,
/// a finished write — with the banner focused and not.
#[test]
fn the_detail_tab_below_100_columns_draws_the_status_line() {
    let statuses = [
        (
            Status::Sticky {
                reason: "rebase still in progress from a previous run".to_owned(),
                keys: Some("⏎ resolve  c continue  a abort".to_owned()),
            },
            "rebase still in progress from a previous run",
        ),
        (Status::Failed("git said no".to_owned()), "git said no"),
        (
            Status::Done("all conflicts resolved · c continue".to_owned()),
            "all conflicts resolved · c continue",
        ),
    ];
    let mut missing = Vec::new();
    for banner_focus in [true, false] {
        for (status, words) in &statuses {
            let mut state = detail_tab_mid_rebase(banner_focus);
            state.status = status.clone();
            assert!(
                matches!(
                    crate::update::detail::effective_layout(
                        state.detail.focus,
                        &state.detail.pane_size,
                        (80, 24)
                    ),
                    DetailLayout::Tab { shown: true }
                ),
                "the fixture is on the Detail tab"
            );
            let drawn = whole_frame(&state, 80, 24);
            if !drawn.iter().any(|row| row.contains(words)) {
                missing.push(format!(
                    "banner focused {banner_focus}, {words:?}:\n{}",
                    drawn.join("\n")
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the status line is not drawn on {} frame(s):\n\n{}",
        missing.len(),
        missing.join("\n\n")
    );
}

// ──────────── the bottom row under a live box (rulings 07b, Q1) ────────────

/// **The start-over box over the resolver clears its band, full width,
/// at every width** (rulings 07b, Q2; ADR-0040 "What each box does"). The
/// resolver's rows are code, and a cut code line or a side label fused to
/// the box's border (`▏ THEIRS╭ Start both.txt over?`, `m6-09` at 80x24)
/// is the fragment beside a border that reads as content. `d` pressed
/// through `update`, as the user presses it.
#[test]
fn the_start_over_box_clears_its_band_over_the_resolver() {
    let mut wrong = Vec::new();
    for (columns, rows) in [(80_u16, 24_u16), (120, 40)] {
        let mut state = standing_on(Some(GraphOverlay::Resolver));
        state.size = crate::update::Size { columns, rows };
        let (state, _) = crate::update::update(
            &state,
            crate::update::AppEvent::Core(graft_core::event::Event::Key(
                graft_core::event::KeyPress::plain(graft_core::event::KeyCode::Char('d')),
            )),
        );
        assert!(state.confirm.is_some(), "`d` opened the start-over box");
        let drawn = whole_frame(&state, columns, rows);
        let top = drawn
            .iter()
            .position(|row| row.contains('╭'))
            .expect("the box's top border");
        let bottom = drawn
            .iter()
            .position(|row| row.contains('╰'))
            .expect("the box's bottom border");
        for (y, row) in drawn.iter().enumerate().take(bottom + 1).skip(top) {
            let cells: Vec<char> = row.chars().collect();
            let left = cells
                .iter()
                .position(|c| matches!(c, '╭' | '│' | '╰'))
                .expect("a left border on every row of the box");
            let right = cells
                .iter()
                .rposition(|c| matches!(c, '╮' | '│' | '╯'))
                .expect("a right border on every row of the box");
            let outside: String = cells[..left].iter().chain(&cells[right + 1..]).collect();
            if !outside.trim().is_empty() {
                wrong.push(format!("{columns}x{rows} row {y}: {row:?}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "a row of the box keeps the resolver beside it:\n{}",
        wrong.join("\n")
    );
}

/// **A status starts in the hints' column** (rulings 07b, Q3): the graph
/// footer indents its status one space (`graft_ui::graph_screen::footer`),
/// and the hints on every bottom row start at column 1 — so the rows
/// `draw_status_or_hints` draws do the same. A Sticky on the 80x24 Detail
/// tab, the banner focused and not, and in the resolver's footer.
#[test]
fn a_status_on_a_standin_footer_starts_at_column_one() {
    let sticky = Status::Sticky {
        reason: "rebase still in progress from a previous run".to_owned(),
        keys: None,
    };
    let mut states: Vec<(String, AppState, u16, u16)> = [true, false]
        .into_iter()
        .map(|focus| {
            let mut state = detail_tab_mid_rebase(focus);
            state.status = sticky.clone();
            (format!("Detail tab, banner focused {focus}"), state, 80, 24)
        })
        .collect();
    for (columns, rows) in [(80_u16, 24_u16), (120, 40)] {
        let mut state = standing_on(Some(GraphOverlay::Resolver));
        state.size = crate::update::Size { columns, rows };
        state.status = sticky.clone();
        states.push((format!("resolver {columns}x{rows}"), state, columns, rows));
    }
    let mut wrong = Vec::new();
    for (name, state, columns, rows) in states {
        let drawn = whole_frame(&state, columns, rows);
        let Some(row) = drawn
            .iter()
            .find(|row| row.contains("rebase still in progress"))
        else {
            wrong.push(format!("{name}: the status is on no row"));
            continue;
        };
        if !row.starts_with(" rebase still in progress") {
            wrong.push(format!("{name}: {row:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A name prompt for a branch, `⇥` live.
fn a_name_prompt() -> crate::update::name_prompt::NamePrompt {
    crate::update::name_prompt::NamePrompt {
        kind: crate::update::name_prompt::NameKind::Branch {
            at: graft_core::oid::Oid::from_bytes(&[1; graft_core::oid::Oid::SHA1_BYTES])
                .expect("20 bytes"),
            checkout: true,
        },
        field: TextField::new(""),
    }
}

/// Every screen a box can stand on: each overlay, the bare graph and the
/// status screen.
fn every_screen() -> Vec<(String, AppState)> {
    let overlays = [
        None,
        Some(GraphOverlay::Resolver),
        Some(GraphOverlay::CommitEditor),
        Some(GraphOverlay::Map),
        Some(GraphOverlay::Journal),
        Some(GraphOverlay::Stash),
    ];
    overlays
        .into_iter()
        .map(|overlay| (format!("{overlay:?}"), standing_on(overlay)))
        .chain(std::iter::once((
            "the status screen".to_owned(),
            AppState {
                screen: Screen::Status,
                ..AppState::default()
            },
        )))
        .collect()
}

/// A key each screen's own bar offers, one or two per screen: the graph's
/// row, the resolver, the commit editor, the Map view, the journal, the
/// stash list, and the detail pane with an overlay behind it.
const BEHIND: [&str; 8] = [
    "b branch",
    "⏎ resolved",
    "⌃⏎/Commit",
    "zoom to commits",
    "J/esc close",
    "s save",
    "j/k move",
    "/ search",
];

/// **Under a live box every bar offers that box's keys and no other**
/// (rulings 07b, Q1). `confirm_key` and `name_prompt_key` claim
/// the next key before any screen does, so a key of the screen behind —
/// the resolver's `o ours … d start over` under `Start both.txt over?` on
/// `m6-09` — is a key that does nothing. The same sweep as the drawing
/// one above: every overlay, the bare graph and the status screen, the
/// detail pane shut and open, 120x40 and 80x24, both boxes. Status is
/// `Ready`, so the ladder falls through to keys.
#[test]
fn the_bottom_row_under_a_live_box_offers_only_its_keys() {
    /// A box by name, how to open it, and the bar it owns.
    type LiveBox = (&'static str, fn(&mut AppState), &'static str);
    let boxes: [LiveBox; 2] = [
        (
            "confirm",
            |state| state.confirm = Some(a_confirm()),
            " ←/→ move  ⏎ choose  esc cancel",
        ),
        (
            "name prompt",
            |state| state.name_prompt = Some(a_name_prompt()),
            " ⏎ create  ⇥ toggle  esc cancel",
        ),
    ];
    let mut wrong = Vec::new();
    for (which, open, keys) in boxes {
        for (name, base) in every_screen() {
            for detail in [false, true] {
                for (columns, rows) in [(120_u16, 40_u16), (80, 24)] {
                    let mut state = base.clone();
                    open(&mut state);
                    state.detail.shown = detail;
                    state.detail.focus = graft_ui::detail::DetailFocus::Files;
                    state.size = crate::update::Size { columns, rows };
                    let drawn = whole_frame(&state, columns, rows);
                    let behind: Vec<&str> = BEHIND
                        .iter()
                        .copied()
                        .filter(|key| drawn.iter().any(|row| row.contains(key)))
                        .collect();
                    // The status screen has no bar to say them on.
                    let said = name == "the status screen"
                        || drawn.iter().any(|row| row.trim_end() == keys);
                    if !behind.is_empty() || !said {
                        wrong.push(format!(
                            "{which} over {name}, detail {detail}, {columns}x{rows}: \
                             keys behind {behind:?}, box's keys drawn {said}\n{}",
                            drawn.join("\n")
                        ));
                    }
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the bottom row under a live box is not its keys on {} frame(s):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
