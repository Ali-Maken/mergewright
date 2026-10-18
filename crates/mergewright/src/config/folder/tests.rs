//! The config folder's move (ticket 26 Part B), driven through
//! `load_settling` — `load` with the environment injected and the names
//! returned, not decided for the test process.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use mergewright_core::on_disk::Generation;
use mergewright_ops::rename::KEPT_ASIDE_DIR;
use tempfile::TempDir;

use super::{ConfigDirMoved, migrate_config_dir};
use crate::config::{ConfigWarning, ThemeChoice, load_settling};
use mergewright_core::on_disk::CURRENT_DIR;

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

    assert_eq!(names, Generation::CURRENT);
    assert_eq!(
        loaded.path.as_deref(),
        Some(home.join(format!("{CURRENT_DIR}/config.toml")).as_path()),
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
            fs::read(home.join(CURRENT_DIR).join(name)).expect("moved"),
            bytes,
            "{name}"
        );
    }
    assert!(!home.join("graft").exists(), "the old folder is gone");
    assert!(
        !home
            .join(format!("{}.moving", Generation::CURRENT.dir()))
            .exists(),
        "no staging left"
    );

    // The next launch finds nothing to move and says nothing.
    let (names, again) = load_settling(xdg(home)).expect("it loads again");
    assert_eq!(names, Generation::CURRENT);
    assert!(again.warnings.is_empty(), "{:?}", again.warnings);
}

/// Review m8-26 Q2, replacing `config_dir_moves_both_present`, which pinned
/// the old rule (old folder left as it was, warned every launch).
#[test]
fn both_present_files_carry_or_go_aside() {
    let dir = TempDir::new().expect("temp dir");
    let home = dir.path();
    old_folder(home);
    let new = home.join(CURRENT_DIR);
    fs::create_dir_all(&new).expect("the new folder is made");
    // Equal: `launches`. Differs: `config.toml`. No new copy: `recent`.
    fs::write(new.join("config.toml"), "[ui]\ntheme = \"dark\"\n").expect("config");
    fs::write(new.join("launches"), "3\n").expect("launches");

    let (names, loaded) = load_settling(xdg(home)).expect("it loads");

    assert_eq!(names, Generation::CURRENT, "the new folder is kept");
    assert_eq!(loaded.config.ui.theme, ThemeChoice::Dark, "and read");
    let aside = new.join("older-version/1");
    assert_eq!(
        fs::read_to_string(aside.join("config.toml")).expect("kept aside"),
        "[ui]\ntheme = \"light\"\n"
    );
    assert!(
        !aside.join("launches").exists(),
        "equal is deleted, not kept"
    );
    assert_eq!(
        fs::read_to_string(new.join("recent")).expect("carried"),
        "/code/api\t1\n"
    );
    assert!(!home.join("graft").exists(), "the old folder is gone");
    let warned: Vec<_> = loaded
        .warnings
        .iter()
        .filter(|warning| matches!(warning, ConfigWarning::ConfigFolder { .. }))
        .collect();
    assert_eq!(warned.len(), 1, "warned once: {:?}", loaded.warnings);
    let words = warned[0].to_string();
    assert!(
        words.contains("an older version wrote")
            && words.contains(&home.join("graft").display().to_string())
            && words.contains(&aside.display().to_string()),
        "{words}"
    );

    // Next launch, nothing new under the old name: silent.
    let (_, again) = load_settling(xdg(home)).expect("it loads again");
    assert!(again.warnings.is_empty(), "{:?}", again.warnings);
    // An older version writes again: a second pass folder.
    fs::create_dir_all(home.join("graft")).expect("old again");
    fs::write(home.join("graft/config.toml"), "x = 1\n").expect("config");
    let (_, third) = load_settling(xdg(home)).expect("it loads a third time");
    assert_eq!(third.warnings.len(), 1, "{:?}", third.warnings);
    assert!(new.join("older-version/2/config.toml").is_file());
}

/// An earlier generation's folder holding `theme` and a launch count.
fn write_folder(home: &Path, generation: Generation, theme: &str) {
    let dir = home.join(generation.dir());
    fs::create_dir_all(&dir).expect("the folder is made");
    fs::write(
        dir.join("config.toml"),
        format!("[ui]\ntheme = \"{theme}\"\n"),
    )
    .expect("config");
    fs::write(dir.join("launches"), "3\n").expect("launches");
}

/// Only `CURRENT`'s folder is left: no earlier one, no half-made move.
fn only_the_new(home: &Path) {
    let current = Generation::CURRENT;
    for older in current.older() {
        assert!(!home.join(older.dir()).exists(), "{older:?}");
        assert!(
            !home.join(format!("{}.lock", older.dir())).exists(),
            "{older:?}'s lock"
        );
    }
    assert!(!home.join(format!("{}.moving", current.dir())).exists());
}

/// ADR-0057 2026-10-11 §8: each earlier folder alone is moved straight to
/// the new one, nothing kept aside, nothing said.
#[test]
fn each_earlier_folder_alone_moves() {
    let current = Generation::CURRENT;
    for alone in current.older() {
        let dir = TempDir::new().expect("temp dir");
        let home = dir.path();
        write_folder(home, alone, "light");
        // The lock a build of `alone` took to move its own folder: no
        // longer needed once the folder is settled (review m8-48 Minor 7).
        fs::write(home.join(format!("{}.lock", alone.dir())), "").expect("its lock");

        let (names, loaded) = load_settling(xdg(home)).expect("it loads");

        assert_eq!(names, current, "{alone:?}");
        assert_eq!(loaded.config.ui.theme, ThemeChoice::Light, "{alone:?}");
        assert!(
            loaded.warnings.is_empty(),
            "{alone:?}: {:?}",
            loaded.warnings
        );
        assert_eq!(
            fs::read_to_string(home.join(current.dir()).join("launches")).expect("moved"),
            "3\n"
        );
        assert!(!home.join(current.dir()).join(KEPT_ASIDE_DIR).exists());
        only_the_new(home);
    }
}

/// ADR-0057 2026-10-11 §8: with every earlier folder there, one new folder
/// results. The newest present is moved; an older one meets it and takes
/// the byte rule, its differing file kept aside; said on that launch only.
#[test]
fn every_earlier_folder_moves_once() {
    let (graft, tributary, current) = (
        Generation::Graft,
        Generation::Tributary,
        Generation::CURRENT,
    );
    let dir = TempDir::new().expect("temp dir");
    let home = dir.path();
    write_folder(home, tributary, "light");
    write_folder(home, graft, "dark");

    let (names, loaded) = load_settling(xdg(home)).expect("it loads");

    assert_eq!(names, current);
    assert_eq!(loaded.config.ui.theme, ThemeChoice::Light, "the newest's");
    let aside = home.join(current.dir()).join(KEPT_ASIDE_DIR).join("1");
    assert_eq!(
        loaded.warnings,
        [ConfigWarning::ConfigFolder {
            words: format!(
                "an older version wrote {} · close it; kept in {}",
                home.join(graft.dir()).display(),
                aside.display()
            )
        }]
    );
    assert_eq!(
        fs::read_to_string(aside.join("config.toml")).expect("kept aside"),
        "[ui]\ntheme = \"dark\"\n"
    );
    assert!(!aside.join("launches").exists(), "equal is deleted");
    only_the_new(home);
    let (_, again) = load_settling(xdg(home)).expect("it loads again");
    assert!(again.warnings.is_empty(), "{:?}", again.warnings);
}

/// Review m8-26 Minor 4: two launches at once, two fds on the flock.
#[test]
fn two_launches_move_the_config_once() {
    for _ in 0..20 {
        let dir = TempDir::new().expect("temp dir");
        let home = dir.path().to_path_buf();
        old_folder(&home);
        let outcomes: Vec<_> = std::thread::scope(|scope| {
            let runs: Vec<_> = (0..2)
                .map(|_| scope.spawn(|| migrate_config_dir(&home, Generation::Graft)))
                .collect();
            runs.into_iter()
                .map(|run| run.join().expect("no panic").expect("moves"))
                .collect()
        });
        let moved = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ConfigDirMoved::Moved { files: 3 }))
            .count();
        assert_eq!(moved, 1, "{outcomes:?}");
        assert!(outcomes.contains(&ConfigDirMoved::Nothing), "{outcomes:?}");
        assert!(
            !home
                .join(format!("{}.moving", Generation::CURRENT.dir()))
                .exists()
        );
        assert!(!home.join("graft").exists());
        for name in ["config.toml", "recent", "launches"] {
            assert!(home.join(CURRENT_DIR).join(name).is_file(), "{name}");
        }
    }
}

/// Review m8-26 Minor 3: the removal's own words.
#[test]
fn a_failed_old_folder_removal_says_why() {
    let dir = TempDir::new().expect("temp dir");
    let home = dir.path();
    old_folder(home);
    fs::create_dir_all(home.join(CURRENT_DIR)).expect("new");
    // A symlink is never settled, so the old folder cannot be removed.
    std::os::unix::fs::symlink("recent", home.join("graft/link")).expect("a symlink");

    let outcome = migrate_config_dir(home, Generation::Graft).expect("settles");

    let ConfigDirMoved::OldLeft { old, why } = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(old, &home.join("graft"));
    let expected = fs::remove_dir(home.join("graft"))
        .expect_err("still not empty")
        .to_string();
    assert_eq!(why, &expected, "the io::Error words");
    let (_, loaded) = load_settling(xdg(home)).expect("it loads");
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.to_string().contains(&expected)),
        "{:?}",
        loaded.warnings
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

    assert_eq!(names, Generation::CURRENT);
    assert!(
        xdg_home
            .join(format!("{CURRENT_DIR}/config.toml"))
            .is_file(),
        "the home in force moved"
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert!(
        user.join(".config/graft/config.toml").is_file()
            && !user.join(format!(".config/{CURRENT_DIR}")).exists(),
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

    assert_eq!(names, Generation::Graft, "the old folder stays in use");
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
    assert!(!home.join(CURRENT_DIR).exists(), "nothing half-made in use");
}
