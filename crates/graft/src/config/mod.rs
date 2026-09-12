//! The config file: `${XDG_CONFIG_HOME:-~/.config}/graft/config.toml`.
//!
//! One call — [`load`] — turns whatever is (or isn't) on disk into a typed
//! [`Config`] plus the warnings worth telling the user about. Config lives in
//! `app` because `app` is where detection and preference meet: [`load`] only
//! reports what the user asked for, and ticket 11 combines that with the
//! detected `Capabilities` (DESIGN.md §4.2).
//!
//! ```text
//! let loaded = config::load()?;              // or load_from(&path) in tests
//! let theme = ThemeKind::select(loaded.config.ui.theme.overridden(), hint);
//! let tier = loaded.config.graph.renderer.overridden().unwrap_or(detected);
//! ```
//!
//! # Three outcomes, none of them silent
//!
//! - **No file.** Defaults, nothing said. Running without a config file is the
//!   normal case, not a problem to report.
//! - **A file Graft can't use** — unreadable, not TOML, or a key whose value
//!   isn't one of the accepted spellings. That is a [`ConfigError`] naming the
//!   file, the line and the problem, and the caller exits on it. A half-valid
//!   file never quietly becomes defaults: a user who wrote `theme = "blu"` and
//!   got the dark theme anyway would have no way to find out why.
//! - **A key Graft doesn't know.** Tolerated, and returned as a
//!   [`ConfigWarning`] rather than an error, so a config file written for a
//!   later version still starts today's binary. Typos land here too, which is
//!   exactly why they are reported instead of ignored.
//!
//! # Which keys exist
//!
//! Only what M0 needs. Every value is a fixed set of spellings, and each set
//! matches the domain type it feeds — [`ThemeChoice`] the three shipped themes
//! of [`graft_ui::theme`], [`RendererChoice`] the three rendering tiers of
//! [`graft_core::capabilities`] — checked by tests in `tests.rs` rather than by
//! eye. Later milestones add keys as features need them; `[keys]` (remapping,
//! DESIGN.md §2.2) is the next one due, and until it exists it loads as a
//! warning rather than an error.

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use graft_core::capabilities::RenderingTier;
use graft_ui::theme::ThemeKind;
use serde::Deserialize;

/// Everything Graft reads from `config.toml`.
///
/// Every field has a default, so this is also the configuration of a machine
/// with no config file at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: Ui,
    pub graph: Graph,
}

/// `[ui]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Ui {
    pub theme: ThemeChoice,
    pub icons: IconSet,
}

/// `[graph]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Graph {
    pub renderer: RendererChoice,
}

/// `ui.theme`: which of the three shipped themes to use, or `auto`.
///
/// This is a *preference*, not a theme: `auto` has no [`ThemeKind`] to name
/// because the answer comes from the terminal's background. [`Self::overridden`]
/// is the whole bridge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    /// Follow the terminal's background colour (`OSC 11`), dark if it says
    /// nothing.
    #[default]
    Auto,
    Dark,
    Light,
    /// Reachable only from here: no terminal background implies it
    /// ([`ThemeKind::HighContrast`]).
    HighContrast,
}

impl ThemeChoice {
    /// The override to hand [`ThemeKind::select`] — `None` for `auto`.
    pub const fn overridden(self) -> Option<ThemeKind> {
        match self {
            ThemeChoice::Auto => None,
            ThemeChoice::Dark => Some(ThemeKind::Dark),
            ThemeChoice::Light => Some(ThemeKind::Light),
            ThemeChoice::HighContrast => Some(ThemeKind::HighContrast),
        }
    }
}

/// `graph.renderer`: which rendering tier to draw the graph with, or `auto`.
///
/// An explicit tier always beats detection, including upwards: forcing `pixel`
/// on a terminal that never acknowledged the graphics query still attempts it
/// (DESIGN.md §4.2), because detection can only prove a terminal answered, not
/// that it can't draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererChoice {
    /// Use whatever capability detection found.
    #[default]
    Auto,
    Pixel,
    Unicode,
    Ascii,
}

impl RendererChoice {
    /// The tier the user forced, or `None` to use the detected one.
    pub const fn overridden(self) -> Option<RenderingTier> {
        match self {
            RendererChoice::Auto => None,
            RendererChoice::Pixel => Some(RenderingTier::Pixel),
            RendererChoice::Unicode => Some(RenderingTier::Unicode),
            RendererChoice::Ascii => Some(RenderingTier::Ascii),
        }
    }
}

/// `ui.icons`: which glyph set the interface draws with.
///
/// Never detected. There is no query for "is this a Nerd Font", so guessing
/// wrong would show every user a screen of replacement boxes; opting in is the
/// only safe direction (PROMPT.md §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconSet {
    /// Unicode every font has. The shipped default, designed to look
    /// deliberate on its own.
    #[default]
    Plain,
    /// Nerd Font glyphs, for users who have a patched font.
    Nerd,
}

/// What [`load`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loaded {
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
    UnknownKey { path: String },
    /// Neither `XDG_CONFIG_HOME` nor `HOME` names a directory to look in, so
    /// no config file could be read at all. Rare — a stripped environment, a
    /// daemon, a container — and worth saying out loud, because every setting
    /// the user wrote is being ignored.
    NoConfigHome,
}

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
#[derive(Debug)]
pub enum ConfigError {
    /// The file exists but could not be read — permissions, a directory in its
    /// place, an I/O failure. A missing file is not an error and never reaches
    /// here.
    Read { path: PathBuf, source: io::Error },
    /// The file is not valid TOML, or a value is not one of the spellings its
    /// key accepts.
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(
                    f,
                    "couldn't read the config file {}: {source}",
                    path.display()
                )
            }
            // toml's own `Display` is the good part here: it quotes the
            // offending line, points at the column and says what it expected.
            // It ends in a newline, which a one-line error message shouldn't.
            ConfigError::Parse { path, source } => {
                write!(f, "{}:\n{}", path.display(), source.to_string().trim_end())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
        }
    }
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
fn load_at(path: Option<PathBuf>) -> Result<Loaded, ConfigError> {
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
fn path_in_environment(read: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
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
const SCHEMA: &[(&str, &[&str])] = &[("graph", &["renderer"]), ("ui", &["icons", "theme"])];

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
