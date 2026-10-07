//! `⌃P` finds branches and tags; COMMANDS RUN RECENTLY; the find job
//! (m8-polish-and-ship/12, ADR-0061). The jobs are answered here by hand:
//! the names job by running `jobs::palette::match_names` on the request the
//! update asked with, so what the test feeds back is what the worker would.

use graft_core::ref_name::RefName;
use graft_git::switch_refs::{RefListing, SwitchRefs};
use graft_ui::palette::{NOTHING_MATCHES, RowKind};

use super::*;
use crate::jobs::palette::{NamesRequest, match_names};
use crate::update::palette::{
    FIND_PLACEHOLDER, FindGroup, GROUP_ROWS, PaletteTarget, RECENT_CAP, RECENT_HEADER, RecentRuns,
    WIDE_ROWS, rows,
};

/// An action row as `⌃P` on the graph lists it: a row carries the context
/// it was spelled in (m8/12e).
fn on_graph(id: &'static str) -> PaletteTarget {
    PaletteTarget::Action {
        id: ActionId(id),
        ctx: graft_ui::keymap::ContextId::Graph,
    }
}

pub(super) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

fn listing(full: &str, tip: Oid, when: i64) -> RefListing {
    RefListing {
        name: name(full),
        tip,
        subject: String::new(),
        author: String::new(),
        when,
        upstream: None,
    }
}

/// The graph of [`on_a_branch_row`] (rows `0x01` `main`, `0x02`
/// `feature`), its badges with a tag `v2.1.0` on `0x02`, at 120×36.
pub(super) fn start() -> AppState {
    let mut state = sized_to_draw(on_a_branch_row());
    let badges = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(1)),
            (name("refs/heads/feature"), oid(2)),
            (name("refs/tags/v2.1.0"), oid(2)),
        ],
        &graft_git::repo::HeadState::Branch(name("refs/heads/main")),
        &HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("badges");
    state.refs = Some(Arc::new(badges));
    state.history.as_mut().expect("a walk").cursor = 0;
    state
}

/// `main` (0x01), `feature` (0x02, newest local), `feat/login` (0x09, not
/// laid out), `origin/feature` (0x02, remote).
fn branches() -> SwitchRefs {
    SwitchRefs {
        refs: vec![
            listing("refs/heads/main", oid(1), 10),
            listing("refs/heads/feature", oid(2), 30),
            listing("refs/heads/feat/login", oid(9), 20),
            listing("refs/remotes/origin/feature", oid(2), 40),
        ],
        previous: None,
    }
}

pub(super) fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

pub(super) fn ctrl_p() -> KeyPress {
    KeyPress::control(KeyCode::Char('p'))
}

pub(super) fn plain(code: KeyCode) -> KeyPress {
    KeyPress::plain(code)
}

/// Types `word` a letter at a time, every command kept.
pub(super) fn typed(state: &AppState, word: &str) -> (AppState, Vec<Command>) {
    let mut state = state.clone();
    let mut commands = Vec::new();
    for c in word.chars() {
        let (after, more) = press(&state, plain(KeyCode::Char(c)));
        state = after;
        commands.extend(more);
    }
    (state, commands)
}

pub(super) fn branch_read(commands: &[Command]) -> Option<JobId> {
    commands.iter().find_map(|c| match c {
        Command::ReadPaletteBranches(id) => Some(*id),
        _ => None,
    })
}

pub(super) fn names_asked(commands: &[Command]) -> Vec<(JobId, NamesRequest)> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::PaletteNames(id, req) => Some((*id, req.clone())),
            _ => None,
        })
        .collect()
}

pub(super) fn cancelled(commands: &[Command]) -> Vec<JobId> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::CancelJob(id) => Some(*id),
            _ => None,
        })
        .collect()
}

pub(super) fn produced(state: &AppState, id: JobId, product: Product) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Produced(id, product))
}

/// The branch read landed with [`branches`].
pub(super) fn read_lands(state: &AppState, id: JobId) -> (AppState, Vec<Command>) {
    produced(
        state,
        id,
        Product::PaletteBranches(Ok(Arc::new(branches()))),
    )
}

/// The worker's answer to `(id, req)`.
pub(super) fn answer(state: &AppState, (id, req): &(JobId, NamesRequest)) -> AppState {
    let hits = match_names(req, &|| false).expect("not stopped");
    produced(state, *id, Product::PaletteNames(Arc::new(hits))).0
}

/// `⌃P`, the branch read landed, `query` typed and its last names job
/// answered.
fn found(query: &str) -> AppState {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a branch read");
    let (open, _) = read_lands(&open, read);
    let (state, commands) = typed(&open, query);
    let last = names_asked(&commands).pop().expect("a names job");
    answer(&state, &last)
}

/// Every shown row: its header, label, kind and target.
pub(super) fn shown(state: &AppState) -> Vec<(String, String, RowKind, PaletteTarget)> {
    let palette = state.palette.as_ref().expect("the palette is up");
    rows(state, palette)
        .into_iter()
        .flat_map(|(header, rows)| {
            let header = header.as_str().to_owned();
            rows.into_iter().map(move |(row, target)| {
                (
                    header.clone(),
                    row.label.as_str().to_owned(),
                    row.kind,
                    target,
                )
            })
        })
        .collect()
}

pub(super) fn headers(state: &AppState) -> Vec<String> {
    let palette = state.palette.as_ref().expect("the palette is up");
    rows(state, palette)
        .into_iter()
        .map(|(header, _)| header.as_str().to_owned())
        .collect()
}

pub(super) fn under_cursor(state: &AppState) -> (String, String, PaletteTarget) {
    let palette = state.palette.as_ref().expect("the palette is up");
    let (header, label, _, target) = shown(state)
        .into_iter()
        .nth(palette.cursor)
        .expect("a row under the cursor");
    (header, label, target)
}

/// The palette's cursor put on the row labelled `label` in `header`.
pub(super) fn cursor_on(mut state: AppState, header: &str, label: &str) -> AppState {
    let at = shown(&state)
        .iter()
        .position(|(h, l, _, _)| h == header && l == label)
        .unwrap_or_else(|| panic!("{header}/{label} shown: {:?}", shown(&state)));
    state.palette.as_mut().expect("up").cursor = at;
    state
}

pub(super) fn selected(state: &AppState) -> Option<Oid> {
    state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id)
}

fn head(state: &AppState) -> Option<Oid> {
    state.refs.as_ref().and_then(|refs| refs.head())
}

// --- seek ---

/// A branch row's `⏎` closes the palette and puts the graph cursor on its
/// tip; nothing is written. (HEAD with git: the pty walk `walk_seek_branch`.)
#[test]
fn enter_on_branch_row_seeks_tip_and_keeps_head() {
    let state = cursor_on(found("feature"), "BRANCHES", "feature");
    let before = head(&state);
    let (after, commands) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none(), "closed");
    assert_eq!(selected(&after), Some(oid(2)));
    assert_eq!(head(&after), before);
    assert!(
        commands.iter().all(|c| !matches!(c, Command::Operate(..))),
        "nothing written: {commands:?}"
    );
}

#[test]
fn enter_on_remote_and_tag_rows_seek() {
    for (query, header, label) in [
        ("origin", "BRANCHES", "origin/feature"),
        ("v2.1", "TAGS", "v2.1.0"),
    ] {
        let state = cursor_on(found(query), header, label);
        assert!(
            selected(&state) != Some(oid(2)),
            "{label}: starts elsewhere"
        );
        let (after, _) = press(&state, plain(KeyCode::Enter));
        assert!(after.palette.is_none());
        assert_eq!(selected(&after), Some(oid(2)), "{label}");
    }
}

#[test]
fn branch_tip_not_laid_out_seeks_through_the_walk() {
    let mut state = cursor_on(found("login"), "BRANCHES", "feat/login");
    state.awaiting = Some(JobId::new(4242));
    let (after, commands) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_none());
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::ShowRows(walk, _) if *walk == JobId::new(4242))),
        "{commands:?}"
    );
}

/// 11b's mouse path: a double click on a branch row seeks, as `⏎`.
#[test]
fn click_on_branch_row_seeks() {
    let mut state = found("feature");
    state.harness.clock = crate::session::Clock::Fixed(1_000);
    let frame = crate::view::tests::draw(&state, 120, 36);
    let row = frame
        .lines()
        .position(|line| line.contains("origin/feature"))
        .unwrap_or_else(|| panic!("drawn:\n{frame}"));
    let click = |state: &AppState| {
        update(
            state,
            AppEvent::Core(Event::Mouse(graft_core::event::MouseInput {
                kind: graft_core::event::MouseKind::Down(graft_core::event::MouseButton::Left),
                column: 60,
                row: u16::try_from(row).expect("a row"),
                modifiers: Modifiers::NONE,
            })),
        )
        .0
    };
    let once = click(&state);
    assert_eq!(under_cursor(&once).1, "origin/feature");
    let twice = click(&once);
    assert!(twice.palette.is_none());
    assert_eq!(selected(&twice), Some(oid(2)));
}

// --- the branch read ---

#[test]
fn palette_open_reads_branches_once() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("⌃P reads the branches");
    let (open, more) = typed(&open, "fe");
    assert_eq!(branch_read(&more), None, "once per open");
    let (_, commands) = press(&start(), plain(KeyCode::Char('?')));
    assert_eq!(branch_read(&commands), None, "`?` reads none");
    let (closed, commands) = press(&open, plain(KeyCode::Escape));
    assert!(closed.palette.is_none());
    assert!(cancelled(&commands).contains(&read), "{commands:?}");
}

#[test]
fn branches_group_reads_then_lists() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let (typing, asked) = typed(&open, "feat");
    let rows = shown(&typing);
    assert!(
        rows.iter()
            .any(|(h, l, k, _)| h == "BRANCHES" && l == "reading…" && *k == RowKind::More),
        "{rows:?}"
    );
    // The tags were there; answer the job asked before the read.
    let typing = answer(&typing, names_asked(&asked).last().expect("asked"));
    let (landed, commands) = read_lands(&typing, read);
    let again = names_asked(&commands);
    assert_eq!(again.len(), 1, "the names job re-asked for the read");
    assert_eq!(&*again[0].1.query, "feat");
    let landed = answer(&landed, &again[0]);
    let branches: Vec<String> = shown(&landed)
        .into_iter()
        .filter(|(h, ..)| h == "BRANCHES")
        .map(|(_, l, _, _)| l)
        .collect();
    assert_eq!(branches, ["feature", "feat/login", "origin/feature"]);
}

#[test]
fn branch_read_failure_reads_in_branches_group() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let (open, _) = produced(
        &open,
        read,
        Product::PaletteBranches(Err("fatal: bad\nthings".to_owned())),
    );
    let (typing, asked) = typed(&open, "v2");
    let typing = answer(&typing, names_asked(&asked).last().expect("tags asked"));
    let rows = shown(&typing);
    assert!(
        rows.iter().any(|(h, l, _, t)| h == "BRANCHES"
            && l == "branches: fatal: bad things"
            && *t == PaletteTarget::Note),
        "{rows:?}"
    );
    assert!(
        rows.iter().any(|(h, l, ..)| h == "TAGS" && l == "v2.1.0"),
        "{rows:?}"
    );
}

#[test]
fn branches_rank_local_then_newest_tip() {
    let state = found("feature");
    let branches: Vec<String> = shown(&state)
        .into_iter()
        .filter(|(h, ..)| h == "BRANCHES")
        .map(|(_, l, _, _)| l)
        .collect();
    assert_eq!(
        branches,
        ["feature", "origin/feature"],
        "local before a newer remote"
    );
    let state = found("feat");
    let branches: Vec<String> = shown(&state)
        .into_iter()
        .filter(|(h, ..)| h == "BRANCHES")
        .map(|(_, l, _, _)| l)
        .collect();
    assert_eq!(
        branches,
        ["feature", "feat/login", "origin/feature"],
        "newer tip first"
    );
}

// --- the empty box, `?`, recent ---

#[test]
fn empty_query_lists_recent_then_actions_only() {
    let (open, commands) = press(&start(), ctrl_p());
    assert!(
        names_asked(&commands).is_empty(),
        "no find for an empty box"
    );
    assert_eq!(headers(&open), ["ACTIONS"]);
    // One run: `⌃P stash ⏎`.
    let (state, _) = typed(&open, "stash");
    let (state, _) = press(&state, plain(KeyCode::Enter));
    let (state, _) = press(&state, plain(KeyCode::Escape));
    let (open, _) = press(&state, ctrl_p());
    assert_eq!(headers(&open), [RECENT_HEADER, "ACTIONS"]);
    let (header, label, _) = under_cursor(&open);
    assert_eq!((header.as_str(), label.as_str()), (RECENT_HEADER, "Stash…"));
}

#[test]
fn empty_everything_field_names_what_it_finds() {
    let (open, _) = press(&start(), ctrl_p());
    let frame = crate::view::tests::draw(&open, 120, 36);
    assert!(frame.contains(FIND_PLACEHOLDER), "{frame}");
    let (typing, _) = typed(&open, "f");
    let frame = crate::view::tests::draw(&typing, 120, 36);
    assert!(
        !frame.contains(FIND_PLACEHOLDER),
        "gone on the first character"
    );
    let (help, _) = press(&start(), plain(KeyCode::Char('?')));
    let frame = crate::view::tests::draw(&help, 120, 36);
    assert!(!frame.contains(FIND_PLACEHOLDER), "`?` finds only keys");
}

#[test]
fn help_palette_has_no_find_groups() {
    let mut state = start();
    state.palette_recent.ran(on_graph("graph.stash"));
    let (help, commands) = press(&state, plain(KeyCode::Char('?')));
    let (help, more) = typed(&help, "feat");
    assert!(names_asked(&commands).is_empty() && names_asked(&more).is_empty());
    for header in headers(&help) {
        assert!(
            !["BRANCHES", "TAGS", RECENT_HEADER].contains(&header.as_str()),
            "{header}"
        );
    }
    let (help, _) = press(&state, plain(KeyCode::Char('?')));
    assert!(
        !headers(&help).contains(&RECENT_HEADER.to_owned()),
        "empty `?` too"
    );
}

#[test]
fn recent_group_header_is_commands_run_recently() {
    assert_eq!(RECENT_HEADER, "COMMANDS RUN RECENTLY");
    let mut state = start();
    state.palette_recent.ran(on_graph("graph.stash"));
    let (open, _) = press(&state, ctrl_p());
    let frame = crate::view::tests::draw(&open, 120, 36);
    assert!(frame.contains("COMMANDS RUN RECENTLY"), "{frame}");
    // Typed: after ACTIONS and the finds, and may repeat an ACTIONS row.
    let typed_state = {
        let (s, c) = typed(&open, "stash");
        names_asked(&c)
            .last()
            .map_or(s.clone(), |last| answer(&s, last))
    };
    let order = headers(&typed_state);
    assert_eq!(
        order.last().map(String::as_str),
        Some(RECENT_HEADER),
        "{order:?}"
    );
    assert_eq!(
        order.first().map(String::as_str),
        Some("ACTIONS"),
        "{order:?}"
    );
}

#[test]
fn recent_row_runs_its_key_path() {
    let (open, _) = press(&start(), ctrl_p());
    let (state, _) = typed(&open, "stash");
    let (ran, _) = press(&state, plain(KeyCode::Enter));
    let (back, _) = press(&ran, plain(KeyCode::Escape));
    let (open, _) = press(&back, ctrl_p());
    let (header, label, _) = under_cursor(&open);
    assert_eq!((header.as_str(), label.as_str()), (RECENT_HEADER, "Stash…"));
    let (rerun, commands) = press(&open, plain(KeyCode::Enter));
    // `S` pressed at that moment: the same job ids.
    let at_enter = AppState {
        palette: None,
        ..open.clone()
    };
    let (by_key, key_commands) = press(&at_enter, plain(KeyCode::Char('S')));
    assert!(rerun.palette.is_none());
    assert_eq!(
        super::palette::without_finds(&open, &commands),
        key_commands
    );
    assert_eq!(
        crate::view::tests::draw(&rerun, 120, 36),
        crate::view::tests::draw(&by_key, 120, 36),
        "the same screen as pressing S"
    );
}

#[test]
fn recent_runs_cap_and_order() {
    let mut recent = RecentRuns::default();
    let themes = [
        graft_ui::theme::ThemeKind::Dark,
        graft_ui::theme::ThemeKind::Light,
    ];
    let ids = [
        "graph.stash",
        "graph.fetch",
        "graph.pull",
        "graph.push",
        "graph.merge",
        "graph.rebase",
        "graph.reset",
        "graph.revert",
        "graph.cherry_pick",
    ];
    for id in ids {
        recent.ran(on_graph(id));
    }
    let kept: Vec<PaletteTarget> = recent.iter().collect();
    assert_eq!(kept.len(), RECENT_CAP);
    assert_eq!(kept[0], on_graph("graph.cherry_pick"), "newest first");
    assert!(!kept.contains(&on_graph("graph.stash")), "oldest dropped");
    recent.ran(on_graph("graph.pull"));
    let kept: Vec<PaletteTarget> = recent.iter().collect();
    assert_eq!(
        kept[0],
        on_graph("graph.pull"),
        "a rerun moves to the front"
    );
    assert_eq!(kept.len(), RECENT_CAP, "distinct");
    for kind in themes {
        recent.ran(PaletteTarget::App(
            crate::update::palette::AppAction::Theme(kind),
        ));
    }
    assert_eq!(recent.iter().count(), RECENT_CAP);
    // Neither a seek nor a note is a command run.
    recent.ran(PaletteTarget::Seek(oid(1)));
    recent.ran(PaletteTarget::Note);
    assert!(
        recent
            .iter()
            .all(|t| matches!(t, PaletteTarget::Action { .. } | PaletteTarget::App(_)))
    );
}

// --- the names job: cancel, stale, re-filter ---

#[test]
fn query_edit_cancels_the_previous_find_job() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let (open, _) = read_lands(&open, read);
    let (one, first) = press(&open, plain(KeyCode::Char('f')));
    let first = names_asked(&first);
    assert_eq!(first.len(), 1);
    let (_, second) = press(&one, plain(KeyCode::Char('e')));
    // The names job, and the commits job (12b's, its own test).
    let commits = one
        .palette
        .as_ref()
        .and_then(|p| p.find.commits.pending.as_ref())
        .map(|(id, _)| *id)
        .expect("a commits job too");
    assert_eq!(cancelled(&second), vec![first[0].0, commits]);
    let asked = names_asked(&second);
    assert_eq!(asked.len(), 1, "one new job");
    assert_eq!(&*asked[0].1.query, "fe");
    // The cursor keys move; they ask nothing.
    let (_, moved) = press(&one, plain(KeyCode::Down));
    assert!(names_asked(&moved).is_empty() && cancelled(&moved).is_empty());
}

#[test]
fn closing_the_palette_cancels_find_jobs() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let (typing, asked) = typed(&open, "fe");
    let names = names_asked(&asked).last().expect("asked").0;
    let (closed, commands) = press(&typing, plain(KeyCode::Escape));
    assert!(closed.palette.is_none());
    let gone = cancelled(&commands);
    assert!(
        gone.contains(&read) && gone.contains(&names),
        "{commands:?}"
    );
    // Run a row: the same.
    let (run, commands) = press(&typing, plain(KeyCode::Enter));
    assert!(run.palette.is_none());
    let gone = cancelled(&commands);
    assert!(
        gone.contains(&read) && gone.contains(&names),
        "{commands:?}"
    );
}

#[test]
fn stale_names_answer_is_dropped() {
    let (open, commands) = press(&start(), ctrl_p());
    let (open, _) = read_lands(&open, branch_read(&commands).expect("a read"));
    let (one, first) = press(&open, plain(KeyCode::Char('m')));
    let first = names_asked(&first).pop().expect("asked");
    let (two, _) = press(&one, plain(KeyCode::Char('a')));
    let before = two.palette.clone();
    let after = answer(&two, &first);
    assert_eq!(after.palette, before, "the old id's answer changed nothing");
}

#[test]
fn shown_rows_always_match_the_current_query() {
    let state = found("feat");
    // `featu`: the `feat` answer is shown, re-filtered; `feat/login` goes.
    let (state, asked) = typed(&state, "u");
    assert_eq!(names_asked(&asked).len(), 1, "pending");
    let rows = shown(&state);
    for (header, label, kind, _) in &rows {
        if *kind == RowKind::Branch || *kind == RowKind::Tag {
            assert!(
                graft_ops::matcher::fuzzy_score("featu", label).is_some(),
                "{header}/{label} does not match `featu`"
            );
        }
    }
    assert!(
        rows.iter()
            .any(|(h, l, ..)| h == "BRANCHES" && l == "feature")
    );
    assert!(
        rows.iter()
            .any(|(h, l, k, _)| h == "BRANCHES" && l == "searching…" && *k == RowKind::More),
        "{rows:?}"
    );
    // `⏎` on a placeholder does nothing.
    let state = cursor_on(state, "BRANCHES", "searching…");
    let (after, _) = press(&state, plain(KeyCode::Enter));
    assert!(after.palette.is_some(), "stays");
}

/// `+N more`: widen to [`WIDE_ROWS`], cursor on the first new row; a query
/// edit resets.
#[test]
fn more_row_widens_then_resets() {
    let mut refs = SwitchRefs {
        refs: (0..60)
            .map(|at| listing(&format!("refs/heads/b{at:02}"), oid(1), i64::from(at)))
            .collect(),
        previous: None,
    };
    refs.refs.push(listing("refs/heads/main", oid(1), 0));
    let (open, commands) = press(&start(), ctrl_p());
    let (open, _) = produced(
        &open,
        branch_read(&commands).expect("a read"),
        Product::PaletteBranches(Ok(Arc::new(refs))),
    );
    let (state, asked) = typed(&open, "b");
    let state = answer(&state, names_asked(&asked).last().expect("asked"));
    let branch_rows = |state: &AppState| {
        shown(state)
            .into_iter()
            .filter(|(h, ..)| h == "BRANCHES")
            .collect::<Vec<_>>()
    };
    let rows = branch_rows(&state);
    assert_eq!(rows.len(), GROUP_ROWS + 1);
    let more = rows.last().expect("a more row");
    assert_eq!(more.1, format!("+{} more", 60 - GROUP_ROWS));
    assert_eq!(more.3, PaletteTarget::More(FindGroup::Branches));
    let state = cursor_on(state, "BRANCHES", &more.1);
    let (wide, _) = press(&state, plain(KeyCode::Enter));
    assert!(wide.palette.is_some(), "stays open");
    let rows = branch_rows(&wide);
    assert_eq!(rows.len(), WIDE_ROWS + 1);
    assert_eq!(
        rows.last().map(|r| r.1.clone()),
        Some(format!("+{} more · type more to narrow", 60 - WIDE_ROWS))
    );
    assert_eq!(rows.last().map(|r| r.3), Some(PaletteTarget::Note));
    assert_eq!(under_cursor(&wide).1, rows[GROUP_ROWS].1, "first new row");
    let (edited, _) = press(&wide, plain(KeyCode::Char('0')));
    assert_eq!(edited.palette.as_ref().expect("up").find.widened, None);
}

#[test]
fn cursor_stays_on_its_row_when_an_answer_lands() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let (state, asked) = typed(&open, "v2");
    let state = answer(&state, names_asked(&asked).last().expect("asked"));
    let state = cursor_on(state, "TAGS", "v2.1.0");
    // The branch read lands above the tags: the cursor keeps its row.
    let (landed, commands) = read_lands(&state, read);
    let landed = names_asked(&commands)
        .last()
        .map_or(landed.clone(), |last| answer(&landed, last));
    assert_eq!(under_cursor(&landed).1, "v2.1.0");
}

#[test]
fn cursor_on_placeholder_moves_to_first_landed_row() {
    let mut state = start();
    state.refs = None;
    let (open, commands) = press(&state, ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let (typing, asked) = typed(&open, "feature");
    assert!(names_asked(&asked).is_empty(), "nothing read to match yet");
    let typing = cursor_on(typing, "BRANCHES", "reading…");
    let (enter, _) = press(&typing, plain(KeyCode::Enter));
    assert!(enter.palette.is_some(), "`⏎` on a placeholder does nothing");
    let (landed, commands) = read_lands(&typing, read);
    let landed = answer(&landed, names_asked(&commands).last().expect("re-asked"));
    assert_eq!(under_cursor(&landed).1, "feature");
}

/// The commits job in flight (12b).
fn commits_pending(state: &AppState) -> JobId {
    state
        .palette
        .as_ref()
        .and_then(|p| p.find.commits.pending.as_ref())
        .map(|(id, _)| *id)
        .expect("a commits job in flight")
}

#[test]
fn nothing_matches_waits_for_every_source() {
    let (open, commands) = press(&start(), ctrl_p());
    let read = branch_read(&commands).expect("a read");
    let files = commands
        .iter()
        .find_map(|c| match c {
            Command::ReadTrackedPaths(id) => Some(*id),
            _ => None,
        })
        .expect("a path read (12b)");
    let (typing, asked) = typed(&open, "qqqq");
    let frame = crate::view::tests::draw(&typing, 120, 36);
    assert!(
        !frame.contains(NOTHING_MATCHES),
        "branches not read yet:\n{frame}"
    );
    let typing = answer(&typing, names_asked(&asked).last().expect("tags asked"));
    let (landed, commands) = read_lands(&typing, read);
    let frame = crate::view::tests::draw(&landed, 120, 36);
    assert!(
        !frame.contains(NOTHING_MATCHES),
        "re-asked, not answered:\n{frame}"
    );
    let landed = answer(&landed, names_asked(&commands).last().expect("re-asked"));
    let frame = crate::view::tests::draw(&landed, 120, 36);
    assert!(
        !frame.contains(NOTHING_MATCHES),
        "files and commits not answered (12b):\n{frame}"
    );
    let (landed, more) = produced(
        &landed,
        files,
        Product::TrackedPaths(Ok(Arc::new(graft_git::status::TrackedPaths::default()))),
    );
    let landed = answer(&landed, names_asked(&more).last().expect("re-asked"));
    let commits = commits_pending(&landed);
    let (landed, _) = produced(&landed, commits, Product::PaletteCommits(Ok(Arc::from([]))));
    let frame = crate::view::tests::draw(&landed, 120, 36);
    assert!(
        frame.contains(NOTHING_MATCHES),
        "every source answered empty:\n{frame}"
    );
}

#[test]
fn an_answer_with_nothing_found_leaves_no_group() {
    let state = found("main");
    assert!(
        !headers(&state).contains(&"TAGS".to_owned()),
        "{:?}",
        headers(&state)
    );
}

/// `docs/shots/m8-palette-find-120x36.txt` and
/// `docs/shots/m8-palette-empty-120x36.txt`. What right looks like: typed
/// `rebase` — `find anything`, `› rebase`, then ACTIONS (the rebase rows,
/// keys and config ids), BRANCHES (`sara/rebase-tidy` then
/// `origin/rebase-tidy`, no key), FILES (`src/rebase.rs`, then
/// `docs/rebase-guide.md`; directory muted), COMMITS (`0202020  Rebase
/// the login branch onto main`, sha muted), COMMANDS RUN RECENTLY (the
/// rebase row again, as ACTIONS draws it), in that order (§1.6, §2.12;
/// 12b re-took it). Empty — the field
/// reads, muted, `type to find actions, branches, tags, files, commits`;
/// COMMANDS RUN RECENTLY (`S Stash…`, `▸` on it) above ACTIONS; no branch
/// or tag rows. Colour unverified (rule 5).
/// The capture's typed state: `rebase` over a branch, a file and a commit
/// matching it, every source answered (12b).
fn rebase_found() -> AppState {
    let mut state = start();
    state.palette_recent.ran(on_graph("graph.rebase"));
    let (open, commands) = press(&state, ctrl_p());
    let (open, _) = produced(
        &open,
        branch_read(&commands).expect("a read"),
        Product::PaletteBranches(Ok(Arc::new(SwitchRefs {
            refs: vec![
                listing("refs/heads/main", oid(1), 10),
                listing("refs/heads/sara/rebase-tidy", oid(2), 30),
                listing("refs/remotes/origin/rebase-tidy", oid(2), 40),
            ],
            previous: None,
        }))),
    );
    let paths = commands
        .iter()
        .find_map(|c| match c {
            Command::ReadTrackedPaths(id) => Some(*id),
            _ => None,
        })
        .expect("a path read");
    let (open, _) = produced(
        &open,
        paths,
        Product::TrackedPaths(Ok(Arc::new(
            graft_git::status::TrackedPaths::from_nul_separated(
                b"README.md\0docs/rebase-guide.md\0src/main.rs\0src/rebase.rs\0",
            ),
        ))),
    );
    let (typing, asked) = typed(&open, "rebase");
    let found = answer(&typing, names_asked(&asked).last().expect("asked"));
    let commits = commits_pending(&found);
    let (found, _) = produced(
        &found,
        commits,
        Product::PaletteCommits(Ok(Arc::from([crate::search::CommitHit {
            id: oid(2),
            subject: b"Rebase the login branch onto main"[..].into(),
        }]))),
    );
    found
}

#[test]
fn capture_find_and_empty_palette() {
    let found = rebase_found();
    assert_eq!(
        headers(&found),
        ["ACTIONS", "BRANCHES", "FILES", "COMMITS", RECENT_HEADER],
        "§1.6's order"
    );
    graft_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-palette-find-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        crate::view::tests::draw(&found, 120, 36),
    );

    let (open, _) = press(&start(), ctrl_p());
    let (ran, _) = typed(&open, "stash");
    let (ran, _) = press(&ran, plain(KeyCode::Enter));
    let (back, _) = press(&ran, plain(KeyCode::Escape));
    let (empty, _) = press(&back, ctrl_p());
    assert_eq!(headers(&empty), [RECENT_HEADER, "ACTIONS"]);
    graft_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-palette-empty-120x36.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        crate::view::tests::draw(&empty, 120, 36),
    );
}
