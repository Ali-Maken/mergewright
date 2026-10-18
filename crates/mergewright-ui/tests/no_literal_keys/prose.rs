//! `no_literal_keys` rows (ticket m8/10b). graft and mergewright-ops literals that read like a hint but hold no key: an article, a count, a
//! description, or a message he never sees. Permanent rows.

pub const ROWS: &[(&str, &str, &str)] = &[
    (
        "graft:update/finish/words.rs",
        "delete the file",
        "the review's words for a file solved by deleting it, not the delete key",
    ),
    (
        "graft:view/tour.rs",
        "practice · {shown} · a throwaway repo, not yours",
        "article",
    ),
    (
        "graft:jobs/rewrite/words.rs",
        "a new file · no commit has it yet",
        "article",
    ),
    (
        "graft:jobs/rewrite/words.rs",
        "a renamed file stays whole",
        "article",
    ),
    (
        "graft:jobs/rewrite/words.rs",
        "a submodule change has no lines to put in",
        "article",
    ),
    (
        "graft:update/banner/pressed.rs",
        "a write is already running",
        "article",
    ),
    (
        "graft:update/confirm.rs",
        "a write is already running",
        "article",
    ),
    (
        "graft:update/finish/keys.rs",
        "a write is already running",
        "article",
    ),
    (
        "graft:update/journal/keys.rs",
        "a lost edit has no commit",
        "article",
    ),
    ("graft:update/journal/keys.rs", "a redo", "article"),
    (
        "graft:update/name_prompt.rs",
        "a tag needs a name",
        "article",
    ),
    (
        "graft:update/name_prompt.rs",
        "a branch needs a name",
        "article",
    ),
    (
        "graft:update/operate/failure.rs",
        "your edits are back · a copy stays in stash@{{{n}}}",
        "article",
    ),
    (
        "graft:update/operate/failure.rs",
        "your edits are back · a copy may still be in {stash}",
        "article (m8/10j: `S` now spelled from `graph.stash`)",
    ),
    (
        "graft:update/operate.rs",
        "a refused launch is worded: `Status::Failed(busy.words())` or the modal's own refusal",
        "not shown: debug",
    ),
    (
        "graft:update/operate.rs",
        "a refused launch is worded: `Status::Failed(busy.words())`",
        "not shown: debug",
    ),
    (
        "graft:update/preview/line_rows.rs",
        "{name} follows · 1 commit from {at}",
        "count",
    ),
    (
        "graft:update/preview/plan_lines.rs",
        "1 new commit",
        "count",
    ),
    ("graft:update/quit.rs", "a write", "article"),
    (
        "graft:update/recover.rs",
        "1 unsaved file set aside, put back after",
        "count",
    ),
    ("graft:update/remote/push_box.rs", "1 commit", "count"),
    (
        "graft:update/resolver/files.rs",
        "a write is already running",
        "article",
    ),
    // Moved out of `resolver.rs` (ticket ease-pass/05r), where it sat after
    // a `#[cfg(test)]` and so was never scanned.
    (
        "graft:update/resolver/gate.rs",
        "a write is already running",
        "article",
    ),
    (
        "graft:update/resolver/handover.rs",
        "a take is still landing · try again in a moment",
        "article",
    ),
    (
        "graft:update/resolver/handover.rs",
        "a path was passed, so `M` was answered",
        "not shown: assert",
    ),
    (
        "graft:update/resolver/open.rs",
        "a write is running",
        "article",
    ),
    ("graft:view/journal/summary.rs", "1 ref", "count"),
    ("graft:view/picker.rs", "1 branch", "count"),
    (
        "mergewright-ops:absorb/build.rs",
        "a staged hunk's patch: {error:?}",
        "article",
    ),
    (
        "mergewright-ops:absorb/read.rs",
        "a raw diff record with no path: {:?}",
        "article",
    ),
    (
        "mergewright-ops:apply/error.rs",
        "your edits are back · a copy stays in stash@{{{n}}}",
        "article",
    ),
    (
        "mergewright-ops:apply/error.rs",
        "your edits are back · a copy may still be in the stash list",
        "article",
    ),
    (
        "mergewright-ops:apply/error.rs",
        "the backup has your edits · a copy is in stash@{{{n}}}",
        "article",
    ),
    ("mergewright-ops:apply/record.rs", "delete branch", "description"),
    ("mergewright-ops:apply/record.rs", "delete tag", "description"),
    (
        "mergewright-ops:apply/set_aside/returned.rs",
        "a copy stays in stash@{{{at}}}",
        "article",
    ),
    (
        "mergewright-ops:finish.rs",
        "a rename with no old path: {}",
        "article",
    ),
    ("mergewright-ops:journal_list/words.rs", "1 commit", "count"),
    (
        "mergewright-ops:operation/explain.rs",
        "that's the newest commit · a plain amend puts the changes in",
        "article",
    ),
    (
        "mergewright-ops:operation/explain.rs",
        "a switch moves no branch · switch back to {back_to} instead",
        "article",
    ),
    (
        "mergewright-ops:rewrite/unpushed.rs",
        "a commit your_commits took has no parent",
        "article",
    ),
    (
        "mergewright-ops:submodule.rs",
        "a log record with no subject: {:?}",
        "article",
    ),
    (
        "mergewright-ops:submodule.rs",
        "a log record with no id: {:?}",
        "article",
    ),
    (
        "mergewright-ops:submodule.rs",
        "a log record on no side: {:?}",
        "article",
    ),
    ("mergewright-ops:undo.rs", "a merge is in progress now", "article"),
    (
        "graft:update/quit.rs",
        "{bin}: {what} finished · a git hook after it was stopped after {n} s",
        "article",
    ),
    (
        "graft:update/quit.rs",
        "{what} is still running · a git hook may be stuck",
        "article",
    ),
];
