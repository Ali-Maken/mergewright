//! `⌃P` finds files and commits (m8-polish-and-ship/12b, ADR-0061). The
//! jobs are answered by hand, as in `palette_find`: the names job by
//! running `match_names` on the request the update asked with; the commits
//! job with hits written here (`search::first_hits` itself is tested
//! against `/`'s walk in `search::tests`).

use mergewright_git::status::TrackedPaths;
use mergewright_ui::palette::RowKind;

use super::palette_find::{
    answer, branch_read, cancelled, ctrl_p, cursor_on, headers, names_asked, oid, plain, press,
    produced, read_lands, selected, shown, start, typed, under_cursor,
};
use super::*;
use crate::search::CommitHit;
use crate::update::palette::{COMMIT_HITS, FindGroup, PaletteTarget, commits_more};
use crate::update::place::Place;

/// `README.md`, `src/main.rs`, `src/rebase.rs`, `docs/main-notes.md`.
fn paths() -> TrackedPaths {
    TrackedPaths::from_nul_separated(b"README.md\0src/main.rs\0src/rebase.rs\0docs/main-notes.md\0")
}

fn tracked_read(commands: &[Command]) -> Vec<JobId> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::ReadTrackedPaths(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn commits_asked(commands: &[Command]) -> Vec<(JobId, String)> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::PaletteCommits(id, query) => Some((*id, query.text().to_owned())),
            _ => None,
        })
        .collect()
}

fn hit(byte: u8, subject: &str) -> CommitHit {
    CommitHit {
        id: oid(byte),
        subject: subject.as_bytes().into(),
    }
}

fn commits_land(state: &AppState, id: JobId, hits: Vec<CommitHit>) -> AppState {
    produced(state, id, Product::PaletteCommits(Ok(Arc::from(hits)))).0
}

/// `⌃P`, both reads landed ([`paths`], `palette_find::branches`), `query`
/// typed, its names job answered; the commits job asked and not answered.
fn opened_typed(query: &str) -> (AppState, Vec<Command>) {
    let (open, commands) = press(&start(), ctrl_p());
    let (open, _) = read_lands(&open, branch_read(&commands).expect("a branch read"));
    let files = tracked_read(&commands);
    let (open, _) = produced(
        &open,
        files[0],
        Product::TrackedPaths(Ok(Arc::new(paths()))),
    );
    let (state, commands) = typed(&open, query);
    let last = names_asked(&commands).pop().expect("a names job");
    (answer(&state, &last), commands)
}

fn rows_of(state: &AppState, header: &str) -> Vec<(String, RowKind, PaletteTarget)> {
    shown(state)
        .into_iter()
        .filter(|(h, ..)| h == header)
        .map(|(_, label, kind, target)| (label, kind, target))
        .collect()
}

// --- the tracked-path read ---

#[test]
fn palette_open_reads_tracked_paths_once() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = tracked_read(&commands);
    assert_eq!(read.len(), 1, "⌃P reads the paths once: {commands:?}");
    let (open, more) = typed(&open, "ma");
    assert!(tracked_read(&more).is_empty(), "not per key");
    let (_, commands) = press(&start(), plain(KeyCode::Char('?')));
    assert!(tracked_read(&commands).is_empty(), "`?` reads none");
    let (closed, commands) = press(&open, plain(KeyCode::Escape));
    assert!(closed.palette.is_none());
    assert!(cancelled(&commands).contains(&read[0]), "{commands:?}");
}

#[test]
fn files_group_reads_then_lists() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = tracked_read(&commands)[0];
    let (typing, _) = typed(&open, "main");
    assert_eq!(
        rows_of(&typing, "FILES"),
        [("reading…".to_owned(), RowKind::More, PaletteTarget::Note)]
    );
    let (landed, commands) = produced(&typing, read, Product::TrackedPaths(Ok(Arc::new(paths()))));
    let again = names_asked(&commands);
    assert_eq!(again.len(), 1, "the names job re-asked for the paths");
    assert!(again[0].1.files.is_some());
    let landed = answer(&landed, &again[0]);
    let files = rows_of(&landed, "FILES");
    let labels: Vec<&str> = files.iter().map(|(l, ..)| l.as_str()).collect();
    assert_eq!(labels, ["src/main.rs", "docs/main-notes.md"]);
    assert_eq!(files[0].1, RowKind::File);
    assert_eq!(files[0].2, PaletteTarget::File(1), "an index into the read");
}

#[test]
fn tracked_read_failure_reads_in_files_group() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = tracked_read(&commands)[0];
    let (open, _) = produced(
        &open,
        read,
        Product::TrackedPaths(Err("too many to list".to_owned())),
    );
    let (typing, asked) = typed(&open, "v2");
    let typing = answer(&typing, names_asked(&asked).last().expect("tags asked"));
    assert_eq!(
        rows_of(&typing, "FILES"),
        [(
            "files: too many to list".to_owned(),
            RowKind::More,
            PaletteTarget::Note
        )]
    );
    assert!(
        rows_of(&typing, "TAGS").iter().any(|(l, ..)| l == "v2.1.0"),
        "other groups unaffected (H9)"
    );
}

// --- a file row's `⏎` ---

fn history_path(state: &AppState) -> Option<std::path::PathBuf> {
    match state.trail.places.last() {
        Some(Place::History(history)) => Some(history.path.clone()),
        _ => None,
    }
}

#[test]
fn enter_on_file_row_opens_its_history_place() {
    let (state, _) = opened_typed("main");
    let state = cursor_on(state, "FILES", "src/main.rs");
    let (after, commands) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none(), "closed");
    assert_eq!(
        history_path(&after),
        Some(std::path::PathBuf::from("src/main.rs"))
    );
    assert!(after.trail.shown);
    assert!(
        commands.iter().any(|c| matches!(c,
            Command::TraceHistory { request, .. }
                if matches!(&**request, crate::update::place::TraceRequest::File { at, path }
                    if *at == oid(1) && path.as_os_str() == "src/main.rs"))),
        "traced from HEAD: {commands:?}"
    );
}

#[test]
fn esc_from_palette_file_history_lands_on_graph() {
    let before = start();
    let cursor = before.history.as_ref().map(|h| h.cursor);
    let (state, _) = opened_typed("main");
    let state = cursor_on(state, "FILES", "src/main.rs");
    let (opened, _) = press(&state, plain(KeyCode::Enter));
    assert!(opened.trail.is_live());
    let (back, _) = press(&opened, plain(KeyCode::Escape));
    assert!(!back.trail.is_live(), "the place is gone");
    assert!(back.palette.is_none());
    assert_eq!(back.history.as_ref().map(|h| h.cursor), cursor);
    assert_eq!(back.detail.focus, mergewright_ui::detail::DetailFocus::Graph);
    let frame = crate::view::tests::draw(&back, 120, 36);
    assert_eq!(
        frame,
        crate::view::tests::draw(&before, 120, 36),
        "the graph as it was before ⌃P"
    );
}

#[test]
fn file_not_in_head_says_not_committed() {
    let (mut state, _) = opened_typed("rebase");
    state.working_tree = Some(Arc::new(
        mergewright_git::status::parse_porcelain_v2(
            b"1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 \
              e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 src/rebase.rs\0",
        )
        .expect("a staged new file"),
    ));
    let state = cursor_on(state, "FILES", "src/rebase.rs");
    let (after, commands) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none(), "closed");
    assert!(!after.trail.is_live(), "no place opens");
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::TraceHistory { .. })),
        "{commands:?}"
    );
    assert_eq!(
        after.status,
        Status::sticky("src/rebase.rs isn't committed yet \u{b7} no history")
    );
}

// --- COMMITS ---

#[test]
fn commit_row_by_sha_prefix_seeks_it() {
    let (state, commands) = opened_typed("0202");
    let (id, query) = commits_asked(&commands).pop().expect("a commits job");
    assert_eq!(query, "0202");
    let state = commits_land(&state, id, vec![hit(2, "second commit")]);
    let rows = rows_of(&state, "COMMITS");
    assert_eq!(
        rows,
        [(
            "0202020  second commit".to_owned(),
            RowKind::Commit,
            PaletteTarget::Seek(oid(2))
        )]
    );
    assert_ne!(selected(&state), Some(oid(2)), "starts elsewhere");
    let state = cursor_on(state, "COMMITS", "0202020  second commit");
    let (after, _) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none());
    assert_eq!(selected(&after), Some(oid(2)));
}

#[test]
fn commit_row_by_subject_word_seeks_it() {
    let (state, commands) = opened_typed("tidy");
    let (id, _) = commits_asked(&commands).pop().expect("a commits job");
    let state = commits_land(&state, id, vec![hit(2, "Tidy the rebase notes")]);
    let state = cursor_on(state, "COMMITS", "0202020  Tidy the rebase notes");
    let (after, _) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none());
    assert_eq!(selected(&after), Some(oid(2)));
}

#[test]
fn commits_more_row_starts_slash_search() {
    let (state, commands) = opened_typed("fix");
    let (id, _) = commits_asked(&commands).pop().expect("a commits job");
    let hits = (0..COMMIT_HITS)
        .map(|n| hit(0x10 + u8::try_from(n).expect("small"), &format!("fix {n}")))
        .collect();
    let state = commits_land(&state, id, hits);
    let rows = rows_of(&state, "COMMITS");
    assert_eq!(rows.len(), COMMIT_HITS + 1);
    let more = rows.last().expect("a more row");
    assert_eq!(
        (more.0.as_str(), more.1, more.2),
        (
            "more · / searches all of history",
            RowKind::More,
            PaletteTarget::More(FindGroup::Commits)
        )
    );
    let state = cursor_on(state, "COMMITS", "more · / searches all of history");
    let (after, commands) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none(), "closed");
    let search = after.search.as_ref().expect("`/` is searching");
    assert_eq!(search.query.text(), "fix");
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::Search(q) if q.text() == "fix")),
        "{commands:?}"
    );
}

/// The more row names `/` as his keymap spells `app.find` (R1): moved, the
/// row says the new key; unbound, it names no key.
#[test]
fn commits_more_row_spells_his_find_key() {
    let moved = |keys: &[&str]| {
        let lines = [(
            "app.find".to_owned(),
            keys.iter().map(|k| (*k).to_owned()).collect(),
        )];
        let (map, warnings) =
            Keymap::with_overrides(&lines, mergewright_ui::keymap::KeySwitches::default());
        assert_eq!(warnings, []);
        map
    };
    assert_eq!(
        commits_more(Keymap::shipped()),
        "more · / searches all of history"
    );
    assert_eq!(
        commits_more(&moved(&["ctrl+f"])),
        "more · ⌃F searches all of history"
    );
    assert_eq!(
        commits_more(&moved(&["none"])),
        "more · search all of history"
    );
}

#[test]
fn stale_commits_answer_is_dropped() {
    let (open, _) = press(&start(), ctrl_p());
    let (one, first) = press(&open, plain(KeyCode::Char('f')));
    let first = commits_asked(&first).pop().expect("asked");
    let (two, second) = press(&one, plain(KeyCode::Char('i')));
    assert!(
        cancelled(&second).contains(&first.0),
        "the edit cancels it: {second:?}"
    );
    let asked = commits_asked(&second);
    assert_eq!(asked.len(), 1, "one new job");
    assert_eq!(asked[0].1, "fi");
    let before = two.palette.clone();
    let after = commits_land(&two, first.0, vec![hit(2, "fix")]);
    assert_eq!(after.palette, before, "the old id's answer changed nothing");
    // `searching…` until the current one lands; old rows kept that still
    // match by sha or subject.
    let landed = commits_land(&two, asked[0].0, vec![hit(2, "fix it"), hit(3, "final")]);
    let (three, _) = press(&landed, plain(KeyCode::Char('x')));
    let rows: Vec<String> = rows_of(&three, "COMMITS")
        .into_iter()
        .map(|(l, ..)| l)
        .collect();
    assert_eq!(rows, ["0202020  fix it", "searching…"]);
}

#[test]
fn commits_failure_reads_in_commits_group() {
    let (state, commands) = opened_typed("main");
    let (id, _) = commits_asked(&commands).pop().expect("asked");
    let (state, _) = produced(
        &state,
        id,
        Product::PaletteCommits(Err("bad object".to_owned())),
    );
    assert_eq!(
        rows_of(&state, "COMMITS"),
        [(
            "commits: search failed: bad object".to_owned(),
            RowKind::More,
            PaletteTarget::Note
        )]
    );
}

#[test]
fn empty_query_has_no_files_or_commits() {
    let (open, commands) = press(&start(), ctrl_p());
    assert!(commits_asked(&commands).is_empty(), "no commits job");
    let (open, _) = produced(
        &open,
        tracked_read(&commands)[0],
        Product::TrackedPaths(Ok(Arc::new(paths()))),
    );
    for header in headers(&open) {
        assert!(!["FILES", "COMMITS"].contains(&header.as_str()), "{header}");
    }
    let (typed_one, _) = typed(&open, "m");
    assert!(headers(&typed_one).contains(&"FILES".to_owned()));
    let (erased, commands) = press(&typed_one, plain(KeyCode::Backspace));
    assert!(commits_asked(&commands).is_empty(), "none for an empty box");
    for header in headers(&erased) {
        assert!(!["FILES", "COMMITS"].contains(&header.as_str()), "{header}");
    }
}

/// Typed: FILES and COMMITS after TAGS, before COMMANDS RUN RECENTLY
/// (§2.12's order).
#[test]
fn files_and_commits_follow_tags() {
    let (state, commands) = opened_typed("main");
    let (id, _) = commits_asked(&commands).pop().expect("asked");
    let state = commits_land(&state, id, vec![hit(1, "main work")]);
    let order = headers(&state);
    let at = |h: &str| order.iter().position(|o| o == h);
    assert!(at("BRANCHES") < at("FILES"), "{order:?}");
    assert!(at("FILES") < at("COMMITS"), "{order:?}");
    assert_ne!(under_cursor(&state).2, PaletteTarget::Note);
}
