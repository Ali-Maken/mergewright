//! Write operations and the safety net around them: running an operation,
//! recording and replaying the undo journal, detecting Mistake Radar
//! patterns, and planning a Ghost Preview of what an operation would do
//! before it runs. The planning and detection halves are pure; the parts
//! that touch a repository go through `git`. Depends on `git` and `core`
//! only.

#![forbid(unsafe_code)]

pub mod absorb;
pub mod apply;
mod branch_config;
pub mod clash;
pub mod conflict;
pub mod facts;
pub mod finish;
pub mod in_the_way;
pub mod init;
pub mod journal;
pub mod journal_list;
pub mod keys;
pub mod matcher;
pub mod merge_state;
pub mod mistake_radar;
mod objects;
pub mod operation;
pub mod preview;
pub mod restore;
pub mod rewrite;
pub mod snapshot;
pub mod submodule;
pub mod switch_list;
pub mod undo;

pub use apply::EditsAt;
