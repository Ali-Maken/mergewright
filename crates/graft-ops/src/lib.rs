//! Write operations and the safety net around them: running an operation,
//! recording and replaying the undo journal, detecting Mistake Radar
//! patterns, and planning a Ghost Preview of what an operation would do
//! before it runs. The planning and detection halves are pure; the parts
//! that touch a repository go through `git`. Depends on `git` and `core`
//! only.

#![forbid(unsafe_code)]

pub mod mistake_radar;
