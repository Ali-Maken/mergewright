//! What a Ghost Preview is of, and what it holds while open: the subject,
//! the request a job answers, and the plan once it lands.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use mergewright_core::commit::Commit;
use mergewright_core::job::JobId;
use mergewright_core::journal::EntryId;
use mergewright_core::oid::Oid;
use mergewright_graph::row::Row;
use mergewright_ops::operation::Operation;
use mergewright_ops::preview::PreviewPlan;

use super::{Showing, nothing_to_do};
use crate::update::detail::Loading;

/// What a preview is a preview of.
///
/// The `Operation` is boxed and carried whole rather than rebuilt at
/// confirm time: the value that runs must be the value that was drawn, by
/// `PartialEq` (`m_opens_preview_and_enter_runs_the_pinned_op`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewSubject {
    /// One history operation, pinned to the tips it was drawn against.
    Operation {
        /// What `Confirm` runs.
        op: Box<Operation>,
        /// Which key opened it, so a `Moved` refusal can say which key to
        /// press again (`m again`, `r again`, `. again`).
        opened_by: &'static str,
    },
    /// The journal's time machine: revert to just before this entry.
    RevertTo(EntryId),
}

impl PreviewSubject {
    /// The operation this subject would run, when it has one.
    #[must_use]
    pub fn operation(&self) -> Option<&Operation> {
        match self {
            PreviewSubject::Operation { op, .. } => Some(op),
            PreviewSubject::RevertTo(_) => None,
        }
    }
}

/// What the preview job asks for: the subject, and the commits the app
/// already has on screen so the job only reads the ones it does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewRequest {
    /// What to plan.
    pub subject: PreviewSubject,
    /// Every commit id the app can draw without reading anything.
    pub resident: HashSet<Oid>,
}

/// Why the preview job drew nothing.
///
/// Two answers, routed apart (m6-conflicts/67): a refusal is not a failure,
/// and must not read as one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewFailure {
    /// `mergewright_ops::preview::plan` refused the operation — the same refusal
    /// the write would give. No box: the status line says why.
    Refused(mergewright_ops::operation::Refusal),
    /// A read failed. The line the `preview failed:` box prints, written
    /// for the user.
    Failed(String),
}

/// What the job came back with: the plan, and the commits it had to read to
/// anchor ghosts whose parents were not on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    /// What the operation would do.
    pub plan: PreviewPlan,
    /// Commits read to anchor the picture, at most [`PREVIEW_ANCHOR_CAP`].
    pub anchors: Vec<Commit>,
    /// The operation as `y` will run it (`mergewright_ops::preview::plan_as_run`):
    /// a rebase completed with its line and drops, anything else as asked.
    /// `preview_produced` moves it into the subject, so the op `y` confirms
    /// is the one the picture was drawn from. `None` for the time machine.
    pub runs: Option<Box<Operation>>,
}

/// One open Ghost Preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewState {
    /// What is being previewed.
    pub subject: PreviewSubject,
    /// The rows the graph had on hand when the preview opened, in drawing
    /// order — the slice the picture is laid out from.
    ///
    /// The commits themselves, not only their ids (which is what the job
    /// gets): the picture must not change under the user because a refresh
    /// landed while they were reading it, and a snapshot is the cheapest way
    /// to promise that. Drawing commits: see `resident::Resident::commits`.
    pub resident: Vec<Commit>,
    /// Which of `resident` the app calls merges, from their source rows
    /// (06r m1): a folded merge draws `◉` in the picture too.
    pub merges: HashSet<Oid>,
    /// The absolute history row number of `resident[0]`.
    pub slice_top: usize,
    /// The planning job. Only this id's answer is wanted.
    pub job: JobId,
    /// The plan, once it lands.
    pub plan: Loading<Arc<Planned>>,
    /// The laid-out rows the graph draws while the plan is `Ready`.
    pub rows: Vec<Row>,
    /// Where the user's screen begins inside [`PreviewState::rows`].
    ///
    /// The slice is three screens wide so a ghost has somewhere to hang,
    /// but only one of those screens is the user's. Drawing from `rows[0]`
    /// shows them the screen *above* theirs the moment they are scrolled
    /// (ticket 17 F1), so the picture starts here instead: the index of the
    /// row that was at the top of the viewport when the preview opened.
    pub view_start: usize,
    /// The commit the cursor was on, as an index into
    /// [`PreviewState::rows`] — what the picture highlights.
    ///
    /// Not [`PreviewState::cursor`], which is the *choice* cursor inside
    /// the box (`0` Cancel, `1` Confirm) and has nothing to do with a row.
    pub selected_row: Option<usize>,
    /// Which of [`PreviewState::rows`] are ghosts, by their synthetic id,
    /// mapped to their index in the plan's ghost list.
    pub ghost_index: HashMap<Oid, usize>,
    /// How many parent links the slice had to drop at its edge.
    pub clipped: usize,
    /// A ghost parent that is neither resident nor an anchor: the picture
    /// cannot be drawn in the right place, so it is not drawn at all and the
    /// box says which commit is missing.
    pub undrawable: Option<Oid>,
    /// Which choice the cursor is on. `0` is always the safe one.
    pub cursor: usize,
    /// The journal browser was open and asked for this; closing the preview
    /// puts it back at the same row.
    pub return_to_journal: bool,
    /// The plan was refused and the box stays up saying why: a stash
    /// put-back's preview (m6-conflicts/86), whose words — `your unsaved
    /// edits to notes.md are in the way` — are about the list under it.
    /// [`PreviewState::plan`] is then `Failed(<the refusal's words>)`,
    /// drawn without the `preview failed:` prefix.
    pub refused: bool,
    /// A merge preview's box or its incoming list (ease-pass 10).
    pub showing: Showing,
}

impl PreviewState {
    /// The plan, if it has landed.
    pub(super) fn planned(&self) -> Option<&Planned> {
        match &self.plan {
            Loading::Ready(planned) => Some(planned),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
        }
    }

    /// Is this preview still waiting for its plan?
    pub(super) fn planning(&self) -> bool {
        matches!(self.plan, Loading::Idle | Loading::Pending(_))
    }

    /// Can `Enter` act at all? Only a plan that landed, that does
    /// something, and that could be drawn offers `Confirm`.
    pub(super) fn acts(&self) -> bool {
        self.undrawable.is_none() && self.planned().is_some_and(|it| !nothing_to_do(&it.plan))
    }

    /// What to call the predicted block on screen, in the user's words:
    /// `after rebase`, `after merge`, `after cherry-pick`, `after revert`,
    /// else `the result`.
    ///
    /// Drawn on a row of its own above the first predicted row
    /// (`view::preview::label_entry`). The label is the separator whose
    /// absence was the complaint: without it the predicted subjects are
    /// word for word the real ones directly above them, and the first
    /// available reading is *these commits are in my history twice*
    /// (m5/20 item 4). **Never says `ghost`** — that is our word, a
    /// `CONTEXT.md` glossary term the interface never defines.
    #[must_use]
    pub fn outcome_label(&self) -> String {
        self.subject
            .operation()
            .map(mergewright_ops::operation::Operation::kind)
            .and_then(outcome_verb)
            .map_or_else(|| "the result".to_owned(), |verb| format!("after {verb}"))
    }
}

/// The word for what the operation does, for [`PreviewState::outcome_label`]
/// — the verb the user pressed a key for, not the type's name. `None` for
/// the kinds a preview never opens over, which fall back to `the result`
/// rather than inventing a verb for them.
fn outcome_verb(kind: mergewright_core::journal::OperationKind) -> Option<&'static str> {
    use mergewright_core::journal::OperationKind;
    match kind {
        OperationKind::Merge => Some("merge"),
        OperationKind::Rebase => Some("rebase"),
        OperationKind::CherryPick => Some("cherry-pick"),
        OperationKind::Revert => Some("revert"),
        _ => None,
    }
}
