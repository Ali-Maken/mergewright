//! The one key the app writes into his config file: `[graph]
//! merged_branches`, when he picks a graph mode in the `?` list (ease-pass
//! 06i). Every other byte of the file is kept — comments, blank lines, key
//! order, other tables — and the write is atomic: a temp file in the same
//! folder, renamed over the real one. The loop runs it (`Command::KeepGraphMode`),
//! as it runs `SaveLayout`; `update` never touches a file.

use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table, TableLike, Value};

use super::types::MergedBranches;

/// Why the mode was not kept. The file is as it was in every case.
#[derive(Debug)]
pub enum KeepError {
    /// The file no longer parses (his edit mid-run): where, 1-based.
    Unparsable {
        /// The line the parser stopped on.
        line: usize,
        /// The column on that line.
        column: usize,
    },
    /// `graph` in the file is something other than a table.
    NotATable,
    /// No config folder could be named (no file was read, and
    /// `default_path()` gave none).
    NoFolder,
    /// Reading or writing the file failed.
    Io {
        /// The file it was writing.
        path: PathBuf,
        /// What the system said.
        error: io::Error,
    },
}

impl fmt::Display for KeepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unparsable { line, column } => write!(
                f,
                "config.toml no longer reads (line {line}, column {column}) · left as it is"
            ),
            Self::NotATable => f.write_str("graph in config.toml isn't a table · left as it is"),
            Self::NoFolder => f.write_str("no config folder could be found"),
            Self::Io { path, error } => write!(f, "can't write {}: {error}", path.display()),
        }
    }
}

/// The loop's arm: keep `mode` in the file that was read at startup
/// (`loaded`), else in the file `load` would read (`default`), and say
/// which. Neither → [`KeepError::NoFolder`].
pub fn keep_in(
    loaded: Option<&Path>,
    default: Option<PathBuf>,
    mode: MergedBranches,
) -> Result<PathBuf, KeepError> {
    let path = loaded
        .map(Path::to_path_buf)
        .or(default)
        .ok_or(KeepError::NoFolder)?;
    keep_merged_branches(&path, mode)?;
    Ok(path)
}

/// Sets `[graph] merged_branches` in the config file at `path`, every other
/// byte kept. Creates the file and folder if missing; refuses a file that
/// no longer parses or whose `graph` isn't a table; writes through a
/// symlink to its target; atomic (temp in the same folder, then rename);
/// keeps permissions.
pub fn keep_merged_branches(path: &Path, mode: MergedBranches) -> Result<(), KeepError> {
    let io = |error| KeepError::Io {
        path: path.to_path_buf(),
        error,
    };
    // A dotfiles link stays a link: the file it points at is replaced.
    let target = match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => fs::canonicalize(path).map_err(io)?,
        _ => path.to_path_buf(),
    };
    // Read fresh, not the startup copy: edits made while running are kept.
    let (text, permissions) = match fs::read_to_string(&target) {
        Ok(text) => {
            let permissions = fs::metadata(&target).map_err(io)?.permissions();
            (text, Some(permissions))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (String::new(), None),
        Err(error) => return Err(io(error)),
    };
    let edited = with_mode(&text, mode)?;
    let folder = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(folder).map_err(io)?;
    // Dropped on any failure below, the temp file is removed with it.
    let mut temp = tempfile::NamedTempFile::new_in(folder).map_err(io)?;
    temp.write_all(edited.as_bytes()).map_err(io)?;
    if let Some(permissions) = permissions {
        fs::set_permissions(temp.path(), permissions).map_err(io)?;
    }
    temp.as_file().sync_all().map_err(io)?;
    temp.persist(&target).map_err(|failed| io(failed.error))?;
    Ok(())
}

/// `text` with `[graph] merged_branches` set to `mode`.
fn with_mode(text: &str, mode: MergedBranches) -> Result<String, KeepError> {
    let mut doc: DocumentMut = text.parse().map_err(|error: toml_edit::TomlError| {
        let at = error.span().map_or(0, |span| span.start);
        let (line, column) = line_and_column(text, at);
        KeepError::Unparsable { line, column }
    })?;
    let value = mode.as_config();
    let key = "merged_branches";
    match doc.get_mut("graph") {
        None => {
            let mut graph = Table::new();
            graph.insert(key, toml_edit::value(value));
            doc.insert("graph", Item::Table(graph));
        }
        Some(Item::Table(graph)) => set(graph, key, value),
        Some(Item::Value(Value::InlineTable(graph))) => set(graph, key, value),
        Some(_) => return Err(KeepError::NotATable),
    }
    Ok(doc.to_string())
}

/// Set `key` in `table` to `value`, keeping the old value's spacing and
/// trailing comment when there was one; added at the table's end when not.
fn set(table: &mut dyn TableLike, key: &str, value: &str) {
    let mut new = Value::from(value);
    match table.get_mut(key) {
        Some(Item::Value(old)) => {
            *new.decor_mut() = old.decor().clone();
            *old = new;
        }
        Some(item) => *item = Item::Value(new),
        None => {
            table.insert(key, Item::Value(new));
        }
    }
}

/// The 1-based line and column of byte `at` in `text`.
fn line_and_column(text: &str, at: usize) -> (usize, usize) {
    let before = text.get(..at).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (line, column)
}

#[cfg(test)]
mod write_tests;
