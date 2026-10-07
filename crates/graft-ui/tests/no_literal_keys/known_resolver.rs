//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10g spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.
//!
//! Left by 10g (ticket NOTES): `e edit  M mergetool` — a W2 stop, its keys
//! land outside the resolver where neither answers; `{reason} · ⏎ marks
//! done` — its callers are in 10f's and 10h's files.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:update/resolver/answers.rs",
        "e edit  M mergetool",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/resolved.rs",
        "{reason} · ⏎ marks done",
        "KNOWN: 10g",
    ),
];
