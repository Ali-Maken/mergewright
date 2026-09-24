//! What a parsed [`ConflictFile`] answers and does: its bytes back
//! (`render`), a side taken per block, its blocks, and where each
//! projection's lines start.

use super::{BlockAt, BlockState, ConflictFile, NoSuchBlock, Projection, Region, Take};

/// Writes a marker and its label, terminator not included: `<<<<<<< HEAD`
/// for `Some(b"HEAD")`, `<<<<<<< ` for `Some(b"")`, `<<<<<<<` for `None`.
fn marker_line(out: &mut Vec<u8>, marker: &[u8], label: Option<&[u8]>) {
    out.extend_from_slice(marker);
    if let Some(label) = label {
        out.push(b' ');
        out.extend_from_slice(label);
    }
}

impl ConflictFile {
    /// The file's bytes: plain lines as they are, open blocks with their
    /// markers, resolved blocks as their taken lines.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for region in &self.regions {
            match region {
                Region::Plain(lines) => {
                    for line in lines {
                        out.extend_from_slice(&line.0);
                    }
                }
                // `reproduced` is deliberately absent: it is not in the file,
                // so attaching one cannot change what gets written back.
                Region::Conflict(block) => {
                    marker_line(&mut out, b"<<<<<<<", block.ours_label.as_deref());
                    out.extend_from_slice(block.ends.start.as_bytes());
                    for line in &block.ours {
                        out.extend_from_slice(&line.0);
                    }
                    if let Some(base) = &block.base {
                        marker_line(&mut out, b"|||||||", base.label.as_deref());
                        out.extend_from_slice(block.ends.base.as_bytes());
                        for line in &base.lines {
                            out.extend_from_slice(&line.0);
                        }
                    }
                    out.extend_from_slice(b"=======");
                    out.extend_from_slice(block.ends.separator.as_bytes());
                    for line in &block.theirs {
                        out.extend_from_slice(&line.0);
                    }
                    marker_line(&mut out, b">>>>>>>", block.theirs_label.as_deref());
                    out.extend_from_slice(block.ends.end.as_bytes());
                }
                Region::Resolved { block, take } => {
                    for line in block.taken(*take) {
                        out.extend_from_slice(&line.0);
                    }
                }
            }
        }
        out
    }

    /// Takes a side for the `block`-th block (counting `Conflict` and
    /// `Resolved` regions in order). Re-taking a resolved block replaces its
    /// take.
    pub fn take(&mut self, block: usize, take: Take) -> Result<(), NoSuchBlock> {
        let region = self
            .regions
            .iter_mut()
            .filter(|region| !matches!(region, Region::Plain(_)))
            .nth(block)
            .ok_or(NoSuchBlock(block))?;
        // A momentary placeholder so the block can move out of the old
        // region and into the new one without a clone.
        let previous = std::mem::replace(region, Region::Plain(Vec::new()));
        let inner = match previous {
            Region::Conflict(inner) | Region::Resolved { block: inner, .. } => inner,
            Region::Plain(_) => unreachable!("plain regions were filtered out"),
        };
        *region = Region::Resolved { block: inner, take };
        Ok(())
    }

    /// Every block, open or taken, with the line it starts on.
    #[must_use]
    pub fn blocks(&self) -> Vec<BlockAt> {
        let mut found = Vec::new();
        let mut line = 1usize;
        for region in &self.regions {
            let state = match region {
                Region::Plain(_) => None,
                Region::Conflict(_) => Some(BlockState::Open),
                Region::Resolved { take, .. } => Some(BlockState::Took(*take)),
            };
            if let Some(state) = state {
                found.push(BlockAt {
                    index: found.len(),
                    state,
                    first_line: line,
                });
            }
            line += region.line_count();
        }
        found
    }

    /// One entry per region: the 1-based line that region starts on when the
    /// file is read as projection `p`. Every projection numbers its own lines
    /// contiguously from 1, so the entries never skip a number.
    ///
    /// - [`Projection::Ours`] / [`Projection::Theirs`]: plain lines, plus
    ///   that side's lines of every block, open or taken.
    /// - [`Projection::Result`]: plain lines, a taken block's taken lines,
    ///   and an open block's ours lines (drawn as a placeholder).
    ///
    /// An empty region gets the number the next line would have.
    #[must_use]
    pub fn first_lines(&self, p: Projection) -> Vec<usize> {
        let mut line = 1usize;
        self.regions
            .iter()
            .map(|region| {
                let first = line;
                line += match (region, p) {
                    (Region::Plain(lines), _) => lines.len(),
                    // An open block's result is its ours lines (a placeholder).
                    (
                        Region::Conflict(block) | Region::Resolved { block, .. },
                        Projection::Ours,
                    )
                    | (Region::Conflict(block), Projection::Result) => block.ours.len(),
                    (
                        Region::Conflict(block) | Region::Resolved { block, .. },
                        Projection::Theirs,
                    ) => block.theirs.len(),
                    (Region::Resolved { block, take }, Projection::Result) => {
                        block.taken(*take).count()
                    }
                };
                first
            })
            .collect()
    }

    /// How many blocks are still open.
    #[must_use]
    pub fn open(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| matches!(region, Region::Conflict(_)))
            .count()
    }
}
