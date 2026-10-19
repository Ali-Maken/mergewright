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

/// Opened at 120 columns, up to rest: the walk complete, every branch's
/// push state read.
fn opened(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, ROWS);
    let state = app.pump(state, "every push state", |state| {
        let frame = drawn(state, 120);
        frame.iter().any(|line| line.contains("\u{2191}2 to push"))
            && frame.iter().any(|line| line.contains("not pushed yet"))
    });
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
    let frame = drawn(&state, 120);

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
    let release = line_with(&frame, "wallet-release is here");
    assert!(
        release.contains("wallet-release is here  \u{2191}2 to push"),
        "{release:?}"
    );
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
    let frame = drawn(&state, 120);
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
    let frame = drawn(state, 120);
    let y = frame
        .iter()
        // The subject yields to the labels; the fold tail names the branch.
        .position(|line| line.contains("from sara/fix-toc"))
        .unwrap_or_else(|| panic!("the merge is drawn\n{}", frame.join("\n")));
    let row = u16::try_from(y).expect("short screen");
    let state = app.mouse_down(state, MouseButton::Left, 40, row);
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the merged commit drawn", |state| {
        drawn(state, 120)
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
                 message, then its labels muted two cells after it (`↑1 to push` on HEAD's \
                 row, `wallet-release is here  ↑2 to push`, `not pushed yet` on spike/cache, \
                 `tag v1.2`); who (`you` bold on your rows, else a first name) and age \
                 right-aligned; no SHA, no `[…]`. Under the cursor row (the opened merge) \
                 its second line (07b, G9): `you · N hours ago · made on feature/receipts · \
                 merged 1 from fix-toc · <sha7>` (made on: the lane's branch, as the branch \
                 column names it), muted, the lanes straight through it.\n\n{}\n",
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

/// The cursor's second line next to the main mock (07b, G9): on his own
/// row, on a teammate's row inside the opened merge, and on the opened
/// merge row.
#[test]
fn capture_the_cursor_rows_second_line() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let mut shots = mergewright_fixtures::Captures::new();
    let own = drawn(&state, 120);
    let merge = open_the_named_merge(&app, &state);
    let frame = drawn(&merge, 120);
    let y = frame
        .iter()
        .position(|line| line.contains("toc: fix the anchors"))
        .expect("the inner row is drawn");
    let teammate = app.mouse_down(
        &merge,
        MouseButton::Left,
        40,
        u16::try_from(y).expect("short"),
    );
    let teammate = at_rest(&app, teammate);
    // A known merge names what it merged by its last segment (G9).
    assert!(
        frame
            .iter()
            .any(|line| line.contains(" \u{b7} merged 1 from fix-toc \u{b7} ")),
        "{}",
        frame.join("\n")
    );
    for (name, frame, right) in [
        (
            "main-cursor-own-120.txt",
            own,
            "the cursor on HEAD's row `Draw the receipts list`; \
          under it `you · N hours ago · made on feature/receipts · <sha7>`",
        ),
        (
            "main-cursor-teammate-120.txt",
            drawn(&teammate, 120),
            "the cursor on Sara's row \
          inside the opened merge; under it `Sara · N hours ago · made on sara/fix-toc · \
          <sha7>`",
        ),
        (
            "main-opened-merge-120.txt",
            frame,
            "the cursor on the opened merge row; under it \
          `you · N hours ago · made on feature/receipts · merged 1 from fix-toc · <sha7>` \
          (the lane's branch, as the branch column names it)",
        ),
    ] {
        shots.check(
            shot(name),
            format!(
                "{HEAD_NOTE}\n120x{ROWS}. What right looks like: {right}; muted, no bold, \
                 the lanes straight down through it (no node, no connector); only the \
                 cursor row has one; never `reading…`.\n\n{}\n",
                frame.join("\n")
            ),
        );
    }
    drop(teammate);
    app.close();
    shots.finish();
}

fn shot(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/ease-pass")
        .join(name)
}
