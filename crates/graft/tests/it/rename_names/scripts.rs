//! Scripts and live files run the current binary and name the current
//! package (m8-polish-and-ship/27b, 49): env names, binary paths, `-p`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use graft_core::on_disk::Generation;

use crate::rename_opens::crates_dir;

/// Every file under `dir`, recursively. A missing `dir` fails the test, so
/// a moved folder never leaves a guard reading nothing.
fn every_file(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{} reads: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("a dir entry").path();
        if path.is_dir() {
            every_file(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The file's text, or `None` when it is not UTF-8. Any other read error
/// (a moved or missing file) fails the test.
fn read_text(file: &Path) -> Option<String> {
    match fs::read_to_string(file) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == ErrorKind::InvalidData => None,
        Err(e) => panic!("{} reads: {e}", file.display()),
    }
}

/// `text` starts with an earlier generation's command as a whole word
/// (`graft`, `trib`), not a longer name such as `graft-fixtures`.
fn names_old_binary(text: &str) -> bool {
    Generation::CURRENT.older().any(|earlier| {
        text.strip_prefix(earlier.command()).is_some_and(|rest| {
            !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '-')
        })
    })
}

/// A path built from segments (`os.path.join(…, "release", "graft")`): a
/// quoted `release`/`debug` followed by a quoted earlier command.
fn names_old_segment(line: &str) -> bool {
    ['"', '\''].iter().any(|&quote| {
        ["release", "debug"].iter().any(|profile| {
            let lead = format!("{quote}{profile}{quote}");
            line.match_indices(lead.as_str()).any(|(i, _)| {
                let rest = line[i + lead.len()..].trim_start();
                rest.strip_prefix(',')
                    .map(str::trim_start)
                    .and_then(|rest| rest.strip_prefix(quote))
                    .is_some_and(|name| {
                        Generation::CURRENT.older().any(|earlier| {
                            name.strip_prefix(earlier.command())
                                .is_some_and(|end| end.starts_with(quote))
                        })
                    })
            })
        })
    })
}

#[test]
fn the_segment_test_reads_joined_paths() {
    let old = Generation::Graft.command();
    let now = Generation::CURRENT.command();
    assert!(names_old_segment(&format!(
        r#"os.path.join(r, "target", "release", "{old}")"#
    )));
    assert!(names_old_segment(&format!("join('debug', '{old}')")));
    assert!(!names_old_segment(&format!(r#"join("release", "{now}")"#)));
    assert!(!names_old_segment(&format!(
        r#""release", "{old}-fixtures""#
    )));
}

/// `.scratch/*/<prefix>*.py`.
fn scratch_scripts(root: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for scratch in fs::read_dir(root.join(".scratch")).expect(".scratch reads") {
        let dir = scratch.expect("a dir entry").path();
        if !dir.is_dir() {
            continue; // a plain file in .scratch: no scripts under it
        }
        for entry in fs::read_dir(&dir).expect("a .scratch dir reads") {
            let path = entry.expect("a dir entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with(prefix) && path.extension().is_some_and(|ext| ext == "py") {
                files.push(path);
            }
        }
    }
    files
}

/// `.scratch/*/pty-walk*.py`: the milestones' pty walk scripts.
fn pty_walk_scripts(root: &Path) -> Vec<PathBuf> {
    scratch_scripts(root, "pty-walk")
}

/// `docs/shots/*.py`: the capture drivers kept beside their captures.
fn shot_scripts(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root.join("docs/shots"))
        .expect("docs/shots reads")
        .map(|entry| entry.expect("a dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .collect()
}

/// No environment name keeps the old prefix (m8-polish-and-ship/27b): every
/// file under `crates/` (MAP.md files included) and `scripts/`, the pty-walk
/// scripts, and ARCHITECTURE.md, read as bytes so no file is skipped as not
/// text. The old names are read nowhere; the allow-list is the generation
/// table and its tests, which spell every generation's prefix once
/// (ADR-0057, 2026-10-11 §2).
#[test]
fn no_old_env_name() {
    let root = crates_dir().parent().expect("repo root").to_path_buf();
    let old = concat!("GRAFT", "_").as_bytes();
    let table = crates_dir().join("graft-core/src/on_disk");
    let mut files = Vec::new();
    every_file(&crates_dir(), &mut files);
    files.retain(|file| file.with_extension("") != table && !file.starts_with(&table));
    every_file(&root.join("scripts"), &mut files);
    files.extend(pty_walk_scripts(&root));
    files.push(root.join("ARCHITECTURE.md"));
    assert!(files.len() > 100, "only {} files swept", files.len());
    let found: Vec<String> = files
        .iter()
        .filter(|file| {
            fs::read(file)
                .unwrap_or_else(|e| panic!("{} reads: {e}", file.display()))
                .windows(old.len())
                .any(|w| w == old)
        })
        .map(|file| file.display().to_string())
        .collect();
    assert!(found.is_empty(), "old env names in:\n{}", found.join("\n"));
}

/// Scripts run the current binary: a stale `target/*/trib` or `graft` from
/// an earlier build would otherwise run old code without failing.
#[test]
fn scripts_run_the_command() {
    let root = crates_dir().parent().expect("repo root").to_path_buf();
    let mut files = Vec::new();
    every_file(&root.join("scripts"), &mut files);
    files.extend(pty_walk_scripts(&root));
    files.extend(shot_scripts(&root));
    let mut found = Vec::new();
    for file in &files {
        let Some(text) = read_text(file) else {
            continue; // not text: no command line in it
        };
        for (at, line) in text.lines().enumerate() {
            // `target/<profile>/graft`, or `$CARGO_TARGET_DIR/release/graft`.
            let stale_path = ["target/", "/release/", "/debug/"].iter().any(|lead| {
                line.match_indices(lead).any(|(i, _)| {
                    let rest = &line[i + lead.len()..];
                    let tail = if *lead == "target/" {
                        rest.split_once('/').map_or("", |(_, tail)| tail)
                    } else {
                        rest
                    };
                    names_old_binary(tail)
                })
            });
            let bin_flag = line
                .match_indices("--bin ")
                .any(|(i, _)| names_old_binary(&line[i + 6..]));
            if stale_path || names_old_segment(line) || bin_flag {
                found.push(format!("{}:{}: {}", file.display(), at + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "scripts run the old binary:\n{}",
        found.join("\n")
    );
}

/// No live file names the old package: `cargo … -p` with the old name stopped working
/// when the package became `mergewright` (49). Records keep it and are not
/// swept: `docs/log/`, `docs/reviews/`, `docs/adr/`, `docs/gaps/`,
/// `docs/perf/`, `docs/design-notes/`, past `docs/shots/` captures,
/// `.scratch/*/issues/` and `.scratch/*/scouts/`, KNOWN_GAPS.md and
/// CHANGELOG.md say what was run then; `.claude/agents/` is the user's.
#[test]
fn no_old_package_spec() {
    let root = crates_dir().parent().expect("repo root").to_path_buf();
    let mut files = Vec::new();
    every_file(&crates_dir(), &mut files);
    every_file(&root.join("scripts"), &mut files);
    every_file(&root.join("docs/capabilities"), &mut files);
    files.extend(scratch_scripts(&root, "pty-"));
    files.extend(shot_scripts(&root));
    for doc in [
        "README.md",
        "TESTING.md",
        "CAPABILITIES.md",
        "docs/generated/capabilities.html",
        ".config/nextest.toml",
    ] {
        let path = root.join(doc);
        assert!(
            path.is_file(),
            "{doc} is missing: the guard would read nothing"
        );
        files.push(path);
    }
    // Built at run time so this file does not match itself.
    let pkg = Generation::Graft.dir();
    let whole = [
        format!("-p {pkg}"),
        format!("-p{pkg}"),
        format!("--package {pkg}"),
        format!("--package={pkg}"),
    ];
    let anywhere = [format!("package({pkg})"), format!("binary_id({pkg}::")];
    let mut found = Vec::new();
    for file in &files {
        let Some(text) = read_text(file) else {
            continue; // not text: no command line in it
        };
        for (at, line) in text.lines().enumerate() {
            let whole_word = whole.iter().any(|spec| {
                line.match_indices(spec.as_str()).any(|(i, _)| {
                    !line[i + spec.len()..]
                        .starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '-')
                })
            });
            if whole_word || anywhere.iter().any(|spec| line.contains(spec.as_str())) {
                found.push(format!("{}:{}: {}", file.display(), at + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "the old package:\n{}", found.join("\n"));
}
