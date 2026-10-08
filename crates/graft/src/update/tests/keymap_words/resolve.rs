//! `c`'s refusal at a conflict names what acts where he is left (m8/10l,
//! m8/11f Part A): the focused banner, the changes pane on a CONFLICTS row,
//! the changes pane on any other row, and the graph. Moved from `stops`.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode as CoreKey, KeyPress};
use graft_git::state::InProgress;
use graft_ops::operation::{Operation, Target};
use graft_ui::detail::WorkingTreeSection;
use graft_ui::keymap::{ActionId, ContextId, Keymap};

use super::{swept, unbinding, whole_tokens};
use crate::update::banner::{RefusedAt, resolve_keys};
use crate::update::state::Status;
use crate::update::{AppEvent, AppState, Command, update};

const BANNER: &str = "⏎ resolve  M mergetool  e edit";
const CONFLICT_ROW: &str = "⏎ resolve  M mergetool  e edit  space marks resolved";

/// A rebase stopped on `a.rs`, his keyboard where `spot` says, the banner
/// unfocused unless the spot is the banner. The changes pane holds a
/// CONFLICTS section (`a.rs`, row 1) and an unstaged one (`b.rs`, row 3).
fn stopped_at(spot: Spot) -> AppState {
    use crate::update::operate::tests::{state_with, unmerged_entry, unstaged_entry};
    let rows = [
        ("a.rs".to_owned(), WorkingTreeSection::Unmerged),
        ("b.rs".to_owned(), WorkingTreeSection::Unstaged),
    ];
    let cursor = if spot == Spot::OtherRow { 3 } else { 1 };
    let mut state = state_with(
        &rows,
        vec![unmerged_entry("a.rs"), unstaged_entry("b.rs")],
        cursor,
    );
    let banner = crate::update::banner::tests::stopped(InProgress::Rebase, &["a.rs"]);
    state.in_progress = banner.in_progress;
    state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    state.on_working_tree = true;
    state.size = crate::update::Size {
        columns: 187,
        rows: 50,
    };
    state.mergetool = Some("vimdiff".to_owned());
    match spot {
        Spot::Banner => state.banner_focus = true,
        Spot::Graph => state.detail.shown = false,
        Spot::ConflictRow | Spot::OtherRow => {}
    }
    state
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spot {
    Banner,
    ConflictRow,
    OtherRow,
    Graph,
}

fn key(state: &AppState, code: CoreKey) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))))
}

fn sticky(state: &AppState) -> (String, Option<String>) {
    match &state.status {
        Status::Sticky { reason, keys } => (reason.clone(), keys.clone()),
        other => panic!("a sticky line: {other:?}"),
    }
}

/// What a clause's key does, pressed at the spot: `true` when it does
/// what the clause says, on `a.rs`.
fn acts_as_said(state: &AppState, clause: &str) -> bool {
    let a = PathBuf::from("a.rs");
    match clause {
        "⏎ resolve" => {
            let (after, _) = key(state, CoreKey::Enter);
            after.resolver.as_ref().is_some_and(|r| r.path == a)
        }
        "M mergetool" => {
            let (_, commands) = key(state, CoreKey::Char('M'));
            commands
                .iter()
                .any(|c| matches!(c, Command::OpenMergetool { path } if *path == a))
        }
        "e edit" => {
            let (_, commands) = key(state, CoreKey::Char('e'));
            commands
                .iter()
                .any(|c| matches!(c, Command::OpenConflictEditor { path } if *path == a))
        }
        "space marks resolved" => {
            let (_, commands) = key(state, CoreKey::Char(' '));
            commands.iter().any(|c| {
                matches!(c, Command::Operate(_, op, ..)
                    if matches!(&**op, Operation::Stage(Target::Paths(p)) if p == std::slice::from_ref(&a)))
            })
        }
        other => panic!("a clause nobody checks: {other}"),
    }
}

/// Part A pin (10l, 11f): where `c`'s refusal leaves him, and that every
/// key its words name, pressed there from the same state, does what its
/// clause says. On a CONFLICTS row `space` marks the file resolved; on any
/// other row no key acts on the conflict, so the reason stands alone.
#[test]
fn the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown() {
    for (spot, place, want) in [
        (Spot::Banner, ContextId::Banner, Some(BANNER)),
        (Spot::ConflictRow, ContextId::Changes, Some(CONFLICT_ROW)),
        (Spot::OtherRow, ContextId::Changes, None),
        (Spot::Graph, ContextId::Graph, None),
    ] {
        let state = stopped_at(spot);
        let (refused, _) = key(&state, CoreKey::Char('c'));
        assert_eq!(
            crate::update::edge::context_of(&refused),
            place,
            "{spot:?}: he is left where he pressed"
        );
        let (reason, keys) = sticky(&refused);
        assert_eq!(
            (reason.as_str(), keys.as_deref()),
            ("1 file still conflicted", want),
            "{spot:?}"
        );
        for clause in want.into_iter().flat_map(|k| k.split("  ")) {
            assert!(
                acts_as_said(&refused, clause),
                "{spot:?}: `{clause}` does not act"
            );
        }
    }
    // Off a CONFLICTS row no `⇥ banner`: one `⇥` from the file list goes to
    // the diff, not the banner (`banner/stop/tab.rs` `tab_target`).
    let (refused, _) = key(&stopped_at(Spot::OtherRow), CoreKey::Char('c'));
    assert!(!key(&refused, CoreKey::Tab).0.banner_focus);
}

/// The refusal's keys spelled from his keymap (10l, 11f): a remap moves
/// each label; an unbound row drops its clause; all unbound → no keys, no
/// `·`.
#[test]
fn the_resolve_keys_spell_his_keys() {
    let banner = refused(ContextId::Banner, false);
    let row = refused(ContextId::Changes, true);
    let other = refused(ContextId::Changes, false);
    let graph = refused(ContextId::Graph, false);
    let shipped = Keymap::shipped();
    assert_eq!(resolve_keys(banner, shipped).as_deref(), Some(BANNER));
    assert_eq!(resolve_keys(row, shipped).as_deref(), Some(CONFLICT_ROW));
    assert_eq!(resolve_keys(other, shipped), None);
    assert_eq!(resolve_keys(graph, shipped), None);

    let banner_rows = [
        ("app.open", "resolve"),
        ("operation.mergetool", "mergetool"),
        ("operation.edit", "edit"),
    ];
    let row_rows = [
        ("app.open", "resolve"),
        ("changes.mergetool", "mergetool"),
        ("changes.edit", "edit"),
        ("changes.stage", "marks resolved"),
    ];
    let swept = swept();
    assert_eq!(resolve_keys(other, &swept), None);
    for (at, rows) in [(banner, &banner_rows[..]), (row, &row_rows[..])] {
        let line = resolve_keys(at, &swept).expect("swept keys");
        for (id, verb) in rows {
            let label = swept.spell(&ActionId(id)).0;
            assert_eq!(whole_tokens(&line, &label), 1, "{id} as {label}: {line}");
            assert!(line.contains(&format!("{label} {verb}")), "{id}: {line}");
        }
        for (gone, verb) in rows {
            let line = resolve_keys(at, &unbinding(gone));
            assert!(
                line.as_deref().is_none_or(|line| !line
                    .split("  ")
                    .any(|clause| clause.split_once(' ').is_some_and(|(_, v)| v == *verb))),
                "{gone} unbound at {at:?}: {line:?}"
            );
        }
        let none = unbinding_all(&rows.iter().map(|(id, _)| *id).collect::<Vec<_>>());
        assert_eq!(resolve_keys(at, &none), None, "{at:?}");
    }

    let mut state = stopped_at(Spot::ConflictRow);
    state.keymap = Arc::new(unbinding_all(
        &row_rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
    ));
    let (refused, _) = key(&state, CoreKey::Char('c'));
    assert_eq!(
        sticky(&refused),
        ("1 file still conflicted".to_owned(), None)
    );
}

fn refused(ctx: ContextId, on_conflict_row: bool) -> RefusedAt {
    RefusedAt {
        ctx,
        on_conflict_row,
    }
}

/// The shipped keymap with every one of `ids` unbound.
fn unbinding_all(ids: &[&str]) -> Keymap {
    let overrides: Vec<(String, Vec<String>)> = ids
        .iter()
        .map(|id| ((*id).to_owned(), vec!["none".to_owned()]))
        .collect();
    let (map, problems) =
        Keymap::with_overrides(&overrides, graft_ui::keymap::KeySwitches::default());
    assert_eq!(problems, [], "{ids:?} = none reads cleanly");
    map
}
