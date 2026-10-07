//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10i spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:update/preview/history.rs",
        "a stash isn't a branch · ⏎ opens it",
        "KNOWN: 10i",
    ),
    (
        "graft:update/preview/line_rows.rs",
        " · R to keep them",
        "KNOWN: 10i",
    ),
    (
        "graft:update/preview/line_rows.rs",
        "on origin: {} · P will need force after",
        "KNOWN: 10i",
    ),
    (
        "graft:update/preview/take.rs",
        "o takes a commit · this row isn't one",
        "KNOWN: 10i",
    ),
    (
        "graft:update/preview/take.rs",
        "o is waiting for the refs",
        "KNOWN: 10i",
    ),
    (
        "graft:update/remote/push_box.rs",
        "f to fetch first — nothing to lease against",
        "KNOWN: 10i",
    ),
    (
        "graft:update/remote.rs",
        "S s stashes them first",
        "KNOWN: 10i",
    ),
    (
        "graft:update/remote.rs",
        "m merge it  r rebase onto it",
        "KNOWN: 10i",
    ),
    ("graft:update/remote.rs", " · esc cancels", "KNOWN: 10i"),
    (
        "graft:update/rewrite_editor/open.rs",
        "R opens it",
        "KNOWN: 10i",
    ),
    (
        "graft:update/rewrite_editor/open.rs",
        "r then R opens it",
        "KNOWN: 10i",
    ),
    (
        "graft:update/rewrite_editor/words.rs",
        "nothing kept yet · p keeps a commit",
        "KNOWN: 10i",
    ),
    (
        "graft:update/rewrite_editor/words.rs",
        "this row melds into \\\"{kept_subject}\\\" · r there changes the message",
        "KNOWN: 10i",
    ),
    (
        "graft:view/rewrite_editor.rs",
        "already in {base} · p keeps it",
        "KNOWN: 10i",
    ),
    (
        "graft:view/rewrite_editor.rs",
        "MESSAGE of the {what} commit · r on \\\"{subject}\\\" changes it",
        "KNOWN: 10i",
    ),
];
