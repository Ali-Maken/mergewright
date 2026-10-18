//! The conflict resolver's words (m8/10g): every line that names a key,
//! spelled from his keymap. Each site is driven through the code that
//! words it, keys pressed past the edge (`dispatch_key`), so a press is a
//! default chord whatever the keymap — what the dispatchers see.

use std::path::PathBuf;
use std::sync::Arc;

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_git::status::UnmergedHow;
use mergewright_ui::conflict_resolver::{RegionView, SideNames, SideRoles};
use mergewright_ui::keymap::{ActionId, ContextId, KeyChord, Keymap};

use super::{swept, unbinding, whole_tokens};
use crate::jobs::conflict::{OnDisk, OpenedFile, Opening, StartOver};
use crate::update::resolver::{OpenedFrom, done_refusal, open_resolver, whole_file_keys};
use crate::update::{AppState, Command, Status, dispatch_key};

const WALK: JobId = JobId::new(4_100);
const READ: JobId = JobId::new(4_900);

/// One site: the action ids its line names, and the line under a keymap.
#[derive(Clone)]
struct Site {
    name: &'static str,
    ids: &'static [&'static str],
    words: fn(&Arc<Keymap>) -> String,
    /// The line with each id spelled by `key` — the pre-10g literal when
    /// `key` is the shipped spelling.
    expected: fn(Spell<'_>) -> String,
}

/// Spells an action id: the shipped label, or the swept one.
type Spell<'a> = &'a dyn Fn(&str) -> String;

fn with_keys(mut state: AppState, keys: &Arc<Keymap>) -> AppState {
    state.keymap = Arc::clone(keys);
    state.size = crate::update::Size {
        columns: 120,
        rows: 48,
    };
    state
}

fn doc(keys: &Arc<Keymap>) -> AppState {
    with_keys(crate::update::tests::resolver_on_screen(WALK), keys)
}

fn card(keys: &Arc<Keymap>) -> AppState {
    with_keys(crate::update::tests::resolver_card_on_screen(WALK), keys)
}

fn press(state: &mut AppState, key: KeyPress) -> Vec<Command> {
    let mut commands = Vec::new();
    dispatch_key(state, &mut commands, key);
    commands
}

fn char_key(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

fn status_keys(state: &AppState) -> String {
    match &state.status {
        Status::Sticky { keys, .. } => keys.clone().unwrap_or_default(),
        other => panic!("expected a sticky line, got {other:?}"),
    }
}

fn notice(state: &AppState) -> String {
    let open = state.resolver.as_ref().expect("the resolver is open");
    open.notice
        .as_ref()
        .map(|(words, _)| words.clone())
        .unwrap_or_default()
}

/// The open file's read answered with `answer`.
fn read_lands(mut state: AppState, answer: Opening) -> AppState {
    let open = state.resolver.as_mut().expect("open");
    open.file = None;
    open.job = Some(READ);
    crate::update::resolver::conflict_opened(&mut state, READ, Ok(answer));
    state
}

fn done(keys: &Arc<Keymap>, file: mergewright_ops::conflict::DoneFile) -> AppState {
    read_lands(
        doc(keys),
        Opening::Done(
            Arc::new(file),
            StartOver::Can {
                kind: mergewright_ops::operation::RestartKind::Markers,
            },
        ),
    )
}

fn done_text(keys: &Arc<Keymap>) -> AppState {
    done(
        keys,
        mergewright_ops::conflict::DoneFile::Text {
            file: mergewright_git::conflict::parse(b"settled\n").expect("plain text parses"),
            unstaged: false,
        },
    )
}

fn view(state: &AppState) -> mergewright_ui::conflict_resolver::DocView {
    crate::view::resolver::resolver_view(state, 120).expect("a document on screen")
}

fn card_region(state: &AppState) -> (String, Option<String>) {
    match view(state).regions.first() {
        Some(RegionView::WholeFile { keys, note, .. }) => (
            keys.as_str().to_owned(),
            note.as_ref().map(|n| n.as_str().to_owned()),
        ),
        other => panic!("expected a card, got {other:?}"),
    }
}

fn names() -> SideNames {
    SideNames {
        ours: mergewright_term::sanitize_line("yours · main"),
        theirs: mergewright_term::sanitize_line("theirs · feature"),
        roles: SideRoles::Merge,
    }
}

const SHAPES: [(UnmergedHow, bool); 7] = [
    (UnmergedHow::BothModified, true),
    (UnmergedHow::DeletedByThem, false),
    (UnmergedHow::DeletedByUs, false),
    (UnmergedHow::AddedByUs, false),
    (UnmergedHow::AddedByThem, false),
    (UnmergedHow::BothDeleted, false),
    (UnmergedHow::BothModified, false),
];

fn shape_keys(i: usize, keys: &Keymap) -> String {
    let (how, binary) = SHAPES[i];
    whole_file_keys(how, binary, &names(), keys)
}

/// Every resolver site (21): the 19 `KNOWN: 10g` rows less the one left
/// (`e edit  M mergetool`, NOTES), plus `resolver.rs`'s undo line and the
/// view's title and finish row the grep did not catch.
fn sites() -> Vec<Site> {
    [
        taken_notices(),
        taken_notices_more(),
        landed_and_drawn(),
        landed_and_drawn_more(),
        refusals(),
        refusals_more(),
        shapes(),
    ]
    .concat()
}

/// The take and `⏎` notices.
fn taken_notices() -> Vec<Site> {
    vec![
        Site {
            name: "take.rs no conflict under the cursor",
            ids: &["resolver.next_block"],
            words: |k| {
                let mut state = doc(k);
                state.resolver.as_mut().expect("open").cursor_block = None;
                press(&mut state, char_key('o'));
                status_keys(&state)
            },
            expected: |key| format!("{} next", key("resolver.next_block")),
        },
        Site {
            name: "resolved.rs ⏎ refused with blocks open",
            ids: &["resolver.next_block"],
            words: |k| {
                let mut state = doc(k);
                press(&mut state, KeyPress::plain(KeyCode::Enter));
                status_keys(&state)
            },
            expected: |key| format!("{} next", key("resolver.next_block")),
        },
        Site {
            name: "take.rs all blocks taken at the press",
            ids: &["resolver.mark_done"],
            words: |k| {
                let mut state = doc(k);
                press(&mut state, char_key('o'));
                press(&mut state, char_key('o'));
                notice(&state)
            },
            expected: |key| {
                format!(
                    "all blocks taken · {} marks conflicted.rs done",
                    key("resolver.mark_done")
                )
            },
        },
    ]
}

/// More of the same group.
fn taken_notices_more() -> Vec<Site> {
    vec![
        Site {
            name: "taken.rs bulk take landed",
            ids: &["resolver.mark_done"],
            words: |k| {
                let mut state = doc(k);
                press(&mut state, KeyPress::control(KeyCode::Char('o')));
                land_all_ours(&mut state);
                notice(&state)
            },
            expected: |key| {
                format!(
                    "took yours in 2 blocks · {} marks conflicted.rs done",
                    key("resolver.mark_done")
                )
            },
        },
        Site {
            name: "taken.rs last take landed",
            ids: &["resolver.mark_done"],
            words: |k| {
                let mut state = doc(k);
                let open = state.resolver.as_mut().expect("open");
                let mut file = open.file.as_deref().expect("a document").clone();
                file.take(1, mergewright_git::conflict::Take::Ours)
                    .expect("block 1");
                open.file = Some(Arc::new(file));
                press(&mut state, char_key('o'));
                land_all_ours(&mut state);
                notice(&state)
            },
            expected: |key| {
                format!(
                    "all blocks taken · {} marks conflicted.rs done",
                    key("resolver.mark_done")
                )
            },
        },
    ]
}

/// A read landing, and what the view draws of it.
fn landed_and_drawn() -> Vec<Site> {
    vec![
        Site {
            name: "answers.rs no conflict markers",
            ids: &["resolver.mark_done"],
            words: |k| {
                let state = read_lands(
                    doc(k),
                    Opening::Unmerged(Arc::new(OpenedFile {
                        file: Some(
                            mergewright_git::conflict::parse(b"plain\n").expect("plain text parses"),
                        ),
                        how: UnmergedHow::BothModified,
                        binary: false,
                        on_disk: None,
                        unreadable: None,
                        start_over: None,
                    })),
                );
                notice(&state)
            },
            expected: |key| {
                format!(
                    "no conflict markers · {} marks it resolved",
                    key("resolver.mark_done")
                )
            },
        },
        Site {
            name: "done.rs a done file refuses",
            ids: &["resolver.start_over"],
            words: |k| done_refusal(&done_text(k)).expect("a done file refuses"),
            expected: |key| {
                format!(
                    "conflicted.rs is done · {} starts it over",
                    key("resolver.start_over")
                )
            },
        },
    ]
}

/// More of the same group.
fn landed_and_drawn_more() -> Vec<Site> {
    vec![
        Site {
            name: "view/resolver.rs done title",
            ids: &["resolver.start_over"],
            words: |k| view(&done_text(k)).title.as_str().to_owned(),
            expected: |key| {
                format!(
                    "conflicted.rs · done ✓ · {} starts it over",
                    key("resolver.start_over")
                )
            },
        },
        Site {
            name: "view/resolver/whole_file.rs done card",
            ids: &["resolver.start_over"],
            words: |k| card_region(&done(k, mergewright_ops::conflict::DoneFile::Removed)).0,
            expected: |key| format!("{} starts it over", key("resolver.start_over")),
        },
        Site {
            name: "view/resolver/whole_file.rs cut note",
            ids: &["resolver.edit"],
            words: |k| {
                let mut state = card(k);
                let whole = state
                    .resolver
                    .as_mut()
                    .and_then(|open| open.whole_file.as_mut())
                    .expect("a card");
                whole.on_disk = Some(OnDisk {
                    size: 11,
                    head: Arc::from(vec![b"kept by us".to_vec()]),
                    cut: true,
                });
                card_region(&state).1.expect("a cut file has a note")
            },
            expected: |key| {
                format!(
                    "… first 1 lines of 11 B · {} opens the whole file",
                    key("resolver.edit")
                )
            },
        },
    ]
}

/// The refusals and the finish row.
fn refusals() -> Vec<Site> {
    vec![
        Site {
            name: "open.rs already open",
            ids: &["app.back"],
            words: |k| {
                let mut state = doc(k);
                reopen(&mut state);
                status_keys(&state)
            },
            expected: |key| format!("{} closes it", key("app.back")),
        },
        Site {
            name: "open.rs a write is running",
            ids: &["app.back"],
            words: |k| {
                let mut state = doc(k);
                state.resolver = None;
                state.operating = Some(JobId::new(4_300));
                reopen(&mut state);
                status_keys(&state)
            },
            expected: |key| format!("{} stops it", key("app.back")),
        },
    ]
}

/// More of the same group.
fn refusals_more() -> Vec<Site> {
    vec![
        Site {
            name: "resolver/unpick.rs U in the resolver",
            ids: &["app.back", "app.undo"],
            words: |k| {
                let mut state = doc(k);
                state.in_progress = None;
                press(&mut state, char_key('U'));
                notice(&state)
            },
            expected: |key| {
                format!(
                    "{} first · {} undoes from the graph",
                    key("app.back"),
                    key("app.undo")
                )
            },
        },
        Site {
            name: "view/resolver.rs finish row",
            ids: &["resolver.finish"],
            words: |k| {
                // Every file staged: nothing left for `c` to refuse.
                let mut state = done_text(k);
                state.working_tree = Some(Arc::new(mergewright_git::status::WorkingTreeStatus {
                    entries: Vec::new(),
                }));
                // A rebase with a banner (a merge has the review row instead).
                {
                    use crate::update::resolver::fixtures::{commit, oid, operation, refs};
                    state.refs = Some(refs(false));
                    state.in_progress = Some(operation(
                        mergewright_git::state::InProgress::Rebase,
                        Some(commit(oid(3), "Add OAuth provider")),
                    ));
                }
                assert!(state.status_read.is_none(), "no read out");
                assert!(state.finish.is_none(), "no summary up");
                assert!(
                    crate::update::banner::banner_view(&state).is_some(),
                    "a banner"
                );
                crate::view::resolver::finish_row(&state)
                    .expect("a finish row")
                    .full
                    .as_str()
                    .to_owned()
            },
            expected: |key| {
                format!(
                    "✓ every file done — {} continue the rebase",
                    key("resolver.finish")
                )
            },
        },
    ]
}

/// The whole-file card keys, one per shape.
fn shapes() -> Vec<Site> {
    vec![
        shape(0, |k| {
            format!(
                "{} keeps yours · main  {} keeps theirs · feature",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
        shape(1, |k| {
            format!(
                "{} keeps the file  {} deletes it",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
        shape(2, |k| {
            format!(
                "{} deletes it  {} keeps theirs · feature",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
        shape(3, |k| {
            format!(
                "{} keeps yours · main  {} drops it",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
        shape(4, |k| {
            format!(
                "{} drops it  {} keeps theirs · feature",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
        shape(5, |k| {
            format!(
                "{} or {} removes it",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
        shape(6, |k| {
            format!(
                "{} keeps yours · main  {} keeps theirs · feature",
                k("resolver.take_ours"),
                k("resolver.take_theirs")
            )
        }),
    ]
}

/// A whole-file card's keys for `SHAPES[i]` — `fn` pointers can't capture,
/// so each index gets its own.
fn shape(i: usize, expected: fn(Spell<'_>) -> String) -> Site {
    let words: fn(&Arc<Keymap>) -> String = match i {
        0 => |k| shape_keys(0, k),
        1 => |k| shape_keys(1, k),
        2 => |k| shape_keys(2, k),
        3 => |k| shape_keys(3, k),
        4 => |k| shape_keys(4, k),
        5 => |k| shape_keys(5, k),
        _ => |k| shape_keys(6, k),
    };
    Site {
        name: "update/resolver/whole_file.rs card keys",
        ids: &["resolver.take_ours", "resolver.take_theirs"],
        words,
        expected,
    }
}

/// The in-flight take lands with every block taken ours.
fn land_all_ours(state: &mut AppState) {
    let open = state.resolver.as_ref().expect("open");
    let job = open.job.expect("a take in flight");
    let mut file = open.file.as_deref().expect("a document").clone();
    for block in 0..file.blocks().len() {
        if matches!(
            file.blocks()[block].state,
            mergewright_git::conflict::BlockState::Open
        ) {
            file.take(block, mergewright_git::conflict::Take::Ours)
                .expect("the block exists");
        }
    }
    let mut commands = Vec::new();
    crate::update::resolver::taken(state, job, Ok(Arc::new(file)), &mut commands, &mut || {
        JobId::new(4_800)
    });
}

fn reopen(state: &mut AppState) {
    let mut commands = Vec::new();
    let _spent = open_resolver(
        state,
        &mut commands,
        PathBuf::from("later.rs"),
        OpenedFrom::Pane,
        &mut || JobId::new(4_801),
    );
}

fn shipped_label(id: &str) -> String {
    Keymap::shipped()
        .live_labels(&ActionId(leak(id)))
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("{id} ships bound"))
}

fn leak(id: &str) -> &'static str {
    Box::leak(id.to_owned().into_boxed_str())
}

#[test]
fn each_resolver_word_spells_the_remapped_key() {
    let shipped = Arc::new(Keymap::shipped().clone());
    let swept = swept();
    let sites = sites();
    assert_eq!(sites.len(), 21, "one row per site");
    for site in &sites {
        // W6: the shipped keymap spells the literal it replaced, byte for byte.
        assert_eq!(
            (site.words)(&shipped),
            (site.expected)(&|id| shipped_label(id)),
            "{}: shipped",
            site.name
        );
        // W1: the swept keymap names each moved key, no default letter.
        let line = (site.words)(&swept);
        let label = |id: &str| {
            swept
                .live_labels(&ActionId(leak(id)))
                .first()
                .cloned()
                .unwrap_or_else(|| panic!("{id} swept bound"))
        };
        assert_eq!(line, (site.expected)(&label), "{}: swept", site.name);
        for id in site.ids {
            let old = shipped_label(id);
            if label(id) != old {
                assert_eq!(
                    whole_tokens(&line, &old),
                    0,
                    "{}: default {old} of {id} still in {line:?}",
                    site.name
                );
            }
        }
    }
}

/// W2: each key's row is the one that answers it where he presses it.
#[test]
fn each_word_names_the_row_that_answers_it() {
    let plain = |c: char| KeyChord::plain(KeyCode::Char(c));
    let pins: [(ContextId, KeyChord, &str); 12] = [
        // The resolver's own rows, where every notice and card is drawn.
        (ContextId::Resolver, plain('n'), "resolver.next_block"),
        (
            ContextId::Resolver,
            KeyChord::plain(KeyCode::Enter),
            "resolver.mark_done",
        ),
        (ContextId::Resolver, plain('d'), "resolver.start_over"),
        (ContextId::Resolver, plain('e'), "resolver.edit"),
        (ContextId::Resolver, plain('o'), "resolver.take_ours"),
        (ContextId::Resolver, plain('t'), "resolver.take_theirs"),
        (ContextId::Resolver, plain('c'), "resolver.finish"),
        // `esc closes it` and `esc first`: shown with the resolver open.
        (
            ContextId::Resolver,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        // `u undoes from the graph`: the place `esc` lands him (W5).
        (ContextId::Graph, plain('u'), "app.undo"),
        // `esc stops it`: the resolver did not open; the graph or the
        // changes pane it was asked from has the keys.
        (
            ContextId::Graph,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (
            ContextId::Changes,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (ContextId::Changes, plain('u'), "app.undo"),
    ];
    for (ctx, chord, id) in pins {
        assert_eq!(
            Keymap::shipped().acting(ctx, chord),
            Some(ActionId(id)),
            "{ctx:?} {chord:?}"
        );
    }
}

/// W3: `t` unbound drops its clause; an alternative keeps the bound one.
#[test]
fn an_unbound_key_drops_its_clause() {
    let keys = unbinding("resolver.take_theirs");
    let ours = shape_keys(0, &keys);
    assert_eq!(ours, "o keeps yours · main");
    assert_eq!(shape_keys(5, &keys), "o removes it");
    for i in 0..SHAPES.len() {
        assert!(!shape_keys(i, &keys).contains('—'), "shape {i}");
    }
    // Every key unbound: no clause at all.
    let both = Keymap::with_overrides(
        &[
            ("resolver.take_ours".to_owned(), vec!["none".to_owned()]),
            ("resolver.take_theirs".to_owned(), vec!["none".to_owned()]),
        ],
        mergewright_ui::keymap::KeySwitches::default(),
    )
    .0;
    assert_eq!(shape_keys(5, &both), "");
    // A clause after words: `⏎` unbound leaves the words alone.
    let state = doc(&Arc::new(unbinding("resolver.mark_done")));
    let mut state = state;
    press(&mut state, char_key('o'));
    press(&mut state, char_key('o'));
    assert_eq!(notice(&state), "all blocks taken");
}

/// The notice and the footer name `⏎`'s row the same way under the sweep.
#[test]
fn a_notice_and_the_footer_spell_mark_done_alike() {
    let swept = swept();
    // Every block taken and landed, nothing in flight: `⏎` is live.
    let mut state = doc(&swept);
    let open = state.resolver.as_mut().expect("open");
    let mut file = open.file.as_deref().expect("a document").clone();
    file.take(1, mergewright_git::conflict::Take::Ours)
        .expect("block 1");
    open.file = Some(Arc::new(file));
    press(&mut state, char_key('o'));
    land_all_ours(&mut state);
    let key = swept
        .live_labels(&ActionId("resolver.mark_done"))
        .first()
        .cloned()
        .expect("bound");
    assert!(notice(&state).contains(&format!("{key} marks")));
    let hints = mergewright_ui::graph_screen::hints_text(
        &mergewright_ui::graph_screen::hints_for(
            &crate::view::rows::resolver_context(&state),
            mergewright_ui::graph_screen::Divider::Hidden,
            1_000,
            &state.keymap,
        ),
        &state.keymap,
    );
    assert!(hints.contains(&format!("{key} done")), "{hints}");
}
