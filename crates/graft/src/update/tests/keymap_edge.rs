//! The keymap's edge in a repository (m8-polish-and-ship/09b, ADR-0055): his
//! `[keys]` remaps translated before any dispatcher sees a key.
//!
//! **One projection for every case** ([`Seen`]): the commands a press emits,
//! what `quit` became, and the frame drawn after it at 120×36 — the same
//! three things for every context and every row, so "the same result" means
//! one thing throughout. No clock is read: the fixtures' harness clock is
//! fixed.
//!
//! The empty state's edge is `crate::empty_state`'s (no `AppState`): its
//! sweep is `crate::empty_state::sweep_tests`.
//!
//! Split (m8/09d, Minor 8): this file holds the fixtures and the sweep
//! keymaps; [`sweep`] the behavioural sweep over every reachable row;
//! [`remaps`] his config's lines through startup; [`unbound`] what an
//! unbound key draws.

mod fields;
mod palette;
mod palette_find;
mod palette_find_files_commits;
mod remaps;
mod sweep;
mod sweep_swallowed;
mod sweep_tables;
mod unbound;

pub(super) use fields::every_field;

use std::collections::HashMap;

use graft_core::ref_name::RefName;
use graft_ui::finish_summary::SummaryFocus;
use graft_ui::graph_screen::every_context;
use graft_ui::keymap::{ActionId, Binding, ContextId, KeyChord, KeyWarning, Keymap, defaults};

use super::*;
use std::sync::Arc;

use crate::update::edge::context_of;

/// What a press did, as far as anyone can see. `selected` is the Map's
/// row: it is drawn as a fill only (ADR-0058 §3), which a text frame cannot
/// show (rule 5), so it is read from the state (m8/09d Major 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Seen {
    commands: Vec<Command>,
    quit: String,
    frame: String,
    selected: usize,
}

pub(super) fn seen(state: &AppState, press: KeyPress) -> Seen {
    seen_as(state, press, &state.keymap)
}

/// [`seen`], with the frame drawn under `keys` whatever keymap `state`
/// presses with. Since m8/10 the bars spell their keys from the keymap, so
/// two keymaps draw two frames even where a press does the same thing: a
/// sweep comparing what a remapped press does with what the default does
/// draws both under the remapped keymap, and so compares the *doing*.
pub(super) fn seen_as(state: &AppState, press: KeyPress, keys: &Arc<Keymap>) -> Seen {
    let (mut after, mut commands) = update(state, AppEvent::Core(Event::Key(press)));
    after.keymap = Arc::clone(keys);
    // A job's keymap only spells the keys its words name (m8/10e), so it
    // is drawn under `keys` like the frame: the sweep compares the doing.
    for command in &mut commands {
        if let Command::Operate(.., carried)
        | Command::Resume(.., carried)
        | Command::Undo(.., carried)
        | Command::Redo(.., carried)
        | Command::RevertTo(.., carried)
        | Command::RecoverFor { keys: carried, .. } = command
        {
            *carried = Arc::clone(keys);
        }
    }
    Seen {
        commands,
        quit: format!("{:?}", after.quit),
        frame: crate::view::tests::draw(&after, after.size.columns, after.size.rows),
        selected: after.map.selected,
    }
}

/// A key no dispatcher answers anywhere: no binding, no arm, no field takes
/// `F20`. What every unbound default must do instead of its old action.
pub(super) fn unanswered() -> KeyPress {
    KeyPress::plain(KeyCode::Function(20))
}

pub(super) fn walk() -> JobId {
    JobId::new(701)
}

/// The merge on row 1 is `feature`'s tip, `HEAD` on `main` at row 0: a
/// branch row, so `m`/`r` open a preview and the branch keys have a branch.
pub(super) fn on_a_branch_row() -> AppState {
    let (mut state, _) = merge_on_screen(walk(), MERGE_ROW);
    let name = |full: &str| RefName::new(full.as_bytes().to_vec()).expect("a valid ref name");
    let oid = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let badges = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(1)),
            (name("refs/heads/feature"), oid(2)),
        ],
        &graft_git::repo::HeadState::Branch(name("refs/heads/main")),
        &HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("badges");
    state.refs = Some(Arc::new(badges));
    state
}

/// One state per context that has one in a repository, standing where
/// `context_of` says that context is. `None`: `Global` (its rows are
/// checked on the graph), `Palette` (ticket 11), `Tour` (19) and
/// `EmptyState` (no `AppState`).
pub(super) fn fixture(ctx: ContextId) -> Option<AppState> {
    use ContextId as C;
    let state = match ctx {
        C::Graph => on_a_branch_row(),
        // The overlays and the panes stand mid-list (09d Major 3): a row
        // above the cursor and a row below, so `j` and `k` both move.
        C::Map => mid_list(three_rows(overlay_over(on_a_branch_row(), Shown::Map))),
        C::Journal => mid_list(three_rows(overlay_over(on_a_branch_row(), Shown::Journal))),
        C::Stash => mid_list(three_rows(overlay_over(on_a_branch_row(), Shown::Stash))),
        C::Preview => {
            crate::update::preview::tests::opened_with(crate::update::preview::tests::a_plan()).0
        }
        C::Confirm => {
            let yes = every_context()
                .into_iter()
                .find(|c| matches!(c, HintContext::ConfirmBox { yes: Some(_) }))
                .expect("a confirm box with a yes button");
            box_over_the_graph(walk(), &yes)
        }
        C::Changes => mid_list(three_files(changes_pane_open(walk(), false))),
        C::Diff => mid_list(commit_pane(walk(), false)),
        C::Resolver => resolver_on_screen(walk()),
        C::Banner => stopped_operation(walk(), true, true),
        C::Finish => finish_screen_for(walk(), SummaryFocus::Files, true),
        C::Rewrite => {
            let mut state = merge_on_screen(walk(), MERGE_ROW).0;
            super::rewrite_editor::tests::open_an_editor_planned(&mut state, true);
            state
        }
        C::Picker => picker_over(on_a_branch_row()),
        // A place keeps its cursor in view after every key it gets, a key
        // it does not answer included (`place_key`'s fall-through). The
        // edge's unbound key never reaches it, so the fixture starts as the
        // loop would leave it: one key in, scrolled to its cursor.
        C::Blame => settled(&super::place::tests::place_on_screen(
            commit_pane(walk(), false),
            true,
            true,
        )),
        C::History => settled(
            &history_screens(
                walk(),
                &HintContext::History {
                    rows: true,
                    on_file: false,
                },
            )
            .remove(0),
        ),
        C::CommitEditor => {
            update(
                &super::commit_editor::tests::ready(&["a.rs"]),
                key(KeyCode::Char('c')),
            )
            .0
        }
        C::Credential => box_over_the_graph(walk(), &HintContext::Credential),
        C::Menu => update(&on_a_branch_row(), key(KeyCode::Char('.'))).0,
        C::Global | C::Palette | C::Tour | C::EmptyState => return None,
    };
    Some(sized_to_draw(state))
}

/// A fixture built without a terminal is 0×0, where every pane is too
/// small to act: give it the 120×36 the frame is drawn at.
pub(super) fn sized_to_draw(mut state: AppState) -> AppState {
    if state.size.columns == 0 {
        state.size = Size {
            columns: 120,
            rows: 36,
        };
    }
    state
}

/// The overlay `state` shows, given three rows: the Map three branches,
/// J three entries, the stash list three stashes.
fn three_rows(mut state: AppState) -> AppState {
    if let Loading::Ready(map) = &state.map.data {
        let mut map = (**map).clone();
        let mut third = map.branches[1].clone();
        third.name = RefName::new(b"refs/heads/b2".to_vec()).expect("a valid ref name");
        third.tip = Oid::from_bytes(&[3; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
        map.branches.push(third);
        state.map.data = Loading::Ready(Arc::new(map));
    }
    if state.journal.shown {
        let entries = (1..=3)
            .map(|n| graft_core::journal::JournalEntry {
                id: graft_core::journal::EntryId::new(n, [0; 10]),
                at_unix_ms: n,
                description: format!("commit \"x{n}\""),
                ..journal_entry()
            })
            .collect();
        state.journal.entries = Loading::Ready(Arc::new(crate::jobs::undo::JournalRead {
            entries,
            unreadable: Vec::new(),
            held: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        }));
        crate::update::journal::rebuild(&mut state);
    }
    if state.stash.shown {
        let entries = (0..3)
            .map(|index| graft_git::stash::StashEntry {
                index,
                message: format!("WIP {index} on main"),
                ..stash_entry()
            })
            .collect();
        state.stash.entries = Loading::Ready(Arc::new(entries));
    }
    state
}

/// The changes pane given a third unstaged file, `c.rs`.
fn three_files(mut state: AppState) -> AppState {
    if let Some(tree) = &state.working_tree {
        let mut tree = (**tree).clone();
        tree.entries.push(status_entry("c.rs", false));
        state.working_tree = Some(Arc::new(tree));
    }
    if let (Loading::Ready(view), Some(list)) = (&state.detail.files, &state.detail.list) {
        let (mut view, mut list) = ((**view).clone(), (**list).clone());
        let mut row = view.rows[view.rows.len() - 1].clone();
        row.name = sanitize_line("c.rs");
        row.entry = Some(2);
        view.rows.push(row);
        view.files += 1;
        if let graft_ui::detail::FileRowKind::Section { count, .. } = &mut view.rows[0].kind {
            *count += 1;
        }
        let mut entry = list.files[1].clone();
        entry.path = "c.rs".into();
        list.files.push(entry);
        state.detail.files = Loading::Ready(DetailArc::new(view));
        state.detail.list = Some(DetailArc::new(list));
    }
    state
}

/// `state` after one `j`: the cursor off the first row.
fn mid_list(state: AppState) -> AppState {
    update(&sized_to_draw(state), key(KeyCode::Char('j'))).0
}

/// `state` after a key nobody answers.
fn settled(state: &AppState) -> AppState {
    update(state, AppEvent::Core(Event::Key(unanswered()))).0
}

/// 09's sweep set: chords no default uses anywhere.
fn free_chords() -> Vec<KeyChord> {
    let mut out: Vec<KeyChord> = ('!'..='~')
        .map(|c| KeyChord::alt(KeyCode::Char(c)))
        .collect();
    out.extend((1..=12).map(|n| KeyChord::alt(KeyCode::Function(n))));
    out.extend(('a'..='z').map(|c| KeyChord {
        ctrl: true,
        ..KeyChord::alt(KeyCode::Char(c))
    }));
    out
}

/// Every movable action on its own free chord: the keymap, and per row the
/// chord it moved to.
pub(super) fn swept() -> (Arc<Keymap>, Vec<(&'static Binding, KeyChord)>) {
    let rows: Vec<&'static Binding> = defaults()
        .iter()
        .filter(|b| {
            // Not a text field's, not `⌃C`, not a row whose keys follow
            // another's, not one a switch keeps off (m8/09c).
            !b.context.is_text_field()
                && !matches!(
                    b.action.0,
                    "app.quit_now"
                        | "app.suspend"
                        | "preview.confirm_merge"
                        | "preview.confirm_rebase"
                )
        })
        .collect();
    let free = free_chords();
    assert!(
        free.len() >= rows.len(),
        "{} free for {}",
        free.len(),
        rows.len()
    );
    for b in defaults() {
        for chord in b.default {
            assert!(!free.contains(chord), "{} uses a sweep chord", b.action.0);
        }
    }
    let moved: Vec<(&'static Binding, KeyChord)> = rows.into_iter().zip(free).collect();
    let lines: Vec<(String, Vec<String>)> = moved
        .iter()
        .map(|(b, chord)| (b.action.0.to_owned(), vec![chord.label().0]))
        .collect();
    let (map, warnings) = Keymap::with_overrides(&lines, graft_ui::keymap::KeySwitches::default());
    assert_eq!(warnings, [], "every chord is free, so nothing collides");
    (Arc::new(map), moved)
}

/// The sweep's chords *ahead of* the defaults: each action answers both and
/// spells the new one first. The baseline presses with it, so whatever a
/// press builds with keys already spelled in it (a `.` menu's `(key)`
/// suffixes, spelled when the menu opens) reads as the swept keymap would
/// spell it, while the default chord still does what it always did.
fn both(moved: &[(&'static Binding, KeyChord)]) -> Arc<Keymap> {
    let lines: Vec<(String, Vec<String>)> = moved
        .iter()
        .map(|(b, chord)| {
            let mut labels = vec![chord.label().0];
            labels.extend(b.default.iter().map(|d| d.label().0));
            (b.action.0.to_owned(), labels)
        })
        .collect();
    let (map, warnings) = Keymap::with_overrides(&lines, graft_ui::keymap::KeySwitches::default());
    // Restating a default as an override reports the shadowing the default
    // table already has (an inner context's `⏎` over `app.open`): the inner
    // row keeps the chord, exactly as under the defaults. Anything else is a
    // real collision the sweep did not mean.
    for warning in &warnings {
        assert!(
            matches!(warning, KeyWarning::Collision { places, .. }
                if places.iter().all(|clash| clash.lost.0.starts_with("app."))),
            "{warning:?}"
        );
    }
    Arc::new(map)
}

/// Each fixture stands where `context_of` says it does, so the sweeps below
/// test the context they name.
#[test]
fn context_of_names_each_fixture() {
    for &ctx in ContextId::ALL {
        if let Some(state) = fixture(ctx) {
            assert_eq!(context_of(&state), ctx);
        }
    }
}

/// The labels of the `.` menu open on `state`, in order.
fn menu_labels(state: &AppState) -> Vec<String> {
    state
        .context_menu
        .as_ref()
        .expect("the menu is open")
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str().to_owned())
        .collect()
}

/// Whether `hint` names `action` as one of its keys.
fn names(hint: &graft_ui::graph_screen::Hint, action: ActionId) -> bool {
    use graft_ui::graph_screen::HintKey;
    hint.keys.iter().any(|key| {
        matches!(key, HintKey::Key(a) | HintKey::EveryChord(a) | HintKey::Fixed(a) if *a == action)
    })
}
