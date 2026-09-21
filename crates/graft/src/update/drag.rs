//! Dragging a branch badge onto a row (`docs/design/02-keymap.md` §2.15):
//! the three-event state machine, and the menu the drop opens.
//!
//! Three facts from the scout shape all of it:
//!
//! - **`Drag(Left)` only reaches `update` while something has claimed the
//!   drag.** `mouse_event`'s match reads it behind a guard, exactly as the
//!   detail pane's divider does, so a badge drag has to claim a state of its
//!   own on the button down or it will never see the motion.
//! - **A context menu open swallows everything but a button down.** The drop
//!   opens a menu, so no drag event arrives after it — which is fine, and is
//!   why nothing here tries to keep dragging afterwards.
//! - **`Shift` is never part of the gesture.** `Shift`+drag stays the
//!   terminal's own text selection (PROMPT.md §3, ADR-worthy rule the keymap
//!   restates); Graft never asks for it.
//!
//! Nothing here writes. A drop ends in the same Ghost Preview every key ends
//! in — `open_preview`, never `Command::Operate` (PROMPT.md §1.7).

use graft_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use graft_core::oid::Oid;
use graft_ops::operation::{
    FastForward, MergeParams, Operation, PickParams, RebaseParams, ResetMode, ResetParams,
    ResetTarget, Source, Tip,
};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::context_menu::ContextMenu;

use super::context_menu::{HistoryVerb, OpenMenu};
use super::events::Command;
use super::geometry;
use super::state::AppState;

/// The branch badge a drag picked up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragSource {
    /// Full ref name, `refs/heads/feature`.
    pub name: String,
    /// Short name, `feature` — what the labels say.
    pub short: String,
    /// The commit it pointed at when the drag started; every operation the
    /// drop offers is pinned to it (design note, hard choice 3).
    pub at: Oid,
    /// `HEAD` is on this branch. Which verbs can run turns on it.
    pub current: bool,
}

/// A badge in the air.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragState {
    /// What was picked up.
    pub source: DragSource,
    /// The absolute history row it was picked up from.
    pub from_row: usize,
    /// The absolute history row the pointer is over, or `None` over the
    /// header, the footer, the working-tree row or a pane.
    pub over: Option<usize>,
}

/// The mouse half of the gesture. `true` when the event was the drag's and
/// nothing else should be made of it.
///
/// Called from [`super::mouse::mouse_event`] **before** the ordinary graph
/// click: a button down on a branch badge starts a drag and selects nothing
/// yet, and the `Up` that follows with no motion is an ordinary click, so
/// picking up a badge never costs the user the selection they were making.
pub fn drag_mouse(state: &mut AppState, commands: &mut Vec<Command>, mouse: MouseInput) -> bool {
    match mouse.kind {
        MouseKind::Down(MouseButton::Left) if state.drag.is_none() => press(state, mouse),
        MouseKind::Drag(MouseButton::Left) if state.drag.is_some() => {
            let over = geometry::graph_row_at(state, mouse.row);
            if let Some(drag) = state.drag.as_mut() {
                drag.over = over;
            }
            true
        }
        MouseKind::Up(_) => release(state, commands),
        _ => false,
    }
}

/// The button down: pick a badge up, or leave the press to the ordinary
/// click handling.
fn press(state: &mut AppState, mouse: MouseInput) -> bool {
    // `Shift`/`Ctrl`+click is the compare gesture (`x`), and the terminal
    // keeps `Shift`+drag for its own selection: neither may start a drag.
    if mouse.modifiers.shift || mouse.modifiers.control || !can_start(state) {
        return false;
    }
    let Some(row) = geometry::graph_row_at(state, mouse.row) else {
        return false;
    };
    let Some(source) = geometry::badge_at(state, mouse.column, mouse.row) else {
        return false;
    };
    state.drag = Some(DragState {
        source,
        from_row: row,
        over: None,
    });
    true
}

/// The button up: open the drop menu, or finish the press as the plain
/// click it turned out to be. `false` when nothing was in the air.
fn release(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    let Some(drag) = state.drag.take() else {
        return false;
    };
    match drag.over {
        // A drop on the row the badge came from is not a drop: merging a
        // branch into itself is nothing, and the user has most likely just
        // clicked.
        Some(target) if target != drag.from_row => {
            state.context_menu = drop_menu(state, &drag.source, target);
        }
        // Released off a row, or back where it started: the gesture is over
        // and nothing opens, so the press was a plain click on the badge's
        // own row — which is what it does now. The selection was not moved
        // on the button down, because at that moment there was no telling a
        // click from a drag.
        _ => super::mouse::select_graph_row(state, commands, drag.from_row),
    }
    true
}

/// A key arriving mid-drag. `true` when the key was spent cancelling.
///
/// Any key drops the badge: the drag is a mouse gesture with no keyboard
/// half, and a hand that has left the mouse has left the gesture. Making
/// `j` mean nothing until the button came back up would be a mode with no
/// sign that it is one.
///
/// `Esc` is the one that is *spent* doing it — that is what the hint bar
/// offers it for, and letting it also close the pane underneath would be two
/// things from one press. Every other key cancels and then goes on to mean
/// what it always means.
pub(super) fn drag_key(state: &mut AppState, key: KeyPress) -> bool {
    state.drag.take().is_some() && key.code == KeyCode::Escape
}

/// Can a drag start at all? Not over a preview, a confirm box, the commit
/// editor, the credential prompt, the name prompt, a context menu or one of
/// the overlays: each of those is either modal or stands where the graph
/// does, so there is no badge on screen to pick up.
fn can_start(state: &AppState) -> bool {
    !(state.preview.is_some()
        || state.confirm.is_some()
        || state.context_menu.is_some()
        || state.commit_editor.is_some()
        || state.credential.is_some()
        || state.name_prompt.is_some()
        || state.stash.shown
        || state.journal.shown
        || state.map.shown)
}

/// The row under the pointer, painted while a drag is over it.
#[must_use]
pub fn drop_target(state: &AppState) -> Option<usize> {
    state.drag.as_ref()?.over
}

/// The menu a drop on `target` opens: the four verbs §2.15 names, each with
/// the reason it cannot run right now, over a title saying what went where.
///
/// Pure. `None` when the target row is not on hand (the window moved under
/// the pointer) or the refs have not arrived — a verb that cannot say which
/// branch it moves is one this menu is better off not offering, the same
/// rule `context_menu::open_for_history_row` follows.
#[must_use]
pub fn drop_menu(state: &AppState, source: &DragSource, target: usize) -> Option<OpenMenu> {
    let refs = state.refs.as_deref()?;
    let oid = state
        .history
        .as_ref()
        .and_then(|history| history.row_at(target))
        .map(|row| row.commit.id)?;
    // The target's own local branch, if it has one: `Merge … into <B>` and
    // `Rebase … onto <B>` name it, and without one the merge is not offered
    // at all (git merges into `HEAD`'s branch, so there must be a branch).
    let onto_branch = refs.on(oid).iter().find_map(|label| match label {
        crate::refs::RefLabel::Branch { name, current, .. } => {
            Some((String::from_utf8_lossy(name).into_owned(), *current))
        }
        _ => None,
    });
    let short_oid: String = oid.to_string().chars().take(7).collect();
    let target_text = onto_branch
        .as_ref()
        .map_or_else(|| short_oid.clone(), |(name, _)| name.clone());

    let verbs = drop_verbs(source, oid, onto_branch.as_ref(), &short_oid);
    if verbs.is_empty() {
        return None;
    }
    let title = sanitize_line(&format!("{} → {target_text}", source.short));
    let labels = verbs
        .iter()
        .map(|(verb, reason)| (verb.label.clone(), reason.clone()))
        .collect();
    Some(OpenMenu {
        row: target,
        menu: ContextMenu::for_drop(title, labels),
        verbs: verbs.into_iter().map(|(verb, _)| verb).collect(),
    })
}

/// One drop menu's rows: the verb, and why it cannot run (`None` when it
/// can).
///
/// Who has to be checked out, and why:
///
/// - **Merge** runs `git merge`, which merges into `HEAD` — so the *target*
///   branch must be the current one.
/// - **Rebase** replays `HEAD`'s branch — so the *dragged* branch must be.
/// - **Cherry-pick** commits onto `HEAD` — likewise.
/// - **Reset** is the one that always works: on the current branch it is
///   `git reset`, with a `--hard` row beside the `--mixed` one; on any other
///   branch it is `update-ref` with compare-and-swap, which needs no
///   checkout and touches neither the index nor the working tree — so it is
///   labelled `Move <b> to here` rather than `Reset`, because that is what
///   it does.
fn drop_verbs(
    source: &DragSource,
    oid: Oid,
    onto_branch: Option<&(String, bool)>,
    short_oid: &str,
) -> Vec<(HistoryVerb, Option<SanitizedText>)> {
    let source_tip = Tip {
        name: source.name.clone(),
        oid: source.at,
    };
    // What `Rebase` and `Cherry-pick` both need, and both say when they do
    // not have it.
    let needs_source =
        || (!source.current).then(|| sanitize_line(&format!("check out {} first", source.short)));
    let mut verbs = Vec::with_capacity(5);

    // Merge is absent, not disabled, on a row with no local branch: there is
    // no branch for `git merge` to merge into, so there is no verb to name.
    if let Some((into, into_current)) = onto_branch {
        // `git merge` merges into `HEAD`, so it is the *target* branch that
        // has to be the checked-out one — not the badge being dragged.
        let reason = (!into_current).then(|| sanitize_line(&format!("check out {into} first")));
        verbs.push((
            verb(
                &format!("Merge {} into {into}", source.short),
                Operation::Merge(MergeParams {
                    source: Source::Branch(source.short.clone()),
                    into: Tip {
                        name: format!("refs/heads/{into}"),
                        oid,
                    },
                    source_tip: source.at,
                    ff: FastForward::Default,
                }),
            ),
            reason,
        ));
    }

    let onto_text = onto_branch.map_or(short_oid, |(name, _)| name.as_str());
    verbs.push((
        verb(
            &format!("Rebase {} onto {onto_text}", source.short),
            Operation::Rebase(RebaseParams {
                branch: source_tip.clone(),
                onto: onto_branch.map_or(Source::Commit(oid), |(name, _)| {
                    Source::Branch(name.clone())
                }),
                onto_tip: oid,
            }),
        ),
        needs_source(),
    ));

    verbs.extend(reset_verbs(source, &source_tip, oid));

    verbs.push((
        verb(
            &format!("Cherry-pick {short_oid} onto {}", source.short),
            Operation::CherryPick(PickParams {
                commits: vec![oid],
                subject: String::new(),
                head: source_tip,
            }),
        ),
        needs_source(),
    ));
    verbs
}

/// The reset row (or rows). Always enabled, and the only verb that is: on
/// the current branch it is `git reset`, and both modes are offered because
/// which one the user wants is not something the gesture can say; on any
/// other branch it is a compare-and-swap `update-ref`, which moves the ref
/// and nothing else — so it is labelled `Move`.
fn reset_verbs(
    source: &DragSource,
    source_tip: &Tip,
    oid: Oid,
) -> Vec<(HistoryVerb, Option<SanitizedText>)> {
    if !source.current {
        return vec![(
            verb(
                &format!("Move {} to here", source.short),
                Operation::Reset(ResetParams {
                    target: ResetTarget::Branch(source_tip.clone()),
                    to: oid,
                }),
            ),
            None,
        )];
    }
    [("mixed", ResetMode::Mixed), ("hard", ResetMode::Hard)]
        .into_iter()
        .map(|(word, mode)| {
            (
                verb(
                    &format!("Reset {} to here, {word}", source.short),
                    Operation::Reset(ResetParams {
                        target: ResetTarget::Head {
                            branch: source_tip.clone(),
                            mode,
                        },
                        to: oid,
                    }),
                ),
                None,
            )
        })
        .collect()
}

/// A drop verb: the label the menu draws and the operation behind it. A
/// trailing `(key)` is reserved for keys that are live right now
/// (`docs/design/02-keymap.md` §2.1) and none of these has one — a reset
/// mode is written after a comma, never parenthesised (m5/12).
fn verb(label: &str, op: Operation) -> HistoryVerb {
    HistoryVerb {
        label: sanitize_line(label),
        op,
    }
}

#[cfg(test)]
mod tests;
