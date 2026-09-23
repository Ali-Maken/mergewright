//! Write operations and the safety net around them: running an operation,
//! recording and replaying the undo journal, detecting Mistake Radar
//! patterns, and planning a Ghost Preview of what an operation would do
//! before it runs. The planning and detection halves are pure; the parts
//! that touch a repository go through `git`. Depends on `git` and `core`
//! only.

#![forbid(unsafe_code)]

pub mod apply;
pub mod conflict;
pub mod facts;
pub mod finish;
pub mod journal;
pub mod mistake_radar;
pub mod operation;
pub mod preview;
pub mod restore;
pub mod snapshot;
pub mod undo;
