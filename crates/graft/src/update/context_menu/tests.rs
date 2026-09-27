//! What the `.` context menu offers on each row, and what choosing one runs.
//!
//! Scripted: nothing here opens a repository. The fixture builders live here
//! rather than in each test file — `verbs/tests.rs` reads
//! [`ref_badges`] from this module so the two agree on what a row's refs are.

use graft_core::event::Modifiers;

use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn next_id_from(counter: &mut u64) -> JobId {
    *counter += 1;
    JobId::new(super::super::detail::LOCAL_JOB_BIT | *counter)
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

/// `history_of(3, 0)`'s cursor row, with a ready radar context and a
/// duplicate-commit finding pinned to `mark` — the same fixture shape
/// `update::detail`'s explain tests use, so this menu and that panel
/// agree on what "marked" means.
fn state_with_finding_at(mark: Oid) -> AppState {
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        ..AppState::default()
    };
    state.radar.context = crate::update::detail::Loading::Ready(std::sync::Arc::new(
        crate::jobs::radar::RadarContext::of_ids([], Vec::new()),
    ));
    state.radar.duplicate_findings.insert(
        mark,
        graft_ops::mistake_radar::Finding {
            kind: graft_core::mistake_radar::MistakeRadarKind::DuplicateCommit,
            severity: graft_core::mistake_radar::Severity::Warning,
            at: mark,
            evidence: graft_ops::mistake_radar::Evidence::Duplicate {
                copies: vec![oid(9)],
            },
        },
    );
    state
}

/// `. more` says the keys the bar dropped are behind `.`, so choosing
/// one there must run what the key runs — the same function, not a
/// second implementation (m5-03 review, Major 1).
#[test]
fn the_menu_runs_what_the_dropped_keys_run() {
    for (action, code) in [
        (ContextAction::Fetch, KeyCode::Char('f')),
        (ContextAction::Pull, KeyCode::Char('p')),
        (ContextAction::Push, KeyCode::Char('P')),
        (ContextAction::Undo, KeyCode::Char('u')),
    ] {
        let mut from_menu = AppState::default();
        let by_menu = dispatch(&mut from_menu, WORKING_TREE_ROW, &[], action);

        let mut from_key = AppState::default();
        let mut by_key = Vec::new();
        super::super::keys::key_event(&mut from_key, &mut by_key, key(code));

        assert_eq!(by_menu, by_key, "{action:?} vs {code:?}");
        assert_eq!(
            from_menu.status, from_key.status,
            "{action:?} vs {code:?} left different status lines"
        );
        assert_eq!(
            from_menu.operating.is_some(),
            from_key.operating.is_some(),
            "{action:?} vs {code:?}"
        );
    }
    // Not vacuous: `f` really does launch a fetch from a bare state.
    let mut state = AppState::default();
    assert!(!dispatch(&mut state, WORKING_TREE_ROW, &[], ContextAction::Fetch).is_empty());
}

/// Every menu lists them, whichever row opened it — they act on `HEAD`
/// and the current branch, not on the row.
#[test]
fn every_menu_lists_the_dropped_keys() {
    for menu in [
        ContextMenu::for_row(false, Vec::new(), 0, None),
        ContextMenu::for_row(true, Vec::new(), 0, None),
        ContextMenu::for_working_tree(),
    ] {
        for action in [
            ContextAction::Fetch,
            ContextAction::Pull,
            ContextAction::Push,
            ContextAction::Undo,
        ] {
            assert!(
                menu.items.iter().any(|item| item.action == action),
                "{action:?} is missing from {:?}",
                menu.items
            );
        }
        assert!(
            menu.items
                .iter()
                .all(|item| item.label.as_str().contains('(')
                    || item.action == ContextAction::Explain),
            "every item but Explain names its key: {:?}",
            menu.items
        );
    }
}

#[test]
fn open_for_history_row_lists_explain_when_the_row_has_a_finding() {
    let mark = oid(3); // `history_of(3, 0)`'s row 0.
    let mut state = state_with_finding_at(mark);

    open_for_history_row(&mut state, 0);

    let open = state.context_menu.expect("menu opened");
    let menu = &open.menu;
    assert_eq!(open.row, 0);
    assert!(
        menu.items
            .iter()
            .any(|item| item.action == ContextAction::Explain),
        "the row under the cursor has a finding, same source `marker_for` reads"
    );
}

#[test]
fn open_for_history_row_omits_explain_on_an_unflagged_row() {
    // A ready radar context with no finding anywhere: `findings_for`
    // returns empty for every row, so `Explain` never lists.
    let mut state = state_with_finding_at(oid(99)); // no row is `oid(99)`.

    open_for_history_row(&mut state, 0);

    let open = state.context_menu.expect("menu opened");
    let menu = &open.menu;
    assert_eq!(open.row, 0);
    assert!(
        !menu
            .items
            .iter()
            .any(|item| item.action == ContextAction::Explain),
        "no finding at this row"
    );
}

#[test]
fn open_for_history_row_omits_explain_without_a_ready_radar_context() {
    // No `History` at all: `row_at` finds nothing, so `marked` is
    // `false` rather than panicking or guessing.
    let mut state = AppState::default();

    open_for_history_row(&mut state, 3);

    let open = state.context_menu.expect("menu opened");
    let menu = &open.menu;
    assert_eq!(open.row, 3);
    assert!(
        !menu
            .items
            .iter()
            .any(|item| item.action == ContextAction::Explain),
        "no history means no row to look a finding up on"
    );
}

/// The working-tree row's own action is `Open details` and nothing
/// else — `Copy SHA` and `Compare with this` have no commit to act on.
/// The dropped keys follow it (`every_menu_lists_the_dropped_keys`).
#[test]
fn open_for_working_tree_lists_open_details_first() {
    let mut state = AppState::default();
    open_for_working_tree(&mut state);
    let open = state.context_menu.expect("menu opened");
    let menu = &open.menu;
    assert_eq!(open.row, WORKING_TREE_ROW);
    assert_eq!(menu.items[0].action, ContextAction::OpenDetails);
    assert!(
        !menu.items.iter().any(|item| matches!(
            item.action,
            ContextAction::CopySha | ContextAction::CompareWithThis | ContextAction::Explain
        )),
        "the working-tree row names no commit: {:?}",
        menu.items
    );
}

#[test]
fn open_selected_dispatches_by_on_working_tree_flag() {
    let mut state = AppState {
        on_working_tree: true,
        ..AppState::default()
    };
    open_selected(&mut state);
    let open = state.context_menu.expect("menu opened");
    assert_eq!(open.row, WORKING_TREE_ROW);
}

#[test]
fn open_selected_does_nothing_without_a_history_row() {
    // Not on the working-tree row and no `History` to name a row from.
    let mut state = AppState::default();
    open_selected(&mut state);
    assert!(state.context_menu.is_none());
}

#[test]
fn context_menu_key_moves_cursor() {
    let mut state = AppState::default();
    open_for_history_row(&mut state, 0);
    let mut commands = Vec::new();
    assert!(context_menu_key(
        &mut state,
        &mut commands,
        key(KeyCode::Char('j'))
    ));
    let open = state.context_menu.as_ref().expect("still open");
    assert_eq!(open.menu.cursor, 1);
}

#[test]
fn context_menu_key_esc_closes_without_dispatching() {
    let mut state = AppState::default();
    open_for_history_row(&mut state, 0);
    let mut commands = Vec::new();
    assert!(context_menu_key(
        &mut state,
        &mut commands,
        key(KeyCode::Escape)
    ));
    assert!(state.context_menu.is_none());
    assert!(commands.is_empty());
}

/// A verb key closes the menu and is **handed on**: `false` is what tells
/// `keys::key_event` to carry on down to `graph_key`, so `m` on a row
/// whose label reads `Merge main into feature (m)` really merges
/// (`docs/design/02-keymap.md` §2.1). Nothing is dispatched *here* — the
/// verb runs where it always runs (m5-history-and-ghost-preview/13).
#[test]
fn context_menu_key_closes_and_hands_a_verb_key_on() {
    for code in [
        KeyCode::Char('y'),
        KeyCode::Char('x'),
        KeyCode::Char('m'),
        KeyCode::Char('r'),
        KeyCode::Char('f'),
        KeyCode::Char('p'),
        KeyCode::Char('P'),
        KeyCode::Char('u'),
    ] {
        let mut state = AppState::default();
        open_for_history_row(&mut state, 0);
        let mut commands = Vec::new();
        assert!(
            !context_menu_key(&mut state, &mut commands, key(code)),
            "{code:?} must fall through to the graph's own handler"
        );
        assert!(
            state.context_menu.is_none(),
            "{code:?} must close the menu on its way past"
        );
        assert!(
            commands.is_empty(),
            "{code:?} dispatches nothing here: the verb runs downstream"
        );
    }
}

/// The keys the menu claims stop here (`true`). `Enter`, the fifth, is
/// pinned by `choose` below — it needs a menu with something real to
/// dispatch, which this bare state has not got.
#[test]
fn context_menu_key_claims_its_movement_and_esc() {
    for code in [
        KeyCode::Char('j'),
        KeyCode::Down,
        KeyCode::Char('k'),
        KeyCode::Up,
        KeyCode::Escape,
        KeyCode::Char('.'),
    ] {
        let mut state = AppState::default();
        open_for_history_row(&mut state, 0);
        let mut commands = Vec::new();
        assert!(
            context_menu_key(&mut state, &mut commands, key(code)),
            "{code:?} is the menu's own key and stops here"
        );
    }
}

#[test]
fn context_menu_key_returns_false_when_closed() {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    assert!(!context_menu_key(
        &mut state,
        &mut commands,
        key(KeyCode::Char('j'))
    ));
}

#[test]
fn apply_copy_sha_matches_ys_command_shape() {
    let mut state = AppState::default();
    let mut ids = 0u64;
    let commands = apply_context_action(
        &mut state,
        ContextAction::CopySha,
        0,
        Some(oid(1)),
        &[],
        &mut || next_id_from(&mut ids),
    );
    assert_eq!(commands, vec![Command::CopyToClipboard(oid(1).to_string())]);
    assert_eq!(
        state.status,
        Status::Copied {
            short: oid(1).to_string().chars().take(7).collect()
        }
    );
}

#[test]
fn apply_compare_with_this_calls_toggle_compare() {
    let mut state = AppState::default();
    let mut ids = 0u64;
    let commands = apply_context_action(
        &mut state,
        ContextAction::CompareWithThis,
        0,
        Some(oid(1)),
        &[],
        &mut || next_id_from(&mut ids),
    );
    // `toggle_compare`'s first branch: pins the base and opens it (m7-14).
    assert!(!commands.is_empty());
    assert_eq!(state.compare_base.map(|base| base.oid), Some(oid(1)));
}

#[test]
fn apply_open_details_opens_commit_on_a_commit_row() {
    let mut state = AppState::default();
    let mut ids = 0u64;
    apply_context_action(
        &mut state,
        ContextAction::OpenDetails,
        0,
        Some(oid(1)),
        &[],
        &mut || next_id_from(&mut ids),
    );
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(oid(1))));
}

#[test]
fn apply_open_details_opens_working_tree_when_oid_is_none() {
    let mut state = AppState::default();
    let mut ids = 0u64;
    apply_context_action(
        &mut state,
        ContextAction::OpenDetails,
        WORKING_TREE_ROW,
        None,
        &[],
        &mut || next_id_from(&mut ids),
    );
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
}

/// `main` at commit 2 with `HEAD` on it, `feature` at commit 3 — which
/// is row 0 of `history_of(3, 0)`.
/// The badges a set of refs makes, with no ahead/behind walk.
///
/// Shared with `verbs/tests.rs`: a verb is offered on the strength of what a
/// row's badges say, so the two files have to be reading the same shape.
pub(super) fn ref_badges(
    refs: &[(graft_core::ref_name::RefName, Oid)],
    head: &graft_git::repo::HeadState,
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

fn refs_over_the_fixture() -> std::sync::Arc<crate::refs::RefBadges> {
    let name = |full: &str| {
        graft_core::ref_name::RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
    };
    std::sync::Arc::new(ref_badges(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
        ],
        &graft_git::repo::HeadState::Branch(name("refs/heads/main")),
    ))
}

fn on_the_feature_row() -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(refs_over_the_fixture()),
        ..AppState::default()
    }
}

/// Puts the cursor on the item with this label and presses `Enter`.
fn choose(state: &mut AppState, label: &str) -> Vec<Command> {
    let open = state.context_menu.as_mut().expect("a menu is open");
    open.menu.cursor = open
        .menu
        .items
        .iter()
        .position(|item| item.label.as_str() == label)
        .unwrap_or_else(|| {
            panic!(
                "no item labelled {label:?} in {:?}",
                open.menu
                    .items
                    .iter()
                    .map(|item| item.label.as_str())
                    .collect::<Vec<_>>()
            )
        });
    let mut commands = Vec::new();
    assert!(context_menu_key(state, &mut commands, key(KeyCode::Enter)));
    commands
}

/// The row's history verbs are listed between its own actions and the
/// dropped keys, and every one of them is a `History` index into the
/// list the app built — the widget names no operation of its own.
#[test]
fn a_branch_row_lists_its_history_verbs_between_its_actions_and_the_dropped_keys() {
    let mut state = on_the_feature_row();
    open_for_history_row(&mut state, 0);
    let open = state.context_menu.as_ref().expect("menu opened");

    let listed: Vec<&str> = open
        .menu
        .items
        .iter()
        .filter(|item| matches!(item.action, ContextAction::History(_)))
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(
        listed,
        vec![
            "Checkout feature",
            "Cherry-pick 0303030",
            "Merge feature into main (m)",
            "Rebase main onto feature (r)",
            "Revert 0303030",
            "Reset main to here, mixed",
            "Reset main to here, hard",
            "Delete branch feature",
        ]
    );
    // Each index points at the verb with that label.
    for (item, verb) in open.menu.items.iter().filter_map(|item| match item.action {
        ContextAction::History(index) => Some((item, &open.verbs[index])),
        _ => None,
    }) {
        assert_eq!(item.label, verb.label);
    }
    let positions: Vec<usize> = open
        .menu
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| matches!(item.action, ContextAction::History(_)))
        .map(|(at, _)| at)
        .collect();
    // m6-conflicts/71: Checkout and Cherry-pick lead; the rest follow the
    // row's own actions.
    assert_eq!(
        &positions[..3],
        &[0, 1, 5],
        "two slots, then after the row's own actions"
    );
    assert_eq!(open.menu.items[4].action, ContextAction::OpenDetails);
    assert_eq!(
        open.menu.items[positions.last().copied().expect("some verbs") + 1].action,
        ContextAction::Fetch,
        "and before the dropped keys"
    );
}

/// Choosing a verb opens the Ghost Preview of the very operation its
/// label named, and runs nothing: `.` ends in a picture, and only
/// `Confirm` in that picture writes (PROMPT.md §1.7).
#[test]
fn a_history_verb_previews_the_operation_its_label_named_and_runs_nothing() {
    let mut state = on_the_feature_row();
    open_for_history_row(&mut state, 0);
    let expected = state
        .context_menu
        .as_ref()
        .expect("menu opened")
        .verbs
        .iter()
        .find(|verb| verb.label.as_str() == "Merge feature into main (m)")
        .expect("the verb is listed")
        .op
        .clone();

    let commands = choose(&mut state, "Merge feature into main (m)");

    assert!(state.context_menu.is_none(), "the menu closed");
    let preview = state.preview.as_ref().expect("a preview opened");
    assert_eq!(
        preview.subject,
        super::super::preview::PreviewSubject::Operation {
            op: Box::new(expected),
            opened_by: ".",
        },
        "the value that will run is the value the picture is drawn from"
    );
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::Operate(..))),
        "nothing ran: {commands:?}"
    );
    assert!(state.operating.is_none());
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::PlanPreview(..)))
            .count(),
        1,
        "{commands:?}"
    );
}

/// Every verb the menu offers really opens a preview — none of them is a
/// label with nothing behind it.
#[test]
fn every_history_verb_the_menu_lists_opens_a_preview() {
    let mut probe = on_the_feature_row();
    open_for_history_row(&mut probe, 0);
    let labels: Vec<String> = probe
        .context_menu
        .as_ref()
        .expect("menu opened")
        .verbs
        .iter()
        .map(|verb| verb.label.as_str().to_owned())
        .collect();
    assert!(!labels.is_empty());

    for label in labels {
        let mut state = on_the_feature_row();
        open_for_history_row(&mut state, 0);
        let commands = choose(&mut state, &label);
        assert!(
            state.preview.is_some(),
            "{label} opened no preview: {:?}",
            state.status
        );
        assert!(
            commands
                .iter()
                .all(|command| !matches!(command, Command::Operate(..))),
            "{label} ran something: {commands:?}"
        );
    }
}

/// The working-tree row names no commit, so it offers no history verb —
/// and nothing in the menu can reach one.
#[test]
fn the_working_tree_row_offers_no_history_verb() {
    let mut state = on_the_feature_row();
    state.on_working_tree = true;
    open_selected(&mut state);
    let open = state.context_menu.as_ref().expect("menu opened");
    assert!(open.verbs.is_empty());
    assert!(
        !open
            .menu
            .items
            .iter()
            .any(|item| matches!(item.action, ContextAction::History(_))),
        "{:?}",
        open.menu.items
    );
}

/// A verb chosen while a write is already running is refused in words
/// and starts nothing — `open_preview`'s own refusal, reached through
/// the menu.
#[test]
fn a_verb_chosen_while_a_write_runs_is_refused_in_words() {
    let mut state = on_the_feature_row();
    state.operating = Some(JobId::new(7));
    open_for_history_row(&mut state, 0);

    let commands = choose(&mut state, "Merge feature into main (m)");

    assert!(state.preview.is_none());
    assert!(commands.is_empty(), "{commands:?}");
    assert!(
        matches!(&state.status, Status::Sticky { reason, .. } if reason.starts_with("wait for")),
        "{:?}",
        state.status
    );
}

#[test]
fn apply_explain_sets_the_flag_and_returns_no_command() {
    let mut state = AppState::default();
    let mut ids = 0u64;
    let commands = apply_context_action(
        &mut state,
        ContextAction::Explain,
        0,
        Some(oid(1)),
        &[],
        &mut || next_id_from(&mut ids),
    );
    assert!(commands.is_empty());
    assert_eq!(state.detail.explain_requested, Some(oid(1)));
}

/// `.` closes the menu it opened, pressed through the whole of `update`
/// rather than through `context_menu_key` alone — which is where the bug
/// was. Group 13 made every advertised key fall through, correctly closing
/// the menu on `x m r u y f p P`; `.` falling through the same way reached
/// `keys::graph_key`, whose `.` reopened the menu it had just closed, so
/// `.` could never shut it. §2.1: a key that opens a place closes it again
/// on the same letter or `Esc` (m5-history-and-ghost-preview/15).
#[test]
fn dot_closes_the_menu_it_opened() {
    use graft_core::event::Event;

    // A real history row, not the working-tree row: `selection::reconcile`
    // clears `on_working_tree` at the end of every event when there is
    // nothing uncommitted to show, which would make the second `.` find
    // nothing to reopen and pass this test for the wrong reason.
    let state = on_the_feature_row();
    let (opened, _) = crate::update::update(
        &state,
        crate::update::AppEvent::Core(Event::Key(key(KeyCode::Char('.')))),
    );
    assert!(opened.context_menu.is_some(), "`.` opens the menu");

    let (closed, commands) = crate::update::update(
        &opened,
        crate::update::AppEvent::Core(Event::Key(key(KeyCode::Char('.')))),
    );
    assert!(
        closed.context_menu.is_none(),
        "`.` reopened what `.` had just closed"
    );
    assert!(
        commands.is_empty(),
        "closing the menu ran something: {commands:?}"
    );
}

// ---------------------------------------------------------------------------
// m6-conflicts/71: Checkout and Cherry-pick lead the menu.
// ---------------------------------------------------------------------------

fn actions(state: &AppState) -> Vec<ContextAction> {
    state
        .context_menu
        .as_ref()
        .expect("a menu is open")
        .menu
        .items
        .iter()
        .map(|item| item.action)
        .collect()
}

/// A marked row: the two slots first, `Explain` right after them.
#[test]
fn explain_comes_after_the_slots_on_a_marked_row() {
    let mut state = state_with_finding_at(oid(3)); // `history_of(3, 0)`'s row 0.
    state.refs = Some(refs_over_the_fixture());
    open_for_history_row(&mut state, 0);
    let got = actions(&state);
    assert_eq!(
        &got[..6],
        &[
            ContextAction::History(0),
            ContextAction::History(1),
            ContextAction::Explain,
            ContextAction::CopySha,
            ContextAction::CompareWithThis,
            ContextAction::OpenDetails,
        ]
    );
}

/// The uncommitted-changes row: `Open details`, then the dropped keys —
/// no slots, no dim item.
#[test]
fn the_uncommitted_row_menu_is_unchanged() {
    let mut state = on_the_feature_row();
    open_for_working_tree(&mut state);
    assert_eq!(
        actions(&state),
        vec![
            ContextAction::OpenDetails,
            ContextAction::Fetch,
            ContextAction::Pull,
            ContextAction::Push,
            ContextAction::Undo,
        ]
    );
}

/// A stash row keeps the order it had: the row's own actions first, then
/// Checkout, then Cherry-pick after the merge/rebase items (none here).
#[test]
fn a_stash_row_menu_is_unchanged() {
    let name = |full: &str| {
        graft_core::ref_name::RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
    };
    let refs = crate::refs::badges(
        &[(name("refs/heads/main"), oid(2))],
        &graft_git::repo::HeadState::Branch(name("refs/heads/main")),
        &std::collections::HashMap::new(),
        &[(oid(3), 0)],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        ..AppState::default()
    };
    open_for_history_row(&mut state, 0);
    let got = actions(&state);
    assert_eq!(
        &got[..4],
        &[
            ContextAction::CopySha,
            ContextAction::CompareWithThis,
            ContextAction::OpenDetails,
            ContextAction::History(0),
        ]
    );
    let open = state.context_menu.as_ref().expect("menu opened");
    let labels: Vec<&str> = open.verbs.iter().map(|verb| verb.label.as_str()).collect();
    // Today's order: Checkout, then `m`/`r`'s items, then Cherry-pick.
    let at = |wanted: &str| {
        labels
            .iter()
            .position(|label| *label == wanted)
            .unwrap_or_else(|| panic!("{wanted:?} missing from {labels:?}"))
    };
    assert_eq!(at("Checkout 0303030, detached"), 0, "{labels:?}");
    assert!(
        at("Merge 0303030 into main (m)") < at("Cherry-pick 0303030"),
        "{labels:?}"
    );
    assert!(open.menu.items.iter().all(|item| item.disabled.is_none()));
}

/// `HEAD`'s row, `main` only: slot 1 is the dim `Checkout — already on
/// main`; `Enter` there keeps the menu open and puts the reason on the
/// status line. Built in m5/10; this pins that the dim slot reaches it.
#[test]
fn enter_on_the_dim_checkout_slot_keeps_the_menu_and_says_why() {
    let mut state = on_the_feature_row();
    open_for_history_row(&mut state, 1); // `oid(2)`, main's tip.
    {
        let open = state.context_menu.as_ref().expect("menu opened");
        assert_eq!(open.menu.items[0].action, ContextAction::Unavailable);
        assert_eq!(open.menu.items[0].label.as_str(), "Checkout");
        assert_eq!(
            open.menu.items[0]
                .disabled
                .as_ref()
                .map(graft_term::SanitizedText::as_str),
            Some("already on main")
        );
        assert_eq!(open.menu.items[1].label.as_str(), "Cherry-pick 0202020");
        assert_eq!(open.menu.cursor, 0);
    }
    let mut commands = Vec::new();
    assert!(context_menu_key(
        &mut state,
        &mut commands,
        key(KeyCode::Enter)
    ));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.context_menu.is_some(), "the menu stays open");
    assert!(state.preview.is_none());
    assert!(
        format!("{:?}", state.status).contains("already on main"),
        "{:?}",
        state.status
    );
}

/// Every enabled slot runs its own verb: `Enter` on the item at slot `n`
/// previews the operation of the verb that item names.
#[test]
fn enter_on_each_enabled_slot_runs_its_own_verb() {
    for row in [0, 1] {
        let mut probe = on_the_feature_row();
        open_for_history_row(&mut probe, row);
        let open = probe.context_menu.as_ref().expect("menu opened");
        let enabled: Vec<(usize, usize)> = open
            .menu
            .items
            .iter()
            .enumerate()
            .filter_map(|(at, item)| match item.action {
                ContextAction::History(index) => Some((at, index)),
                _ => None,
            })
            .collect();
        for (at, index) in enabled {
            let mut state = on_the_feature_row();
            open_for_history_row(&mut state, row);
            let open = state.context_menu.as_ref().expect("menu");
            // The label drawn at slot `n` is the verb that runs, not only
            // the index it carries.
            assert_eq!(
                open.menu.items[at].label, open.verbs[index].label,
                "row {row}, item {at}"
            );
            let expected = open.verbs[index].op.clone();
            state.context_menu.as_mut().expect("menu").menu.cursor = at;
            let mut commands = Vec::new();
            assert!(context_menu_key(
                &mut state,
                &mut commands,
                key(KeyCode::Enter)
            ));
            let preview = state.preview.as_ref().expect("a preview opened");
            assert_eq!(
                preview.subject,
                crate::update::preview::PreviewSubject::Operation {
                    op: Box::new(expected),
                    opened_by: ".",
                },
                "row {row}, item {at}"
            );
        }
    }
}
