//! Preview, remote and rewrite-editor words (m8/10i): each site's keys
//! spelled from his keymap, byte-identical under the shipped one (10b W6).
//!
//! Sites reached through one helper stand for all their callers:
//! `CannotStop::words` (`remote::in_flight` `Esc` on any unstoppable write),
//! `esc_hint` (`busy_text`, the progress line, undo's busy line, and the
//! footer's `ends_with_esc_hint`), `diverged_keys` (`remote_failed` and
//! `forget_stale_divergence`'s mark).

use mergewright_ui::keymap::{ActionId, ContextId, KeyChord, KeyCode, Keymap};

use super::{swept, unbinding, whole_tokens};

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
        name: "preview::CannotStop::History",
        keys: &["operation.abort"],
        words: |k| crate::update::preview::HISTORY_CANNOT_STOP.words(k),
        expected: |k| {
            format!(
                "runs to the end \u{b7} a conflict stops it on the banner, where {} aborts",
                k("operation.abort")
            )
        },
    },
    Site {
        name: "preview::CannotStop::PutBack",
        keys: &["app.undo"],
        words: |k| crate::update::preview::PUT_BACK_CANNOT_STOP.words(k),
        expected: |k| format!("runs to the end \u{b7} {} undoes it after", k("app.undo")),
    },
    Site {
        name: "preview::not_loaded",
        keys: &["app.back"],
        words: |k| crate::update::preview::not_loaded("abc1234", k),
        expected: |k| {
            format!(
                "preview cannot be drawn here: abc1234 is not loaded \u{b7} {}",
                k("app.back")
            )
        },
    },
    Site {
        name: "preview::stash_isnt_a_branch",
        keys: &["app.open"],
        words: crate::update::preview::stash_isnt_a_branch,
        expected: |k| format!("a stash isn't a branch \u{b7} {} opens it", k("app.open")),
    },
    Site {
        name: "preview::keep_them (already-in row)",
        keys: &["preview.rewrite"],
        words: |k| some(crate::update::preview::keep_them(k)),
        expected: |k| format!("{} to keep them", k("preview.rewrite")),
    },
    Site {
        name: "preview::published_words",
        keys: &["graph.push"],
        words: |k| crate::update::preview::published_words("feature-a", k),
        expected: |k| {
            format!(
                "on origin: feature-a \u{b7} {} will need force after",
                k("graph.push")
            )
        },
    },
    Site {
        name: "preview::not_a_commit_words (take)",
        keys: &["graph.take"],
        words: crate::update::preview::not_a_commit_words,
        expected: |k| {
            format!(
                "{} takes a commit \u{b7} this row isn't one",
                k("graph.take")
            )
        },
    },
    Site {
        name: "preview::waiting_for_refs (take)",
        keys: &["graph.take"],
        words: crate::update::preview::waiting_for_refs,
        expected: |k| format!("{} is waiting for the refs", k("graph.take")),
    },
    Site {
        name: "remote::nothing_to_lease (push box)",
        keys: &["graph.fetch"],
        words: crate::update::remote::nothing_to_lease,
        expected: |k| {
            format!(
                "{} to fetch first \u{2014} nothing to lease against",
                k("graph.fetch")
            )
        },
    },
    Site {
        name: "remote::overwrite_keys",
        keys: &["graph.stash", "stash.save"],
        words: |k| some(crate::update::remote::overwrite_keys(k)),
        expected: |k| {
            format!(
                "{} {} stashes them first",
                k("graph.stash"),
                k("stash.save")
            )
        },
    },
    Site {
        name: "remote::diverged_keys",
        keys: &["graph.merge", "graph.rebase"],
        words: |k| some(crate::update::remote::diverged_keys(k)),
        expected: |k| {
            format!(
                "{} merge it  {} rebase onto it",
                k("graph.merge"),
                k("graph.rebase")
            )
        },
    },
    Site {
        name: "remote::esc_hint",
        keys: &["app.back"],
        words: crate::update::remote::esc_hint,
        expected: |k| format!(" \u{b7} {} cancels", k("app.back")),
    },
    Site {
        name: "rewrite_editor::opens_it (row)",
        keys: &["graph.rewrite"],
        words: |k| some(crate::update::rewrite_editor::opens_it(false, k)),
        expected: |k| format!("{} opens it", k("graph.rewrite")),
    },
    Site {
        name: "rewrite_editor::opens_it (rebase)",
        keys: &["graph.rebase", "preview.rewrite"],
        words: |k| some(crate::update::rewrite_editor::opens_it(true, k)),
        expected: |k| {
            format!(
                "{} then {} opens it",
                k("graph.rebase"),
                k("preview.rewrite")
            )
        },
    },
    Site {
        name: "rewrite_editor::nothing_kept_yet",
        keys: &["rewrite.pick"],
        words: crate::update::rewrite_editor::nothing_kept_yet,
        expected: |k| {
            format!(
                "nothing kept yet \u{b7} {} keeps a commit",
                k("rewrite.pick")
            )
        },
    },
    Site {
        name: "rewrite_editor::melds_into",
        keys: &["rewrite.reword"],
        words: |k| crate::update::rewrite_editor::melds_into("Add OAuth", k),
        expected: |k| {
            format!(
                "this row melds into \"Add OAuth\" \u{b7} {} there changes the message",
                k("rewrite.reword")
            )
        },
    },
    Site {
        name: "view::rewrite_editor::title",
        keys: &["confirm.yes"],
        words: |k| crate::view::rewrite_editor::title("feature", 3, "main", k),
        expected: |k| {
            format!(
                "REWRITE feature \u{b7} 3 commits since it left main \u{b7} newest first \u{b7} \
                 nothing written until {}",
                k("confirm.yes")
            )
        },
    },
    Site {
        name: "view::rewrite_editor::already_in_note",
        keys: &["rewrite.pick"],
        words: |k| crate::view::rewrite_editor::already_in_note("main", k),
        expected: |k| format!("already in main \u{b7} {} keeps it", k("rewrite.pick")),
    },
    Site {
        name: "view::rewrite_editor::message_label",
        keys: &["rewrite.reword"],
        words: |k| crate::view::rewrite_editor::message_label("squashed", "Add OAuth", k),
        expected: |k| {
            format!(
                "MESSAGE of the squashed commit \u{b7} {} on \"Add OAuth\" changes it",
                k("rewrite.reword")
            )
        },
    },
];

/// The label each key carried before 10i.
fn letter(id: &str) -> &'static str {
    match id {
        "operation.abort" => "a",
        "app.undo" => "u",
        "app.back" => "esc",
        "app.open" => "\u{23ce}",
        "preview.rewrite" | "graph.rewrite" => "R",
        "graph.push" => "P",
        "graph.take" => "o",
        "graph.fetch" => "f",
        "graph.stash" => "S",
        "stash.save" => "s",
        "graph.merge" => "m",
        "graph.rebase" | "rewrite.reword" => "r",
        "rewrite.pick" => "p",
        "confirm.yes" => "y",
        other => panic!("no letter for {other}"),
    }
}

fn spelled(keymap: &Keymap, id: &'static str) -> String {
    keymap
        .spell_keys(&[ActionId(id)])
        .unwrap_or_else(|| panic!("{id} is bound"))
}

/// W1 + W6: shipped → the pre-10i literal byte for byte; swept → each key
/// its new label, no default letter left standing alone.
#[test]
fn each_word_spells_the_remapped_key() {
    assert_eq!(SITES.len(), 19, "every 10i site reachable as a function");
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

/// W2: each key's row is the one his press answers where the words show
/// or send him. Contexts: `a aborts` → Banner (a conflict stops there);
/// `u undoes it after`, `⏎ opens it` (the stash row), `P`, `o`, `f`, `S`,
/// `m`/`r` (the diverged line), `esc cancels` (a running write, focus on
/// the graph) and `R opens it` → Graph; `S` also from Changes (the
/// overwrite line shows there too); `s` → Stash (the box `S` opens);
/// `esc` on the undrawable box and `R to keep them` / `r then R`'s `R` →
/// Preview; `p`, `r`, `y` → Rewrite (the editor's rows; the `r` field's
/// own words spell the shipped keys, W4).
#[test]
fn each_word_names_the_row_that_answers_it() {
    const PINS: &[(ContextId, KeyChord, &str)] = &[
        (ContextId::Banner, KeyChord::key('a'), "operation.abort"),
        (ContextId::Graph, KeyChord::key('u'), "app.undo"),
        (
            ContextId::Preview,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (
            ContextId::Graph,
            KeyChord::plain(KeyCode::Enter),
            "app.open",
        ),
        (ContextId::Preview, KeyChord::key('R'), "preview.rewrite"),
        (ContextId::Graph, KeyChord::key('P'), "graph.push"),
        (ContextId::Graph, KeyChord::key('o'), "graph.take"),
        (ContextId::Graph, KeyChord::key('f'), "graph.fetch"),
        (ContextId::Graph, KeyChord::key('S'), "graph.stash"),
        (ContextId::Changes, KeyChord::key('S'), "graph.stash"),
        (ContextId::Stash, KeyChord::key('s'), "stash.save"),
        (ContextId::Graph, KeyChord::key('m'), "graph.merge"),
        (ContextId::Graph, KeyChord::key('r'), "graph.rebase"),
        (
            ContextId::Graph,
            KeyChord::plain(KeyCode::Escape),
            "app.back",
        ),
        (ContextId::Graph, KeyChord::key('R'), "graph.rewrite"),
        (ContextId::Rewrite, KeyChord::key('p'), "rewrite.pick"),
        (ContextId::Rewrite, KeyChord::key('r'), "rewrite.reword"),
        (ContextId::Rewrite, KeyChord::key('y'), "confirm.yes"),
    ];
    for (ctx, chord, id) in PINS {
        assert_eq!(
            Keymap::shipped().acting(*ctx, *chord),
            Some(ActionId(id)),
            "{id} in {ctx:?}"
        );
    }
}

/// W3: an unbound key drops its clause whole; never `—` for a key.
#[test]
fn an_unbound_key_drops_its_clause() {
    let no_merge = unbinding("graph.merge");
    assert_eq!(
        crate::update::remote::diverged_keys(&no_merge).as_deref(),
        Some("r rebase onto it")
    );
    let neither = Keymap::with_overrides(
        &[
            ("graph.merge".to_owned(), vec!["none".to_owned()]),
            ("graph.rebase".to_owned(), vec!["none".to_owned()]),
        ],
        mergewright_ui::keymap::KeySwitches::default(),
    )
    .0;
    assert_eq!(crate::update::remote::diverged_keys(&neither), None);

    let no_save = unbinding("stash.save");
    assert_eq!(crate::update::remote::overwrite_keys(&no_save), None);

    let no_back = unbinding("app.back");
    assert_eq!(crate::update::remote::esc_hint(&no_back), "");
    assert!(!crate::update::remote::ends_with_esc_hint(
        "push origin",
        &no_back
    ));
    assert_eq!(
        crate::update::preview::not_loaded("abc1234", &no_back),
        "preview cannot be drawn here: abc1234 is not loaded"
    );

    let no_abort = unbinding("operation.abort");
    assert_eq!(
        crate::update::preview::HISTORY_CANNOT_STOP.words(&no_abort),
        "runs to the end \u{b7} a conflict stops it on the banner"
    );
    assert_eq!(
        crate::update::preview::PUT_BACK_CANNOT_STOP.words(&unbinding("app.undo")),
        "runs to the end"
    );
    assert_eq!(
        crate::update::preview::stash_isnt_a_branch(&unbinding("app.open")),
        "a stash isn't a branch"
    );
    assert_eq!(
        crate::update::preview::keep_them(&unbinding("preview.rewrite")),
        None
    );
    assert_eq!(
        crate::update::preview::published_words("feature-a", &unbinding("graph.push")),
        "on origin: feature-a"
    );
}

/// W3 for the take, fetch and rewrite-editor keys.
#[test]
fn an_unbound_take_fetch_or_rewrite_key_drops_its_clause() {
    let no_take = unbinding("graph.take");
    assert_eq!(
        crate::update::preview::not_a_commit_words(&no_take),
        "this row isn't one"
    );
    assert_eq!(
        crate::update::preview::waiting_for_refs(&no_take),
        "waiting for the refs"
    );
    assert_eq!(
        crate::update::remote::nothing_to_lease(&unbinding("graph.fetch")),
        "nothing to lease against"
    );
    assert_eq!(
        crate::update::rewrite_editor::opens_it(false, &unbinding("graph.rewrite")),
        None
    );
    assert_eq!(
        crate::update::rewrite_editor::opens_it(true, &unbinding("preview.rewrite")),
        None
    );
    let no_pick = unbinding("rewrite.pick");
    assert_eq!(
        crate::update::rewrite_editor::nothing_kept_yet(&no_pick),
        "nothing kept yet"
    );
    assert_eq!(
        crate::view::rewrite_editor::already_in_note("main", &no_pick),
        "already in main"
    );
    let no_reword = unbinding("rewrite.reword");
    assert_eq!(
        crate::update::rewrite_editor::melds_into("Add OAuth", &no_reword),
        "this row melds into \"Add OAuth\""
    );
    assert_eq!(
        crate::view::rewrite_editor::message_label("squashed", "Add OAuth", &no_reword),
        "MESSAGE of the squashed commit"
    );
    assert_eq!(
        crate::view::rewrite_editor::title("feature", 1, "main", &unbinding("confirm.yes")),
        "REWRITE feature \u{b7} 1 commit since it left main \u{b7} newest first"
    );
}

/// W3: no site draws `—` for an unbound key, whichever is unbound. The
/// push box's `—` is prose (`f to fetch first — nothing to lease
/// against`), so that site checks no `—` *starts* the line.
#[test]
fn no_unbound_key_draws_a_dash() {
    let ids: Vec<&'static str> = SITES.iter().flat_map(|site| site.keys).copied().collect();
    for id in ids {
        let map = unbinding(id);
        for site in SITES {
            if let Ok(line) = std::panic::catch_unwind(|| (site.words)(&map)) {
                let prose = (site.expected)(&|_| "x".to_owned())
                    .matches('\u{2014}')
                    .count();
                assert!(
                    line.matches('\u{2014}').count() <= prose
                        && !line.trim_start().starts_with('\u{2014}'),
                    "{}: {line}",
                    site.name
                );
            }
        }
    }
}
