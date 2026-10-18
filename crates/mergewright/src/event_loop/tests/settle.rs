//! G-settle (m8-polish-and-ship/03): per-row work starts when the cursor
//! rests. Driven through the loop on a timeline of its own ([`Timed`]): the
//! loop reads the time from its wakes (`Wakes::now`), so a held key 16 ms
//! apart is exactly that, with no sleeping and nothing a busy machine can
//! stretch.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mergewright_core::oid::Oid;
use mergewright_ui::detail::{FileListView, FileRow, FileRowKind, FileStatusView};

use super::super::rest::{BURST_GAP, QUIET_MIN};
use super::super::{Phase as LoopPhase, Waited};
use super::*;
use crate::history::History;
use crate::update::detail::{DetailSpec, Loading};
use crate::update::{Product, follow_behind};

mod fold;

/// What the script does next.
enum Step {
    /// Hand this wake out.
    Wake(Wake),
    /// Answer the newest `ListFiles` the loop dispatched with one file,
    /// `a.rs`.
    AnswerFiles,
}

/// Wakes on a virtual timeline: each step arrives at its own instant, the
/// loop's clock is the timeline's, and waiting past a deadline moves the
/// timeline there instead of sleeping. Records how many commands the loop
/// had dispatched when each step was handed out and when each frame was
/// drawn.
struct Timed<'j> {
    script: VecDeque<(Instant, Step)>,
    now: Instant,
    jobs: &'j Recorded,
    /// Ends the run as soon as the script is spent, without waiting for
    /// rest: what leaves the loop's state mid-burst for a test to read.
    end_when_spent: bool,
    /// `handed[i]`: commands dispatched before step `i` was handed out.
    handed: Vec<usize>,
    /// Per frame drawn: steps handed out so far.
    drawn: Vec<usize>,
}

impl<'j> Timed<'j> {
    fn new(jobs: &'j Recorded, steps: Vec<(Duration, Step)>) -> Self {
        let start = Instant::now();
        Self {
            script: steps
                .into_iter()
                .map(|(at, step)| (start + at, step))
                .collect(),
            now: start,
            jobs,
            end_when_spent: false,
            handed: Vec::new(),
            drawn: Vec::new(),
        }
    }

    fn hand_out(&mut self) -> Option<Wake> {
        let (at, step) = self.script.pop_front()?;
        self.now = self.now.max(at);
        let commands = self.jobs.commands();
        self.handed.push(commands.len());
        Some(match step {
            Step::Wake(wake) => wake,
            Step::AnswerFiles => {
                let id = commands
                    .iter()
                    .rev()
                    .find_map(|command| match command {
                        Command::ListFiles(id, ..) => Some(*id),
                        _ => None,
                    })
                    .expect("a file list was asked for");
                Wake::Event(AppEvent::Produced(id, Product::Files(Ok(one_file()))))
            }
        })
    }
}

impl Wakes for Timed<'_> {
    fn next_wake(&mut self) -> Option<Wake> {
        self.hand_out()
    }

    fn next_wake_by(&mut self, deadline: Instant) -> Waited {
        match self.script.front() {
            Some((at, _)) if *at <= deadline => self.hand_out().map_or(Waited::Ended, Waited::Woke),
            None if self.end_when_spent => Waited::Ended,
            _ => {
                self.now = self.now.max(deadline);
                Waited::TimedOut
            }
        }
    }

    fn waiting(&mut self) -> bool {
        self.script.front().is_none_or(|(at, _)| *at <= self.now)
    }

    fn joining_waiting(&mut self) -> bool {
        self.script.front().is_some_and(|(at, step)| {
            *at <= self.now
                && match step {
                    Step::Wake(wake) => joins_frame(wake),
                    Step::AnswerFiles => true,
                }
        })
    }

    fn mark(&mut self, phase: LoopPhase) {
        if phase == LoopPhase::Drawn {
            self.drawn.push(self.handed.len());
        }
    }

    fn now(&self) -> Instant {
        self.now
    }
}

fn one_file() -> (Arc<FileListView>, Arc<mergewright_git::diff::FileList>) {
    let view = FileListView {
        rows: vec![FileRow {
            depth: 0,
            name: mergewright_term::sanitize::sanitize_line("a.rs"),
            kind: FileRowKind::File {
                status: FileStatusView::Modified,
                added: Some(1),
                removed: Some(1),
            },
            entry: Some(0),
        }],
        files: 1,
        added: 1,
        removed: 1,
        ..FileListView::default()
    };
    let list = mergewright_git::diff::FileList {
        files: vec![mergewright_git::diff::FileEntry {
            path: b"a.rs".to_vec(),
            old_path: None,
            status: mergewright_git::diff::FileStatus::Modified,
            added: Some(1),
            removed: Some(1),
        }],
        added: 1,
        removed: 1,
    };
    (Arc::new(view), Arc::new(list))
}

const COMMITS: u8 = 81;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes")
}

/// The commit on row `row`: rows run newest first, commit `COMMITS` on top.
fn commit_on(row: usize) -> Oid {
    oid(COMMITS - u8::try_from(row).expect("small"))
}

/// `COMMITS` commits, every seventh a merge (commit `k` merges `k - 2` into
/// `k - 1`), laid out, the cursor on `cursor`.
fn history_with_merges(cursor: usize) -> History {
    use mergewright_core::commit::{Commit, Identity, Timestamp};
    let commits: Vec<Commit> = (1..=COMMITS)
        .rev()
        .map(|id| {
            let who = Identity {
                name: b"Test".to_vec(),
                email: b"test@example.invalid".to_vec(),
                when: Timestamp {
                    seconds: i64::from(id),
                    offset_seconds: 0,
                },
            };
            let parents = match id {
                1 => vec![],
                id if id % 7 == 0 => vec![oid(id - 1), oid(id - 2)],
                id => vec![oid(id - 1)],
            };
            Commit {
                id: oid(id),
                parents,
                author: who.clone(),
                committer: who,
                message: format!("Commit {id}").into_bytes(),
            }
        })
        .collect();
    let rows = mergewright_graph::layout::layout(&commits, mergewright_graph::seeds::Seeds::default());
    History {
        window: Some(crate::history::Window {
            request: 0,
            first: 0,
            laid_out: rows.len(),
            done: true,
            rows: rows.into(),
            promoted: false,
            fold: [].into(),
            sought: None,
        }),
        rows: usize::from(COMMITS),
        phase: Phase::Complete,
        cursor,
        reanchor: None,
        ..History::started()
    }
}

const COLUMNS: u16 = 120;
const ROWS: u16 = 40;

/// The graph with the refs read (the evil probe waits for them), the
/// cursor on `cursor`, the pane `shown` or not.
fn graph(cursor: usize, shown: bool) -> AppState {
    let mut state = AppState {
        history: Some(history_with_merges(cursor)),
        refs: Some(Arc::new(crate::refs::RefBadges::default())),
        size: Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        // Ages against the pinned instant, not the wall (m8/19d-b2).
        harness: crate::update::Harness {
            clock: crate::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW),
            ..crate::update::Harness::default()
        },
        ..AppState::default()
    };
    state.detail.shown = shown;
    state
}

/// `count` presses of `j`, `gap` apart from the start.
fn held_j(count: u32, gap: Duration) -> Vec<(Duration, Step)> {
    (0..count).map(|n| (gap * n, Step::Wake(j()))).collect()
}

/// Runs the loop over `wakes` from `state`: the loop's end state, what it
/// dispatched, the wakes' record and the frames written.
fn run(state: AppState, wakes: &mut Timed<'_>) -> (AppState, String) {
    let (mut terminal, recorder) = terminal(COLUMNS, ROWS);
    let jobs = wakes.jobs;
    let (exit, state) =
        super::super::running::pump_keeping(&mut terminal, wakes, jobs, &mut session(), state);
    exit.expect("writing to memory cannot fail");
    (state, recorder.written())
}

/// Anything the pane or the evil probe starts for a row.
fn starts_follow(command: &Command) -> bool {
    matches!(
        command,
        Command::ListFiles(..)
            | Command::ReadFileDiff(..)
            | Command::ContainedIn(..)
            | Command::EvilProbe(..)
    )
}

/// The ids a follow's commands run under.
fn follow_ids(commands: &[Command]) -> Vec<mergewright_core::job::JobId> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::ListFiles(id, ..)
            | Command::ReadFileDiff(id, ..)
            | Command::ContainedIn(id, _)
            | Command::EvilProbe(id, _) => Some(*id),
            _ => None,
        })
        .collect()
}

fn cancels(commands: &[Command]) -> Vec<mergewright_core::job::JobId> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::CancelJob(id) => Some(*id),
            _ => None,
        })
        .collect()
}

/// The commits a slice of commands asked "contained in" and the evil probe
/// about.
fn asked_about(commands: &[Command]) -> (Vec<Oid>, Vec<Oid>) {
    let contained = commands
        .iter()
        .filter_map(|command| match command {
            Command::ContainedIn(_, oid) => Some(*oid),
            _ => None,
        })
        .collect();
    let evil = commands
        .iter()
        .filter_map(|command| match command {
            Command::EvilProbe(_, oid) => Some(*oid),
            _ => None,
        })
        .collect();
    (contained, evil)
}

#[test]
fn a_held_j_follows_the_first_row_and_the_row_it_rests_on_and_nothing_between() {
    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(60, Duration::from_millis(16)));
    let (state, _) = run(graph(0, true), &mut wakes);
    let commands = jobs.commands();
    let handed = wakes.handed.clone();
    assert_eq!(handed.len(), 60);
    assert_eq!(cursor(&state), 60);
    assert!(
        (3..=60).any(|row| (usize::from(COMMITS) - row) % 7 == 0),
        "the range crosses a merge row"
    );

    // Key 1: one set, at once, for row 1.
    let first = &commands[handed[0]..handed[1]];
    assert_eq!(
        first
            .iter()
            .filter(|c| matches!(c, Command::ListFiles(..)))
            .count(),
        1,
        "{first:?}"
    );
    assert_eq!(asked_about(first).0, vec![commit_on(1)]);

    // Key 2: the first set's pending ids are cancelled, nothing started.
    let second = &commands[handed[1]..handed[2]];
    assert!(!second.iter().any(starts_follow), "{second:?}");
    let mut cancelled = cancels(second);
    cancelled.sort_unstable();
    let mut pending = follow_ids(first);
    pending.sort_unstable();
    assert_eq!(cancelled, pending, "every pending id of the first follow");

    // Keys 3–60: nothing at all for the pane or the probe.
    let burst = &commands[handed[2]..handed[59]];
    assert!(
        !burst
            .iter()
            .any(|c| starts_follow(c) || matches!(c, Command::CancelJob(_))),
        "{burst:?}"
    );
    let last_key = &commands[handed[59]..];
    // After the quiet window: exactly one set, for the final row (a merge,
    // so the probe asks too).
    assert_eq!(
        last_key
            .iter()
            .filter(|c| matches!(c, Command::ListFiles(..)))
            .count(),
        1,
        "{last_key:?}"
    );
    assert_eq!(
        asked_about(last_key),
        (vec![commit_on(60)], vec![commit_on(60)])
    );
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit_on(60))));
    assert!(!follow_behind(&state));
}

#[test]
fn rest_comes_the_quiet_window_after_the_last_key_not_before() {
    // Keys 90 ms apart: 2 × 90 is capped at the burst gap. A key due before
    // the window ends keeps the burst going.
    let jobs = Recorded::default();
    let gap = Duration::from_millis(90);
    let mut wakes = Timed::new(&jobs, held_j(4, gap));
    let (state, _) = run(graph(0, true), &mut wakes);
    let commands = jobs.commands();
    let lists = commands
        .iter()
        .filter(|c| matches!(c, Command::ListFiles(..)))
        .count();
    assert_eq!(lists, 2, "the first key and the rest: {commands:?}");
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit_on(4))));
    assert!(BURST_GAP > gap && QUIET_MIN < gap);
}

#[test]
fn a_single_j_follows_in_the_same_turn() {
    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(1, Duration::ZERO));
    let (state, _) = run(graph(0, true), &mut wakes);
    // Frame 0 is drawn before the key; frame 1 is the key's.
    assert_eq!(wakes.drawn[..2], [0, 1], "{:?}", wakes.drawn);
    let commands = jobs.commands();
    assert_eq!(asked_about(&commands).0, vec![commit_on(1)]);
    assert!(
        commands.iter().any(|c| matches!(c, Command::ListFiles(..))),
        "{commands:?}"
    );
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit_on(1))));
}

/// `j j j` then `key`, the script ending there with no rest: whatever
/// `key` did, it did on the cursor's row.
fn three_js_then(key: Wake) -> (AppState, Vec<Command>) {
    let jobs = Recorded::default();
    let mut steps = held_j(3, Duration::from_millis(16));
    steps.push((Duration::from_millis(48), Step::Wake(key)));
    let mut wakes = Timed::new(&jobs, steps);
    wakes.end_when_spent = true;
    let (state, _) = run(graph(0, true), &mut wakes);
    (state, jobs.commands())
}

#[test]
fn enter_after_a_burst_acts_on_the_cursors_commit() {
    let (state, commands) = three_js_then(super::key(KeyPress::plain(KeyCode::Enter)));
    assert_eq!(cursor(&state), 3);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit_on(3))));
    assert!(
        asked_about(&commands).0.contains(&commit_on(3)),
        "{commands:?}"
    );
}

#[test]
fn tab_after_a_burst_acts_on_the_cursors_commit() {
    let (state, commands) = three_js_then(super::key(KeyPress::plain(KeyCode::Tab)));
    assert_eq!(cursor(&state), 3);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit_on(3))));
    assert_eq!(state.detail.focus, mergewright_ui::detail::DetailFocus::Files);
    assert!(
        asked_about(&commands).0.contains(&commit_on(3)),
        "{commands:?}"
    );
}

#[test]
fn with_the_pane_hidden_only_the_evil_probe_runs_on_the_leading_edge_and_at_rest() {
    // From row 3, the first `j` lands on row 4 (commit 77, a merge) and the
    // fiftieth on row 53 (commit 28, a merge); rows 11, 18, … between are
    // merges too.
    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(50, Duration::from_millis(16)));
    let (state, _) = run(graph(3, false), &mut wakes);
    assert_eq!(cursor(&state), 53);
    let commands = jobs.commands();
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ListFiles(..) | Command::ContainedIn(..))),
        "{commands:?}"
    );
    assert_eq!(asked_about(&commands).1, vec![commit_on(4), commit_on(53)]);
    let handed = &wakes.handed;
    let probe = follow_ids(&commands[handed[0]..handed[1]]);
    assert_eq!(cancels(&commands[handed[1]..handed[2]]), probe);
}

#[test]
fn after_rest_the_pane_shows_the_cursors_commit_in_the_first_frame_after_its_files() {
    let jobs = Recorded::default();
    let mut steps = held_j(10, Duration::from_millis(16));
    steps.push((Duration::from_millis(600), Step::AnswerFiles));
    let mut wakes = Timed::new(&jobs, steps);
    let (state, written) = run(graph(0, true), &mut wakes);
    let frames: Vec<&str> = written.split("\u{1b}[?2026h").skip(1).collect();
    assert_eq!(frames.len(), wakes.drawn.len(), "one stamp per frame");
    // The first frame drawn once the answer (step 10) was handed out.
    let after = wakes
        .drawn
        .iter()
        .position(|&handed| handed > 10)
        .expect("a frame after the answer");
    assert!(frames[after].contains("a.rs"), "{:?}", frames[after]);
    let sha = commit_on(10).to_string();
    let shown = &state.detail.spec;
    assert_eq!(*shown, Some(DetailSpec::Commit(commit_on(10))));
    assert!(matches!(state.detail.files, Loading::Ready(_)));
    assert!(
        frames[..after]
            .iter()
            .any(|frame| frame.contains(&sha[..7])),
        "the title named the commit before its files came"
    );
}

/// The newest `ListFiles` id in `commands`.
fn listed(commands: &[Command]) -> Option<mergewright_core::job::JobId> {
    commands.iter().rev().find_map(|command| match command {
        Command::ListFiles(id, ..) => Some(*id),
        _ => None,
    })
}

/// Review M1 (m8-03): a compare opened from the menu (no base pinned) does
/// not follow the cursor, so a burst must leave its read alone. Before the
/// fix, the second `j` cancelled the compare's file list and nothing asked
/// again: the body said `reading…` with nothing reading.
#[test]
fn a_burst_leaves_a_menu_compares_pending_files_alone() {
    let mut state = graph(0, true);
    let compare = DetailSpec::Compare {
        base: commit_on(20),
        head: commit_on(10),
    };
    let mut ids = state.local_jobs;
    let opened = state.detail.open(compare, &mut || ids.next_id());
    state.local_jobs = ids;
    // Back on the graph (`Tab`), so `j` moves the cursor.
    state.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
    let id = listed(&opened).expect("the compare listed its files");

    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(10, Duration::from_millis(16)));
    let (state, _) = run(state, &mut wakes);
    assert_eq!(cursor(&state), 10);
    let commands = jobs.commands();
    assert!(!cancels(&commands).contains(&id), "{commands:?}");
    assert_eq!(state.detail.spec, Some(compare));
    assert_eq!(state.detail.files, Loading::Pending(id));
    assert!(!state.detail.abandoned);
    assert!(!crate::view::detail_view(&state).following);

    // The answer still lands.
    let (answered, _) = crate::update::update(
        &state,
        AppEvent::Produced(id, Product::Files(Ok(one_file()))),
    );
    assert!(matches!(answered.detail.files, Loading::Ready(_)));
}

/// Review M3 (m8-03): with a base pinned the pane follows as a compare
/// against it — held, it asks on the leading edge and once at rest, for
/// the row the cursor rested on.
#[test]
fn a_pinned_compare_follows_at_rest() {
    let mut state = graph(0, true);
    let base = commit_on(40);
    state.compare_base = Some(crate::update::detail::CompareBase { oid: base, row: 40 });
    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(10, Duration::from_millis(16)));
    let (state, _) = run(state, &mut wakes);
    assert_eq!(cursor(&state), 10);
    let commands = jobs.commands();
    let handed = &wakes.handed;
    let lists = |from: usize, to: usize| {
        commands[from..to]
            .iter()
            .filter(|c| matches!(c, Command::ListFiles(..)))
            .count()
    };
    assert_eq!(lists(handed[0], handed[1]), 1, "the leading edge");
    assert_eq!(lists(handed[1], handed[9]), 0, "mid-burst");
    assert_eq!(lists(handed[9], commands.len()), 1, "at rest");
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare {
            base,
            head: commit_on(10)
        })
    );
    assert!(!follow_behind(&state));
}

/// Update-level: a burst starts with the pane following row `from`, and the
/// state the burst left plus the first follow's commands.
fn mid_burst(from: usize, shown: bool) -> (AppState, Vec<Command>) {
    let start = graph(from, shown);
    let (followed, first) = crate::update::update(&start, AppEvent::Core(Event::Key(j_press())));
    let mut deferring = followed;
    deferring.defer_follow = true;
    let (moved, _) = crate::update::update(&deferring, AppEvent::Core(Event::Key(j_press())));
    // Hidden, off a merge, the follow is not held (it starts nothing) but
    // it still cancels the probe: either way the first follow's work is
    // abandoned.
    assert_eq!(follow_behind(&moved), shown);
    (moved, first)
}

fn j_press() -> KeyPress {
    KeyPress::plain(KeyCode::Char('j'))
}

#[test]
fn a_late_file_list_for_a_row_the_cursor_left_changes_nothing_shown() {
    let (moved, first) = mid_burst(0, true);
    let id = first
        .iter()
        .find_map(|c| match c {
            Command::ListFiles(id, ..) => Some(*id),
            _ => None,
        })
        .expect("the first follow listed files");
    let (late, commands) = crate::update::update(
        &moved,
        AppEvent::Produced(id, Product::Files(Ok(one_file()))),
    );
    assert_eq!(late.detail, moved.detail);
    assert_eq!(
        crate::view::detail_view(&late),
        crate::view::detail_view(&moved)
    );
    assert!(!commands.iter().any(starts_follow), "{commands:?}");
}

#[test]
fn a_late_contained_in_for_a_row_the_cursor_left_changes_nothing_shown() {
    let (moved, first) = mid_burst(0, true);
    let (id, asked) = first
        .iter()
        .find_map(|c| match c {
            Command::ContainedIn(id, oid) => Some((*id, *oid)),
            _ => None,
        })
        .expect("the first follow asked contained-in");
    let found = mergewright_git::contained::Contained {
        branches: vec![b"main".to_vec()],
        remotes: vec![],
        tags: vec![],
        more: 0,
    };
    let (late, _) = crate::update::update(
        &moved,
        AppEvent::Produced(id, Product::ContainedIn(asked, Ok(found))),
    );
    assert_eq!(late.detail, moved.detail);
    assert_eq!(
        late.detail.contained.get(&asked),
        None,
        "asked again on a visit"
    );
    assert_eq!(
        crate::view::detail_view(&late),
        crate::view::detail_view(&moved)
    );
}

#[test]
fn a_late_evil_verdict_for_a_merge_the_cursor_left_changes_nothing_shown() {
    // Row 3 → 4 (commit 77, a merge) follows; the next `j` leaves it.
    let (moved, first) = mid_burst(3, false);
    let (id, merge) = first
        .iter()
        .find_map(|c| match c {
            Command::EvilProbe(id, oid) => Some((*id, *oid)),
            _ => None,
        })
        .expect("the merge was probed");
    let verdict = mergewright_ops::mistake_radar::probes::EvilVerdict::Clean;
    let (late, _) = crate::update::update(
        &moved,
        AppEvent::Produced(id, Product::EvilProbe(merge, verdict)),
    );
    assert_eq!(late.detail, moved.detail);
    assert!(late.detail.evil_cache.get(&merge).is_none());
}

/// `docs/shots/m8/settle-following.txt`. What right looks like: mid-burst,
/// the cursor is on row 8 and the pane still shows row 1's commit (the
/// last it followed), its title row ending in `reading…`. Graph, cursor
/// row and header move with the keys. Muted colour **unverified** (rule 5).
#[test]
fn capture_the_pane_following_a_burst() {
    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(8, Duration::from_millis(16)));
    wakes.end_when_spent = true;
    let (state, _) = run(graph(0, true), &mut wakes);
    assert_eq!(cursor(&state), 8);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(commit_on(1))));
    assert!(follow_behind(&state));

    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(COLUMNS, ROWS))
        .expect("test backend opens");
    terminal
        .draw(|frame| crate::view::view(&session(), &state, frame))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();
    let lines: Vec<String> = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    let sha = commit_on(1).to_string();
    assert!(
        lines
            .iter()
            .any(|line| line.contains(&format!("COMMIT {}", &sha[..7]))
                && line.ends_with("reading\u{2026}")),
        "{}",
        lines.join("\n")
    );
    mergewright_fixtures::capture(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/m8/settle-following.txt"),
        format!("{}\n", lines.join("\n")),
    );
}
