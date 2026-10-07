//! `no_literal_keys` rows (ticket m8/10b). Literal keys ticket m8/10e spells from the keymap. Each row goes when its site is
//! fixed — a fixed literal left here fails the test as stale.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft-ops:apply/error/words.rs",
        "your edits are back{} · u undoes it, staging included",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:apply/error.rs",
        " · u restores the backup",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:apply/remote.rs",
        "push rejected: {} has commits you don't · p pull, then m or r · P force with lease",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:apply/remote.rs",
        "push refused: {} moved since your last fetch (stale lease) · f to fetch, then look",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "R rewrites your branch",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "a puts staged changes into a commit on your branch",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "HEAD has no commit yet · c makes the first one",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "force-push refused: {remote_side} is not where the box said \\\n                 (fetched since, or never fetched) · f to fetch, then look",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "needs a clean tree · S stashes {count} changed {} first",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "a {} has nothing to skip · c continues, a aborts",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "a {} keeps each commit's own message · c continues",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "nothing staged to put into {} · space stages",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "you're not on a branch · switch first (C)",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "none of your staged lines belong to a commit on your branch · c commits them",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "every commit on {branch} is already pushed · absorb only changes unpushed \\\n                 commits · c commits them",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "{branch} has no commits of its own yet · c commits them",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "{} is already pushed · a only changes older commits you haven't pushed",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:operation.rs",
        "S stashes {them} first",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:apply/error.rs",
        "{}: stopped · the {} is paused · c continues, a aborts",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:apply/error.rs",
        "{}: a git hook stopped it · c continues, a aborts",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:restore.rs",
        "undo cancelled part-way · {} · U restores the backup",
        "KNOWN: 10e",
    ),
    (
        "graft-ops:restore.rs",
        "{failure} · rolled back part-way · {} still changed: {} · U restores the backup",
        "KNOWN: 10e",
    ),
];
