//! The marker, `<common_dir>/tributary/migrated`: absent (old names), or one
//! line, `switched` (new names in force, old leftovers maybe on disk) or
//! `clean` (moved, nothing old left).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use graft_core::on_disk::{Generation, MIGRATED_MARKER};

use super::MigrateError;

/// What the marker says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Marker {
    Switched,
    Clean,
}

impl Marker {
    fn line(self) -> &'static str {
        match self {
            Marker::Switched => "switched\n",
            Marker::Clean => "clean\n",
        }
    }
}

fn path(common: &Path) -> PathBuf {
    common.join(Generation::CURRENT.dir()).join(MIGRATED_MARKER)
}

/// The marker, `None` when there is none. One read, no spawn.
pub(super) fn read(common: &Path) -> Result<Option<Marker>, MigrateError> {
    let path = path(common);
    match fs::read_to_string(&path) {
        Ok(text) => match text.trim() {
            "switched" => Ok(Some(Marker::Switched)),
            "clean" => Ok(Some(Marker::Clean)),
            other => Err(MigrateError::Files {
                path,
                words: format!("unknown marker {other:?}"),
            }),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(MigrateError::files(&path, &error)),
    }
}

/// Write the marker whole or not at all: a temporary beside it, synced,
/// renamed over it, the folder synced.
pub(super) fn write(common: &Path, marker: Marker) -> Result<(), MigrateError> {
    let path = path(common);
    let temporary = path.with_extension("tmp");
    let fail = |error: io::Error| MigrateError::files(&path, &error);
    let mut file = File::create(&temporary).map_err(fail)?;
    file.write_all(marker.line().as_bytes()).map_err(fail)?;
    file.sync_all().map_err(fail)?;
    fs::rename(&temporary, &path).map_err(fail)?;
    if let Some(folder) = path.parent() {
        File::open(folder)
            .and_then(|folder| folder.sync_all())
            .map_err(fail)?;
    }
    Ok(())
}
