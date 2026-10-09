//! Every context the footer can be put in, and every hint it can draw —
//! the enumerations the app's key tests walk ([`every_context`],
//! [`every_hint`]).

use super::lists::{TAIL, context_list};
use super::{
    ACTIONS, BACK_TO_BLAME, BACK_TO_HISTORY, BLAME, ConfirmYes, DOORWAY, DOORWAY_BY_PALETTE,
    EXPLAIN, F_HISTORY, FOLD, Hint, HintContext, LiveC, RESIZE, RowVerbs, SEARCH_NEXT, SEARCH_PREV,
    TabWord, WAY_BACK, WAY_BACK_PREV, WayToBanner,
};
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
    let mut plain = vec![
        HintContext::WorkingTreeRow,
        HintContext::Preview,
        HintContext::Picker,
        HintContext::PaneOverOverlay { opens: false },
        HintContext::PaneOverOverlay { opens: true },
        HintContext::Dragging,
        HintContext::ConfirmBox { yes: None },
        HintContext::NamePrompt { toggle: true },
        HintContext::NamePrompt { toggle: false },
        HintContext::Credential,
        // m7-power-tools/08: `R rewrite` on his commit rows, and the editor's
        // field.
        HintContext::RewriteMessage,
        // m8-polish-and-ship/15 A4: a stash's row; `marked_form` and the
        // live and pane wraps reach it like any row.
        HintContext::StashRow,
    ];
    // Each commit-row shape once per `RowVerbs` (m7-power-tools/40), bare
    // and under `R rewrite`.
    for verbs in RowVerbs::ALL {
        let rows = [
            HintContext::BranchRow {
                current: false,
                verbs,
            },
            HintContext::BranchRow {
                current: true,
                verbs,
            },
            HintContext::PlainRow { verbs },
        ];
        for row in rows {
            plain.push(HintContext::Rewritable(Box::new(row.clone())));
            plain.push(row);
        }
    }
    let mut all: Vec<HintContext> = plain.clone();
    all.extend(rewrite_editor_forms());
    all.extend(banner_forms());
    all.extend(changes_pane_forms());
    all.extend(changes_pane_live_forms());
    all.extend(resolver_forms());
    all.extend(finish_forms());
    // m6-conflicts/50: a box `y` takes, once per acting button.
    all.extend(
        ConfirmYes::ALL
            .into_iter()
            .map(|yes| HintContext::ConfirmBox { yes: Some(yes) }),
    );
    all.extend(plain.iter().filter_map(marked_form));
    // m8/15b: a search left open after `⏎`, over every graph-row form,
    // marked or not; before the live wrap, which takes it inside.
    let searching: Vec<HintContext> = all.iter().filter_map(searching_form).collect();
    all.extend(searching);
    let rows: Vec<HintContext> = all.iter().flat_map(pane_focused_form).collect();
    all.extend(rows);
    let live: Vec<HintContext> = all.iter().flat_map(operation_live_form).collect();
    all.extend(live);
    all.extend(blame_forms());
    all.extend(history_forms());
    all
}

/// The rewrite editor's four flag pairs (m7-power-tools/08).
fn rewrite_editor_forms() -> Vec<HintContext> {
    [false, true]
        .into_iter()
        .flat_map(|narrow| {
            [false, true]
                .into_iter()
                .map(move |can_start| HintContext::RewriteEditor { narrow, can_start })
        })
        .collect()
}

/// The history place's three reachable forms and the way back to it over
/// one inner bar (m7-power-tools/21b). `History { rows: false, on_file:
/// true }` is not a form of its own: without rows the list is `esc back`
/// whatever the focus ([`super::lists::history_place_list`]), so it would
/// sweep the same bar twice — as [`pane_focused_form`] leaves out shapes
/// the app never makes. One `BackToHistory` is enough for
/// [`blame_forms`]' reason: it adds a lead and takes nothing away.
fn history_forms() -> Vec<HintContext> {
    vec![
        HintContext::History {
            rows: true,
            on_file: false,
        },
        HintContext::History {
            rows: true,
            on_file: true,
        },
        HintContext::History {
            rows: false,
            on_file: false,
        },
        HintContext::BackToHistory(Box::new(HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        })),
    ]
}

/// The blame place's eight flag triples, and the way back to it led on one
/// bar of each kind the graph or the pane draws while the trail waits
/// (m7-power-tools/20). One of each is enough: `BackToBlame` adds a lead
/// and takes nothing away, and every inner bar is swept on its own.
fn blame_forms() -> Vec<HintContext> {
    let mut forms: Vec<HintContext> = [false, true]
        .into_iter()
        .flat_map(|lines| {
            [false, true].into_iter().flat_map(move |finding| {
                [false, true]
                    .into_iter()
                    .map(move |ignore_file| HintContext::Blame {
                        finding,
                        ignore_file,
                        lines,
                    })
            })
        })
        .collect();
    let under = [
        HintContext::WorkingTreeRow,
        HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        HintContext::BranchRow {
            current: false,
            verbs: RowVerbs::NONE,
        },
        HintContext::PaneFocused {
            row: Box::new(HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            }),
            opens: false,
            blamable: true,
            folds: false,
            graph_hidden: false,
        },
        HintContext::PaneFocused {
            row: Box::new(HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            }),
            opens: true,
            blamable: false,
            folds: true,
            graph_hidden: false,
        },
        HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        },
    ];
    forms.extend(
        under
            .into_iter()
            .map(|inner| HintContext::BackToBlame(Box::new(inner))),
    );
    forms
}

/// Every [`HintContext::Banner`] form a live operation can put it in: a
/// merge is never skippable and every other kind is (m6-conflicts/70 added
/// `merge`), each with a conflict or without.
fn banner_forms() -> Vec<HintContext> {
    let mut forms = Vec::new();
    for merge in [false, true] {
        for conflicted in [true, false] {
            forms.push(HintContext::Banner {
                skippable: !merge,
                conflicted,
                merge,
            });
        }
    }
    forms
}

/// Every [`HintContext::ChangesPane`] form with nothing live: `⇥ diff` from
/// the file list, `⇥ graph` from the diff (no banner to go to), each over
/// `unmerged`, and each over an overlay or not (m8/09e: under the Map,
/// journal or stash list `live` is not read, so `None` stands for all).
fn changes_pane_forms() -> Vec<HintContext> {
    let mut forms = Vec::new();
    for over_overlay in [false, true] {
        for tab in [TabWord::Graph, TabWord::Diff] {
            for unmerged in [false, true] {
                forms.push(HintContext::ChangesPane {
                    unmerged,
                    live: None,
                    tab,
                    over_overlay,
                });
            }
        }
    }
    forms
}

/// Every live-operation form of [`HintContext::ChangesPane`]: each
/// [`LiveC`] over each `unmerged`, with each `⇥` word that `c` can stand
/// beside. `c finish`/`c continue` always have a banner, so from the diff
/// `⇥` names it; `Refuses` is a bisect (no banner, `⇥ graph`) or a
/// shell-begun rebase (a banner, `⇥ banner`).
fn changes_pane_live_forms() -> Vec<HintContext> {
    let mut forms = Vec::new();
    for c in LIVE_C {
        let tabs: &[TabWord] = match c {
            LiveC::Finish | LiveC::Continue | LiveC::ContinueAmends => {
                &[TabWord::Diff, TabWord::Banner]
            }
            LiveC::Refuses => &[TabWord::Diff, TabWord::Banner, TabWord::Graph],
        };
        for &tab in tabs {
            for unmerged in [false, true] {
                forms.push(HintContext::ChangesPane {
                    unmerged,
                    live: Some(c),
                    tab,
                    over_overlay: false,
                });
            }
        }
    }
    forms
}

/// Every [`LiveC`], for the forms that carry one.
const LIVE_C: [LiveC; 4] = [
    LiveC::Finish,
    LiveC::Continue,
    LiveC::ContinueAmends,
    LiveC::Refuses,
];

/// Every [`HintContext::Finish`] form: each focus, a git-combined row under
/// the cursor or not. Only the files focus reads `expandable`, so the other
/// two are named once.
fn finish_forms() -> Vec<HintContext> {
    use crate::finish_summary::SummaryFocus;
    vec![
        HintContext::Finish {
            focus: SummaryFocus::Files,
            expandable: true,
        },
        HintContext::Finish {
            focus: SummaryFocus::Files,
            expandable: false,
        },
        HintContext::Finish {
            focus: SummaryFocus::Subject,
            expandable: false,
        },
        HintContext::Finish {
            focus: SummaryFocus::Body,
            expandable: false,
        },
    ]
}

/// `context` wrapped in [`HintContext::OperationLive`] every way `graft`
/// can wrap it — each [`LiveC`], one `⇥` from the banner or not — for the
/// contexts `graft::view::rows::footer_context` wraps: a graph row, marked
/// or not, and a pane focused on a commit (its `c` passes through to the
/// graph's). Not the changes pane: it carries its own `live`
/// ([`changes_pane_live_forms`]).
///
/// Exhaustive on purpose: see [`every_context`].
fn operation_live_form(context: &HintContext) -> Vec<HintContext> {
    match context {
        // `R rewrite` is drawn only while nothing is live (08, ruling 10).
        HintContext::Marked(row) if matches!(**row, HintContext::Rewritable(_)) => Vec::new(),
        // A live operation hides `o` and `a` (graft `view::rows::live_wrapped`
        // passes `RowVerbs::NONE`): a row with either never reaches the bar.
        _ if has_row_verbs(context) => Vec::new(),
        // `R` is never live, so neither is a search over it.
        HintContext::Searching(row) if over_rewritable(row) => Vec::new(),
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::StashRow
        | HintContext::PlainRow { .. }
        | HintContext::Marked(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. } => {
            // `space stage` only at an `edit` stop with the graph focused
            // (07d D1): never for a pane with the keyboard.
            let pane = matches!(context, HintContext::PaneFocused { .. });
            LIVE_C
                .into_iter()
                .flat_map(|c| {
                    let stages: &[bool] = if c == LiveC::ContinueAmends && !pane {
                        &[false, true]
                    } else {
                        &[false]
                    };
                    stages.iter().flat_map(move |&space_stages| {
                        [Some(WayToBanner::Forward), Some(WayToBanner::Back), None]
                            .into_iter()
                            .map(move |way_to_banner| HintContext::OperationLive {
                                row: Box::new(context.clone()),
                                c,
                                way_to_banner,
                                space_stages,
                            })
                    })
                })
                .collect()
        }
        HintContext::ChangesPane { .. }
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::Picker
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::Rewritable(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => Vec::new(),
    }
}

/// Every [`HintContext::Resolver`] form [`every_context`] names: the four
/// `base`/`restart` combinations of a document, a whole-file card and a done
/// file (whose lists read neither flag, so one form covers each) — each under
/// every [`SideRoles`]. Each once more with `finishable` (m6-conflicts/31),
/// except under [`SideRoles::StashPop`] and [`SideRoles::SetAside`]: a
/// stash put back has no operation to finish, so no `c`.
fn resolver_forms() -> Vec<HintContext> {
    ALL_ROLES
        .iter()
        .flat_map(|&roles| {
            let finishables: &[bool] = if matches!(roles, SideRoles::StashPop | SideRoles::SetAside)
            {
                &[false]
            } else {
                &[false, true]
            };
            finishables.iter().flat_map(move |&finishable| {
                let form = move |base, restart, whole_file, done| HintContext::Resolver {
                    base,
                    restart,
                    whole_file,
                    done,
                    roles,
                    finishable,
                };
                [
                    // A document: the four `base`/`restart` combinations.
                    form(false, false, false, false),
                    form(true, false, false, false),
                    form(false, true, false, false),
                    form(true, true, false, false),
                    // A whole-file card.
                    form(false, false, true, false),
                    // A done file, read-only (m6-conflicts/25): its list
                    // reads `restart` alone (m6-conflicts/28), so two forms
                    // cover it.
                    form(false, true, false, true),
                    form(false, false, false, true),
                ]
            })
        })
        .collect()
}

/// Every [`SideRoles`], for [`resolver_forms`].
const ALL_ROLES: [SideRoles; 6] = [
    SideRoles::Merge,
    SideRoles::Rebase,
    SideRoles::CherryPick,
    SideRoles::Revert,
    SideRoles::StashPop,
    SideRoles::SetAside,
];

/// `context` is `R rewrite`'s row, marked or not.
fn over_rewritable(context: &HintContext) -> bool {
    match context {
        HintContext::Rewritable(_) => true,
        HintContext::Marked(row) => over_rewritable(row),
        _ => false,
    }
}

/// The cursor row inside `context` carries `o take` or `a amend`.
fn has_row_verbs(context: &HintContext) -> bool {
    match context {
        HintContext::BranchRow { verbs, .. } | HintContext::PlainRow { verbs } => {
            *verbs != RowVerbs::NONE
        }
        HintContext::Marked(row)
        | HintContext::Rewritable(row)
        | HintContext::Searching(row)
        | HintContext::PaneFocused { row, .. } => has_row_verbs(row),
        _ => false,
    }
}

/// `context` wrapped in [`HintContext::Marked`], for the contexts a Mistake
/// Radar marker can actually reach — the cursor's own row, once
/// `graft::view::rows::hint_context` has ruled out the working tree, the
/// changes pane and a drag in the air. Everything else has no cursor row to
/// carry a marker.
///
/// Exhaustive on purpose: see [`every_context`].
fn marked_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_) => Some(HintContext::Marked(Box::new(context.clone()))),
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Picker
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::Marked(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. }
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => None,
    }
}

/// `context` wrapped in [`HintContext::Searching`] (m8/15b), for the
/// graph-row forms `graft::view::rows::footer_context` wraps while a search
/// stands with its prompt closed: a row, marked or under `R`. Not a pane's
/// bar, an overlay's or a box's: the search's keys are the graph's.
///
/// Exhaustive on purpose: see [`every_context`].
fn searching_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::Marked(_) => Some(HintContext::Searching(Box::new(context.clone()))),
        HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Picker
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. }
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => None,
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
        // `R` needs the graph's keyboard (08, ruling 10): never under a pane.
        HintContext::Marked(row) if matches!(**row, HintContext::Rewritable(_)) => Vec::new(),
        HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Marked(_) => {
            // `(opens, blamable, folds)`: the files list on a file, on a
            // directory, on neither (the `✉ message` row); the diff on a
            // line or not. Never `folds` without `opens` (only the files
            // list folds), nor with `blamable` (a directory has no file to
            // blame): neither shape exists in the app.
            // The last: the Detail tab, graph hidden (m8/11b decision 3).
            [
                (true, true, false, false),
                (true, false, true, false),
                (true, false, false, false),
                (false, true, false, false),
                (false, false, false, false),
                (false, false, false, true),
            ]
            .into_iter()
            .map(
                |(opens, blamable, folds, graph_hidden)| HintContext::PaneFocused {
                    row: Box::new(context.clone()),
                    opens,
                    blamable,
                    folds,
                    graph_hidden,
                },
            )
            .collect()
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Picker
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::PaneFocused { .. }
        | HintContext::Searching(_)
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::Rewritable(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => Vec::new(),
    }
}

/// Every hint in every context's table, planned ones included, deduped by
/// `(keys, verb, state)` — what `graft`'s guard tests walk so that neither
/// half of the table can drift from what the app binds.
#[must_use]
pub fn every_hint() -> Vec<Hint> {
    let mut all = Vec::new();
    for context in every_context() {
        all.extend(context_list(&context));
    }
    all.extend(TAIL);
    all.push(DOORWAY);
    all.push(DOORWAY_BY_PALETTE);
    all.push(ACTIONS);
    all.push(EXPLAIN);
    all.push(RESIZE);
    all.push(WAY_BACK);
    all.push(WAY_BACK_PREV);
    all.push(SEARCH_NEXT);
    all.push(SEARCH_PREV);
    all.push(BLAME);
    all.push(BACK_TO_BLAME);
    all.push(BACK_TO_HISTORY);
    all.push(F_HISTORY);
    // A directory row's `space fold` (`diff.fold_dir`). Before m8/10 it
    // passed as the graph's `space fold`, the same string; as an action it
    // is its own row.
    all.push(FOLD);
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
