//! The coverage matrix, `docs/shots/35-coverage-matrix.txt`: the gallery's
//! graph and details tables, then one table per acceptance walk saying which
//! capture files exist at which sizes. `gallery --matrix`, `m3_walk --matrix`
//! `m4_walk --matrix` and `m5_walk --matrix` all write it through
//! [`write_matrix`], so none can drop another's table (m4-remotes/07). The
//! M6 walk's table is in it too, and `m5_walk --matrix` also writes that table
//! alone to [`M6_MATRIX_PATH`] (m6-conflicts/07).
//!
//! The walks' capture stems are listed **here**, not read from their
//! scenarios: an example that writes the matrix then compiles no walk it does
//! not run. Each walk's `captures_are_named_keyed_and_listed_in_the_matrix`
//! test keeps its scenarios' stems and the list below equal.

use std::path::Path;

use crate::gallery::gallery_matrix;

/// Where the matrix is written, under the workspace root.
pub const MATRIX_PATH: &str = "docs/shots/35-coverage-matrix.txt";

/// The sizes the M3–M5 walks capture at.
pub const SIZES: [&str; 2] = ["80x24", "160x48"];

/// The sizes the M6 walk captures at (m6-conflicts/07): the resolver's
/// mockups are drawn at 120×40, and 80×24 is the floor a person meets.
/// The journeys' landings (m6-conflicts/21, 43) are also taken at 160×48,
/// at 187×50 (the user's own window, field notes) and at 200×50, where the
/// list column and the three panes appear; the earlier stems read `—` there.
pub const M6_SIZES: [&str; 5] = ["120x40", "80x24", "160x48", "187x50", "200x50"];

/// Where `m5_walk --matrix` writes the M6 table alone.
pub const M6_MATRIX_PATH: &str = ".scratch/m6-conflicts/matrix.txt";

/// The M3 walk's capture stems, sorted (`tests/it/m3_walk.rs`).
pub const M3_STEMS: [&str; 9] = [
    "m3-01-stage-file",
    "m3-02-stage-hunk",
    "m3-03-discard-confirm",
    "m3-04-commit-editor",
    "m3-05-commit-refused",
    "m3-06-stash-list",
    "m3-07-stash-prompt",
    "m3-08-journal-after-unstage",
    "m3-09-undo-drift-box",
];

/// The M4 walk's capture stems, sorted (`tests/it/m4_walk/`).
pub const M4_STEMS: [&str; 8] = [
    "m4-01-push-box-plain",
    "m4-02-push-box-lease",
    "m4-03-credential-masked",
    "m4-04-credential-username",
    "m4-05-progress-footer",
    "m4-06-rejected-sticky",
    "m4-07-journal-remote-rows",
    "m4-08-undo-after-push",
];

/// The M5 walk's capture stems, sorted (`tests/it/m5_walk/`).
pub const M5_STEMS: [&str; 8] = [
    "m5-01-preview-rebase",
    "m5-02-preview-merge",
    "m5-03-preview-reset-hard",
    "m5-04-time-machine",
    "m5-05-preview-too-small",
    "m5-06-stop-lands-in-resolver",
    "m5-07-branch-row-hints",
    "m5-11-diverged-pull-sticky",
];

/// The M6 walk's capture stems, sorted (`tests/it/m5_walk/resolving.rs`,
/// `resolved_operations.rs`, and the journeys' landings in
/// `conflict_journeys.rs`); their files live under `docs/shots/m6/`.
pub const M6_STEMS: [&str; 19] = [
    "m6-01-resolver-three",
    "m6-02-whole-file-card",
    "m6-03-card-large-file",
    "m6-04-card-binary",
    "m6-05-still-open-refusal",
    "m6-06-relaunch-sticky",
    "m6-07-all-resolved",
    "m6-08-no-more-conflicts",
    "m6-09-start-over-box",
    "m6-10-stop-lands-in-resolver",
    "m6-11-card-on-notes",
    "m6-12-after-t",
    "m6-13-after-ctrl-t",
    "m6-14-last-file-done",
    "m6-15-finish-row-after-esc",
    "m6-16-summary-expanded",
    "m6-17-after-commit",
    "m6-18-launch-shell-begun",
    "m6-19-after-esc-esc",
];

/// Every table, in the order the file has them.
#[must_use]
pub fn coverage_matrix(workspace: &Path) -> String {
    let mut out = gallery_matrix(workspace);
    out.push_str(&working_tree_matrix(workspace));
    out.push_str(&remotes_matrix(workspace));
    out.push_str(&history_matrix(workspace));
    out.push_str(&conflicts_matrix(workspace));
    out
}

/// Write the matrix under `workspace` and return it.
pub fn write_matrix(workspace: &Path) -> Result<String, String> {
    let matrix = coverage_matrix(workspace);
    let out = workspace.join(MATRIX_PATH);
    std::fs::write(&out, &matrix).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(matrix)
}

/// The working-tree captures' table: one row per stem, a column per size.
#[must_use]
pub fn working_tree_matrix(workspace: &Path) -> String {
    walk_table(
        "Working tree, undo and stash coverage: capture × size (m3-working-tree-undo-and-stash/07)",
        ".scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py",
        &M3_STEMS,
        workspace,
    )
}

/// The remote captures' table: one row per stem, a column per size.
#[must_use]
pub fn remotes_matrix(workspace: &Path) -> String {
    walk_table(
        "Remotes coverage: capture × size (m4-remotes/07)",
        ".scratch/m4-remotes/pty-walk-m4.py",
        &M4_STEMS,
        workspace,
    )
}

/// The history captures' table: one row per stem, a column per size.
#[must_use]
pub fn history_matrix(workspace: &Path) -> String {
    walk_table(
        "History and Ghost Preview coverage: capture × size (m5-history-and-ghost-preview/11)",
        ".scratch/m5-history-and-ghost-preview/pty-walk-m5.py",
        &M5_STEMS,
        workspace,
    )
}

/// The conflict captures' table: one row per stem, a column per size, the
/// files under `docs/shots/m6/`.
#[must_use]
pub fn conflicts_matrix(workspace: &Path) -> String {
    table(
        &Table {
            title: "Conflicts coverage: capture × size (m6-conflicts/07)",
            script: ".scratch/m6-conflicts/pty-walk-m6.py",
            dir: "docs/shots/m6",
            sizes: &M6_SIZES,
        },
        &M6_STEMS,
        workspace,
    )
}

/// Write the M6 table alone to [`M6_MATRIX_PATH`] and return it.
pub fn write_conflicts_matrix(workspace: &Path) -> Result<String, String> {
    let matrix = conflicts_matrix(workspace);
    let out = workspace.join(M6_MATRIX_PATH);
    std::fs::write(&out, &matrix).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(matrix)
}

/// What a walk's table is about: its title, the script that writes its
/// files, the directory they are in, the sizes.
struct Table<'a> {
    title: &'a str,
    script: &'a str,
    dir: &'a str,
    sizes: &'a [&'a str],
}

/// One M3–M5 walk's table: `C` where `docs/shots/<stem>-<size>.txt`
/// exists, `—` where it does not.
fn walk_table(title: &str, script: &str, stems: &[&str], workspace: &Path) -> String {
    table(
        &Table {
            title,
            script,
            dir: "docs/shots",
            sizes: &SIZES,
        },
        stems,
        workspace,
    )
}

/// One walk's table: `C` where `<dir>/<stem>-<size>.txt` exists, `—` where
/// it does not.
fn table(table: &Table<'_>, stems: &[&str], workspace: &Path) -> String {
    let Table {
        title,
        script,
        dir,
        sizes,
    } = *table;
    let width = stems
        .iter()
        .map(|stem| stem.len())
        .max()
        .unwrap_or(0)
        .max(7);
    let mut out = String::new();
    out.push_str(&format!("\n{title}\n"));
    out.push_str(&format!("Files {dir}/<stem>-<WxH>.txt from {script};\n"));
    out.push_str(
        "C the file exists, — it does not. Unicode tier only; colour is never captured.\n\n",
    );
    out.push_str(&format!("{:width$}", "capture"));
    for size in sizes {
        out.push_str(&format!("  {size:<8}"));
    }
    out.push('\n');
    for stem in stems {
        out.push_str(&format!("{stem:width$}"));
        for size in sizes {
            let exists = workspace.join(format!("{dir}/{stem}-{size}.txt")).is_file();
            out.push_str(&format!("  {:<8}", if exists { "C" } else { "—" }));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A walk's table marks exactly the files that exist, by stem and size.
    #[test]
    fn a_walk_table_marks_only_the_files_that_exist() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let shots = temp.path().join("docs/shots");
        std::fs::create_dir_all(&shots).expect("docs/shots");
        std::fs::write(shots.join("x-01-one-80x24.txt"), "").expect("a capture");
        let table = walk_table("T", "s.py", &["x-01-one", "x-02-two"], temp.path());
        let rows: Vec<&str> = table.lines().filter(|l| l.starts_with("x-")).collect();
        assert_eq!(
            rows,
            [
                "x-01-one  C         —       ",
                "x-02-two  —         —       "
            ]
        );
        assert!(table.starts_with("\nT\nFiles docs/shots/<stem>-<WxH>.txt from s.py;\n"));
    }

    /// The six tables, each present, in milestone order.
    fn six_tables_in_order(written: &str) {
        let titles: Vec<usize> = [
            "Graph coverage",
            "Details coverage",
            "Working tree, undo and stash coverage",
            "Remotes coverage",
            "History and Ghost Preview coverage",
            "Conflicts coverage",
        ]
        .iter()
        .map(|title| {
            written
                .find(title)
                .unwrap_or_else(|| panic!("no table {title:?}"))
        })
        .collect();
        assert!(
            titles.windows(2).all(|w| w[0] < w[1]),
            "tables in order: {titles:?}"
        );
    }

    /// `write_matrix` — the one call every `--matrix` command makes — writes
    /// all six tables to `MATRIX_PATH`, so no command can drop another's
    /// table (m4-remotes/07 review, M3).
    #[test]
    fn write_matrix_writes_all_six_tables_where_the_ticket_wants_them() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let shots = temp.path().join("docs/shots");
        std::fs::create_dir_all(&shots).expect("docs/shots");
        std::fs::write(shots.join("m3-01-stage-file-80x24.txt"), "").expect("an M3 capture");
        std::fs::write(shots.join("m4-08-undo-after-push-160x48.txt"), "").expect("an M4 capture");
        std::fs::write(shots.join("m5-04-time-machine-80x24.txt"), "").expect("an M5 capture");
        std::fs::create_dir_all(shots.join("m6")).expect("docs/shots/m6");
        std::fs::write(shots.join("m6/m6-06-relaunch-sticky-120x40.txt"), "")
            .expect("an M6 capture");

        let returned = write_matrix(temp.path()).expect("the matrix is written");
        let written =
            std::fs::read_to_string(temp.path().join(MATRIX_PATH)).expect("the file exists");
        assert_eq!(returned, written, "what it returns is what it wrote");

        six_tables_in_order(&written);
        let row = |stem: &str| {
            written
                .lines()
                .find(|line| line.starts_with(stem))
                .unwrap_or_else(|| panic!("no row {stem}"))
                .to_owned()
        };
        assert_eq!(
            row("m3-01-stage-file")
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>(),
            ["C", "—"]
        );
        assert_eq!(
            row("m4-08-undo-after-push")
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>(),
            ["—", "C"]
        );
        assert_eq!(
            row("m4-01-push-box-plain")
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>(),
            ["—", "—"]
        );
        assert_eq!(
            row("m5-04-time-machine")
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>(),
            ["C", "—"]
        );
        assert_eq!(
            row("m6-06-relaunch-sticky")
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>(),
            ["C", "—", "—", "—", "—"],
            "M6 reads docs/shots/m6/, 120x40 first"
        );
    }

    /// `write_conflicts_matrix` writes the M6 table alone where the ticket
    /// wants it (`--matrix`, m6-conflicts/07).
    #[test]
    fn the_m6_table_is_written_alone_to_the_scratch() {
        let temp = tempfile::tempdir().expect("a temp dir");
        std::fs::create_dir_all(temp.path().join(".scratch/m6-conflicts")).expect("the scratch");
        let returned = write_conflicts_matrix(temp.path()).expect("the table is written");
        let written =
            std::fs::read_to_string(temp.path().join(M6_MATRIX_PATH)).expect("the file exists");
        assert_eq!(returned, written);
        assert!(written.contains("Conflicts coverage"));
        assert!(!written.contains("History and Ghost Preview coverage"));
        assert!(written.contains("Files docs/shots/m6/<stem>-<WxH>.txt"));
        assert_eq!(
            written
                .lines()
                .filter(|line| line.starts_with("m6-"))
                .count(),
            M6_STEMS.len()
        );
    }

    /// The stem lists are sorted and distinct, so the tables read in order
    /// and a stem cannot be listed twice.
    #[test]
    fn stems_are_sorted_and_distinct() {
        for stems in [&M3_STEMS[..], &M4_STEMS[..], &M5_STEMS[..], &M6_STEMS[..]] {
            let mut sorted = stems.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted, stems);
        }
    }
}
