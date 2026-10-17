//! §2.6's conflict-resolver rows, in their place in [`super::defaults`].

use super::{Binding, ENTER, LEFT, RIGHT, Resolver, b, ctrl, k};

pub(super) const ROWS: &[Binding] = &[
    // §2.6 Conflict resolver.
    b("resolver.take_ours", Resolver, &[k('o')], "Take ours"),
    b("resolver.take_theirs", Resolver, &[k('t')], "Take theirs"),
    b("resolver.take_both", Resolver, &[k('b')], "Take both"),
    b(
        "resolver.all_ours",
        Resolver,
        &[ctrl('o')],
        "Ours in every block",
    ),
    b(
        "resolver.all_theirs",
        Resolver,
        &[ctrl('t')],
        "Theirs in every block",
    ),
    b(
        "resolver.file_ours",
        Resolver,
        &[k('O')],
        "Ours for the whole file, done",
    ),
    b(
        "resolver.file_theirs",
        Resolver,
        &[k('T')],
        "Theirs for the whole file, done",
    ),
    b("resolver.base", Resolver, &[k('B')], "Show / hide BASE"),
    b(
        "resolver.start_over",
        Resolver,
        &[k('d')],
        "Start this file over",
    ),
    b("resolver.next_block", Resolver, &[k('n')], "Next block"),
    b("resolver.prev_block", Resolver, &[k('N')], "Previous block"),
    // §2.2's movement keys, in this file only (ticket ease-pass/05r): the
    // arrows first, so the bar and the words spell `←/→` (Q19 a).
    b(
        "resolver.step_prev",
        Resolver,
        &[LEFT, k('h')],
        "Previous clash in this file, taken ones included",
    ),
    b(
        "resolver.step_next",
        Resolver,
        &[RIGHT, k('l')],
        "Next clash in this file, taken ones included",
    ),
    b("resolver.finish", Resolver, &[k('c')], "Finish"),
    b(
        "resolver.mark_done",
        Resolver,
        &[ENTER],
        "Mark this file done",
    ),
    b(
        "resolver.edit",
        Resolver,
        &[k('e')],
        "Open $EDITOR at this block",
    ),
    b(
        "resolver.mergetool",
        Resolver,
        &[k('M')],
        "Launch the mergetool",
    ),
];
