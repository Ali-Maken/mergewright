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
const CONFLICT_ROW: &str = "⏎ resolve  M mergetool  e edit  space mark resolved";

/// A rebase stopped on `a.rs`, his keyboard where `spot` says, the banner
/// unfocused unless the spot is the banner. The changes pane holds a
/// CONFLICTS section (`a.rs`, row 1) and an unstaged one (`b.rs`, row 3).
pub(super) fn stopped_at(spot: Spot) -> AppState {
    use crate::update::operate::tests::{state_with, unmerged_entry, unstaged_entry};
    let rows = [
        ("a.rs".to_owned(), WorkingTreeSection::Unmerged),
        ("b.rs".to_owned(), WorkingTreeSection::Unstaged),
    ];
    let cursor = if matches!(spot, Spot::OtherRow | Spot::DiffOtherRow) {
        3
    } else {
        1
    };
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
        Spot::DiffOtherRow => state.detail.focus = graft_ui::detail::DetailFocus::Diff,
        Spot::ConflictRow | Spot::OtherRow => {}
    }
    state
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Spot {
    Banner,
    ConflictRow,
    OtherRow,
    /// The changes pane's diff, its file list's cursor off the CONFLICTS rows.
    DiffOtherRow,
    Graph,
}

pub(super) fn key(state: &AppState, code: CoreKey) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))))
}

pub(super) fn sticky(state: &AppState) -> (String, Option<String>) {
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
        "space mark resolved" => {
            let (_, commands) = key(state, CoreKey::Char(' '));
            commands.iter().any(|c| {
                matches!(c, Command::Operate(_, op, ..)
                    if matches!(&**op, Operation::Stage(Target::Paths(p)) if p == std::slice::from_ref(&a)))
            })
        }
        banner if banner.ends_with(" banner") => {
            let tabs = banner.split(' ').count() - 1;
            reaches_banner_in(state, CoreKey::Tab, tabs)
        }
        other => panic!("a clause nobody checks: {other}"),
    }
}

/// `presses` of `code` from `state` focus the banner, and one fewer does
/// not: the count is the true one, driven, not read off a counter.
pub(super) fn reaches_banner_in(state: &AppState, code: CoreKey, presses: usize) -> bool {
    let mut now = state.clone();
    for press in 1..=presses {
        now = key(&now, code).0;
        if now.banner_focus != (press == presses) {
            return false;
        }
    }
    true
}

/// Part A pin (10l, 11f): where `c`'s refusal leaves him, and that every
/// key its words name, pressed there from the same state, does what its
/// clause says. On a CONFLICTS row `space` marks the file resolved; on any
/// other row no key acts on the conflict, so the words name the `⇥`s that
/// reach the banner (m8/20b R4).
#[test]
fn the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown() {
    for (spot, place, want) in [
        (Spot::Banner, ContextId::Banner, Some(BANNER)),
        (Spot::ConflictRow, ContextId::Changes, Some(CONFLICT_ROW)),
        (Spot::OtherRow, ContextId::Changes, Some("⇥ ⇥ banner")),
        (Spot::DiffOtherRow, ContextId::Changes, Some("⇥ banner")),
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
}

/// m8/20b B1 (R4): off a CONFLICTS row the refusal names the way on — as
/// many `⇥` as it takes: two from the file list (the diff, then the
/// banner), one from the diff. Each count is driven: that many presses
/// focus the banner, one fewer does not. On a CONFLICTS row: unchanged.
#[test]
fn off_a_conflict_row_the_refusal_names_the_tabs_that_reach_the_banner() {
    for (spot, want, presses) in [
        (Spot::OtherRow, "⇥ ⇥ banner", 2),
        (Spot::DiffOtherRow, "⇥ banner", 1),
    ] {
        let (refused, _) = key(&stopped_at(spot), CoreKey::Char('c'));
        assert_eq!(
            sticky(&refused),
            ("1 file still conflicted".to_owned(), Some(want.to_owned())),
            "{spot:?}"
        );
        assert!(
            reaches_banner_in(&refused, CoreKey::Tab, presses),
            "{spot:?}"
        );
    }
    let (refused, _) = key(&stopped_at(Spot::ConflictRow), CoreKey::Char('c'));
    assert_eq!(sticky(&refused).1.as_deref(), Some(CONFLICT_ROW));
}

/// m8/20b B3 (R5): the banner clause is spelled from his keymap —
/// `app.next_pane = "ctrl+n"` reads `⌃N ⌃N banner`; unbound, the reason stands alone. Both `c` and `u`.
#[test]
fn the_banner_clause_is_spelled_from_his_keymap() {
    let ctrl_n = with("app.next_pane", "ctrl+n");
    let mut state = stopped_at(Spot::OtherRow);
    state.keymap = Arc::new(ctrl_n);
    let (refused, _) = key(&state, CoreKey::Char('c'));
    assert_eq!(sticky(&refused).1.as_deref(), Some("⌃N ⌃N banner"));
    let (refused, _) = key(&state, CoreKey::Char('u'));
    assert_eq!(
        sticky(&refused),
        (
            "finish or abort the rebase first".to_owned(),
            Some("⌃N ⌃N banner".to_owned())
        )
    );

    state.keymap = Arc::new(unbinding_all(&["app.next_pane", "app.prev_pane"]));
    let (refused, _) = key(&state, CoreKey::Char('c'));
    assert_eq!(
        sticky(&refused),
        ("1 file still conflicted".to_owned(), None)
    );
    let (refused, _) = key(&state, CoreKey::Char('u'));
    assert_eq!(
        sticky(&refused),
        ("finish or abort the rebase first".to_owned(), None)
    );
}

/// The shipped keymap with `id` bound to `chord` alone.
pub(super) fn with(id: &str, chord: &str) -> Keymap {
    let (map, problems) = Keymap::with_overrides(
        &[(id.to_owned(), vec![chord.to_owned()])],
        graft_ui::keymap::KeySwitches::default(),
    );
    assert_eq!(problems, [], "{id} = {chord} reads cleanly");
    map
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
    assert_eq!(resolve_keys(other, shipped).as_deref(), Some("⇥ ⇥ banner"));
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
        ("changes.stage", "mark resolved"),
    ];
    let swept = swept();
    assert_eq!(
        resolve_keys(other, &swept),
        Some(format!(
            "{0} {0} banner",
            swept.spell(&ActionId("app.next_pane")).0
        ))
    );
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
    // The changes pane's file list: two `⇥` (or two `⇧⇥`) to the banner.
    let (forward, back) = if ctx == ContextId::Changes {
        (2, 2)
    } else {
        (1, 3)
    };
    RefusedAt {
        ctx,
        on_conflict_row,
        way: crate::update::banner::BannerWay {
            forward,
            back: Some(back),
        },
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

/// m8/20c B4: `space` remapped, the CONFLICTS row's bar and `c`'s refusal
/// both read his key before the one label, `mark resolved`.
#[test]
fn the_mark_resolved_clause_is_spelled_from_his_keymap() {
    let mut state = stopped_at(Spot::ConflictRow);
    state.keymap = Arc::new(with("changes.stage", "x"));
    let want = format!("x {}", graft_ui::graph_screen::hints::MARK_RESOLVED);
    let banner = crate::update::banner::banner_view(&state);
    let context = crate::view::rows::footer_context(&state, &[], banner.as_ref());
    let bar: Vec<String> = graft_ui::graph_screen::hints_for(
        &context,
        graft_ui::graph_screen::Divider::Shown,
        state.size.columns,
        &state.keymap,
    )
    .iter()
    .filter_map(|hint| hint.text(&state.keymap))
    .collect();
    assert!(bar.contains(&want), "bar: {bar:?}");
    let (refused, _) = key(&state, CoreKey::Char('c'));
    let keys = sticky(&refused).1.expect("refusal keys");
    assert!(keys.ends_with(&want), "refusal: {keys}");
}
