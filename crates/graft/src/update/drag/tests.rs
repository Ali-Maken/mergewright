//! Dragging a branch badge, over scripted state: the three-event gesture,
//! what the drop offers, and what choosing one of its verbs runs.
//!
//! Nothing here opens a repository. The fixture is
//! `view::tests::history_of(3, 0)` with `main` on row 1 and `feature` on
//! row 0, the same one `update::context_menu`'s tests use.

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_term::SanitizedText;

use super::*;
use crate::update::events::AppEvent;
use crate::update::update;

fn oid(byte: u8) -> graft_core::oid::Oid {
    graft_core::oid::Oid::from_bytes(&[byte; graft_core::oid::Oid::SHA1_BYTES])
        .expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

/// The badge set a list of refs produces, with no ahead/behind counts —
/// they cost a walk and nothing here draws them.
fn ref_badges(
    refs: &[(RefName, graft_core::oid::Oid)],
    head: &HeadState,
) -> crate::refs::RefBadges {
    crate::refs::badges(
        refs,
        head,
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here")
}

/// Three commits, `feature` on the newest (row 0) and `main` on the next
/// (row 1), with `head` saying which of the two is checked out.
fn fixture(head: &HeadState) -> AppState {
    let refs = ref_badges(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
        ],
        head,
    );
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        size: crate::update::Size {
            columns: 100,
            rows: 24,
        },
        ..AppState::default()
    }
}

/// `main` checked out — so `feature` is a badge on a branch that is *not*
/// current, which is the interesting half of the disabled rules.
fn on_main() -> AppState {
    fixture(&HeadState::Branch(name("refs/heads/main")))
}

/// The screen row an absolute history row is drawn at, and the first column
/// of the branch badge on it. Found by asking the production hit-test,
/// column by column — where the badge is drawn is `graft_ui`'s to say, and
/// a test that hardcoded a column would be asserting its own arithmetic.
fn badge_cell(state: &AppState, row: usize) -> (u16, u16) {
    let screen_row = screen_row_of(state, row);
    let column = (0..state.size.columns)
        .find(|&column| crate::update::geometry::badge_at(state, column, screen_row).is_some())
        .unwrap_or_else(|| panic!("no branch badge on screen row {screen_row}"));
    (column, screen_row)
}

/// The screen row an absolute history row is drawn at.
fn screen_row_of(state: &AppState, row: usize) -> u16 {
    crate::update::geometry::rows_above_graph(state)
        + u16::try_from(row - state.history.as_ref().expect("a history").top).expect("a small row")
}

fn mouse(kind: MouseKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseInput {
        kind,
        column,
        row,
        modifiers: Modifiers::NONE,
    })
}

fn send(state: &AppState, event: Event) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(event))
}

/// Press on `feature`'s badge, move over `main`'s row, release there.
fn drag_feature_onto_main(state: &AppState) -> (AppState, Vec<Command>) {
    let (column, from) = badge_cell(state, 0);
    let onto = screen_row_of(state, 1);
    let (state, _) = send(
        state,
        mouse(MouseKind::Down(MouseButton::Left), column, from),
    );
    let (state, _) = send(
        &state,
        mouse(MouseKind::Drag(MouseButton::Left), column, onto),
    );
    send(
        &state,
        mouse(MouseKind::Up(MouseButton::Left), column, onto),
    )
}

fn labels(state: &AppState) -> Vec<String> {
    state
        .context_menu
        .as_ref()
        .expect("a menu is open")
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str().to_owned())
        .collect()
}

#[test]
fn down_drag_up_opens_the_drop_menu_at_the_target_row() {
    let state = on_main();
    let (after, _) = drag_feature_onto_main(&state);
    let open = after.context_menu.expect("the drop opened a menu");
    assert_eq!(open.row, 1, "the menu belongs to the row it was dropped on");
    assert_eq!(
        open.menu.title.as_ref().map(SanitizedText::as_str),
        Some("feature → main"),
        "the title says what went where"
    );
    assert!(after.drag.is_none(), "the badge is no longer in the air");
}

/// The badge's own row is not a target: the press and the release in the
/// same place is a click, and it moves the cursor there like any other.
#[test]
fn up_on_the_same_row_is_a_click_not_a_drop() {
    let mut state = on_main();
    // Start the cursor somewhere else, so "it moved" is observable.
    state.history.as_mut().expect("a history").cursor = 2;
    let (column, row) = badge_cell(&state, 0);
    let (state, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, row),
    );
    assert!(state.drag.is_some(), "the press picked the badge up");
    let (state, _) = send(&state, mouse(MouseKind::Up(MouseButton::Left), column, row));
    assert!(state.context_menu.is_none(), "no menu opened");
    assert!(state.drag.is_none(), "the gesture is over");
    assert_eq!(
        state.history.expect("a history").cursor,
        0,
        "the press was a plain click and selected the badge's row"
    );
}

#[test]
fn esc_mid_drag_cancels() {
    let state = on_main();
    let (column, from) = badge_cell(&state, 0);
    let onto = screen_row_of(&state, 1);
    let (state, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, from),
    );
    let (state, _) = send(
        &state,
        mouse(MouseKind::Drag(MouseButton::Left), column, onto),
    );
    assert_eq!(
        state.drag.as_ref().and_then(|drag| drag.over),
        Some(1),
        "the badge is over `main`'s row"
    );
    let (state, _) = send(
        &state,
        Event::Key(KeyPress {
            code: KeyCode::Escape,
            modifiers: Modifiers::NONE,
        }),
    );
    assert!(state.drag.is_none(), "`Esc` dropped the badge");
    assert!(state.context_menu.is_none(), "and opened nothing");
}

/// Not current: rebase and cherry-pick say what to do about it, and reset
/// reads `Move …` because that is what `update-ref` does.
#[test]
fn drop_menu_disables_with_reasons() {
    let state = on_main();
    let (after, _) = drag_feature_onto_main(&state);
    let menu = &after.context_menu.as_ref().expect("a menu").menu;
    let row = |label: &str| {
        menu.items
            .iter()
            .find(|item| item.label.as_str() == label)
            .unwrap_or_else(|| panic!("no item {label:?} in {:?}", labels(&after)))
    };
    for label in [
        "Rebase feature onto main",
        "Cherry-pick 03030303 onto feature",
    ] {
        let item = menu
            .items
            .iter()
            .find(|item| {
                item.label
                    .as_str()
                    .starts_with(label.split(' ').next().unwrap())
            })
            .expect("the verb is listed");
        assert_eq!(
            item.disabled.as_ref().map(SanitizedText::as_str),
            Some("check out feature first"),
            "{label} needs its own branch checked out"
        );
    }
    assert!(
        row("Move feature to here").disabled.is_none(),
        "moving a branch that is not checked out needs no checkout"
    );
    assert_eq!(
        row("Merge feature into main")
            .disabled
            .as_ref()
            .map(SanitizedText::as_str),
        None,
        "`main` is checked out, so it can be merged into"
    );
    // The cursor does not start on a row that cannot run.
    assert!(
        menu.selected_refusal().is_none(),
        "the cursor opens on a row that works"
    );
}

/// Every verb ends in a picture. Nothing here may launch a write.
#[test]
fn drop_verbs_build_expected_operations() {
    let state = on_main();
    let (after, _) = drag_feature_onto_main(&state);
    let open = after.context_menu.clone().expect("a menu");
    for (index, item) in open.menu.items.iter().enumerate() {
        if item.disabled.is_some() {
            continue;
        }
        let mut chosen = after.clone();
        chosen
            .context_menu
            .as_mut()
            .expect("a menu")
            .menu
            .cursor
            .clone_from(&index);
        let mut commands = Vec::new();
        assert!(crate::update::context_menu::context_menu_key(
            &mut chosen,
            &mut commands,
            KeyPress {
                code: KeyCode::Enter,
                modifiers: Modifiers::NONE,
            },
        ));
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::PlanPreview(..))),
            "{} asked for a preview, got {commands:?}",
            item.label.as_str()
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::Operate(..))),
            "{} wrote instead of previewing",
            item.label.as_str()
        );
    }
}

/// The operations the four verbs carry, read straight off the pure builder
/// rather than through the menu, so a label change cannot hide a wrong
/// target.
#[test]
fn drop_verbs_name_the_right_refs() {
    let state = on_main();
    let source = DragSource {
        name: "refs/heads/feature".to_owned(),
        short: "feature".to_owned(),
        at: oid(3),
        current: false,
    };
    let open = drop_menu(&state, &source, 1).expect("a menu for `main`'s row");
    let op = |label: &str| {
        let index = open
            .menu
            .items
            .iter()
            .position(|item| item.label.as_str() == label)
            .unwrap_or_else(|| panic!("no item {label:?}"));
        open.verbs[index].op.clone()
    };
    assert!(
        matches!(
            op("Merge feature into main"),
            Operation::Merge(MergeParams { ref into, source_tip, .. })
                if into.name == "refs/heads/main" && into.oid == oid(2) && source_tip == oid(3)
        ),
        "merge goes into `main` at its own commit"
    );
    assert!(
        matches!(
            op("Rebase feature onto main"),
            Operation::Rebase(RebaseParams { ref branch, onto_tip, .. })
                if branch.name == "refs/heads/feature" && onto_tip == oid(2)
        ),
        "rebase replays `feature` onto `main`'s commit"
    );
    assert!(
        matches!(
            op("Move feature to here"),
            Operation::Reset(ResetParams { target: ResetTarget::Branch(ref tip), to })
                if tip.name == "refs/heads/feature" && to == oid(2)
        ),
        "a branch that is not checked out moves by `update-ref`"
    );
    assert!(
        matches!(
            op("Cherry-pick 0202020 onto feature"),
            Operation::CherryPick(PickParams { ref commits, ref head, .. })
                if commits == &[oid(2)] && head.name == "refs/heads/feature"
        ),
        "cherry-pick takes the target commit onto `feature`"
    );
}

/// On the current branch the reset is a real `git reset`, and both modes
/// are offered.
#[test]
fn a_current_branch_drops_two_reset_modes() {
    let state = fixture(&HeadState::Branch(name("refs/heads/feature")));
    let source = DragSource {
        name: "refs/heads/feature".to_owned(),
        short: "feature".to_owned(),
        at: oid(3),
        current: true,
    };
    let open = drop_menu(&state, &source, 1).expect("a menu");
    let labels: Vec<&str> = open
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(
        labels.contains(&"Reset feature to here, mixed")
            && labels.contains(&"Reset feature to here, hard"),
        "{labels:?}"
    );
    assert!(
        !labels.iter().any(|label| label.starts_with("Move ")),
        "the checked-out branch resets rather than moving: {labels:?}"
    );
    // `main` is not checked out here, so merging into it cannot run.
    let merge = open
        .menu
        .items
        .iter()
        .find(|item| item.label.as_str() == "Merge feature into main")
        .expect("merge is listed");
    assert_eq!(
        merge.disabled.as_ref().map(SanitizedText::as_str),
        Some("check out main first")
    );
}

/// A modal box owns the screen; there is no badge under the pointer to
/// pick up, whatever is drawn behind it.
#[test]
fn drag_cannot_start_over_a_preview() {
    let mut state = on_main();
    let (column, row) = badge_cell(&state, 0);
    // Proof the cell really does carry a badge, so the assertion below is
    // about the preview and not about an empty column.
    assert!(crate::update::geometry::badge_at(&state, column, row).is_some());

    let mut commands = Vec::new();
    let mut counter = 0_u64;
    let opened = crate::update::preview::open_preview(
        &mut state,
        &mut commands,
        crate::update::preview::PreviewSubject::Operation {
            op: Box::new(Operation::Reset(ResetParams {
                target: ResetTarget::Branch(Tip {
                    name: "refs/heads/feature".to_owned(),
                    oid: oid(3),
                }),
                to: oid(2),
            })),
            opened_by: ".",
        },
        false,
        &mut || {
            counter += 1;
            graft_core::job::JobId::new(crate::update::detail::LOCAL_JOB_BIT | counter)
        },
    );
    assert!(opened, "the preview opened");

    let (after, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, row),
    );
    assert!(after.drag.is_none(), "no badge was picked up over the box");
}

/// A drag cannot start while the search prompt owns the keyboard: ticket 10
/// piece 4 is explicit that a prompt open rules a drag out, and a hand-listed
/// `can_start` omitted it (review m5-10-30-31 m1) — the prompt then claimed
/// every key, so `Esc` mid-drag never reached `drag_key` and the footer kept
/// advertising `release drop on a row` to someone typing a search.
#[test]
fn drag_cannot_start_while_the_search_prompt_is_open() {
    let mut state = on_main();
    let (column, row) = badge_cell(&state, 0);
    state.search_prompt = Some(String::new());

    let (after, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, row),
    );
    assert!(
        after.drag.is_none(),
        "no badge was picked up over the prompt"
    );
}

/// Same rule for a focused banner: it owns every key while it has focus
/// (`update::banner`), so a drag started under it would strand `Esc` the
/// same way the search prompt did.
#[test]
fn drag_cannot_start_while_a_banner_has_focus() {
    let mut state = on_main();
    let (column, row) = badge_cell(&state, 0);
    state.banner_focus = true;

    let (after, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, row),
    );
    assert!(
        after.drag.is_none(),
        "no badge was picked up under the focused banner"
    );
}

/// Dragging sideways into the detail pane at the same screen row must not
/// keep the graph row as the target: `over` used to come from `row` alone
/// (`geometry::graph_row_at`), so releasing over the pane opened the drop
/// menu for a row the pointer was never on (review m5-10-30-31 m3).
#[test]
fn dragging_into_the_detail_pane_gives_no_target() {
    let mut state = on_main();
    state.detail.shown = true;
    // 160+ columns is `DetailLayout::Right` (`detail_pane/layout.rs`): the
    // pane sits beside the graph, not below it, so the same screen row has
    // graph columns and pane columns to tell apart. Below 160 the pane is a
    // strip under the graph or a full-width tab, and every column on a
    // graph row's height is still the graph.
    state.size = crate::update::Size {
        columns: 160,
        rows: 24,
    };
    let (column, row) = badge_cell(&state, 0);
    let (state, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, row),
    );
    assert!(state.drag.is_some(), "the press picked up a badge");

    let graph = crate::update::geometry::context_menu_graph_bounds(&state)
        .expect("the graph has a rectangle while it is on screen");
    let pane_column = graph.x + graph.width;
    assert!(
        pane_column < state.size.columns,
        "there is room for a detail pane beside the graph at this width"
    );

    let (after, _) = send(
        &state,
        mouse(MouseKind::Drag(MouseButton::Left), pane_column, row),
    );
    assert_eq!(
        after.drag.expect("still dragging").over,
        None,
        "the pane's columns at the graph's row height are not a graph target"
    );
}

/// A resize cancels a drag (ticket 10 piece 4, review m5-10-30-31 m2): the
/// badge's row and column are read off the layout the pointer moved over,
/// and a resize is exactly the event that changes what that layout is.
#[test]
fn resize_cancels_a_drag() {
    let state = on_main();
    let (column, row) = badge_cell(&state, 0);
    let (state, _) = send(
        &state,
        mouse(MouseKind::Down(MouseButton::Left), column, row),
    );
    assert!(state.drag.is_some(), "the press picked up a badge");

    let (after, _) = send(
        &state,
        Event::Resize {
            columns: state.size.columns,
            rows: state.size.rows,
        },
    );
    assert!(after.drag.is_none(), "the resize dropped the badge");
}

/// Only a local branch is draggable: §2.15 says a branch badge, and the
/// other four badge kinds name nothing this repository may move.
#[test]
fn badge_at_ignores_remote_tag_stash_and_head() {
    let refs = ref_badges(
        &[
            (name("refs/heads/main"), oid(1)),
            (name("refs/remotes/origin/feature"), oid(3)),
            (name("refs/tags/v1"), oid(2)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
    );
    let state = AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        size: crate::update::Size {
            columns: 100,
            rows: 24,
        },
        ..AppState::default()
    };
    // Rows 0 and 1 carry the remote and the tag; only row 2 has a branch.
    for row in [0_usize, 1] {
        let screen_row = screen_row_of(&state, row);
        assert!(
            (0..state.size.columns).all(|column| crate::update::geometry::badge_at(
                &state, column, screen_row
            )
            .is_none()),
            "row {row} carries no draggable badge"
        );
    }
    let (_, screen_row) = badge_cell(&state, 2);
    let source = (0..state.size.columns)
        .find_map(|column| crate::update::geometry::badge_at(&state, column, screen_row))
        .expect("`main` is draggable");
    assert_eq!(source.short, "main");
    assert!(source.current, "`main` is checked out");
}
