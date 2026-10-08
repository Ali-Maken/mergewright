//! Stop, outcome and failure words (m8/10f): each site's keys spelled from
//! his keymap through `graft_ui::keymap`, byte-identical under the shipped
//! one (10b W6).
//!
//! The banner's `c continue  a abort` is one helper,
//! `banner::continue_abort`: `focus_keys::nothing_conflicted`,
//! `pressed::{continued_in_git, land_after_continue_in_git, skip_pressed}`
//! and the journal's `in progress` row (`view::journal::summary`) all take
//! it, so its row here stands for all five.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::event::KeyPress;
use graft_core::job::JobId;
use graft_git::state::InProgress;
use graft_ops::EditsAt;
use graft_ops::operation::{Operation, Refusal, Target};
use graft_ui::keymap::{ActionId, ContextId, KeyChord, KeyCode, Keymap};

use super::{swept, unbinding, whole_tokens};
use crate::update::commit_editor::{Begun, operation_refusal};
use crate::update::state::Status;
use crate::update::{AppEvent, AppState, Product};

/// One site: its words under a keymap, and those words with each key
/// spelled by `spell` (the shipped letter, or the swept label).
struct Site {
    name: &'static str,
    keys: &'static [&'static str],
    words: fn(&Keymap) -> String,
    expected: fn(&dyn Fn(&'static str) -> String) -> String,
}

fn keys_of(status: Status) -> String {
    match status {
        Status::Sticky {
            keys: Some(keys), ..
        } => keys,
        other => panic!("a sticky line with keys: {other:?}"),
    }
}

fn refusal(kind: InProgress, begun: Begun, keys: &Keymap) -> String {
    let refusal = operation_refusal((kind, begun), 1, keys);
    format!("{} | {}", refusal.reason, refusal.keys.unwrap_or_default())
}

fn stage() -> Operation {
    Operation::Stage(Target::Paths(vec![PathBuf::from("a.txt")]))
}

const SITES: &[Site] = &[
    Site {
        name: "banner::continue_abort (focus_keys, pressed x3, journal summary)",
        keys: &["operation.continue", "operation.abort"],
        words: |keys| crate::update::banner::continue_abort(keys).unwrap_or_default(),
        expected: |k| {
            format!(
                "{} continue  {} abort",
                k("operation.continue"),
                k("operation.abort")
            )
        },
    },
    Site {
        name: "banner::abort_key (resume.rs failed resume)",
        keys: &["operation.abort"],
        words: |keys| crate::update::banner::abort_key(keys).unwrap_or_default(),
        expected: |k| format!("{} abort", k("operation.abort")),
    },
    Site {
        name: "operation_refusal merge in Graft",
        keys: &["operation.continue", "operation.abort"],
        words: |keys| refusal(InProgress::Merge, Begun::InGraft, keys),
        expected: |k| {
            format!(
                "merge in progress | ⇥ banner, then {} finish  {} abort",
                k("operation.continue"),
                k("operation.abort")
            )
        },
    },
    Site {
        name: "operation_refusal rebase in Graft",
        keys: &["operation.continue", "operation.abort"],
        words: |keys| refusal(InProgress::Rebase, Begun::InGraft, keys),
        expected: |k| {
            format!(
                "rebase in progress | ⇥ banner, then {} continue  {} abort",
                k("operation.continue"),
                k("operation.abort")
            )
        },
    },
    Site {
        name: "operation_refusal merge in shell",
        keys: &["operation.continue"],
        words: |keys| refusal(InProgress::Merge, Begun::InShell, keys),
        expected: |k| {
            format!(
                "merge begun in the shell | ⇥ banner, then {} finishes it",
                k("operation.continue")
            )
        },
    },
    Site {
        name: "operation_refusal rebase in shell",
        keys: &["operation.continue"],
        words: |keys| refusal(InProgress::Rebase, Begun::InShell, keys),
        expected: |k| {
            format!(
                "rebase begun in the shell — {} hands it to git | ",
                k("operation.continue")
            )
        },
    },
    Site {
        name: "edits_clause Stash (S p, W5)",
        keys: &["graph.stash", "stash.pop"],
        words: |keys| {
            crate::update::operate::edits_clause(EditsAt::Stash { n: 2 }, keys).unwrap_or_default()
        },
        expected: |k| {
            format!(
                "your edits are in stash@{{2}} · {} {} puts them back",
                k("graph.stash"),
                k("stash.pop")
            )
        },
    },
    Site {
        name: "edits_clause Back, no copy listed (S, 10j ruling 1b)",
        keys: &["graph.stash"],
        words: |keys| {
            crate::update::operate::edits_clause(EditsAt::Back { copy: None }, keys)
                .unwrap_or_default()
        },
        expected: |k| {
            format!(
                "your edits are back · a copy may still be in {}",
                k("graph.stash")
            )
        },
    },
    Site {
        name: "edits_clause Backup with copy",
        keys: &["app.undo"],
        words: |keys| {
            crate::update::operate::edits_clause(EditsAt::Backup { copy: Some(1) }, keys)
                .unwrap_or_default()
        },
        expected: |k| {
            format!(
                "{} restores the backup · a copy is in stash@{{1}}",
                k("app.undo")
            )
        },
    },
    Site {
        name: "edits_clause Backup",
        keys: &["app.undo"],
        words: |keys| {
            crate::update::operate::edits_clause(EditsAt::Backup { copy: None }, keys)
                .unwrap_or_default()
        },
        expected: |k| format!("{} restores the backup", k("app.undo")),
    },
    Site {
        name: "refusal_status DirtyTree",
        keys: &["graph.stash"],
        words: |keys| {
            keys_of(crate::update::operate::refusal_status(
                &stage(),
                &Refusal::DirtyTree { count: 3 },
                120,
                keys,
            ))
        },
        expected: |k| format!("{} stash 3 changed files", k("graph.stash")),
    },
    Site {
        name: "staging_not_kept_status",
        keys: &["app.undo"],
        words: |keys| {
            keys_of(crate::update::operate::staging_not_kept_status(
                "checkout fix",
                &[PathBuf::from("base.txt")],
                &[],
                200,
                keys,
            ))
        },
        expected: |k| format!("{} undoes it, staging included", k("app.undo")),
    },
];

/// The letter each id spelled before 10f.
fn letter(id: &str) -> &'static str {
    match id {
        "operation.continue" => "c",
        "operation.abort" => "a",
        "graph.stash" => "S",
        "stash.pop" => "p",
        "app.undo" => "u",
        "changes.edit" => "e",
        "changes.mergetool" => "M",
        "diff.side_by_side" => "v",
        "app.next_pane" => "⇥",
        other => panic!("no letter for {other}"),
    }
}

fn spelled(keymap: &Keymap, id: &'static str) -> String {
    keymap
        .spell_keys(&[ActionId(id)])
        .unwrap_or_else(|| panic!("{id} is bound"))
}

/// Pin: every site's words under the shipped keymap are the pre-10f
/// literal byte for byte (W6); under the swept keymap each key is its new
/// label and no default letter of it stands alone (W1).
#[test]
fn each_stop_word_spells_the_remapped_key() {
    assert_eq!(SITES.len(), 12, "every stop word reachable as a function");
    let shipped = Keymap::shipped();
    let swept = swept();
    for site in SITES {
        assert_eq!(
            (site.words)(shipped),
            (site.expected)(&|id| letter(id).to_owned()),
            "{}: shipped keymap, byte-identical",
            site.name
        );
        let line = (site.words)(&swept);
        assert_eq!(
            line,
            (site.expected)(&|id| spelled(&swept, id)),
            "{}: swept keymap",
            site.name
        );
        let bare = (site.expected)(&|_| String::new());
        for id in site.keys {
            let moved = spelled(&swept, id);
            assert_ne!(moved, letter(id), "{}: {id} moved", site.name);
            assert_eq!(
                whole_tokens(&line, letter(id)),
                whole_tokens(&bare, letter(id)),
                "{}: no default {} left: {line}",
                site.name,
                letter(id)
            );
        }
    }
}

/// W2: each key's row is the one his press answers where the words send
/// him. Context per site: the banner's keys (every stop, the refusal's
/// `⇥, then`, the amend-at-edit-stop `continues`, the journal row) → Banner;
/// `S` → Graph and `p` → Stash (the stash list `S` opens, W5); undo → the
/// graph; the markers refusal's `e`/`M` → Changes (`Space` on a CONFLICTS
/// row, `operate.rs` `Unmerged`); `v` → Diff; the finish notice's `⇥` →
/// Finish.
#[test]
fn each_word_names_the_row_that_answers_it() {
    const PINS: &[(ContextId, KeyChord, &str)] = &[
        (ContextId::Banner, KeyChord::key('c'), "operation.continue"),
        (ContextId::Banner, KeyChord::key('a'), "operation.abort"),
        (ContextId::Graph, KeyChord::key('S'), "graph.stash"),
        (ContextId::Stash, KeyChord::key('p'), "stash.pop"),
        (ContextId::Graph, KeyChord::key('u'), "app.undo"),
        (ContextId::Banner, KeyChord::key('u'), "app.undo"),
        (ContextId::Changes, KeyChord::key('e'), "changes.edit"),
        (ContextId::Changes, KeyChord::key('M'), "changes.mergetool"),
        (ContextId::Diff, KeyChord::key('v'), "diff.side_by_side"),
        (
            ContextId::Finish,
            KeyChord::plain(KeyCode::Tab),
            "app.next_pane",
        ),
    ];
    for (ctx, chord, id) in PINS {
        assert_eq!(
            Keymap::shipped().acting(*ctx, *chord),
            Some(ActionId(id)),
            "{id} in {ctx:?}"
        );
        assert_eq!(spelled(Keymap::shipped(), id), letter(id), "{id}");
    }
}

/// W3: an unbound key drops its clause whole; no `—` stands in for it.
#[test]
fn an_unbound_key_drops_its_clause() {
    let no_abort = unbinding("operation.abort");
    assert_eq!(
        crate::update::banner::continue_abort(&no_abort).as_deref(),
        Some("c continue")
    );
    assert_eq!(crate::update::banner::abort_key(&no_abort), None);
    assert_eq!(
        refusal(InProgress::Merge, Begun::InGraft, &no_abort),
        "merge in progress | ⇥ banner, then c finish"
    );
    let no_continue = unbinding("operation.continue");
    assert_eq!(
        refusal(InProgress::Rebase, Begun::InShell, &no_continue),
        "rebase begun in the shell | "
    );
    assert_eq!(
        operation_refusal((InProgress::Merge, Begun::InShell), 1, &no_continue).keys,
        None
    );
}

/// W3 for the failure lines: `S p` drops whole with either key unbound,
/// `S stash …` drops to no keys, undo's clauses go.
#[test]
fn an_unbound_key_drops_the_failure_clause() {
    let no_stash = unbinding("graph.stash");
    assert_eq!(
        crate::update::operate::edits_clause(EditsAt::Stash { n: 0 }, &no_stash).as_deref(),
        Some("your edits are in stash@{0}")
    );
    assert_eq!(
        crate::update::operate::refusal_status(
            &stage(),
            &Refusal::DirtyTree { count: 1 },
            120,
            &no_stash
        ),
        Status::Sticky {
            reason: "stage needs a clean tree".to_owned(),
            keys: None
        }
    );
    assert_eq!(
        crate::update::operate::edits_clause(EditsAt::Back { copy: None }, &no_stash).as_deref(),
        Some("your edits are back")
    );
    let no_pop = unbinding("stash.pop");
    assert_eq!(
        crate::update::operate::edits_clause(EditsAt::Stash { n: 0 }, &no_pop).as_deref(),
        Some("your edits are in stash@{0}")
    );
    let no_undo = unbinding("app.undo");
    assert_eq!(
        crate::update::operate::edits_clause(EditsAt::Backup { copy: None }, &no_undo),
        None
    );
    assert_eq!(
        crate::update::operate::edits_clause(EditsAt::Backup { copy: Some(1) }, &no_undo)
            .as_deref(),
        Some("a copy is in stash@{1}")
    );
    let Status::Sticky { reason, keys } = crate::update::operate::staging_not_kept_status(
        "checkout fix",
        &[PathBuf::from("base.txt")],
        &[],
        200,
        &no_undo,
    ) else {
        panic!("sticky");
    };
    assert_eq!(keys, None);
    assert!(!reason.contains("undoes"), "{reason}");
    assert!(!reason.ends_with(" · "), "{reason}");
}

// ------------------------------------------- m8/10j: the stops by place

/// The places `M`'s refusal and the open failure can leave him in.
const PLACES: [ContextId; 5] = [
    ContextId::Banner,
    ContextId::Resolver,
    ContextId::Changes,
    ContextId::Diff,
    ContextId::Graph,
];

/// The shipped row a key acts as at `ctx`.
fn acting(ctx: ContextId, key: char) -> Option<&'static str> {
    Keymap::shipped()
        .acting(ctx, KeyChord::key(key))
        .map(|id| id.0)
}

/// Ruling 1's pin: `e` answers with the place's own edit row; `a` is
/// `operation.abort` on the banner only; on the diff `e` is no edit row.
#[test]
fn the_mergetool_fallback_names_the_row_where_it_is_shown() {
    assert_eq!(acting(ContextId::Banner, 'e'), Some("operation.edit"));
    assert_eq!(acting(ContextId::Resolver, 'e'), Some("resolver.edit"));
    assert_eq!(acting(ContextId::Changes, 'e'), Some("changes.edit"));
    assert_ne!(acting(ContextId::Diff, 'e'), Some("changes.edit"));
    assert_eq!(acting(ContextId::Banner, 'a'), Some("operation.abort"));
    assert_ne!(acting(ContextId::Resolver, 'a'), Some("operation.abort"));
    assert_ne!(acting(ContextId::Changes, 'a'), Some("operation.abort"));
}

fn fallback(ctx: ContextId, keys: &Keymap) -> Option<String> {
    crate::update::banner::mergetool_fallback(ctx, Path::new("src/auth.rs"), keys)
}

/// Each place's line carries its own `e`, and no other place's.
#[test]
fn the_mergetool_fallback_spells_each_place_s_own_key() {
    let shipped = Keymap::shipped();
    assert_eq!(
        fallback(ContextId::Banner, shipped).as_deref(),
        Some("e edits src/auth.rs  a abort"),
        "the banner's line, byte-identical"
    );
    assert_eq!(
        fallback(ContextId::Resolver, shipped).as_deref(),
        Some("e edits src/auth.rs")
    );
    assert_eq!(
        fallback(ContextId::Changes, shipped).as_deref(),
        Some("e edits src/auth.rs")
    );
    assert_eq!(fallback(ContextId::Diff, shipped), None);
    assert_eq!(fallback(ContextId::Graph, shipped), None);
    let swept = swept();
    let rows = [
        (ContextId::Banner, "operation.edit"),
        (ContextId::Resolver, "resolver.edit"),
        (ContextId::Changes, "changes.edit"),
    ];
    for (ctx, own) in rows {
        let line = fallback(ctx, &swept).expect("an edit row here");
        assert!(
            line.starts_with(&format!("{} edits src/auth.rs", spelled(&swept, own))),
            "{ctx:?}: {line}"
        );
        for (_, other) in rows.iter().filter(|(_, id)| *id != own) {
            assert_eq!(
                whole_tokens(&line, &spelled(&swept, other)),
                0,
                "{ctx:?} names {other}: {line}"
            );
        }
        assert_eq!(whole_tokens(&line, "e"), 0, "{ctx:?}: {line}");
    }
    assert_eq!(
        fallback(ContextId::Banner, &swept),
        Some(format!(
            "{} edits src/auth.rs  {} abort",
            spelled(&swept, "operation.edit"),
            spelled(&swept, "operation.abort")
        ))
    );
}

/// W3: an unbound edit or abort drops its clause; no `—`, no `·`.
#[test]
fn an_unbound_edit_or_abort_drops_its_clause() {
    assert_eq!(
        fallback(ContextId::Resolver, &unbinding("resolver.edit")),
        None
    );
    assert_eq!(
        fallback(ContextId::Banner, &unbinding("operation.abort")).as_deref(),
        Some("e edits src/auth.rs")
    );
    assert_eq!(
        fallback(ContextId::Banner, &unbinding("operation.edit")).as_deref(),
        Some("a abort")
    );
}

/// The state `fixture` builds for `ctx`, his keymap `keys`, no
/// `merge.tool`, every listed file unmerged.
fn at(ctx: ContextId, keys: &Arc<Keymap>) -> AppState {
    let mut state = crate::update::tests::keymap_edge::fixture(ctx).expect("a fixture");
    state.keymap = Arc::clone(keys);
    state.mergetool = None;
    if let Some(tree) = state.working_tree.as_mut() {
        let mut all = (**tree).clone();
        for entry in &mut all.entries {
            entry.unmerged = true;
            entry.unmerged_how = Some(graft_git::status::UnmergedHow::BothModified);
        }
        *tree = Arc::new(all);
    }
    state
}

fn sticky_keys(state: &AppState) -> (String, Option<String>) {
    match &state.status {
        Status::Sticky { reason, keys } => (reason.clone(), keys.clone()),
        other => panic!("a sticky line: {other:?}"),
    }
}

fn press(state: &mut AppState, c: char) {
    let mut commands = Vec::new();
    crate::update::dispatch_key(
        state,
        &mut commands,
        KeyPress::plain(graft_core::event::KeyCode::Char(c)),
    );
}

/// Driven: `M` with no `merge.tool` from the banner, the resolver and a
/// CONFLICTS row; and the mergetool coming back failed in each
/// (`mergetool_returned`'s `Err` arm, `update/mod.rs`). Each line is that
/// place's spelling under his (swept) keymap.
#[test]
fn no_merge_tool_words_follow_the_place_he_pressed_m() {
    let swept = swept();
    for (ctx, own) in [
        (ContextId::Banner, "operation.edit"),
        (ContextId::Resolver, "resolver.edit"),
        (ContextId::Changes, "changes.edit"),
    ] {
        let mut state = at(ctx, &swept);
        assert_eq!(crate::update::edge::context_of(&state), ctx);
        press(&mut state, 'M');
        let (reason, keys) = sticky_keys(&state);
        assert_eq!(reason, "no merge.tool configured", "{ctx:?}");
        let keys = keys.expect("an edit clause");
        let mut words = keys.split_whitespace();
        assert_eq!(words.next(), Some(spelled(&swept, own).as_str()), "{keys}");
        assert_eq!(words.next(), Some("edits"), "{keys}");
        let path = words.next().expect("a path");
        assert_eq!(
            Some(keys.clone()),
            fallback_at(ctx, path, &swept),
            "{ctx:?}"
        );

        let state = at(ctx, &swept);
        let (state, _) = crate::update::update(
            &state,
            AppEvent::Produced(
                JobId::new(9_001),
                Product::Mergetool {
                    path: PathBuf::from("src/auth.rs"),
                    result: Err("vimdiff exited 1".to_owned()),
                },
            ),
        );
        assert_eq!(crate::update::edge::context_of(&state), ctx);
        assert_eq!(
            sticky_keys(&state),
            ("vimdiff exited 1".to_owned(), fallback(ctx, &swept)),
            "{ctx:?}: mergetool came back failed"
        );
    }
}

fn fallback_at(ctx: ContextId, path: &str, keys: &Keymap) -> Option<String> {
    crate::update::banner::mergetool_fallback(ctx, Path::new(path), keys)
}

/// The open failure's flow (`resolver::tests::file_list::
/// an_unmerged_open_failure_still_closes`, there reached by a click on the
/// list): the resolver's read fails unmerged and it closes.
fn open_failure(keys: &Arc<Keymap>) -> AppState {
    const READ: JobId = JobId::new(9_002);
    let mut state = at(ContextId::Resolver, keys);
    state.resolver.as_mut().expect("open").job = Some(READ);
    crate::update::update(
        &state,
        AppEvent::Produced(
            READ,
            Product::ConflictOpened(Err(crate::jobs::conflict::OpenFailure {
                words: "could not read the file: denied".to_owned(),
                not_unmerged: false,
                done: None,
            })),
        ),
    )
    .0
}

/// Ruling 4's pin: the close leaves him on the graph, where neither `e`
/// nor `M` is an edit or mergetool row (`M` is `graph.map`).
#[test]
fn an_open_failure_lands_him_where_its_words_are_read() {
    let state = open_failure(&swept());
    assert!(state.resolver.is_none(), "closed");
    assert_eq!(crate::update::edge::context_of(&state), ContextId::Graph);
    assert_eq!(acting(ContextId::Graph, 'M'), Some("graph.map"));
    assert!(
        !matches!(
            acting(ContextId::Graph, 'e'),
            Some("operation.edit" | "resolver.edit" | "changes.edit")
        ),
        "{:?}",
        acting(ContextId::Graph, 'e')
    );
}

fn edit_or_mergetool(ctx: ContextId, keys: &Keymap) -> Option<String> {
    crate::update::banner::edit_or_mergetool(ctx, keys)
}

#[test]
fn the_open_failure_names_only_keys_that_work_where_he_lands() {
    let shipped = Keymap::shipped();
    for ctx in [ContextId::Banner, ContextId::Resolver, ContextId::Changes] {
        assert_eq!(
            edit_or_mergetool(ctx, shipped).as_deref(),
            Some("e edit  M mergetool"),
            "{ctx:?}"
        );
    }
    assert_eq!(edit_or_mergetool(ContextId::Graph, shipped), None);
    assert_eq!(edit_or_mergetool(ContextId::Diff, shipped), None);
    let swept = swept();
    for (ctx, edit, tool) in [
        (ContextId::Banner, "operation.edit", "operation.mergetool"),
        (ContextId::Resolver, "resolver.edit", "resolver.mergetool"),
        (ContextId::Changes, "changes.edit", "changes.mergetool"),
    ] {
        assert_eq!(
            edit_or_mergetool(ctx, &swept),
            Some(format!(
                "{} edit  {} mergetool",
                spelled(&swept, edit),
                spelled(&swept, tool)
            )),
            "{ctx:?}"
        );
        assert_eq!(
            edit_or_mergetool(ctx, &unbinding(edit)).as_deref(),
            Some("M mergetool"),
            "{ctx:?}"
        );
        assert_eq!(
            edit_or_mergetool(ctx, &unbinding(tool)).as_deref(),
            Some("e edit"),
            "{ctx:?}"
        );
        let (neither, problems) = Keymap::with_overrides(
            &[
                (edit.to_owned(), vec!["none".to_owned()]),
                (tool.to_owned(), vec!["none".to_owned()]),
            ],
            graft_ui::keymap::KeySwitches::default(),
        );
        assert_eq!(problems, []);
        assert_eq!(edit_or_mergetool(ctx, &neither), None, "{ctx:?}");
    }
    // Driven: the line after the close, under the sweep, is the graph's —
    // the reason alone, no `·`.
    assert_eq!(
        sticky_keys(&open_failure(&swept)),
        ("could not read the file: denied".to_owned(), None)
    );
}

/// Both lines read one place → edit-row lookup: at each place, the `e`
/// either names is the same label.
#[test]
fn the_fallback_and_the_open_failure_name_the_same_edit_key() {
    let swept = swept();
    let first =
        |line: Option<String>| line.and_then(|l| l.split_whitespace().next().map(str::to_owned));
    for ctx in PLACES {
        let fallback = fallback(ctx, &swept).filter(|l| l.contains(" edits "));
        let open = edit_or_mergetool(ctx, &swept).filter(|l| l.contains(" edit"));
        assert_eq!(first(fallback), first(open), "{ctx:?}");
    }
}

/// Ruling 5: `⏎` from `resolver.mark_done`, the row the footer names.
#[test]
fn the_finish_refusal_spells_mark_done_from_the_resolver_row() {
    use crate::update::resolver::finish_refused_notice;
    assert_eq!(
        Keymap::shipped().acting(ContextId::Resolver, KeyChord::plain(KeyCode::Enter)),
        Some(ActionId("resolver.mark_done"))
    );
    assert_eq!(
        finish_refused_notice("1 file still conflicted", Keymap::shipped()),
        ("1 file still conflicted · ⏎ marks done".to_owned(), false)
    );
    let swept = swept();
    let key = spelled(&swept, "resolver.mark_done");
    let (words, flag) = finish_refused_notice("1 file still conflicted", &swept);
    assert_eq!(words, format!("1 file still conflicted · {key} marks done"));
    assert!(!flag);
    assert_eq!(whole_tokens(&words, "⏎"), 0, "{words}");
    assert_eq!(
        finish_refused_notice("1 file still conflicted", &unbinding("resolver.mark_done")),
        ("1 file still conflicted".to_owned(), false)
    );
}

/// Both callers, driven: `c` in the resolver with files still conflicted
/// (`banner/pressed.rs`), and a status read settling a `CHECKING` notice
/// to a refusal (`status_read::settle`). Each notice names his key.
#[test]
fn both_finish_refusals_spell_his_key() {
    let swept = swept();
    let want = format!(
        "2 files still conflicted · {} marks done",
        spelled(&swept, "resolver.mark_done")
    );
    let notice = |state: &AppState| {
        state
            .resolver
            .as_ref()
            .and_then(|open| open.notice.clone())
            .map(|(words, _)| words)
    };

    let mut state = at(ContextId::Resolver, &swept);
    press(&mut state, 'c');
    assert_eq!(notice(&state), Some(want.clone()), "c pressed");

    let mut state = at(ContextId::Resolver, &swept);
    state.resolver.as_mut().expect("open").notice =
        Some((crate::update::status_read::CHECKING.to_owned(), false));
    crate::update::status_read::settle(&mut state);
    assert_eq!(notice(&state), Some(want), "the read settled");
}

/// One inline stop word (m8/10k): the words a real event draws under
/// `keys`, and those words with each key spelled by `spell`.
struct Inline {
    name: &'static str,
    keys: &'static [&'static str],
    drawn: fn(&Arc<Keymap>) -> String,
    expected: fn(&dyn Fn(&'static str) -> String) -> String,
}

/// `key` pressed at `ctx` as the edge hands it on once it has translated
/// his chord: `dispatch_key` takes the default chord, whatever his keymap.
fn press_at(state: &mut AppState, ctx: ContextId, key: char) {
    assert_eq!(crate::update::edge::context_of(state), ctx);
    let mut commands = Vec::new();
    crate::update::dispatch_key(
        state,
        &mut commands,
        KeyPress::plain(graft_core::event::KeyCode::Char(key)),
    );
}

fn job_ids() -> impl FnMut() -> JobId {
    let mut next = 7_000;
    move || {
        next += 1;
        JobId::new(next)
    }
}

/// Launches `op` on `state` and answers it with `answer`, through
/// `update` as the job's reply arrives.
fn answered(
    mut state: AppState,
    op: Operation,
    answer: crate::jobs::operate::Operated,
) -> AppState {
    let mut commands = Vec::new();
    crate::update::operate::launch(&mut state, &mut commands, op, &mut job_ids())
        .expect("nothing else is running");
    let id = state.operating.expect("the launch took the slot");
    crate::update::update(
        &state,
        AppEvent::Produced(id, Product::Operated(Box::new(answer))),
    )
    .0
}

fn done_keys(status: &Status) -> String {
    match status {
        Status::Done {
            keys: Some(keys), ..
        } => keys.clone(),
        other => panic!("a done line with keys: {other:?}"),
    }
}

fn sticky_reason(status: &Status) -> String {
    match status {
        Status::Sticky { reason, .. } => reason.clone(),
        other => panic!("a sticky line: {other:?}"),
    }
}

/// A diff pane on `one_pair_diff`'s hunk, his keymap `keys`.
fn on_a_hunk(keys: &Arc<Keymap>, mode: graft_ui::detail::DiffMode) -> AppState {
    let mut state = crate::update::operate::tests::state_on_diff(
        graft_ui::detail::WorkingTreeSection::Unstaged,
        crate::update::operate::tests::one_pair_diff(),
        1,
    );
    state.detail.mode = mode;
    state.keymap = Arc::clone(keys);
    state
}

const INLINE: &[Inline] = &[
    Inline {
        name: "outcome: amend at an edit stop (rewrite_stop_tests)",
        keys: &["operation.continue"],
        drawn: |keys| {
            use crate::update::banner::rewrite_stop_tests::{a_amends, amended, edit_stop};
            // `a` pressed as shipped; the words are set when the amend
            // answers, under his keymap.
            let (mut launched, id) = a_amends(&edit_stop(), "wip: tests");
            launched.keymap = Arc::clone(keys);
            done_keys(&amended(&launched, id).status)
        },
        expected: |k| format!("{} continues", k("operation.continue")),
    },
    Inline {
        name: "outcome: a one-commit cherry-pick",
        keys: &["app.undo"],
        drawn: |keys| {
            let state = AppState {
                keymap: Arc::clone(keys),
                ..AppState::default()
            };
            let oid = graft_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes");
            let op = Operation::CherryPick(graft_ops::operation::PickParams {
                commits: vec![oid],
                subject: "fix the bug".to_owned(),
                head: graft_ops::operation::Tip {
                    name: "refs/heads/main".to_owned(),
                    oid,
                },
            });
            let done =
                crate::jobs::operate::Operated::Done(crate::update::banner::tests::done_applied());
            done_keys(&answered(state, op, done).status)
        },
        expected: |k| format!("{} undoes", k("app.undo")),
    },
    Inline {
        name: "failure: MarkersRemain on Space at a CONFLICTS row",
        keys: &["changes.edit", "changes.mergetool"],
        drawn: |keys| {
            let state = AppState {
                keymap: Arc::clone(keys),
                ..AppState::default()
            };
            let refused = crate::jobs::operate::Operated::Failed {
                error: "src/auth.rs still has conflict markers".to_owned(),
                refresh: None,
                remote: None,
                refused: Some(Refusal::MarkersRemain {
                    path: PathBuf::from("src/auth.rs"),
                }),
                edits: None,
            };
            keys_of(answered(state, stage(), refused).status)
        },
        expected: |k| {
            format!(
                "{} edit  {} mergetool",
                k("changes.edit"),
                k("changes.mergetool")
            )
        },
    },
    Inline {
        name: "finish: an empty subject (finish/tests opened)",
        keys: &["app.next_pane"],
        drawn: |keys| {
            let mut state = crate::update::finish::tests::opened();
            state.keymap = Arc::clone(keys);
            if let Some(open) = state.finish.as_mut() {
                open.subject = graft_ui::text_input::TextField::new("");
            }
            let mut commands = Vec::new();
            crate::update::dispatch_key(
                &mut state,
                &mut commands,
                KeyPress::plain(graft_core::event::KeyCode::Char('y')),
            );
            let finish = state.finish.as_ref().expect("the summary is open");
            let (words, _) = finish.notice.clone().expect("a notice");
            words
        },
        expected: |k| {
            format!(
                "the message is empty \u{b7} {} to write one",
                k("app.next_pane")
            )
        },
    },
    Inline {
        name: "abort box: a merge begun in the shell (banner/tests merge_read)",
        keys: &["app.undo"],
        drawn: |keys| {
            let mut state = crate::update::banner::tests::merge_read(Vec::new(), Some("main"));
            state.keymap = Arc::clone(keys);
            press_at(&mut state, ContextId::Banner, 'a');
            let pending = state
                .confirm
                .as_ref()
                .unwrap_or_else(|| panic!("the abort box: {:?}", state.status));
            pending
                .view
                .message
                .iter()
                .map(|line| line.as_str().to_owned())
                .collect::<Vec<_>>()
                .join(" | ")
        },
        expected: |k| {
            format!(
                "main goes back to before the merge | {} puts the merge back",
                k("app.undo")
            )
        },
    },
    Inline {
        name: "diff_cursor: Space in the side-by-side view",
        keys: &["diff.side_by_side"],
        drawn: |keys| {
            let mut state = on_a_hunk(keys, graft_ui::detail::DiffMode::SideBySide);
            let mut commands = Vec::new();
            assert!(crate::update::operate::stage_at_diff_cursor(
                &mut state,
                &mut commands,
                &mut job_ids()
            ));
            sticky_reason(&state.status)
        },
        expected: |k| {
            format!(
                "side by side has no line cursor \u{2014} press {} for the unified view to stage a hunk",
                k("diff.side_by_side")
            )
        },
    },
    Inline {
        name: "diff_cursor: the discard box's backup line",
        keys: &["app.undo"],
        drawn: |keys| {
            let mut state = on_a_hunk(keys, graft_ui::detail::DiffMode::Unified);
            assert!(crate::update::operate::request_discard_at_diff_cursor(
                &mut state
            ));
            let pending = state.confirm.as_ref().expect("the discard box");
            pending
                .view
                .message
                .last()
                .map(|line| line.as_str().to_owned())
                .expect("a backup line")
        },
        expected: |k| {
            format!(
                "Graft keeps a backup first, so `{}` can undo it afterwards.",
                k("app.undo")
            )
        },
    },
];

/// The seven inline stop words (10j ruling 2, m8/10k), each driven through
/// the event that draws it: under the shipped keymap byte-identical to the
/// pre-10f literal; under the swept keymap each key its new label, as a
/// whole token, and no default letter of a moved action left.
#[test]
fn each_inline_stop_word_spells_the_remapped_key() {
    assert_eq!(INLINE.len(), 7, "the seven inline stop words");
    let shipped = Arc::new(Keymap::shipped().clone());
    let swept = swept();
    for site in INLINE {
        assert_eq!(
            (site.drawn)(&shipped),
            (site.expected)(&|id| letter(id).to_owned()),
            "{}: shipped keymap, byte-identical",
            site.name
        );
        let line = (site.drawn)(&swept);
        assert_eq!(
            line,
            (site.expected)(&|id| spelled(&swept, id)),
            "{}: swept keymap",
            site.name
        );
        let bare = (site.expected)(&|_| String::new());
        for id in site.keys {
            let moved = spelled(&swept, id);
            assert_ne!(moved, letter(id), "{}: {id} moved", site.name);
            // The backup line quotes its key in backticks: trimmed too,
            // unless the label itself ends in one.
            let whole = line
                .split_whitespace()
                .filter(|word| {
                    *word == moved
                        || word.trim_matches(|c: char| "(),.;:`".contains(c)) == moved
                        || word.strip_prefix('`').and_then(|w| w.strip_suffix('`'))
                            == Some(moved.as_str())
                })
                .count();
            assert!(
                whole >= 1,
                "{}: {moved} as a whole token: {line}",
                site.name
            );
            assert_eq!(
                whole_tokens(&line, letter(id)),
                whole_tokens(&bare, letter(id)),
                "{}: no default {} left: {line}",
                site.name,
                letter(id)
            );
        }
    }
}

/// m8-polish-and-ship/20 item 17: the `InTheWay` rung is picked by the
/// width of what is drawn — the sanitised path — so a path full of control
/// characters (each drawn as a one-cell placeholder) still fits the row.
#[test]
fn an_in_the_way_rung_is_measured_as_drawn() {
    use unicode_width::UnicodeWidthStr;
    let keys = Keymap::shipped().clone();
    let path = format!("src/{}.rs", "\u{200b}".repeat(30));
    let refused = Refusal::InTheWay {
        doing: "switching to other".to_owned(),
        ignored: Vec::new(),
        untracked: vec![PathBuf::from(path)],
    };
    let Status::Sticky { reason, keys: next } =
        crate::update::operate::refusal_status(&stage(), &refused, 80, &keys)
    else {
        panic!("a sticky line");
    };
    let drawn = graft_term::sanitize_line(&reason);
    let suffix = next
        .map(|next| format!(" \u{b7} {next}"))
        .unwrap_or_default();
    assert!(
        drawn.as_str().width() + suffix.width() < 80,
        "{drawn}{suffix}"
    );
}
