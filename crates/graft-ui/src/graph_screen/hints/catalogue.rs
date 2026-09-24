//! Every context the footer can be put in, and every hint it can draw —
//! the enumerations the app's key tests walk ([`every_context`],
//! [`every_hint`]).

use super::lists::{TAIL, context_list};
use super::{EXPLAIN, Hint, HintContext, MORE, RESIZE, WAY_BACK};
use crate::conflict_resolver::SideRoles;

/// **Every context the footer can be put in**, every field combination, each
/// row context also wrapped in [`HintContext::Marked`]. The one enumeration
/// every sweep reads — `graft-ui`'s own hint tests, [`every_hint`] and
/// `graft`'s key sweeps.
///
/// It exists because three hand-written copies of this list drifted:
/// [`HintContext::Dragging`] was added in m5/10 and only *one* of them
/// gained it, so the drag's bar went two tickets without a sweep over it.
///
/// **It cannot come from the type.** Rust has no way to enumerate a type's
/// values without a derive macro, and `Marked`/`PaneFocused` wrap a context
/// of their own, so the set is not even finite without choosing a depth by
/// hand. What the compiler can do is refuse, and three exhaustive matches
/// make it: [`marked_form`] and [`pane_focused_form`] here, and
/// `tests::coverage_slot`, which additionally spells out every **boolean
/// field combination**. A shape added to the enum stops this crate
/// compiling until it is named in all three, and
/// `tests::every_context_reaches_every_shape` then fails until the list
/// below actually produces one.
///
/// So adding a variant is still a human's edit; it is no longer a human's
/// *omission* (m5-history-and-ghost-preview/33, review M3, which corrected
/// an earlier claim that the compiler forced the list itself — it does
/// not).
#[must_use]
pub fn every_context() -> Vec<HintContext> {
    let plain = [
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
        HintContext::Banner {
            skippable: true,
            conflicted: true,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: true,
        },
        HintContext::Banner {
            skippable: true,
            conflicted: false,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: false,
        },
        HintContext::Preview,
        HintContext::ChangesPane { unmerged: false },
        HintContext::ChangesPane { unmerged: true },
        HintContext::Journal,
        HintContext::Stash,
        HintContext::Map,
        HintContext::PaneOverOverlay { opens: false },
        HintContext::PaneOverOverlay { opens: true },
        HintContext::Dragging,
        HintContext::ConfirmBox,
        HintContext::NamePrompt { toggle: true },
        HintContext::NamePrompt { toggle: false },
        HintContext::Credential,
    ];
    let mut all: Vec<HintContext> = plain.to_vec();
    all.extend(resolver_forms());
    all.extend(plain.iter().filter_map(marked_form));
    let rows: Vec<HintContext> = all.iter().flat_map(pane_focused_form).collect();
    all.extend(rows);
    let live: Vec<HintContext> = all.iter().filter_map(operation_live_form).collect();
    all.extend(live);
    all
}

/// `context` wrapped in [`HintContext::OperationLive`], for the contexts
/// `graft::view::rows::footer_context` can wrap: an unmodal graph-screen
/// bar — a cursor row, marked or not, or the changes pane. Not a pane
/// focused on a commit's diff: `⇥` from there lands on the banner too, but
/// `graft` does not wrap it (the pane's own list has no `⇥` to replace).
///
/// Exhaustive on purpose: see [`every_context`].
fn operation_live_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow
        | HintContext::Marked(_) => Some(HintContext::OperationLive(Box::new(context.clone()))),
        // Not the changes pane: it is one `Tab` from the banner only from
        // its diff, where its own bar's `space`/`d` are not answered.
        HintContext::ChangesPane { .. }
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::PaneFocused { .. }
        | HintContext::OperationLive(_) => None,
    }
}

/// Every [`HintContext::Resolver`] form [`every_context`] names: the four
/// `base`/`restart` combinations of a document, and a whole-file card
/// (whose list reads neither flag, so one form covers it) — each under
/// every [`SideRoles`].
fn resolver_forms() -> Vec<HintContext> {
    ALL_ROLES
        .iter()
        .flat_map(|&roles| {
            let document = move |base, restart| HintContext::Resolver {
                base,
                restart,
                whole_file: false,
                roles,
            };
            [
                document(false, false),
                document(true, false),
                document(false, true),
                document(true, true),
                HintContext::Resolver {
                    base: false,
                    restart: false,
                    whole_file: true,
                    roles,
                },
            ]
        })
        .collect()
}

/// Every [`SideRoles`], for [`resolver_forms`].
const ALL_ROLES: [SideRoles; 5] = [
    SideRoles::Merge,
    SideRoles::Rebase,
    SideRoles::CherryPick,
    SideRoles::Revert,
    SideRoles::StashPop,
];

/// `context` wrapped in [`HintContext::Marked`], for the contexts a Mistake
/// Radar marker can actually reach — the cursor's own row, once
/// `graft::view::rows::hint_context` has ruled out the working tree, the
/// changes pane and a drag in the air. Everything else has no cursor row to
/// carry a marker.
///
/// Exhaustive on purpose: see [`every_context`].
fn marked_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::BranchRow { .. } | HintContext::PlainRow => {
            Some(HintContext::Marked(Box::new(context.clone())))
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Marked(_)
        | HintContext::PaneFocused { .. }
        | HintContext::OperationLive(_) => None,
    }
}

/// `context` wrapped in [`HintContext::PaneFocused`], for the contexts
/// `graft::view::rows::hint_context` can reach with the pane focused and
/// `⏎` spent — a cursor row, marked or not.
///
/// Not the working-tree row: a pane open on *it* is the changes pane, so
/// the classifier answers `ChangesPane` long before it looks at the cursor
/// row. Not the changes pane or the banner either — neither list has an
/// `⏎` this could take, and wrapping a list to change nothing would be a
/// context nobody can tell from the one inside it.
///
/// Not the overlays: a pane focused with one of them behind it draws
/// neither the overlay's list nor a row's, because the graph is not there
/// to answer the row's keys. That is [`HintContext::PaneOverOverlay`],
/// which [`every_context`] names in its own right.
///
/// Exhaustive on purpose: see [`every_context`].
fn pane_focused_form(context: &HintContext) -> Vec<HintContext> {
    match context {
        HintContext::BranchRow { .. } | HintContext::PlainRow | HintContext::Marked(_) => {
            [true, false]
                .into_iter()
                .map(|opens| HintContext::PaneFocused {
                    row: Box::new(context.clone()),
                    opens,
                })
                .collect()
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::PaneFocused { .. }
        | HintContext::OperationLive(_) => Vec::new(),
    }
}

/// Every hint in every context's table, planned ones included, deduped by
/// `(key, verb, state)` — what `graft`'s guard tests walk so that neither
/// half of the table can drift from what the app binds.
#[must_use]
pub fn every_hint() -> Vec<Hint> {
    let mut all = Vec::new();
    for context in every_context() {
        all.extend(context_list(&context));
    }
    all.extend(TAIL);
    all.push(MORE);
    all.push(EXPLAIN);
    all.push(RESIZE);
    all.push(WAY_BACK);
    let mut seen = Vec::new();
    all.retain(|hint| {
        let fresh = !seen.contains(hint);
        if fresh {
            seen.push(*hint);
        }
        fresh
    });
    all
}
