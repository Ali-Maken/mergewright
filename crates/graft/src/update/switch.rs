//! The switch rule: one function every switch goes through — `.` Checkout,
//! the name prompt's `⇥`, and the branch picker's `⏎` (m7-power-tools/10b;
//! keymap §2.1; field notes :79, approved 2026-09-26).
//!
//! 1. A detached target → the preview, then `y`.
//! 2. His last status read shows changed tracked files → the preview with
//!    the set-aside line (ADR-0045 §4, §8), then `y`; run with
//!    [`OnEdits::SetAside`].
//! 3. Otherwise → launched at once with [`OnEdits::Refuse`], no preview.
//!
//! Untracked files alone are not unsaved edits: ADR-0045 §4 leaves them,
//! and git refuses in its own words if one is in the way.
//!
//! A stale "clean" never sets edits aside unseen: the instant launch carries
//! `Refuse`, so `graft_ops` refuses [`Refusal::DirtyTree`] over edits made
//! since the read, and [`instant_refused`] answers that refusal with the
//! preview of the same switch — one `y` more, as with edits.

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_ops::operation::{
    BranchCreateParams, CheckoutParams, CheckoutTarget, OnEdits, Operation, Refusal, Tip,
};

use super::events::Command;
use super::preview::PreviewSubject;
use super::state::{AppState, Status};

/// Where a switch goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitchTarget {
    /// A local branch by full name (`refs/heads/x`).
    Branch(String),
    /// A remote-only branch: a new local `local` tracking `remote`.
    Track {
        /// The remote-tracking ref and the tip the list saw.
        remote: Tip,
        /// The new local branch's short name.
        local: String,
    },
    /// A commit, detached: always previewed.
    Detached(Oid),
    /// A new branch `name` at `at`, switched to (the name prompt's `⇥`).
    NewBranch {
        /// Its short name.
        name: String,
        /// The commit it starts at.
        at: Oid,
    },
}

/// The one switch rule. `opened_by` is the key the preview names if one
/// opens (`.`, `b`), and the key a `Moved` refusal says to press again.
///
/// Every refusal — a write already running, git mid-operation, the refs not
/// read yet — is worded on the status line by the path that met it.
pub fn switch_to(
    state: &mut AppState,
    target: SwitchTarget,
    opened_by: &'static str,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    // Each refusal wrote its own line (the doc above); a caller that must
    // restore something on refusal uses `switch_into` instead.
    let _worded: bool = switch_into(state, &mut commands, target, opened_by, next_id);
    commands
}

/// [`switch_to`], saying whether the preview opened or the write launched.
/// `false`: refused, and the refusal is already worded.
pub(super) fn switch_into(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    target: SwitchTarget,
    opened_by: &'static str,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let previews = previews(state, &target);
    let Some(op) = operation(state, target) else {
        // The push box's words (`remote::push_box::REFS_PENDING`, private
        // to that module): the same wait, the same sentence.
        state.status =
            Status::Waiting("still reading the refs \u{2014} try again in a moment".to_owned());
        return false;
    };
    if previews {
        return super::preview::open_preview(
            state,
            commands,
            PreviewSubject::Operation {
                op: Box::new(op),
                opened_by,
            },
            false,
            next_id,
        );
    }
    let op = with_on_edits(op, OnEdits::Refuse);
    // A create moved no history before this rule and ran stoppable; it
    // still does. A checkout runs as its preview's `y` runs it.
    let launched = if matches!(op, Operation::BranchCreate(_)) {
        super::operate::launch(state, commands, op, next_id)
    } else {
        super::operate::launch_unstoppable(
            state,
            commands,
            op,
            super::preview::HISTORY_CANNOT_STOP,
            Some(opened_by),
            next_id,
        )
    };
    let ok = launched.is_ok();
    super::operate::say_if_refused(state, launched);
    ok
}

/// Whether this switch opens the preview first: a detached target, or
/// changed tracked files in his last status read (rules 1 and 2).
pub(crate) fn previews(state: &AppState, target: &SwitchTarget) -> bool {
    matches!(target, SwitchTarget::Detached(_)) || has_edits(state)
}

/// Changed tracked files, staged or not, in the last status read. Untracked
/// files never count. No read yet reads as none: the instant launch's
/// `Refuse` is what stands behind a read that is missing or stale.
fn has_edits(state: &AppState) -> bool {
    state.working_tree.as_ref().is_some_and(|tree| {
        tree.entries
            .iter()
            .any(|entry| entry.is_staged() || entry.is_unstaged())
    })
}

/// The previewed form of the switch (`SetAside`); `None` when `HEAD` is not
/// known yet for a checkout.
fn operation(state: &AppState, target: SwitchTarget) -> Option<Operation> {
    let to = match target {
        SwitchTarget::NewBranch { name, at } => {
            return Some(Operation::BranchCreate(BranchCreateParams {
                name,
                at,
                checkout: true,
                on_edits: OnEdits::SetAside,
            }));
        }
        SwitchTarget::Branch(name) => CheckoutTarget::Branch(name),
        SwitchTarget::Detached(oid) => CheckoutTarget::Detached(oid),
        SwitchTarget::Track { remote, local } => CheckoutTarget::Track { remote, local },
    };
    let head_before = super::context_menu::verbs::head_ref(state.refs.as_deref()?)?;
    Some(Operation::Checkout(CheckoutParams {
        to,
        head_before,
        on_edits: OnEdits::SetAside,
    }))
}

fn with_on_edits(op: Operation, on_edits: OnEdits) -> Operation {
    match op {
        Operation::Checkout(params) => Operation::Checkout(CheckoutParams { on_edits, ..params }),
        Operation::BranchCreate(params) => {
            Operation::BranchCreate(BranchCreateParams { on_edits, ..params })
        }
        other => other,
    }
}

/// The switch a target names, as `.` Checkout's verb built it.
#[must_use]
pub(crate) fn target_of(op: &Operation) -> Option<SwitchTarget> {
    match op {
        Operation::Checkout(params) => Some(match &params.to {
            CheckoutTarget::Branch(name) => SwitchTarget::Branch(name.clone()),
            CheckoutTarget::Detached(oid) => SwitchTarget::Detached(*oid),
            CheckoutTarget::Track { remote, local } => SwitchTarget::Track {
                remote: remote.clone(),
                local: local.clone(),
            },
        }),
        _ => None,
    }
}

/// `operate::operated`'s refused arm: an instant switch refused for edits
/// made since the last read opens the preview of the same switch instead
/// of a refusal line (rule 2). `None`: not an instant switch refused
/// `DirtyTree` — the caller words it as before.
pub(crate) fn instant_refused(
    state: &mut AppState,
    op: &Operation,
    refusal: &Refusal,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<Vec<Command>> {
    if !matches!(refusal, Refusal::DirtyTree { .. }) {
        return None;
    }
    let opened_by = match op {
        Operation::Checkout(params) if params.on_edits == OnEdits::Refuse => ".",
        Operation::BranchCreate(params)
            if params.checkout && params.on_edits == OnEdits::Refuse =>
        {
            "b"
        }
        _ => return None,
    };
    // The key that launched it, when the launch recorded one (a checkout
    // does; a create ran as `b` always has).
    let opened_by = state
        .remote
        .running
        .as_ref()
        .and_then(|running| running.opened_by)
        .unwrap_or(opened_by);
    let mut commands = Vec::new();
    // The launch's `Busy` is over; the preview (or its refusal) says what
    // is true now.
    state.status = Status::Ready;
    // A refusal to open (a preview already up, git mid-operation) wrote its
    // own line.
    let _opened: bool = super::preview::open_preview(
        state,
        &mut commands,
        PreviewSubject::Operation {
            op: Box::new(with_on_edits(op.clone(), OnEdits::SetAside)),
            opened_by,
        },
        false,
        next_id,
    );
    Some(commands)
}

/// What a switch's clean landing says before its tick: `on <b>`, or `on <b>
/// (new, tracks <origin/b>)`. `None` for a detached checkout (today's words)
/// and every other kind. Drawn `on <b> ✓ · u goes back` — the tick where
/// every `Done` draws it (`view::status::done_line`; conductor ruling,
/// m7-power-tools/10b), with `your edits are back` between when edits were
/// set aside.
#[must_use]
pub(crate) fn landed_words(op: &Operation) -> Option<String> {
    let Operation::Checkout(params) = op else {
        return None;
    };
    match &params.to {
        CheckoutTarget::Branch(name) => Some(format!(
            "on {}",
            name.strip_prefix("refs/heads/").unwrap_or(name)
        )),
        CheckoutTarget::Track { remote, local } => Some(format!(
            "on {local} (new, tracks {})",
            remote
                .name
                .strip_prefix("refs/remotes/")
                .unwrap_or(&remote.name)
        )),
        CheckoutTarget::Detached(_) => None,
    }
}

/// The key half of a switch's landing line.
pub(crate) const GOES_BACK: &str = "u goes back";

#[cfg(test)]
mod tests;
