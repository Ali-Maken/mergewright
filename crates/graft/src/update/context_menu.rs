//! The `.`/right-click popup: opening it, answering its own keys, and
//! carrying out whatever it dispatches (`context-actions-menu`,
//! m2-details-and-diffs/12).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;

use graft_ui::context_menu::{ContextAction, ContextMenu};

use super::detail::DetailSpec;
use super::events::Command;
use super::preview::PreviewSubject;
use super::state::{AppState, Status};

pub mod verbs;

pub use verbs::HistoryVerb;

/// The `.`/right-click popup while it is open: the row it opened on, the
/// menu the widget draws, and what each of its history rows means.
///
/// The three travel together because they are one answer to one `.`: the
/// widget reports a row by index ([`ContextAction::History`]), and `verbs`
/// is the list that index points into. Keeping them in one field is what
/// makes it impossible for the labels on screen and the operations behind
/// them to drift apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenMenu {
    /// The absolute `History` row it opened on, or [`WORKING_TREE_ROW`].
    pub row: usize,
    /// What the widget draws.
    pub menu: ContextMenu,
    /// The history verbs this row offers, in the order the menu lists them.
    /// Empty on the working-tree row and on a row the refs have not caught
    /// up with.
    pub verbs: Vec<HistoryVerb>,
}

/// The sentinel `row` for the uncommitted-changes row's menu: it names no
/// absolute `History` row (`AppState::on_working_tree` is what selects it
/// instead), so this is never a real row index.
pub(super) const WORKING_TREE_ROW: usize = usize::MAX;

/// `.`/right-click on a commit row: open its menu. `marked` asks the same
/// source of truth the marker column asks (`view::radar::findings_for`,
/// `view/entries.rs`) rather than a second, hand-maintained flag — a row
/// with at least one finding lists `Explain`.
pub(super) fn open_for_history_row(next: &mut AppState, row: usize) {
    let mut marked = false;
    let mut verbs = Vec::new();
    if let Some(entry) = next
        .history
        .as_ref()
        .and_then(|history| history.row_at(row))
    {
        let id = entry.commit.id;
        let parents = entry.commit.parents.clone();
        let subject = String::from_utf8_lossy(entry.commit.subject()).into_owned();
        let findings = crate::view::radar::findings_for(next, id, &parents, &subject);
        marked = !findings.is_empty();
        // No refs yet means no branch to name: the row's own actions and the
        // dropped keys still work, and a verb that cannot say which branch it
        // moves is one this menu is better off not offering.
        if let Some(refs) = next.refs.as_deref() {
            verbs = verbs::history_verbs(id, &parents, &subject, refs, &findings);
        }
    }
    let labels = verbs.iter().map(|verb| verb.label.clone()).collect();
    next.context_menu = Some(OpenMenu {
        row,
        menu: ContextMenu::for_row(marked, labels),
        verbs,
    });
}

/// `.`/right-click on the uncommitted-changes row: `Open details` alone.
pub(super) fn open_for_working_tree(next: &mut AppState) {
    next.context_menu = Some(OpenMenu {
        row: WORKING_TREE_ROW,
        menu: ContextMenu::for_working_tree(),
        verbs: Vec::new(),
    });
}

/// `.`: open the menu for whatever the cursor now names.
pub(super) fn open_selected(next: &mut AppState) {
    if next.on_working_tree {
        open_for_working_tree(next);
        return;
    }
    let Some(row) = next.history.as_ref().map(|history| history.cursor) else {
        return;
    };
    open_for_history_row(next, row);
}

/// The menu's own keys, while it is open. It claims exactly four — `j`/`k`
/// (and the arrows) move, `Enter` dispatches the selected action and closes,
/// `Esc` closes without acting — and **nothing else**
/// (`docs/design/02-keymap.md` §2.1, "an overlay that needs only a cursor is
/// not a modal pane").
///
/// Returning `true` means the key stops here. Every other key gets `false`:
/// the menu closes and the key carries on down `keys::key_event` to mean what
/// it always means, so `m` on a row labelled `Merge main into feature (m)`
/// really merges instead of being eaten. A trailing `(key)` in a label is a
/// promise that the letter is live right now; the old catch-all
/// (`_ => close` then an unconditional `true`) broke that promise for every
/// verb the menu advertises (m5-history-and-ghost-preview/13).
///
/// Nothing further down `key_event` reads `next.context_menu`, so closing
/// here and falling through is the whole of "as if the menu had not been
/// open" — `remote::cancel_operating`'s `context_menu.is_some()` guard sees
/// the `None` this already wrote.
pub(super) fn context_menu_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let Some(open) = next.context_menu.as_mut() else {
        return false;
    };
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => open.menu.move_cursor(1),
        KeyCode::Char('k') | KeyCode::Up => open.menu.move_cursor(-1),
        KeyCode::Enter => {
            let action = open.menu.selected();
            // The verb list travels with the action it was chosen from, so
            // nothing can dispatch against a list that has since been
            // rebuilt.
            let row = open.row;
            let verbs = std::mem::take(&mut open.verbs);
            next.context_menu = None;
            commands.extend(dispatch(next, row, &verbs, action));
        }
        KeyCode::Escape => next.context_menu = None,
        _ => {
            next.context_menu = None;
            return false;
        }
    }
    true
}

/// The `Oid` a menu opened on `row` acts on: `None` on the working-tree
/// sentinel, else whatever row that absolute index names.
fn oid_for(next: &AppState, row: usize) -> Option<Oid> {
    if row == WORKING_TREE_ROW {
        return None;
    }
    next.history
        .as_ref()
        .and_then(|history| history.row_at(row))
        .map(|entry| entry.commit.id)
}

/// `apply_context_action`'s `next_id` plumbing from `next.local_jobs` — the
/// same copy-out/copy-back idiom `keys::toggle_compare_selected` uses,
/// needed because `next` is borrowed both by the closure and by the call.
pub(super) fn dispatch(
    next: &mut AppState,
    row: usize,
    verbs: &[HistoryVerb],
    action: ContextAction,
) -> Vec<Command> {
    let oid = oid_for(next, row);
    let mut local_jobs = next.local_jobs;
    let commands =
        apply_context_action(next, action, row, oid, verbs, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    commands
}

/// What a chosen context-menu item does. `oid` is `None` only on the
/// working-tree row; `CopySha`, `CompareWithThis` and `Explain` can never be
/// reached there (`ContextMenu::for_working_tree` never lists them), so that
/// combination is a routing bug, not a user-facing error — `debug_assert!`
/// and no command.
pub fn apply_context_action(
    state: &mut AppState,
    action: ContextAction,
    row: usize,
    oid: Option<Oid>,
    verbs: &[HistoryVerb],
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match action {
        ContextAction::CopySha => {
            let Some(oid) = commit_of(oid, "CopySha") else {
                return Vec::new();
            };
            let sha = oid.to_string();
            let short = sha.chars().take(7).collect();
            if !matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
                state.status = Status::Copied { short };
            }
            vec![Command::CopyToClipboard(sha)]
        }
        ContextAction::CompareWithThis => {
            let Some(oid) = commit_of(oid, "CompareWithThis") else {
                return Vec::new();
            };
            super::detail::toggle_compare(state, row, oid, next_id)
        }
        ContextAction::OpenDetails => {
            let spec = match oid {
                Some(oid) => DetailSpec::Commit(oid),
                None => DetailSpec::WorkingTree,
            };
            state.detail.open(spec, next_id)
        }
        ContextAction::Explain => {
            let Some(oid) = commit_of(oid, "Explain") else {
                return Vec::new();
            };
            state.detail.explain_requested = Some(oid);
            Vec::new()
        }
        // The keys the hint bar drops at 80 and 120 columns, so that `. more`
        // leads to them (m5-03 review, Major 1). Each one calls exactly what
        // its key calls — a second path to the same function, never a second
        // implementation. `oid` and `row` are beside the point: all four act
        // on `HEAD` and the current branch, wherever the cursor is.
        //
        // The `bool` each key function returns says whether it claimed the
        // key. From the menu the action was chosen outright, and a refusal
        // (no remote, nothing to undo) already says so on the status line, so
        // there is nothing left here to decide on.
        ContextAction::Fetch => {
            let mut commands = Vec::new();
            let _claimed: bool = super::remote::fetch_key(state, &mut commands, next_id);
            commands
        }
        ContextAction::Pull => {
            let mut commands = Vec::new();
            let _claimed: bool = super::remote::pull_key(state, &mut commands, next_id);
            commands
        }
        ContextAction::Push => {
            let _claimed: bool = super::remote::push_key(state);
            Vec::new()
        }
        ContextAction::Undo => super::undo::undo_key(state, false, next_id),
        // A history verb: `Checkout`, `Merge`, `Rebase`, `Cherry-pick`,
        // `Revert`, `Reset`, a ref deletion or one of the Fix items. The
        // widget reported which row was chosen; `verbs` is the list that
        // index points into, and `update::context_menu::verbs` decided what
        // each one means (m5-history-and-ghost-preview/08, piece 6).
        ContextAction::History(index) => {
            let Some(chosen) = verbs.get(index) else {
                debug_assert!(
                    false,
                    "History({index}) against {} verbs: the menu and the list it was built from disagree",
                    verbs.len()
                );
                return Vec::new();
            };
            preview_history_verb(state, chosen, next_id)
        }
    }
}

/// The commit an action acts on. `CopySha`, `CompareWithThis` and `Explain`
/// are never listed on the working-tree row (`ContextMenu::for_working_tree`
/// lists none of them), so `None` here is a routing bug rather than anything
/// the user did: assert in debug, and answer with no command.
fn commit_of(oid: Option<Oid>, action: &str) -> Option<Oid> {
    debug_assert!(
        oid.is_some(),
        "{action} is unreachable on the working-tree row (for_working_tree never lists it)"
    );
    oid
}

/// A chosen history verb: open the Ghost Preview of the very operation its
/// label named.
///
/// `.` ends in a picture, never in a write — only `Confirm` in that picture
/// runs anything (PROMPT.md §1.7).
fn preview_history_verb(
    state: &mut AppState,
    chosen: &HistoryVerb,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    // The `bool` says whether a preview opened. Every refusal inside
    // `open_preview` — a write already running, git mid-operation — has
    // written its own sticky line saying so, and the item was chosen
    // outright, so there is nothing left here to decide on: the same
    // reasoning as the four dropped keys above.
    let _opened: bool = super::preview::open_preview(
        state,
        &mut commands,
        PreviewSubject::Operation {
            op: Box::new(chosen.op.clone()),
            opened_by: ".",
        },
        false,
        next_id,
    );
    commands
}

#[cfg(test)]
mod tests {
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
            ContextMenu::for_row(false, Vec::new()),
            ContextMenu::for_row(true, Vec::new()),
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

    /// The keys the menu claims stop here (`true`). `Enter`, the fourth, is
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
        // `toggle_compare`'s first branch: pins the base, no command.
        assert!(commands.is_empty());
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
    fn refs_over_the_fixture() -> std::sync::Arc<crate::refs::RefBadges> {
        let name = |full: &str| {
            graft_core::ref_name::RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
        };
        let badges = crate::refs::badges(
            &[
                (name("refs/heads/main"), oid(2)),
                (name("refs/heads/feature"), oid(3)),
            ],
            &graft_git::repo::HeadState::Branch(name("refs/heads/main")),
            &std::collections::HashMap::new(),
            &[],
            |_, _| {
                Ok(graft_git::history::AheadBehind {
                    ahead: 0,
                    behind: 0,
                })
            },
        )
        .expect("no ahead/behind walk can fail here");
        std::sync::Arc::new(badges)
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
                "Merge feature into main (m)",
                "Rebase main onto feature (r)",
                "Cherry-pick 0303030",
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
        assert_eq!(positions.first(), Some(&3), "after the row's own actions");
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
}
