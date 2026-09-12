//! The layout of every fixture shape, drawn as text and snapshotted.
//!
//! This is the one place the engine meets real repositories. Each shape in
//! the corpus is read through `graft-git` (a dev-dependency here — not an
//! ADR-0006 crate edge; `graft-graph` itself still depends on `graft-core`
//! alone), laid out one commit at a time, and drawn as a small grid: `*`
//! the node, `|` an edge passing straight through, `/` an edge bending in
//! from the right to end at the node, `\` a new edge leaving the node to
//! the right. Beside it: the short SHA, the name of the lane the row draws
//! in, and the subject.
//!
//! The snapshots are the reviewable form of DESIGN.md §3.2 on the shapes
//! PROMPT.md §1.2 names: read them, do not just re-accept them.

use std::collections::HashMap;
use std::path::Path;

use graft_core::commit::Commit;
use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_git::branches::Branches;
use graft_git::history::{CHUNK, Order, Walk};
use graft_git::repo::{RefEntry, RefTarget, RepoHandle};
use graft_graph::identity::BranchIdentity;
use graft_graph::lane::Lane;
use graft_graph::layout::{EmittedOrder, LayoutCursor, layout};
use graft_graph::merge_name::recover_branch_name;
use graft_graph::row::Row;
use graft_graph::seeds::{BranchSeed, Seeds};

const HEADS: &[u8] = b"refs/heads/";
const REMOTES: &[u8] = b"refs/remotes/";

#[test]
fn every_shape_lays_out_as_snapshotted() {
    for_each_shape(|shape, dir| {
        let text = picture(shape, dir);
        insta::assert_snapshot!(shape.name(), text);
    });
}

/// Calls `visit` on every corpus shape this machine can hold.
///
/// The one shape a machine may legitimately not be able to hold is the
/// non-UTF-8 ref name (a ref name is a file name, and APFS insists on
/// UTF-8). A skip must say so, never pass quietly.
pub(super) fn for_each_shape(mut visit: impl FnMut(Shape, &Path)) {
    for &shape in Shape::ALL {
        match graft_fixtures::try_borrow(shape) {
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

/// Opens a fixture and builds the seeds the app would from its refs.
pub(super) fn open_with_seeds(shape: Shape, dir: &Path) -> (RepoHandle, Seeds) {
    let handle = RepoHandle::open(dir)
        .unwrap_or_else(|why| panic!("opening the {shape} fixture at {}: {why}", dir.display()));
    let refs = handle
        .references()
        .unwrap_or_else(|why| panic!("reading the refs of the {shape} fixture: {why}"));
    let branches = Branches::resolve(&handle, None)
        .unwrap_or_else(|why| panic!("resolving the branches of the {shape} fixture: {why}"));
    let seeds = seeds_for(&refs, &branches);
    (handle, seeds)
}

/// Reads one shape, lays it out, and draws it.
fn picture(shape: Shape, dir: &Path) -> String {
    let handle = RepoHandle::open(dir)
        .unwrap_or_else(|why| panic!("opening the {shape} fixture at {}: {why}", dir.display()));
    let refs = handle
        .references()
        .unwrap_or_else(|why| panic!("reading the refs of the {shape} fixture: {why}"));
    let branches = Branches::resolve(&handle, None)
        .unwrap_or_else(|why| panic!("resolving the branches of the {shape} fixture: {why}"));
    let seeds = seeds_for(&refs, &branches);
    let (commits, walked) = walk_everything(&handle, shape);

    // The one-call convenience and the resumable cursor must agree on real
    // history, not just on the generated kind.
    let rows = layout(&commits, seeds.clone());
    let drawn = draw(&commits, seeds, &labels(&refs, &commits));
    assert_eq!(
        rows, drawn.rows,
        "{shape}: layout() and LayoutCursor disagree"
    );
    for (row, commit) in rows.iter().zip(&commits) {
        let targets: Vec<Oid> = row.outgoing.iter().map(|&(_, target)| target).collect();
        assert_eq!(
            targets, commit.parents,
            "{shape}: every parent edge is drawn, in order"
        );
    }

    let mut text = format!(
        "shape: {shape}\norder: requested {:?}, in force {:?}{}\n",
        walked.requested,
        walked.in_force,
        if walked.provisional {
            " (provisional)"
        } else {
            ""
        },
    );
    text.push_str(&format!(
        "default: {}\nintegration: [{}]\ncommits: {}, columns: {}\n\n",
        branches
            .default_branch()
            .map_or_else(|| "none".to_owned(), |name| lossy(name.shorthand())),
        branches
            .integration()
            .iter()
            .map(|name| lossy(name.shorthand()))
            .collect::<Vec<_>>()
            .join(", "),
        commits.len(),
        drawn.width,
    ));
    text.push_str(&drawn.lines.join("\n"));
    text.push('\n');
    text
}

/// What the walk reported about itself.
pub(super) struct Walked {
    requested: Order,
    in_force: Order,
    pub(super) provisional: bool,
}

/// Every commit the refs reach, in the order the graph is drawn in.
///
/// Topological order is requested; the corpus has no commit-graph file, so
/// the walk serves commit-date order instead and says so, and the header
/// of each snapshot records it. For this corpus that is still a valid
/// topological order — its timestamps strictly increase — so the layout
/// never meets a parent before its child here. Should a shape ever change
/// that, the snapshot will show the lane closing early rather than hide it.
pub(super) fn walk_everything(handle: &RepoHandle, shape: Shape) -> (Vec<Commit>, Walked) {
    let mut walk = Walk::all_refs(handle, Order::Topological)
        .unwrap_or_else(|why| panic!("starting the walk of the {shape} fixture: {why}"));
    let mut commits = Vec::new();
    loop {
        let chunk = walk
            .next_chunk(CHUNK, &|| false)
            .unwrap_or_else(|why| panic!("walking the {shape} fixture: {why}"));
        assert!(!chunk.stopped, "nothing asked the walk to stop");
        commits.extend(chunk.commits);
        if chunk.done {
            break;
        }
    }
    let walked = Walked {
        requested: walk.requested_order(),
        in_force: walk.order_in_force(),
        provisional: walk.is_provisional(),
    };
    (commits, walked)
}

/// The seeds the app would build from these refs: the default branch on
/// lane 0, the integration branches next to it, and every local and
/// remote-tracking branch known — a remote tip ahead of its local branch is
/// a tip in its own right, and its lane should carry the remote's name, not
/// a short SHA.
fn seeds_for(refs: &[RefEntry], branches: &Branches) -> Seeds {
    let tips: HashMap<&[u8], Oid> = refs
        .iter()
        .filter_map(|entry| match entry.target {
            RefTarget::Object(id) => Some((entry.name.as_bytes(), id)),
            // A symbolic branch aliases another ref, whose own entry carries
            // the tip; nothing is lost by leaving the alias out.
            RefTarget::Symbolic(_) => None,
        })
        .collect();
    let seed = |name: &graft_core::ref_name::RefName| {
        tips.get(name.as_bytes()).map(|&tip| BranchSeed {
            tip,
            identity: BranchIdentity::of_name(name.shorthand()),
        })
    };
    Seeds {
        default_branch: branches.default_branch().and_then(seed),
        integration_branches: branches.integration().iter().filter_map(seed).collect(),
        known_branches: refs
            .iter()
            .filter(|entry| {
                let name = entry.name.as_bytes();
                name.starts_with(HEADS) || name.starts_with(REMOTES)
            })
            .filter_map(|entry| seed(&entry.name))
            .collect(),
    }
}

/// A name for every identity a lane could carry here: each ref's shorthand,
/// each branch name a merge subject gives away, and each commit's short SHA.
fn labels(refs: &[RefEntry], commits: &[Commit]) -> HashMap<BranchIdentity, String> {
    let mut table = HashMap::new();
    for entry in refs {
        let short = entry.name.shorthand();
        table.insert(BranchIdentity::of_name(short), lossy(short));
    }
    for commit in commits {
        if let Some(name) = recover_branch_name(commit.subject()) {
            table.insert(BranchIdentity::of_name(&name), lossy(&name));
        }
        table.insert(
            BranchIdentity::of_short_sha(commit.id),
            short_sha(commit.id),
        );
    }
    table
}

/// The finished drawing, plus the rows it was drawn from.
struct Drawing {
    rows: Vec<Row>,
    width: usize,
    lines: Vec<String>,
}

/// Lays the commits out one at a time, reading the open lanes before and
/// after each so every row can be labelled with the lane it draws in.
fn draw(commits: &[Commit], seeds: Seeds, labels: &HashMap<BranchIdentity, String>) -> Drawing {
    let mut cursor = LayoutCursor::new(seeds);
    let mut order = EmittedOrder::new();
    let mut steps = Vec::with_capacity(commits.len());
    for commit in commits {
        let before = cursor.active_lanes().to_vec();
        let row = cursor
            .extend(std::slice::from_ref(commit), &mut order)
            .remove(0);
        let after = cursor.active_lanes().to_vec();
        steps.push((row, before, after));
    }
    // Columns are only ever appended, so the final count is the widest row.
    let width = cursor.active_lanes().len();

    let lines = steps
        .iter()
        .map(|(row, before, after)| row_line(row, before, after, width, labels))
        .collect();
    let rows = steps.into_iter().map(|(row, _, _)| row).collect();
    Drawing { rows, width, lines }
}

/// One row: the grid, then `sha7  lane-label  subject`.
fn row_line(
    row: &Row,
    before: &[Option<Lane>],
    after: &[Option<Lane>],
    width: usize,
    labels: &HashMap<BranchIdentity, String>,
) -> String {
    let opened: Vec<usize> = row
        .outgoing
        .iter()
        .map(|&(column, _)| column)
        .filter(|&column| column != row.lane)
        .collect();
    let mut grid = String::with_capacity(width * 2);
    for column in 0..width {
        let glyph = if column == row.lane {
            '*'
        } else if row.incoming.contains(&column) {
            '/'
        } else if opened.contains(&column) {
            '\\'
        } else if before.get(column).is_some_and(Option::is_some) {
            '|'
        } else {
            ' '
        };
        grid.push(glyph);
        grid.push(' ');
    }

    // The lane a row draws in is the one that was waiting for it, or — for
    // a fresh tip — the one it just opened. A root that is also a tip opens
    // and closes its lane inside the row, so neither view shows it.
    let lane = open_at(before, row.lane).or_else(|| open_at(after, row.lane));
    let label = lane.map_or("?", |lane| {
        labels
            .get(&lane.identity)
            .map_or("(unnamed)", String::as_str)
    });

    format!(
        "{grid} {}  {label:<18} {}",
        short_sha(row.commit.id),
        lossy(row.commit.subject())
    )
}

/// The lane open at `column`, if any.
fn open_at(lanes: &[Option<Lane>], column: usize) -> Option<&Lane> {
    lanes.get(column).and_then(Option::as_ref)
}

fn short_sha(id: Oid) -> String {
    id.to_string().chars().take(7).collect()
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
