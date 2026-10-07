//! Stop, outcome and failure words (m8/10f): each site's keys spelled from
//! his keymap through `graft_ui::keymap`, byte-identical under the shipped
//! one (10b W6).
//!
//! The banner's `c continue  a abort` is one helper,
//! `banner::continue_abort`: `focus_keys::nothing_conflicted`,
//! `pressed::{continued_in_git, land_after_continue_in_git, skip_pressed}`
//! and the journal's `in progress` row (`view::journal::summary`) all take
//! it, so its row here stands for all five.

use std::path::PathBuf;

use graft_git::state::InProgress;
use graft_ops::EditsAt;
use graft_ops::operation::{Operation, Refusal, Target};
use graft_ui::keymap::{ActionId, ContextId, KeyChord, KeyCode, Keymap};

use super::{swept, unbinding, whole_tokens};
use crate::update::commit_editor::{Begun, operation_refusal};
use crate::update::state::Status;

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
    assert_eq!(SITES.len(), 11, "every stop word reachable as a function");
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
