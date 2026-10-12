//! The documentation can't drift from the program (m8-polish-and-ship/23):
//! `FLAGS` ↔ `parse`, `CONFIG_KEYS` ↔ the config loader, the committed man
//! page ↔ `man_page`, and the README's keys and config ↔ both tables.

use std::collections::BTreeSet;
use std::ffi::OsString;

use graft_ui::keymap::{ContextId, KeyChord, defaults};
use graft_ui::theme::Token;

use super::config_keys::is_pattern;
use super::flags::FlagAction;
use super::man::chord_text;
use super::{CONFIG_KEYS, FLAGS, Invocation, man_page, parse, usage, version};
use crate::config::{self, Config, KEYS_SUSPEND, SCHEMA};
use graft_core::on_disk::CURRENT_DIR;

const MAN_PAGE: &str = include_str!("../../../../docs/man/mwr.1");
const README: &str = include_str!("../../../../README.md");

fn parsed(word: &str) -> Invocation {
    parse([OsString::from(word)]).expect("a flag in FLAGS parses")
}

fn invocation_of(action: FlagAction) -> Invocation {
    match action {
        FlagAction::Usage => Invocation::Usage(usage(FLAGS)),
        FlagAction::GenerateMan => Invocation::GenerateMan,
        FlagAction::Version => Invocation::Version(version()),
    }
}

#[test]
fn every_flag_row_parses_in_each_spelling_and_every_action_has_a_row() {
    for flag in FLAGS {
        let action = FlagAction::of(flag.long)
            .unwrap_or_else(|| panic!("--{} has a row but parse has no action for it", flag.long));
        assert_eq!(parsed(&format!("--{}", flag.long)), invocation_of(action));
        if let Some(short) = flag.short {
            assert_eq!(parsed(&format!("-{short}")), invocation_of(action));
        }
    }
    for &action in FlagAction::ALL {
        assert!(
            FLAGS
                .iter()
                .any(|flag| FlagAction::of(flag.long) == Some(action)),
            "{action:?} has no row in FLAGS"
        );
    }
}

#[test]
fn help_is_usage_of_flags() {
    assert_eq!(parsed("--help"), Invocation::Usage(usage(FLAGS)));
    for flag in FLAGS.iter().filter(|flag| !flag.hidden) {
        assert!(usage(FLAGS).contains(&format!("--{}", flag.long)));
    }
}

/// The committed `docs/man/mwr.1` is what `--generate-man` prints today.
#[test]
fn committed_man_page_is_the_generated_one() {
    let generated = man_page(FLAGS, defaults(), CONFIG_KEYS);
    assert!(
        MAN_PAGE == generated,
        "docs/man/mwr.1 is stale: run `cargo run -q -p mergewright -- --generate-man > docs/man/mwr.1`"
    );
}

/// The FILES and ENVIRONMENT paths name the folder the app uses today
/// (`Generation::CURRENT`), never the old `graft/` one.
#[test]
fn man_page_names_the_config_folder_in_force() {
    let page = man_page(FLAGS, defaults(), CONFIG_KEYS);
    let dir = CURRENT_DIR;
    for path in [
        format!("$XDG_CONFIG_HOME/{dir}/config.toml"),
        format!("$XDG_CONFIG_HOME/{dir}/recent"),
        format!("$XDG_CONFIG_HOME/{dir}/launches"),
        format!("$XDG_STATE_HOME/{dir}/"),
    ] {
        assert!(page.contains(&path), "missing {path}");
    }
    assert!(!page.contains("/graft/"), "old folder named");
    assert!(!page.contains("/graft\n") && !page.contains("/graft."));
    assert!(page.starts_with(".TH MWR 1"));
}

#[test]
fn man_page_is_ascii_and_names_every_action_flag_and_key() {
    let page = man_page(FLAGS, defaults(), CONFIG_KEYS);
    assert!(page.is_ascii());
    for binding in defaults() {
        assert!(
            page.contains(&format!("({})", binding.action.0)),
            "{}",
            binding.action.0
        );
    }
    for flag in FLAGS {
        assert!(page.contains(&format!("\\-\\-{}", flag.long.replace('-', "\\-"))));
    }
    for doc in CONFIG_KEYS {
        assert!(page.contains(&format!(".B {}", doc.key.replace('\\', "\\e"))));
    }
    for section in ["ENVIRONMENT", "FILES", "SEE ALSO", "KEYS", "CONFIGURATION"] {
        assert!(page.contains(&format!(".SH {section}\n")), "{section}");
    }
}

/// The man page writes chords as `[keys]` takes them: each reads back as
/// the same key.
#[test]
fn every_chord_the_man_page_writes_reads_back_as_itself() {
    for binding in defaults() {
        for chord in binding.default {
            let text = chord_text(chord);
            assert_eq!(KeyChord::parse(&text).as_ref(), Ok(chord), "{text}");
        }
    }
}

/// The loader's keys, as `section.key`: its `SCHEMA` and `[keys]`' switch.
fn loader_keys() -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = SCHEMA
        .iter()
        .flat_map(|(section, names)| names.iter().map(move |name| format!("{section}.{name}")))
        .collect();
    keys.insert(format!("keys.{KEYS_SUSPEND}"));
    keys
}

#[test]
fn config_keys_and_the_loader_name_the_same_keys() {
    let documented: BTreeSet<String> = CONFIG_KEYS
        .iter()
        .filter(|doc| !is_pattern(doc))
        .map(|doc| format!("{}.{}", doc.section, doc.key))
        .collect();
    assert_eq!(documented, loader_keys());
    // The two tables the loader reads off the raw file, not `SCHEMA`.
    let patterns: Vec<&str> = CONFIG_KEYS
        .iter()
        .filter(|doc| is_pattern(doc))
        .map(|doc| doc.section)
        .collect();
    assert_eq!(patterns, ["keys.<context>", "theme.<kind>"]);
}

/// Each key with a literal default, written alone with that value: loads
/// with no warning, and the result is `Config::default()` — the documented
/// default is the real one.
#[test]
fn every_documented_default_loads_cleanly_as_the_real_default() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("config.toml");
    for doc in CONFIG_KEYS
        .iter()
        .filter(|doc| !doc.default.starts_with('('))
    {
        let text = format!(
            "[{}]\n{} = {}\n",
            doc.section,
            quoted_key(doc.key),
            doc.default
        );
        std::fs::write(&path, &text).expect("write config");
        let loaded = config::load_from(&path).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert!(loaded.warnings.is_empty(), "{text}: {:?}", loaded.warnings);
        assert_eq!(loaded.config, Config::default(), "{text}");
    }
}

fn quoted_key(key: &str) -> String {
    if key.contains('-') {
        format!("\"{key}\"")
    } else {
        key.to_owned()
    }
}

/// The README's "Daily keys" table: each row's action exists, and the key it
/// shows is one of that action's defaults, spelled as the app spells it.
#[test]
fn readme_daily_keys_are_the_tables_keys() {
    let rows = readme_section("## Daily keys")
        .lines()
        .filter(|line| line.starts_with("| `"))
        .collect::<Vec<_>>();
    assert!(
        rows.len() >= 12,
        "expected at least 12 rows, got {}",
        rows.len()
    );
    for row in rows {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        let (key, action) = (cells[1].trim_matches('`'), cells[3].trim_matches('`'));
        let binding = defaults()
            .iter()
            .find(|b| b.action.0 == action)
            .unwrap_or_else(|| panic!("README names `{action}`, which is not an action"));
        let labels: Vec<String> = binding.default.iter().map(|c| c.label().0).collect();
        assert!(
            labels.iter().any(|label| label == key),
            "README shows `{key}` for `{action}`; its keys are {labels:?}"
        );
    }
}

/// Every `[section]` and `key =` in the README's TOML blocks is in
/// `CONFIG_KEYS`; every `[keys.<context>]` names a context and its keys name
/// that context's actions; every `[theme.<kind>]` a kind and its tokens.
#[test]
fn readme_config_names_only_real_keys() {
    let mut section = String::new();
    let mut seen = 0;
    for line in readme_toml_lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_owned();
            assert!(section_exists(&section), "README has [{section}]");
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        assert!(key_exists(&section, key), "README has {section}.{key}");
        seen += 1;
    }
    assert!(
        seen >= 8,
        "expected the config section's examples, saw {seen} keys"
    );
}

/// The README's example config, as one file: loads, with no warning from the
/// loader and none from the keymap (a rebind that collides would say so).
#[test]
fn readme_example_config_loads_without_a_word() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("config.toml");
    std::fs::write(&path, readme_toml_lines().join("\n")).expect("write config");

    let loaded = config::load_from(&path).unwrap_or_else(|e| panic!("README's config: {e}"));
    let (_keymap, problems) = config::keymap_from(&loaded);

    assert!(problems.is_empty(), "{problems:?}");
    assert!(!loaded.config.keys.remaps.is_empty());
    assert!(!loaded.config.theme.dark.is_empty());
}

/// Backticked `section.key` and `context.action` names anywhere in the
/// README exist.
#[test]
fn readme_backticked_names_exist() {
    let sections: BTreeSet<&str> = CONFIG_KEYS.iter().map(|doc| doc.section).collect();
    let contexts: BTreeSet<&str> = ContextId::ALL.iter().map(|c| c.config_name()).collect();
    for name in README.split('`').skip(1).step_by(2) {
        let Some((head, tail)) = name.split_once('.') else {
            continue;
        };
        let checked = sections.contains(head) || contexts.contains(head);
        if !checked || tail.contains(['.', ' ', '/']) || tail.is_empty() {
            continue;
        }
        let is_action = defaults().iter().any(|b| b.action.0 == name);
        assert!(is_action || key_exists(head, tail), "README names `{name}`");
    }
}

/// The README's command-line block is `usage(FLAGS)` byte for byte, so a new
/// flag cannot drift past it (review m8-hygiene-release m2).
#[test]
fn readme_command_line_is_the_usage_of_flags() {
    let block = format!("```text\n{}\n```", usage(FLAGS));
    assert!(
        readme_section("## Command line").contains(&block),
        "README `## Command line` must hold:\n{block}"
    );
}

fn readme_section(heading: &str) -> &'static str {
    let start = README
        .find(heading)
        .unwrap_or_else(|| panic!("README has no {heading}"));
    let rest = &README[start + heading.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}

fn readme_toml_lines() -> Vec<&'static str> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in README.lines() {
        if line.starts_with("```") {
            inside = line == "```toml";
            continue;
        }
        if inside {
            lines.push(line);
        }
    }
    lines
}

fn section_exists(section: &str) -> bool {
    if let Some(context) = section.strip_prefix("keys.") {
        return ContextId::ALL.iter().any(|c| c.config_name() == context);
    }
    if let Some(kind) = section.strip_prefix("theme.") {
        return ["dark", "light", "high-contrast", "high_contrast"].contains(&kind);
    }
    CONFIG_KEYS.iter().any(|doc| doc.section == section)
}

fn key_exists(section: &str, key: &str) -> bool {
    if section.starts_with("keys.") {
        return defaults()
            .iter()
            .any(|b| b.action.0 == format!("{}.{key}", &section[5..]));
    }
    if section.starts_with("theme.") {
        return Token::ALL.iter().any(|t| t.name() == key);
    }
    CONFIG_KEYS
        .iter()
        .any(|doc| doc.section == section && doc.key == key)
}
