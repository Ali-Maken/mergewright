//! What a config file does, and what a bad one says.
//!
//! No test here reads a real home directory: files go in a `tempfile` dir, and
//! the environment `default_path` reads is injected.

use std::ffi::OsString;
use std::path::Path;

use tempfile::TempDir;

use super::*;

/// A config file in a fresh temporary directory, and the directory that keeps
/// it alive for the length of the test.
fn config_file(text: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("config.toml");
    fs::write(&path, text).expect("write config");
    (dir, path)
}

/// The error `load_from` produced, or a failure naming what loaded instead.
fn load_error(path: &Path) -> ConfigError {
    match load_from(path) {
        Err(error) => error,
        Ok(loaded) => panic!("expected an error, loaded {loaded:?}"),
    }
}

#[test]
fn a_missing_file_is_the_defaults_and_says_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let loaded = load_from(&dir.path().join("config.toml")).expect("missing file loads");

    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.config.ui.theme, ThemeChoice::Auto);
    assert_eq!(loaded.config.ui.icons, IconSet::Plain);
    assert_eq!(loaded.config.graph.renderer, RendererChoice::Auto);
    assert_eq!(loaded.warnings, []);
    assert_eq!(loaded.path, None);
}

#[test]
fn an_empty_file_is_the_defaults_too() {
    let (_dir, path) = config_file("");
    let loaded = load_from(&path).expect("empty file loads");

    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.warnings, []);
    assert_eq!(loaded.path.as_deref(), Some(path.as_path()));
}

#[test]
fn every_key_is_read() {
    let (_dir, path) = config_file(
        "\
[ui]
theme = \"high-contrast\"
icons = \"nerd\"

[graph]
renderer = \"pixel\"
",
    );
    let loaded = load_from(&path).expect("valid file loads");

    assert_eq!(loaded.config.ui.theme, ThemeChoice::HighContrast);
    assert_eq!(loaded.config.ui.icons, IconSet::Nerd);
    assert_eq!(loaded.config.graph.renderer, RendererChoice::Pixel);
    assert_eq!(loaded.warnings, []);
    assert_eq!(loaded.path.as_deref(), Some(path.as_path()));
}

#[test]
fn keys_the_file_omits_keep_their_defaults() {
    let (_dir, path) = config_file("[ui]\ntheme = \"light\"\n");
    let loaded = load_from(&path).expect("partial file loads");

    assert_eq!(loaded.config.ui.theme, ThemeChoice::Light);
    assert_eq!(loaded.config.ui.icons, IconSet::Plain);
    assert_eq!(loaded.config.graph.renderer, RendererChoice::Auto);
}

#[test]
fn broken_toml_names_the_file_the_line_and_the_problem() {
    let (_dir, path) = config_file("[ui]\ntheme = \n");
    let error = load_error(&path);

    assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    let message = error.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("line 2"), "{message}");
    assert!(message.contains("string"), "{message}");
}

#[test]
fn a_value_that_isnt_a_known_spelling_is_an_error_not_a_default() {
    // The whole point: `blu` must not quietly load as the dark theme, because
    // a user who got the default anyway has no way to find the typo.
    let (_dir, path) = config_file("[ui]\ntheme = \"blu\"\n");
    let error = load_error(&path);

    let message = error.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("blu"), "{message}");
    // serde lists what it would have accepted, which is the fix.
    assert!(message.contains("high-contrast"), "{message}");
}

#[test]
fn one_bad_value_rejects_the_whole_file_rather_than_half_of_it() {
    let (_dir, path) = config_file("[ui]\ntheme = \"light\"\nicons = \"pretty\"\n");
    let error = load_error(&path);

    assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    assert!(error.to_string().contains("pretty"), "{error}");
}

#[test]
fn a_value_of_the_wrong_type_is_an_error() {
    let (_dir, path) = config_file("[graph]\nrenderer = 3\n");
    let error = load_error(&path);

    assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
}

#[test]
fn a_section_of_the_wrong_type_is_an_error() {
    let (_dir, path) = config_file("ui = \"dark\"\n");
    let error = load_error(&path);

    assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
}

#[test]
fn an_unreadable_file_is_an_error_not_the_defaults() {
    // A directory where the file should be: readable metadata, unreadable
    // contents. `NotFound` is the only kind that means "no config".
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("config.toml");
    fs::create_dir(&path).expect("create dir");

    let error = load_error(&path);
    assert!(matches!(error, ConfigError::Read { .. }), "{error:?}");
    assert!(
        error.to_string().contains(&path.display().to_string()),
        "{error}"
    );
}

#[test]
fn an_unknown_key_warns_and_the_rest_still_loads() {
    let (_dir, path) = config_file(
        "\
[ui]
theme = \"dark\"
icns = \"nerd\"

[keys]
quit = \"q\"
",
    );
    let loaded = load_from(&path).expect("unknown keys still load");

    assert_eq!(loaded.config.ui.theme, ThemeChoice::Dark);
    assert_eq!(loaded.config.ui.icons, IconSet::Plain);
    assert_eq!(
        loaded.warnings,
        [
            ConfigWarning::UnknownKey {
                path: "keys".to_owned()
            },
            ConfigWarning::UnknownKey {
                path: "ui.icns".to_owned()
            },
        ]
    );
}

#[test]
fn an_unknown_key_warning_names_the_key() {
    let warning = ConfigWarning::UnknownKey {
        path: "ui.icns".to_owned(),
    };

    assert!(warning.to_string().contains("ui.icns"), "{warning}");
}

#[test]
fn theme_spellings_are_the_ones_the_theme_module_uses() {
    // Locks the config file's vocabulary to `ThemeKind::name`, so renaming a
    // theme in `ui` can't silently leave the config file spelling the old one.
    // The match is exhaustive: a fourth theme fails to compile here.
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        let choice = match kind {
            ThemeKind::Dark => ThemeChoice::Dark,
            ThemeKind::Light => ThemeChoice::Light,
            ThemeKind::HighContrast => ThemeChoice::HighContrast,
        };

        let (_dir, path) = config_file(&format!("[ui]\ntheme = \"{}\"\n", kind.name()));
        let loaded = load_from(&path).expect("a theme name from `ui` parses");

        assert_eq!(loaded.config.ui.theme, choice);
        assert_eq!(choice.overridden(), Some(kind));
    }
}

#[test]
fn auto_is_the_absence_of_an_override() {
    assert_eq!(ThemeChoice::Auto.overridden(), None);
    assert_eq!(RendererChoice::Auto.overridden(), None);
}

#[test]
fn every_rendering_tier_can_be_forced() {
    // Exhaustive over `RenderingTier`: a new tier in `core` fails to compile
    // until `graph.renderer` can name it.
    for tier in [
        RenderingTier::Pixel,
        RenderingTier::Unicode,
        RenderingTier::Ascii,
    ] {
        let (choice, spelling) = match tier {
            RenderingTier::Pixel => (RendererChoice::Pixel, "pixel"),
            RenderingTier::Unicode => (RendererChoice::Unicode, "unicode"),
            RenderingTier::Ascii => (RendererChoice::Ascii, "ascii"),
        };

        let (_dir, path) = config_file(&format!("[graph]\nrenderer = \"{spelling}\"\n"));
        let loaded = load_from(&path).expect("a tier name parses");

        assert_eq!(loaded.config.graph.renderer, choice);
        assert_eq!(choice.overridden(), Some(tier));
    }
}

#[test]
fn xdg_config_home_wins_over_home() {
    let path = path_in_environment(|name| match name {
        "XDG_CONFIG_HOME" => Some(OsString::from("/xdg")),
        "HOME" => Some(OsString::from("/home/someone")),
        _ => None,
    });

    assert_eq!(path.as_deref(), Some(Path::new("/xdg/graft/config.toml")));
}

#[test]
fn home_supplies_dot_config_when_xdg_is_unset() {
    let path =
        path_in_environment(|name| (name == "HOME").then(|| OsString::from("/home/someone")));

    assert_eq!(
        path.as_deref(),
        Some(Path::new("/home/someone/.config/graft/config.toml"))
    );
}

#[test]
fn a_relative_xdg_config_home_is_ignored() {
    // The XDG spec says a non-absolute value is treated as unset.
    let path = path_in_environment(|name| match name {
        "XDG_CONFIG_HOME" => Some(OsString::from("relative/config")),
        "HOME" => Some(OsString::from("/home/someone")),
        _ => None,
    });

    assert_eq!(
        path.as_deref(),
        Some(Path::new("/home/someone/.config/graft/config.toml"))
    );
}

#[test]
fn no_home_at_all_means_no_path() {
    assert_eq!(path_in_environment(|_| None), None);
}

#[test]
fn nowhere_to_look_loads_defaults_and_says_so() {
    let loaded = load_at(None).expect("defaults still load");

    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.path, None);
    assert_eq!(loaded.warnings, [ConfigWarning::NoConfigHome]);
    assert!(
        loaded.warnings[0].to_string().contains("HOME"),
        "{}",
        loaded.warnings[0]
    );
}

#[test]
fn the_default_branch_is_read_as_a_full_branch_name() {
    let (_dir, path) = config_file("[graph]\ndefault_branch = \"trunk\"\n");
    let loaded = load_from(&path).expect("a valid default branch loads");

    assert_eq!(
        loaded.config.graph.default_branch,
        Some(RefName::new("refs/heads/trunk").expect("valid"))
    );
    assert_eq!(loaded.warnings, [], "default_branch is a known key");
}

#[test]
fn no_default_branch_leaves_detection_to_decide() {
    let (_dir, path) = config_file("[graph]\nrenderer = \"unicode\"\n");
    let loaded = load_from(&path).expect("loads");
    assert_eq!(loaded.config.graph.default_branch, None);
}

#[test]
fn a_default_branch_git_would_refuse_names_the_file_and_the_line() {
    let (_dir, path) =
        config_file("[graph]\nrenderer = \"auto\"\ndefault_branch = \"two..dots\"\n");
    let error = load_error(&path);

    assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    let message = error.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("line 3"), "{message}");
    assert!(message.contains("two..dots"), "{message}");
}

#[test]
fn max_lanes_is_read_and_absent_means_the_width_decides() {
    let (_dir, path) = config_file("[graph]\nmax_lanes = 12\n");
    let loaded = load_from(&path).expect("a valid lane limit loads");
    assert_eq!(loaded.config.graph.max_lanes, Some(12));
    assert_eq!(loaded.warnings, [], "max_lanes is a known key");

    let (_dir, path) = config_file("[graph]\nrenderer = \"unicode\"\n");
    let loaded = load_from(&path).expect("loads");
    assert_eq!(
        loaded.config.graph.max_lanes, None,
        "unset means the cap follows the terminal's width"
    );
    assert_eq!(DEFAULT_MAX_LANES, 8);
}

#[test]
fn a_lane_limit_with_no_room_for_the_marker_is_refused() {
    for text in ["[graph]\nmax_lanes = 1\n", "[graph]\nmax_lanes = 0\n"] {
        let (_dir, path) = config_file(text);
        let error = load_error(&path);
        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
        assert!(error.to_string().contains("+N"), "{error}");
    }
    let (_dir, path) = config_file("[graph]\nmax_lanes = \"many\"\n");
    assert!(matches!(load_error(&path), ConfigError::Parse { .. }));
}
