//! The first-launch line (m8-polish-and-ship/18d, rulings R7): for his
//! first [`SHOWN_FOR`] launches on a terminal, until he first opens the
//! palette, the graph's status row (when nothing else is there) and the
//! empty state's row above the rule say where help is.
//!
//! One file, `<config_dir>/graft/launches`: `<n>\n` (launches counted so
//! far) or `done\n` (he opened the palette; never rewritten). Only
//! `event_loop::places::drive` counts; headless `run`, the gallery and every
//! `Harness` never read or write it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Launches the line is shown on, counting the first as 1.
pub(crate) const SHOWN_FOR: u32 = 5;

/// What the file says once he has opened the palette.
const DONE: &str = "done";

/// Why the count could not be read or written.
#[derive(Debug)]
pub(crate) enum FirstRunError {
    Read { path: PathBuf, source: io::Error },
    Garbled { path: PathBuf },
    Write { path: PathBuf, source: io::Error },
}

impl FirstRunError {
    /// `first-launch: <path>: <reason>`, for the status row.
    pub(crate) fn words(&self) -> String {
        match self {
            Self::Read { path, source } => {
                format!("first-launch: {}: not read · {source}", path.display())
            }
            Self::Garbled { path } => {
                format!("first-launch: {}: not a count", path.display())
            }
            Self::Write { path, source } => {
                format!("first-launch: {}: not saved · {source}", path.display())
            }
        }
    }
}

/// The file the count lives in.
pub(crate) fn launches_file(config_dir: &Path) -> PathBuf {
    config_dir
        .join(crate::config::CONFIG_NAMES.dir())
        .join("launches")
}

/// What the file holds.
enum Count {
    Launches(u32),
    Done,
}

fn read(path: &Path) -> Result<Count, FirstRunError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Count::Launches(0)),
        Err(source) => {
            return Err(FirstRunError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let text = text.trim();
    if text == DONE {
        return Ok(Count::Done);
    }
    text.parse()
        .map(Count::Launches)
        .map_err(|_| FirstRunError::Garbled {
            path: path.to_path_buf(),
        })
}

/// Temp file, then rename, as `recent::remember` writes: never a half file.
fn write(path: &Path, text: &str) -> Result<(), FirstRunError> {
    let fail = |source| FirstRunError::Write {
        path: path.to_path_buf(),
        source,
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(fail)?;
    let temp = dir.join(format!(".launches.{}.tmp", std::process::id()));
    let written =
        std::fs::write(&temp, format!("{text}\n")).and_then(|()| std::fs::rename(&temp, path));
    if let Err(error) = written {
        // Take the temporary away; a failure to do so is folded into the
        // reason, so both are said.
        return Err(fail(match std::fs::remove_file(&temp) {
            Ok(()) => error,
            Err(remove) if remove.kind() == io::ErrorKind::NotFound => error,
            Err(remove) => io::Error::new(
                error.kind(),
                format!("{error}; and {} was left behind: {remove}", temp.display()),
            ),
        }));
    }
    Ok(())
}

/// One launch: read, add one, write. A missing file is 0. `Ok(true)` while
/// the count after adding is at most [`SHOWN_FOR`] and the file is not
/// `done`; `done` is never rewritten.
pub(crate) fn count_launch(config_dir: &Path) -> Result<bool, FirstRunError> {
    let path = launches_file(config_dir);
    match read(&path)? {
        Count::Done => Ok(false),
        Count::Launches(n) => {
            let n = n.saturating_add(1);
            write(&path, &n.to_string())?;
            Ok(n <= SHOWN_FOR)
        }
    }
}

/// He opened the palette: the line is not shown again, on any launch.
pub(crate) fn first_launch_done(config_dir: &Path) -> Result<(), FirstRunError> {
    write(&launches_file(config_dir), DONE)
}

/// This launch's line, counted (`drive`, once per process), and what to
/// say once when the count could not be read or written: the line is then
/// not shown, and the app runs on.
pub(crate) fn for_this_launch(config_dir: Option<&Path>) -> (FirstLaunch, Option<String>) {
    // No config folder: nowhere to count, and `recent` already says so.
    let Some(config_dir) = config_dir else {
        return (FirstLaunch::default(), None);
    };
    match count_launch(config_dir) {
        Ok(true) => (FirstLaunch::shown(config_dir.to_path_buf()), None),
        Ok(false) => (FirstLaunch::default(), None),
        Err(error) => (FirstLaunch::default(), Some(error.words())),
    }
}

/// The process's one answer to "is the line still shown". Held by
/// `event_loop::places::drive` only (m8/18e, review M1): every place's state
/// is seeded with [`FirstLaunch::line`], a plain value, and the first palette
/// open in any of them asks the shell to [`FirstLaunch::end`] it, so every
/// later place arrives without the line. No state holds this: `update` stays
/// pure. The default is never shown and never writes — headless `run`, the
/// gallery, every `Harness`.
#[derive(Clone, Debug, Default)]
pub(crate) struct FirstLaunch(Option<Arc<Shown>>);

/// A shown line: whether it still is, and where `done` goes.
#[derive(Debug)]
struct Shown {
    still: AtomicBool,
    config_dir: PathBuf,
}

impl FirstLaunch {
    /// Shown, ended by writing `done` under `config_dir`.
    pub(crate) fn shown(config_dir: PathBuf) -> Self {
        Self(Some(Arc::new(Shown {
            still: AtomicBool::new(true),
            config_dir,
        })))
    }

    /// `Some(config_dir)` while the line still shows this process: what
    /// every place's state is seeded with on arrival.
    pub(crate) fn line(&self) -> Option<PathBuf> {
        self.0
            .as_ref()
            .filter(|shown| shown.still.load(Ordering::Relaxed))
            .map(|shown| shown.config_dir.clone())
    }

    /// The palette opened in some place: no later place shows the line.
    /// The folder to write `done` under the first time only (`None` after,
    /// and for the default); the caller writes it with
    /// [`first_launch_done`] — the empty state's loop, or the repository's
    /// loop on `Command::FirstLaunchDone`.
    pub(crate) fn end(&self) -> Option<PathBuf> {
        self.0
            .as_ref()
            .filter(|shown| shown.still.swap(false, Ordering::Relaxed))
            .map(|shown| shown.config_dir.clone())
    }
}

/// A temp config dir whose `launches` holds `text` (absent for `None`); a
/// folder in its place for `Some("/")` (unreadable). For tests outside this
/// module that may not touch files themselves (`update`'s purity check).
#[cfg(test)]
pub(crate) fn test_config(text: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = launches_file(dir.path());
    match text {
        None => {}
        Some("/") => std::fs::create_dir_all(&path).expect("a folder"),
        Some(text) => {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, text).expect("seed");
        }
    }
    dir
}

/// What `launches` holds under `config_dir`; `None` when absent.
#[cfg(test)]
pub(crate) fn test_read(config_dir: &Path) -> Option<String> {
    std::fs::read_to_string(launches_file(config_dir)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded(text: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        if let Some(text) = text {
            let path = launches_file(dir.path());
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, text).expect("seed");
        }
        dir
    }

    fn file(dir: &tempfile::TempDir) -> String {
        std::fs::read_to_string(launches_file(dir.path())).expect("read back")
    }

    #[test]
    fn count_launch_table() {
        use std::os::unix::fs::PermissionsExt;

        for (before, shown, after) in [
            (None, true, "1\n"),
            (Some("4\n"), true, "5\n"),
            (Some("5\n"), false, "6\n"),
            (Some("done\n"), false, "done\n"),
        ] {
            let dir = seeded(before);
            assert_eq!(
                count_launch(dir.path()).expect("counts"),
                shown,
                "{before:?}"
            );
            assert_eq!(file(&dir), after, "{before:?}");
        }
        let dir = seeded(Some("x7\n"));
        assert!(matches!(
            count_launch(dir.path()),
            Err(FirstRunError::Garbled { .. })
        ));

        // A read-only `graft` folder: nothing to read (0), nothing writable.
        let dir = tempfile::tempdir().expect("temp dir");
        let graft = dir.path().join("graft");
        std::fs::create_dir(&graft).expect("mkdir");
        std::fs::set_permissions(&graft, std::fs::Permissions::from_mode(0o555)).expect("chmod");
        let error = count_launch(dir.path()).expect_err("unwritable");
        std::fs::set_permissions(&graft, std::fs::Permissions::from_mode(0o755))
            .expect("chmod back");
        assert!(matches!(error, FirstRunError::Write { .. }), "{error:?}");
        assert!(
            error.words().starts_with("first-launch: "),
            "{}",
            error.words()
        );
    }

    #[test]
    fn ending_asks_for_done_once_and_the_default_never_does() {
        let dir = seeded(Some("2\n"));
        let line = FirstLaunch::shown(dir.path().to_path_buf());
        let other_place = line.clone();
        assert_eq!(line.line().as_deref(), Some(dir.path()));
        let to_write = line.end().expect("the first end writes");
        first_launch_done(&to_write).expect("written");
        assert_eq!(other_place.line(), None);
        assert_eq!(file(&dir), "done\n");
        assert_eq!(other_place.end(), None, "once per process");

        let never = FirstLaunch::default();
        assert_eq!(never.line(), None);
        assert_eq!(never.end(), None);
    }
}
