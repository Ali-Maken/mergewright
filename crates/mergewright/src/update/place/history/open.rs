//! Opening the history place: from blame's lines or a file, and the `F` key.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// Open a history place for `from` on the trail and start its trace.
/// Pushed on top: `F` starts a fresh trail around it (`f_key`), `L` pushes
/// onto blame's.
pub fn open_history(state: &mut AppState, from: HistoryFrom) -> Vec<Command> {
    let head = state.refs.as_ref().and_then(|refs| refs.head());
    let Opening {
        at,
        path,
        lines,
        blamed,
        anchor,
        request,
    } = match opening(from, head) {
        Ok(opening) => opening,
        // Nothing committed to trace: said, and no place opens.
        Err(words) => {
            state.status = Status::sticky(words);
            return Vec::new();
        }
    };
    let job = state.local_jobs.next_id();
    let place = HistoryState {
        path,
        at,
        lines,
        blamed,
        range: None,
        trace: TraceLoad::Tracing { job },
        selected: None,
        list_top: 0,
        file: FileLoad::Idle,
        file_top: 0,
        change: None,
        focus: HistoryFocus::List,
        side_by_side: false,
        ignore_whitespace: false,
        max_lines: MAX_LINES,
        anchor,
        last_click: None,
    };
    if state.trail.places.len() >= TRAIL_MAX {
        state.trail.places.remove(0);
    }
    state.trail.places.push(Place::History(Box::new(place)));
    state.trail.shown = true;
    vec![Command::TraceHistory {
        id: job,
        request: Box::new(request),
    }]
}

/// What a history place opens with, worked out from where it was asked.
struct Opening {
    at: Oid,
    path: PathBuf,
    lines: bool,
    blamed: Option<Oid>,
    anchor: Option<LineRef>,
    request: TraceRequest,
}

/// The opening for `from`; a working-tree ask traces from `head`. `Err`
/// carries the words to say when there is no `HEAD` to trace from.
fn opening(from: HistoryFrom, head: Option<Oid>) -> Result<Opening, String> {
    match from {
        HistoryFrom::Lines {
            at,
            path,
            line,
            blamed,
            block,
        } => {
            let (at, from_disk) = match at {
                Rev::Commit(commit) => (commit, false),
                Rev::WorkingTree => (head.ok_or_else(|| uncommitted_words(line))?, true),
            };
            let request = TraceRequest::Lines {
                at,
                path: path.clone(),
                line,
                block: LineRange::Block {
                    start: block.0,
                    end: block.1,
                },
                from_disk,
            };
            Ok(Opening {
                at,
                path,
                lines: true,
                blamed,
                anchor: None,
                request,
            })
        }
        HistoryFrom::File {
            at,
            path,
            line,
            side,
        } => {
            let at = match at {
                Rev::Commit(commit) => commit,
                Rev::WorkingTree => head.ok_or_else(|| not_committed_words(&path))?,
            };
            let request = TraceRequest::File {
                at,
                path: path.clone(),
            };
            Ok(Opening {
                at,
                path,
                lines: false,
                blamed: None,
                anchor: line.map(|line| LineRef { side, line }),
                request,
            })
        }
    }
}

/// A FILES row's `⏎` in `⌃P` (m8-polish-and-ship/12b): the history place
/// on `path`, traced from `HEAD`, as `F` on a file row opens it. Any trail
/// is left first (nothing restored: the palette was over it), so `Esc`
/// pops this place and lands where `⌃P` was opened, its graph cursor
/// untouched. A path the status reads as added in the index has no
/// commit to trace: said, and no place opens (as `F` on an untracked
/// file).
pub(crate) fn open_file_history(state: &mut AppState, commands: &mut Vec<Command>, path: PathBuf) {
    let added = state
        .working_tree
        .as_ref()
        .and_then(|status| status.entry(&path))
        .is_some_and(|entry| {
            !entry.unmerged && entry.staged == Some(mergewright_git::status::Change::Added)
        });
    if added {
        state.status = Status::sticky(not_committed_words(&path));
        return;
    }
    if state.trail.is_live() {
        super::super::trail::leave(state, commands, false);
    }
    let from = HistoryFrom::File {
        at: Rev::WorkingTree,
        path,
        line: None,
        side: Side::New,
    };
    commands.extend(open_history(state, from));
}

/// `<path> isn't committed yet · no history`.
fn not_committed_words(path: &std::path::Path) -> String {
    format!(
        "{} isn't committed yet \u{b7} no history",
        sanitize_path(path).as_str()
    )
}

/// `F` in the detail pane: the history of the file the cursor is on — its
/// row, or any line of its diff, the changes pane included. Claimed exactly
/// where `B` is (`place::blamable`): the same guard, so the two keys, and
/// the bar that offers them, cannot disagree. `true` when `F` was taken;
/// otherwise it falls through to Focus mode (`update::keys::graph_key`).
pub(crate) fn f_key(next: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !super::super::blamable(next) {
        return false;
    }
    let Some(cursor) = diff_cursor(next) else {
        return false;
    };
    if cursor.section == Some(WorkingTreeSection::Untracked) {
        next.status = Status::sticky(not_committed_words(&cursor.path));
        return true;
    }
    // The line he was on, from the diff; a file row's `at` is its first
    // change, which is not a line he chose (F2: the top from a file row).
    let on_a_line = next.detail.focus == mergewright_ui::detail::DetailFocus::Diff;
    let (at, line) = match cursor.spec {
        DetailSpec::Commit(commit) => (Rev::Commit(commit), cursor.at.filter(|_| on_a_line)),
        DetailSpec::Compare { head, .. } => (
            Rev::Commit(head),
            cursor.at.filter(|at| on_a_line && at.side == Side::New),
        ),
        // The changes pane counts lines on disk or in the index, not in
        // `HEAD`: the file opens at its top.
        DetailSpec::WorkingTree => (Rev::WorkingTree, None),
    };
    let from = HistoryFrom::File {
        at,
        path: cursor.path.clone(),
        line: line.map(|at| at.line),
        side: line.map_or(Side::New, |at| at.side),
    };
    let base = super::super::begin_trail(next, commands, &cursor);
    let opened = open_history(next, from);
    super::super::settle_trail(next, base);
    commands.extend(opened);
    true
}
