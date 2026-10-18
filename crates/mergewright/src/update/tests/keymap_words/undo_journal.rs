//! Undo, journal, recover and status-row words (m8/10h): each site's keys
//! spelled from his keymap, byte-identical under the shipped one except
//! search's `Esc` → `esc` (10b W6, 10 decision 3).
//!
//! Sites reached through one helper stand for all their callers:
//! `recover::undoes` (`recover::done_status`, `amend_into::landing`),
//! `clash::brings_back` (`clash::dropped_status`, the stash pop box),
//! `banner::tab_keys` (`in_progress_refusal`, the `OperationLive` answer,
//! `c`'s refusal off a CONFLICTS row, `operation_refusal`).

use mergewright_ui::keymap::{ActionId, ContextId, KeyChord, KeyCode, Keymap};

use super::{swept, unbinding, whole_tokens};
use crate::update::state::Status;

struct Site {
    name: &'static str,
    keys: &'static [&'static str],
    words: fn(&Keymap) -> String,
    expected: fn(&dyn Fn(&'static str) -> String) -> String,
}

fn some(words: Option<String>) -> String {
    words.unwrap_or_else(|| panic!("bound, so words"))
}

const SITES: &[Site] = &[
    Site {
        name: "banner::way_keys (live refusal, OperationLive)",
        keys: &["app.next_pane"],
        words: |k| {
            some(crate::update::banner::way_keys(
                crate::update::banner::BannerWay::tabs(2),
                k,
            ))
        },
        expected: |k| format!("{0} {0} banner", k("app.next_pane")),
    },
    Site {
        name: "undo::redo_clause (nothing to undo)",
        keys: &["app.redo"],
        words: |k| some(crate::update::undo::redo_clause("checkout fix", k)),
        expected: |k| format!("{} redoes checkout fix", k("app.redo")),
    },
    Site {
        name: "undo::journal_keys (revert stopped)",
        keys: &["graph.journal"],
        words: |k| some(crate::update::undo::journal_keys(k)),
        expected: |k| format!("{} journal", k("graph.journal")),
    },
    Site {
        name: "undo::notice_line (relaunch notice)",
        keys: &["graph.journal"],
        words: |k| {
            crate::update::undo::notice_line(
                "checkout fix",
                Some("your edits are back"),
                usize::MAX,
                k,
            )
        },
        expected: |k| {
            format!(
                "an operation was interrupted: checkout fix \u{b7} your edits are back \u{b7} {} to inspect",
                k("graph.journal")
            )
        },
    },
    Site {
        name: "undo::unreadable_words (relaunch notice)",
        keys: &["app.undo"],
        words: crate::update::undo::unreadable_words,
        expected: |k| {
            format!(
                "its state could not be read, so {} restores the whole tree",
                k("app.undo")
            )
        },
    },
    Site {
        name: "journal::answers::offer_branch_keys",
        keys: &["journal.branch"],
        words: |k| {
            some(crate::update::journal::answers::offer_branch_keys(
                "spike", "abc1234", k,
            ))
        },
        expected: |k| format!("{} makes branch \"spike\" at abc1234", k("journal.branch")),
    },
    Site {
        name: "journal::keys::lost_edit_keys",
        keys: &["journal.revert"],
        words: |k| some(crate::update::journal::keys::lost_edit_keys(k)),
        expected: |k| format!("{} puts it back", k("journal.revert")),
    },
    Site {
        name: "journal::keys::branch_needs_row",
        keys: &["journal.branch"],
        words: |k| some(crate::update::journal::keys::branch_needs_row(k)),
        expected: |k| {
            format!(
                "{} makes a branch on a $ or \u{2717} row",
                k("journal.branch")
            )
        },
    },
    Site {
        name: "recover::undoes (done_status, amend_into landing)",
        keys: &["app.undo"],
        words: |k| some(crate::update::recover::undoes(k)),
        expected: |k| format!("{} undoes", k("app.undo")),
    },
    Site {
        name: "recover::undoes_it (preview_lines)",
        keys: &["app.undo"],
        words: |k| some(crate::update::recover::undoes_it(k)),
        expected: |k| format!("{} undoes it", k("app.undo")),
    },
    Site {
        name: "switch::goes_back (operate outcome)",
        keys: &["app.undo"],
        words: |k| some(crate::update::switch::goes_back(k)),
        expected: |k| format!("{} goes back", k("app.undo")),
    },
    Site {
        name: "clash::undo_clause_words set-aside",
        keys: &["app.back", "app.undo"],
        words: |k| some(crate::update::clash::undo_clause_words(Some("rebase"), k)),
        expected: |k| format!("{}, {} undoes the rebase too", k("app.back"), k("app.undo")),
    },
    Site {
        name: "clash::undo_clause_words",
        keys: &["app.back", "app.undo"],
        words: |k| some(crate::update::clash::undo_clause_words(None, k)),
        expected: |k| format!("{}, {} puts it all back", k("app.back"), k("app.undo")),
    },
    Site {
        name: "clash::dropped_status",
        keys: &["app.undo"],
        words: |k| match crate::update::clash::dropped_status(
            Some(0),
            mergewright_core::journal::PutBackWhy::Pop,
            k,
        ) {
            Status::Done {
                keys: Some(keys), ..
            } => keys,
            other => panic!("done with keys: {other:?}"),
        },
        expected: |k| format!("{} brings it back", k("app.undo")),
    },
    Site {
        name: "preview::boxes::popped_line (stash pop box)",
        keys: &["app.undo"],
        words: |k| crate::update::preview::boxes::popped_line("stash@{0}", k),
        expected: |k| {
            format!(
                "stash@{{0}} is dropped \u{b7} {} brings it back",
                k("app.undo")
            )
        },
    },
    Site {
        name: "search::keys_words (finished, hits)",
        keys: &["app.next_match", "app.prev_match", "app.back"],
        words: |k| crate::search::keys_words(false, 2, k),
        expected: |k| {
            format!(
                "  {}/{} next/prev  {} close",
                k("app.next_match"),
                k("app.prev_match"),
                k("app.back")
            )
        },
    },
    Site {
        name: "search::keys_words (running, no hits)",
        keys: &["app.back"],
        words: |k| crate::search::keys_words(true, 0, k),
        expected: |k| format!("  {} cancel", k("app.back")),
    },
    Site {
        name: "place::answer::cannot_tell",
        keys: &["app.back"],
        words: |k| crate::update::place::answer::cannot_tell(None, None, k),
        expected: |k| {
            format!(
                "can't tell where the line came from \u{b7} {} back",
                k("app.back")
            )
        },
    },
    Site {
        name: "quit::wait_words",
        keys: &["app.back"],
        words: |k| crate::update::quit::wait_words("push", k),
        expected: |k| {
            format!(
                "push \u{b7} quitting when it finishes \u{b7} {} stays",
                k("app.back")
            )
        },
    },
    Site {
        name: "quit::long_wait_words",
        keys: &["app.quit", "app.back"],
        words: |k| crate::update::quit::long_wait_words("push", k),
        expected: |k| {
            format!(
                "push \u{b7} still running after 10 s \u{b7} {} stop it now \u{b7} {} stays",
                k("app.quit"),
                k("app.back")
            )
        },
    },
    Site {
        name: "view::frame::bars::config_words",
        keys: &["app.help"],
        words: |k| crate::view::frame::bars::config_words(2, k),
        expected: |k| format!("config: 2 problems \u{b7} {} shows them", k("app.help")),
    },
    Site {
        name: "view::status::dismiss",
        keys: &["app.back"],
        words: |k| some(crate::view::status::dismiss(k)),
        expected: |k| format!("{} dismiss", k("app.back")),
    },
];

/// The letter each id spelled before 10h. Search's `Esc` was `Esc`; the
/// shipped keymap spells `esc` (W6, 10 decision 3) — the one byte change.
fn letter(id: &str) -> &'static str {
    match id {
        "app.next_pane" => "\u{21e5}",
        "app.redo" => "U",
        "graph.journal" => "J",
        "app.undo" => "u",
        "journal.branch" => "b",
        "journal.revert" => "\u{23ce}",
        "app.back" => "esc",
        "app.next_match" => "n",
        "app.prev_match" => "N",
        "app.quit" => "q",
        "app.help" => "?",
        other => panic!("no letter for {other}"),
    }
}

fn spelled(keymap: &Keymap, id: &'static str) -> String {
    keymap
        .spell_keys(&[ActionId(id)])
        .unwrap_or_else(|| panic!("{id} is bound"))
}

/// W1 + W6: shipped → the pre-10h literal byte for byte (search: `esc`);
/// swept → each key its new label, no default letter left standing alone.
#[test]
fn each_word_spells_the_remapped_key() {
    assert_eq!(SITES.len(), 22, "every 10h site reachable as a function");
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
            assert_ne!(spelled(&swept, id), letter(id), "{}: {id} moved", site.name);
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

/// W6: the search hint under the shipped keymap — `Esc` → `esc` is the
/// only change from the pre-10h literal.
#[test]
fn the_search_hint_normalises_esc_only() {
    let shipped = Keymap::shipped();
    let cases = [
        (true, 0, "  Esc cancel"),
        (true, 3, "  n/N next/prev  Esc cancel"),
        (false, 0, "  Esc close"),
        (false, 3, "  n/N next/prev  Esc close"),
    ];
    for (running, hits, old) in cases {
        assert_eq!(
            crate::search::keys_words(running, hits, shipped),
            old.replace("Esc", "esc")
        );
    }
}

/// W2: each key's row is the one his press answers where the words show.
/// Contexts: undo's refusal, redo, `J`, `u` (recover, switch, the stash
/// pop, the dropped copy), search's `n`/`N`/esc (`update/search.rs:26`
/// steps on the translated key; `keys/escape.rs:11` cancels), the quit
/// wait and the config notice → Graph; the journal's rows → Journal;
/// a clash's `esc,` → Resolver (the notice lands with it open, then `u` on
/// the graph); `^`'s `esc back` → Blame; the credential row → Credential,
/// a text field (W4: shipped).
#[test]
fn each_word_names_the_row_that_answers_it() {
    const PINS: &[(ContextId, KeyChord, &str)] = &[
        (
            ContextId::Graph,
            KeyChord::plain(KeyCode::Tab),
            "app.next_pane",
        ),
        (ContextId::Graph, KeyChord::key('U'), "app.redo"),
        (ContextId::Graph, KeyChord::key('J'), "graph.journal"),
        (ContextId::Graph, KeyChord::key('u'), "app.undo"),
        (ContextId::Journal, KeyChord::key('b'), "journal.branch"),
        (
            ContextId::Journal,
            KeyChord::plain(KeyCode::Enter),
            "journal.revert",
        ),
        (
            ContextId::Resolver,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (ContextId::Graph, KeyChord::key('n'), "app.next_match"),
        (ContextId::Graph, KeyChord::key('N'), "app.prev_match"),
        (
            ContextId::Graph,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (
            ContextId::Blame,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (ContextId::Graph, KeyChord::key('q'), "app.quit"),
        (ContextId::Graph, KeyChord::key('?'), "app.help"),
        (
            ContextId::Credential,
            KeyChord::plain(KeyCode::Enter),
            "credential.send",
        ),
        (
            ContextId::Credential,
            KeyChord::ctrl('u'),
            "credential.clear",
        ),
        (
            ContextId::Credential,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
    ];
    for (ctx, chord, id) in PINS {
        assert_eq!(
            Keymap::shipped().acting(*ctx, *chord),
            Some(ActionId(id)),
            "{id} in {ctx:?}"
        );
    }
    assert!(ContextId::Credential.is_text_field());
}

/// W4: the credential prompt is a text field; its keys spell from the
/// shipped keymap whatever his config says.
#[test]
fn the_credential_keys_are_the_shipped_ones_under_any_keymap() {
    let shipped_keys = "\u{23ce} send  \u{2303}U clear  esc cancel";
    assert_eq!(crate::view::credential::footer_keys(), shipped_keys);
    let mut state = crate::update::tests::keymap_edge::fixture(ContextId::Credential)
        .expect("a credential fixture");
    state.keymap = std::sync::Arc::new(Keymap::shipped().clone());
    let shipped = crate::view::credential::footer(&state);
    state.keymap = swept();
    let under_sweep = crate::view::credential::footer(&state);
    assert!(shipped.starts_with(shipped_keys), "{shipped}");
    assert_eq!(under_sweep, shipped, "W4: a text field's keys are fixed");
}

/// W3: an unbound key drops its clause whole; never `—`.
#[test]
fn an_unbound_key_drops_its_clause() {
    let no_redo = unbinding("app.redo");
    assert_eq!(
        crate::update::undo::redo_clause("checkout fix", &no_redo),
        None
    );

    let no_journal = unbinding("graph.journal");
    assert_eq!(crate::update::undo::journal_keys(&no_journal), None);
    let notice = crate::update::undo::notice_line(
        "checkout fix",
        Some("your edits are back"),
        usize::MAX,
        &no_journal,
    );
    assert_eq!(
        notice,
        "an operation was interrupted: checkout fix \u{b7} your edits are back"
    );
    assert_eq!(
        crate::update::undo::notice_line("checkout fix", None, usize::MAX, &no_journal),
        "an operation was interrupted: checkout fix"
    );
}

/// W3 for `esc`: dismiss, search, the quit wait, blame's `^`, a clash.
#[test]
fn an_unbound_back_drops_its_clause() {
    let no_back = unbinding("app.back");
    assert_eq!(crate::view::status::dismiss(&no_back), None);
    let failed = Status::Failed("hook said no".to_owned());
    let line = crate::view::status::graph_problem(&failed, &no_back).expect("a problem");
    assert_eq!(line.keys, None);
    assert_eq!(
        crate::search::keys_words(false, 2, &no_back),
        "  n/N next/prev"
    );
    assert_eq!(
        crate::update::quit::wait_words("push", &no_back),
        "push \u{b7} quitting when it finishes"
    );
    assert_eq!(
        crate::update::place::answer::cannot_tell(None, None, &no_back),
        "can't tell where the line came from"
    );
    assert_eq!(
        crate::update::clash::undo_clause_words(None, &no_back).as_deref(),
        Some("u puts it all back")
    );
}

/// W3 for `u`, `N` and `?`.
#[test]
fn an_unbound_undo_or_help_drops_its_clause() {
    let no_undo = unbinding("app.undo");
    assert_eq!(
        crate::update::clash::undo_clause_words(None, &no_undo),
        None
    );
    assert_eq!(crate::update::recover::undoes(&no_undo), None);
    assert_eq!(crate::update::switch::goes_back(&no_undo), None);
    assert_eq!(
        crate::update::preview::boxes::popped_line("stash@{0}", &no_undo),
        "stash@{0} is dropped"
    );
    assert_eq!(
        crate::update::undo::unreadable_words(&no_undo),
        "its state could not be read"
    );

    let no_prev = unbinding("app.prev_match");
    assert_eq!(
        crate::search::keys_words(false, 2, &no_prev),
        "  n next  esc close"
    );
    let no_help = unbinding("app.help");
    assert_eq!(
        crate::view::frame::bars::config_words(1, &no_help),
        "config: 1 problem"
    );
}

/// W3: no site draws `—` for an unbound key, whichever is unbound. A site
/// whose only key is unbound has no words (its fn answers `None`, which
/// `some` turns into a panic), so it is skipped here: nothing is drawn.
#[test]
fn no_unbound_key_draws_a_dash() {
    let ids = [
        "app.redo",
        "graph.journal",
        "app.back",
        "app.undo",
        "app.prev_match",
        "app.help",
        "app.next_pane",
        "app.quit",
        "journal.branch",
        "journal.revert",
    ];
    for map in &ids.map(unbinding) {
        for site in SITES {
            if let Ok(line) = std::panic::catch_unwind(|| (site.words)(map)) {
                assert!(!line.contains('\u{2014}'), "{}: {line}", site.name);
            }
        }
    }
}

/// m8/20b B2 (R5): `u`/`U` refused while a rebase stands, from the changes
/// pane's file list, name the two `⇥` that reach the banner (driven: two
/// presses focus it, one does not); from the graph, one.
#[test]
fn u_refused_from_the_changes_pane_names_two_tabs() {
    use super::resolve::{Spot, key, reaches_banner_in, sticky, stopped_at};
    use mergewright_core::event::KeyCode as CoreKey;
    for (spot, want, presses) in [
        (Spot::OtherRow, "⇥ ⇥ banner", 2),
        (Spot::Graph, "⇥ banner", 1),
    ] {
        for press in ['u', 'U'] {
            let (refused, _) = key(&stopped_at(spot), CoreKey::Char(press));
            assert_eq!(
                sticky(&refused),
                (
                    "finish or abort the rebase first".to_owned(),
                    Some(want.to_owned())
                ),
                "{spot:?} {press}"
            );
            assert!(
                reaches_banner_in(&refused, CoreKey::Tab, presses),
                "{spot:?} {press}"
            );
        }
    }
}

/// Review words-focus M2: on the focused banner `u`/`U` name no way to
/// the banner — he is on it, and `⇥` there would take him off it. The
/// reason stands alone.
#[test]
fn u_refused_on_the_focused_banner_names_no_way_to_it() {
    use super::resolve::{Spot, key, sticky, stopped_at};
    use mergewright_core::event::KeyCode as CoreKey;
    for press in ['u', 'U'] {
        let (refused, _) = key(&stopped_at(Spot::Banner), CoreKey::Char(press));
        assert_eq!(
            sticky(&refused),
            ("finish or abort the rebase first".to_owned(), None),
            "{press}"
        );
    }
}

/// The rebase stopped, the changes pane open, the keyboard on the graph.
fn graph_with_the_pane_open() -> crate::update::AppState {
    let mut state = super::resolve::stopped_at(super::resolve::Spot::ConflictRow);
    state.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
    assert!(state.detail.shown && !state.banner_focus);
    state
}

fn u_keys(state: &crate::update::AppState) -> Option<String> {
    use mergewright_core::event::KeyCode as CoreKey;
    let (refused, _) = super::resolve::key(state, CoreKey::Char('u'));
    let (reason, keys) = super::resolve::sticky(&refused);
    assert_eq!(reason, "finish or abort the rebase first");
    keys
}

/// m8/20d R5': graph focused, pane open, `u` names the shorter way —
/// one `⇧⇥` (driven: one press focuses the banner), not three `⇥`.
#[test]
fn from_the_graph_with_the_pane_open_the_clause_takes_the_shorter_way() {
    use mergewright_core::event::KeyCode as CoreKey;
    let state = graph_with_the_pane_open();
    assert_eq!(u_keys(&state).as_deref(), Some("⇧⇥ banner"));
    assert!(super::resolve::reaches_banner_in(
        &state,
        CoreKey::BackTab,
        1
    ));
}

/// m8/20d: from the file list both ways take two presses; a tie goes
/// forward.
#[test]
fn from_the_file_list_a_tie_goes_forward() {
    use mergewright_core::event::KeyCode as CoreKey;
    let state = super::resolve::stopped_at(super::resolve::Spot::OtherRow);
    assert_eq!(u_keys(&state).as_deref(), Some("⇥ ⇥ banner"));
    assert!(super::resolve::reaches_banner_in(
        &state,
        CoreKey::BackTab,
        2
    ));
    assert!(super::resolve::reaches_banner_in(&state, CoreKey::Tab, 2));
}

/// m8/20d: the way is spelled from his keymap. `prev_pane` unbound → the
/// forward way, however long; `next_pane` unbound → the back way; a moved (`f6`; `p` collides with pull, rewrite pick and stash pop)
/// `prev_pane` is named by its own key.
#[test]
fn the_way_is_spelled_from_his_keymap() {
    use mergewright_ui::keymap::KeySwitches;
    let graph = graph_with_the_pane_open();
    let with = |overrides: &[(&str, &str)]| {
        let (map, problems) = Keymap::with_overrides(
            &overrides
                .iter()
                .map(|(id, key)| ((*id).to_owned(), vec![(*key).to_owned()]))
                .collect::<Vec<_>>(),
            KeySwitches::default(),
        );
        assert_eq!(problems, [], "{overrides:?} reads cleanly");
        let mut state = graph.clone();
        state.keymap = std::sync::Arc::new(map);
        state
    };
    assert_eq!(
        u_keys(&with(&[("app.prev_pane", "none")])).as_deref(),
        Some("⇥ ⇥ ⇥ banner")
    );
    assert_eq!(
        u_keys(&with(&[("app.next_pane", "none")])).as_deref(),
        Some("⇧⇥ banner")
    );
    assert_eq!(
        u_keys(&with(&[("app.prev_pane", "f6")])).as_deref(),
        Some("F6 banner")
    );
}
