//! The hard-coded 30-commit history the spike draws. Newest first, like
//! `git log`: row 0 is the tip of `main`.
//!
//! Each row carries hand-written drawing instructions — the spike tests
//! rendering and alignment, not the layout algorithm (DESIGN.md §3):
//!
//! - `to_node`: lanes whose edge enters the top of the row and ends at this
//!   commit's node (edges from its children; more than one at a fork point).
//! - `from_node`: lanes whose edge leaves the node to the bottom of the row
//!   (one per parent; more than one at a merge).
//! - `through`: lanes passing straight through this row.
//!
//! All text is ASCII on purpose: the text drawing code clips by bytes.

pub struct GraphRow {
    pub id: &'static str,
    pub msg: &'static str,
    /// Ref badges, `""` when none.
    pub refs: &'static str,
    /// The lane this commit's node sits in.
    pub lane: usize,
    pub to_node: &'static [usize],
    pub from_node: &'static [usize],
    pub through: &'static [usize],
}

/// Concurrently open lanes in this history (lane 0 = `main`).
pub const LANES: usize = 3;

/// Character columns the graph pane occupies: two per lane plus one spare.
pub const GRAPH_COLS: u16 = LANES as u16 * 2 + 1;

/// Per-lane RGBA colours, readable on light and dark backgrounds.
pub const LANE_COLORS: [[u8; 4]; LANES] = [
    [76, 158, 232, 255], // lane 0, main: blue
    [232, 148, 76, 255], // lane 1: orange
    [83, 197, 116, 255], // lane 2: green
];

const fn row(
    id: &'static str,
    msg: &'static str,
    refs: &'static str,
    lane: usize,
    to_node: &'static [usize],
    from_node: &'static [usize],
    through: &'static [usize],
) -> GraphRow {
    GraphRow {
        id,
        msg,
        refs,
        lane,
        to_node,
        from_node,
        through,
    }
}

/// The history: `main` on lane 0 with four merges, three fork points (one
/// where three branches meet), and up to three lanes open at once — then
/// the same shapes again as older history, with `ff…` ids and a `[v1.9]`
/// badge.
///
/// It repeats on purpose. The spike has to keep a scroll range on a
/// window taller than the dataset, and the alternative (capping the
/// visible rows) was the bug that showed `rows 1-24/30` in a window with
/// room for all 30. The seam at row 30 joins a root (no parents drawn) to
/// a tip (no children drawn), so no edge crosses it: this is a rendering
/// fixture, not a plausible history.
pub const ROWS: [GraphRow; 60] = [
    row(
        "9f3e2a1",
        "chore: release 2.0",
        "[main] [HEAD]",
        0,
        &[],
        &[0],
        &[],
    ),
    row("b7d01c4", "chore: fmt sweep", "", 0, &[0], &[0], &[]),
    row("4a99e02", "test: golden frames", "", 0, &[0], &[0], &[]),
    row("d2b6f77", "fix: strip cache leak", "", 0, &[0], &[0], &[]),
    row(
        "1e08c53",
        "Merge branch 'feature-d'",
        "",
        0,
        &[0],
        &[0, 1],
        &[],
    ),
    row("7c4413b", "d: encode placements", "", 1, &[1], &[1], &[0]),
    row("a15b9d0", "docs: user guide", "", 0, &[0], &[0], &[1]),
    row("f3caa08", "d: probe cell size", "", 1, &[1], &[1], &[0]),
    row(
        "08d17ee",
        "core: refactor renderer",
        "",
        0,
        &[0, 1],
        &[0],
        &[],
    ),
    row("63f0b2a", "perf: cache layout rows", "", 0, &[0], &[0], &[]),
    row(
        "e91ac45",
        "Merge branch 'feature-b'",
        "",
        0,
        &[0],
        &[0, 1],
        &[],
    ),
    row("2b7de91", "b: smooth scrolling", "", 1, &[1], &[1], &[0]),
    row("c8f302d", "fix: typo in help", "", 0, &[0], &[0], &[1]),
    row("5d6e8fa", "b: keybindings", "", 1, &[1], &[1], &[0]),
    row(
        "90a4bc7",
        "Merge branch 'feature-c'",
        "",
        0,
        &[0],
        &[0, 2],
        &[1],
    ),
    row("37e91d5", "c: lexer states", "", 2, &[2], &[2], &[0, 1]),
    row("ab205c9", "b: pane styles", "", 1, &[1], &[1], &[0, 2]),
    row("6ff37a2", "chore: bump deps", "", 0, &[0], &[0], &[1, 2]),
    row("d40b81e", "c: parser skeleton", "", 2, &[2], &[2], &[0, 1]),
    row("83c5f60", "b: layout pass", "", 1, &[1], &[1], &[0, 2]),
    row(
        "f7218d9",
        "fix: crash on empty repo",
        "",
        0,
        &[0, 1, 2],
        &[0],
        &[],
    ),
    row(
        "0b9ea34",
        "Merge branch 'feature-a'",
        "",
        0,
        &[0],
        &[0, 1],
        &[],
    ),
    row("94d0c7f", "a: public api", "", 1, &[1], &[1], &[0]),
    row("e56a1b8", "core: logging", "", 0, &[0], &[0], &[1]),
    row("1a8f4e3", "a: data model", "", 1, &[1], &[1], &[0]),
    row("78c2d90", "docs: readme", "", 0, &[0], &[0], &[1]),
    row("c31709a", "a: scaffold module", "", 1, &[1], &[1], &[0]),
    row("42e8b5d", "core: config loading", "", 0, &[0, 1], &[0], &[]),
    row("9d150cf", "core: project skeleton", "", 0, &[0], &[0], &[]),
    row("5b0a2e7", "init: empty repo", "", 0, &[0], &[], &[]),
    row("ff3e2a1", "chore: release 2.0", "[v1.9]", 0, &[], &[0], &[]),
    row("ffd01c4", "chore: fmt sweep", "", 0, &[0], &[0], &[]),
    row("ff99e02", "test: golden frames", "", 0, &[0], &[0], &[]),
    row("ffb6f77", "fix: strip cache leak", "", 0, &[0], &[0], &[]),
    row(
        "ff08c53",
        "Merge branch 'feature-d'",
        "",
        0,
        &[0],
        &[0, 1],
        &[],
    ),
    row("ff4413b", "d: encode placements", "", 1, &[1], &[1], &[0]),
    row("ff5b9d0", "docs: user guide", "", 0, &[0], &[0], &[1]),
    row("ffcaa08", "d: probe cell size", "", 1, &[1], &[1], &[0]),
    row(
        "ffd17ee",
        "core: refactor renderer",
        "",
        0,
        &[0, 1],
        &[0],
        &[],
    ),
    row("fff0b2a", "perf: cache layout rows", "", 0, &[0], &[0], &[]),
    row(
        "ff1ac45",
        "Merge branch 'feature-b'",
        "",
        0,
        &[0],
        &[0, 1],
        &[],
    ),
    row("ff7de91", "b: smooth scrolling", "", 1, &[1], &[1], &[0]),
    row("fff302d", "fix: typo in help", "", 0, &[0], &[0], &[1]),
    row("ff6e8fa", "b: keybindings", "", 1, &[1], &[1], &[0]),
    row(
        "ffa4bc7",
        "Merge branch 'feature-c'",
        "",
        0,
        &[0],
        &[0, 2],
        &[1],
    ),
    row("ffe91d5", "c: lexer states", "", 2, &[2], &[2], &[0, 1]),
    row("ff205c9", "b: pane styles", "", 1, &[1], &[1], &[0, 2]),
    row("fff37a2", "chore: bump deps", "", 0, &[0], &[0], &[1, 2]),
    row("ff0b81e", "c: parser skeleton", "", 2, &[2], &[2], &[0, 1]),
    row("ffc5f60", "b: layout pass", "", 1, &[1], &[1], &[0, 2]),
    row(
        "ff218d9",
        "fix: crash on empty repo",
        "",
        0,
        &[0, 1, 2],
        &[0],
        &[],
    ),
    row(
        "ff9ea34",
        "Merge branch 'feature-a'",
        "",
        0,
        &[0],
        &[0, 1],
        &[],
    ),
    row("ffd0c7f", "a: public api", "", 1, &[1], &[1], &[0]),
    row("ff6a1b8", "core: logging", "", 0, &[0], &[0], &[1]),
    row("ff8f4e3", "a: data model", "", 1, &[1], &[1], &[0]),
    row("ffc2d90", "docs: readme", "", 0, &[0], &[0], &[1]),
    row("ff1709a", "a: scaffold module", "", 1, &[1], &[1], &[0]),
    row("ffe8b5d", "core: config loading", "", 0, &[0, 1], &[0], &[]),
    row("ff150cf", "core: project skeleton", "", 0, &[0], &[0], &[]),
    row("ff0a2e7", "init: empty repo", "", 0, &[0], &[], &[]),
];
