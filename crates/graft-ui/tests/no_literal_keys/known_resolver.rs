//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10g spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:update/resolver/answers/taken.rs",
        " · ⏎ marks {} done",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/answers.rs",
        "no conflict markers · ⏎ marks it resolved",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/answers.rs",
        "e edit  M mergetool",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/done.rs",
        "{path} is done · d starts it over",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/open.rs",
        "esc closes it",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/open.rs",
        "esc stops it",
        "KNOWN: 10g",
    ),
    ("graft:update/resolver/resolved.rs", "n next", "KNOWN: 10g"),
    (
        "graft:update/resolver/resolved.rs",
        "{reason} · ⏎ marks done",
        "KNOWN: 10g",
    ),
    ("graft:update/resolver/take.rs", "n next", "KNOWN: 10g"),
    (
        "graft:update/resolver/take.rs",
        "all blocks taken · ⏎ marks {} done",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o keeps {ours}  t keeps {theirs}",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o keeps the file  t deletes it",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o deletes it  t keeps {theirs}",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o keeps {ours}  t drops it",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o drops it  t keeps {theirs}",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o or t removes it",
        "KNOWN: 10g",
    ),
    (
        "graft:update/resolver/whole_file.rs",
        "o keeps {ours}  t keeps {theirs}",
        "KNOWN: 10g",
    ),
    (
        "graft:view/resolver/whole_file.rs",
        "d starts it over",
        "KNOWN: 10g",
    ),
    (
        "graft:view/resolver/whole_file.rs",
        "… first {} lines of {} · e opens the whole file",
        "KNOWN: 10g",
    ),
];
