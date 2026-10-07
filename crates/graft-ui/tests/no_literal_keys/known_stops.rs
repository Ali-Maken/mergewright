//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10f spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.
//!
//! The two left are 10b W2 stops (m8/10f NOTES): `e edits {}` after a
//! mergetool is reached from three places whose `e` are three rows
//! (`operation.edit`, `resolver.edit`, `changes.edit`), so no one row
//! answers it. A planner ruling picks; they stay literal until then.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:update/operate/failure.rs",
        "a copy is in stash@{{{n}}}",
        "prose: the article, not the abort key (10f split it from `u restores the backup`)",
    ),
    (
        "graft:update/banner/resume.rs",
        "e edits {}  a abort",
        "KNOWN: 10f W2 stop — e is three rows (banner/focus_keys.rs:115, resolver/handover.rs:43, detail/keys.rs:234)",
    ),
    (
        "graft:update/mod.rs",
        "e edits {}  a abort",
        "KNOWN: 10f W2 stop — e is three rows (banner/focus_keys.rs:115, resolver/handover.rs:43, detail/keys.rs:234)",
    ),
];
