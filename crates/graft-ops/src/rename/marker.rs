//! The marker, `<common_dir>/<CURRENT dir>/migrated`: absent (earlier
//! names), or one line, `switched <source dir>` (new names in force, moved
//! from that generation, old leftovers maybe on disk) or `clean` (moved,
//! nothing old left).
//!
//! The source is recorded, never inferred from which earlier markers are
//! left (review m8-48 Major 1): the stray pass removes those markers, so a
//! pass cut short would otherwise read a newer generation as scratch and
//! remove what an older build wrote there unread.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use graft_core::on_disk::{Generation, MIGRATED_MARKER};

use super::MigrateError;

/// What the marker says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Marker {
    /// Switched from this generation. `None` for a bare `switched`, which
    /// ticket 48's builds wrote before the source was recorded: the source
    /// is then unknown, and no generation is taken for scratch.
    Switched(Option<Generation>),
    Clean,
}

impl Marker {
    fn line(self) -> String {
        match self {
            Marker::Switched(Some(from)) => format!("switched {}\n", from.dir()),
            Marker::Switched(None) => "switched\n".to_owned(),
            Marker::Clean => "clean\n".to_owned(),
        }
    }

    /// The marker `line` spells, if it is one.
    fn parse(line: &str) -> Option<Self> {
        match line.split_once(' ') {
            None if line == "switched" => Some(Marker::Switched(None)),
            None if line == "clean" => Some(Marker::Clean),
            Some(("switched", dir)) => Generation::CURRENT
                .older()
                .find(|generation| generation.dir() == dir)
                .map(|from| Marker::Switched(Some(from))),
            _ => None,
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
        Ok(text) => match Marker::parse(text.trim()) {
            Some(marker) => Ok(Some(marker)),
            None => Err(MigrateError::Files {
                path,
                words: format!("unknown marker {:?}", text.trim()),
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
