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

mod keys;
mod load;
#[cfg(test)]
mod tests;
mod types;

pub(crate) use keys::keymap_from;
pub use load::{ConfigError, ConfigWarning, Loaded, default_path, load, load_from};
pub use types::{
    Config, DEFAULT_MAX_LANES, Diff, Graph, IconSet, Keys, Radar, RadarSetting, RendererChoice,
    ThemeChoice, Ui,
};

#[cfg(test)]
use load::{load_at, path_in_environment};
