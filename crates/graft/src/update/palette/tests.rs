//! The palette's route and landing (m8-polish-and-ship/12e, review
//! `docs/reviews/m8-palette-theme.md` Majors 1–2): an `IN PROGRESS` row runs
//! as the banner's key would, focused or not; a cursor on a placeholder
//! moves to a row that lands in any group.

use std::collections::HashMap;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::state::InProgress;
use graft_git::switch_refs::{RefListing, SwitchRefs};
use graft_ui::keymap::{ContextId, defaults};

use crate::jobs::palette::{NamesRequest, match_names};
use crate::update::banner::rewrite_stop_tests::edit_stop;
use crate::update::banner::tests::stopped;
use crate::update::palette::{PaletteTarget, RecentRuns, rows};
use crate::update::state::Status;
use crate::update::{AppEvent, AppState, Command, JobId, Product, update};

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

fn plain(code: KeyCode) -> KeyPress {
    KeyPress::plain(code)
}

/// Every shown row, flattened: its group's header, its label, its target.
fn shown(state: &AppState) -> Vec<(String, String, PaletteTarget)> {
    let palette = state.palette.as_ref().expect("the palette is up");
    rows(state, palette)
        .into_iter()
        .flat_map(|(header, rows)| {
            let header = header.as_str().to_owned();
            rows.into_iter()
                .map(move |(row, target)| (header.clone(), row.label.as_str().to_owned(), target))
        })
        .collect()
}

fn under_cursor(state: &AppState) -> (String, String) {
    let palette = state.palette.as_ref().expect("the palette is up");
    let (header, label, _) = shown(state)
        .into_iter()
        .nth(palette.cursor)
        .expect("a row under the cursor");
    (header, label)
}

/// The palette's cursor put on the row labelled `label` in `header`.
fn cursor_on(mut state: AppState, header: &str, label: &str) -> AppState {
    let at = shown(&state)
        .iter()
        .position(|(h, l, _)| h == header && l == label)
        .unwrap_or_else(|| panic!("{header}/{label} shown: {:?}", shown(&state)));
    state.palette.as_mut().expect("up").cursor = at;
    state
}

// ── Major 1: IN PROGRESS rows run the banner's key ──────────────────────

/// The `IN PROGRESS` rows `?` lists on `state`, by label.
fn in_progress_labels(state: &AppState) -> Vec<String> {
    let (open, _) = press(state, plain(KeyCode::Char('?')));
    shown(&open)
        .into_iter()
        .filter(|(header, _, _)| header == "IN PROGRESS")
        .map(|(_, label, _)| label)
        .collect()
}

/// The banner's default chord for the row labelled `label`.
fn banner_chord(label: &str) -> KeyPress {
    defaults()
        .iter()
        .find(|b| b.context == ContextId::Banner && b.label == label)
        .and_then(|b| b.default.first())
        .unwrap_or_else(|| panic!("a banner row labelled {label}"))
        .to_press()
}

/// `?` on `state`, the cursor on `label` under `IN PROGRESS`, `⏎`.
fn run_row(state: &AppState, label: &str) -> (AppState, Vec<Command>) {
    let (open, _) = press(state, plain(KeyCode::Char('?')));
    let open = cursor_on(open, "IN PROGRESS", label);
    press(&open, plain(KeyCode::Enter))
}

/// `after` with what a palette run and a focused press may differ in set
/// aside: the focus fields, and the run kept in COMMANDS RUN RECENTLY.
fn but_focus(mut after: AppState) -> AppState {
    after.banner_focus = false;
    after.detail.focus = graft_ui::detail::DetailFocus::Graph;
    after.palette_recent = RecentRuns::default();
    after
}

#[test]
fn in_progress_rows_run_as_the_focused_banner_would() {
    // A merge tool configured: `M` hands the terminal over, a command to
    // compare. With none, its refusal spells the keys that work where the
    // keyboard is (`banner/resume.rs:73-77`), and the run leaves it where
    // it was — words that differ by design, as focus does.
    let unfocused = |mut state: AppState| {
        state.banner_focus = false;
        state.mergetool = Some("vimdiff".to_owned());
        state
    };
    let stops = vec![
        ("rebase edit stop", edit_stop()),
        (
            "rebase conflict stop",
            unfocused(stopped(InProgress::Rebase, &["a.rs"])),
        ),
        (
            "cherry-pick conflict stop",
            unfocused(stopped(InProgress::CherryPick, &["a.rs"])),
        ),
        (
            "merge conflict stop",
            unfocused(stopped(InProgress::Merge, &["a.rs"])),
        ),
    ];
    for (name, stop) in stops {
        assert!(!stop.banner_focus, "{name}: the banner is unfocused");
        let labels = in_progress_labels(&stop);
        assert!(!labels.is_empty(), "{name}: IN PROGRESS listed");
        if name.starts_with("merge") {
            assert!(
                !labels.iter().any(|l| l == "Skip this commit"),
                "{name}: no Skip row: {labels:?}"
            );
        }
        for label in &labels {
            let (by_row, row_commands) = run_row(&stop, label);
            let mut focused = stop.clone();
            focused.banner_focus = true;
            let (by_key, key_commands) = press(&focused, banner_chord(label));
            assert_eq!(row_commands, key_commands, "{name}: {label}: commands");
            // `c`'s refusal at a conflict names the keys acting where he
            // stands (m8/10l), and the palette run leaves the graph focused:
            // the reason is compared, the keys are not — as `M`'s refusal.
            let refused = label == "Continue" && name.contains("conflict");
            let (by_row_cmp, by_key_cmp) = keys_excepted(refused, &by_row, by_key, name);
            assert_eq!(
                but_focus(by_row_cmp),
                but_focus(by_key_cmp),
                "{name}: {label}: state"
            );
            assert_eq!(
                by_row.banner_focus, stop.banner_focus,
                "{name}: {label}: banner focus kept"
            );
            assert_eq!(
                by_row.detail.focus, stop.detail.focus,
                "{name}: {label}: detail focus kept"
            );
        }
    }
}

/// The two runs to compare: for `c`'s refusal at a conflict, keys dropped
/// after checking the palette run names only keys acting where it left him.
fn keys_excepted(
    refused: bool,
    by_row: &AppState,
    by_key: AppState,
    name: &str,
) -> (AppState, AppState) {
    if !refused {
        return (by_row.clone(), by_key);
    }
    let place = crate::update::edge::context_of(by_row);
    assert_eq!(
        sticky_keys(by_row),
        crate::update::banner::resolve_keys(place, &by_row.keymap),
        "{name}: Continue: no key named that does not act at {place:?}"
    );
    (without_keys(by_row.clone()), without_keys(by_key))
}

/// The sticky line's keys, when the status is one.
fn sticky_keys(state: &AppState) -> Option<String> {
    match &state.status {
        Status::Sticky { keys, .. } => keys.clone(),
        other => panic!("not a sticky line: {other:?}"),
    }
}

/// `state` with its sticky line's keys dropped, reason kept.
fn without_keys(mut state: AppState) -> AppState {
    if let Status::Sticky { keys, .. } = &mut state.status {
        *keys = None;
    }
    state
}

#[test]
fn abort_from_the_palette_never_amends() {
    let stop = edit_stop();
    assert!(!stop.banner_focus, "the graph has the keyboard");
    let (aborted, commands) = run_row(&stop, "Abort");
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ReadHeadMessage(_) | Command::Operate(..))),
        "nothing amends: {commands:?}"
    );
    let confirm = aborted.confirm.as_ref().expect("the abort preview is up");
    assert_eq!(confirm.view.title.as_str(), "Abort the rebase?");
    let (skipped, commands) = run_row(&stop, "Skip this commit");
    assert!(commands.is_empty(), "skip asks first: {commands:?}");
    let confirm = skipped.confirm.as_ref().expect("skip's preview is up");
    assert_eq!(confirm.view.title.as_str(), "Skip this commit?");
}

#[test]
fn every_action_row_names_a_context_its_run_reaches() {
    let focused_banner = stopped(InProgress::Rebase, &["a.rs"]);
    let mut journal = edit_stop();
    journal.journal.shown = true;
    let places = vec![
        ("the graph", edit_stop()),
        ("a focused banner", focused_banner),
        ("the journal", journal),
    ];
    for (name, state) in places {
        let at_open = crate::update::edge::context_of(&state);
        for opener in [
            plain(KeyCode::Char('?')),
            KeyPress::control(KeyCode::Char('p')),
        ] {
            let (open, _) = press(&state, opener);
            let palette = open
                .palette
                .as_ref()
                .unwrap_or_else(|| panic!("{name}: {opener:?} opens the palette"));
            assert_eq!(palette.opened_from, at_open, "{name}");
            let actions: Vec<(String, ContextId)> = shown(&open)
                .into_iter()
                .filter_map(|(_, label, target)| match target {
                    PaletteTarget::Action { ctx, .. } => Some((label, ctx)),
                    _ => None,
                })
                .collect();
            assert!(!actions.is_empty(), "{name}: rows listed");
            for (label, ctx) in actions {
                assert!(
                    [ContextId::Banner, ContextId::Global, at_open].contains(&ctx),
                    "{name}: {label} spelled in {ctx:?}, opened on {at_open:?}"
                );
            }
        }
    }
}

// ── Major 2: a placeholder cursor reaches any landed row ────────────────

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

/// A repository whose refs carry the tag `v2.1.0`: TAGS can be matched
/// before the branch read lands.
fn with_a_tag() -> AppState {
    let badges = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(1)),
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
    AppState {
        refs: Some(Arc::new(badges)),
        ..AppState::default()
    }
}

fn typed(state: &AppState, word: &str) -> (AppState, Vec<Command>) {
    let mut state = state.clone();
    let mut commands = Vec::new();
    for c in word.chars() {
        let (after, more) = press(&state, plain(KeyCode::Char(c)));
        state = after;
        commands.extend(more);
    }
    (state, commands)
}

fn names_asked(commands: &[Command]) -> Option<(JobId, NamesRequest)> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::PaletteNames(id, req) => Some((*id, req.clone())),
            _ => None,
        })
        .next_back()
}

fn answer(state: &AppState, (id, req): &(JobId, NamesRequest)) -> AppState {
    let hits = match_names(req, &|| false).expect("not stopped");
    update(
        state,
        AppEvent::Produced(*id, Product::PaletteNames(Arc::new(hits))),
    )
    .0
}

#[test]
fn cursor_on_placeholder_moves_to_a_landed_row_in_another_group() {
    let (open, commands) = press(&with_a_tag(), KeyPress::control(KeyCode::Char('p')));
    let read = commands
        .iter()
        .find_map(|c| match c {
            Command::ReadPaletteBranches(id) => Some(*id),
            _ => None,
        })
        .expect("a branch read");
    let (typing, asked) = typed(&open, "v2");
    let asked = names_asked(&asked).expect("tags asked");
    let typing = cursor_on(typing, "BRANCHES", "reading…");
    // TAGS answers while BRANCHES still reads.
    let tags = answer(&typing, &asked);
    assert_eq!(
        under_cursor(&tags),
        ("TAGS".to_owned(), "v2.1.0".to_owned()),
        "{:?}",
        shown(&tags)
    );
    // BRANCHES answers above it: the cursor keeps the tag.
    let branches = SwitchRefs {
        refs: vec![RefListing {
            name: name("refs/heads/v2-fix"),
            tip: oid(3),
            subject: String::new(),
            author: String::new(),
            when: 10,
            upstream: None,
        }],
        previous: None,
    };
    let (landed, commands) = update(
        &tags,
        AppEvent::Produced(read, Product::PaletteBranches(Ok(Arc::new(branches)))),
    );
    let landed = answer(&landed, &names_asked(&commands).expect("re-asked"));
    assert!(
        shown(&landed)
            .iter()
            .any(|(h, l, _)| h == "BRANCHES" && l == "v2-fix"),
        "{:?}",
        shown(&landed)
    );
    assert_eq!(
        under_cursor(&landed),
        ("TAGS".to_owned(), "v2.1.0".to_owned())
    );
}

#[test]
fn cursor_on_placeholder_stays_while_every_group_is_a_placeholder() {
    let mut nothing_read = with_a_tag();
    nothing_read.refs = None;
    let (open, _) = press(&nothing_read, KeyPress::control(KeyCode::Char('p')));
    let (typing, asked) = typed(&open, "v2");
    assert!(names_asked(&asked).is_none(), "nothing read to match yet");
    let typing = cursor_on(typing, "FILES", "reading…");
    let commits = typing
        .palette
        .as_ref()
        .and_then(|p| p.find.commits.pending.as_ref())
        .map(|(id, _)| *id)
        .expect("a commits job in flight");
    let (landed, _) = update(
        &typing,
        AppEvent::Produced(commits, Product::PaletteCommits(Ok(Arc::from([])))),
    );
    assert_eq!(
        under_cursor(&landed),
        ("FILES".to_owned(), "reading…".to_owned()),
        "{:?}",
        shown(&landed)
    );
}
