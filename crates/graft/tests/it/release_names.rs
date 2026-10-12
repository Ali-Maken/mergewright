//! What ships is named from the table (m8-polish-and-ship/53, ADR-0057
//! amendment §11): our Homebrew formula template and README's Install lines
//! name the current generation's package and command, and the Install lines
//! name what `dist plan` (docs/release/plan.txt) says it builds.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::fs;
use std::path::{Path, PathBuf};

use graft_core::on_disk::Generation;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/graft sits two levels below the repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()))
}

/// `mergewright` → `Mergewright`: Homebrew's class name for a formula file.
fn class_name(dir: &str) -> String {
    let mut chars = dir.chars();
    let first = chars.next().expect("a dir name is never empty");
    first.to_uppercase().chain(chars).collect()
}

/// The archive names the template takes a URL for: `@URL:<archive>@`.
fn template_archives(template: &str) -> Vec<&str> {
    template
        .split("@URL:")
        .skip(1)
        .map(|rest| rest.split('@').next().expect("split yields one piece"))
        .collect()
}

#[test]
fn formula_template_names_the_table() {
    let template = read("packaging/homebrew/mergewright.rb.in");
    let dir = Generation::CURRENT.dir();
    let command = Generation::CURRENT.command();

    let class = format!("class {} < Formula", class_name(dir));
    assert!(template.contains(&class), "the template's class is `{class}`");

    let archives = template_archives(&template);
    let targets = [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
    ];
    let wanted: Vec<String> = targets
        .iter()
        .map(|target| format!("{dir}-{target}.tar.xz"))
        .collect();
    assert_eq!(archives, wanted, "one archive per target, named {dir}-<target>");
    for archive in &wanted {
        assert!(
            template.contains(&format!("@SHA256:{archive}@")),
            "{archive} has a sha256"
        );
    }

    for line in [
        format!("bin.install \"{command}\""),
        format!("man1.install \"{command}.1\""),
        format!("#{{bin}}/{command} --version"),
    ] {
        assert!(template.contains(&line), "the template has `{line}`");
    }

    // The formula's licence is the workspace's.
    let cargo = read("Cargo.toml");
    let license = cargo
        .lines()
        .find_map(|line| line.strip_prefix("license = "))
        .expect("the workspace names a licence");
    assert!(
        template.contains(&format!("  license {license}\n")),
        "the template's licence is the workspace's {license}"
    );

    // Comment lines are dropped by formula.py; what is left names no
    // earlier generation.
    let code: Vec<&str> = template
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    for old in Generation::CURRENT.older() {
        for name in [old.dir(), old.command(), old.title()] {
            let quoted = [format!("\"{name}"), format!("{name}-"), format!("/{name}")];
            for line in &code {
                assert!(
                    !quoted.iter().any(|form| line.contains(form.as_str())),
                    "`{line}` names the earlier `{name}`"
                );
            }
        }
    }
}

/// README's Install section, `## Install` up to the next `## `.
fn readme_install() -> String {
    let readme = read("README.md");
    let start = readme.find("## Install").expect("README has an Install section");
    let rest = &readme[start + 2..];
    let end = rest.find("\n## ").map_or(rest.len(), |at| at + 2);
    readme[start..start + end].to_owned()
}

#[test]
fn readme_install_lines_match_the_plan() {
    let plan = read("docs/release/plan.txt");
    let install = readme_install();
    let dir = Generation::CURRENT.dir();

    // The package dist announces (`  mergewright 0.1.0`) is the table's.
    let package = plan
        .lines()
        .find_map(|line| {
            let line = line.strip_prefix("  ")?;
            if line.starts_with(' ') {
                return None;
            }
            line.split_once(' ').map(|(name, _version)| name)
        })
        .expect("plan.txt announces a package");
    assert_eq!(package, dir, "dist announces the table's package");

    let installer = plan
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with("-installer.sh"))
        .expect("plan.txt lists a shell installer");
    let curl = install
        .lines()
        .find(|line| line.starts_with("curl "))
        .expect("README's Install has a curl line");
    assert!(
        curl.contains(&format!("/{installer} ")),
        "the curl line fetches `{installer}`, the plan's installer: {curl}"
    );

    let brew = install
        .lines()
        .find(|line| line.starts_with("brew install "))
        .expect("README's Install has a brew line");
    assert!(
        brew.starts_with(&format!("brew install <owner>/tap/{package} ")),
        "the brew line installs the plan's package `{package}`: {brew}"
    );

    assert!(
        install.contains("cargo install --locked --path crates/graft"),
        "README's Install keeps the checkout line"
    );
    let command = Generation::CURRENT.command();
    assert!(
        install.contains(&format!("docs/man/{command}.1")),
        "README's Install names the man page `{command}.1`"
    );

    for old in Generation::CURRENT.older() {
        for line in [curl, brew] {
            assert!(
                !line.contains(&format!("/{}", old.command())) && !line.contains(&format!("/{}", old.dir())),
                "`{line}` names the earlier `{}`",
                old.dir()
            );
        }
    }
}
