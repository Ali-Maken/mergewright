//! Ease-pass acceptance (ease-pass/14): the three mocks walked in the real
//! app, over real `git`, through `update` and the real job pool.
//!
//! - [`journeys`]: every row of the note's `journeys` fence walked and
//!   counted against it through [`within_the_note`]; one key over fails
//!   ([`one_key_over_fails_both_counted_journeys`]).
//! - [`graph`], [`commit`], [`merge`]: one test per behaviour id of the
//!   note's "Mock behaviours" (G1–G14, C1–C15, M1–M19), named by the id.
//! - [`questions`]: one per answered question, `q<N>_…`.
//! - [`team`]: G12 on 01r's topologies, `g12_…`, and their two captures.
//! - [`fold`]: the fold checks `fold_…` and `picks_shell_merge`.
//! - [`shots`]: every capture under `docs/shots/ease-pass/` has a caption,
//!   a mock path, and stays under 200 KB.
//!
//! Text and shape checks only: **colour is unverified** (rule 5).

mod bar_and_pane;
mod commit;
mod fixture;
mod flow;
mod fold;
mod graph;
mod journeys;
mod merge;
mod merge_finish;
mod questions;
mod questions_late;
mod screen;
mod team;

use std::path::Path;

/// The ease-pass note, from the crate's own directory.
const NOTE: &str = "../../docs/design-notes/ease-pass.md";

/// The fence's opening line.
const FENCE: &str = "```journeys";

/// One row of the note's `journeys` fence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JourneyRow {
    pub id: String,
    pub keys: u32,
    pub list: String,
}

/// Why the fence could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenceError {
    NoFence,
    Unclosed,
    NoHeader,
    /// A row whose keys column is not a number: the line, 1-based within
    /// the fence, and its text.
    BadRow(usize, String),
    Twice(String),
}

/// A walk that took more keys than its row allows, or walked an id the fence
/// has no row for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverCount {
    Over {
        id: String,
        walked: u32,
        allowed: u32,
    },
    NoRow(String),
    Fence(FenceError),
}

/// The rows of the note's `journeys` fence. Its first line is a header
/// naming the columns `id keys list best tool`; each column starts where
/// its header word starts.
pub fn journeys_in(note: &str) -> Result<Vec<JourneyRow>, FenceError> {
    let mut lines = note
        .lines()
        .skip_while(|line| line.trim_end() != FENCE)
        .skip(1);
    let header = lines.next().ok_or(FenceError::NoFence)?;
    let (Some(list_at), Some(best_at)) = (header.find("list"), header.find("best tool")) else {
        return Err(FenceError::NoHeader);
    };
    let mut rows: Vec<JourneyRow> = Vec::new();
    let mut closed = false;
    for (number, line) in lines.enumerate() {
        if line.trim_end() == "```" {
            closed = true;
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let bad = || FenceError::BadRow(number + 2, line.to_owned());
        let chars: Vec<char> = line.chars().collect();
        let column = |from: usize, to: usize| -> String {
            chars
                .get(from.min(chars.len())..to.min(chars.len()))
                .map(|cells| cells.iter().collect::<String>().trim().to_owned())
                .unwrap_or_default()
        };
        let mut words = column(0, list_at)
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .into_iter();
        let (Some(id), Some(keys), None) = (words.next(), words.next(), words.next()) else {
            return Err(bad());
        };
        let keys: u32 = keys.parse().map_err(|_| bad())?;
        if rows.iter().any(|row| row.id == id) {
            return Err(FenceError::Twice(id));
        }
        rows.push(JourneyRow {
            id,
            keys,
            list: column(list_at, best_at),
        });
    }
    if !closed {
        return Err(FenceError::Unclosed);
    }
    if rows.is_empty() && header.trim().is_empty() {
        return Err(FenceError::NoFence);
    }
    Ok(rows)
}

/// `Ok` when a walk of `id` took no more than its row's keys.
pub fn within_the_note(id: &str, walked_keys: u32, note: &str) -> Result<(), OverCount> {
    let rows = journeys_in(note).map_err(OverCount::Fence)?;
    let row = rows
        .iter()
        .find(|row| row.id == id)
        .ok_or_else(|| OverCount::NoRow(id.to_owned()))?;
    if walked_keys > row.keys {
        return Err(OverCount::Over {
            id: id.to_owned(),
            walked: walked_keys,
            allowed: row.keys,
        });
    }
    Ok(())
}

/// Each fence row's walk: its id and the test in [`journeys`] that walks it.
const WALKS: &[(&str, &str)] = &[
    ("commit-all-subject", "commit_all_subject"),
    ("commit-with-body", "commit_with_body"),
    ("commit-leave-first", "commit_leave_first"),
    ("merge-one-clash", "merge_one_clash"),
    ("merge-clean", "merge_clean"),
    ("merge-three-clashes", "merge_three_clashes"),
    ("undo-a-pick", "undo_a_pick"),
    ("pause-and-resume", "pause_and_resume_on_the_merge"),
    ("open-folded-merge", "open_folded_merge"),
];

/// The keys the fence allows `id`.
pub fn allowed(id: &str) -> u32 {
    journeys_in(&note())
        .unwrap_or_else(|why| panic!("the fence: {why:?}"))
        .into_iter()
        .find(|row| row.id == id)
        .map_or_else(|| panic!("no fence row {id:?}"), |row| row.keys)
}

/// A finished walk of `id` that pressed `route`: prints `journey <id>
/// <walked>/<allowed>` and fails when it is over the fence's count.
pub fn counted(id: &str, route: &[&str]) {
    let walked = u32::try_from(route.len()).unwrap_or(u32::MAX);
    println!("journey {id} {walked}/{} {route:?}", allowed(id));
    if let Err(over) = within_the_note(id, walked, &note()) {
        panic!("{over:?}: {route:?}");
    }
}

/// The note's text.
pub fn note() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()))
}

#[test]
fn the_fence_reads_nine_rows_with_their_lists() {
    let rows = journeys_in(&note()).expect("the fence parses");
    assert_eq!(rows.len(), 9, "{rows:#?}");
    let commit = rows
        .iter()
        .find(|row| row.id == "commit-all-subject")
        .expect("a row");
    assert_eq!((commit.keys, commit.list.as_str()), (3, "c type ⏎"));
    let merge = rows
        .iter()
        .find(|row| row.id == "merge-one-clash")
        .expect("a row");
    assert_eq!((merge.keys, merge.list.as_str()), (5, "m y o y y"));
}

/// The negative test: one key over either counted journey fails, naming
/// both numbers; at the count it passes.
#[test]
fn one_key_over_fails_both_counted_journeys() {
    let note = note();
    for (id, keys) in [("commit-all-subject", 3), ("merge-one-clash", 5)] {
        assert_eq!(within_the_note(id, keys, &note), Ok(()));
        assert_eq!(
            within_the_note(id, keys + 1, &note),
            Err(OverCount::Over {
                id: id.to_owned(),
                walked: keys + 1,
                allowed: keys,
            })
        );
    }
    assert_eq!(
        within_the_note("no-such-journey", 1, &note),
        Err(OverCount::NoRow("no-such-journey".to_owned()))
    );
}

/// Every fence row has a walk in `journeys.rs` that counts it, and every
/// walk names a fence row.
#[test]
fn every_ease_journey_id_has_a_walk() {
    let rows = journeys_in(&note()).expect("the fence parses");
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/it/ease_walk/journeys.rs"),
    )
    .expect("journeys.rs reads");
    for row in &rows {
        let (_, test) = WALKS
            .iter()
            .find(|(id, _)| *id == row.id)
            .unwrap_or_else(|| panic!("no walk for {:?}", row.id));
        assert!(
            source.contains(&format!("#[test]\nfn {test}()")),
            "journeys.rs has no test {test}"
        );
        let plain = format!("counted(\"{}\"", row.id);
        let wrapped = format!("counted(\n        \"{}\"", row.id);
        assert!(
            source.contains(&plain) || source.contains(&wrapped),
            "journeys.rs never counts {:?}",
            row.id
        );
    }
    for (id, _) in WALKS {
        assert!(rows.iter().any(|row| row.id == *id), "{id} is no fence row");
    }
}

#[test]
fn a_bad_fence_is_named() {
    assert_eq!(journeys_in("no fence"), Err(FenceError::NoFence));
    let unclosed = "```journeys\nid  keys  list  best tool\na   1     x     y\n";
    assert_eq!(journeys_in(unclosed), Err(FenceError::Unclosed));
    let bad = "```journeys\nid  keys  list  best tool\na   one   x     y\n```\n";
    assert!(matches!(journeys_in(bad), Err(FenceError::BadRow(2, _))));
}
