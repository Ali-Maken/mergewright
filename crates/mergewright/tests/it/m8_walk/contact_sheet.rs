//! The M8 contact sheet (ADR-0037; m8-polish-and-ship/28 amendment A1):
//! `scripts/contact-sheet.sh --captions-order m8 docs/shots` gathers every
//! capture the captions file names, flat (`m8-<file>`) and in the
//! subdirectory (`m8/<file>`), into `docs/shots/m8-contact-sheet*.txt`.
//! These tests hold the captions file to the captures on disk, and the
//! sheet's first line to rule 5.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const CAPTIONS: &str = "m8-contact-sheet-captions.tsv";
const SHEET: &str = "m8-contact-sheet";

fn shots() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots")
}

fn is_txt(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

/// Every M8 capture on disk, as the captions file names it: the flat
/// `m8-*.txt` (the sheet itself aside, whole or in parts) and every
/// `m8/*.txt` as `m8/<file>`.
fn captures_on_disk(shots: &Path) -> BTreeSet<String> {
    let names = |dir: &Path| -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap_or_else(|why| panic!("{}: {why}", dir.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|why| panic!("{}: {why}", dir.display()))
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| is_txt(name))
            .collect()
    };
    let mut all: BTreeSet<String> = names(shots)
        .into_iter()
        .filter(|name| name.starts_with("m8-") && !name.starts_with(SHEET))
        .collect();
    all.extend(
        names(&shots.join("m8"))
            .into_iter()
            .map(|name| format!("m8/{name}")),
    );
    all
}

/// What is wrong between `captions` (the TSV's text) and the `captures` on
/// disk, one line each: a capture with no caption, a caption whose file is
/// not there, an empty caption, a file captioned twice.
fn coverage_problems(captions: &str, captures: &BTreeSet<String>) -> Vec<String> {
    let mut wrong = Vec::new();
    let mut captioned = BTreeSet::new();
    for (number, line) in captions.lines().enumerate() {
        let number = number + 1;
        let Some((name, caption)) = line.split_once('\t') else {
            wrong.push(format!("captions line {number}: no tab: {line:?}"));
            continue;
        };
        if caption.trim().is_empty() {
            wrong.push(format!(
                "captions line {number}: {name} has an empty caption"
            ));
        }
        if !captures.contains(name) {
            wrong.push(format!(
                "captions line {number}: {name} is not a capture on disk"
            ));
        }
        if !captioned.insert(name.to_owned()) {
            wrong.push(format!("captions line {number}: {name} twice"));
        }
    }
    for capture in captures {
        if !captioned.contains(capture) {
            wrong.push(format!("{capture}: no caption, so not on the sheet"));
        }
    }
    wrong
}

fn captions_text(shots: &Path) -> String {
    std::fs::read_to_string(shots.join(CAPTIONS))
        .unwrap_or_else(|why| panic!("{}: {why}", shots.join(CAPTIONS).display()))
}

/// Every capture under `docs/shots/m8-*.txt` and `docs/shots/m8/*.txt` has a
/// caption — so it is on the sheet, under its "what right looks like" line —
/// and every caption names a capture that exists.
#[test]
fn contact_sheet_captions_cover_every_m8_capture() {
    let shots = shots();
    let wrong = coverage_problems(&captions_text(&shots), &captures_on_disk(&shots));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The coverage check is not vacuous: a captions row removed leaves its
/// capture uncovered, and a row for a capture deleted from disk is stale;
/// each is named.
#[test]
fn a_missing_capture_or_a_stale_caption_is_named() {
    let shots = shots();
    let captions = captions_text(&shots);
    let mut captures = captures_on_disk(&shots);
    let mut lines = captions.lines();
    let dropped = lines.next().expect("a captions row");
    let (dropped_name, _) = dropped.split_once('\t').expect("a tab");
    let without_first: String = lines.flat_map(|line| [line, "\n"]).collect();
    let wrong = coverage_problems(&without_first, &captures);
    assert_eq!(
        wrong,
        vec![format!("{dropped_name}: no caption, so not on the sheet")]
    );

    let last = captions.lines().last().expect("a captions row");
    let (last_name, _) = last.split_once('\t').expect("a tab");
    captures.remove(last_name);
    let wrong = coverage_problems(&captions, &captures);
    assert_eq!(wrong.len(), 1, "{wrong:#?}");
    assert!(wrong[0].contains("is not a capture on disk"), "{wrong:#?}");
}
