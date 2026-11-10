//! A fetch that moved a ref before it stopped (ADR-0062,
//! m8-polish-and-ship/21c Part B): the status line, J's row and its expanded
//! list say what moved, and `Esc` on such a fetch says so too (B+1).
//!
//! The setup is the 21b-3 probe's: a teammate pushed to `origin`, and a
//! second remote is not there, so `git fetch --all` moves `origin/main` and
//! then fails on the second.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::time::Duration;

use mergewright::update::{AppState, Status};
use mergewright_core::event::KeyCode;
use mergewright_core::journal::{EntryStatus, JournalEntry, OperationKind, RefMove};
use mergewright_fixtures::remote::{HungRemote, LocalRemote};

use crate::app_harness::App;
use crate::captures::{check_shot, frame};
use crate::remote_by_key::{finish, local, plain, ready, run};
use mergewright_core::on_disk::DIR;

/// The second remote, which is not there: a fixed path, so the failure's
/// words are the same on every machine.
const GONE: &str = "file:///nonexistent/graft-21c.git";

fn tracking(remote: &LocalRemote) -> String {
    format!("refs/remotes/origin/{}", remote.default_branch())
}

fn journal(repo: &std::path::Path) -> Vec<JournalEntry> {
    mergewright_ops::journal::Journal::open(&repo.join(format!(".git/{DIR}/undo-journal.jsonl")))
        .expect("the journal the writes made is readable")
        .entries()
        .to_vec()
}

/// The status line's words, whatever its kind.
fn line(status: &Status) -> String {
    match status {
        Status::Done { what, .. } => what.clone(),
        Status::Sticky { reason, keys, .. } => match keys {
            Some(keys) => format!("{reason} \u{b7} {keys}"),
            None => reason.clone(),
        },
        other => format!("{other:?}"),
    }
}

/// The last journal entry is the fetch, `Aborted`, and its `fetched_refs`
/// hold `origin/main` moving from `before` to `theirs` — nothing else.
fn recorded_the_move(work: &std::path::Path, tracking: &str, before: &str, theirs: &str) {
    let last = journal(work).pop().expect("the fetch was journaled");
    assert_eq!(last.kind, OperationKind::Fetch);
    assert!(
        matches!(last.status, EntryStatus::Aborted { .. }),
        "{:?}",
        last.status
    );
    let oid = |hex: &str| mergewright_core::oid::Oid::from_hex(hex).expect("40 hex digits");
    assert_eq!(
        last.fetched_refs,
        Some(std::collections::BTreeMap::from([(
            tracking.to_owned(),
            RefMove {
                before: Some(oid(before)),
                after: Some(oid(theirs)),
            }
        )]))
    );
}

/// 21c B+1: `Esc` on a fetch that has already moved `origin/main` and now
/// waits on a remote that never answers. The line says it was cancelled
/// **and** what moved before it stopped, `Done` as any cancel; the journal
/// holds the move.
#[test]
fn esc_on_a_fetch_that_moved_a_ref_says_what_moved() {
    let remote = local();
    let work = remote.work();
    let tracking = tracking(&remote);
    let before = run(&remote, work, &["rev-parse", &tracking]);
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    // `--all` fetches in the order the remotes were added: `origin` first,
    // then the one that hangs.
    let hung = HungRemote::listen().expect("the hung remote listens");
    run(&remote, work, &["remote", "add", "hung", &hung.url()]);

    let (app, state) = App::open_with_clock(
        work,
        mergewright::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW),
    );
    let state = ready(&app, state);
    let state = app.press(&state, plain('f'));
    assert!(state.operating.is_some());
    assert!(
        hung.wait_for_connection(Duration::from_secs(30)),
        "git never connected to the hung remote"
    );
    assert_eq!(
        run(&remote, work, &["rev-parse", &tracking]),
        theirs,
        "origin was fetched before git reached the hung remote"
    );

    let state = app.key(&state, KeyCode::Escape);
    let state = finish(&app, state, "the cancelled fetch");
    assert_eq!(
        state.status,
        Status::done("fetch all remotes cancelled \u{b7} origin/main moved before it stopped")
    );
    assert_eq!(state.remote.cancelling, None);
    recorded_the_move(work, &tracking, &before, &theirs);
    app.close();
}

/// The probe's state, after `f`: the fetch failed on `gone` after it moved
/// `origin/main`. Returns the app, its state, and the two tips.
fn fetched_part_way() -> (LocalRemote, App, AppState, String, String) {
    let remote = local();
    let work = remote.work().to_path_buf();
    let tracking = tracking(&remote);
    let before = run(&remote, &work, &["rev-parse", &tracking]);
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    run(&remote, &work, &["remote", "add", "gone", GONE]);

    let (app, state) = App::open_with_clock(
        &work,
        mergewright::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW),
    );
    let state = ready(&app, state);
    let state = app.press(&state, plain('f'));
    let state = finish(&app, state, "the fetch that fails on gone");
    recorded_the_move(&work, &tracking, &before, &theirs);
    (remote, app, state, before, theirs)
}

/// `J`, every stage answered.
fn in_j(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('J'));
    assert!(state.journal.shown, "J opened");
    app.pump(state, "J's stages 2 and 3", |state| {
        !state.journal.finding_lost()
    })
}

/// The fetch's `●` row (its description is cut to `fetch all…` where the
/// failure takes the room; the status line names it too, so the glyph
/// decides).
fn fetch_row(frame: &[String]) -> &str {
    frame
        .iter()
        .find(|line| line.contains('\u{25cf}') && line.contains("fetch all"))
        .unwrap_or_else(|| panic!("no fetch row:\n{}", frame.join("\n")))
}

/// Box 11: the status line after `f`, and J's fetch row collapsed and
/// expanded, at 120x40 and 80x24 — each asserted, then written to
/// `docs/shots/m8/`.
///
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it partial_fetch::capture_a_partial_fetch`.
#[test]
fn capture_a_partial_fetch() {
    let (remote, app, state, before, theirs) = fetched_part_way();
    let status = line(&state.status);
    assert!(
        status.ends_with(" \u{b7} origin/main moved before it stopped"),
        "{status}"
    );
    assert!(
        status.contains("no repository at '/nonexistent/graft-21c.git'"),
        "{status}"
    );
    let at_120 = frame(&state, 120, 40);
    check_shot(
        "m8/21c-status-partial-fetch-120x40.txt",
        &format!("{}\n{}\n", STATUS_HEADER, at_120.join("\n")),
    );

    let state = in_j(&app, &state);
    for (columns, rows) in [(120, 40), (80, 24)] {
        capture_j_at(&app, &state, (columns, rows), (&before, &theirs));
    }
    drop(state);
    drop(remote);
    app.close();
}

/// J at `columns`x`rows`: the fetch row collapsed, then expanded on `Space`
/// — asserted, then written to `docs/shots/m8/`. `tips` is origin/main's
/// tip before the fetch and after it.
fn capture_j_at(app: &App, state: &AppState, (columns, rows): (u16, u16), tips: (&str, &str)) {
    let collapsed = frame(state, columns, rows);
    let row = fetch_row(&collapsed);
    assert!(
        !row.contains("nothing written") && !row.contains("\u{2190} u"),
        "{row}"
    );
    // The failure leads and is kept whole where the row allows; at these
    // widths `no repository at '…/graft-21c.git' …` takes the room and the
    // moves clause after it is cut (NOTES, 21c Part B).
    assert!(row.contains("failed: no repository at"), "{row}");
    // …and J's status row, the failure line, says what moved.
    assert!(
        collapsed
            .iter()
            .any(|line| line.starts_with("⊗ fetch all remotes failed")
                && line.ends_with(" \u{b7} origin/main moved before it stopped")),
        "{}",
        collapsed.join("\n")
    );
    // The cursor on the fetch row (`k` from wherever J put it), then `Space`.
    let mut on_fetch = state.clone();
    for _ in 0..collapsed.len() {
        if fetch_row(&frame(&on_fetch, columns, rows)).contains('\u{25b8}') {
            break;
        }
        on_fetch = app.key(&on_fetch, KeyCode::Char('k'));
    }
    assert!(
        fetch_row(&frame(&on_fetch, columns, rows)).contains('\u{25b8}'),
        "k never reached the fetch row:\n{}",
        frame(&on_fetch, columns, rows).join("\n")
    );
    let expanded = app.key(&on_fetch, KeyCode::Char(' '));
    let open = frame(&expanded, columns, rows);
    let heading = open
        .iter()
        .position(|line| line.contains("refs the fetch moved"))
        .unwrap_or_else(|| panic!("no moved list:\n{}", open.join("\n")));
    assert!(
        open[heading]
            .trim_start()
            .starts_with("refs the fetch moved \u{b7} u does not move them back"),
        "{}",
        open[heading]
    );
    let (before, theirs) = tips;
    assert_eq!(
        open[heading + 1].trim_start(),
        format!(
            "\u{2506} origin/main  {} \u{2192} {}",
            &before[..7],
            &theirs[..7]
        ),
    );
    check_shot(
        &format!("m8/21c-journal-partial-fetch-{columns}x{rows}.txt"),
        &format!(
            "{}\n== J, collapsed ==\n\n{}\n\n== the fetch row, expanded (Space) ==\n\n{}\n",
            journal_header(columns, rows),
            collapsed.join("\n"),
            open.join("\n"),
        ),
    );
}

/// The status capture's caption.
const STATUS_HEADER: &str = "\
# m8/21c-status-partial-fetch-120x40.txt: 120x40, drawn by `view::view` from a
# real AppState (crates/mergewright/tests/it/partial_fetch.rs, capture_a_partial_fetch,
# asserted). A teammate pushed to origin; a second remote `gone` is not there;
# then `f`. Not a pty capture: TestBackend, no colour recorded \u{2014} colour is
# unverified (CLAUDE.md rule 5).
#
# What right looks like: the bottom line says the fetch failed (`no repository
# at '/nonexistent/graft-21c.git' \u{2026}`) and ends `\u{b7} origin/main moved before
# it stopped` \u{2014} origin's half of the fetch landed before gone's failed.
";

/// A journal capture's caption.
fn journal_header(columns: u16, rows: u16) -> String {
    format!(
        "\
# m8/21c-journal-partial-fetch-{columns}x{rows}.txt: {columns}x{rows}, drawn by `view::view`
# from a real AppState (crates/mergewright/tests/it/partial_fetch.rs,
# capture_a_partial_fetch, asserted). The fetch above failed on `gone` after it
# moved origin/main; then `J`, then `Space` on the fetch row. Not a pty capture:
# TestBackend, no colour recorded \u{2014} colour is unverified (CLAUDE.md rule 5).
#
# What right looks like: the \u{25cf} fetch row leads with `failed: no repository at
# '/nonexistent/graft-21c.git' \u{2026}` and has no `nothing written` mark and no
# `\u{2190} u`. Its summary goes on `\u{b7} origin/main moved before it stopped \u{b7} not
# undoable`, but this failure is long: at this width the row is cut after it
# (verdict first), so the moves show on J's status row above the legend (the
# failure line, ending `\u{b7} origin/main moved before it stopped`) and in the
# expanded row: `refs the fetch moved \u{b7} u does not move them back` over
# `\u{2506} origin/main  <7 hex> \u{2192} <7 hex>`, then the Command Log.
"
    )
}
