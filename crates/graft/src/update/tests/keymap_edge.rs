//! The keymap's edge in a repository (m8-polish-and-ship/09b, ADR-0055): his
//! `[keys]` remaps translated before any dispatcher sees a key.
//!
//! **One projection for every case** ([`Seen`]): the commands a press emits,
//! what `quit` became, and the frame drawn after it at 120×36 — the same
//! three things for every context and every row, so "the same result" means
//! one thing throughout. No clock is read: the fixtures' harness clock is
//! fixed.
//!
//! The empty state's edge is `crate::empty_state::tests`' (no `AppState`).

use std::collections::HashMap;

use graft_core::ref_name::RefName;
use graft_ui::finish_summary::SummaryFocus;
use graft_ui::graph_screen::every_context;
use graft_ui::keymap::{Binding, ContextId, KeyChord, Keymap, defaults};

use super::*;
use crate::update::edge::{SharedKeymap, context_of};

/// What a press did, as far as anyone can see.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    commands: Vec<Command>,
    quit: String,
    frame: String,
}

fn seen(state: &AppState, press: KeyPress) -> Seen {
    let (after, commands) = update(state, AppEvent::Core(Event::Key(press)));
    Seen {
        commands,
        quit: format!("{:?}", after.quit),
        frame: crate::view::tests::draw(&after, after.size.columns, after.size.rows),
    }
}

/// A key no dispatcher answers anywhere: no binding, no arm, no field takes
/// `F20`. What every unbound default must do instead of its old action.
fn unanswered() -> KeyPress {
    KeyPress::plain(KeyCode::Function(20))
}

fn walk() -> JobId {
    JobId::new(701)
}

/// The merge on row 1 is `feature`'s tip, `HEAD` on `main` at row 0: a
/// branch row, so `m`/`r` open a preview and the branch keys have a branch.
fn on_a_branch_row() -> AppState {
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
fn fixture(ctx: ContextId) -> Option<AppState> {
    use ContextId as C;
    let state = match ctx {
        C::Graph => on_a_branch_row(),
        C::Map => overlay_over(on_a_branch_row(), Shown::Map),
        C::Journal => overlay_over(on_a_branch_row(), Shown::Journal),
        C::Stash => overlay_over(on_a_branch_row(), Shown::Stash),
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
        C::Changes => changes_pane_open(walk(), false),
        C::Diff => commit_pane(walk(), false),
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
fn sized_to_draw(mut state: AppState) -> AppState {
    if state.size.columns == 0 {
        state.size = Size {
            columns: 120,
            rows: 36,
        };
    }
    state
}

/// `state` after a key nobody answers.
fn settled(state: &AppState) -> AppState {
    update(state, AppEvent::Core(Event::Key(unanswered()))).0
}

/// The default chord of `row` that acts as `row` in `ctx` out of the box:
/// the first context of the scope holding it is `row`'s. `None` when every
/// default is shadowed there (`app.open`'s `⏎` in the Map is `map.seek`).
fn acting_default(ctx: ContextId, row: &Binding) -> Option<KeyChord> {
    row.default.iter().copied().find(|chord| {
        ctx.scope()
            .iter()
            .find_map(|inner| {
                defaults()
                    .iter()
                    .find(|b| b.context == *inner && b.default.contains(chord))
            })
            .is_some_and(|holder| holder.action == row.action)
    })
}

/// The rows a fixture of `ctx` is checked against: its own and `Global`'s.
fn rows_for(ctx: ContextId) -> impl Iterator<Item = &'static Binding> {
    defaults()
        .iter()
        .filter(move |b| b.context == ctx || b.context == ContextId::Global)
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
fn swept() -> (SharedKeymap, Vec<(&'static Binding, KeyChord)>) {
    let rows: Vec<&'static Binding> = defaults()
        .iter()
        .filter(|b| !b.context.is_text_field() && b.action.0 != "app.quit_now")
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
    let (map, warnings) = Keymap::with_overrides(&lines);
    assert_eq!(warnings, [], "every chord is free, so nothing collides");
    (SharedKeymap::new(map), moved)
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

/// ADR-0055's consequence: every movable action on its own free chord. In
/// each context, the free chord does what the action's default does under
/// the default keymap, and every default does what a key nobody answers
/// does. In a text field every key goes on as typed, remapped or not.
#[test]
fn the_sweep_moves_every_action_and_every_context_follows() {
    let (swept, moved) = swept();
    let mut failures = Vec::new();
    let mut checked = 0;
    for &ctx in ContextId::ALL {
        let Some(base) = fixture(ctx) else {
            continue;
        };
        let remapped = AppState {
            keymap: swept.clone(),
            ..base.clone()
        };
        let nothing = seen(&base, unanswered());
        let rows: Vec<&str> = rows_for(ctx).map(|b| b.action.0).collect();
        for (row, to) in moved.iter().filter(|(b, _)| rows.contains(&b.action.0)) {
            let id = row.action.0;
            let (want, unbound) = if ctx.is_text_field() {
                (seen(&base, to.to_press()), None)
            } else {
                let acts = acting_default(ctx, row);
                let want = acts.map_or_else(|| nothing.clone(), |d| seen(&base, d.to_press()));
                (want, Some(&nothing))
            };
            if seen(&remapped, to.to_press()) != want {
                failures.push(format!("{ctx:?}: {id}'s new chord {}", to.label().0));
            }
            for chord in row.default {
                let want = match unbound {
                    Some(nothing) => nothing.clone(),
                    None => seen(&base, chord.to_press()),
                };
                if seen(&remapped, chord.to_press()) != want {
                    failures.push(format!("{ctx:?}: {id}'s old {}", chord.label().0));
                }
            }
            checked += 1;
        }
    }
    assert_eq!(failures, Vec::<String>::new());
    // 18 contexts with a fixture; the graph alone has its own and Global's.
    assert!(checked > 300, "only {checked} rows swept");
}

/// What 09's NOTES list as in the table but answered by a later ticket.
/// Each ticket deletes its own entries.
const NOT_YET_ANSWERED: &[(&str, &str)] = &[
    ("app.help", "ticket 11: `?`"),
    ("app.palette", "ticket 11: the palette"),
    ("palette.run", "ticket 11: the palette"),
    ("empty_state.clone", "ticket 18: clone"),
    ("empty_state.tour", "ticket 19: the practice tour"),
];

/// Rows whose default does nothing *in this fixture* — the key is answered,
/// but the fixture's data gives it nothing to change. Each says why.
const NO_EFFECT_IN_FIXTURE: &[(&str, &str)] = &[
    (
        "app.next_pane",
        "graph: no pane shown to move to (`detail_key`'s `Tab if detail.shown`)",
    ),
    (
        "app.prev_pane",
        "graph: no pane shown to move to (`BackTab if detail.shown`)",
    ),
    ("app.back", "graph: nothing open to back out of"),
    ("app.next_match", "graph: no search kept to step through"),
    ("app.prev_match", "graph: no search kept to step through"),
    ("graph.lanes", "graph: no lanes folded behind `+N`"),
    ("graph.hide_stash", "graph: no stash to hide"),
    (
        "graph.write_commit_graph",
        "graph: no commit-graph offer standing (`offer_keys_live`)",
    ),
    (
        "graph.decline_commit_graph",
        "graph: no commit-graph offer standing (`offer_keys_live`)",
    ),
    (
        "preview.rewrite",
        "preview: no suggested drop to keep (`keeps_drops`)",
    ),
    ("preview.left", "preview: its cursor is already leftmost"),
    (
        "confirm.left",
        "confirm box: its cursor is already leftmost",
    ),
    (
        "changes.mergetool",
        "changes: the row is not an unmerged file",
    ),
    (
        "diff.word_diff",
        "colour only (`word_tint`): the text frame cannot show it",
    ),
    (
        "diff.blame",
        "diff: no diff line under the cursor (`place::b_key`)",
    ),
    (
        "diff.fold_dir",
        "diff: no directory row under the cursor (`space_folds`)",
    ),
    (
        "rewrite.move_up",
        "rewrite editor: the cursor is on the first row",
    ),
    (
        "blame.heatmap",
        "colour only (`blame.heatmap`): the text frame cannot show it",
    ),
];

/// Default keymap: each row's default, pressed in its context's fixture,
/// changes what is seen — or the row is listed above with its reason.
/// Replaces 09's "every row is answered" criterion: `bound_actions` has no
/// context and cannot drive it (scout 09).
#[test]
fn every_row_is_answered_in_its_context() {
    let exempt = |id: &str| {
        NOT_YET_ANSWERED
            .iter()
            .chain(NO_EFFECT_IN_FIXTURE)
            .any(|(listed, _)| *listed == id)
    };
    let mut dead = Vec::new();
    let mut answered_by_exempt = Vec::new();
    for &ctx in ContextId::ALL {
        let Some(base) = fixture(ctx) else {
            continue;
        };
        let nothing = seen(&base, unanswered());
        let rows = defaults().iter().filter(|b| {
            b.context == ctx || (ctx == ContextId::Graph && b.context == ContextId::Global)
        });
        for row in rows {
            let id = row.action.0;
            let Some(chord) = acting_default(ctx, row) else {
                if !exempt(id) {
                    dead.push(format!("{ctx:?}: {id} is shadowed"));
                }
                continue;
            };
            let acts = seen(&base, chord.to_press()) != nothing;
            match (acts, exempt(id)) {
                (false, false) => dead.push(format!("{ctx:?}: {id} ({})", chord.label().0)),
                (true, true) => answered_by_exempt.push(format!("{ctx:?}: {id}")),
                _ => {}
            }
        }
    }
    assert_eq!(dead, Vec::<String>::new(), "rows nothing answers");
    assert_eq!(
        answered_by_exempt,
        Vec::<String>::new(),
        "listed as dead, but answered: delete the entry"
    );
}

/// Journey J8 from the file on disk: `suspend_on_ctrl_z` beside a
/// `[keys.graph]` table, read by `config::load_from`, made a keymap by
/// `keymap_from`, seeded as startup seeds it. (The file is written by
/// `config::tests`: `update` names no file system, tests included.)
/// `M` was the Map's: merge takes it, the Map loses it — the one problem.
#[test]
fn j8_a_keys_table_beside_suspend_moves_merge_and_says_one_problem() {
    let loaded = crate::config::tests::loaded_from_a_file(
        "[keys]\nsuspend_on_ctrl_z = true\n\n[keys.graph]\nmerge = \"M\"\n",
    );
    assert!(
        loaded.config.keys.suspend_on_ctrl_z,
        "suspend still reads true"
    );
    assert_eq!(
        loaded.warnings,
        [],
        "no unknown-key warning for `keys.graph` or its line"
    );
    let (keymap, problems) = crate::config::keymap_from(&loaded);
    assert_eq!(problems.len(), 1, "{problems:?}");

    let mut base = sized_to_draw(on_a_branch_row());
    base.seed_keys(SharedKeymap::new(keymap), problems);
    let first = crate::view::tests::draw(&base, base.size.columns, base.size.rows);
    assert!(
        first.contains("config: 1 problem · ? shows them"),
        "the bottom row says so:\n{first}"
    );
    // `docs/shots/m8-keymap-config-problem-120x36.txt`. What right looks
    // like: the graph as normal, the bottom row `config: 1 problem · ?
    // shows them` (muted). Colour unverified (rule 5).
    graft_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-keymap-config-problem-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        &first,
    );

    let (merged, _) = update(
        &base,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('M')))),
    );
    assert!(
        merged.preview.is_some(),
        "M on a branch row opens the merge preview"
    );
    assert_eq!(
        seen(&base, KeyPress::plain(KeyCode::Char('m'))),
        seen(&base, unanswered()),
        "m does what a key nobody answers does"
    );
}

/// The notice is there until the first key, and never with no problems.
#[test]
fn the_config_notice_goes_at_the_first_key_and_needs_a_problem() {
    let mut state = on_a_branch_row();
    state.seed_keys(
        SharedKeymap::default(),
        vec!["graf.merge is not an action".to_owned()],
    );
    let draw = |state: &AppState| crate::view::tests::draw(state, 120, 36);
    assert!(draw(&state).contains("config: 1 problem · ? shows them"));
    let after = update(&state, AppEvent::Core(Event::Key(unanswered()))).0;
    assert!(!draw(&after).contains("config:"), "gone at the first key");

    let mut clean = on_a_branch_row();
    clean.seed_keys(SharedKeymap::default(), Vec::new());
    assert!(
        !draw(&clean).contains("config:"),
        "nothing to report, nothing said"
    );
}

/// Every text field `text_field_open` names, each open and empty: the
/// search prompt, blame's find, J's find, the stash prompt, the commit
/// editor, the credential box, the name prompt, the picker, the rewrite
/// editor's `r` field and the finish screen's subject.
fn every_field() -> Vec<(&'static str, AppState)> {
    let typed = |state: AppState, c: char| update(&state, key(KeyCode::Char(c))).0;
    let mut rewrite = on_a_branch_row();
    super::rewrite_editor::tests::open_an_editor_field(&mut rewrite);
    let fields = vec![
        ("search", typed(on_a_branch_row(), '/')),
        (
            "blame find",
            typed(fixture(ContextId::Blame).expect("blame"), '/'),
        ),
        (
            "J find",
            typed(fixture(ContextId::Journal).expect("J"), '/'),
        ),
        (
            "stash prompt",
            typed(fixture(ContextId::Stash).expect("stash"), 's'),
        ),
        (
            "commit editor",
            fixture(ContextId::CommitEditor).expect("editor"),
        ),
        (
            "credential",
            fixture(ContextId::Credential).expect("credential"),
        ),
        (
            "name prompt",
            box_over_the_graph(walk(), &HintContext::NamePrompt { toggle: true }),
        ),
        ("picker", fixture(ContextId::Picker).expect("picker")),
        ("rewrite field", rewrite),
        (
            "finish subject",
            finish_screen_for(walk(), SummaryFocus::Subject, true),
        ),
    ];
    fields
        .into_iter()
        .map(|(name, state)| (name, sized_to_draw(state)))
        .collect()
}

/// ADR-0055 §2.1: typed text is never remapped. With `graph.merge = "x"`,
/// `x` typed into each field is the letter `x` — the same press, the same
/// result, as under the default keymap — and the field shows one more `x`.
#[test]
fn x_typed_into_every_field_is_an_x_with_merge_on_x() {
    // `x` is `graph.compare`'s: merge keeps it, compare loses it — on the
    // graph `x` now merges, and still types an `x` everywhere text goes.
    let (map, warnings) =
        Keymap::with_overrides(&[("graph.merge".to_owned(), vec!["x".to_owned()])]);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let remapped = SharedKeymap::new(map);
    for (name, base) in every_field() {
        assert!(base.text_field_open(), "{name}: the field is open");
        let moved = AppState {
            keymap: remapped.clone(),
            ..base.clone()
        };
        let press = KeyPress::plain(KeyCode::Char('x'));
        assert_eq!(
            seen(&moved, press),
            seen(&base, press),
            "{name}: as under the default keymap"
        );
        let before = seen(&base, unanswered()).frame.matches('x').count();
        let after = seen(&moved, press).frame.matches('x').count();
        assert!(
            after > before,
            "{name}: the field shows the x ({before} → {after})"
        );
    }
}
