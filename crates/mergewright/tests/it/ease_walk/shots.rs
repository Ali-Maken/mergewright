//! The ease-pass captures (ease-pass/14c): each stays under the 200 KB cap, has
//! one caption line with a mock path that exists, and the contact sheet
//! names every capture with no `NOT RECORDED`.
//!
//! The captures themselves are re-taken by the walks that own them (the
//! `check_shot` calls in `graph_rows_tests`, `commit_screen`, `finish` …).
//! This file only checks the set. Colour is unverified (rule 5); a capture
//! is a character grid, and whether it looks right is the user's to judge
//! (rule 6).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Hard cap per file, CLAUDE.md "Captures stay small" (1000, not 1024).
const CAP_BYTES: u64 = 200_000;

const CAPTIONS: &str = "ease-pass-contact-sheet-captions.tsv";

fn shots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/ease-pass")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file in the directory that is a capture: not the captions, not a
/// sheet part.
fn captures() -> BTreeSet<String> {
    let dir = shots_dir();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".txt") && !name.starts_with("ease-pass-contact-sheet") {
            names.insert(name);
        }
    }
    names
}

/// The captions file as `(file, caption)` rows, blank lines skipped.
fn captions() -> Vec<(String, String)> {
    let path = shots_dir().join(CAPTIONS);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.split_once('\t') {
            Some((name, caption)) => (name.to_string(), caption.to_string()),
            None => panic!("{CAPTIONS}: no tab in `{line}`"),
        })
        .collect()
}

/// Every capture, and every sheet part, is under 200 KB.
#[test]
fn every_capture_is_under_the_cap() {
    let dir = shots_dir();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
    let mut seen = 0;
    for entry in entries {
        let entry = entry.unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
        let meta = entry
            .metadata()
            .unwrap_or_else(|why| panic!("{:?}: {why}", entry.path()));
        seen += 1;
        assert!(
            meta.len() <= CAP_BYTES,
            "{} is {} bytes, over the {CAP_BYTES} cap",
            entry.path().display(),
            meta.len()
        );
    }
    assert!(seen > 50, "only {seen} files under {}", dir.display());
}

/// The captions file has one row per capture and no row for a missing one;
/// each row names a mock that exists and says colour is unverified or
/// leaves colour out.
#[test]
fn every_capture_has_a_caption_with_its_mock() {
    let named: Vec<(String, String)> = captions();
    let names: BTreeSet<&str> = named.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names.len(), named.len(), "a capture is captioned twice");
    let on_disk = captures();
    for name in &on_disk {
        assert!(names.contains(name.as_str()), "{name} has no caption");
    }
    for (name, caption) in &named {
        assert!(
            on_disk.contains(name),
            "{CAPTIONS} names {name}, which is missing"
        );
        assert!(
            !caption.starts_with("NOT RECORDED"),
            "{name} is not recorded"
        );
        let mock = caption
            .strip_prefix("mock ")
            .and_then(|rest| rest.split_once(": "))
            .map(|(path, _)| path)
            .unwrap_or_else(|| panic!("{name}: the caption does not start `mock <path>: `"));
        assert!(
            repo_root().join(mock).is_file(),
            "{name}: mock {mock} does not exist"
        );
        // Rule 5: the caption of a capture never lets colour pass as checked.
        assert!(
            caption.to_lowercase().contains("unverified")
                || !caption.to_lowercase().contains("colour"),
            "{name}: the caption speaks of colour without saying it is unverified"
        );
    }
}

/// The assembled sheet (whole, or in parts) names every capture under its
/// heading, and no heading reads `NOT RECORDED`.
#[test]
fn the_contact_sheet_names_every_capture() {
    let dir = shots_dir();
    let mut sheet = String::new();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|why| panic!("{}: {why}", dir.display()));
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("ease-pass-contact-sheet") && name.ends_with(".txt") {
            let part =
                std::fs::read_to_string(entry.path()).unwrap_or_else(|why| panic!("{name}: {why}"));
            sheet.push_str(&part);
        }
    }
    assert!(
        !sheet.is_empty(),
        "no contact sheet under {}",
        dir.display()
    );
    assert!(
        !sheet.contains("NOT RECORDED"),
        "the sheet has a capture or a promise that is NOT RECORDED"
    );
    for name in captures() {
        assert!(
            sheet.contains(&format!("docs/shots/ease-pass/{name}\n")),
            "the sheet has no section for {name}: re-run scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass"
        );
        let text =
            std::fs::read_to_string(dir.join(&name)).unwrap_or_else(|why| panic!("{name}: {why}"));
        assert!(
            sheet.contains(&text),
            "the sheet holds an older {name}: re-run scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass"
        );
    }
}
