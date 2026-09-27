//! A pull's fetch, run before `apply` decides anything else
//! (m7-power-tools/03b, ADR-0045 §4 extended to a fast-forward pull).
//!
//! Whether a pull sets edits aside depends on what the fetch brings, which
//! is unknowable before it runs; and ADR-0045 §4 needs the set-aside `S`
//! on the `Pending` line before it is stored. So the fetch runs first, then
//! [`RepoFacts::incoming`] is read, then `apply` goes on as for a rebase —
//! with the merge alone left to run. A fetch that fails opens and closes
//! the entry exactly as before 03b: both command lines, `Aborted` with
//! git's reason, nothing set aside.

use graft_git::status::StatusEntry;

use crate::facts::{self, FactsError};
use crate::operation::{Incoming, Invocation, Operation, PullParams, RepoFacts};

use super::engine::Engine;
use super::run::{PlanRun, run_plan};
use super::{AfterState, ApplyError, Context, Opening, Pending, open_entry};

/// What running a pull's fetch ahead of its entry left.
pub(super) enum Ahead {
    /// Not a pull: the plan runs whole, after the entry opens, as ever.
    Nothing,
    /// The fetch went through: the plan's first line is done, the rest is
    /// left to run. The fetch's cleanup (an askpass server that would not
    /// close) goes on the entry with the rest's.
    Fetched(Vec<AfterState>),
    /// The fetch failed or was stopped: the entry opens and closes on this
    /// run as it did when the fetch ran inside it.
    Failed(PlanRun),
}

/// Run a pull's fetch (the plan's first invocation) and, when it goes
/// through, fill `facts.incoming`. Anything else: [`Ahead::Nothing`].
///
/// # Errors
/// The incoming read failed or was stopped. The fetch has run by then and
/// no entry is open: like a plain fetch, it moved only tracking refs, which
/// a pull never records.
pub(super) fn fetch_first(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &mut RepoFacts,
    plan: &[Invocation],
) -> Result<Ahead, ApplyError> {
    let (Operation::Pull(params), Some(fetch)) = (op, plan.first()) else {
        return Ok(Ahead::Nothing);
    };
    let run = run_plan(ctx, std::slice::from_ref(fetch), op.remote_names().as_ref());
    if run.failure.is_some() {
        return Ok(Ahead::Failed(run));
    }
    facts.incoming = incoming(ctx, params, facts).map_err(|error| match error {
        FactsError::Stopped => ApplyError::Stopped,
        other => ApplyError::Facts(other),
    })?;
    Ok(Ahead::Fetched(run.cleanup))
}

/// Step 2 for a pull whose fetch failed or was stopped: the entry opens on
/// it all the same, as it did when it opened before the fetch. A stop the
/// fetch already answered (`Esc` on a hung remote) would otherwise make
/// `begin` refuse the entry, and the cancelled pull would leave no record —
/// so the opening runs with that stop set aside, then puts it back.
/// Anything else: [`open_entry`] as ever.
pub(super) fn open_after(
    ctx: &mut Context<'_>,
    ahead: &Ahead,
    opening: Opening,
    engine: Option<&Engine>,
) -> Result<Pending, ApplyError> {
    if !matches!(ahead, Ahead::Failed(_)) {
        return open_entry(ctx, opening, engine);
    }
    let stop = ctx.stop;
    ctx.stop = &();
    let pending = open_entry(ctx, opening, engine);
    ctx.stop = stop;
    pending
}

/// Run what is left of the plan: nothing when the fetch failed (its run is
/// the answer), the merge after a fetch that went through (its cleanup kept,
/// first), the whole plan for any other operation.
pub(super) fn run_rest(
    ctx: &Context<'_>,
    op: &Operation,
    ahead: Ahead,
    to_run: &[Invocation],
) -> PlanRun {
    match ahead {
        Ahead::Failed(fetched) => fetched,
        Ahead::Fetched(cleanup) => {
            let mut run = run_plan(ctx, to_run, op.remote_names().as_ref());
            run.cleanup.splice(0..0, cleanup);
            run
        }
        Ahead::Nothing => run_plan(ctx, to_run, op.remote_names().as_ref()),
    }
}

/// What the fetch brought, read only over changed tracked files: a clean
/// tree (untracked files aside) spawns nothing, since there is nothing to
/// set aside. `None` too when the branch or its tracking ref is absent —
/// git's own merge says so.
fn incoming(
    ctx: &Context<'_>,
    params: &PullParams,
    facts: &RepoFacts,
) -> Result<Option<Incoming>, FactsError> {
    let changed = |entry: &StatusEntry| entry.is_staged() || entry.is_unstaged();
    if !facts.status.entries.iter().any(changed) {
        return Ok(None);
    }
    if ctx.stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let tracking = &params.upstream.tracking_ref;
    let refs = facts::read_refs(ctx.repo, &[params.branch.clone(), tracking.clone()])?;
    let (Some(Some(head)), Some(Some(tip))) = (refs.get(&params.branch), refs.get(tracking)) else {
        return Ok(None);
    };
    facts::read_incoming(ctx.repo, *head, *tip, ctx.stop).map(Some)
}
