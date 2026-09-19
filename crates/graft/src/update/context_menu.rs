//! The `.`/right-click popup: opening it, answering its own keys, and
//! carrying out whatever it dispatches (`context-actions-menu`,
//! m2-details-and-diffs/12).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;

use graft_ui::context_menu::{ContextAction, ContextMenu};

use super::detail::DetailSpec;
use super::events::Command;
use super::state::{AppState, Status};

/// The sentinel `row` for the uncommitted-changes row's menu: it names no
/// absolute `History` row (`AppState::on_working_tree` is what selects it
/// instead), so this is never a real row index.
pub(super) const WORKING_TREE_ROW: usize = usize::MAX;

/// `.`/right-click on a commit row: open its menu. `marked` asks the same
/// source of truth the marker column asks (`view::radar::findings_for`,
/// `view/entries.rs`) rather than a second, hand-maintained flag — a row
/// with at least one finding lists `Explain`.
pub(super) fn open_for_history_row(next: &mut AppState, row: usize) {
    let marked = next
        .history
        .as_ref()
        .and_then(|history| history.row_at(row))
        .is_some_and(|entry| {
            let subject = String::from_utf8_lossy(entry.commit.subject());
            !crate::view::radar::findings_for(
                next,
                entry.commit.id,
                &entry.commit.parents,
                &subject,
            )
            .is_empty()
        });
    next.context_menu = Some((row, ContextMenu::for_row(marked)));
}

/// `.`/right-click on the uncommitted-changes row: `Open details` alone.
pub(super) fn open_for_working_tree(next: &mut AppState) {
    next.context_menu = Some((WORKING_TREE_ROW, ContextMenu::for_working_tree()));
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

/// The menu's own keys, while it is open: `j`/`k` move, `Enter` dispatches
/// the selected action and closes it, everything else — `Esc` included —
/// closes it without acting. Modal: no key reaches the graph while a menu is
/// open, so this returns `true` whenever one was.
pub(super) fn context_menu_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let Some((row, menu)) = next.context_menu.as_mut() else {
        return false;
    };
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => menu.move_cursor(1),
        KeyCode::Char('k') | KeyCode::Up => menu.move_cursor(-1),
        KeyCode::Enter => {
            let action = menu.selected();
            let row = *row;
            next.context_menu = None;
            commands.extend(dispatch(next, row, action));
        }
        _ => next.context_menu = None,
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
pub(super) fn dispatch(next: &mut AppState, row: usize, action: ContextAction) -> Vec<Command> {
    let oid = oid_for(next, row);
    let mut local_jobs = next.local_jobs;
    let commands = apply_context_action(next, action, row, oid, &mut || local_jobs.next_id());
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
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match action {
        ContextAction::CopySha => {
            let Some(oid) = oid else {
                debug_assert!(
                    false,
                    "CopySha is unreachable on the working-tree row (for_working_tree never lists it)"
                );
                return Vec::new();
            };
            let sha = oid.to_string();
            let short = sha.chars().take(7).collect();
            if !matches!(state.status, Status::Failed(_) | Status::WriteFailed(_)) {
                state.status = Status::Copied { short };
            }
            vec![Command::CopyToClipboard(sha)]
        }
        ContextAction::CompareWithThis => {
            let Some(oid) = oid else {
                debug_assert!(
                    false,
                    "CompareWithThis is unreachable on the working-tree row (for_working_tree never lists it)"
                );
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
            let Some(oid) = oid else {
                debug_assert!(
                    false,
                    "Explain is unreachable on the working-tree row (for_working_tree never lists it)"
                );
                return Vec::new();
            };
            state.detail.explain_requested = Some(oid);
            Vec::new()
        }
    }
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

    #[test]
    fn open_for_history_row_lists_explain_when_the_row_has_a_finding() {
        let mark = oid(3); // `history_of(3, 0)`'s row 0.
        let mut state = state_with_finding_at(mark);

        open_for_history_row(&mut state, 0);

        let (row, menu) = state.context_menu.expect("menu opened");
        assert_eq!(row, 0);
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

        let (row, menu) = state.context_menu.expect("menu opened");
        assert_eq!(row, 0);
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

        let (row, menu) = state.context_menu.expect("menu opened");
        assert_eq!(row, 3);
        assert!(
            !menu
                .items
                .iter()
                .any(|item| item.action == ContextAction::Explain),
            "no history means no row to look a finding up on"
        );
    }

    #[test]
    fn open_for_working_tree_lists_open_details_alone() {
        let mut state = AppState::default();
        open_for_working_tree(&mut state);
        let (row, menu) = state.context_menu.expect("menu opened");
        assert_eq!(row, WORKING_TREE_ROW);
        assert_eq!(menu.items.len(), 1);
        assert_eq!(menu.items[0].action, ContextAction::OpenDetails);
    }

    #[test]
    fn open_selected_dispatches_by_on_working_tree_flag() {
        let mut state = AppState {
            on_working_tree: true,
            ..AppState::default()
        };
        open_selected(&mut state);
        let (row, _) = state.context_menu.expect("menu opened");
        assert_eq!(row, WORKING_TREE_ROW);
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
        let (_, menu) = state.context_menu.as_ref().expect("still open");
        assert_eq!(menu.cursor, 1);
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
            &mut || next_id_from(&mut ids),
        );
        assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
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
            &mut || next_id_from(&mut ids),
        );
        assert!(commands.is_empty());
        assert_eq!(state.detail.explain_requested, Some(oid(1)));
    }
}
