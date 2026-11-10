//! The M3 acceptance walk (m3-working-tree-undo-and-stash/07): one scenario
//! per promise the milestone makes, each a scratch repository, a scripted
//! dirty state, the real app driven by keys over the real job pool and real
//! `git`, and then **repository truth read back by running git** — never by
//! trusting `AppState`. `examples/m3_walk.rs` includes this file by `#[path]`
//! and prints one line per scenario; `every scenario` below runs the same
//! list under `cargo test`, one test per scenario so the fast tier stays
//! parallel and no single test nears the 10 s line.
//!
//! The spec pictured `Harness::prelude` fed through `run_headless`. Neither
//! fits: `run_headless` takes no keys, and the prelude is a flat key list
//! with no way to wait for a write to land or to edit a file between two
//! keys. So the walk drives `update` directly, the way `tests/it/undo_by_key`
//! does, through [`Step`]s that can wait, mark and shell out. The prelude is
//! still what the pty captures use ([`Capture::keys`]), now that it waits for
//! a write in flight (`update::prelude`).
#![allow(
    clippy::expect_used,
    reason = "a walk step that cannot proceed has nothing left to prove; the panic names why"
)]

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mergewright::update::detail::Loading;
use mergewright::update::{AppEvent, AppState, Command, Status, update};
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_fixtures::Shape;

use mergewright::jobs::Dispatch as _;

use crate::app_harness::App;
use crate::repo_truth::{
    RepoTruth, blob_of, check, diff_truth, fsck_clean, git_path, git_truth,
    neutralise_global_config, run_git, same,
};
use mergewright_core::on_disk::{DIR, Own};

/// What one scenario's `setup` and a `Shell` step run: plain files and git.
pub type Fix = fn(&Path) -> Result<(), String>;
/// A wait: `true` once the state has what the next key needs.
pub type Ready = fn(&AppState) -> bool;
/// A fact about the UI state that git cannot see (a notice, a refusal).
pub type See = fn(&AppState) -> Result<(), String>;
/// The scenario's verdict, from git and the marks the steps took.
pub type Expect = fn(&Path, &Marks) -> Result<(), String>;

/// One thing the walk does to the running app, in order.
pub enum Step {
    /// Press one key.
    Key(KeyPress),
    /// Type a string, one character key at a time.
    Type(&'static str),
    /// Feed job answers until the state is ready for the next key.
    Await(&'static str, Ready),
    /// A write was queued by the last key: run it to its answer and insist
    /// it succeeded (`Status::Done`).
    Write(&'static str),
    /// Move the file-list cursor onto the row named, the `nth` (0-based) of
    /// that name — a file with staged and unstaged changes is listed twice.
    CursorTo(&'static str, usize),
    /// The terminal regained focus: the status is re-read, the walk is not
    /// restarted. Waits for the new status.
    Focus,
    /// Change the repository out of band, as an editor would.
    Shell(Fix),
    /// Record repository truth under this name for `expect`.
    Mark(&'static str),
    /// Check a fact only the UI state holds.
    See(&'static str, See),
    /// `Tab` until the subject has focus (the commit screen's `⇥` walks
    /// subject → body → files), then `Enter`.
    CommitFromSubject,
}

/// A pty capture of this scenario (the pieces the user judges by eye).
pub struct Capture {
    /// `m3-<nn>-<scenario>`, the file's stem under `docs/shots/`.
    pub stem: &'static str,
    /// Driven headless first, so the captured run starts from their result
    /// (a commit to undo, an edit to drift).
    pub headless: Vec<Step>,
    /// The captured run's `Harness::prelude`.
    pub keys: Vec<KeyPress>,
    /// Text the settled frame must contain, or the capture says it did not.
    pub on_screen: &'static str,
}

/// One scenario of the walk.
pub struct Scenario {
    /// Its name on the command line and in `TESTING.md`.
    pub name: &'static str,
    /// The corpus shape copied into the scratch directory.
    pub shape: Shape,
    /// The dirty state, scripted with plain files and git.
    pub setup: Fix,
    /// The directory the app opens, relative to the scratch root, when not
    /// the root itself (a linked worktree).
    pub open: Option<&'static str>,
    /// What the walk does.
    pub steps: Vec<Step>,
    /// What must then be true, by git.
    pub expect: Expect,
    /// Its capture, when it has a visual result.
    pub capture: Option<Capture>,
}

/// The truths a run recorded: after setup, and at every `Mark`.
#[derive(Default)]
pub struct Marks {
    /// After `setup`, before the app opened.
    pub before: RepoTruth,
    /// At each `Step::Mark`.
    pub at: BTreeMap<&'static str, RepoTruth>,
}

impl Marks {
    /// The truth recorded under `name`.
    pub fn get(&self, name: &str) -> Result<&RepoTruth, String> {
        self.at
            .get(name)
            .ok_or_else(|| format!("no mark named {name}"))
    }
}

// ───────────────────────────── running a scenario ─────────────────────────────

/// Copy the shape, script the state, drive the app, then judge by git;
/// every scenario ends with `fsck` clean and one backup ref per journal
/// entry that took one.
pub fn run_scenario(scenario: &Scenario) -> Result<(), String> {
    let scratch = mergewright_fixtures::try_copy_into_temp(scenario.shape)
        .map_err(|why| format!("fixture {}: {why:?}", scenario.shape.name()))?;
    let root = scratch.path().to_path_buf();
    let repo = scenario
        .open
        .map_or_else(|| root.clone(), |sub| root.join(sub));
    let mut marks = Marks::default();
    let driven = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        // Before the setup runs: a linked worktree it creates shares this
        // config, and every command below is already a git write.
        neutralise_global_config(&root)?;
        (scenario.setup)(&root)?;
        marks.before = git_truth(&repo)?;
        drive_app(&repo, &scenario.steps, &mut marks)
    }));
    match driven {
        Ok(result) => result?,
        Err(payload) => return Err(format!("panicked: {}", panic_text(payload.as_ref()))),
    }
    (scenario.expect)(&repo, &marks)?;
    fsck_clean(&repo)?;
    backups_match_journal(&repo)
}

/// Set a scenario up for a pty capture: the shape, its setup and the
/// capture's headless steps. Hands back the scratch (alive while the
/// captured run lasts) and the directory to open.
pub fn prepare_capture(
    scenario: &Scenario,
) -> Result<(mergewright_fixtures::Scratch, PathBuf), String> {
    let capture = scenario
        .capture
        .as_ref()
        .ok_or_else(|| format!("{} has no capture", scenario.name))?;
    let scratch = mergewright_fixtures::try_copy_into_temp(scenario.shape)
        .map_err(|why| format!("fixture {}: {why:?}", scenario.shape.name()))?;
    let root = scratch.path().to_path_buf();
    let repo = scenario
        .open
        .map_or_else(|| root.clone(), |sub| root.join(sub));
    neutralise_global_config(&root)?;
    (scenario.setup)(&root)?;
    if !capture.headless.is_empty() {
        drive_app(&repo, &capture.headless, &mut Marks::default())?;
    }
    Ok((scratch, repo))
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "(no message)".to_owned())
}

/// Open the app on `repo`, run the steps, and always close the job pool.
fn drive_app(repo: &Path, steps: &[Step], marks: &mut Marks) -> Result<(), String> {
    let (app, state) = App::open(repo);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let state = drive(&app, state, repo, steps, marks)?;
        no_bookkeeping_in_the_graph(&state)?;
        Ok(state)
    }));
    app.close();
    match outcome {
        Ok(result) => result.map(drop),
        Err(payload) => Err(format!("panicked: {}", panic_text(payload.as_ref()))),
    }
}

/// Nothing Mergewright wrote for itself is drawn as history (m3-07 review, C1).
///
/// Every undoable write leaves a backup chain under
/// `<prefix in force>backup/<worktree>/<ulid>` (`on_disk::REF_PREFIX`), kept indefinitely by design. They
/// used to seed the history walk, which put two commits titled
/// `mergewright <ulid> …` **above `HEAD`** per operation, forever — so a day's
/// staging buried the branch the graph is about, and
/// `docs/shots/m3-09-undo-drift-box-80x24.txt` recorded it.
///
/// Checked after every scenario rather than in one of them: the rule is
/// about the whole app, and the scenarios that make backups are exactly the
/// ones that would stop making them if this were opt-in.
fn no_bookkeeping_in_the_graph(state: &AppState) -> Result<(), String> {
    let Some(window) = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
    else {
        return Ok(()); // The scenario never drew history; nothing to judge.
    };
    let drawn: Vec<String> = window
        .rows
        .iter()
        .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned())
        .collect();
    check(
        !drawn.iter().any(|subject| Own::is_message(subject)),
        || format!("a backup commit is drawn in the history window: {drawn:?}"),
    )
}

fn drive(
    app: &App,
    mut state: AppState,
    repo: &Path,
    steps: &[Step],
    marks: &mut Marks,
) -> Result<AppState, String> {
    for step in steps {
        state = match step {
            Step::Key(key) => app.press(&state, *key),
            Step::Type(text) => text
                .chars()
                .fold(state, |state, c| app.key(&state, KeyCode::Char(c))),
            Step::Await(what, ready) => app.pump(state, what, ready),
            Step::Write(what) => settle_write(app, state, what)?,
            Step::CursorTo(name, nth) => cursor_to(app, state, name, *nth)?,
            Step::Focus => focus_gained(app, &state)?,
            Step::Shell(fix) => {
                fix(repo)?;
                state
            }
            Step::Mark(name) => {
                marks.at.insert(name, git_truth(repo)?);
                state
            }
            Step::See(what, see) => {
                see(&state).map_err(|why| format!("{what}: {why}"))?;
                state
            }
            Step::CommitFromSubject => tab_to_the_subject_and_enter(app, state)?,
        };
    }
    Ok(state)
}

fn settle_write(app: &App, state: AppState, what: &str) -> Result<AppState, String> {
    check(state.operating.is_some(), || {
        format!("{what}: no write was queued; status {:?}", state.status)
    })?;
    let state = app.pump(state, what, |state| state.operating.is_none());
    match &state.status {
        Status::Done { .. } => Ok(state),
        other => Err(format!(
            "{what} did not succeed: {other:?}; confirm box: {}",
            state.confirm.is_some()
        )),
    }
}

/// `Tab` back to the subject, then `Enter` (ease-pass/08: `⏎` from the
/// subject commits).
fn tab_to_the_subject_and_enter(app: &App, state: AppState) -> Result<AppState, String> {
    let mut state = state;
    for _ in 0..3 {
        let focus = state
            .commit_editor
            .as_ref()
            .ok_or("the commit editor is not open")?
            .focus;
        if focus == mergewright_ui::commit_editor::CommitFocus::Subject {
            return Ok(app.key(&state, KeyCode::Enter));
        }
        state = app.key(&state, KeyCode::Tab);
    }
    Err("`Tab` never reached the subject".to_owned())
}

fn file_rows(state: &AppState) -> Result<(Vec<String>, usize), String> {
    match &state.detail.files {
        Loading::Ready(files) => Ok((
            files
                .rows
                .iter()
                .map(|row| row.name.as_str().to_owned())
                .collect(),
            files.cursor,
        )),
        other => Err(format!("the file list is not ready: {other:?}")),
    }
}

fn cursor_to(app: &App, state: AppState, name: &str, nth: usize) -> Result<AppState, String> {
    let (rows, _) = file_rows(&state)?;
    let want = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.as_str() == name)
        .nth(nth)
        .map(|(at, _)| at)
        .ok_or_else(|| format!("no row #{nth} named {name} among {rows:?}"))?;
    let mut state = state;
    for _ in 0..=rows.len() * 2 {
        let (_, at) = file_rows(&state)?;
        if at == want {
            return Ok(state);
        }
        let step = if at < want { 'j' } else { 'k' };
        state = app.key(&state, KeyCode::Char(step));
    }
    Err(format!("the cursor never reached row #{nth} {name}"))
}

/// `FocusGained` re-reads the status and nothing else; wait for the answer.
fn focus_gained(app: &App, state: &AppState) -> Result<AppState, String> {
    let old = state.working_tree.clone();
    let (next, commands) = update(state, AppEvent::Core(Event::FocusGained));
    check(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        || format!("focus did not re-read the working tree: {commands:?}"),
    )?;
    check(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..))),
        || format!("focus restarted the history walk: {commands:?}"),
    )?;
    for command in commands {
        app.jobs.dispatch(command);
    }
    Ok(app.pump(next, "the status after focus", move |state| {
        match (&old, &state.working_tree) {
            (Some(old), Some(new)) => !Arc::ptr_eq(old, new),
            (None, Some(_)) => true,
            _ => false,
        }
    }))
}

/// One `<prefix in force>backup/…` ref per journal entry whose operation took a
/// backup (touches the index or the worktree, or pins an object).
fn backups_match_journal(repo: &Path) -> Result<(), String> {
    let path = git_path(repo, &format!("{DIR}/undo-journal.jsonl"))?;
    let expected = match std::fs::read_to_string(&path) {
        Ok(text) => text
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|line| line["status"] == "pending")
            .filter(|line| {
                line["touches"]["index"] == true
                    || line["touches"]["worktree"] == true
                    || line["pins"].as_array().is_some_and(|pins| !pins.is_empty())
            })
            .count(),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => 0,
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    let refs = run_git(repo, &["for-each-ref", &format!("refs/{DIR}/backup")])?
        .lines()
        .count();
    check(refs == expected, || {
        format!("{refs} backup refs for {expected} journal entries that took a backup")
    })
}

// ───────────────────────────── the scenarios ─────────────────────────────

/// Sixteen lines: an edit to the first and the last makes two hunks under
/// git's default three lines of context.
const BASE: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nsixteen\n";
/// Both hunks.
const EDITED: &str = "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nSIXTEEN\n";
/// The first hunk only.
const FIRST_HUNK: &str = "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nsixteen\n";
/// The first hunk with two `+` lines (`-one`, `+ONE`, `+ONE-B`), then the
/// second hunk.
const TWO_PLUS_LINES: &str = "ONE\nONE-B\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nSIXTEEN\n";
/// What the index holds after staging only the `+ONE-B` line of that.
const ONLY_SECOND_PLUS_LINE: &str = "one\nONE-B\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\n\
ten\neleven\ntwelve\nthirteen\nfourteen\nfifteen\nsixteen\n";
/// Bytes no text path survives: NUL, 0xFF, LF, CR, 0xFE.
const AWKWARD: &[u8] = &[0x00, 0xff, 0x0a, 0x0d, 0xfe];
/// An edit made after a commit, so undo finds the tree drifted.
const DRIFTED: &str = "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nSIXTEEN\nDRIFT\n";

fn write(repo: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(repo.join(name), bytes).map_err(|why| format!("writing {name}: {why}"))
}

/// Commit `name` with `content` on the current branch.
fn tracked(repo: &Path, name: &str, content: &str) -> Result<(), String> {
    write(repo, name, content.as_bytes())?;
    run_git(repo, &["add", name])?;
    run_git(repo, &["commit", "-q", "-m", &format!("add {name}")])?;
    Ok(())
}

fn key(code: KeyCode) -> Step {
    Step::Key(KeyPress::plain(code))
}

fn keys(codes: &[KeyCode]) -> Vec<Step> {
    codes.iter().map(|code| key(*code)).collect()
}

fn ch(c: char) -> KeyCode {
    KeyCode::Char(c)
}

fn plain(codes: &[KeyCode]) -> Vec<KeyPress> {
    codes.iter().map(|code| KeyPress::plain(*code)).collect()
}

fn typed(text: &str) -> Vec<KeyPress> {
    text.chars().map(|c| KeyPress::plain(ch(c))).collect()
}

fn status_and_rows(state: &AppState) -> bool {
    state.working_tree.is_some()
        && state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
}

fn files_ready(state: &AppState) -> bool {
    matches!(state.detail.files, Loading::Ready(_))
}

fn diff_ready(state: &AppState) -> bool {
    matches!(state.detail.diff, Loading::Ready(_)) && state.detail.diff_raw.is_some()
}

fn editor_open(state: &AppState) -> bool {
    state.commit_editor.is_some() && state.head_message.is_none() && state.commit_template.is_none()
}

fn stash_list_ready(state: &AppState) -> bool {
    state.working_tree.is_some() && matches!(state.stash.entries, Loading::Ready(_))
}

/// The preview box drawn over the stash list has its plan (m6-conflicts/86:
/// `⏎` and `p` there preview first, `y` runs).
fn preview_planned(state: &AppState) -> bool {
    state
        .preview
        .as_ref()
        .is_some_and(|preview| !matches!(preview.plan, Loading::Idle | Loading::Pending(_)))
}

fn confirm_open(state: &AppState) -> bool {
    state.confirm.is_some()
}

/// Open the changes pane the way the user does: `k` from the top row onto
/// the uncommitted row, `Enter`.
fn pane() -> Vec<Step> {
    vec![
        Step::Await(
            "the first rows and the working-tree status",
            status_and_rows,
        ),
        key(ch('k')),
        Step::See("k from the top row selects the uncommitted row", |state| {
            check(state.on_working_tree, || {
                "the cursor is not on the uncommitted row".to_owned()
            })
        }),
        key(KeyCode::Enter),
        Step::Await("the changes pane's file list", files_ready),
    ]
}

/// The pane's keys, for a prelude.
fn pane_keys() -> Vec<KeyPress> {
    plain(&[ch('k'), KeyCode::Enter])
}

/// `Tab` to the subject, then `Enter`.
fn commit_from_the_subject() -> Vec<Step> {
    vec![Step::CommitFromSubject]
}

/// `d`, move off the safe answer, confirm.
fn confirm_discard() -> Vec<Step> {
    keys(&[ch('d'), KeyCode::Right, KeyCode::Enter])
}

fn chain(parts: Vec<Vec<Step>>) -> Vec<Step> {
    parts.into_iter().flatten().collect()
}

/// Every scenario, in the order the walk runs them.
#[must_use]
pub fn scenarios() -> Vec<Scenario> {
    let mut all = staging();
    all.extend(discarding());
    all.extend(committing());
    all.extend(stashing());
    all.extend(journal_and_reads());
    all
}

/// No journal file, or an empty one: nothing was written.
fn journal_is_empty(repo: &Path) -> Result<bool, String> {
    let journal = git_path(repo, &format!("{DIR}/undo-journal.jsonl"))?;
    match std::fs::read(&journal) {
        Ok(bytes) => Ok(bytes.is_empty()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(why) => Err(format!("{}: {why}", journal.display())),
    }
}

fn index_has(truth: &RepoTruth, blob: &str, path: &str) -> bool {
    truth
        .index
        .iter()
        .any(|line| line.contains(blob) && line.ends_with(path))
}

// ── staging ──

fn staging() -> Vec<Scenario> {
    vec![
        stage_file(),
        unstage_file(),
        stage_hunk(),
        stage_line(),
        stage_all(),
    ]
}

fn stage_file() -> Scenario {
    Scenario {
        name: "stage-file",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                Step::CursorTo("sample.txt", 0),
                key(ch(' ')),
                Step::Write("the stage"),
                Step::Mark("staged"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let edited = blob_of(repo, EDITED.as_bytes())?;
            let staged = marks.get("staged")?;
            check(index_has(staged, &edited, "sample.txt"), || {
                format!(
                    "after Space the index should hold the edited blob {edited}: {:?}",
                    staged.index
                )
            })?;
            same("after u", &marks.before, &git_truth(repo)?)
        },
        capture: Some(Capture {
            stem: "m3-01-stage-file",
            headless: Vec::new(),
            keys: [pane_keys(), plain(&[ch(' ')])].concat(),
            on_screen: "STAGED (1)",
        }),
    }
}

fn unstage_file() -> Scenario {
    Scenario {
        name: "unstage-file",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", EDITED.as_bytes())?;
            run_git(repo, &["add", "sample.txt"]).map(drop)
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                Step::CursorTo("sample.txt", 0),
                key(ch(' ')),
                Step::Write("the unstage"),
                Step::Mark("unstaged"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let base = blob_of(repo, BASE.as_bytes())?;
            let unstaged = marks.get("unstaged")?;
            check(index_has(unstaged, &base, "sample.txt"), || {
                format!(
                    "after Space the index should hold HEAD's blob {base}: {:?}",
                    unstaged.index
                )
            })?;
            check(
                unstaged.files.get(Path::new("sample.txt")) == Some(&EDITED.as_bytes().to_vec()),
                || "the worktree kept the edit".to_owned(),
            )?;
            same("after u", &marks.before, &git_truth(repo)?)
        },
        capture: Some(Capture {
            stem: "m3-08-journal-after-unstage",
            headless: Vec::new(),
            keys: [pane_keys(), plain(&[ch(' '), KeyCode::Escape, ch('J')])].concat(),
            on_screen: "← u",
        }),
    }
}

fn stage_hunk() -> Scenario {
    Scenario {
        name: "stage-hunk",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                Step::CursorTo("sample.txt", 0),
                // `Enter` in the file list moves focus to the diff; the
                // line cursor opens on body row 0, the first hunk's heading.
                key(KeyCode::Enter),
                Step::Await("the file's diff", diff_ready),
                key(ch(' ')),
                Step::Write("the hunk stage"),
            ],
        ]),
        expect: |repo, _| {
            let first = blob_of(repo, FIRST_HUNK.as_bytes())?;
            let now = git_truth(repo)?;
            check(index_has(&now, &first, "sample.txt"), || {
                format!(
                    "the index should hold the first hunk only ({first}): {:?}",
                    now.index
                )
            })?;
            check(
                now.files.get(Path::new("sample.txt")) == Some(&EDITED.as_bytes().to_vec()),
                || "the worktree keeps both hunks".to_owned(),
            )
        },
        capture: Some(Capture {
            stem: "m3-02-stage-hunk",
            headless: Vec::new(),
            keys: [pane_keys(), plain(&[KeyCode::Enter, ch(' ')])].concat(),
            on_screen: "STAGED (1)",
        }),
    }
}

fn stage_line() -> Scenario {
    Scenario {
        name: "stage-line",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", TWO_PLUS_LINES.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                Step::CursorTo("sample.txt", 0),
                key(KeyCode::Enter),
                Step::Await("the file's diff", diff_ready),
                // Row 0 `@@`, 1 `-one`, 2 `+ONE`, 3 `+ONE-B`.
                key(ch('j')),
                key(ch('j')),
                key(ch('j')),
                key(ch(' ')),
                Step::Write("the line stage"),
            ],
        ]),
        expect: |repo, _| {
            let want = blob_of(repo, ONLY_SECOND_PLUS_LINE.as_bytes())?;
            let now = git_truth(repo)?;
            check(index_has(&now, &want, "sample.txt"), || {
                format!(
                    "the index should hold `one` + `ONE-B` + the rest ({want}); it holds {:?}, :sample.txt = {:?}",
                    now.index,
                    run_git(repo, &["show", ":sample.txt"])
                )
            })
        },
        capture: None,
    }
}

fn stage_all() -> Scenario {
    Scenario {
        name: "stage-all",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "a.txt", BASE)?;
            tracked(repo, "b.txt", BASE)?;
            write(repo, "a.txt", EDITED.as_bytes())?;
            write(repo, "b.txt", FIRST_HUNK.as_bytes())?;
            write(repo, "c.txt", b"new\n")
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                Step::Key(KeyPress::control(ch('a'))),
                Step::Write("stage all"),
            ],
        ]),
        expect: |repo, _| {
            let staged = run_git(repo, &["diff", "--cached", "--name-only"])?;
            let unstaged = run_git(repo, &["diff", "--name-only"])?;
            let untracked = run_git(repo, &["ls-files", "--others", "--exclude-standard"])?;
            check(
                staged == "a.txt\nb.txt\nc.txt\n" && unstaged.is_empty() && untracked.is_empty(),
                || format!("staged {staged:?}, unstaged {unstaged:?}, untracked {untracked:?}"),
            )
        },
        capture: None,
    }
}

// ── discarding ──

fn discarding() -> Vec<Scenario> {
    vec![
        discard_tracked_unstaged(),
        discard_untracked_binary(),
        discard_staged_and_unstaged(),
    ]
}

fn discard_tracked_unstaged() -> Scenario {
    Scenario {
        name: "discard-tracked-unstaged",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![Step::CursorTo("sample.txt", 0)],
            confirm_discard(),
            vec![
                Step::Write("the discard"),
                Step::Mark("discarded"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let discarded = marks.get("discarded")?;
            check(
                discarded.files.get(Path::new("sample.txt")) == Some(&BASE.as_bytes().to_vec()),
                || "after Discard the file should read as HEAD's".to_owned(),
            )?;
            same("after u", &marks.before, &git_truth(repo)?)
        },
        capture: Some(Capture {
            stem: "m3-03-discard-confirm",
            headless: Vec::new(),
            keys: [pane_keys(), plain(&[ch('d')])].concat(),
            on_screen: "Discard",
        }),
    }
}

fn discard_untracked_binary() -> Scenario {
    Scenario {
        name: "discard-untracked-binary",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "bin.dat", AWKWARD)
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![Step::CursorTo("bin.dat", 0)],
            confirm_discard(),
            vec![
                Step::Write("the delete"),
                Step::Mark("deleted"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let deleted = marks.get("deleted")?;
            check(!deleted.files.contains_key(Path::new("bin.dat")), || {
                "after Delete the file should be gone".to_owned()
            })?;
            let now = git_truth(repo)?;
            check(
                now.files.get(Path::new("bin.dat")).map(Vec::as_slice) == Some(AWKWARD),
                || {
                    format!(
                        "after u bin.dat should be {AWKWARD:02x?}: {:?}",
                        now.files.get(Path::new("bin.dat"))
                    )
                },
            )?;
            let status = run_git(repo, &["status", "--porcelain"])?;
            check(status.contains("?? bin.dat"), || {
                format!("status: {status:?}")
            })?;
            same("after u", &marks.before, &now)
        },
        capture: None,
    }
}

fn discard_staged_and_unstaged() -> Scenario {
    Scenario {
        name: "discard-staged-and-unstaged",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", FIRST_HUNK.as_bytes())?;
            run_git(repo, &["add", "sample.txt"])?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            // The file is listed under STAGED and under UNSTAGED. `d` on
            // either row discards the file: both sides go back to HEAD.
            vec![Step::CursorTo("sample.txt", 1)],
            confirm_discard(),
            vec![
                Step::Write("the discard"),
                Step::Mark("discarded"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let base = blob_of(repo, BASE.as_bytes())?;
            let discarded = marks.get("discarded")?;
            check(
                discarded.files.get(Path::new("sample.txt")) == Some(&BASE.as_bytes().to_vec())
                    && index_has(discarded, &base, "sample.txt"),
                || {
                    format!(
                        "after Discard the worktree and the index should both read as HEAD's:\n  {}",
                        diff_truth(&marks.before, discarded).join("\n  ")
                    )
                },
            )?;
            same("after u", &marks.before, &git_truth(repo)?)
        },
        capture: None,
    }
}

// ── committing ──

fn committing() -> Vec<Scenario> {
    vec![
        commit(),
        commit_refused_nothing_staged(),
        amend(),
        commit_on_unborn_head(),
        undo_drift_guard(),
    ]
}

fn staged_edit(repo: &Path) -> Result<(), String> {
    tracked(repo, "sample.txt", BASE)?;
    write(repo, "sample.txt", EDITED.as_bytes())?;
    run_git(repo, &["add", "sample.txt"]).map(drop)
}

fn commit() -> Scenario {
    Scenario {
        name: "commit",
        shape: Shape::Trivial,
        setup: staged_edit,
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                key(ch('c')),
                Step::Await("the commit editor", editor_open),
                Step::Type("shout three"),
            ],
            commit_from_the_subject(),
            vec![
                Step::Write("the commit"),
                Step::Mark("committed"),
                key(ch('u')),
                Step::Write("the undo"),
                Step::Mark("undone"),
                key(ch('U')),
                Step::Write("the redo"),
            ],
        ]),
        expect: |repo, marks| {
            let committed = marks.get("committed")?;
            let made = committed.head.clone().ok_or("no commit was made")?;
            check(Some(&made) != marks.before.head.as_ref(), || {
                "HEAD did not move".to_owned()
            })?;
            let subject = run_git(repo, &["log", "-1", "--format=%s", &made])?;
            check(subject.trim() == "shout three", || {
                format!("subject {subject:?}")
            })?;
            let undone = marks.get("undone")?;
            check(
                undone.head == marks.before.head && undone.index == marks.before.index,
                || {
                    format!(
                        "after u HEAD should be back and the index re-staged:\n  {}",
                        diff_truth(&marks.before, undone).join("\n  ")
                    )
                },
            )?;
            let now = git_truth(repo)?;
            check(now.head.as_deref() == Some(made.as_str()), || {
                format!("after U HEAD should be {made} again: {:?}", now.head)
            })
        },
        capture: Some(Capture {
            stem: "m3-04-commit-editor",
            headless: Vec::new(),
            keys: [pane_keys(), plain(&[ch('c')]), typed("shout three")].concat(),
            on_screen: "COMMIT",
        }),
    }
}

fn commit_refused_nothing_staged() -> Scenario {
    Scenario {
        name: "commit-refused-nothing-staged",
        shape: Shape::Trivial,
        setup: |repo| tracked(repo, "sample.txt", BASE),
        open: None,
        steps: chain(vec![
            vec![
                Step::Await(
                    "the first rows and the working-tree status",
                    status_and_rows,
                ),
                key(ch('c')),
                Step::Await("the commit editor", editor_open),
                Step::Type("nothing to commit"),
            ],
            commit_from_the_subject(),
            vec![Step::See("the editor refuses and stays open", |state| {
                let editor = state.commit_editor.as_ref().ok_or("the editor closed")?;
                let refusal = editor.refusal();
                check(
                    refusal == Some(mergewright_ui::commit_editor::Refusal::NoFile),
                    || format!("refusal {refusal:?}"),
                )?;
                check(state.operating.is_none(), || {
                    "a write was queued".to_owned()
                })
            })],
        ]),
        expect: |repo, marks| {
            let now = git_truth(repo)?;
            same("nothing changed", &marks.before, &now)?;
            check(journal_is_empty(repo)?, || {
                "a refused commit must not be journaled".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m3-05-commit-refused",
            headless: Vec::new(),
            keys: plain(&[ch('c')]),
            on_screen: "Nothing goes in. Put a file in with space.",
        }),
    }
}

fn amend() -> Scenario {
    Scenario {
        name: "amend",
        shape: Shape::Trivial,
        setup: staged_edit,
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                key(ch('a')),
                Step::Await("the editor with HEAD's message", editor_open),
            ],
            // `add sample.txt` is HEAD's subject, prefilled.
            keys(&[KeyCode::Backspace; 14]),
            vec![Step::Type("amended sample")],
            commit_from_the_subject(),
            vec![
                Step::Write("the amend"),
                Step::Mark("amended"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let amended = marks.get("amended")?;
            let made = amended.head.clone().ok_or("no commit")?;
            check(Some(&made) != marks.before.head.as_ref(), || {
                "HEAD did not move".to_owned()
            })?;
            let subject = run_git(repo, &["log", "-1", "--format=%s", &made])?;
            check(subject.trim() == "amended sample", || {
                format!("subject {subject:?}")
            })?;
            let edited = blob_of(repo, EDITED.as_bytes())?;
            let in_tree = run_git(repo, &["rev-parse", &format!("{made}:sample.txt")])?;
            check(in_tree.trim() == edited, || {
                "the amended tree lacks the staged edit".to_owned()
            })?;
            let now = git_truth(repo)?;
            check(
                now.head == marks.before.head && now.index == marks.before.index,
                || {
                    format!(
                        "after u the old commit and the staged edit should be back:\n  {}",
                        diff_truth(&marks.before, &now).join("\n  ")
                    )
                },
            )?;
            let subject = run_git(repo, &["log", "-1", "--format=%s"])?;
            check(subject.trim() == "add sample.txt", || {
                format!("subject after u {subject:?}")
            })
        },
        capture: None,
    }
}

fn commit_on_unborn_head() -> Scenario {
    Scenario {
        name: "commit-on-unborn-head",
        shape: Shape::UnbornBranch,
        setup: |repo| {
            write(repo, "first.txt", b"first\n")?;
            run_git(repo, &["add", "first.txt"]).map(drop)
        },
        open: None,
        steps: chain(vec![
            vec![
                Step::Await("the working-tree status", |state| {
                    state.working_tree.is_some()
                }),
                key(ch('c')),
                Step::Await("the commit editor", editor_open),
                Step::Type("first"),
            ],
            commit_from_the_subject(),
            vec![
                Step::Write("the commit"),
                Step::Mark("committed"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let branch = run_git(repo, &["symbolic-ref", "HEAD"])?.trim().to_owned();
            check(marks.before.head.is_none(), || {
                "the fixture's HEAD should be unborn".to_owned()
            })?;
            let committed = marks.get("committed")?;
            check(
                committed.head.is_some() && committed.refs.contains_key(&branch),
                || {
                    format!(
                        "the commit should have created {branch}: {:?}",
                        committed.refs.keys()
                    )
                },
            )?;
            let now = git_truth(repo)?;
            check(
                now.head.is_none() && !now.refs.contains_key(&branch),
                || {
                    format!(
                        "after u HEAD should be unborn and {branch} gone: {:?} {:?}",
                        now.head,
                        now.refs.keys()
                    )
                },
            )?;
            same("after u", &marks.before, &now)
        },
        capture: None,
    }
}

/// The drift box says the undo leaves the user's edits as they are, and
/// names none it replaces.
fn drift_names_no_edit(state: &AppState) -> Result<(), String> {
    let Some(pending) = state.confirm.as_ref() else {
        return Err("the drift box is not open".to_owned());
    };
    let words: Vec<&str> = pending
        .view
        .message
        .iter()
        .map(mergewright_term::SanitizedText::as_str)
        .collect();
    let words = words.join(" ");
    check(
        words.contains("Undo leaves your edits since as they are.") && !words.contains("replaces"),
        || format!("the box should leave the edit unnamed: {words}"),
    )
}

fn undo_drift_guard() -> Scenario {
    let commit_then_edit = || {
        chain(vec![
            pane(),
            vec![
                key(ch('c')),
                Step::Await("the commit editor", editor_open),
                Step::Type("before the drift"),
            ],
            commit_from_the_subject(),
            vec![
                Step::Write("the commit"),
                Step::Shell(|repo| write(repo, "sample.txt", DRIFTED.as_bytes())),
            ],
        ])
    };
    Scenario {
        name: "undo-drift-guard",
        shape: Shape::Trivial,
        setup: staged_edit,
        open: None,
        steps: chain(vec![
            commit_then_edit(),
            vec![
                Step::Mark("edited"),
                key(ch('u')),
                Step::Await("the drift box", confirm_open),
                // m6-conflicts/77: a commit's undo writes refs and `HEAD`,
                // never `sample.txt`, so the box names no edit it replaces
                // (`drift_expect` checks the edit is still on disk after).
                Step::See("the commit drift is not named", drift_names_no_edit),
                // `Stop` is under the cursor: Enter keeps everything.
                key(KeyCode::Enter),
                Step::See("Stop closes the box", |state| {
                    check(state.confirm.is_none() && state.operating.is_none(), || {
                        "the box is still open or a write ran".to_owned()
                    })
                }),
                Step::Mark("stopped"),
                key(ch('u')),
                Step::Await("the drift box again", confirm_open),
                key(KeyCode::Right),
                key(KeyCode::Enter),
                Step::Write("the forced undo"),
            ],
        ]),
        expect: drift_expect,
        capture: Some(Capture {
            stem: "m3-09-undo-drift-box",
            headless: commit_then_edit(),
            keys: plain(&[ch('u')]),
            on_screen: "Undo",
        }),
    }
}

/// The drift guard's verdict: Stop changed nothing; the forced undo put HEAD
/// and the index back, left the later edit on disk, and kept it in its own
/// backup.
fn drift_expect(repo: &Path, marks: &Marks) -> Result<(), String> {
    let edited = marks.get("edited")?;
    same("Stop changed nothing", edited, marks.get("stopped")?)?;
    let now = git_truth(repo)?;
    check(
        now.head == marks.before.head && now.index == marks.before.index,
        || {
            format!(
                "after the forced undo HEAD and the index should be back:\n  {}",
                diff_truth(&marks.before, &now).join("\n  ")
            )
        },
    )?;
    // A commit touches refs and index, so the drifted worktree is
    // left alone — and kept in the undo entry's own backup.
    check(
        now.files.get(Path::new("sample.txt")) == Some(&DRIFTED.as_bytes().to_vec()),
        || "the edit made after the commit should still be on disk".to_owned(),
    )?;
    let drifted = blob_of(repo, DRIFTED.as_bytes())?;
    let backup = last_backup_ref(repo)?;
    let objects = run_git(repo, &["rev-list", "--objects", &backup])?;
    check(
        objects.contains(&format!("{drifted} worktree/sample.txt")),
        || format!("the undo's backup {backup} should hold the drifted blob {drifted}:\n{objects}"),
    )
}

/// The backup ref of the journal's last entry.
fn last_backup_ref(repo: &Path) -> Result<String, String> {
    let path = git_path(repo, &format!("{DIR}/undo-journal.jsonl"))?;
    let text =
        std::fs::read_to_string(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .rfind(|line| line["status"] == "pending")
        .and_then(|line| line["backup_ref"].as_str().map(str::to_owned))
        .ok_or_else(|| "the last journal entry has no backup ref".to_owned())
}

// ── stashing ──

fn stashing() -> Vec<Scenario> {
    vec![
        stash_save(),
        stash_pop(),
        stash_apply(),
        stash_drop(),
        stash_apply_refused_dirty_touched_file(),
    ]
}

fn open_stashes() -> Vec<Step> {
    vec![
        Step::Await("the working-tree status", |state| {
            state.working_tree.is_some()
        }),
        key(ch('S')),
        Step::Await("the stash list", stash_list_ready),
    ]
}

fn stash_save() -> Scenario {
    Scenario {
        name: "stash-save",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            open_stashes(),
            vec![
                key(ch('s')),
                Step::Type("wip"),
                key(KeyCode::Enter),
                Step::Write("the stash push"),
                Step::Mark("stashed"),
                key(ch('u')),
                Step::Write("the undo"),
                Step::Mark("undone"),
                key(ch('U')),
                Step::Write("the redo"),
            ],
        ]),
        expect: |repo, marks| {
            let branch = run_git(repo, &["symbolic-ref", "--short", "HEAD"])?
                .trim()
                .to_owned();
            let stashed = marks.get("stashed")?;
            check(
                stashed.stash == [format!("stash@{{0}}: On {branch}: wip")],
                || format!("stash list after s: {:?}", stashed.stash),
            )?;
            check(
                stashed.files.get(Path::new("sample.txt")) == Some(&BASE.as_bytes().to_vec()),
                || "after the stash the tree should be clean".to_owned(),
            )?;
            same("after u", &marks.before, marks.get("undone")?)?;
            let now = git_truth(repo)?;
            check(
                now.stash == stashed.stash && now.files == stashed.files,
                || {
                    format!(
                        "after U the stash should be back: {}",
                        diff_truth(stashed, &now).join("; ")
                    )
                },
            )
        },
        capture: Some(Capture {
            stem: "m3-07-stash-prompt",
            headless: Vec::new(),
            keys: [plain(&[ch('S'), ch('s')]), typed("wip")].concat(),
            on_screen: "wip",
        }),
    }
}

fn stash_pop() -> Scenario {
    Scenario {
        name: "stash-pop",
        shape: Shape::Stash,
        setup: |_| Ok(()),
        open: None,
        steps: chain(vec![
            open_stashes(),
            vec![
                key(ch('p')),
                Step::Await("the pop preview", preview_planned),
                key(ch('y')),
                Step::Write("the pop"),
                Step::Mark("popped"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let popped = marks.get("popped")?;
            check(popped.stash.len() + 1 == marks.before.stash.len(), || {
                format!("after p one stash fewer: {:?}", popped.stash)
            })?;
            check(
                popped.files != marks.before.files || popped.index != marks.before.index,
                || "after p the tree should carry the stash's changes".to_owned(),
            )?;
            same("after u", &marks.before, &git_truth(repo)?)
        },
        capture: Some(Capture {
            stem: "m3-06-stash-list",
            headless: Vec::new(),
            keys: plain(&[ch('S')]),
            on_screen: "STASHES",
        }),
    }
}

fn stash_apply() -> Scenario {
    Scenario {
        name: "stash-apply",
        shape: Shape::Stash,
        setup: |_| Ok(()),
        open: None,
        steps: chain(vec![
            open_stashes(),
            vec![
                key(KeyCode::Enter),
                Step::Await("the apply preview", preview_planned),
                key(ch('y')),
                Step::Write("the apply"),
                Step::Mark("applied"),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let applied = marks.get("applied")?;
            check(applied.stash == marks.before.stash, || {
                format!("apply keeps the stash: {:?}", applied.stash)
            })?;
            check(
                applied.files != marks.before.files || applied.index != marks.before.index,
                || "after Enter the tree should carry the stash's changes".to_owned(),
            )?;
            same("after u", &marks.before, &git_truth(repo)?)
        },
        capture: None,
    }
}

/// Prune every unreachable object now. (Not `reflog expire --all`: that
/// would empty the stash reflog the scenario then checks — the remaining
/// stash entries are exactly what must survive.)
fn prune_now(repo: &Path) -> Result<(), String> {
    run_git(repo, &["gc", "--prune=now", "-q"]).map(drop)
}

fn stash_drop() -> Scenario {
    Scenario {
        name: "stash-drop",
        shape: Shape::Stash,
        setup: |_| Ok(()),
        open: None,
        steps: chain(vec![
            open_stashes(),
            confirm_discard(),
            vec![
                Step::Write("the drop"),
                Step::Mark("dropped"),
                Step::Shell(prune_now),
                key(ch('u')),
                Step::Write("the undo"),
            ],
        ]),
        expect: |repo, marks| {
            let dropped = marks.get("dropped")?;
            check(dropped.stash.len() + 1 == marks.before.stash.len(), || {
                format!("after d one stash fewer: {:?}", dropped.stash)
            })?;
            let now = git_truth(repo)?;
            check(now.stash == marks.before.stash, || {
                format!(
                    "after gc and u the stash list should be back: {:?}",
                    now.stash
                )
            })?;
            same("after u", &marks.before, &now)
        },
        capture: None,
    }
}

fn stash_apply_refused_dirty_touched_file() -> Scenario {
    Scenario {
        name: "stash-apply-refused-dirty-touched-file",
        shape: Shape::Stash,
        setup: |repo| {
            let touched = run_git(repo, &["stash", "show", "--name-only", "stash@{0}"])?;
            let first = touched.lines().next().ok_or("stash@{0} touches no file")?;
            let path = repo.join(first);
            let mut bytes = std::fs::read(&path).map_err(|why| format!("{first}: {why}"))?;
            bytes.extend_from_slice(b"an uncommitted edit\n");
            std::fs::write(&path, bytes).map_err(|why| format!("{first}: {why}"))
        },
        open: None,
        steps: chain(vec![
            open_stashes(),
            vec![
                key(KeyCode::Enter),
                // Since m6-conflicts/86 `⏎` previews first; the refusal is the
                // preview box's own words, over the list, and nothing ran.
                Step::Await("the refused preview", preview_planned),
                Step::See("the refusal names the file", |state| {
                    match state.preview.as_ref().map(|preview| &preview.plan) {
                        Some(Loading::Failed(text)) if text.contains("are in the way") => Ok(()),
                        other => Err(format!("preview {other:?}")),
                    }
                }),
            ],
        ]),
        expect: |repo, marks| {
            same("nothing changed", &marks.before, &git_truth(repo)?)?;
            check(journal_is_empty(repo)?, || {
                "a refused apply must not be journaled".to_owned()
            })
        },
        capture: None,
    }
}

// ── the journal, worktrees and reads ──

fn journal_and_reads() -> Vec<Scenario> {
    vec![
        interrupted_entry(),
        linked_worktree(),
        refresh_on_focus(),
        no_index_write_on_read(),
    ]
}

/// A real discard through the app, then its outcome line removed: what a
/// process killed between the write and the journal's last line leaves.
fn discard_then_tear(repo: &Path) -> Result<(), String> {
    tracked(repo, "sample.txt", BASE)?;
    write(repo, "sample.txt", EDITED.as_bytes())?;
    let steps = chain(vec![
        pane(),
        vec![Step::CursorTo("sample.txt", 0)],
        confirm_discard(),
        vec![Step::Write("the discard")],
    ]);
    drive_app(repo, &steps, &mut Marks::default())?;
    let path = git_path(repo, &format!("{DIR}/undo-journal.jsonl"))?;
    let text =
        std::fs::read_to_string(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    let mut lines: Vec<&str> = text.lines().collect();
    check(lines.len() == 2, || {
        format!(
            "expected a pending and an outcome line, got {}",
            lines.len()
        )
    })?;
    lines.pop();
    std::fs::write(&path, format!("{}\n", lines[0]))
        .map_err(|why| format!("{}: {why}", path.display()))
}

fn interrupted_entry() -> Scenario {
    Scenario {
        name: "interrupted-entry",
        shape: Shape::Trivial,
        setup: discard_then_tear,
        open: None,
        steps: vec![
            Step::Await("the launch journal read", |state| {
                mergewright::update::undo::interrupted_notice(state).is_some()
            }),
            Step::See("the footer notice names the operation", |state| {
                let notice =
                    mergewright::update::undo::interrupted_notice(state).ok_or("no notice")?;
                check(
                    notice.contains("discard sample.txt") && notice.contains("J to inspect"),
                    || format!("notice {notice:?}"),
                )
            }),
            key(ch('u')),
            Step::Write("the undo"),
        ],
        expect: |repo, marks| {
            check(
                marks.before.files.get(Path::new("sample.txt")) == Some(&BASE.as_bytes().to_vec()),
                || "setup should have left the discarded file at HEAD's content".to_owned(),
            )?;
            let now = git_truth(repo)?;
            check(
                now.files.get(Path::new("sample.txt")) == Some(&EDITED.as_bytes().to_vec()),
                || "after u the edit the interrupted discard threw away should be back".to_owned(),
            )
        },
        capture: None,
    }
}

fn linked_worktree() -> Scenario {
    Scenario {
        name: "linked-worktree",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            run_git(repo, &["worktree", "add", "-q", "-b", "side", "linked"])?;
            let exclude = repo.join(".git/info/exclude");
            let mut text = std::fs::read_to_string(&exclude).unwrap_or_default();
            text.push_str("linked/\n");
            std::fs::write(&exclude, text).map_err(|why| format!("info/exclude: {why}"))?;
            write(&repo.join("linked"), "sample.txt", EDITED.as_bytes())
        },
        open: Some("linked"),
        steps: chain(vec![
            pane(),
            vec![
                Step::CursorTo("sample.txt", 0),
                key(ch(' ')),
                Step::Write("the stage"),
            ],
        ]),
        expect: |linked, _| {
            let main = linked.parent().ok_or("the linked worktree has no parent")?;
            let own = main.join(format!(".git/worktrees/linked/{DIR}/undo-journal.jsonl"));
            let mains = main.join(format!(".git/{DIR}/undo-journal.jsonl"));
            check(own.is_file(), || format!("{} should exist", own.display()))?;
            check(!mains.exists(), || {
                format!("{} should not exist", mains.display())
            })?;
            let entries = std::fs::read_to_string(&own)
                .map_err(|why| why.to_string())?
                .lines()
                .filter(|line| line.contains("\"pending\""))
                .count();
            check(entries == 1, || {
                format!("{entries} entries in the linked journal")
            })?;
            let refs = run_git(
                main,
                &[
                    "for-each-ref",
                    "--format=%(refname)",
                    &format!("refs/{DIR}/backup"),
                ],
            )?;
            check(
                refs.lines().count() == 1
                    && refs.starts_with(&format!("refs/{DIR}/backup/linked/")),
                || format!("backup refs: {refs:?}"),
            )
        },
        capture: None,
    }
}

fn refresh_on_focus() -> Scenario {
    Scenario {
        name: "refresh-on-focus",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                Step::See("one unstaged file, nothing untracked", |state| {
                    let counts = state.working_tree.as_ref().ok_or("no status")?.counts();
                    check(counts.unstaged == 1 && counts.untracked == 0, || {
                        format!("{counts:?}")
                    })
                }),
                Step::Shell(|repo| write(repo, "extra.txt", b"typed in an editor\n")),
                Step::Focus,
                Step::See("the counts follow the out-of-band file", |state| {
                    let counts = state.working_tree.as_ref().ok_or("no status")?.counts();
                    check(counts.unstaged == 1 && counts.untracked == 1, || {
                        format!("{counts:?}")
                    })
                }),
            ],
        ]),
        expect: |repo, _| {
            let status = run_git(repo, &["status", "--porcelain"])?;
            check(status.contains("?? extra.txt"), || {
                format!("status {status:?}")
            })
        },
        capture: None,
    }
}

fn no_index_write_on_read() -> Scenario {
    Scenario {
        name: "no-index-write-on-read",
        shape: Shape::Trivial,
        setup: |repo| {
            tracked(repo, "sample.txt", BASE)?;
            write(repo, "sample.txt", FIRST_HUNK.as_bytes())?;
            run_git(repo, &["add", "sample.txt"])?;
            write(repo, "sample.txt", EDITED.as_bytes())
        },
        open: None,
        steps: chain(vec![
            pane(),
            vec![
                key(KeyCode::Escape),
                key(ch('J')),
                Step::Await("the journal read", |state| {
                    state.journal.shown && state.journal.reading.is_none()
                }),
                key(ch('J')),
                key(ch('S')),
                Step::Await("the stash list", stash_list_ready),
                key(KeyCode::Escape),
            ],
        ]),
        expect: |repo, marks| {
            let now = git_truth(repo)?;
            check(now.index_bytes == marks.before.index_bytes, || {
                "opening the pane, the journal and the stash list rewrote .git/index".to_owned()
            })?;
            same("reads changed nothing", &marks.before, &now)
        },
        capture: None,
    }
}

// ───────────────────────────── under cargo test ─────────────────────────────

#[cfg(test)]
mod every_scenario {
    use super::{Scenario, run_scenario, scenarios};

    fn named(name: &str) -> Scenario {
        scenarios()
            .into_iter()
            .find(|scenario| scenario.name == name)
            .unwrap_or_else(|| panic!("no scenario named {name}"))
    }

    fn passes(name: &str) {
        if let Err(why) = run_scenario(&named(name)) {
            panic!("FAIL {name}: {why}");
        }
    }

    macro_rules! walk {
        ($($test:ident => $name:literal),* $(,)?) => {
            $(#[test] fn $test() { passes($name); })*
            /// Every scenario the walk lists has a test here, so none can
            /// be added to the walk and skipped by the fast tier.
            #[test]
            fn every_scenario_has_a_test() {
                let listed: Vec<&str> = scenarios().iter().map(|s| s.name).collect();
                let tested = [$($name),*];
                assert_eq!(listed, tested);
            }
        };
    }

    walk! {
        stage_file => "stage-file",
        unstage_file => "unstage-file",
        stage_hunk => "stage-hunk",
        stage_line => "stage-line",
        stage_all => "stage-all",
        discard_tracked_unstaged => "discard-tracked-unstaged",
        discard_untracked_binary => "discard-untracked-binary",
        discard_staged_and_unstaged => "discard-staged-and-unstaged",
        commit => "commit",
        commit_refused_nothing_staged => "commit-refused-nothing-staged",
        amend => "amend",
        commit_on_unborn_head => "commit-on-unborn-head",
        undo_drift_guard => "undo-drift-guard",
        stash_save => "stash-save",
        stash_pop => "stash-pop",
        stash_apply => "stash-apply",
        stash_drop => "stash-drop",
        stash_apply_refused_dirty_touched_file => "stash-apply-refused-dirty-touched-file",
        interrupted_entry => "interrupted-entry",
        linked_worktree => "linked-worktree",
        refresh_on_focus => "refresh-on-focus",
        no_index_write_on_read => "no-index-write-on-read",
    }

    /// Every capture names a distinct `m3-<nn>-…` stem, presses keys and
    /// says what must be on screen; the coverage table lists each stem.
    #[test]
    fn captures_are_named_keyed_and_listed_in_the_matrix() {
        use crate::coverage::{M3_STEMS, working_tree_matrix};
        use crate::repo_truth::keys_label;
        let all = scenarios();
        let captures: Vec<_> = all.iter().filter_map(|s| s.capture.as_ref()).collect();
        assert_eq!(captures.len(), 9, "the ticket names nine visual results");
        let mut stems: Vec<&str> = captures.iter().map(|c| c.stem).collect();
        stems.sort_unstable();
        stems.dedup();
        assert_eq!(stems.len(), captures.len(), "stems repeat");
        assert_eq!(stems, M3_STEMS, "the coverage table's list of stems");
        let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let matrix = working_tree_matrix(&workspace);
        for capture in &captures {
            assert!(capture.stem.starts_with("m3-"), "{}", capture.stem);
            assert!(!capture.keys.is_empty(), "{} presses nothing", capture.stem);
            assert!(
                !keys_label(&capture.keys).is_empty(),
                "{} has no key label",
                capture.stem
            );
            assert!(
                !capture.on_screen.is_empty(),
                "{} expects nothing",
                capture.stem
            );
            assert!(
                matrix.contains(capture.stem),
                "{} missing from the matrix",
                capture.stem
            );
        }
    }

    /// `--capture` starts from the scenario's setup: the scratch is a
    /// repository with the scripted state, alive until dropped.
    #[test]
    fn prepare_capture_leaves_the_scripted_repository_open() {
        use super::prepare_capture;
        let (scratch, repo) = prepare_capture(&named("commit-refused-nothing-staged")).unwrap();
        assert!(repo.join(".git").is_dir());
        assert!(repo.join("sample.txt").is_file());
        drop(scratch);
        assert!(
            !repo.exists(),
            "dropping the scratch deletes the repository"
        );
    }

    /// The truth diff is what a failing scenario prints: it must name the
    /// change and stay short.
    #[test]
    fn diff_truth_names_what_moved_and_stops_at_20_lines() {
        use super::{RepoTruth, diff_truth};
        let before = RepoTruth::default();
        let after = RepoTruth {
            head: Some("abc".to_owned()),
            files: (0..40).map(|n| (format!("f{n}").into(), vec![n])).collect(),
            ..RepoTruth::default()
        };
        let diff = diff_truth(&before, &after);
        assert_eq!(diff.len(), 20);
        assert_eq!(diff[0], "HEAD: None → Some(\"abc\")");
        assert!(diff[1].starts_with("f0: new, 1 bytes"), "{}", diff[1]);
    }
}
