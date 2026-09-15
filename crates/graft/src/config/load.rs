//! Reading a config file off disk into a [`Loaded`].
//!
//! Split from `config/mod.rs` (m1-the-graph/72c) to keep files under the
//! 400-line rule; [`super::types`] has the typed shapes this parses into.

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::types::Config;

/// What [`load`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loaded {
    /// The settings in effect: the file's values over the defaults.
    pub config: Config,
    /// The file the values came from, or `None` when no file was read — either
    /// because there isn't one or because there is nowhere to look.
    pub path: Option<PathBuf>,
    /// Everything worth mentioning about a file Graft nonetheless loaded, in
    /// key order (not file order — TOML's table is sorted). Empty is the
    /// normal case.
    pub warnings: Vec<ConfigWarning>,
}

/// Something worth telling the user about a config file Graft nonetheless
/// loaded.
///
/// Warnings never stop startup. M0 collects them; later milestones surface
/// them in the diagnostics view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigWarning {
    /// A key Graft has no meaning for, as a dotted path (`ui.icns`). Either a
    /// typo, which does nothing, or a key from a newer version, which also
    /// does nothing here.
    UnknownKey {
        /// The dotted path of the key nobody recognised.
        path: String,
    },
    /// Neither `XDG_CONFIG_HOME` nor `HOME` names a directory to look in, so
    /// no config file could be read at all. Rare — a stripped environment, a
    /// daemon, a container — and worth saying out loud, because every setting
    /// the user wrote is being ignored.
    NoConfigHome,
}

/// A warning is not a failure — nothing was prevented, and nobody can `?` on
/// one — so this is a plain `Display` rather than a `thiserror` error type.
impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigWarning::UnknownKey { path } => {
                write!(f, "unknown config key `{path}`, ignored")
            }
            ConfigWarning::NoConfigHome => write!(
                f,
                "neither XDG_CONFIG_HOME nor HOME is set, so no config file was read"
            ),
        }
    }
}

/// Why a config file could not be used.
///
/// Both variants are fatal by design: the caller prints this and exits before
/// the terminal session starts, so there is no half-initialised terminal to
/// restore (ticket 11).
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file exists but could not be read — permissions, a directory in its
    /// place, an I/O failure. A missing file is not an error and never reaches
    /// here.
    #[error("couldn't read the config file {}: {source}", path.display())]
    Read {
        /// The file Graft tried to read.
        path: PathBuf,
        /// Why it could not be.
        source: io::Error,
    },
    /// The file is not valid TOML, or a value is not one of the spellings its
    /// key accepts.
    // toml's own `Display` is the good part here: it quotes the offending
    // line, points at the column and says what it expected. It ends in a
    // newline, which a one-line error message shouldn't.
    #[error("{}:\n{}", path.display(), source.to_string().trim_end())]
    Parse {
        /// The file that would not parse.
        path: PathBuf,
        /// Where it went wrong, with the line and column toml reported.
        source: toml::de::Error,
    },
}

/// Load the user's config from the standard path.
///
/// Returns defaults if there is no file there. Tests use [`load_from`] with a
/// path of their own; nothing here ever reads a real home directory in a test.
pub fn load() -> Result<Loaded, ConfigError> {
    load_at(default_path())
}

/// Load a config file from an explicit path, for tests and for a future
/// `--config` flag.
pub fn load_from(path: &Path) -> Result<Loaded, ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        // No config file is the common case and says nothing about the user's
        // intent, so it is not a warning either.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Loaded::default()),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let mut loaded = parse(path, &text)?;
    loaded.path = Some(path.to_path_buf());
    Ok(loaded)
}

/// Where `load` looks: `$XDG_CONFIG_HOME/graft/config.toml`, falling back to
/// `$HOME/.config/graft/config.toml`. `None` when neither is usable.
pub fn default_path() -> Option<PathBuf> {
    path_in_environment(|name| env::var_os(name))
}

/// `load` once the path decision is made, so a test can drive the
/// no-config-home branch without touching the process environment.
pub(super) fn load_at(path: Option<PathBuf>) -> Result<Loaded, ConfigError> {
    match path {
        Some(path) => load_from(&path),
        None => Ok(Loaded {
            warnings: vec![ConfigWarning::NoConfigHome],
            ..Loaded::default()
        }),
    }
}

/// [`default_path`] against an arbitrary environment.
///
/// A relative `XDG_CONFIG_HOME` is ignored rather than resolved: the XDG base
/// directory spec says a non-absolute value must be treated as unset, and
/// resolving it against the current directory would put a user's config
/// somewhere different every time they `cd`.
pub(super) fn path_in_environment(read: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let config_home = read("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            read("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".config"))
        })?;

    Some(config_home.join("graft").join("config.toml"))
}

/// The whole file, in two passes over the same text.
///
/// serde reads the keys Graft knows and ignores the rest; a second pass over
/// the raw table finds exactly what it ignored. Doing it the other way round —
/// `deny_unknown_fields` — would make tomorrow's config key today's fatal
/// error.
fn parse(path: &Path, text: &str) -> Result<Loaded, ConfigError> {
    let fail = |source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    };

    let config: Config = toml::from_str(text).map_err(fail)?;
    // Can't fail once the typed parse has accepted the document; propagated
    // rather than assumed, because "can't happen" is not a reason to panic.
    let table: toml::Table = toml::from_str(text).map_err(fail)?;

    Ok(Loaded {
        config,
        path: None,
        warnings: unknown_keys(&table),
    })
}

/// Every key of the config file, as `(section, keys)`.
///
/// This is the only place the key names are written out for the warning pass;
/// the types above are the authority on what they mean. Adding a key means
/// adding it here too, or the user gets told their new setting is unknown —
/// which is the safe direction for the mistake to go.
const SCHEMA: &[(&str, &[&str])] = &[
    ("graph", &["default_branch", "max_lanes", "renderer"]),
    ("ui", &["icons", "theme"]),
];

/// What serde ignored, as warnings, in the file's key order.
fn unknown_keys(table: &toml::Table) -> Vec<ConfigWarning> {
    let mut warnings = Vec::new();

    for (name, value) in table {
        let Some((_, known)) = SCHEMA.iter().find(|(section, _)| section == name) else {
            warnings.push(ConfigWarning::UnknownKey { path: name.clone() });
            continue;
        };

        // A known section is always a table here: the typed parse has already
        // accepted this document, and it would have rejected `ui = "dark"`.
        let known: BTreeSet<&str> = known.iter().copied().collect();
        let entries = value.as_table().into_iter().flatten();
        for key in entries.map(|(key, _)| key) {
            if !known.contains(key.as_str()) {
                warnings.push(ConfigWarning::UnknownKey {
                    path: format!("{name}.{key}"),
                });
            }
        }
    }

    warnings
}
