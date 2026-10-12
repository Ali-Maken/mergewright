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

use graft_core::on_disk::Generation;

use super::folder::{ConfigDirMoved, config_names, decide, migrate_config_dir};
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
    /// A `[keys.<context>]` entry whose value is neither a key nor a list of
    /// keys (`merge = 3`), as its dotted path. The line is not applied.
    NotAKey {
        /// `keys.<context>.<verb>`.
        path: String,
    },
    /// A `[theme.<kind>] lane-N` line. Lane hues are fixed so no theme can
    /// make two adjacent lanes alike (PROMPT.md §5); the line is not applied.
    LaneColour {
        /// `theme.<kind>.lane-N`.
        path: String,
    },
    /// A `[theme.<kind>]` token whose value is not `"#RRGGBB"`. Not applied.
    BadColour {
        /// `theme.<kind>.<token>`.
        path: String,
        /// What he wrote: the text of a string, the TOML of anything else.
        value: String,
    },
    /// The config folder's move to its new name (ticket 26) left something
    /// to say: the old folder still beside the new one, or the move failed
    /// and the old one is still in use.
    ConfigFolder {
        /// What happened, in words.
        words: String,
    },
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
            ConfigWarning::NotAKey { path } => {
                write!(f, "`{path}` is not a key or a list of keys, ignored")
            }
            ConfigWarning::LaneColour { path } => {
                write!(f, "{path}: lane colours are fixed, not themeable")
            }
            ConfigWarning::BadColour { path, value } => {
                write!(f, "{path}: \"{value}\" is not a #RRGGBB colour")
            }
            ConfigWarning::ConfigFolder { words } => write!(f, "config folder: {words}"),
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
///
/// First moves the config folder to its new name, once (ticket 26,
/// `config::folder`), so this read is already of the new folder; a move that
/// could not finish keeps the old folder for this run and says why among the
/// warnings. Called once, at startup: the names it settles on hold for the
/// rest of the process (`recent`, `launches`).
pub fn load() -> Result<Loaded, ConfigError> {
    let (names, loaded) = load_settling(|name| env::var_os(name))?;
    decide(names);
    Ok(loaded)
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

/// Where `load` looks: `$XDG_CONFIG_HOME/tributary/config.toml`, falling
/// back to `$HOME/.config/tributary/config.toml` — the old folder name when
/// its move failed this run. `None` when neither is usable.
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
    config_home(read).map(|home| config_file(&home, config_names()))
}

/// The config home the environment names: `$XDG_CONFIG_HOME`, else
/// `$HOME/.config`, each only when absolute (see [`path_in_environment`]).
pub(super) fn config_home(read: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    read("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            read("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".config"))
        })
}

/// `config.toml` in the app's folder under `config_home`, by `names`.
fn config_file(config_home: &Path, names: Generation) -> PathBuf {
    config_home.join(names.dir()).join("config.toml")
}

/// The config folder's move under `config_home`, the way [`load`] makes it
/// (ticket 26): the names to use for the rest of the run, and what to say.
/// No config home, nothing to move — the new names, unused.
pub(super) fn settle_folder(config_home: Option<&Path>) -> (Generation, Option<ConfigWarning>) {
    // The move's source: the generation released just before `CURRENT`.
    // ADR-0057 2026-10-11 §8 (every earlier folder) is not built yet.
    let (Some(home), Some(from)) = (config_home, Generation::CURRENT.older().next()) else {
        return (Generation::CURRENT, None);
    };
    match migrate_config_dir(home, from) {
        Ok(ConfigDirMoved::Nothing | ConfigDirMoved::Moved { .. }) => (Generation::CURRENT, None),
        Ok(ConfigDirMoved::OlderWrote { old, kept_aside }) => (
            Generation::CURRENT,
            Some(ConfigWarning::ConfigFolder {
                words: match kept_aside {
                    Some(aside) => format!(
                        "an older version wrote {} · close it; kept in {}",
                        old.display(),
                        aside.display()
                    ),
                    None => format!("an older version wrote {} · close it", old.display()),
                },
            }),
        ),
        Ok(ConfigDirMoved::OldLeft { old, why }) => (
            Generation::CURRENT,
            Some(ConfigWarning::ConfigFolder {
                words: format!("moved; {} could not be removed: {why}", old.display()),
            }),
        ),
        Err(error) => (
            from,
            Some(ConfigWarning::ConfigFolder {
                words: format!("not moved to its new name, the old one stays in use: {error}"),
            }),
        ),
    }
}

/// [`load`] against an arbitrary environment, the names it settled on
/// returned rather than decided for the process — what a test drives.
pub(super) fn load_settling(
    read: impl Fn(&str) -> Option<OsString>,
) -> Result<(Generation, Loaded), ConfigError> {
    let home = config_home(read);
    let (names, moved) = settle_folder(home.as_deref());
    let mut loaded = load_at(home.map(|home| config_file(&home, names)))?;
    loaded.warnings.extend(moved);
    Ok((names, loaded))
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

    let mut config: Config = toml::from_str(text).map_err(fail)?;
    // Can't fail once the typed parse has accepted the document; propagated
    // rather than assumed, because "can't happen" is not a reason to panic.
    let table: toml::Table = toml::from_str(text).map_err(fail)?;

    let mut warnings = unknown_keys(&table);
    let (remaps, unread) = super::keys::remaps(table.get("keys"));
    config.keys.remaps = remaps;
    warnings.extend(unread);
    let (theme, unread) = super::theme::overrides(table.get("theme"));
    config.theme = theme;
    warnings.extend(unread);
    Ok(Loaded {
        config,
        path: None,
        warnings,
    })
}

/// Every key of the config file, as `(section, keys)`.
///
/// This is the only place the key names are written out for the warning pass;
/// the types above are the authority on what they mean. Adding a key means
/// adding it here too, or the user gets told their new setting is unknown —
/// which is the safe direction for the mistake to go.
pub(crate) const SCHEMA: &[(&str, &[&str])] = &[
    ("diff", &["context", "ignore_whitespace", "word_tint"]),
    ("graph", &["default_branch", "max_lanes", "renderer"]),
    (
        "radar",
        &[
            "foxtrot",
            "backward",
            "accidental-pull",
            "evil",
            "duplicate",
            "force-pushed",
            "risky-state",
        ],
    ),
    ("ui", &["icons", "overview", "theme"]),
];

/// What serde ignored, as warnings, in the file's key order.
fn unknown_keys(table: &toml::Table) -> Vec<ConfigWarning> {
    let mut warnings = Vec::new();

    for (name, value) in table {
        // `[keys]` holds remap sub-tables beside its one switch: what is
        // unknown or unreadable there is `keys::remaps`' to say.
        // `[theme.<kind>]` the same: `theme::overrides` says its own.
        if name == "keys" || name == "theme" {
            continue;
        }
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
