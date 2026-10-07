//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10h spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:search.rs",
        "  n/N next/prev  Esc cancel",
        "KNOWN: 10h",
    ),
    (
        "graft:search.rs",
        "  n/N next/prev  Esc close",
        "KNOWN: 10h",
    ),
    (
        "graft:update/amend_into/landing.rs",
        "u undoes",
        "KNOWN: 10h",
    ),
    ("graft:update/clash.rs", "u brings it back", "KNOWN: 10h"),
    (
        "graft:update/journal/answers.rs",
        "b makes {name} at {}",
        "KNOWN: 10h",
    ),
    (
        "graft:update/journal/keys.rs",
        "⏎ puts it back",
        "KNOWN: 10h",
    ),
    (
        "graft:update/journal/keys.rs",
        "b makes a branch on a $ or ✗ row",
        "KNOWN: 10h",
    ),
    (
        "graft:update/place/answer.rs",
        "can't tell where line {line} came from in {} · esc back",
        "KNOWN: 10h",
    ),
    (
        "graft:update/place/answer.rs",
        "can't tell where the line came from · esc back",
        "KNOWN: 10h",
    ),
    (
        "graft:update/preview/boxes.rs",
        "{name} is dropped · u brings it back",
        "KNOWN: 10h",
    ),
    (
        "graft:update/quit.rs",
        "{what} · quitting when it finishes · esc stays",
        "KNOWN: 10h",
    ),
    ("graft:update/recover.rs", "u undoes", "KNOWN: 10h"),
    ("graft:update/recover.rs", "u undoes it", "KNOWN: 10h"),
    ("graft:update/switch.rs", "u goes back", "KNOWN: 10h"),
    ("graft:update/undo.rs", "⇥ banner", "KNOWN: 10h"),
    ("graft:update/undo.rs", "U redoes {waiting}", "KNOWN: 10h"),
    ("graft:update/undo.rs", "J journal", "KNOWN: 10h"),
    ("graft:update/undo.rs", " · J to inspect", "KNOWN: 10h"),
    (
        "graft:view/credential.rs",
        "⏎ send  ⌃U clear  esc cancel",
        "KNOWN: 10h",
    ),
    (
        "graft:view/frame/bars.rs",
        "config: {count} {noun} · ? shows them",
        "KNOWN: 10h",
    ),
    ("graft:view/status.rs", "esc dismiss", "KNOWN: 10h"),
];
