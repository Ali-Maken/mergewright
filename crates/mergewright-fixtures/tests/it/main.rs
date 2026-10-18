//! `mergewright-fixtures`'s integration tests, in one binary (PROMPT.md §7.1).

mod corpus;
#[cfg(feature = "heavy-tests")]
mod heavy;
#[cfg(feature = "heavy-tests")]
mod team;
