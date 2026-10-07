//! Typed config: the `[ui]`/`[graph]` sections and their value enums.
//!
//! Split from `config/mod.rs` (m1-the-graph/72c) to keep files under the
//! 400-line rule; [`super::load`] is what turns a file on disk into these.

use graft_core::capabilities::RenderingTier;
use graft_core::mistake_radar::{MistakeRadarKind, Severity};
use graft_core::ref_name::RefName;
use graft_ops::mistake_radar::Policy;
use graft_ui::theme::ThemeKind;
use serde::Deserialize;

/// Everything Graft reads from `config.toml`.
///
/// Every field has a default, so this is also the configuration of a machine
/// with no config file at all.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    /// `[ui]`: how Graft looks.
    pub ui: Ui,
    /// `[graph]`: how history is drawn.
    pub graph: Graph,
    /// `[diff]`: the detail pane's diff options at startup.
    pub diff: Diff,
    /// `[radar]`: the Mistake Radar's per-kind severity overrides.
    pub radar: Radar,
    /// `[keys]`: the few bindings that are the user's to choose.
    pub keys: Keys,
    /// `[theme.<kind>]`: his colours over a built-in theme. Read off the raw
    /// table by `config::theme`, not by serde.
    #[serde(skip)]
    pub theme: super::theme::ThemeOverrides,
}

/// `[keys]`: bindings Graft cannot pick for everyone.
///
/// One field so far. ADR-0002 gives `Ctrl+Z` to undo, because in a Git client
/// undo is the thing a hand reaches for and a suspend the user did not mean
/// looks like a crash. A user who lives in job control disagrees, and this is
/// how they say so.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Keys {
    /// `keys.suspend_on_ctrl_z`: `Ctrl+Z` stops Graft the way it stops any
    /// other terminal program, instead of undoing (ADR-0002). Off by
    /// default — which is the whole point of the ADR.
    pub suspend_on_ctrl_z: bool,
    /// `[keys.<context>] <verb> = "<chord>" | [...]` (ADR-0055): each line as
    /// `("<context>.<verb>", chords)`, sorted by id — the `toml` table is
    /// sorted, not in file order. Read off the raw table by `config::keys`,
    /// not by serde; what a line means (unknown action, bad chord,
    /// collision) is the keymap's to say (`Keymap::with_overrides`).
    #[serde(skip)]
    pub remaps: Vec<(String, Vec<String>)>,
}

/// `[ui]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Ui {
    /// `ui.theme`.
    pub theme: ThemeChoice,
    /// `ui.icons`.
    pub icons: IconSet,
    /// `ui.overview`: the overview strip starts shown. `O` still toggles it
    /// per run. Off by default.
    pub overview: bool,
}

/// `[graph]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Graph {
    /// `graph.renderer`.
    pub renderer: RendererChoice,
    /// `graph.default_branch`: the branch that owns lane 0, overriding
    /// ADR-0001's detection. Written as a branch name (`trunk`), held as the
    /// full ref name (`refs/heads/trunk`). `None` lets detection decide.
    #[serde(deserialize_with = "branch_name")]
    pub default_branch: Option<RefName>,
    /// `graph.max_lanes`: how many lanes the graph draws before folding the
    /// rest behind `+N` (DESIGN.md §3.5). `None` (unset) means the cap
    /// follows the terminal's width instead of one fixed number
    /// (ADR-0022 decision 3). The marker counts as one, so a set value
    /// below two is refused: lane 0 and the marker is the least that makes
    /// sense.
    #[serde(deserialize_with = "lane_limit", default)]
    pub max_lanes: Option<usize>,
}

/// `[diff]`: the detail pane's diff options — context, whitespace and word
/// tint — the same three fields `w`/`[`/`]`/`W` change at runtime
/// (`update/detail.rs::DetailState::options`/`word_tint`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Diff {
    /// `diff.context`: lines of context around each change.
    pub context: u8,
    /// `diff.ignore_whitespace`: whitespace-only changes hidden.
    pub ignore_whitespace: bool,
    /// `diff.word_tint`: changed words tinted inside paired lines.
    pub word_tint: bool,
}

impl Default for Diff {
    fn default() -> Self {
        Self {
            context: 3,
            ignore_whitespace: false,
            word_tint: true,
        }
    }
}

/// `[radar]`: one [`RadarSetting`] per Mistake Radar kind (PROMPT.md §1.4),
/// keyed by its kebab-case name (`graft_ops::mistake_radar::MistakeRadarKind::name`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Radar {
    /// `radar.foxtrot`.
    pub foxtrot: RadarSetting,
    /// `radar.backward`.
    pub backward: RadarSetting,
    /// `radar.accidental-pull`.
    #[serde(rename = "accidental-pull")]
    pub accidental_pull: RadarSetting,
    /// `radar.evil`.
    pub evil: RadarSetting,
    /// `radar.duplicate`.
    pub duplicate: RadarSetting,
    /// `radar.force-pushed`.
    #[serde(rename = "force-pushed")]
    pub force_pushed: RadarSetting,
    /// `radar.risky-state`.
    #[serde(rename = "risky-state")]
    pub risky_state: RadarSetting,
}

impl Radar {
    /// The [`Policy`] this section describes: [`Policy::default`] with only
    /// the keys this config actually sets overridden.
    #[must_use]
    pub fn policy(self) -> Policy {
        let mut policy = Policy::default();
        let settings = [
            (MistakeRadarKind::FoxtrotMerge, self.foxtrot),
            (MistakeRadarKind::BackwardMerge, self.backward),
            (MistakeRadarKind::AccidentalPullMerge, self.accidental_pull),
            (MistakeRadarKind::EvilMerge, self.evil),
            (MistakeRadarKind::DuplicateCommit, self.duplicate),
            (MistakeRadarKind::ForcePushedRemote, self.force_pushed),
            (MistakeRadarKind::RiskyState, self.risky_state),
        ];
        for (kind, setting) in settings {
            match setting {
                RadarSetting::Default => {}
                RadarSetting::Off => policy.set(kind, None),
                RadarSetting::Info => policy.set(kind, Some(Severity::Info)),
                RadarSetting::Warning => policy.set(kind, Some(Severity::Warning)),
            }
        }
        policy
    }
}

/// One kind's `[radar]` setting: follow the default, turn it off, or force a
/// severity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RadarSetting {
    /// PROMPT.md §1.4's severity for the kind, unchanged.
    #[default]
    Default,
    /// The kind never yields a finding.
    Off,
    /// Reports as [`Severity::Info`], however loud the default is.
    Info,
    /// Reports as [`Severity::Warning`], however quiet the default is.
    Warning,
}

/// The lane cap under 100 columns when `graph.max_lanes` is unset — the
/// narrowest of the three width breakpoints (ADR-0022 decision 3,
/// `graft_ui::graph_screen::max_lanes_for`).
pub const DEFAULT_MAX_LANES: usize = 8;

/// `graph.max_lanes`, refusing anything below two: one lane leaves no room
/// for the `+N` marker, so `1` would silently mean "never fold", and `0`
/// nothing at all. A refusal is a parse error with a line and column, like
/// every other bad value. Absent from the file, the field default (`None`)
/// applies without calling this at all.
fn lane_limit<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let limit = usize::deserialize(deserializer)?;
    if limit < 2 {
        return Err(serde::de::Error::custom(format!(
            "max_lanes = {limit} leaves no room for lanes and the `+N` marker; use 2 or more"
        )));
    }
    Ok(Some(limit))
}

/// `graph.default_branch = "trunk"` as `refs/heads/trunk`.
///
/// A name git would refuse is an error, not a default: it fails the typed
/// parse, so the error carries the file, line and column like every other bad
/// value. A valid name for a branch that does not exist is not an error here —
/// `Branches::resolve` ignores it, so a stale setting never leaves the graph
/// without a spine.
fn branch_name<'de, D>(deserializer: D) -> Result<Option<RefName>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    RefName::new(format!("refs/heads/{name}"))
        .map(Some)
        .map_err(|why| {
            serde::de::Error::custom(format!("`{name}` is not a branch name git accepts: {why}"))
        })
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
    /// Always the dark theme, whatever the terminal's background is.
    Dark,
    /// Always the light theme, whatever the terminal's background is.
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
    /// Force the pixel tier. Honoured even when detection found no image
    /// protocol: see this enum's note.
    Pixel,
    /// Force the Unicode tier: box-drawing characters, no images.
    Unicode,
    /// Force the ASCII tier: nothing outside 7-bit ASCII.
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

impl IconSet {
    /// The glyph set the graph screen draws badges with.
    ///
    /// The one translation of `ui.icons` into what the widgets take: it is
    /// read once into [`crate::update::AppState::icons`], because the badge
    /// glyphs decide how wide a badge is and so where a *drag* has to hit it
    /// (m5-history-and-ghost-preview/33).
    #[must_use]
    pub const fn glyphs(self) -> graft_ui::graph_screen::Icons {
        match self {
            IconSet::Plain => graft_ui::graph_screen::Icons::Plain,
            IconSet::Nerd => graft_ui::graph_screen::Icons::Nerd,
        }
    }
}
