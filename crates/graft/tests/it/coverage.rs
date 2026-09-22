//! The coverage matrix, `docs/shots/35-coverage-matrix.txt`: the gallery's
//! graph and details tables, then one table per acceptance walk saying which
//! capture files exist at which sizes. `gallery --matrix`, `m3_walk --matrix`
//! `m4_walk --matrix` and `m5_walk --matrix` all write it through
//! [`write_matrix`], so none can drop another's table (m4-remotes/07).
//!
//! The walks' capture stems are listed **here**, not read from their
//! scenarios: an example that writes the matrix then compiles no walk it does
//! not run. Each walk's `captures_are_named_keyed_and_listed_in_the_matrix`
//! test keeps its scenarios' stems and the list below equal.

use std::path::Path;

use crate::gallery::gallery_matrix;

/// Where the matrix is written, under the workspace root.
pub const MATRIX_PATH: &str = "docs/shots/35-coverage-matrix.txt";

/// The sizes every walk captures at.
pub const SIZES: [&str; 2] = ["80x24", "160x48"];

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
    "m5-06-banner-conflicts",
    "m5-07-branch-row-hints",
    "m5-11-diverged-pull-sticky",
];

/// Every table, in the order the file has them.
#[must_use]
pub fn coverage_matrix(workspace: &Path) -> String {
    let mut out = gallery_matrix(workspace);
    out.push_str(&working_tree_matrix(workspace));
    out.push_str(&remotes_matrix(workspace));
    out.push_str(&history_matrix(workspace));
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

/// One walk's table: `C` where `docs/shots/<stem>-<size>.txt` exists, `—`
/// where it does not.
fn walk_table(title: &str, script: &str, stems: &[&str], workspace: &Path) -> String {
    let width = stems
        .iter()
        .map(|stem| stem.len())
        .max()
        .unwrap_or(0)
        .max(7);
    let mut out = String::new();
    out.push_str(&format!("\n{title}\n"));
    out.push_str(&format!(
        "Files docs/shots/<stem>-<WxH>.txt from {script};\n"
    ));
    out.push_str(
        "C the file exists, — it does not. Unicode tier only; colour is never captured.\n\n",
    );
    out.push_str(&format!("{:width$}", "capture"));
    for size in SIZES {
        out.push_str(&format!("  {size:<8}"));
    }
    out.push('\n');
    for stem in stems {
        out.push_str(&format!("{stem:width$}"));
        for size in SIZES {
            let exists = workspace
                .join(format!("docs/shots/{stem}-{size}.txt"))
                .is_file();
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

    /// The five tables, each present, in milestone order.
    fn five_tables_in_order(written: &str) {
        let titles: Vec<usize> = [
            "Graph coverage",
            "Details coverage",
            "Working tree, undo and stash coverage",
            "Remotes coverage",
            "History and Ghost Preview coverage",
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
    /// all five tables to `MATRIX_PATH`, so no command can drop another's
    /// table (m4-remotes/07 review, M3).
    #[test]
    fn write_matrix_writes_all_five_tables_where_the_ticket_wants_them() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let shots = temp.path().join("docs/shots");
        std::fs::create_dir_all(&shots).expect("docs/shots");
        std::fs::write(shots.join("m3-01-stage-file-80x24.txt"), "").expect("an M3 capture");
        std::fs::write(shots.join("m4-08-undo-after-push-160x48.txt"), "").expect("an M4 capture");
        std::fs::write(shots.join("m5-04-time-machine-80x24.txt"), "").expect("an M5 capture");

        let returned = write_matrix(temp.path()).expect("the matrix is written");
        let written =
            std::fs::read_to_string(temp.path().join(MATRIX_PATH)).expect("the file exists");
        assert_eq!(returned, written, "what it returns is what it wrote");

        five_tables_in_order(&written);
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
    }

    /// The stem lists are sorted and distinct, so the tables read in order
    /// and a stem cannot be listed twice.
    #[test]
    fn stems_are_sorted_and_distinct() {
        for stems in [&M3_STEMS[..], &M4_STEMS[..], &M5_STEMS[..]] {
            let mut sorted = stems.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted, stems);
        }
    }
}
