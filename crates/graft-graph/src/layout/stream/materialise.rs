//! Turning laid-out shapes back into rows, and the exact-read helpers the
//! rest of the module builds on.

use std::collections::HashMap;

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;

use crate::layout::source::CommitSource;
use crate::row::{MergeNames, Row, RowShape};

use super::{StreamError, Work};

/// Turns laid-out shapes into rows: reads their commits, names the merge
/// lanes they draw (reading a merge only when it lies above the batch), and
/// assembles each.
pub(super) fn materialise<S: CommitSource>(
    source: &mut S,
    work: &mut Work,
    shapes: Vec<RowShape>,
) -> Result<Vec<Row>, StreamError<S::Error>> {
    let handles: Vec<CommitHandle> = shapes.iter().map(|shape| shape.commit).collect();
    let commits = read_exactly(source, &handles)?;
    work.hydrated += commits.len();
    // `gather`'s reader borrows the source mutably, so the ids it asks for —
    // only merge parents, for the short-sha fallback — are resolved first.
    let parent_ids: HashMap<CommitHandle, Oid> = shapes
        .iter()
        .flat_map(|shape| shape.lanes.iter().flatten())
        .filter_map(|identity| match *identity {
            crate::lane::LaneIdentity::Merge { parent, .. } => Some((parent, source.id_of(parent))),
            crate::lane::LaneIdentity::Known(_) => None,
        })
        .collect();
    let names = MergeNames::gather(
        &shapes,
        &commits,
        |merges| read_exactly(source, merges),
        |parent| {
            *parent_ids
                .get(&parent)
                .unwrap_or_else(|| unreachable!("{parent:?} is a merge parent of these shapes"))
        },
    )?;
    work.merge_reads += names.reads();
    let id_of = |handle| source.id_of(handle);
    shapes
        .into_iter()
        .zip(commits)
        .map(|(shape, commit)| Ok(Row::assemble(shape, commit, &names, &id_of)?))
        .collect()
}

/// The lengths of the chunks `rows` rows fill, `chunk_rows` at a time.
pub(super) fn rows_per_chunk(rows: usize, chunk_rows: usize) -> impl Iterator<Item = usize> {
    (0..rows.div_ceil(chunk_rows)).map(move |chunk| (rows - chunk * chunk_rows).min(chunk_rows))
}

/// Reads `handles` from `source` and checks it returned exactly them, in
/// order.
fn read_exactly<S: CommitSource>(
    source: &mut S,
    handles: &[CommitHandle],
) -> Result<Vec<Commit>, StreamError<S::Error>> {
    let commits = source.read(handles).map_err(StreamError::Source)?;
    if commits.len() != handles.len() {
        return Err(StreamError::ShortRead {
            asked: handles.len(),
            got: commits.len(),
        });
    }
    for (&handle, commit) in handles.iter().zip(&commits) {
        let asked = source.id_of(handle);
        if commit.id != asked {
            return Err(StreamError::WrongCommit {
                asked,
                got: commit.id,
            });
        }
    }
    Ok(commits)
}

/// Re-serves the links of `handles` from `source` and checks it returned
/// exactly them, in order.
pub(super) fn links_exactly<S: CommitSource>(
    source: &mut S,
    work: &mut Work,
    handles: &[CommitHandle],
) -> Result<Vec<Link>, StreamError<S::Error>> {
    let links = source.links(handles).map_err(StreamError::Source)?;
    work.commits_reread += links.len();
    work.links_read += links.len();
    if links.len() != handles.len() {
        return Err(StreamError::ShortRead {
            asked: handles.len(),
            got: links.len(),
        });
    }
    for (&handle, link) in handles.iter().zip(&links) {
        if link.commit != handle {
            return Err(StreamError::WrongCommit {
                asked: source.id_of(handle),
                got: source.id_of(link.commit),
            });
        }
    }
    Ok(links)
}
