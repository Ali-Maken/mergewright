//! What the `.` context menu offers on each row, and what choosing one runs.
//!
//! Scripted: nothing here opens a repository. The fixture builders live here
//! rather than in each test file — `verbs/tests.rs` reads
//! [`ref_badges`] from this module so the two agree on what a row's refs are.

use crate::spelled::Spelled as _;
use graft_core::event::Modifiers;
use graft_ops::operation::Operation;

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

/// `? keys` says the keys the bar dropped are behind `.`, so choosing
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
        ContextMenu::for_row(
            false,
            Vec::new(),
            0,
            None,
            graft_ui::keymap::Keymap::shipped(),
        ),
        ContextMenu::for_row(
            true,
            Vec::new(),
            0,
            None,
            graft_ui::keymap::Keymap::shipped(),
        ),
        ContextMenu::for_working_tree(false, graft_ui::keymap::Keymap::shipped()),
        ContextMenu::for_working_tree(true, graft_ui::keymap::Keymap::shipped()),
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

/// The working-tree row's own actions are absorb (first, 09b) and `Open
/// details` — `Copy SHA` and `Compare with this` have no commit to act on.
/// The dropped keys follow (`every_menu_lists_the_dropped_keys`).
#[test]
fn open_for_working_tree_lists_absorb_then_open_details() {
    let mut state = AppState::default();
    open_for_working_tree(&mut state);
    let open = state.context_menu.expect("menu opened");
    let menu = &open.menu;
    assert_eq!(open.row, WORKING_TREE_ROW);
    assert_eq!(menu.items[0].action, ContextAction::Absorb);
    assert_eq!(menu.items[1].action, ContextAction::OpenDetails);
    assert!(
        !menu.items.iter().any(|item| matches!(
            item.action,
            ContextAction::CopySha | ContextAction::CompareWithThis | ContextAction::Explain
        )),
        "the working-tree row names no commit: {:?}",
        menu.items
    );
}

/// m7-power-tools/09: the absorb item (first since 09b) is dim with nothing
/// staged and live once something is — read from the working tree the
/// screen holds.
#[test]
fn the_working_tree_menu_offers_absorb_when_something_is_staged() {
    let absorb = |state: &mut AppState| {
        open_for_working_tree(state);
        let open = state.context_menu.take().expect("menu opened");
        let first = open.menu.items.first().expect("items").clone();
        assert_eq!(first.action, ContextAction::Absorb);
        first.disabled.map(|why| why.as_str().to_owned())
    };
    let mut state = AppState::default();
    assert_eq!(absorb(&mut state).as_deref(), Some("nothing staged"));
    state.working_tree = Some(std::sync::Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![graft_git::status::StatusEntry {
            path: "a.rs".into(),
            original_path: None,
            staged: Some(graft_git::status::Change::Modified),
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    }));
    assert_eq!(absorb(&mut state), None);
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
            "Cherry-pick 0303030 (o)",
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

/// A branch checkout on a clean tree switches at once (the switch rule,
/// m7-power-tools/10b): the verb's own switch, launched `Refuse`, no preview.
fn is_instant_checkout(op: &Operation) -> bool {
    matches!(
        op,
        Operation::Checkout(params) if matches!(params.to, graft_ops::operation::CheckoutTarget::Branch(_))
    )
}

/// The one `Operate` in `commands` is `op`'s switch, run `Refuse`, and
/// nothing is previewed.
fn switched_at_once(state: &AppState, commands: &[Command], op: &Operation, what: &str) {
    assert!(
        state.preview.is_none(),
        "{what}: a clean checkout previews nothing"
    );
    let expected = match op {
        Operation::Checkout(params) => Operation::Checkout(graft_ops::operation::CheckoutParams {
            on_edits: graft_ops::operation::OnEdits::Refuse,
            ..params.clone()
        }),
        other => panic!("{what}: not a checkout: {other:?}"),
    };
    let launched: Vec<&Operation> = commands
        .iter()
        .filter_map(|command| match command {
            Command::Operate(_, op) => Some(op.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(launched, [&expected], "{what}");
}

/// Every verb the menu offers really does something — none of them is a
/// label with nothing behind it. Each opens its preview and runs nothing,
/// except a branch checkout on this clean tree, which switches at once
/// (m7-power-tools/10b).
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
        let op = state
            .context_menu
            .as_ref()
            .and_then(|open| open.verbs.iter().find(|verb| verb.label.as_str() == label))
            .map(|verb| verb.op.clone())
            .expect("the verb behind the label");
        let commands = choose(&mut state, &label);
        if is_instant_checkout(&op) {
            switched_at_once(&state, &commands, &op, &label);
            continue;
        }
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

/// The uncommitted-changes row: absorb first (09b ruling B, keymap §2.1),
/// then `Open details` and the dropped keys — no slots — in the order they
/// had, one place down. Staged or not, the order is the same.
#[test]
fn the_uncommitted_row_menu_keeps_its_old_order_below_absorb() {
    let mut state = on_the_feature_row();
    for staged in [false, true] {
        if staged {
            state.working_tree = Some(staged_tree());
        }
        open_for_working_tree(&mut state);
        assert_eq!(
            actions(&state),
            vec![
                ContextAction::Absorb,
                ContextAction::OpenDetails,
                ContextAction::Fetch,
                ContextAction::Pull,
                ContextAction::Push,
                ContextAction::Undo,
            ]
        );
        state.context_menu = None;
    }
}

/// 09b: the uncommitted row's menu opens on its first item, dim or not —
/// the commit row's rule (`enter_on_the_dim_checkout_slot_keeps_the_menu_and_says_why`
/// pins its cursor at 0 on the dim Checkout). So `. ⏎` is absorb.
#[test]
fn the_uncommitted_row_menu_opens_on_absorb() {
    let mut state = on_the_feature_row();
    open_for_working_tree(&mut state);
    let open = state.context_menu.as_ref().expect("menu opened");
    assert_eq!(open.menu.cursor, 0);
    assert_eq!(open.menu.selected(), ContextAction::Absorb);
    assert_eq!(
        open.menu
            .selected_refusal()
            .map(graft_term::SanitizedText::as_str),
        Some("nothing staged"),
        "dim in place with nothing staged"
    );
    state.context_menu = None;
    state.working_tree = Some(staged_tree());
    open_for_working_tree(&mut state);
    let open = state.context_menu.as_ref().expect("menu opened");
    assert_eq!(open.menu.cursor, 0);
    assert_eq!(open.menu.selected(), ContextAction::Absorb);
    assert_eq!(open.menu.selected_refusal(), None);
}

/// One staged file.
fn staged_tree() -> std::sync::Arc<graft_git::status::WorkingTreeStatus> {
    std::sync::Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![graft_git::status::StatusEntry {
            path: "a.rs".into(),
            original_path: None,
            staged: Some(graft_git::status::Change::Modified),
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    })
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
        assert_eq!(open.menu.items[1].label.as_str(), "Cherry-pick (o)");
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
/// previews the operation of the verb that item names — or, for a branch
/// checkout on this clean tree, runs that very switch at once (10b).
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
            if is_instant_checkout(&expected) {
                switched_at_once(
                    &state,
                    &commands,
                    &expected,
                    &format!("row {row}, item {at}"),
                );
                continue;
            }
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

// ---------------------------------------------------------------------------
// m7-power-tools/11: `o` takes the cursor commit; slot 2 is its menu door.
// ---------------------------------------------------------------------------

/// `history_of(3, 0)` over [`refs_over_the_fixture`] with the cursor on
/// `row`: row 0 is `feature` (`0303…`), row 1 `HEAD` on `main` (`0202…`),
/// row 2 main's parent (`0101…`), an ancestor of `HEAD`.
fn on_row(row: usize) -> AppState {
    let mut state = on_the_feature_row();
    state.history.as_mut().expect("history").cursor = row;
    state
}

/// Row 0 made a merge: a second parent the fixture does not draw.
fn with_a_merge_on_row_0(state: &mut AppState) {
    let window = state
        .history
        .as_mut()
        .and_then(|history| history.window.as_mut())
        .expect("a window");
    let mut rows = window.rows.to_vec();
    rows[0].commit.parents.push(oid(9));
    window.rows = rows.into();
}

fn press_o(state: &mut AppState) -> Vec<Command> {
    let mut commands = Vec::new();
    super::super::keys::key_event(state, &mut commands, key(KeyCode::Char('o')));
    commands
}

/// `feature`'s commit, taken onto `main`.
fn take_feature() -> Operation {
    Operation::CherryPick(graft_ops::operation::PickParams {
        commits: vec![oid(3)],
        subject: "Commit 3\u{1b}[31m".to_owned(),
        head: graft_ops::operation::Tip {
            name: "refs/heads/main".to_owned(),
            oid: oid(2),
        },
    })
}

fn sticky(reason: &str) -> Status {
    Status::Sticky {
        reason: reason.to_owned(),
        keys: None,
    }
}

#[test]
fn o_opens_the_take_preview() {
    let mut state = on_row(0);
    let commands = press_o(&mut state);
    assert!(!commands.is_empty(), "the plan was asked for");
    let preview = state.preview.as_ref().expect("a preview opened");
    assert_eq!(
        preview.subject,
        crate::update::preview::PreviewSubject::Operation {
            op: Box::new(take_feature()),
            opened_by: "o",
        }
    );
    let view = crate::update::preview::preview_box(&state).expect("a box");
    assert_eq!(
        view.title,
        graft_term::sanitize_line("Preview: take 0303030 \"Commit 3\u{1b}[31m\" onto main"),
    );
    assert!(
        view.title
            .as_str()
            .starts_with("Preview: take 0303030 \"Commit 3"),
        "{:?}",
        view.title
    );
    // The bar keeps every preview's `y confirm`: no verb of its own.
    assert_eq!(view.again, None);
}

#[test]
fn slot_two_enter_and_o_open_the_same_preview() {
    let mut by_key = on_row(0);
    press_o(&mut by_key);
    let mut by_menu = on_row(0);
    open_for_history_row(&mut by_menu, 0);
    choose(&mut by_menu, "Cherry-pick 0303030 (o)");

    let op_of = |state: &AppState| {
        state
            .preview
            .as_ref()
            .and_then(|preview| preview.subject.operation().cloned())
            .expect("a preview opened")
    };
    assert_eq!(op_of(&by_key), op_of(&by_menu));
    assert_eq!(op_of(&by_key), take_feature());
    let title = |state: &AppState| {
        crate::update::preview::preview_box(state)
            .expect("a box")
            .title
    };
    assert_eq!(title(&by_key), title(&by_menu));
}

#[test]
fn o_on_head_is_refused_without_a_preview() {
    let mut state = on_row(1);
    let commands = press_o(&mut state);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
    assert_eq!(state.status, sticky("0202020 is already on main"));
}

#[test]
fn o_on_a_merge_is_refused_without_a_preview() {
    let mut state = on_row(0);
    with_a_merge_on_row_0(&mut state);
    let commands = press_o(&mut state);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
    assert_eq!(
        state.status,
        sticky("0303030 is a merge \u{b7} taking a merge isn't in Graft yet")
    );
}

/// An ancestor of `HEAD` is not known from the row: `o` opens the preview,
/// the plan refuses it (11a), and the box closes in the same words.
#[test]
fn o_on_an_ancestor_is_refused_in_words() {
    let mut state = on_row(2);
    press_o(&mut state);
    let job = state.preview.as_ref().expect("a preview opened").job;
    crate::update::preview::preview_produced(
        &mut state,
        job,
        Err(crate::update::preview::PreviewFailure::Refused(
            graft_ops::operation::Refusal::AlreadyOnBranch {
                commit: oid(1),
                branch: "main".to_owned(),
            },
        )),
    );
    assert!(state.preview.is_none(), "the box closed");
    assert_eq!(state.status, sticky("0101010 is already on main"));
}

#[test]
fn o_on_the_uncommitted_row_is_refused_in_words() {
    let mut state = on_row(0);
    state.on_working_tree = true;
    let commands = press_o(&mut state);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
    assert_eq!(
        state.status,
        sticky("o takes a commit \u{b7} this row isn't one")
    );
}

/// An orphan branch: HEAD has no tip, yet other branches' commits show.
/// `o` says `check_pick`'s words for that HEAD (ticket 11 Decided).
#[test]
fn o_on_an_unborn_head_says_check_picks_words() {
    let mut state = on_row(0);
    let name = |full: &str| {
        graft_core::ref_name::RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
    };
    state.refs = Some(std::sync::Arc::new(ref_badges(
        &[(name("refs/heads/feature"), oid(3))],
        &graft_git::repo::HeadState::Unborn(name("refs/heads/orphan")),
    )));
    let commands = press_o(&mut state);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
    assert_eq!(
        state.status,
        sticky("HEAD has no commit yet \u{b7} c makes the first one")
    );
}

#[test]
fn o_on_a_stash_row_is_refused_in_words() {
    let mut state = stash_on_row_0();
    let commands = press_o(&mut state);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
    assert_eq!(
        state.status,
        sticky("o takes a commit \u{b7} this row isn't one")
    );
}

/// `a_stash_row_menu_is_unchanged`'s state: `refs/stash` on row 0.
fn stash_on_row_0() -> AppState {
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
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        ..AppState::default()
    }
}

/// Slot 2 on `HEAD`'s row and on a merge: dim in its place, the sha
/// dropped, `o`'s words as its reason.
#[test]
fn slot_two_dims_on_head_and_on_a_merge() {
    let slot_two = |state: &AppState| {
        let open = state.context_menu.as_ref().expect("menu opened");
        let item = &open.menu.items[1];
        (
            item.action,
            item.label.as_str().to_owned(),
            item.disabled
                .as_ref()
                .map(|reason| reason.as_str().to_owned()),
        )
    };
    let mut head = on_row(1);
    open_for_history_row(&mut head, 1);
    assert_eq!(
        slot_two(&head),
        (
            ContextAction::Unavailable,
            "Cherry-pick (o)".to_owned(),
            Some("already on main".to_owned())
        )
    );
    let mut merge = on_row(0);
    with_a_merge_on_row_0(&mut merge);
    open_for_history_row(&mut merge, 0);
    assert_eq!(
        slot_two(&merge),
        (
            ContextAction::Unavailable,
            "Cherry-pick (o)".to_owned(),
            Some("a merge \u{b7} taking a merge isn't in Graft yet".to_owned())
        )
    );
}

/// `⏎` on the dim slot 2 does what `⏎` on the dim slot 1 does: nothing
/// dispatched, the menu stays, the reason on the status line.
#[test]
fn slot_two_dim_behaves_as_slot_one_dim() {
    let enter_at = |at: usize| {
        let mut state = on_row(1);
        open_for_history_row(&mut state, 1);
        state.context_menu.as_mut().expect("menu").menu.cursor = at;
        let mut commands = Vec::new();
        assert!(context_menu_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter)
        ));
        (state, commands)
    };
    let (one, one_commands) = enter_at(0);
    let (two, two_commands) = enter_at(1);
    for (state, commands) in [(&one, &one_commands), (&two, &two_commands)] {
        assert!(commands.is_empty(), "{commands:?}");
        assert!(state.context_menu.is_some(), "the menu stays open");
        assert!(state.preview.is_none());
        assert!(state.operating.is_none());
    }
    assert_eq!(one.status, sticky("already on main"));
    assert_eq!(two.status, sticky("already on main"));
}

#[test]
fn slot_two_label_carries_o() {
    let mut state = on_row(0);
    open_for_history_row(&mut state, 0);
    let open = state.context_menu.as_ref().expect("menu opened");
    assert_eq!(open.menu.items[1].label.as_str(), "Cherry-pick 0303030 (o)");
    assert_eq!(open.menu.items[1].action, ContextAction::History(1));
    assert_eq!(open.menu.items[1].disabled, None);
}

#[test]
fn a_stash_row_menu_carries_no_o() {
    let mut state = stash_on_row_0();
    open_for_history_row(&mut state, 0);
    let open = state.context_menu.as_ref().expect("menu opened");
    let labels: Vec<&str> = open.verbs.iter().map(|verb| verb.label.as_str()).collect();
    assert!(labels.contains(&"Cherry-pick 0303030"), "{labels:?}");
    assert!(
        !labels.iter().any(|label| label.ends_with("(o)")),
        "{labels:?}"
    );
}

// ---------------------------------------------------------------------------
// m7-power-tools/40: `o take` and `a amend` on the bar where they act.
// ---------------------------------------------------------------------------

/// The bar drawn over `state` at 187 columns, from the drawn rows (the
/// cursor row's badges and verbs).
fn bar_words(state: &AppState) -> Vec<String> {
    let mut state = state.clone();
    state.size = crate::update::state::Size {
        columns: 187,
        rows: 44,
    };
    let context = crate::view::rows::graph_view(&state, 0).hint_context;
    graft_ui::graph_screen::hints_for(
        &context,
        graft_ui::graph_screen::Divider::Hidden,
        187,
        graft_ui::keymap::Keymap::shipped(),
    )
    .iter()
    .map(|hint| format!("{} {}", hint.key(), hint.verb))
    .collect()
}

fn draws(state: &AppState, words: &str) -> bool {
    bar_words(state).iter().any(|hint| hint == words)
}

/// [`on_row`] with only `main` badged: row 0 is a plain commit off his
/// branch, no badge to lead the bar with.
fn plain_off_branch_row() -> AppState {
    let mut state = on_row(0);
    let name = |full: &str| {
        graft_core::ref_name::RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
    };
    state.refs = Some(std::sync::Arc::new(ref_badges(
        &[(name("refs/heads/main"), oid(2))],
        &graft_git::repo::HeadState::Branch(name("refs/heads/main")),
    )));
    state
}

fn merge_in_progress(state: &mut AppState) {
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
}

#[test]
fn a_commit_off_his_branch_hints_o_take() {
    let words = bar_words(&plain_off_branch_row());
    assert_eq!(words[..2], ["⏎ open", "o take"], "{words:?}");
    // On a badged row off his branch, `o take` still follows `⏎ open`.
    let words = bar_words(&on_row(0));
    let open = words.iter().position(|w| w == "⏎ open").expect("⏎ open");
    assert_eq!(words[open + 1], "o take", "{words:?}");
}

#[test]
fn heads_row_has_no_o() {
    assert!(!draws(&on_row(1), "o take"), "{:?}", bar_words(&on_row(1)));
}

#[test]
fn a_merge_row_has_no_o() {
    let mut state = on_row(0);
    with_a_merge_on_row_0(&mut state);
    assert!(!draws(&state, "o take"), "{:?}", bar_words(&state));
}

#[test]
fn an_ancestor_of_head_hints_o_like_the_menu() {
    let state = on_row(2);
    assert!(draws(&state, "o take"), "{:?}", bar_words(&state));
}

#[test]
fn heads_row_hints_a_amend() {
    let words = bar_words(&on_row(1));
    let fetch = words.iter().position(|w| w == "f fetch").expect("f fetch");
    assert_eq!(words[fetch + 1], "a amend", "{words:?}");
}

#[test]
fn an_older_row_has_no_a() {
    for row in [0, 2] {
        assert!(
            !draws(&on_row(row), "a amend"),
            "row {row}: {:?}",
            bar_words(&on_row(row))
        );
    }
}

#[test]
fn a_pending_amend_hides_a() {
    let mut state = on_row(1);
    state.amend_into = crate::update::amend_into::AmendInto::pending_for_test(next_id_from(&mut 0));
    assert!(!draws(&state, "a amend"), "{:?}", bar_words(&state));
}

#[test]
fn a_live_op_row_hints_neither() {
    for row in [0, 1, 2] {
        let mut state = on_row(row);
        merge_in_progress(&mut state);
        let words = bar_words(&state);
        assert!(
            !words.iter().any(|w| w == "o take" || w == "a amend"),
            "row {row}: {words:?}"
        );
    }
}

/// The two bars of m7-power-tools/40, drawn whole at 187x44.
#[test]
fn capture_bar_take_and_amend_rows() {
    let shot = |state: &AppState, name: &str, right: &str| {
        let mut sized = state.clone();
        sized.size = crate::update::state::Size {
            columns: 187,
            rows: 44,
        };
        let context = crate::view::rows::graph_view(&sized, 0).hint_context;
        let bar = graft_ui::graph_screen::hints_text(
            &graft_ui::graph_screen::hints_for(
                &context,
                graft_ui::graph_screen::Divider::Hidden,
                187,
                graft_ui::keymap::Keymap::shipped(),
            ),
            graft_ui::keymap::Keymap::shipped(),
        );
        let screen = format!("{context:?}\n\"{bar}\"\n");
        let header = format!(
            "# {name}: the 187-column bar (not a pty), from `view::rows::graph_view`'s context. Fixture: \
             capture_bar_take_and_amend_rows in\n# crates/graft/src/update/context_menu/tests.rs \
             (m7-power-tools/40). Colour unverified.\n# Right: {right}\n#\n"
        );
        graft_fixtures::capture(
            format!("{}/../../docs/shots/m7/{name}", env!("CARGO_MANIFEST_DIR")),
            format!("{header}{screen}"),
        );
    };
    shot(
        &plain_off_branch_row(),
        "bar-take-row-187x44.txt",
        "cursor on a commit off his branch; bar reads `⏎ open  o take …`.",
    );
    shot(
        &on_row(1),
        "bar-amend-row-187x44.txt",
        "cursor on HEAD's row; bar reads `⏎ open  c commit  P push  p pull  f fetch  a amend …`; no `o`.",
    );
}

/// Every row of the fixture — `feature` off his branch, `HEAD`, an ancestor
/// of `HEAD`, and the same first row made a merge: the predicate, the menu's
/// slot 2 and the bar's `o` say the same thing.
#[test]
fn the_menu_and_the_bar_agree_on_o() {
    let mut merged = on_row(0);
    with_a_merge_on_row_0(&mut merged);
    let states = vec![on_row(0), on_row(1), on_row(2), merged];
    let mut seen = Vec::new();
    for state in states {
        let can = crate::update::can_take(&state);
        let mut menu = state.clone();
        let row = menu.history.as_ref().expect("history").cursor;
        open_for_history_row(&mut menu, row);
        let open = menu.context_menu.as_ref().expect("menu opened");
        let slot = open
            .menu
            .items
            .iter()
            .find(|item| item.label.as_str().starts_with("Cherry-pick"))
            .expect("slot 2");
        let live = slot.disabled.is_none();
        let drawn = draws(&state, "o take");
        assert_eq!((live, drawn), (can, can), "row {row}");
        seen.push(can);
    }
    assert_eq!(seen, [true, false, true, false]);
}
