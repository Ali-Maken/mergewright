//! [`attach_base`]: each block of the file given its base, found by
//! aligning the file's blocks against git's own `diff3` output.

use std::fmt;

use super::{Base, ConflictFile, Line, Region, Reproduced};

/// A block whose `(ours, theirs)` lines no run of git's own output has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseMismatch {
    /// The first block (by [`ConflictFile::blocks`] index) with no match.
    pub block: usize,
}

impl fmt::Display for BaseMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never "after edits": a pristine file reaches here too when its
        // shape is one this alignment does not model.
        write!(
            f,
            "block {} matches nothing in git's own merge of the three stages, so it has no base",
            self.block
        )
    }
}

impl std::error::Error for BaseMismatch {}

/// Which side of a block a run is read on.
#[derive(Clone, Copy)]
enum Side {
    Ours,
    Theirs,
}

/// The lines a run of reproduced regions has on one side: plain lines as
/// they are, each block's lines for that side.
fn side_lines(run: &[Region], side: Side) -> impl Iterator<Item = &Line> {
    run.iter().flat_map(move |region| match region {
        Region::Plain(lines) => lines.iter(),
        Region::Conflict(block) | Region::Resolved { block, .. } => match side {
            Side::Ours => block.ours.iter(),
            Side::Theirs => block.theirs.iter(),
        },
    })
}

/// How many lines a run of regions has on one side.
fn side_count(run: &[Region], side: Side) -> usize {
    side_lines(run, side).count()
}

/// Where each conflict (open or taken) sits in a region list.
fn conflict_positions(regions: &[Region]) -> Vec<usize> {
    regions
        .iter()
        .enumerate()
        .filter(|(_, region)| !matches!(region, Region::Plain(_)))
        .map(|(at, _)| at)
        .collect()
}

/// The base of a run of reproduced regions: the base lines of each block with
/// the plain lines between them kept in place, labelled as the run's first
/// block was. `None` when a block in the run carries no base at all —
/// `merge-file --diff3` always writes one, so this is defensive.
fn run_base(run: &[Region]) -> Option<Base> {
    let mut lines = Vec::new();
    let mut label: Option<Option<Vec<u8>>> = None;
    for region in run {
        match region {
            Region::Plain(plain) => lines.extend(plain.iter().cloned()),
            Region::Conflict(block) | Region::Resolved { block, .. } => {
                let base = block.base.as_ref()?;
                lines.extend(base.lines.iter().cloned());
                if label.is_none() {
                    label = Some(base.label.clone());
                }
            }
        }
    }
    Some(Base {
        label: label?,
        lines,
    })
}

/// One file run matched to one reproduced run.
struct Aligned {
    /// How many of the file's blocks the match covers: 1 normally, more when
    /// git split one reproduced conflict into several.
    file_blocks: usize,
    /// The reproduced base for them.
    base: Option<Base>,
    /// The reproduced conflict number to resume the search at.
    next: usize,
}

/// Matches the file's blocks from `from_block` against the reproduced
/// conflicts from `from_repro`: the shortest run of reproduced regions
/// (growing first) that some run of file regions is byte-equal to on both
/// sides. Runs on either side start and end on a conflict and carry the plain
/// lines in between, so growing a run is exactly what git's joining and
/// splitting do to one region of the file.
fn find_alignment(
    file: &[Region],
    file_at: &[usize],
    from_block: usize,
    reproduced: &[Region],
    repro_at: &[usize],
    from_repro: usize,
) -> Option<Aligned> {
    let file_from = file_at[from_block];
    let rest = &file[file_from..];
    let (most_ours, most_theirs) = (side_count(rest, Side::Ours), side_count(rest, Side::Theirs));

    for (j, &repro_from) in repro_at.iter().enumerate().skip(from_repro) {
        for (k, &repro_to) in repro_at.iter().enumerate().skip(j) {
            let repro_run = &reproduced[repro_from..=repro_to];
            let (want_ours, want_theirs) = (
                side_count(repro_run, Side::Ours),
                side_count(repro_run, Side::Theirs),
            );
            // Both sides only grow with `k`: past what the rest of the file
            // could supply, this start is hopeless.
            if want_ours > most_ours || want_theirs > most_theirs {
                break;
            }
            for (p, &file_to) in file_at.iter().enumerate().skip(from_block) {
                let file_run = &file[file_from..=file_to];
                if side_count(file_run, Side::Ours) > want_ours
                    || side_count(file_run, Side::Theirs) > want_theirs
                {
                    break;
                }
                if side_lines(file_run, Side::Ours).eq(side_lines(repro_run, Side::Ours))
                    && side_lines(file_run, Side::Theirs).eq(side_lines(repro_run, Side::Theirs))
                {
                    return Some(Aligned {
                        file_blocks: p - from_block + 1,
                        base: run_base(repro_run),
                        next: k + 1,
                    });
                }
            }
        }
    }
    None
}

/// Gives each of `file`'s blocks the base git's `diff3` output has for it, as
/// [`Block::reproduced`], walking both in order and comparing content only.
///
/// **Nothing this writes is rendered**: `reproduced` is not part of the
/// file's bytes, so [`ConflictFile::render`], [`write_back`] and the line
/// numbers in [`ConflictFile::blocks`] are the same before and after. A block
/// the file itself gave a base (a `diff3`/`zdiff3` file) keeps that base
/// untouched, labels included.
///
/// git's `xdl_merge` at its default level both **joins** and **splits**, and
/// `--diff3` does neither (measured on git 2.54, 2026-09-22):
///
/// - Conflicts fewer than four unchanged lines apart are joined into one
///   block, so a file block can be *several* reproduced blocks with the plain
///   lines between them.
/// - A conflict whose two sides share interior lines is refined into several
///   blocks, so a file block can be a *fraction* of one reproduced block:
///   base `1…7` against ours `A b c d e f B` and theirs `X b c d e f Y` is
///   two blocks on disk (`A`/`X`, five plain lines, `B`/`Y`) and one block
///   with seven-line sides under `--diff3`.
///
/// So the match is between *runs*: the shortest run of reproduced regions
/// that some run of file regions is byte-equal to on both sides, each run
/// starting and ending on a conflict and carrying the plain lines between.
/// Where git split, every file block in the run is given the whole run's base
/// with [`Reproduced::shared`] set — git itself cannot say which of those
/// base lines belongs to which block, which is why `--diff3` refuses to
/// refine, so claiming otherwise would be a guess.
///
/// A reproduced conflict the file no longer has is skipped — a take written
/// to disk removes its block from the file while the stages, and so `diff3`,
/// still have it — but a file block is never given a base by position: the
/// whole call fails with the first block that matches nothing, and attaches
/// nothing. After a hand edit inside a block the answer is a mismatch, not a
/// guess. Returns how many blocks were given a base.
pub fn attach_base(file: &mut ConflictFile, diff3: &ConflictFile) -> Result<usize, BaseMismatch> {
    let file_at = conflict_positions(&file.regions);
    let repro_at = conflict_positions(&diff3.regions);

    // Decide first, attach second: a mismatch on the third block must leave
    // the first two as they were.
    let mut found: Vec<Option<Reproduced>> = Vec::with_capacity(file_at.len());
    let mut cursor = 0usize;
    while found.len() < file_at.len() {
        let index = found.len();
        let hit = find_alignment(
            &file.regions,
            &file_at,
            index,
            &diff3.regions,
            &repro_at,
            cursor,
        )
        .ok_or(BaseMismatch { block: index })?;
        let shared = hit.file_blocks > 1;
        for _ in 0..hit.file_blocks {
            found.push(hit.base.clone().map(|base| Reproduced { base, shared }));
        }
        cursor = hit.next;
    }

    let mut given = 0usize;
    let blocks = file.regions.iter_mut().filter_map(|region| match region {
        Region::Plain(_) => None,
        Region::Conflict(block) | Region::Resolved { block, .. } => Some(block),
    });
    for (block, reproduced) in blocks.zip(found) {
        if reproduced.is_some() {
            given += 1;
        }
        block.reproduced = reproduced;
    }
    Ok(given)
}
