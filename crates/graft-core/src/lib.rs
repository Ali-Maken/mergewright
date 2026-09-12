//! Plain data shared across Graft: commit and ref identity, events, the record
//! shapes other crates write and read, and the errors for `core`'s own parsing.
//! No behaviour and no algorithms live here — laying out a graph, parsing git
//! output, detecting a Mistake Radar pattern, replaying a journal entry and
//! probing a terminal each belong to the crate that owns that behaviour
//! (ADR-0007). A type earns a place here only when two crates that may not
//! depend on each other both need its shape. `core` depends on no other Graft
//! crate; everything else depends on it.

pub mod capabilities;
