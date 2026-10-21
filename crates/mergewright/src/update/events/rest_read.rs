//! [`RestRead`]: the reads `update::settle` asks once the cursor rests —
//! never mid-burst, so a held `j` asks none of them.

/// One read asked at rest (`Command::AtRest`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestRead {
    /// Count the commits each folded merge brought in, for its `+N`
    /// (m8-polish-and-ship/17b, `jobs::fold_count`). One read per rest
    /// (ADR-0065 D2), asked once per merge per session.
    CountFolded {
        /// Every folding merge on screen with no count yet, top row first.
        merges: Vec<mergewright_core::oid::Oid>,
    },
    /// Read the focused branch's four answers and its route for the branch
    /// pane (ease-pass 09, `jobs::branch_status`). The job reads the
    /// branch's upstream itself.
    BranchStatus(Box<crate::update::branch_pane::StatusAsk>),
}
