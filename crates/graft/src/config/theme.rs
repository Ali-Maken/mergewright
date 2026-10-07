//! `[theme.<kind>] <token> = "#RRGGBB"` (m8-polish-and-ship/16): his
//! colours over a built-in theme, one key at a time.
//!
//! Read off the raw table like `[keys]`'s remaps, so the typed parse and the
//! overrides come from the same text. What a colour *does* is
//! `graft_ui::theme`'s: [`Theme::with_overrides`] applies it, and
//! [`contrast_misses`] says which floor it misses.
//!
//! [`Theme::with_overrides`]: graft_ui::theme::Theme::with_overrides

use graft_core::capabilities::Rgb;
use graft_ui::theme::{ThemeKind, Token, contrast_misses};

use super::load::ConfigWarning;

/// His `[theme.dark]`, `[theme.light]` and `[theme.high-contrast]` lines,
/// each as `(token, colour)` in key order (the `toml` table is sorted).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeOverrides {
    /// `[theme.dark]`.
    pub dark: Vec<(Token, Rgb)>,
    /// `[theme.light]`.
    pub light: Vec<(Token, Rgb)>,
    /// `[theme.high-contrast]` (or `high_contrast`).
    pub high_contrast: Vec<(Token, Rgb)>,
}

impl ThemeOverrides {
    /// The overrides for one theme, for `Theme::with_overrides`.
    #[must_use]
    pub fn for_kind(&self, kind: ThemeKind) -> &[(Token, Rgb)] {
        match kind {
            ThemeKind::Dark => &self.dark,
            ThemeKind::Light => &self.light,
            ThemeKind::HighContrast => &self.high_contrast,
        }
    }

    fn for_kind_mut(&mut self, kind: ThemeKind) -> &mut Vec<(Token, Rgb)> {
        match kind {
            ThemeKind::Dark => &mut self.dark,
            ThemeKind::Light => &mut self.light,
            ThemeKind::HighContrast => &mut self.high_contrast,
        }
    }
}

/// The kinds a `[theme.<kind>]` table may name. `high_contrast` beside
/// `ThemeKind::name`'s `high-contrast`, the spelling `ui.theme` takes: both
/// are what a person writes.
fn kind_named(name: &str) -> Option<ThemeKind> {
    match name {
        "dark" => Some(ThemeKind::Dark),
        "light" => Some(ThemeKind::Light),
        "high-contrast" | "high_contrast" => Some(ThemeKind::HighContrast),
        _ => None,
    }
}

/// What `[theme]` held, and a warning per line not applied.
pub(super) fn overrides(theme: Option<&toml::Value>) -> (ThemeOverrides, Vec<ConfigWarning>) {
    let mut overrides = ThemeOverrides::default();
    let mut warnings = Vec::new();
    let Some(theme) = theme else {
        return (overrides, warnings);
    };
    let Some(table) = theme.as_table() else {
        warnings.push(ConfigWarning::UnknownKey {
            path: "theme".to_owned(),
        });
        return (overrides, warnings);
    };
    for (name, value) in table {
        let (Some(kind), Some(entries)) = (kind_named(name), value.as_table()) else {
            warnings.push(ConfigWarning::UnknownKey {
                path: format!("theme.{name}"),
            });
            continue;
        };
        for (key, colour) in entries {
            let path = format!("theme.{name}.{key}");
            if is_lane(key) {
                warnings.push(ConfigWarning::LaneColour { path });
                continue;
            }
            let Some(token) = Token::from_name(key) else {
                warnings.push(ConfigWarning::UnknownKey { path });
                continue;
            };
            match colour.as_str().and_then(hex) {
                Some(hue) => overrides.for_kind_mut(kind).push((token, hue)),
                None => warnings.push(ConfigWarning::BadColour {
                    path,
                    value: written(colour),
                }),
            }
        }
    }
    (overrides, warnings)
}

/// What he wrote, for a [`ConfigWarning::BadColour`]: a string's text, a
/// scalar as TOML spells it, anything bigger by its kind (`array`, `table`).
fn written(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        toml::Value::Integer(number) => number.to_string(),
        toml::Value::Float(number) => number.to_string(),
        toml::Value::Boolean(flag) => flag.to_string(),
        other => other.type_str().to_owned(),
    }
}

/// `lane-<digits>`: PROMPT.md §5's lane names, which no theme may change.
fn is_lane(key: &str) -> bool {
    key.strip_prefix("lane-")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// `#RRGGBB`, hex digits in either case; anything else → `None`.
fn hex(text: &str) -> Option<Rgb> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    Some(Rgb {
        red: channel(0)?,
        green: channel(2)?,
        blue: channel(4)?,
    })
}

/// Every contrast floor his overrides miss, in all three themes — he edits
/// one file for every terminal, so a miss in `[theme.light]` is named while
/// he runs dark. Each as `[theme.<kind>] selection on bg 1.0:1 — under 3.0`.
///
/// Empty under `NO_COLOR`: no colour is drawn, so nothing visible is wrong
/// (CLAUDE.md: a status never reads as an error when nothing is).
pub(crate) fn theme_problems(overrides: &ThemeOverrides, no_color: bool) -> Vec<String> {
    if no_color {
        return Vec::new();
    }
    [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast]
        .into_iter()
        .flat_map(|kind| {
            contrast_misses(kind, overrides.for_kind(kind))
                .into_iter()
                .map(move |miss| format!("[theme.{}] {}", kind.name(), miss.words()))
        })
        .collect()
}
