//! Wording helpers and in-progress mapping.

use super::{InProgress, KeyNames, OpKey, OperationKind, PathBuf, RefName, clause};

/// One wording of a refusal: why, and what to do. `next` is `None` for every
/// variant but [`Refusal::InTheWay`] today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalWords {
    /// What stands in the way.
    pub reason: String,
    /// The next step (`move it aside first`), kept apart so the status line
    /// can keep it whole while the reason is shortened.
    pub next: Option<String>,
}

/// [`Refusal::InTheWay`]'s ladder. The first ignored path names the line
/// when there is one, since an ignored file decides the next step; `+N more`
/// counts every other path, ignored or not (the form of Recover's
/// `put back your edit to <path> +N more`). The last rung drops `(ignored)`.
/// A rung no shorter than the one before it is skipped.
pub(super) fn in_the_way_rungs(
    doing: &str,
    ignored: &[PathBuf],
    untracked: &[PathBuf],
    keys: &dyn KeyNames,
) -> Vec<RefusalWords> {
    let Some(first) = ignored.first().or(untracked.first()) else {
        return vec![RefusalWords {
            reason: format!("nothing is in the way of {doing}"),
            next: None,
        }];
    };
    let more = ignored.len() + untracked.len() - 1;
    let is_ignored = !ignored.is_empty();
    let tag = if is_ignored { " (ignored)" } else { "" };
    let (count, verb, them) = match more {
        0 => (String::new(), "is", "it"),
        more => (format!(" +{more} more"), "are", "them"),
    };
    // No `next` when the stash key is unbound (W3): the reason stands alone.
    let next = if is_ignored {
        Some(format!("move {them} aside first"))
    } else {
        clause(keys, OpKey::Stash, &format!("stashes {them} first"))
    };
    let components: Vec<String> = first
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let named = |path: &str| format!("{path}{tag}{count} {verb} in the way");
    let whole = first.display().to_string();
    let mut reasons = vec![format!("{} of {doing}", named(&whole)), named(&whole)];
    let mut shortest = whole;
    if let [top, .., name] = components.as_slice() {
        if components.len() >= 3 {
            reasons.push(named(&format!("{top}/…/{name}")));
        }
        shortest = format!("…/{name}");
        reasons.push(named(&shortest));
    }
    // Rung 5: `(ignored)` goes last of all. It says why git status never
    // warned him; the file and the next step are what he acts on. Untracked
    // rows have no tag, so this is no shorter and the rule below skips it.
    reasons.push(format!("{shortest}{count} {verb} in the way"));
    let mut rungs: Vec<RefusalWords> = Vec::new();
    for reason in reasons {
        let shorter = rungs
            .last()
            .is_none_or(|last| reason.chars().count() < last.reason.chars().count());
        if shorter {
            rungs.push(RefusalWords {
                reason,
                next: next.clone(),
            });
        }
    }
    rungs
}

/// `src/a.rs`, `src/a.rs and src/b.rs`, `src/a.rs and 2 more files`.
/// [`Refusal::StashInTheWay`] in words: each non-empty set as its own
/// clause, joined by `and`. `is` only for one file that is not his edit
/// (`new.txt is in the way`); every other shape is plural. An ignored file
/// reads as in [`Refusal::InTheWay`] (review 28 m4): the first ignored one
/// names the files, `(ignored) +N more`, and the next step is `move it
/// aside first`, since `S` can't take it.
pub(super) fn in_the_way_words(
    unsaved: &[PathBuf],
    staged: &[PathBuf],
    untracked: &[PathBuf],
    ignored: &[PathBuf],
) -> String {
    let mut clauses = Vec::new();
    if !unsaved.is_empty() {
        clauses.push(format!("your unsaved edits to {}", name_paths(unsaved)));
    }
    if !staged.is_empty() {
        clauses.push(format!("your staged edits to {}", name_paths(staged)));
    }
    let files = untracked.len() + ignored.len();
    match (ignored.first(), files) {
        (Some(first), 1) => clauses.push(format!("{} (ignored)", first.display())),
        (Some(first), files) => {
            clauses.push(format!("{} (ignored) +{} more", first.display(), files - 1));
        }
        (None, 0) => {}
        (None, _) => clauses.push(name_paths(untracked)),
    }
    let verb = if unsaved.is_empty() && staged.is_empty() && files == 1 {
        "is"
    } else {
        "are"
    };
    let sentence = format!("{} {verb} in the way", clauses.join(" and "));
    match (ignored.is_empty(), files) {
        (true, _) => sentence,
        (false, 1) => format!("{sentence} · move it aside first"),
        (false, _) => format!("{sentence} · move them aside first"),
    }
}

/// `c commits them`.
pub(super) fn commits_them(keys: &dyn KeyNames) -> Option<String> {
    clause(keys, OpKey::Commit, "commits them")
}

/// `words — <key clause>`, or `words` alone when the key is unbound.
pub(super) fn dashed(words: String, key: Option<String>) -> String {
    match key {
        Some(key) => format!("{words} — {key}"),
        None => words,
    }
}

/// A branch as the words name it: `feature-b`, not `refs/heads/feature-b`.
pub(super) fn ref_words(name: &RefName) -> String {
    String::from_utf8_lossy(name.shorthand()).into_owned()
}

pub(super) fn name_paths(paths: &[PathBuf]) -> String {
    match paths {
        [] => "no files".to_owned(),
        [one] => one.display().to_string(),
        [first, second] => format!("{} and {}", first.display(), second.display()),
        [first, rest @ ..] => format!("{} and {} more files", first.display(), rest.len()),
    }
}

/// The in-progress state an operation of this kind opens when git stops on
/// a conflict: what the executor matches `in_progress` against to tell
/// "stopped, still live" from "failed" (m5/07). `None` for every other kind.
///
/// One table, asked two ways: [`Operation::starts_in_progress`] for an
/// operation about to run, this for a journal entry already written — a
/// second copy is a second thing to keep in step (m5/07 review, M11).
#[must_use]
pub(crate) fn opens_in_progress(kind: OperationKind) -> Option<InProgress> {
    match kind {
        OperationKind::Merge => Some(InProgress::Merge),
        // A rewrite runs git's `rebase -i` (ADR-0047).
        OperationKind::Rebase | OperationKind::Rewrite => Some(InProgress::Rebase),
        OperationKind::CherryPick => Some(InProgress::CherryPick),
        OperationKind::Revert => Some(InProgress::Revert),
        _ => None,
    }
}

/// The in-progress state a **stop** of this kind is read against
/// (m8-polish-and-ship/13d Q4): the one it opens, or the one it runs only
/// inside — finishing or aborting a merge begun in the shell, which
/// `finish_merge::check` admits only while git is merging. Such a stop can
/// leave that merge paused, as it was before `c`.
///
/// Not [`opens_in_progress`], which recovery and the banner ask: an
/// operation that runs inside a merge does not own it, and its entry is
/// never held `Pending` for it — Graft did not begin that merge.
#[must_use]
pub(crate) fn runs_in_progress(kind: OperationKind) -> Option<InProgress> {
    match kind {
        OperationKind::FinishMerge | OperationKind::AbortMerge => Some(InProgress::Merge),
        other => opens_in_progress(other),
    }
}
