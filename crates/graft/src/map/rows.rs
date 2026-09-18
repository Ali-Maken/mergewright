//! Every row [`super::build_map`] draws: one [`build_row`] per local branch,
//! in the order [`super::walk::sort_and_cap_locals`] already fixed. Split out
//! of `map.rs` (m2 review M8) — the layer above [`super::walk`], which does
//! the walking.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::CommitTable;
use graft_git::run::Stop;
use graft_graph::identity::BranchIdentity;
use graft_ops::mistake_radar::{Policy, RepoFacts};

use super::walk::walk_branch;
use super::{MapBranch, MapError, bucket_index};

/// What every row's walk shares, gathered once by [`super::build_map`] before
/// the per-branch loop.
pub(crate) struct RowContext<'a> {
    pub(crate) handle: &'a graft_git::repo::RepoHandle,
    pub(crate) table: &'a Arc<CommitTable>,
    pub(crate) current_ref: Option<&'a RefName>,
    pub(crate) default_chain: &'a HashSet<Oid>,
    pub(crate) tag_targets: &'a [(Vec<u8>, Oid)],
    pub(crate) repo_facts: &'a RepoFacts<'a>,
    pub(crate) policy: &'a Policy,
    pub(crate) from: i64,
    pub(crate) to: i64,
    pub(crate) buckets: usize,
}

/// One branch's row: its walk, its `merged_into` elbow, and (for the default
/// row only) the set of commits its walk visited, which the caller keeps as
/// `default_visited` for every later row's `merged_into` check.
#[allow(clippy::too_many_arguments)]
fn build_row(
    ctx: &RowContext<'_>,
    name: &RefName,
    tip: Oid,
    tip_date: i64,
    is_default: bool,
    default_row_index: Option<usize>,
    default_visited: &HashSet<Oid>,
    stop: &dyn Stop,
) -> Result<(MapBranch, HashSet<Oid>), MapError> {
    let row = walk_branch(
        ctx.handle,
        ctx.table,
        tip,
        ctx.buckets,
        ctx.from,
        ctx.to,
        is_default,
        ctx.default_chain,
        ctx.tag_targets,
        ctx.repo_facts,
        ctx.policy,
        stop,
    )?;
    let merged_into = if is_default {
        None
    } else if default_visited.contains(&tip) {
        default_row_index.map(|row_index| {
            (
                row_index,
                bucket_index(tip_date, ctx.from, ctx.to, ctx.buckets),
            )
        })
    } else {
        None
    };
    let branch = MapBranch {
        name: name.clone(),
        identity: BranchIdentity::of_name(name.shorthand()),
        tip,
        tip_date,
        counts: row.counts,
        is_current: ctx.current_ref == Some(name),
        is_default,
        merged_into,
        marks: row.marks,
        complete: row.complete,
    };
    let visited = if is_default {
        row.visited
    } else {
        HashSet::new()
    };
    Ok((branch, visited))
}

/// Every row, in order: [`build_row`] once per local branch, feeding
/// `default_visited` forward from the default row (which always sorts
/// before any row that could be merged into it) to every row after it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_rows(
    rows: &RowContext<'_>,
    locals: &[(RefName, Oid)],
    default_ref: Option<&RefName>,
    tip_dates: &HashMap<RefName, i64>,
    progress: &mut dyn FnMut(u32, u32),
    stop: &dyn Stop,
) -> Result<(Vec<MapBranch>, bool), MapError> {
    let total = u32::try_from(locals.len()).unwrap_or(u32::MAX);
    let mut result_branches = Vec::with_capacity(locals.len());
    let mut default_row_index: Option<usize> = None;
    let mut default_visited: HashSet<Oid> = HashSet::new();
    let mut overall_complete = true;

    for (index, (name, tip)) in locals.iter().enumerate() {
        if stop.should_stop() {
            return Err(MapError::Stopped);
        }
        let is_default = default_ref == Some(name);
        let tip_date = tip_dates[name];
        let (branch, visited) = build_row(
            rows,
            name,
            *tip,
            tip_date,
            is_default,
            default_row_index,
            &default_visited,
            stop,
        )?;
        overall_complete &= branch.complete;
        if is_default {
            default_row_index = Some(index);
            default_visited = visited;
        }
        result_branches.push(branch);
        progress(u32::try_from(index + 1).unwrap_or(u32::MAX), total);
    }

    Ok((result_branches, overall_complete))
}
