//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10i spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale. Since 10i every site spells from the keymap; what is left is
//! prose its rewording put in a literal of its own (kept here, in 10i's file, not `prose.rs`).

pub const ROWS: &[(&str, &str, &str)] = &[(
    "mergewright:update/preview/history.rs",
    "a stash isn't a branch",
    "article",
)];
