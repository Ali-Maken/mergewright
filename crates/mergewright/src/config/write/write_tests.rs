//! The graph mode's write (ease-pass 06i): every byte but the key's kept,
//! refusals leave the file alone, atomic, through a symlink, permissions
//! kept. Each case on its own temp dir.

use std::fs;
use std::path::{Path, PathBuf};

use super::super::load_from;
use super::*;

const SHOWN: MergedBranches = MergedBranches::Shown;

fn file_with(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("config.toml");
    fs::write(&path, text).expect("written");
    (dir, path)
}

/// The file loads to `mode` with no warning.
fn loads_as(path: &Path, mode: MergedBranches) {
    let loaded = load_from(path).expect("loads");
    assert_eq!(loaded.warnings, Vec::new(), "{loaded:?}");
    let raw = loaded
        .config
        .graph
        .merged_branches
        .as_ref()
        .map(|raw| &raw.0);
    assert_eq!(MergedBranches::from_config(raw), Ok(mode));
}

/// Only the folder's own `config.toml` is left: no temp file.
fn only_the_file(dir: &Path) {
    let names: Vec<_> = fs::read_dir(dir)
        .expect("listed")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    assert!(
        names
            .iter()
            .all(|name| name == "config.toml" || name == "real.toml"),
        "{names:?}"
    );
}

fn kept(text: &str) -> String {
    let (dir, path) = file_with(text);
    keep_merged_branches(&path, SHOWN).expect("kept");
    loads_as(&path, SHOWN);
    only_the_file(dir.path());
    fs::read_to_string(&path).expect("read")
}

#[test]
fn a_missing_folder_and_file_are_created_holding_only_the_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("mergewright").join("config.toml");
    keep_merged_branches(&path, SHOWN).expect("kept");
    assert_eq!(
        fs::read_to_string(&path).expect("read"),
        "[graph]\nmerged_branches = \"shown\"\n"
    );
    loads_as(&path, SHOWN);
    only_the_file(&dir.path().join("mergewright"));
}

#[test]
fn no_graph_table_appends_one_after_every_original_byte() {
    let original =
        "# my config\n\n[ui]\ntheme = \"dark\" # by eye\n\n[keys.graph]\nmerge = \"M\"\n";
    let after = kept(original);
    assert!(after.starts_with(original), "{after}");
    assert!(
        after[original.len()..].contains("[graph]\nmerged_branches = \"shown\"\n"),
        "{after}"
    );
}

#[test]
fn a_graph_table_with_other_keys_gains_only_the_line() {
    let original = "[graph]\n# lanes\nmax_lanes = 12\n\n[ui]\ntheme = \"dark\"\n";
    let after = kept(original);
    assert_eq!(
        after,
        "[graph]\n# lanes\nmax_lanes = 12\nmerged_branches = \"shown\"\n\n[ui]\ntheme = \"dark\"\n"
    );
}

#[test]
fn an_existing_value_changes_alone_its_comment_kept_and_a_bad_one_replaced() {
    let original = "[graph]\nmerged_branches = \"folded\"   # tried it\nmax_lanes = 12\n";
    assert_eq!(
        kept(original),
        "[graph]\nmerged_branches = \"shown\"   # tried it\nmax_lanes = 12\n"
    );
    assert_eq!(
        kept("[graph]\nmerged_branches = \"open\"\n"),
        "[graph]\nmerged_branches = \"shown\"\n"
    );
}

#[test]
fn a_dotted_key_is_edited_in_place() {
    let original = "# top\ngraph.merged_branches = \"folded\"\n[ui]\ntheme = \"dark\"\n";
    assert_eq!(
        kept(original),
        "# top\ngraph.merged_branches = \"shown\"\n[ui]\ntheme = \"dark\"\n"
    );
}

#[test]
fn an_unparsable_file_and_a_graph_that_isnt_a_table_are_refused_untouched() {
    for (text, words) in [
        (
            "[graph]\nmerged_branches = \n",
            "config.toml no longer reads (line 2, column",
        ),
        (
            "graph = 3\n",
            "graph in config.toml isn't a table · left as it is",
        ),
    ] {
        let (dir, path) = file_with(text);
        let error = keep_merged_branches(&path, SHOWN).expect_err("refused");
        assert!(error.to_string().starts_with(words), "{error}");
        assert_eq!(fs::read_to_string(&path).expect("read"), text);
        only_the_file(dir.path());
    }
}

#[cfg(unix)]
#[test]
fn a_read_only_folder_is_an_io_error_and_the_file_is_unchanged() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, path) = file_with("[ui]\n");
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).expect("chmod");
    let result = keep_merged_branches(&path, SHOWN);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).expect("chmod back");
    let error = result.expect_err("refused");
    assert!(matches!(error, KeepError::Io { .. }), "{error:?}");
    assert!(error.to_string().starts_with("can't write "), "{error}");
    assert_eq!(fs::read_to_string(&path).expect("read"), "[ui]\n");
    only_the_file(dir.path());
}

#[cfg(unix)]
#[test]
fn a_symlink_stays_a_link_its_target_updated_and_0600_stays_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let real = dir.path().join("real.toml");
    fs::write(&real, "[ui]\n").expect("written");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).expect("chmod");
    let link = dir.path().join("config.toml");
    std::os::unix::fs::symlink(&real, &link).expect("linked");
    keep_merged_branches(&link, SHOWN).expect("kept");
    assert!(
        fs::symlink_metadata(&link)
            .expect("meta")
            .file_type()
            .is_symlink()
    );
    let after = fs::read_to_string(&real).expect("read");
    assert!(after.starts_with("[ui]\n"), "{after}");
    assert!(
        after.ends_with("[graph]\nmerged_branches = \"shown\"\n"),
        "{after}"
    );
    loads_as(&link, SHOWN);
    let mode = fs::metadata(&real).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    only_the_file(dir.path());
}

#[test]
fn the_loop_writes_the_file_read_else_the_default_else_no_folder() {
    let (_dir, read) = file_with("");
    let (_other, default) = file_with("");
    assert_eq!(
        keep_in(Some(&read), Some(default.clone()), SHOWN).expect("kept"),
        read
    );
    loads_as(&read, SHOWN);
    assert_eq!(fs::read_to_string(&default).expect("read"), "");
    assert_eq!(
        keep_in(None, Some(default.clone()), SHOWN).expect("kept"),
        default
    );
    loads_as(&default, SHOWN);
    let error = keep_in(None, None, SHOWN).expect_err("nowhere");
    assert!(matches!(error, KeepError::NoFolder), "{error:?}");
    assert_eq!(error.to_string(), "no config folder could be found");
}
