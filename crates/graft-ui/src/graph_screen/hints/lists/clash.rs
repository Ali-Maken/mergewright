//! The clash screen's bars (ticket ease-pass/03b, design note M11, Q12 a):
//! one per [`ClashAt`], for a merge and for every other operation.
//!
//! Every key here acts today, in the resolver: `o`/`t`/`b`/`e` pick,
//! `u` is `app.undo`, `n` is `resolver.next_block`, `⏎` marks the file done,
//! `c` finishes or goes on, and `esc` (`app.back`) leaves the screen. Three
//! words run ahead of their handlers until P lands: `u undo pick`, `n next
//! open clash` and `esc pause, come back later` (the ticket names it).
//!
//! A words-only bar, as the mocks draw it: each hint has a short rung that
//! is a prefix of its words (`o take yours`, `u undo`, `esc pause`), so the
//! three most relevant still fit a 40-column bar.

use super::super::keys::{
    BACK, BOTH, MARK_DONE, NEXT_BLOCK, OURS, RESOLVER_EDIT, RESOLVER_FINISH, THEIRS, UNDO,
};
use super::super::{ClashAt, ClashBar, ClashOp, Hint, hint, named};
use crate::conflict_resolver::SideRoles;

/// The clash screen's bar for `bar`.
pub(super) fn clash_list(bar: &ClashBar) -> Vec<Hint> {
    let mut list = match &bar.at {
        ClashAt::Open => picks(bar.op),
        ClashAt::Picked => vec![named(NEXT_BLOCK, "next open clash", "next"), undo_pick()],
        ClashAt::FileSolved { next } => {
            let mut list = file_done(bar.op);
            list.extend([
                named(NEXT_BLOCK, format!("next file: {next}"), "next file"),
                undo_pick(),
            ]);
            list
        }
        ClashAt::AllSolved => {
            let mut list = file_done(bar.op);
            list.extend([hint(RESOLVER_FINISH, finish_word(bar.op)), undo_pick()]);
            list
        }
    };
    if !matches!(bar.at, ClashAt::Picked) {
        list.push(named(BACK, "pause, come back later", "pause"));
    }
    list
}

/// The picks on an open clash, sides named by the operation's roles: a
/// merge's `yours`/`theirs`, a rebase's `upstream`/`yours`.
fn picks(op: ClashOp) -> Vec<Hint> {
    let roles = match op {
        ClashOp::Merge => SideRoles::Merge,
        ClashOp::Other(roles) => roles,
    };
    let ((ours, ours_short), (theirs, theirs_short)) = take_words(roles);
    vec![
        named(OURS, ours, ours_short),
        named(THEIRS, theirs, theirs_short),
        named(BOTH, "take both", "take"),
        named(RESOLVER_EDIT, "edit by hand", "edit"),
        undo_pick(),
    ]
}

/// `o`'s and `t`'s words, whole and short, for `roles` — the role words
/// [`SideRoles::ours`] and [`SideRoles::theirs`] give, with the side of the
/// screen each is on.
fn take_words(roles: SideRoles) -> ((&'static str, &'static str), (&'static str, &'static str)) {
    let ours = match roles {
        SideRoles::Merge | SideRoles::CherryPick | SideRoles::Revert | SideRoles::StashPop => {
            ("take yours (left)", "take yours")
        }
        SideRoles::Rebase => ("take upstream (left)", "take upstream"),
        SideRoles::SetAside => ("take branch (left)", "take branch"),
    };
    let theirs = match roles {
        SideRoles::Merge => ("take theirs (right)", "take theirs"),
        SideRoles::Rebase | SideRoles::SetAside => ("take yours (right)", "take yours"),
        SideRoles::CherryPick => ("take picked (right)", "take picked"),
        SideRoles::Revert => ("take reverting (right)", "take reverting"),
        SideRoles::StashPop => ("take stashed (right)", "take stashed"),
    };
    (ours, theirs)
}

/// `u undo pick`.
fn undo_pick() -> Hint {
    named(UNDO, "undo pick", "undo")
}

/// Outside a merge a solved file is finished by `⏎`, today's word first
/// (Q12 a); a merge's files are done by their last pick.
fn file_done(op: ClashOp) -> Vec<Hint> {
    match op {
        ClashOp::Merge => Vec::new(),
        ClashOp::Other(_) => vec![hint(MARK_DONE, "done")],
    }
}

/// `c`'s word, as today's resolver says it: `finish` in a merge (P points
/// it at the review later), `continue` elsewhere.
fn finish_word(op: ClashOp) -> &'static str {
    match op {
        ClashOp::Merge => "finish",
        ClashOp::Other(_) => "continue",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words agree with the resolver's own role words for every role.
    #[test]
    fn the_take_words_name_the_roles_the_resolver_names() {
        for roles in [
            SideRoles::Merge,
            SideRoles::Rebase,
            SideRoles::CherryPick,
            SideRoles::Revert,
            SideRoles::StashPop,
            SideRoles::SetAside,
        ] {
            let ((ours, _), (theirs, _)) = take_words(roles);
            assert_eq!(ours, format!("take {} (left)", roles.ours()));
            assert_eq!(theirs, format!("take {} (right)", roles.theirs()));
        }
    }
}
