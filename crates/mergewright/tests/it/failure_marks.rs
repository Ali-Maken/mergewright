//! ease-pass 26: the failure mark is the renderer's. These scan the source for
//! the ways a producer could still type one, and pin each Sticky site by its kind
//! (ease-pass 28).
#![allow(
    clippy::expect_used,
    reason = "an unreadable workspace tree has nothing to scan; the panic names why"
)]

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/mergewright is two levels below the root")
        .to_path_buf()
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn is_test_file(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == "tests")
        || path.file_name().is_some_and(|n| n == "tests.rs")
}

/// Non-test files under `rel` (from the repo root), with their text.
fn production(rel: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(&root().join(rel), &mut files);
    files
        .into_iter()
        .filter(|path| !is_test_file(path))
        .map(|path| {
            let text = fs::read_to_string(&path).expect("read source");
            (path.display().to_string(), text)
        })
        .collect()
}

const MARKS: [char; 5] = ['⊗', '✗', '✘', '⚠', '!'];

/// Producers: every `Status::Failed(..)` constructor passes words only.
#[test]
fn no_failed_status_producer_pre_marks() {
    let mut seen = 0;
    let mut marked = Vec::new();
    for (path, text) in production("crates/mergewright/src") {
        for (at, _) in text.match_indices("Status::Failed(") {
            seen += 1;
            let rest = &text[at + "Status::Failed(".len()..];
            let rest = rest.strip_prefix("format!(").unwrap_or(rest);
            if let Some(words) = rest.strip_prefix('"')
                && (words.starts_with("(x)") || words.starts_with(MARKS))
            {
                marked.push(format!("{path}: {}", &words[..words.len().min(30)]));
            }
        }
    }
    assert!(seen >= 20, "the scan found only {seen} producers");
    assert!(marked.is_empty(), "pre-marked producers: {marked:#?}");
}

/// The grep box: a literal `✗ ✘` outside tests is a lost/drop mark, never a
/// failure (`×` is a count, e.g. `lease ×2`, and is not scanned). `view/status.rs` holds
/// the invariant test that asserts `✗` is absent. Each is listed with its verdict; a new one fails until judged.
#[test]
fn every_cross_literal_is_a_lost_or_drop_mark() {
    let allowed = [
        ("journal_screen/pieces.rs", "lost: `└ ✗ lost`"),
        ("journal_screen/legend.rs", "lost: legend"),
        ("rewrite_editor/draw.rs", "drop: `drop ✗`"),
    ];
    let mut stray = Vec::new();
    for rel in ["crates/mergewright/src", "crates/mergewright-ui/src"] {
        for (path, text) in production(rel) {
            for (number, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains(['✗', '✘', '\u{2717}'])
                    && !allowed.iter().any(|(file, _)| path.ends_with(file))
                    && !path.ends_with("view/status.rs")
                {
                    stray.push(format!("{path}:{}: {}", number + 1, line.trim()));
                }
            }
        }
    }
    assert!(stray.is_empty(), "unjudged cross literals: {stray:#?}");
}

#[derive(Clone, Copy)]
enum Kind {
    Failure,
    Notice,
    /// Both kinds live here, split by a condition the snippet names.
    Split,
}

/// Sticky sites whose words read like a failure, each judged (ease-pass 28,
/// Q1). `Failure` sites build `Status::sticky_failure` (or `kind: Failure`);
/// `Notice` sites build `Status::sticky`. Changing one is a decision: it
/// moves in this table too, or the test fails.
/// Every site judged, as `(file, snippet, kind)`.
use Kind::{Failure, Notice, Split};

const SITES: [(&str, &str, Kind); 13] = [
    (
        "update/answer.rs",
        "Err(why) => next.status = Status::sticky_failure(why)",
        Failure,
    ),
    (
        "update/answers.rs",
        "next.status = Status::sticky_failure(why);",
        Failure,
    ),
    (
        "update/journal/answers.rs",
        "Err(why) => state.status = Status::sticky(why.clone())",
        Notice,
    ),
    ("update/undo/answers.rs", "Status::sticky(why)", Notice),
    (
        "update/undo/answers.rs",
        "state.status = Status::sticky_failure(error);",
        Failure,
    ),
    (
        "update/commit_editor/returned.rs",
        "Err(why) => state.status = Status::sticky_failure(why)",
        Failure,
    ),
    (
        "update/commit_editor/open.rs",
        "Status::sticky_failure(format!(\"amend: {why}\"))",
        Failure,
    ),
    (
        "update/amend_into.rs",
        "Status::sticky_failure(format!(\"couldn't plan the put-in: {why}\"))",
        Failure,
    ),
    (
        "update/place/answer.rs",
        "state.status = Status::sticky(no_blame_words(why));",
        Notice,
    ),
    ("update/operate/failure.rs", "if refused.is_some() {", Split),
    (
        "update/operate/failure.rs",
        "Status::sticky_failure(error)",
        Split,
    ),
    (
        "event_loop/running/frame.rs",
        "Status::sticky_failure(format!(\"trace: {reason}\"))",
        Failure,
    ),
    (
        "update/remote/in_flight.rs",
        "kind: ProblemKind::Failure",
        Failure,
    ),
];

#[test]
fn sticky_sites_are_pinned_by_kind() {
    let base = root().join("crates/mergewright/src");
    for (file, snippet, kind) in SITES {
        let text = fs::read_to_string(base.join(file)).expect("read site");
        assert!(
            text.contains(snippet),
            "{file} no longer has `{snippet}`: re-judge it"
        );
        let wrong = match kind {
            Failure => snippet.replace("sticky_failure(", "sticky("),
            Notice => snippet.replace("sticky(", "sticky_failure("),
            Split => snippet.to_owned(),
        };
        assert!(
            wrong == snippet || !text.contains(&wrong),
            "{file}: `{wrong}` is the other kind"
        );
    }
}
