//! Stop words: `M`'s fallback, the open failure and the finish refusals, by the place
//! he is in (m8/10j). Moved out of `stops.rs` (m8/21b-5, move-only).

use super::*;

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
