//! Ghost Preview: **you see the result before you confirm.**
//!
//! Every history-changing key — `m`, `r`, the `.` menu's verbs, the
//! journal's time machine — opens this instead of running anything. A job
//! asks `mergewright_ops::preview` what the operation *would* do; the answer is
//! drawn as ghost rows in the graph and as a box docked above the footer
//! naming the refs that move, the working tree's fate, a conflict forecast
//! and the exact git commands. `Enter` on `Confirm` then runs **the same
//! [`Operation`] value** that was previewed (design note, hard choice 3):
//! the picture and the act are the same act, and a tip that moved in
//! between is refused by `check` rather than quietly re-planned.
//!
//! Modal, like the confirm box and the context menu: while it is open
//! [`preview_key`] claims every key but `Ctrl+C`, so a `u` typed over a
//! preview does nothing at all. It sits between the confirm box and the
//! context menu in [`super::keys::key_event`] — a confirm box may open over
//! a preview, never the reverse, and the menu closes before a preview opens.
//!
//! Seam rule (ARCHITECTURE.md): this module builds an [`Operation`] and
//! reads a [`PreviewPlan`]. **Every word in the box comes from the plan** —
//! there is no hand-written `git …` line anywhere here (`KNOWN_GAPS` row
//! 229), which is what keeps the box and the Command Log from describing
//! one operation two ways.

mod line_rows;
#[cfg(test)]
pub(crate) use answer::not_loaded;
#[cfg(test)]
pub(crate) use history::stash_isnt_a_branch;
pub(crate) use line_rows::{ForkTone, fork_rows, onto_label};
#[cfg(test)]
pub(crate) use line_rows::{keep_them, published_words};
#[cfg(test)]
pub(crate) use take::{not_a_commit_words, waiting_for_refs};
pub mod slice;

use std::collections::HashMap;
use std::sync::Arc;

use mergewright_core::event::{KeyCode, KeyPress, Modifiers};
use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_graph::row::Row;
use mergewright_ops::operation::{Operation, PickParams, RebaseParams, RecoverParams, Source, Tip};
use mergewright_ops::preview::{
    ConflictForecast, DropSuggestion, GhostCommit, HeadMove, PreviewPlan, RefMove, Target,
    UndoNote, WorktreeEffect,
};
use mergewright_term::{SanitizedText, sanitize_line};
use mergewright_ui::keymap::{ActionId, Keymap};
use mergewright_ui::preview_box::{Again, PREVIEW_MIN, PreviewBox, preview_rows};

mod answer;
pub(crate) mod boxes;
mod history;
mod keys;
mod plan_lines;
mod rect;
mod resident;
mod state;
pub use state::{Planned, PreviewFailure, PreviewRequest, PreviewState, PreviewSubject};
mod take;
pub use answer::{
    close_preview, confirm_preview, preview_badges, preview_box, preview_click, preview_produced,
};
use boxes::{
    closing_box, planning_title, put_back_box, put_in_words, ready_box, stash_box, stash_put_back,
    stash_put_back_op, stash_verb,
};
pub use history::{HistoryKey, history_key, is_history_key};
pub(super) use history::{current_branch, short_name_of, source_at};
#[cfg(test)]
use keys::confirm_key;
use keys::{confirm_by_key, enter, footer_keys, keeps_drops_here};
pub use keys::{fits, preview_key};
use plan_lines::{
    capped_list, conflict_line, list, plan_lines, ref_line, short, undo_line, with_cursor,
    worktree_line,
};
mod merge_view;
pub use merge_view::Showing;
use merge_view::{merge_box, merge_key, withheld};
use rect::choice_at;
pub use rect::{BoxRect, preview_box_rect, preview_rows_taken};
use take::take_title_of;
pub(crate) use take::{is_take_key, take_key};

pub use self::slice::{BadgeOverlay, PREVIEW_ANCHOR_CAP};

use self::slice::{SliceError, badges_for_preview, build_slice};
use super::detail::Loading;
use super::events::Command;
use super::state::{AppState, Status};
use crate::refs::{RefBadges, RefLabel};

/// Why `Esc` will not stop a running history operation.
///
/// Rendered after the operation's own description
/// (`remote::stop_write`: `"{description} {why}"`), so it reads
/// `rebase feature/auth onto main runs to the end · a conflict stops it on
/// the banner, where a aborts`.
///
/// A merge or a rebase is not one git write. Killing git mid-rebase leaves
/// `rebase-merge/` on disk with the journal entry `Pending` and no outcome
/// line — a state `mergewright_ops` does not model as `Aborted`, and which the
/// banner would then have to adopt as if the user had asked for it. The
/// honest answer is not to offer the cancel: a conflict stops the operation
/// by itself, on the banner, where `a` aborts it properly.
pub const HISTORY_CANNOT_STOP: CannotStop = CannotStop::History;

/// Why `Esc` will not stop a running put-back from J (m7/30o Minor 7):
/// `put main back to 9a8b7c6 runs to the end · u undoes it after`. A
/// put-back has no conflict and no banner, so [`HISTORY_CANNOT_STOP`]'s
/// words would name a place he never lands on; `u` is what takes it back.
pub const PUT_BACK_CANNOT_STOP: CannotStop = CannotStop::PutBack;

/// Which of the two "`Esc` cannot stop this" lines a running write gets.
/// Worded where it is drawn ([`CannotStop::words`]), from his keymap
/// (m8/10i): the key the line names is the one that answers on the banner
/// or the graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CannotStop {
    /// [`HISTORY_CANNOT_STOP`]: a merge, rebase, pick or jump.
    History,
    /// [`PUT_BACK_CANNOT_STOP`]: a put-back from J.
    PutBack,
}

impl CannotStop {
    /// `runs to the end · a conflict stops it on the banner, where a
    /// aborts` (`operation.abort`, on the banner) or `runs to the end · u
    /// undoes it after` (`app.undo`, on the graph). An unbound key drops
    /// its words: `, where a aborts` alone (the conflict fact stays), or the
    /// whole `u` clause.
    #[must_use]
    pub fn words(self, keys: &Keymap) -> String {
        match self {
            Self::History => {
                let abort = keys
                    .key_verb(&[ActionId("operation.abort")], "aborts")
                    .map_or_else(String::new, |aborts| format!(", where {aborts}"));
                format!("runs to the end \u{b7} a conflict stops it on the banner{abort}")
            }
            Self::PutBack => keys.after("runs to the end", ActionId("app.undo"), "undoes it after"),
        }
    }
}

/// A plan that would change nothing: a merge already up to date, a reset to
/// where the branch already is. There is nothing to confirm, so the box
/// offers only `Close` — an enabled `Confirm` that does nothing is how a
/// user learns to stop reading the box.
fn nothing_to_do(plan: &PreviewPlan) -> bool {
    plan.ref_moves.is_empty()
        && plan.head.is_none()
        && plan.ghosts.is_empty()
        && matches!(plan.worktree, WorktreeEffect::Untouched)
}

/// Why a preview cannot open right now, as the words to show: a write is
/// already running, or git is mid-operation. `None` when it can.
///
/// Split out of [`open_preview`] so a caller whose own modal stays open can
/// show the refusal inside that modal instead of on the status line under
/// it: branch-from-stash's name prompt (m6-conflicts/69, conductor
/// 2026-09-25).
pub(crate) fn preview_refusal(state: &AppState) -> Option<String> {
    if let Some(busy) = super::operate::in_flight(state) {
        return Some(busy.words());
    }
    // Every live operation refuses, a merge begun in the shell included:
    // keymap §2.6a's exception covers `u`/`U` only, and this opens a Ghost
    // Preview or the journal's revert. No keys: the preview and the journal
    // overlays hide the banner, so `⇥ banner` would point at nothing.
    let kind = state.in_progress.as_ref()?.kind;
    Some(super::undo::live_refusal_words(kind))
}

/// Opens a preview of `subject`, or refuses in words.
///
/// Refuses — with no job started — while another write is running, while a
/// preview is already open, and while git has an operation in progress:
/// none of the three can be previewed honestly, and a preview drawn against
/// a repository that is about to change is a picture of nothing.
///
/// `true` when a preview opened.
pub fn open_preview(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    subject: PreviewSubject,
    return_to_journal: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    // A preview is already on screen. It is modal and it is the answer to
    // this key, so there is nothing to say that the screen is not already
    // saying.
    if state.preview.is_some() {
        return false;
    }
    if let Some(reason) = preview_refusal(state) {
        state.status = Status::Sticky { reason, keys: None };
        return false;
    }

    // The slice: every resident row in [top − graph_rows, top + 2·graph_rows),
    // snapshotted now. Taken **before** `state.preview` is set, so the box's
    // own rows have not yet shrunk the window it is measured in.
    let height = state.graph_rows();
    let resident::Resident {
        top: slice_top,
        commits: resident,
        merges,
    } = resident::slice(state, height);
    let id = next_id();
    let request = PreviewRequest {
        subject: subject.clone(),
        resident: resident.iter().map(|commit| commit.id).collect(),
    };
    state.preview = Some(PreviewState {
        subject,
        resident,
        merges,
        slice_top,
        job: id,
        plan: Loading::Pending(id),
        rows: Vec::new(),
        view_start: 0,
        selected_row: None,
        ghost_index: HashMap::new(),
        clipped: 0,
        undrawable: None,
        cursor: 0,
        return_to_journal,
        refused: false,
        showing: Showing::Box,
    });
    commands.push(Command::PlanPreview(id, Box::new(request)));
    true
}

#[cfg(test)]
pub(super) mod tests;
