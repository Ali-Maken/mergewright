//! The rows from §2.11's undo journal browser to the credential prompt, in
//! their place in [`super::defaults`] (moved out of `table.rs` for size).

use super::{
    BACKSPACE, Binding, Credential, DELETE, END, ENTER, ESC, EmptyState, HOME, History, Journal,
    LEFT, Palette, RIGHT, SPACE, Tour, b, ctrl, k,
};

pub(super) const ROWS: &[Binding] = &[
    // §2.11 Undo journal browser.
    b(
        "journal.revert",
        Journal,
        &[ENTER],
        "Undo / redo this row, else put back to before it (preview)",
    ),
    b(
        "journal.branch",
        Journal,
        &[k('b')],
        "New branch at this row's commit",
    ),
    b(
        "journal.expand",
        Journal,
        &[SPACE],
        "Expand / collapse the row",
    ),
    b("journal.close", Journal, &[k('J')], "Close the journal"),
    // §2.17 History.
    b("history.next_change", History, &[k('n')], "Next change"),
    b("history.prev_change", History, &[k('N')], "Previous change"),
    // §2.12 Command palette (a text field: fixed).
    b("palette.run", Palette, &[ENTER], "Run the row's action"),
    // §2.13 First-run / empty state.
    b("empty_state.open", EmptyState, &[k('o')], "Open…"),
    b("empty_state.init", EmptyState, &[k('i')], "Init"),
    b("empty_state.clone", EmptyState, &[k('c')], "Clone…"),
    b("empty_state.tour", EmptyState, &[k('t')], "Practice tour"),
    // §2.18 Practice tour: `Esc` leaves, only when nothing else on screen
    // takes it (`mergewright::update::keys::escape::escape_key`, its last link).
    b("tour.leave", Tour, &[ESC], "Leave the practice tour"),
    // §2.14 Credential prompt (a text field: fixed).
    b("credential.send", Credential, &[ENTER], "Submit the answer"),
    b(
        "credential.clear",
        Credential,
        &[ctrl('u')],
        "Clear the field",
    ),
    b(
        "credential.edit_text",
        Credential,
        &[BACKSPACE, DELETE, LEFT, RIGHT, HOME, END],
        "Edit the typed text",
    ),
];
