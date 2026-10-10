//! The config folder's move (ticket 26 Part B), driven through
//! `load_settling` — `load` with the environment injected and the names
//! returned, not decided for the test process.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use graft_core::on_disk::OnDiskNames;
use tempfile::TempDir;

use crate::config::{ConfigWarning, ThemeChoice, load_settling};

/// His three files under `<home>/graft`, the config asking for the light
/// theme.
fn old_folder(home: &Path) {
    let old = home.join("graft");
    fs::create_dir_all(&old).expect("the old folder is made");
    fs::write(old.join("config.toml"), "[ui]\ntheme = \"light\"\n").expect("config");
    fs::write(old.join("recent"), "/code/api\t1\n").expect("recent");
    fs::write(old.join("launches"), "3\n").expect("launches");
}

fn xdg(home: &Path) -> impl Fn(&str) -> Option<OsString> + '_ {
    move |name| (name == "XDG_CONFIG_HOME").then(|| home.as_os_str().to_owned())
}

#[test]
fn config_dir_moves_before_first_load() {
    let dir = TempDir::new().expect("temp dir");
    let home = dir.path();
    old_folder(home);
    let before: Vec<_> = ["config.toml", "recent", "launches"]
        .map(|name| fs::read(home.join("graft").join(name)).expect("reads"))
        .into();

    let (names, loaded) = load_settling(xdg(home)).expect("it loads");

    assert_eq!(names, OnDiskNames::New);
    assert_eq!(
        loaded.path.as_deref(),
        Some(home.join("tributary/config.toml").as_path()),
        "the first read is of the new folder"
    );
    assert_eq!(
        loaded.config.ui.theme,
        ThemeChoice::Light,
        "his setting held"
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    for (name, bytes) in ["config.toml", "recent", "launches"].iter().zip(before) {
        assert_eq!(
            fs::read(home.join("tributary").join(name)).expect("moved"),
            bytes,
            "{name}"
        );
    }
    assert!(!home.join("graft").exists(), "the old folder is gone");
    assert!(!home.join("tributary.moving").exists(), "no staging left");

    // The next launch finds nothing to move and says nothing.
    let (names, again) = load_settling(xdg(home)).expect("it loads again");
    assert_eq!(names, OnDiskNames::New);
    assert!(again.warnings.is_empty(), "{:?}", again.warnings);
}

#[test]
fn config_dir_moves_both_present() {
    let dir = TempDir::new().expect("temp dir");
    let home = dir.path();
    old_folder(home);
    let new = home.join("tributary");
    fs::create_dir_all(&new).expect("the new folder is made");
    fs::write(new.join("config.toml"), "[ui]\ntheme = \"dark\"\n").expect("config");

    let (names, loaded) = load_settling(xdg(home)).expect("it loads");

    assert_eq!(names, OnDiskNames::New, "the new folder is kept");
    assert_eq!(loaded.config.ui.theme, ThemeChoice::Dark, "and read");
    let warned: Vec<_> = loaded
        .warnings
        .iter()
        .filter(|warning| matches!(warning, ConfigWarning::ConfigFolder { .. }))
        .collect();
    assert_eq!(warned.len(), 1, "warned once: {:?}", loaded.warnings);
    let words = warned[0].to_string();
    assert!(
        words.contains(&home.join("graft").display().to_string()),
        "names the old folder: {words}"
    );
    assert_eq!(
        fs::read_to_string(home.join("graft/config.toml")).expect("still there"),
        "[ui]\ntheme = \"light\"\n",
        "the old folder is left as it was"
    );
}

#[test]
fn config_dir_moves_env_override_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let xdg_home = dir.path().join("xdg");
    let user = dir.path().join("user");
    fs::create_dir_all(&xdg_home).expect("xdg home");
    old_folder(&user.join(".config"));
    old_folder(&xdg_home);

    let read = |name: &str| match name {
        "XDG_CONFIG_HOME" => Some(xdg_home.as_os_str().to_owned()),
        "HOME" => Some(user.as_os_str().to_owned()),
        _ => None,
    };
    let (names, loaded) = load_settling(read).expect("it loads");

    assert_eq!(names, OnDiskNames::New);
    assert!(
        xdg_home.join("tributary/config.toml").is_file(),
        "the home in force moved"
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert!(
        user.join(".config/graft/config.toml").is_file()
            && !user.join(".config/tributary").exists(),
        "the root the environment overrode is never touched"
    );
}

#[test]
fn a_move_that_fails_keeps_the_old_folder_and_says_so() {
    let dir = TempDir::new().expect("temp dir");
    let home = dir.path();
    old_folder(home);
    // Not a file or a folder: the copy refuses it, before anything is used.
    std::os::unix::fs::symlink("recent", home.join("graft/link")).expect("a symlink");

    let (names, loaded) = load_settling(xdg(home)).expect("it loads");

    assert_eq!(names, OnDiskNames::Old, "the old folder stays in use");
    assert_eq!(
        loaded.path.as_deref(),
        Some(home.join("graft/config.toml").as_path())
    );
    assert_eq!(loaded.config.ui.theme, ThemeChoice::Light);
    assert!(
        matches!(
            loaded.warnings.as_slice(),
            [ConfigWarning::ConfigFolder { .. }]
        ),
        "{:?}",
        loaded.warnings
    );
    assert!(!home.join("tributary").exists(), "nothing half-made in use");
}
