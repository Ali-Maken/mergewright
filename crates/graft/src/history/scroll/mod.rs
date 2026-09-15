//! Moving through history on screen: the cursor, the viewport, and asking the
//! walk for the rows they need.
//!
//! All of it is plain arithmetic on [`History`], so `update` stays pure: a
//! move returns the [`RowRequest`] it needs sent, if any, and the answer comes
//! back later through [`History::apply`].
//!
//! Split in two: [`apply`] takes updates in and drives the cursor and view;
//! [`query`] reads the window on hand and asks the walk for more of it.

mod apply;
mod query;

/// A cursor movement the graph pane understands (DESIGN.md §2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Down this many rows (`j`, `↓`).
    Down(usize),
    /// Up this many rows (`k`, `↑`).
    Up(usize),
    /// A screen down, keeping the cursor's place on screen (`PgDn`).
    PageDown,
    /// A screen up, keeping the cursor's place on screen (`PgUp`).
    PageUp,
    /// The first row (`g`).
    Top,
    /// The last row (`G`). On a walk not yet laid out to its end, a request
    /// that walks there while the viewport stays put.
    Bottom,
}
