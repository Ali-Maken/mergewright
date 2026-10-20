//! Lays out a corpus shape the way the app will, for the graph column's
//! snapshots and its example. Mirrors `mergewright-graph`'s own corpus test: refs
//! become seeds, the whole walk becomes rows.

use std::collections::HashMap;
use std::path::Path;

use mergewright_core::oid::Oid;
use mergewright_fixtures::Shape;
use mergewright_git::branches::Branches;
use mergewright_git::history::{CHUNK, Order, Walk};
use mergewright_git::repo::{HeadState, RefEntry, RefTarget, RepoHandle};
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::layout::layout;
use mergewright_graph::row::Row;
use mergewright_graph::seeds::{BranchSeed, Seeds};
use mergewright_ui::graph_column::{GraphColumn, GraphLine, NodeKind};

/// One shape, laid out.
pub struct Laid {
    pub rows: Vec<Row>,
    head: Option<Oid>,
    stash: Option<Oid>,
}

impl Laid {
    /// Every row drawn plainly, with `HEAD` and the stash marked.
    pub fn lines(&self) -> Vec<GraphLine<'_>> {
        self.rows
            .iter()
            .map(|row| {
                let mut line = GraphLine::plain(row);
                if Some(row.commit.id) == self.head {
                    line.node = NodeKind::Head;
                } else if Some(row.commit.id) == self.stash {
                    line.node = NodeKind::Stash;
                }
                line
            })
            .collect()
    }

    /// Cells wide enough for the widest row.
    pub fn width(&self) -> u16 {
        let columns = self
            .rows
            .iter()
            .map(|row| row.lanes.len().max(row.lane + 1))
            .max()
            .unwrap_or(0);
        GraphColumn::width_for(columns)
    }
}

/// Calls `visit` on every shape this machine can hold. The non-UTF-8 ref
/// name cannot exist on APFS; that skip is announced, any other one fails.
pub fn for_each_shape(mut visit: impl FnMut(Shape, &Path)) {
    for &shape in Shape::ALL {
        match mergewright_fixtures::try_borrow(shape) {
            Ok(dir) => visit(shape, &dir),
            Err(why) => {
                let reason = why.to_string();
                assert!(
                    reason.contains("not valid UTF-8"),
                    "{shape} unavailable for a reason that is not the known one: {reason}"
                );
                eprintln!("skipped: {shape} — {reason}");
            }
        }
    }
}

/// Reads and lays out one shape.
pub fn lay_out(shape: Shape, dir: &Path) -> Laid {
    let handle = RepoHandle::open(dir)
        .unwrap_or_else(|why| panic!("opening the {shape} fixture at {}: {why}", dir.display()));
    let refs = handle
        .references()
        .unwrap_or_else(|why| panic!("reading the refs of the {shape} fixture: {why}"));
    let branches = Branches::resolve(&handle, None)
        .unwrap_or_else(|why| panic!("resolving the branches of the {shape} fixture: {why}"));
    let tips: HashMap<&[u8], Oid> = refs
        .iter()
        .filter_map(|entry| match entry.target {
            RefTarget::Object(id) => Some((entry.name.as_bytes(), id)),
            RefTarget::Symbolic(_) => None,
        })
        .collect();

    let head = match handle
        .head()
        .unwrap_or_else(|why| panic!("reading HEAD of the {shape} fixture: {why}"))
    {
        HeadState::Branch(name) => tips.get(name.as_bytes()).copied(),
        HeadState::Detached(id) => Some(id),
        HeadState::Unborn(_) => None,
    };
    let stash = tips.get(b"refs/stash".as_slice()).copied();

    let table = mergewright_git::history::CommitTable::open(&handle)
        .unwrap_or_else(|why| panic!("opening the id table of the {shape} fixture: {why}"));
    let mut walk = Walk::all_refs(&handle, Order::Topological, std::sync::Arc::new(table))
        .unwrap_or_else(|why| panic!("starting the walk of the {shape} fixture: {why}"));
    let mut commits = Vec::new();
    loop {
        let chunk = walk
            .next_chunk(CHUNK, &|| false)
            .unwrap_or_else(|why| panic!("walking the {shape} fixture: {why}"));
        commits.extend(chunk.commits);
        if chunk.done {
            break;
        }
    }

    Laid {
        rows: layout(&commits, seeds(&refs, &branches, &tips)),
        head,
        stash,
    }
}

/// The default branch on lane 0, integration branches beside it, every local
/// and remote-tracking branch known by name.
fn seeds(refs: &[RefEntry], branches: &Branches, tips: &HashMap<&[u8], Oid>) -> Seeds {
    let seed = |name: &mergewright_core::ref_name::RefName| {
        tips.get(name.as_bytes()).map(|&tip| BranchSeed {
            tip,
            identity: BranchIdentity::of_name(name.shorthand()),
            follows: None,
        })
    };
    Seeds {
        default_branch: branches.default_branch().and_then(seed),
        integration_branches: branches.integration().iter().filter_map(seed).collect(),
        known_branches: refs
            .iter()
            .filter(|entry| {
                let name = entry.name.as_bytes();
                name.starts_with(b"refs/heads/") || name.starts_with(b"refs/remotes/")
            })
            .filter_map(|entry| seed(&entry.name))
            .collect(),
    }
}

/// A commit's short SHA and subject, for reading a drawing row by row.
pub fn caption(row: &Row) -> String {
    let sha: String = row.commit.id.to_string().chars().take(7).collect();
    format!("{sha}  {}", String::from_utf8_lossy(row.commit.subject()))
}
