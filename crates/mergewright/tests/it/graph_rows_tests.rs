//! The graph rows next to the main mock, through the real app (ease-pass
//! 07, G5–G8): a branch column, the message with its labels after it, who
//! and age — no SHA, no `[branch]` badge. A repository shaped like his
//! work repo, with upstreams faked so each local branch has a push state:
//! `HEAD` on `feature/receipts` one ahead, `wallet-release` two ahead and
//! not checked out, `main` and `ops/ci` level with theirs, `spike/cache`
//! never pushed; two authors besides the user; `v1.2` tagged on `main`.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::event::{KeyCode, MouseButton};

use crate::app_harness::App;
use crate::captures::{at_rest, frame_at};

mod cursor_second_line;

const ROWS: u16 = 30;
/// After every commit, so ages read the same.
const NOW: i64 = 1_700_100_000;
/// The user's own address: the fixtures' hermetic author.
const ME: &str = "fixture@example.invalid";
const SARA: &str = "--author=Sara Khan <sara@example.invalid>";
const OMAR: &str = "--author=Omar Haddad <omar@example.invalid>";
const NAMED_MERGE: &str = "Merge branch 'sara/fix-toc' into wallet-release";

/// The history, one command a row, each a minute after the last.
const HISTORY: &[&[&str]] = &[
    &["config", "user.email", ME],
    &["commit", "--quiet", "--allow-empty", "-m", "Add the readme"],
    &["tag", "v1.2"],
    &["switch", "--quiet", "-c", "ops/ci"],
    &[
        "commit",
        "--quiet",
        "--allow-empty",
        OMAR,
        "-m",
        "Pin the CI image",
    ],
    &["switch", "--quiet", "-c", "spike/cache", "main"],
    &[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Try a read cache",
    ],
    &["switch", "--quiet", "-c", "wallet-release", "main"],
    &[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Start the wallet release",
    ],
    &["switch", "--quiet", "-c", "sara/fix-toc"],
    &[
        "commit",
        "--quiet",
        "--allow-empty",
        SARA,
        "-m",
        "toc: fix the anchors",
    ],
    &["switch", "--quiet", "wallet-release"],
    &[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        NAMED_MERGE,
        "sara/fix-toc",
    ],
    &["switch", "--quiet", "-c", "feature/receipts"],
    &[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Draw the receipts list",
    ],
    // Upstreams, faked: a remote that is never reached.
    &["remote", "add", "origin", "/nonexistent"],
];

/// Each upstream's tip, where its branch's push state needs it.
const UPSTREAMS: [(&str, &str); 4] = [
    ("main", "main"),
    ("ops/ci", "ops/ci"),
    ("wallet-release", "wallet-release~1"),
    ("feature/receipts", "feature/receipts~1"),
];

/// The repository above; every command a minute after the last.
fn main_screen_repo() -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repo = scratch.path();
    mergewright_fixtures::git::init_repo(repo).expect("git init");
    let mut at: i64 = 1_700_000_000;
    let mut step = |args: &[&str]| {
        at += 60;
        mergewright_fixtures::git::run_at(repo, args, &format!("@{at} +0000"))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    };
    for args in HISTORY {
        step(args);
    }
    for (branch, at) in UPSTREAMS {
        let remote = format!("refs/remotes/origin/{branch}");
        step(&["update-ref", &remote, at]);
        step(&["config", &format!("branch.{branch}.remote"), "origin"]);
        let merge = format!("refs/heads/{branch}");
        step(&["config", &format!("branch.{branch}.merge"), &merge]);
    }
    scratch
}

fn drawn(state: &AppState, columns: u16) -> Vec<String> {
    frame_at(state, columns, ROWS, Some(Clock::Fixed(NOW)))
}

/// The rows' words at 120, left of the branch pane (ease-pass 09): the
/// pane's own words are not a row's.
fn row_words(state: &AppState) -> Vec<String> {
    drawn(state, 120)
        .iter()
        .map(|line| crate::row_reads::graph_side(line, 120))
        .collect()
}

/// Wide enough for every label here. Wider than his full-screen 187
/// (field notes): there the folded merge's row needs 113 message cells for
/// its subject, its tail and `wallet-release is here  ↑2 to push`, and the
/// label gives way whole (ease-pass 09 ruling W).
const WIDE: u16 = 200;

/// The rows' words at [`WIDE`], left of the branch pane: what G7's labels
/// say. At 120 beside the pane the message column is about 40 cells and
/// the labels give way first, whole (ease-pass 09 ruling W) — so
/// `wallet-release is here  ↑2 to push` is not drawn there, by design.
/// Rows sit on the same lines at any width.
fn label_words(state: &AppState) -> Vec<String> {
    drawn(state, WIDE)
        .iter()
        .map(|line| crate::row_reads::graph_side(line, WIDE))
        .collect()
}

/// Opened at 120 columns, up to rest: the walk complete, every branch's
/// push state read (they arrive with the refs).
fn opened(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, ROWS);
    let state = app.pump(state, "every push state", |state| state.refs.is_some());
    let state = at_rest(&app, state);
    (app, state)
}

fn line_with<'f>(frame: &'f [String], needle: &str) -> &'f str {
    frame
        .iter()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row with {needle:?}\n{}", frame.join("\n")))
}

/// The commit rows: those ending in an age.
fn commit_rows(frame: &[String]) -> Vec<&String> {
    frame
        .iter()
        .filter(|line| {
            line.split_whitespace().last().is_some_and(|age| {
                age == "now"
                    || age
                        .strip_suffix(['m', 'h', 'd', 'w', 'y'])
                        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            })
        })
        .collect()
}

/// G5–G8 on one frame: each push state where it belongs, the tag, who and
/// age, and nothing of the old row (no SHA, no `[…]` badge).
#[test]
fn every_local_branch_says_where_it_stands_and_who_made_each_row() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let frame = label_words(&state);

    let head = line_with(&frame, "Draw the receipts list");
    // 22 cells in a 20-cell column: the name is cut, never ` (you)`.
    assert!(head.contains("feature/recei\u{2026} (you)"), "{head:?}");
    assert!(
        head.contains("Draw the receipts list  \u{2191}1 to push"),
        "{head:?}"
    );
    // Once per run: the release's first commit rides HEAD's lane, under
    // the row that named it, so its branch cell is blank.
    let start = line_with(&frame, "Start the wallet release");
    let before = &start[..start.find("Start the").expect("found by it")];
    assert!(!before.chars().any(char::is_alphabetic), "{start:?}");
    // The merge is wallet-release's tip (ADR-0064): the column names it, so
    // no `is here` label repeats it (as for ops/ci), and the push state stays.
    let release = line_with(&frame, "Merge branch 'sara/fix-toc'");
    assert!(release.contains(" wallet-release "), "{release:?}");
    assert!(release.contains("\u{2191}2 to push"), "{release:?}");
    let spike = line_with(&frame, "Try a read cache");
    assert!(spike.contains("not pushed yet"), "{spike:?}");
    let ci = line_with(&frame, "Pin the CI image");
    assert!(
        !ci.contains("to push") && !ci.contains("not pushed"),
        "{ci:?}"
    );
    // Its lane may be seeded from `origin/ops/ci`, level with it: the
    // column still names the local branch, and no label repeats it.
    assert!(ci.contains(" ops/ci ") && !ci.contains("is here"), "{ci:?}");
    let readme = line_with(&frame, "Add the readme");
    assert!(readme.contains("tag v1.2"), "{readme:?}");
    assert!(
        !readme.contains("main is here  \u{2191}"),
        "level with origin: {readme:?}"
    );

    nothing_of_the_old_row(&frame);
    let who = |row: &str| {
        let words: Vec<&str> = row.split_whitespace().collect();
        words[words.len() - 2].to_owned()
    };
    assert_eq!(who(head), "you", "{head:?}");
    assert_eq!(who(ci), "Omar", "a first name only: {ci:?}");
    drop(state);
    app.close();
}

/// ADR-0064 on the fixture: `feature/receipts` sits on `wallet-release`.
/// The merge row is wallet-release's tip, so the lane takes its name there;
/// the release's first commit continues it and leaves the cell blank
/// (written once where it changes); the remote copies add no `origin/…`.
#[test]
fn a_stacked_branchs_commits_carry_its_own_name() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let frame = label_words(&state);
    let merge = line_with(&frame, "Merge branch 'sara/fix-toc'");
    assert!(merge.contains(" wallet-release "), "{merge:?}");
    let start = line_with(&frame, "Start the wallet release");
    let before = &start[..start.find("Start the").expect("found by it")];
    assert!(!before.chars().any(char::is_alphabetic), "{start:?}");
    // `origin/…` appears only as a label (`origin/x is here`), never as a
    // row's branch.
    for line in commit_rows(&frame) {
        for (at, _) in line.match_indices("origin/") {
            assert!(line[at..].contains(" is here"), "{line:?}");
        }
    }
    drop(state);
    app.close();
}

/// No row draws the old row's `[…]` badge or a short SHA.
fn nothing_of_the_old_row(frame: &[String]) {
    let rows = commit_rows(frame);
    assert!(rows.len() >= 6, "{}", frame.join("\n"));
    for row in &rows {
        assert!(!row.contains('['), "no badge brackets: {row:?}");
        assert!(
            !row.split_whitespace()
                .any(|word| word.len() == 7 && word.chars().all(|c| c.is_ascii_hexdigit())),
            "no short SHA: {row:?}"
        );
    }
}

/// `space` on the folded merge opens it: the row inside names the merged
/// branch by its last segment, `fix-toc`, dim, and reads its author.
#[test]
fn an_opened_merges_row_names_the_merged_branch_by_its_last_segment() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let state = open_the_named_merge(&app, &state);
    let frame = label_words(&state);
    let inside = line_with(&frame, "toc: fix the anchors");
    assert!(inside.contains(" fix-toc "), "{inside:?}");
    // `sara/fix-toc is here` follows as a label: the column comes first.
    assert!(
        inside.find(" fix-toc ") < inside.find("sara/fix-toc"),
        "the last segment only: {inside:?}"
    );
    assert!(inside.contains("Sara"), "{inside:?}");
    drop(state);
    app.close();
}

fn open_the_named_merge(app: &App, state: &AppState) -> AppState {
    let frame = label_words(state);
    let y = frame
        .iter()
        // The fold tail names the branch; at 120 it gives way to the
        // subject, so the merge is found where it has room.
        .position(|line| line.contains("from sara/fix-toc"))
        .unwrap_or_else(|| panic!("the merge is drawn\n{}", frame.join("\n")));
    let row = u16::try_from(y).expect("short screen");
    let state = app.mouse_down(state, MouseButton::Left, 40, row);
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the merged commit drawn", |state| {
        row_words(state)
            .iter()
            .any(|line| line.contains("toc: fix the anchors"))
    });
    at_rest(app, state)
}

const HEAD_NOTE: &str = "ease-pass 07 (G5-G8), drawn by `view::view` from a real AppState over \
     the job pool (crates/mergewright/tests/it/graph_rows_tests.rs): `feature/receipts` \
     (HEAD) 1 ahead of origin, `wallet-release` 2 ahead and not checked out, `main` and \
     `ops/ci` level, `spike/cache` never pushed, `v1.2` on main, Sara and Omar besides you; \
     the merge of `sara/fix-toc` opened with space. Judge beside \
     prototypes/ease-mocks/mergewright-main-screen.html. Not a pty capture: a character grid, \
     colour unverified (CLAUDE.md rule 5); the look is yours to judge (rule 6).";

/// The four captures next to the main mock: 120, 100 and 90 columns, and
/// 120 after `#` (colours off: the same grid; what `#` changes is colour,
/// which a capture cannot show).
#[test]
fn capture_the_main_screen_rows_at_120_100_and_90() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let state = open_the_named_merge(&app, &state);
    // One regeneration pass writes all four.
    let mut shots = mergewright_fixtures::Captures::new();
    for (columns, branch) in [(120u16, 20), (100, 14), (90, 12)] {
        let frame = drawn(&state, columns);
        shots.check(
            shot(&format!("main-{columns}.txt")),
            format!(
                "{HEAD_NOTE}\n{columns}x{ROWS}. What right looks like: a {branch}-cell branch \
                 column after the lanes, naming each run's branch once, in its role's colour, \
                 `(you)` after HEAD's; the opened merge's row reads `fix-toc`, dim; the \
                 message, then the labels that fit, whole, muted two cells after it — beside \
                 the branch pane they give way first, right-most first, and only then is the \
                 subject cut with `…` (ease-pass 09 ruling W; every label is asserted at \
                 200); who (`you` bold on your rows, else a first name) and age \
                 right-aligned; no SHA, no `[…]`. Under the cursor row (the opened merge) \
                 its second line (07b, G9): `you · N hours ago · made on …` (made on: the \
                 lane's branch, as the branch column names it), cut at the pane's rule, \
                 muted, the lanes straight through it.\n\n{}\n",
                frame.join("\n")
            ),
        );
    }
    let off = app.key(&state, KeyCode::Char('#'));
    let frame = drawn(&off, 120);
    shots.check(
        shot("main-colours-off-120.txt"),
        format!(
            "{HEAD_NOTE}\n120x{ROWS}, after `#`. What right looks like: the same grid as \
             main-120.txt — `#` takes colour off and keeps bold and dim, so the roles still \
             read by line shape (┃ │ ║ ┊). Colour unverified (rule 5): this grid cannot show \
             that colour is gone; `view::frame::colours_off_tests` checks the buffer.\n\n{}\n",
            frame.join("\n")
        ),
    );
    drop(off);
    app.close();
    shots.finish();
}

fn shot(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/ease-pass")
        .join(name)
}
