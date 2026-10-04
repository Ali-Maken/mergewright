//! The resolver's tables and the finish summary's: the bar over a file
//! still unmerged, a whole-file card's, a done file's, and the summary's
//! keys by focus. Split from `lists` so each file stays readable; the
//! priority table itself stays there.

use super::super::keys::{
    ALL_SIDES, BACK, BASE, BOTH, DOWN_UP, FIELD_COMMIT, FIELD_EDITOR, FIELD_ENTER, FIELD_ESC,
    FIELD_TAB, FILE_SIDES, FINISH_COMMIT, MARK_DONE, NEXT_BLOCK, NEXT_PANE, OURS, PREV_BLOCK,
    RESOLVER_EDIT, RESOLVER_FINISH, RESOLVER_MERGETOOL, START_OVER, THEIRS, TOGGLE_DIFF,
};
use super::super::{Hint, HintContext, hint};
use crate::conflict_resolver::SideRoles;
use crate::finish_summary::SummaryFocus;

/// The resolver's table for `context`: the done file's, or the bar over a
/// file still unmerged, each offering `c` while it would act.
pub(super) fn resolver_context_list(context: &HintContext) -> Vec<Hint> {
    let HintContext::Resolver {
        base,
        restart,
        whole_file,
        roles,
        done,
        finishable,
    } = *context
    else {
        // Only `context_list`'s `Resolver` arm calls this; no other
        // context has a resolver table to draw.
        return Vec::new();
    };
    let list = if done {
        resolver_done_list(restart)
    } else {
        resolver_bar(base, restart, whole_file, roles)
    };
    offering_c(list, roles, finishable)
}

/// A resolver table with `c` added **first** while `c` would act
/// (m6-conflicts/31, P8; first since m6-conflicts/60: it is the next action,
/// UX 1.4): `c finish` in a merge, which opens the summary, else
/// `c continue`. Unchanged while it would refuse.
fn offering_c(mut list: Vec<Hint>, roles: SideRoles, finishable: bool) -> Vec<Hint> {
    if finishable {
        let verb = if roles == SideRoles::Merge {
            "finish"
        } else {
            "continue"
        };
        list.insert(0, hint(RESOLVER_FINISH, verb));
    }
    list
}

/// The resolver's bar over a file still unmerged: a whole-file card or a
/// document.
fn resolver_bar(base: bool, restart: bool, whole_file: bool, roles: SideRoles) -> Vec<Hint> {
    if whole_file {
        resolver_card_list(roles)
    } else {
        resolver_list(base, restart, roles)
    }
}

/// The resolver's own key list (`docs/design-notes/m6-conflicts.md`, "New
/// UI" footer mockups): `o <ours>  t <theirs>  b both  [B base]  e edit
/// O/T file  n next  N prev  ⏎ done  j/k file  M mergetool  [d start over]  esc close`,
/// `B base` after `b both` only while `base` (Proposed change 2b), `d start
/// over` before `esc close` only while `restart` (Proposed change 2c).
/// `o`/`t` read as `roles` ([`SideRoles`]): `o upstream  t yours` in a
/// rebase, never git's own `ours`/`theirs`.
///
/// Live since m6-conflicts/04, which built `update::resolver` — every one
/// of these is answered by `resolver_key`, the two in brackets included
/// since m6-conflicts/05 bound them (`toggle_base`, `restart_box`).
fn resolver_list(base: bool, restart: bool, roles: SideRoles) -> Vec<Hint> {
    let mut list = vec![
        hint(OURS, roles.ours()),
        hint(THEIRS, roles.theirs()),
        hint(BOTH, "both"),
        // `Ctrl+O`/`Ctrl+T` as one hint (P3, user 2026-09-23; review
        // m6-amend-16 M2): `o`/`t` just before already name the roles.
        hint(ALL_SIDES, "all"),
        hint(FILE_SIDES, "file"),
    ];
    if base {
        list.push(hint(BASE, "base"));
    }
    list.extend([
        hint(RESOLVER_EDIT, "edit"),
        hint(NEXT_BLOCK, "next"),
        hint(PREV_BLOCK, "prev"),
        hint(MARK_DONE, "done"),
        hint(DOWN_UP, "file"),
        hint(RESOLVER_MERGETOOL, "mergetool"),
    ]);
    if restart {
        list.push(hint(START_OVER, "start over").keeps_words());
    }
    list.push(hint(BACK, "close").keeps_words());
    list
}

/// A whole-file card's own list (m6-conflicts/05, m6-05 review Major 1):
/// `o <ours>  t <theirs>  e edit  n next  M mergetool  esc close`, `o`/`t`
/// read as `roles` ([`SideRoles`]), as on [`resolver_list`].
///
/// Only the keys a card answers. `b` is refused in words (there is no
/// "both" of a path), `⏎` has nothing to mark — `o`/`t` already stage —
/// `B` has no blocks to show a base in, `d` has no markers to bring back,
/// and `N` has no previous file to walk to. `n` crosses to the next
/// conflicted file, as it does from a document's last block.
fn resolver_card_list(roles: SideRoles) -> Vec<Hint> {
    vec![
        hint(OURS, roles.ours()),
        hint(THEIRS, roles.theirs()),
        hint(RESOLVER_EDIT, "edit"),
        hint(NEXT_BLOCK, "next"),
        hint(RESOLVER_MERGETOOL, "mergetool"),
        hint(BACK, "close").keeps_words(),
    ]
}

/// A done file, read-only (m6-conflicts/25): `d start over  e edit  esc
/// close`, or `e edit  esc close` where ops cannot start it over
/// (m6-conflicts/28: a key shown must work). Every other resolver key is
/// refused there in words.
fn resolver_done_list(restart: bool) -> Vec<Hint> {
    let mut list = Vec::with_capacity(3);
    if restart {
        list.push(hint(START_OVER, "start over").keeps_words());
    }
    list.push(hint(RESOLVER_EDIT, "edit"));
    list.push(hint(BACK, "close").keeps_words());
    list
}

/// The finish summary's keys (keymap §2.6b), by focus (m6-conflicts/19),
/// drawn in the summary's own bottom border (m6-conflicts/59). `y` commits
/// from the files, the one confirm key of every preview (P12); `⏎` commits
/// from the subject; in the body it is a newline, so `⌃⏎` commits there.
/// `⇥` names where focus goes next. `⏎/space diff` (both the same toggle,
/// named like `^O/^T all`) is drawn only on a row git combined, the only rows whose
/// change he has not seen.
///
/// Bound by `graft`'s `update::finish::finish_key` (m6-conflicts/34, the
/// app half of ticket 19).
pub(super) fn finish_list(focus: SummaryFocus, expandable: bool) -> Vec<Hint> {
    match focus {
        SummaryFocus::Files if expandable => vec![
            hint(FINISH_COMMIT, "commit"),
            hint(TOGGLE_DIFF, "diff"),
            hint(DOWN_UP, "move"),
            hint(NEXT_PANE, "message"),
            hint(BACK, "back"),
        ],
        SummaryFocus::Files => vec![
            hint(FINISH_COMMIT, "commit"),
            hint(DOWN_UP, "move"),
            hint(NEXT_PANE, "message"),
            hint(BACK, "back"),
        ],
        SummaryFocus::Subject => vec![
            hint(FIELD_ENTER, "commit"),
            hint(FIELD_TAB, "body"),
            hint(FIELD_EDITOR, "$EDITOR"),
            hint(FIELD_ESC, "back"),
        ],
        SummaryFocus::Body => vec![
            hint(FIELD_COMMIT, "commit"),
            hint(FIELD_TAB, "files"),
            hint(FIELD_EDITOR, "$EDITOR"),
            hint(FIELD_ESC, "back"),
        ],
    }
}
