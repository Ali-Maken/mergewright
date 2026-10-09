//! Every config key, as the man page and the README describe it
//! (m8-polish-and-ship/23). Paired both ways with the loader's own list by
//! `cli::docs_tests`, and every literal default here is loaded and compared
//! with `Config::default()`, so a row cannot name a key the loader ignores
//! or a default it does not have.

/// One `config.toml` key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigKeyDoc {
    /// The table it lives in: `ui`, or a pattern `keys.<context>`.
    pub section: &'static str,
    /// The key, or a pattern `<action>`.
    pub key: &'static str,
    /// The values it takes, as a person writes them.
    pub values: &'static str,
    /// Its default, as TOML spells it (`"auto"`, `false`, `3`), or words in
    /// parentheses where there is no literal (`(unset)`).
    pub default: &'static str,
    /// What it does.
    pub help: &'static str,
}

const fn row(
    section: &'static str,
    key: &'static str,
    values: &'static str,
    default: &'static str,
    help: &'static str,
) -> ConfigKeyDoc {
    ConfigKeyDoc {
        section,
        key,
        values,
        default,
        help,
    }
}

const RADAR_VALUES: &str = "default | off | info | warning";
const RADAR_DEFAULT: &str = "\"default\"";

/// Every key the loader reads, in the file's section order.
pub const CONFIG_KEYS: &[ConfigKeyDoc] = &[
    row(
        "ui",
        "theme",
        "auto | dark | light | high-contrast",
        "\"auto\"",
        "The colour theme. auto follows the terminal's background, dark if it does not say.",
    ),
    row(
        "ui",
        "icons",
        "plain | nerd",
        "\"plain\"",
        "Glyphs: plain works in every font; nerd needs a Nerd Font.",
    ),
    row(
        "ui",
        "overview",
        "true | false",
        "false",
        "Start with the overview strip shown. O still toggles it.",
    ),
    row(
        "graph",
        "renderer",
        "auto | pixel | unicode | ascii",
        "\"auto\"",
        "How the graph is drawn. auto uses what the terminal says it can do.",
    ),
    row(
        "graph",
        "default_branch",
        "any branch name",
        "(unset)",
        "The branch that owns the leftmost lane. Unset: detected.",
    ),
    row(
        "graph",
        "max_lanes",
        "any number from 2 up",
        "(unset)",
        "Lanes drawn before the rest fold behind +N. Unset: 8, 10 or 12 by terminal width.",
    ),
    row(
        "diff",
        "context",
        "any number of lines",
        "3",
        "Lines of context around each change.",
    ),
    row(
        "diff",
        "ignore_whitespace",
        "true | false",
        "false",
        "Hide whitespace-only changes.",
    ),
    row(
        "diff",
        "word_tint",
        "true | false",
        "true",
        "Tint the changed words inside a changed line.",
    ),
    row(
        "radar",
        "foxtrot",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: a merge on the default branch that rewrote its first-parent history.",
    ),
    row(
        "radar",
        "backward",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: the default branch merged into a feature branch.",
    ),
    row(
        "radar",
        "accidental-pull",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: a branch merged with its own remote copy, as git pull leaves.",
    ),
    row(
        "radar",
        "evil",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: a merge carrying changes neither parent had.",
    ),
    row(
        "radar",
        "duplicate",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: the same patch on several branches.",
    ),
    row(
        "radar",
        "force-pushed",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: a remote-tracking branch that was force-pushed.",
    ),
    row(
        "radar",
        "risky-state",
        RADAR_VALUES,
        RADAR_DEFAULT,
        "Mistake Radar: a detached HEAD, or unpushed commits on the default branch.",
    ),
    row(
        "keys",
        "suspend_on_ctrl_z",
        "true | false",
        "false",
        "ctrl+z suspends Graft like any terminal program; undo keeps u.",
    ),
    row(
        "keys.<context>",
        "<action>",
        "one key, a list of keys, or \"none\"",
        "(the KEYS table)",
        "Rebind an action. The context and action names are in KEYS.",
    ),
    row(
        "theme.<kind>",
        "<token>",
        "\"#RRGGBB\"",
        "(the built-in theme)",
        "Override one colour of the dark, light or high-contrast theme. Lane colours are fixed.",
    ),
];

/// Whether a row is a pattern (`keys.<context>`) rather than one key.
pub(crate) fn is_pattern(doc: &ConfigKeyDoc) -> bool {
    doc.section.contains('<') || doc.key.contains('<')
}
