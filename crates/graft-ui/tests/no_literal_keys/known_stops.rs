//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10f spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:update/banner/focus_keys.rs",
        "c continue  a abort",
        "KNOWN: 10f",
    ),
    (
        "graft:update/banner/pressed.rs",
        "c continue  a abort",
        "KNOWN: 10f",
    ),
    (
        "graft:update/banner/pressed.rs",
        "c continue  a abort",
        "KNOWN: 10f",
    ),
    (
        "graft:update/banner/pressed.rs",
        "c continue  a abort",
        "KNOWN: 10f",
    ),
    (
        "graft:update/banner/pressed.rs",
        "u puts the merge back",
        "KNOWN: 10f",
    ),
    (
        "graft:update/banner/resume.rs",
        "e edits {}  a abort",
        "KNOWN: 10f",
    ),
    ("graft:update/banner/resume.rs", "a abort", "KNOWN: 10f"),
    (
        "graft:update/commit_editor/refusal.rs",
        "{to_banner}, then c finish  a abort",
        "KNOWN: 10f",
    ),
    (
        "graft:update/commit_editor/refusal.rs",
        "{to_banner}, then c continue  a abort",
        "KNOWN: 10f",
    ),
    (
        "graft:update/finish/keys.rs",
        "the message is empty · ⇥ to write one",
        "KNOWN: 10f",
    ),
    ("graft:update/mod.rs", "e edits {}  a abort", "KNOWN: 10f"),
    (
        "graft:update/operate/failure.rs",
        "your edits are in stash@{{{n}}} · S p puts them back",
        "KNOWN: 10f",
    ),
    (
        "graft:update/operate/failure.rs",
        "u restores the backup · a copy is in stash@{{{n}}}",
        "KNOWN: 10f",
    ),
    (
        "graft:update/operate/failure.rs",
        "u restores the backup",
        "KNOWN: 10f",
    ),
    (
        "graft:update/operate/failure.rs",
        "e edit  M mergetool",
        "KNOWN: 10f",
    ),
    (
        "graft:update/operate/failure.rs",
        "S stash {count} changed {}",
        "KNOWN: 10f",
    ),
    (
        "graft:update/operate/outcome/staging_not_kept.rs",
        "u undoes it, staging included",
        "KNOWN: 10f",
    ),
    (
        "graft:update/operate/outcome.rs",
        "c continues",
        "KNOWN: 10f",
    ),
    ("graft:update/operate/outcome.rs", "u undoes", "KNOWN: 10f"),
    (
        "graft:view/journal/summary.rs",
        "in progress — c continue  a abort",
        "KNOWN: 10f",
    ),
];
