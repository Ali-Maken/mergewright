//! M6's journeys, counted (m6-conflicts/21): every row of the note's
//! `journey-counts` block walked as the user takes it, from where he is to
//! done, over the real app and real git. Each journey's route sits between
//! `Step::Route(true)` and `Step::Route(false)`; `run_scenario` fails it when
//! the route sends more inputs than the row allows (`journeys.rs`). After
//! every event the landing is asserted — from the **drawn** frame
//! (`Step::Frame`), from the state where the frame cannot say it, and from
//! git (`Step::Shell`).
//!
//! Fixtures (`fixture.rs`): `conflicting` (one file, one block),
//! `three_blocks`, `two_files` (`gone.txt` modify/delete, `notes.txt` two
//! blocks, a clean `README.md` change) — each begun in the app, or in the
//! shell (`*_in_the_shell`) and the app then opened fresh.

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::OperationKind;
use graft_fixtures::Shape;

use super::fixture::{
    Fixture, Remote, conflicting, one_conflict_in_the_shell, three_blocks,
    three_blocks_in_the_shell, two_files, two_files_in_the_shell, two_files_pulled_in_the_shell,
};
use super::gestures::on_badge;
use super::steps::{Capture, Scenario, Sees, Step, Wait, When};
use super::vocabulary::{
    branch, ch, chain, entries_of, key, notice_containing, plain, status_containing, undo,
};
use crate::repo_truth::check;

/// The card layout (strip + card), and the column + three panes.
const CARD: (u16, u16) = (120, 40);
const WIDE: (u16, u16) = (200, 50);

pub(super) fn journeys() -> Vec<Scenario> {
    vec![
        a_merge_to_stop(),
        a_merge_to_stop_two_files(),
        captured(a_stop_to_first_conflict(), "m6-10-stop-lands-in-resolver"),
        captured(b_launch_to_first_conflict(), "m6-18-launch-shell-begun"),
        b_launch_to_first_conflict_after_a_pull(),
        b_launch_to_first_conflict_one_file(),
        b_launch_to_first_conflict_three_blocks(),
        one_file_one_conflict(),
        captured(three_conflicts_all_theirs(), "m6-13-after-ctrl-t"),
        captured(modify_delete_keep_yours(), "m6-11-card-on-notes"),
        captured(two_files_in_order(CARD), "m6-14-last-file-done"),
        two_files_in_order(WIDE),
        captured(two_files_out_of_order(), "m6-12-after-t"),
        captured(a_finish(), "m6-15-finish-summary"),
        b_finish(),
        b_finish_again_after_undo(),
        captured(summary_open_git_combined(), "m6-16-summary-expanded"),
        captured(back_after_esc(), "m6-19-after-esc-esc"),
        open_a_done_file(),
        walk_back_a_done_file(),
        a_whole_one_file(),
        captured(a_whole_two_files(), "m6-17-after-commit"),
        b_whole_two_files(),
        editor_to_finish(),
    ]
}

/// `scenario` with the pty capture named `stem` (m6-conflicts/21 Scope 7,
/// taken by ticket 43): one frame per landing. A capture run starts fresh on
/// the scenario's fixture and presses its keys through the real loop, so
/// each capture's keys are the whole way to its landing from the app's
/// first frame; the frame is taken once the screen settles with the
/// landing's words on it.
fn captured(scenario: Scenario, stem: &'static str) -> Scenario {
    let m = || KeyPress::plain(ch('m'));
    // `m m`: the banner's merge, then its confirm — the merge stops.
    let mm = || vec![m(), m()];
    // …then `o` keeps yours of `gone.txt` and `T` takes theirs of `notes.txt`.
    let both_done = || [mm(), plain(&[ch('o'), ch('T')])].concat();
    let then = |first: Vec<KeyPress>, more: &[KeyCode]| [first, plain(more)].concat();
    let (keys, on_screen) = match stem {
        "m6-10-stop-lands-in-resolver" => (mm(), "▸ gone.txt"),
        "m6-11-card-on-notes" => (then(mm(), &[ch('o')]), "▸ notes.txt"),
        "m6-12-after-t" => (then(mm(), &[ch('n'), ch('t')]), "took theirs"),
        "m6-13-after-ctrl-t" => (
            [mm(), vec![KeyPress::control(ch('t'))]].concat(),
            "took theirs",
        ),
        "m6-14-last-file-done" => (both_done(), "every file done"),
        "m6-15-finish-summary" => (then(both_done(), &[ch('c')]), "RESOLVED BY YOU"),
        "m6-16-summary-expanded" => (then(both_done(), &[ch('c'), ch(' ')]), "feature's line"),
        "m6-17-after-commit" => (
            then(both_done(), &[ch('c'), KeyCode::Enter]),
            "merged feature into main",
        ),
        // The launch's first frame: the app opens on the shell-begun merge.
        "m6-18-launch-shell-begun" => (vec![], "CONFLICTS"),
        "m6-19-after-esc-esc" => (then(mm(), &[KeyCode::Escape, KeyCode::Escape]), "⇥ banner"),
        other => panic!("no capture keys for {other}"),
    };
    Scenario {
        capture: Some(Capture {
            stem,
            headless: vec![],
            keys,
            on_screen,
            when: When::Settled,
        }),
        ..scenario
    }
}

fn journey(name: &'static str, setup: super::steps::Fix, steps: Vec<Vec<Step>>) -> Scenario {
    Scenario {
        name,
        shape: Shape::Trivial,
        remote: Remote::None,
        size: CARD,
        setup,
        steps: chain(steps),
        expect: |_, _| Ok(()),
        capture: None,
    }
}

// ───────────────────────────── the steps a route is made of ─────────────────────────────

fn route() -> Step {
    Step::Route(true)
}

fn end() -> Step {
    Step::Route(false)
}

/// The graph, cursor on `feature`: where an in-app merge starts.
fn on_feature() -> Vec<Step> {
    vec![Step::WaitFor(Wait::Ready), Step::CursorTo("feature")]
}

/// `m` [read the picture] `m` (P1): the merge runs and stops, and the
/// resolver it lands in (P5) has read its first file.
fn m_m() -> Vec<Step> {
    vec![
        key(ch('m')),
        Step::WaitFor(Wait::PreviewReady),
        key(ch('m')),
        Step::Settle("the merge"),
        Step::WaitFor(Wait::ResolverLanded),
    ]
}

/// The app opened on a merge git began and stopped: the banner has the
/// keyboard.
fn launched() -> Vec<Step> {
    vec![Step::WaitFor(Wait::BannerFocused)]
}

/// A key that finishes a file (`o`/`t` on a card, `O`/`T`), its stage, and
/// the resolver's next read.
fn done_by(side: char, path: &'static str) -> Vec<Step> {
    vec![
        key(ch(side)),
        Step::Settle("the file's resolution"),
        Step::WaitFor(Wait::ResolvedPath(path)),
        Step::WaitFor(Wait::ConflictOpened),
    ]
}

/// `two_files` by the in-spec keys: `gone.txt` kept (`o` on its card),
/// `notes.txt` theirs everywhere (`T`).
fn both_files_done() -> Vec<Step> {
    chain(vec![done_by('o', "gone.txt"), done_by('T', "notes.txt")])
}

/// `c` → the summary, `⏎` → the commit, everything re-read.
fn c_enter() -> Vec<Step> {
    vec![
        key(ch('c')),
        Step::WaitFor(Wait::FinishReady),
        key(KeyCode::Enter),
        Step::Settle("the merge commit"),
        Step::WaitFor(Wait::Ready),
        Step::WaitFor(Wait::NotInProgress),
        Step::WaitFor(Wait::LandedOnHead),
    ]
}

// ───────────────────────────── landings ─────────────────────────────

/// The stop's landing (J1, J2), and the launch's `⏎` (J3): the resolver on
/// the list's first file, cursor on its first block, `▸` on it, header
/// `MERGING`, the resolver's keys leading the bottom row.
fn in_the_resolver_on(first: &'static str) -> Vec<Step> {
    vec![
        Step::See(
            "the resolver is on the list's first file",
            on_the_first_file,
        ),
        Step::Frame(
            "the resolver, drawn",
            match first {
                "shared.txt" => &[
                    Sees::Has("MERGING"),
                    Sees::Row("▸", "shared.txt"),
                    Sees::BottomStarts("o yours"),
                ],
                "three.txt" => &[
                    Sees::Has("MERGING"),
                    Sees::Row("▸", "three.txt"),
                    Sees::BottomStarts("o yours"),
                ],
                _ => &[
                    Sees::Has("MERGING"),
                    Sees::Row("▸", "gone.txt"),
                    Sees::BottomStarts("o yours"),
                ],
            },
        ),
    ]
}

/// The stop is a state, and it says `merge stopped · …`.
fn stopped() -> Step {
    Step::See("the stop reads merge stopped · …", |state| {
        let stop = state.stopped.as_ref().ok_or("no stop recorded")?;
        let reason = graft::update::banner::stop_reason(stop, None);
        check(reason.starts_with("merge stopped · "), || {
            format!("the stop reads {reason:?}")
        })
    })
}

/// The resolver is open on the first file the list shows: on its first
/// block when it has blocks, on its card when it is a whole-file shape.
fn on_the_first_file(state: &AppState) -> Result<(), String> {
    let open = state.resolver.as_ref().ok_or("no resolver")?;
    let listed = graft::update::resolver::listed_files(state);
    let first = listed.first().ok_or("the list is empty")?;
    check(open.path == first.path, || {
        format!(
            "on {}, the list's first is {}",
            open.path.display(),
            first.path.display()
        )
    })?;
    if open.file.is_some() {
        check(open.cursor_block == Some(0), || {
            format!("cursor block {:?}, not 0", open.cursor_block)
        })
    } else {
        check(open.whole_file.is_some(), || {
            "neither a document nor a card".to_owned()
        })
    }
}

fn resolver_on(state: &AppState, path: &str) -> Result<(), String> {
    let open = state.resolver.as_ref().ok_or("no resolver")?;
    check(open.path == std::path::Path::new(path), || {
        format!("the resolver is on {}, not {path}", open.path.display())
    })
}

/// The last file done: the resolver stays, its notice offering `c` (P8,
/// ticket 31).
fn every_file_done() -> Vec<Step> {
    vec![
        Step::See("every file done, and c offered", |state| {
            notice_containing(state, "every file done ✓ · c finishes the merge")
        }),
        Step::Frame(
            "every file done, drawn",
            &[Sees::Has("every file done ✓ · c finishes the merge")],
        ),
        Step::Shell(|fixture| {
            let unmerged = fixture.git(&["ls-files", "-u"])?;
            check(unmerged.trim().is_empty(), || {
                format!("git still lists unmerged entries: {unmerged:?}")
            })
        }),
    ]
}

/// After `⏎` in the summary: the graph has the keyboard, its cursor on the
/// merge commit (`main`'s tip, `HEAD`), the status `merged feature into
/// main ✓`, and git agrees: `HEAD^2` is `feature`, nothing in progress.
fn merged_landing() -> Vec<Step> {
    vec![
        Step::See("the graph, cursor on the merge commit", |state| {
            check(
                state.resolver.is_none()
                    && state.finish.is_none()
                    && state.commit_editor.is_none()
                    && !state.banner_focus,
                || "something other than the graph has the keyboard".to_owned(),
            )?;
            check(on_badge(state, "main"), || {
                "the cursor is not on HEAD (main's tip)".to_owned()
            })?;
            status_containing(state, "merged feature into main")
        }),
        Step::Frame(
            "the merge's status, drawn",
            &[Sees::Has("merged feature into main ✓")],
        ),
        Step::Shell(second_parent_is_feature),
    ]
}

fn second_parent_is_feature(fixture: &Fixture) -> Result<(), String> {
    let second = fixture.git(&["rev-parse", "HEAD^2"])?;
    let feature = fixture.git(&["rev-parse", "feature"])?;
    check(second.trim() == feature.trim(), || {
        format!("HEAD^2 is {second:?}, feature is {feature:?}")
    })
}

// ───────────────────────────── J1, J2, J3 ─────────────────────────────

fn a_merge_to_stop() -> Scenario {
    journey(
        "a-merge-to-stop",
        conflicting,
        vec![
            on_feature(),
            vec![route()],
            m_m(),
            vec![end(), stopped()],
            in_the_resolver_on("shared.txt"),
        ],
    )
}

fn a_merge_to_stop_two_files() -> Scenario {
    journey(
        "a-merge-to-stop@two-files",
        two_files,
        vec![
            on_feature(),
            vec![route()],
            m_m(),
            vec![end(), stopped()],
            in_the_resolver_on("gone.txt"),
        ],
    )
}

fn a_stop_to_first_conflict() -> Scenario {
    journey(
        "a-stop-to-first-conflict",
        two_files,
        vec![
            on_feature(),
            m_m(),
            // The frame right after the stop is the resolver: no key.
            vec![route(), end()],
            in_the_resolver_on("gone.txt"),
        ],
    )
}

/// J3's launch landing: the banner, the pane on `CONFLICTS`, the stop
/// recorded as begun outside Graft. The footer draws the fullest rung of
/// `stop_wordings` that fits beside the bar (`update/banner/stop.rs:197`),
/// so at 120 columns it reads `merge in progress · …` without the origin;
/// the origin is asserted on the stop's full reason, the frame on the
/// words every rung keeps.
fn on_the_banner_begun_outside() -> Vec<Step> {
    vec![
        Step::See("the banner has the keyboard", |state| {
            check(state.banner_focus && state.resolver.is_none(), || {
                "the launch did not land on the banner".to_owned()
            })
        }),
        Step::See("the stop says begun outside Graft", |state| {
            let stop = state.stopped.as_ref().ok_or("no stop recorded")?;
            let reason = graft::update::banner::stop_reason(stop, None);
            check(
                reason.starts_with("merge in progress · begun outside Graft · "),
                || format!("the stop reads {reason:?}"),
            )
        }),
        Step::WaitFor(Wait::FrameSettled),
        Step::Frame(
            "the launch, drawn",
            &[Sees::Has("merge in progress"), Sees::Has("CONFLICTS")],
        ),
    ]
}

fn launch_then_enter(first: &'static str) -> Vec<Vec<Step>> {
    vec![
        launched(),
        on_the_banner_begun_outside(),
        vec![
            route(),
            key(KeyCode::Enter),
            Step::WaitFor(Wait::ConflictOpened),
            end(),
        ],
        in_the_resolver_on(first),
    ]
}

fn b_launch_to_first_conflict() -> Scenario {
    journey(
        "b-launch-to-first-conflict",
        two_files_in_the_shell,
        launch_then_enter("gone.txt"),
    )
}

fn b_launch_to_first_conflict_one_file() -> Scenario {
    journey(
        "b-launch-to-first-conflict@one-file",
        one_conflict_in_the_shell,
        launch_then_enter("shared.txt"),
    )
}

fn b_launch_to_first_conflict_three_blocks() -> Scenario {
    journey(
        "b-launch-to-first-conflict@three-blocks",
        three_blocks_in_the_shell,
        launch_then_enter("three.txt"),
    )
}

fn b_launch_to_first_conflict_after_a_pull() -> Scenario {
    Scenario {
        remote: Remote::Local,
        ..journey(
            "b-launch-to-first-conflict@pull",
            two_files_pulled_in_the_shell,
            launch_then_enter("gone.txt"),
        )
    }
}

// ───────────────────────────── J4, J5, J5b ─────────────────────────────

fn one_file_one_conflict() -> Scenario {
    journey(
        "one-file-one-conflict",
        conflicting,
        vec![
            on_feature(),
            m_m(),
            vec![route()],
            done_by('O', "shared.txt"),
            vec![end()],
            every_file_done(),
            vec![Step::Shell(|fixture| is_heads("shared.txt", fixture))],
        ],
    )
}

/// `path` on disk is `HEAD`'s (yours, whole), and staged as that.
fn is_heads(path: &str, fixture: &Fixture) -> Result<(), String> {
    let head = fixture.git(&["show", &format!("HEAD:{path}")])?;
    let disk = fixture.bytes(path)?;
    check(disk == head.as_bytes(), || {
        format!(
            "{path} is {:?}, HEAD's is {head:?}",
            String::from_utf8_lossy(&disk)
        )
    })?;
    let unmerged = fixture.git(&["ls-files", "-u", "--", path])?;
    check(unmerged.trim().is_empty(), || {
        format!("{path} is still unmerged: {unmerged:?}")
    })?;
    let differs = fixture.git(&["diff", "--cached", "--name-only", "HEAD", "--", path])?;
    check(differs.trim().is_empty(), || {
        format!("{path} staged differs from HEAD's")
    })
}

fn three_conflicts_all_theirs() -> Scenario {
    journey(
        "three-conflicts-all-theirs",
        three_blocks,
        vec![
            on_feature(),
            m_m(),
            vec![Step::Shell(|fixture| {
                let markers = String::from_utf8_lossy(&fixture.bytes("three.txt")?)
                    .lines()
                    .filter(|line| line.starts_with("<<<<<<< "))
                    .count();
                check(markers == 3, || {
                    format!("three.txt stopped on {markers} blocks, not 3")
                })
            })],
            vec![route()],
            done_by('T', "three.txt"),
            vec![end()],
            every_file_done(),
            vec![Step::Shell(three_is_theirs_block_by_block)],
        ],
    )
}

/// `three.txt` equals git's own per-block answer: `git merge-file --theirs`
/// over the base, ours and theirs the merge had — not `checkout --theirs`.
fn three_is_theirs_block_by_block(fixture: &Fixture) -> Result<(), String> {
    let git_dir = fixture.git(&["rev-parse", "--absolute-git-dir"])?;
    let dir = std::path::Path::new(git_dir.trim()).join("walk-merge-file");
    std::fs::create_dir_all(&dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let base = fixture.git(&["merge-base", "HEAD", "feature"])?;
    let mut names = Vec::new();
    for (name, rev) in [
        ("ours", "HEAD"),
        ("base", base.trim()),
        ("theirs", "feature"),
    ] {
        let body = fixture.git(&["show", &format!("{rev}:three.txt")])?;
        let path = dir.join(name);
        std::fs::write(&path, body).map_err(|why| format!("{}: {why}", path.display()))?;
        names.push(path.display().to_string());
    }
    let want = fixture.git(&[
        "merge-file",
        "-p",
        "--theirs",
        &names[0],
        &names[1],
        &names[2],
    ])?;
    let got = fixture.bytes("three.txt")?;
    check(got == want.as_bytes(), || {
        format!(
            "three.txt is {:?}, per-block theirs is {want:?}",
            String::from_utf8_lossy(&got)
        )
    })
}

fn modify_delete_keep_yours() -> Scenario {
    journey(
        "modify-delete-keep-yours",
        two_files,
        vec![
            on_feature(),
            m_m(),
            vec![route()],
            done_by('o', "gone.txt"),
            vec![
                end(),
                Step::See("on notes.txt, gone.txt done", |state| {
                    resolver_on(state, "notes.txt")?;
                    notice_containing(state, "gone.txt done ✓ · ▸ notes.txt")
                }),
                Step::Frame(
                    "the notice and the next file, drawn",
                    &[
                        Sees::Has("gone.txt done ✓ · ▸ notes.txt"),
                        Sees::Row("▸", "notes.txt"),
                    ],
                ),
                Step::Shell(|fixture| is_heads("gone.txt", fixture)),
            ],
        ],
    )
}

// ───────────────────────────── J8, J9 ─────────────────────────────

/// A click on each list row selects that file (not counted: the walk only
/// asserts it works at this width).
fn clicks_select() -> Vec<Step> {
    vec![
        Step::Click("notes.txt"),
        Step::WaitFor(Wait::ConflictOpened),
        Step::See("the click opened notes.txt", |state| {
            resolver_on(state, "notes.txt")
        }),
        Step::Click("gone.txt"),
        Step::WaitFor(Wait::ConflictOpened),
        Step::See("the click opened gone.txt", |state| {
            resolver_on(state, "gone.txt")
        }),
    ]
}

fn two_files_in_order(size: (u16, u16)) -> Scenario {
    let (name, arrangement): (&'static str, Step) = if size == WIDE {
        (
            "two-files-in-order@200x50",
            Step::Frame(
                "the list column and three panes",
                &[
                    Sees::Has("FILES · 2"),
                    Sees::Has("result"),
                    Sees::Lacks("files · "),
                ],
            ),
        )
    } else {
        (
            "two-files-in-order",
            Step::Frame(
                "the strip and the card",
                &[Sees::Lacks("FILES · "), Sees::Lacks("│ result")],
            ),
        )
    };
    Scenario {
        size,
        ..journey(
            name,
            two_files,
            vec![
                on_feature(),
                m_m(),
                vec![arrangement],
                clicks_select(),
                vec![route()],
                both_files_done(),
                vec![end()],
                every_file_done(),
                vec![Step::Frame(
                    "both files done in the list",
                    &[Sees::Row("gone.txt", "✓"), Sees::Row("notes.txt", "✓")],
                )],
            ],
        )
    }
}

fn two_files_out_of_order() -> Scenario {
    journey(
        "two-files-out-of-order",
        two_files,
        vec![
            on_feature(),
            m_m(),
            vec![
                route(),
                key(ch('n')),
                Step::WaitFor(Wait::ConflictOpened),
                Step::See("n crossed to notes.txt", |state| {
                    resolver_on(state, "notes.txt")
                }),
            ],
            done_by('T', "notes.txt"),
            vec![Step::See("back on gone.txt, the file not done", |state| {
                resolver_on(state, "gone.txt")
            })],
            done_by('o', "gone.txt"),
            vec![end()],
            every_file_done(),
        ],
    )
}

// ───────────────────────────── J6, J6b, J10 ─────────────────────────────

/// The summary as `c` opens it on `two_files`.
fn the_summary() -> Step {
    Step::Frame(
        "the finish summary",
        &[
            Sees::Has("RESOLVED BY YOU · 2"),
            Sees::Row("gone.txt", "kept · yours · main"),
            Sees::Row("notes.txt", "theirs · feature in 2 of 2 conflicts"),
            Sees::Has("COMBINED BY GIT ON ITS OWN · 1"),
            Sees::Row("▸", "README.md"),
            Sees::Above("COMBINED BY GIT ON ITS OWN", "README.md"),
        ],
    )
}

fn a_finish() -> Scenario {
    Scenario {
        expect: |_, marks| {
            let before = branch(&marks.before, "main")?;
            let undone = branch(marks.get("undone")?, "main")?;
            check(before == undone, || {
                format!("u left main on {undone}, the pre-merge tip is {before}")
            })
        },
        ..journey(
            "a-finish",
            two_files,
            vec![
                on_feature(),
                m_m(),
                both_files_done(),
                // The start: every file done, the resolver still open (P8).
                every_file_done(),
                vec![
                    Step::See("c is pressed in the resolver", |state| {
                        check(state.resolver.is_some(), || "no resolver open".to_owned())
                    }),
                    route(),
                    key(ch('c')),
                    Step::WaitFor(Wait::FinishReady),
                    the_summary(),
                    key(KeyCode::Enter),
                    Step::Settle("the merge commit"),
                    Step::WaitFor(Wait::Ready),
                    Step::WaitFor(Wait::NotInProgress),
                    Step::WaitFor(Wait::LandedOnHead),
                    end(),
                ],
                merged_landing(),
                // A truth check, not counted: `u` restores the pre-merge tip.
                undo(),
                vec![Step::Mark("undone")],
            ],
        )
    }
}

/// A shell-begun `two_files` merge with both files done in the app.
fn shell_merge_resolved_in_the_app() -> Vec<Vec<Step>> {
    vec![
        launched(),
        vec![key(KeyCode::Enter), Step::WaitFor(Wait::ConflictOpened)],
        both_files_done(),
    ]
}

fn merge_head_is_back(fixture: &Fixture) -> Result<(), String> {
    let merge_head = fixture.git(&["rev-parse", "-q", "--verify", "MERGE_HEAD"])?;
    let feature = fixture.git(&["rev-parse", "feature"])?;
    check(merge_head.trim() == feature.trim(), || {
        format!("MERGE_HEAD is {merge_head:?}, feature is {feature:?}")
    })
}

fn b_finish() -> Scenario {
    Scenario {
        expect: |_, marks| {
            let finished = marks.get("finished")?;
            let entries = entries_of(finished, OperationKind::FinishMerge);
            check(entries == 1, || {
                format!("{entries} FinishMerge entries, not one")
            })
        },
        ..journey(
            "b-finish",
            two_files_in_the_shell,
            chain_all(vec![
                shell_merge_resolved_in_the_app(),
                vec![vec![route()], c_enter(), vec![end()]],
                vec![
                    merged_landing(),
                    vec![Step::Mark("finished")],
                    undo(),
                    vec![Step::Shell(merge_head_is_back)],
                ],
            ]),
        )
    }
}

fn b_finish_again_after_undo() -> Scenario {
    journey(
        "b-finish-again-after-undo",
        two_files_in_the_shell,
        chain_all(vec![
            shell_merge_resolved_in_the_app(),
            vec![
                c_enter(),
                undo(),
                vec![
                    Step::WaitFor(Wait::BackInProgress),
                    Step::Shell(merge_head_is_back),
                    Step::See("u lands on the banner", |state| {
                        check(state.banner_focus, || {
                            "the banner does not have the keyboard".to_owned()
                        })
                    }),
                ],
            ],
            vec![vec![route()], c_enter(), vec![end()], merged_landing()],
        ]),
    )
}

fn summary_open_git_combined() -> Scenario {
    journey(
        "summary-open-git-combined",
        two_files,
        vec![
            on_feature(),
            m_m(),
            both_files_done(),
            vec![
                key(ch('c')),
                Step::WaitFor(Wait::FinishReady),
                the_summary(),
                Step::Frame(
                    "README.md's change not shown yet",
                    &[
                        Sees::Lacks("feature's line"),
                        Sees::Above("README.md", "MESSAGE"),
                    ],
                ),
                route(),
                key(ch(' ')),
                Step::WaitFor(Wait::FrameSettled),
                end(),
                Step::Frame(
                    "README.md's change under its row, the rest moved down",
                    &[
                        Sees::Above("README.md", "feature's line"),
                        Sees::Above("feature's line", "MESSAGE"),
                    ],
                ),
            ],
        ],
    )
}

// ───────────────────────────── J7 ─────────────────────────────

fn back_after_esc() -> Scenario {
    journey(
        "back-after-esc",
        two_files,
        vec![
            on_feature(),
            m_m(),
            // The journey's start, not counted: out to the graph.
            vec![
                key(KeyCode::Escape),
                key(KeyCode::Escape),
                Step::See("the graph has the keyboard", |state| {
                    check(state.resolver.is_none() && !state.banner_focus, || {
                        "Esc Esc did not reach the graph".to_owned()
                    })
                }),
                Step::Frame(
                    "the way back leads the bar",
                    &[Sees::BottomStarts("⇥ banner")],
                ),
                route(),
                key(KeyCode::Tab),
                Step::See("⇥ gives the banner the keyboard", |state| {
                    check(state.banner_focus, || {
                        "⇥ did not reach the banner".to_owned()
                    })
                }),
                key(KeyCode::Enter),
                Step::WaitFor(Wait::ConflictOpened),
                end(),
            ],
            in_the_resolver_on("gone.txt"),
        ],
    )
}

// ───────────────────────────── J11, J12 (Amendment 3) ─────────────────────────────

fn open_a_done_file() -> Scenario {
    journey(
        "open-a-done-file",
        two_files,
        vec![
            on_feature(),
            m_m(),
            done_by('o', "gone.txt"),
            vec![
                // `k` (P7) rather than the click: after a click the bottom
                // row keeps the last `Done` status (see ticket 21 NOTES).
                route(),
                key(ch('k')),
                Step::WaitFor(Wait::ConflictOpened),
                end(),
                Step::See("gone.txt open read-only", |state| {
                    resolver_on(state, "gone.txt")?;
                    check(
                        state
                            .resolver
                            .as_ref()
                            .is_some_and(|open| open.done.is_some()),
                        || "gone.txt is not open as done".to_owned(),
                    )
                }),
                Step::Frame(
                    "the done file, drawn",
                    &[
                        Sees::Row("▸ gone.txt", "done ✓"),
                        Sees::Has("e edit"),
                        Sees::Has("esc close"),
                    ],
                ),
            ],
        ],
    )
}

fn walk_back_a_done_file() -> Scenario {
    journey(
        "walk-back-a-done-file",
        two_files,
        vec![
            on_feature(),
            m_m(),
            vec![key(ch('n')), Step::WaitFor(Wait::ConflictOpened)],
            done_by('T', "notes.txt"),
            vec![
                Step::See("the landing after ⏎: the next file open", |state| {
                    resolver_on(state, "gone.txt")
                }),
                route(),
                key(ch('j')),
                Step::WaitFor(Wait::ConflictOpened),
                key(ch('d')),
                key(KeyCode::Right),
                key(KeyCode::Enter),
                Step::Settle("the start over"),
                Step::WaitFor(Wait::ConflictedPath("notes.txt")),
                Step::WaitFor(Wait::ConflictOpened),
                end(),
                Step::See("notes.txt open again, cursor on block 0", |state| {
                    resolver_on(state, "notes.txt")?;
                    let open = state.resolver.as_ref().ok_or("no resolver")?;
                    check(open.done.is_none() && open.cursor_block == Some(0), || {
                        format!(
                            "done {:?}, cursor block {:?}",
                            open.done.is_some(),
                            open.cursor_block
                        )
                    })
                }),
                Step::Frame("the list's ▸ on notes.txt", &[Sees::Row("▸", "notes.txt")]),
                Step::Shell(|fixture| {
                    let stages = fixture.git(&["ls-files", "-u", "--", "notes.txt"])?;
                    check(stages.lines().count() == 3, || {
                        format!("notes.txt has stages {stages:?}, not three")
                    })
                }),
            ],
        ],
    )
}

// ───────────────────────────── whole rows ─────────────────────────────

fn a_whole_one_file() -> Scenario {
    journey(
        "a-whole-one-file",
        conflicting,
        vec![
            on_feature(),
            vec![route()],
            m_m(),
            done_by('O', "shared.txt"),
            c_enter(),
            vec![end()],
            merged_landing(),
        ],
    )
}

fn a_whole_two_files() -> Scenario {
    journey(
        "a-whole-two-files",
        two_files,
        vec![
            on_feature(),
            vec![route()],
            m_m(),
            both_files_done(),
            c_enter(),
            vec![end()],
            merged_landing(),
        ],
    )
}

fn b_whole_two_files() -> Scenario {
    journey(
        "b-whole-two-files",
        two_files_in_the_shell,
        vec![
            launched(),
            vec![
                route(),
                key(KeyCode::Enter),
                Step::WaitFor(Wait::ConflictOpened),
            ],
            both_files_done(),
            c_enter(),
            vec![end()],
            merged_landing(),
        ],
    )
}

// ───────────────────────────── editor-to-finish (ticket 39) ─────────────────────────────

/// `two_files` merged in the shell, then resolved there too: `gone.txt`
/// kept, `notes.txt` theirs — the app opens on a merge with nothing left
/// but the commit.
fn two_files_resolved_in_the_shell(fixture: &Fixture) -> Result<(), String> {
    two_files_in_the_shell(fixture)?;
    fixture.git(&["add", "--", "gone.txt"])?;
    fixture.git(&["checkout", "--theirs", "--", "notes.txt"])?;
    fixture.git(&["add", "--", "notes.txt"]).map(drop)
}

fn editor_to_finish() -> Scenario {
    journey(
        "editor-to-finish",
        two_files_resolved_in_the_shell,
        vec![
            vec![Step::WaitFor(Wait::OnTheBanner)],
            // The start: the commit editor, opened from the changes pane.
            vec![
                key(KeyCode::Tab),
                key(ch('g')),
                key(ch('k')),
                key(KeyCode::Enter),
                Step::WaitFor(Wait::ChangesPane),
                key(ch('c')),
                Step::See("the commit editor is open", |state| {
                    check(state.commit_editor.is_some(), || {
                        "c in the changes pane did not open the editor".to_owned()
                    })
                }),
                route(),
                key(KeyCode::Escape),
                key(KeyCode::Tab),
                key(KeyCode::Tab),
                Step::See("⇥ ⇥ reached the banner", |state| {
                    check(state.banner_focus, || "not on the banner".to_owned())
                }),
            ],
            c_enter(),
            vec![end()],
            merged_landing(),
        ],
    )
}

fn chain_all(parts: Vec<Vec<Vec<Step>>>) -> Vec<Vec<Step>> {
    parts.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::super::journeys::{inputs_of, journey_id, note_text, parse_counts, within_the_note};
    use super::*;

    /// A scenario's route, read from its steps the way `Walk` counts it as
    /// it drives: every input between `Route(true)` and `Route(false)`.
    fn route_of(scenario: &Scenario) -> Result<Option<Vec<String>>, String> {
        let mut inputs: Option<Vec<String>> = None;
        let mut counting = false;
        for step in &scenario.steps {
            match step {
                Step::Route(on) => {
                    counting = *on;
                    if *on {
                        inputs.get_or_insert_with(Vec::new);
                    }
                }
                _ if counting => inputs
                    .as_mut()
                    .ok_or("counting with no route")?
                    .extend(inputs_of(step)?),
                _ => {}
            }
        }
        Ok(inputs)
    }

    /// Every row of the block is walked, and every journey walks a row
    /// (both ways); a whole row is the sum of its parts; each route's
    /// count is within its row.
    #[test]
    fn journey_counts_match_the_note() {
        let rows = parse_counts(&note_text().unwrap()).unwrap();
        let walked: BTreeSet<&str> = journeys()
            .iter()
            .map(|scenario| {
                journey_id(scenario.name, &rows)
                    .unwrap_or_else(|| panic!("{} walks no row of the block", scenario.name))
            })
            .collect();
        let noted: BTreeSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(walked, noted, "journeys walked vs the note's rows");

        for row in rows.iter().filter(|row| !row.parts.is_empty()) {
            let sum: usize = row
                .parts
                .iter()
                .map(|part| {
                    rows.iter()
                        .find(|other| &other.id == part)
                        .and_then(|other| other.keys)
                        .unwrap_or_else(|| panic!("{}: part {part} has no count", row.id))
                })
                .sum();
            assert_eq!(
                row.keys,
                Some(sum),
                "{}: the whole is not its parts",
                row.id
            );
        }

        for scenario in journeys() {
            let route = route_of(&scenario).unwrap();
            within_the_note(&scenario, route.as_ref()).unwrap();
            // The count table TESTING.md quotes (`--nocapture`).
            let sent = route.unwrap_or_default();
            println!(
                "route {}: {} [{}]",
                scenario.name,
                sent.len(),
                sent.join(" ")
            );
        }
    }

    /// One key more than the note allows fails, naming both numbers and
    /// the keys sent.
    #[test]
    fn one_key_over_the_note_fails() {
        let mut scenario = journeys()
            .into_iter()
            .find(|scenario| scenario.name == "one-file-one-conflict")
            .unwrap();
        let end = scenario
            .steps
            .iter()
            .position(|step| matches!(step, Step::Route(false)))
            .unwrap();
        scenario.steps.insert(end, key(ch('j')));
        let route = route_of(&scenario).unwrap();
        let why = within_the_note(&scenario, route.as_ref()).unwrap_err();
        assert!(
            why.starts_with(
                "one-file-one-conflict: 2 inputs sent, the note's journey-counts allows 1: "
            ),
            "{why}"
        );
    }
}
