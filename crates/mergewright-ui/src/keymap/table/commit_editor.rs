//! §2.10's commit-message-editor rows, in their place in
//! [`super::defaults`].

use super::{BACKTAB, Binding, CommitEditor, KeyChord, KeyCode, TAB, b, ctrl};

pub(super) const ROWS: &[Binding] = &[
    // §2.10 Commit message editor (a text field: fixed).
    b(
        "commit.commit",
        CommitEditor,
        &[KeyChord::ctrl_code(KeyCode::Enter)],
        "Commit",
    ),
    b("commit.next_field", CommitEditor, &[TAB], "Next field"),
    b(
        "commit.prev_field",
        CommitEditor,
        &[BACKTAB],
        "Previous field",
    ),
    b(
        "commit.edit_outside",
        CommitEditor,
        &[ctrl('e')],
        "Open $EDITOR instead",
    ),
    b(
        "commit.sign_off",
        CommitEditor,
        &[ctrl('s')],
        "Sign off: on / off",
    ),
    b(
        "commit.co_author",
        CommitEditor,
        &[ctrl('C')],
        "Insert a co-author trailer",
    ),
    // Ease pass (Q8 a): the options are off the commit screen, so no key;
    // they are listed from `⌃P`, or `?` off the text fields.
    b(
        "commit.toggle_amend",
        CommitEditor,
        &[],
        "Amend the last commit",
    ),
    b(
        "commit.toggle_no_verify",
        CommitEditor,
        &[],
        "Skip the commit hooks",
    ),
    b(
        "commit.toggle_allow_empty",
        CommitEditor,
        &[],
        "Allow an empty commit",
    ),
];
