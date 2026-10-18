//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10f spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.
//!
//! The two 10b W2 stops (`e edits {}  a abort`) are spelled by place since
//! m8/10j (`banner::mergetool_fallback`).

pub const ROWS: &[(&str, &str, &str)] = &[(
    "graft:update/operate/failure.rs",
    "a copy is in stash@{{{n}}}",
    "prose: the article, not the abort key (10f split it from `u restores the backup`)",
)];
