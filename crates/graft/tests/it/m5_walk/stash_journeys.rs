//! The stash put-back pass's journeys, counted (m6-conflicts/75, picks
//! 15–17): a stash that clashes, edits set aside around a rebase or a
//! switch, and the `.` menu's fixed slots — every row of
//! `docs/design-notes/m6-stash-put-back.md` "Journeys" (:37-46), walked as
//! the user takes it over the real app and real git, each route counted
//! against the note's `journey-counts` block by `run_scenario`. The group
//! walks in `tests/it/m6_stash_put_back.rs` stay; these are the counted
//! ones, and each ends on **git's** truth: the stash list, `git status`, the
//! branch, and `origin/main` where it pushes.
//!
//! A put-back loses the staged/unstaged split (ADR-0045 Consequences; since
//! m6-conflicts/83 the preview's `staged` counts only what is lost): the
//! walks assert the edits come back byte for byte and the staging does not.

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress};
use graft_fixtures::Shape;

use super::fixture::{EPOCH, Fixture, Remote};
use super::steps::{Capture, Scenario, Sees, Step, Wait, When};
use super::vocabulary::{ch, chain, key, notice_containing, plain, status_containing};
use crate::repo_truth::{check, run_git};

/// The card layout: the width the note's words are written for.
const CARD: (u16, u16) = (120, 40);

/// What every walk writes over a tracked file: his edit.
const EDIT: &str = "my edit\n";

/// The file the stash clashes in.
const NOTES: &str = "notes.md";

pub(super) fn journeys() -> Vec<Scenario> {
    vec![
        captured(stash_pop_clash_landing(), "m6-20-stash-clash-landing"),
        captured(stash_pop_clash_to_done(), "m6-21-stash-clash-done"),
        stash_apply_clash_to_done(),
        stash_clash_put_back(),
        captured(pull_rebase_dirty_preview(), "m6-22-set-aside-preview"),
        captured(pull_rebase_dirty(), "m6-24-set-aside-done"),
        rejected_push_dirty_to_pushed(),
        switch_dirty(),
        captured(switch_dirty_clash(), "m6-23-set-aside-clash-landing"),
        pull_rebase_stop_then_clash(),
        captured(menu_checkout(), "m6-25-menu-fixed-slots"),
        menu_cherry_pick(),
    ]
}

/// `scenario` with the pty capture named `stem`: the keys are the whole way
/// from the app's first frame. The cursor opens on `HEAD`'s row, or on the
/// uncommitted row when the tree is dirty (`j` from it lands on `HEAD`); the
/// `j k` in m6-23 and the `k` in m6-25 are counted by no journey — they only
/// reach the row the route starts on (the walk's `Step::CursorTo`).
fn captured(scenario: Scenario, stem: &'static str) -> Scenario {
    let (keys, on_screen) = match stem {
        "m6-20-stash-clash-landing" => (plain(&[ch('S'), ch('p'), ch('y')]), "stashed · stash@{0}"),
        "m6-21-stash-clash-done" => (
            plain(&[ch('S'), ch('p'), ch('y'), ch('o'), KeyCode::Enter]),
            "stash@{0} dropped",
        ),
        "m6-22-set-aside-preview" => (plain(&[ch('p'), ch('r')]), "your edits: 3 files"),
        "m6-23-set-aside-clash-landing" => (
            plain(&[ch('j'), ch('k'), ch('.'), KeyCode::Enter, ch('y')]),
            "yours · edits set aside",
        ),
        "m6-24-set-aside-done" => (plain(&[ch('p'), ch('r'), ch('r')]), "your edits are back"),
        "m6-25-menu-fixed-slots" => (plain(&[ch('k'), ch('.')]), "Checkout fix"),
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

fn journey(
    name: &'static str,
    remote: Remote,
    setup: super::steps::Fix,
    steps: Vec<Vec<Step>>,
) -> Scenario {
    Scenario {
        name,
        shape: if remote == Remote::None {
            Shape::Trivial
        } else {
            Shape::Linear
        },
        remote,
        size: CARD,
        setup,
        steps: chain(steps),
        expect: |_, _| Ok(()),
        capture: None,
    }
}

fn route() -> Step {
    Step::Route(true)
}

fn end() -> Step {
    Step::Route(false)
}

// ───────────────────────────── fixtures ─────────────────────────────

/// A fixture step or a truth check: `Err` says what is wrong.
type Checked = Result<(), String>;

/// Commit every `(name, body)` at once as `subject`, dated `minute`.
fn commit_all(fixture: &Fixture, files: &[(&str, &str)], subject: &str, minute: i64) -> Checked {
    for (name, body) in files {
        fixture.dirty(name, body)?;
        fixture.git(&["add", "--", name])?;
    }
    let when = format!("@{} +0000", EPOCH + minute * 60);
    graft_fixtures::git::run_at(fixture.repo(), &["commit", "-q", "-m", subject], &when)
        .map(drop)
        .map_err(|why| format!("committing {subject:?}: {why}"))
}

/// `notes.md` committed, edited to `stashed` and stashed, then `committed`
/// committed over the same line: popping or applying the stash clashes in
/// `notes.md`, one block. `main` checked out, the tree clean.
fn stash_over_notes(fixture: &Fixture) -> Checked {
    fixture.commit(NOTES, "base\n", "add the notes", 1)?;
    fixture.dirty(NOTES, "stashed\n")?;
    fixture.git(&["stash", "push", "-q"])?;
    fixture.commit(NOTES, "committed\n", "rewrite the notes", 2)
}

/// A teammate pushed; a local commit `ours` adding three files; those three
/// edited, `one.txt` staged (a staged modification: the staging a put-back
/// loses). `p` finds the branch diverged and the tree dirty.
fn diverged_with_three_edits(fixture: &Fixture) -> Checked {
    fixture
        .remote()?
        .commit_on_remote("the teammate's commit")
        .map_err(|why| format!("the teammate's push: {why}"))?;
    commit_all(
        fixture,
        &[
            ("one.txt", "committed\n"),
            ("two.txt", "committed\n"),
            ("three.txt", "committed\n"),
        ],
        "ours",
        1,
    )?;
    for name in THREE {
        fixture.dirty(name, EDIT)?;
    }
    fixture.git(&["add", "--", "one.txt"]).map(drop)
}

const THREE: [&str; 3] = ["one.txt", "two.txt", "three.txt"];

/// `main` one commit past the fork, `fix` one commit of its own (`fix the
/// bug`, adding `fix.txt`), `main` checked out, the tree clean.
fn main_and_fix(fixture: &Fixture) -> Checked {
    commit_all(
        fixture,
        &[("base.txt", "base\n"), ("notes.txt", "notes\n")],
        "add the base",
        1,
    )?;
    fixture.git(&["branch", "fix"])?;
    fixture.commit("main-one.txt", "one\n", "main moves on", 2)?;
    fixture.git(&["switch", "-q", "fix"])?;
    fixture.commit("fix.txt", "fixed\n", "fix the bug", 3)?;
    fixture.git(&["switch", "-q", "main"]).map(drop)
}

/// [`main_and_fix`] with `base.txt` and `notes.txt` edited — both held the
/// same by `fix` — and `base.txt` staged.
fn main_and_fix_dirty(fixture: &Fixture) -> Checked {
    main_and_fix(fixture)?;
    fixture.dirty("base.txt", EDIT)?;
    fixture.dirty("notes.txt", EDIT)?;
    fixture.git(&["add", "--", "base.txt"]).map(drop)
}

/// [`main_and_fix`], then `main` rewrites `base.txt` and `fix` is checked
/// out with `base.txt` edited: switching to `main` sets the edit aside, and
/// putting it back clashes in `base.txt`, one block.
fn fix_dirty_over_mains_base(fixture: &Fixture) -> Checked {
    main_and_fix(fixture)?;
    fixture.commit("base.txt", "main's base\n", "main changes the base", 4)?;
    fixture.git(&["switch", "-q", "fix"])?;
    fixture.dirty("base.txt", EDIT)
}

/// `f.txt` and `g.txt` pushed as a base; a teammate's commit changing both
/// on `origin/main`; a local `ours` changing `f.txt` and adding `ours.txt`;
/// `g.txt` edited in the tree. `p r r` stops on `f.txt`; once the rebase is
/// continued, putting `g.txt` back clashes with theirs.
fn diverged_on_f_with_g_edited(fixture: &Fixture) -> Checked {
    commit_all(
        fixture,
        &[("f.txt", "base\n"), ("g.txt", "base\n")],
        "f and g",
        1,
    )?;
    fixture.git(&["push", "-q", "origin", "HEAD:refs/heads/main"])?;
    commit_all(
        fixture,
        &[("f.txt", "theirs\n"), ("g.txt", "theirs\n")],
        "theirs",
        2,
    )?;
    fixture.git(&["push", "-q", "origin", "HEAD:refs/heads/main"])?;
    fixture.git(&["reset", "-q", "--hard", "HEAD~1"])?;
    commit_all(
        fixture,
        &[("f.txt", "ours\n"), ("ours.txt", "ours\n")],
        "ours",
        3,
    )?;
    fixture.dirty("g.txt", EDIT)
}

// ───────────────────────────── git's truth ─────────────────────────────

fn stash_list(fixture: &Fixture) -> Result<String, String> {
    fixture.git(&["stash", "list"])
}

fn no_stash(fixture: &Fixture) -> Checked {
    let list = stash_list(fixture)?;
    check(list.trim().is_empty(), || {
        format!("git stash list is {list:?}, not empty")
    })
}

fn nothing_unmerged(fixture: &Fixture) -> Checked {
    let unmerged = fixture.git(&["ls-files", "-u"])?;
    check(unmerged.trim().is_empty(), || {
        format!("git still lists unmerged entries: {unmerged:?}")
    })
}

fn on_branch(fixture: &Fixture, name: &str) -> Checked {
    let head = fixture.git(&["symbolic-ref", "HEAD"])?;
    check(head.trim() == format!("refs/heads/{name}"), || {
        format!("HEAD is {head:?}, not {name}")
    })
}

fn file_is(fixture: &Fixture, name: &str, body: &str) -> Checked {
    let bytes = fixture.bytes(name)?;
    check(bytes == body.as_bytes(), || {
        format!(
            "{name} is {:?}, not {body:?}",
            String::from_utf8_lossy(&bytes)
        )
    })
}

/// `git status --porcelain` is exactly these lines (sorted as git sorts).
fn status_is(fixture: &Fixture, lines: &[&str]) -> Checked {
    let status = fixture.git(&["status", "--porcelain", "--untracked-files=no"])?;
    let got: Vec<&str> = status.lines().collect();
    check(got == lines, || {
        format!("git status --porcelain is {got:?}, not {lines:?}")
    })
}

/// `HEAD~1` is `origin/main`: rebased onto the teammate's commit.
fn rebased_onto_origin(fixture: &Fixture) -> Checked {
    let parent = fixture.git(&["rev-parse", "HEAD~1"])?;
    let origin = fixture.git(&["rev-parse", "refs/remotes/origin/main"])?;
    check(parent.trim() == origin.trim(), || {
        format!("HEAD~1 is {parent:?}, origin/main {origin:?}")
    })
}

// ───────────────────────────── the stash clash (pick 15) ─────────────────────────────

/// `S` and its list: not a key of a route's own until the list is there.
fn stash_list_open() -> Vec<Step> {
    vec![key(ch('S')), Step::WaitFor(Wait::StashReady)]
}

/// A pop or apply, its preview confirmed with `y` (m6-conflicts/86), that
/// clashed: the `S` list closed, the resolver on
/// `notes.md` with the stash's side named, the notice saying how to put it
/// all back; git kept the stash.
fn clash_landing(verb_key: KeyCode) -> Vec<Step> {
    vec![
        Step::Key(KeyPress::plain(verb_key)),
        Step::WaitFor(Wait::PreviewReady),
        key(ch('y')),
        Step::Settle("the pop or apply"),
        Step::WaitFor(Wait::ConflictedPath(NOTES)),
        Step::WaitFor(Wait::ConflictOpened),
        Step::See("the clash landed in the resolver", |state| {
            check(!state.stash.shown, || "the stash list is open".to_owned())?;
            resolver_on(state, NOTES)?;
            notice_containing(
                state,
                "stash@{0} clashed in 1 file · esc, u puts it all back",
            )
        }),
        Step::Frame(
            "the landing, drawn",
            &[
                Sees::Has("yours · main"),
                Sees::Has("stashed · stash@{0}"),
                Sees::Has("stash@{0} clashed in 1 file · esc, u puts it all back"),
                Sees::BottomStarts("o yours"),
            ],
        ),
        Step::Shell(|fixture| {
            let list = stash_list(fixture)?;
            check(list.lines().count() == 1, || {
                format!("git kept the stash: {list:?}")
            })
        }),
    ]
}

/// `o` keeps yours in the one block, `⏎` marks `notes.md` done: the stage,
/// then the pop's drop (not an apply's), each run to its answer.
fn o_enter_done() -> Vec<Step> {
    vec![
        key(ch('o')),
        Step::WaitFor(Wait::ConflictOpened),
        key(KeyCode::Enter),
        Step::Settle("the stage behind ⏎"),
        Step::WaitFor(Wait::ResolvedPath(NOTES)),
        Step::Settle("the drop after the last file, if any"),
        Step::WaitFor(Wait::Ready),
        Step::WaitFor(Wait::FrameSettled),
    ]
}

fn resolver_on(state: &AppState, path: &str) -> Checked {
    let open = state.resolver.as_ref().ok_or("no resolver")?;
    check(open.path == std::path::Path::new(path), || {
        format!("the resolver is on {}, not {path}", open.path.display())
    })
}

/// `stash-pop-clash-to-done` up to the landing: `S p y`, three of its
/// five keys. The capture m6-20 is this frame.
fn stash_pop_clash_landing() -> Scenario {
    journey(
        "stash-pop-clash-to-done@landing",
        Remote::None,
        stash_over_notes,
        vec![
            vec![Step::WaitFor(Wait::Ready), route()],
            stash_list_open(),
            clash_landing(ch('p')),
            vec![end()],
        ],
    )
}

/// `stash-pop-clash-to-done`: `S p y o ⏎` — the pop clashes, yours kept, the
/// last file done drops the stash: `every file done · stash@{0} dropped ✓ ·
/// u brings it back`, `git stash list` empty, `notes.md` HEAD's.
fn stash_pop_clash_to_done() -> Scenario {
    journey(
        "stash-pop-clash-to-done",
        Remote::None,
        stash_over_notes,
        vec![
            vec![Step::WaitFor(Wait::Ready), route()],
            stash_list_open(),
            clash_landing(ch('p')),
            o_enter_done(),
            vec![
                end(),
                Step::See("every file done, the stash dropped", |state| {
                    check(state.resolver.is_none(), || {
                        "the resolver is still open".to_owned()
                    })?;
                    status_containing(
                        state,
                        "every file done · stash@{0} dropped · u brings it back",
                    )
                }),
                Step::Frame(
                    "the done line, drawn",
                    &[Sees::Has(
                        "every file done · stash@{0} dropped ✓ · u brings it back",
                    )],
                ),
                Step::Shell(|fixture| {
                    no_stash(fixture)?;
                    nothing_unmerged(fixture)?;
                    on_branch(fixture, "main")?;
                    file_is(fixture, NOTES, "committed\n")?;
                    // Yours is HEAD's: the resolution changed nothing.
                    status_is(fixture, &[])
                }),
            ],
        ],
    )
}

/// `stash-apply-clash-to-done`: `S ⏎ y o ⏎` — the same, but an apply keeps
/// its stash: `every file done · stash@{0} kept ✓`, one entry listed.
fn stash_apply_clash_to_done() -> Scenario {
    journey(
        "stash-apply-clash-to-done",
        Remote::None,
        stash_over_notes,
        vec![
            vec![Step::WaitFor(Wait::Ready), route()],
            stash_list_open(),
            clash_landing(KeyCode::Enter),
            o_enter_done(),
            vec![
                end(),
                Step::See("every file done, the stash kept", |state| {
                    check(state.resolver.is_none(), || {
                        "the resolver is still open".to_owned()
                    })?;
                    status_containing(state, "every file done · stash@{0} kept")
                }),
                Step::Frame(
                    "the done line, drawn",
                    &[Sees::Has("every file done · stash@{0} kept ✓")],
                ),
                Step::Shell(|fixture| {
                    let list = stash_list(fixture)?;
                    check(list.lines().count() == 1, || {
                        format!("an apply keeps its stash: {list:?}")
                    })?;
                    nothing_unmerged(fixture)?;
                    file_is(fixture, NOTES, "committed\n")?;
                    status_is(fixture, &[])
                }),
            ],
        ],
    )
}

/// `stash-clash-put-back`: from the pop's clash landing (not counted),
/// `esc u` — the tree as before the pop, the stash still listed where git
/// kept it. No take was made, so no drift box asks.
fn stash_clash_put_back() -> Scenario {
    journey(
        "stash-clash-put-back",
        Remote::None,
        stash_over_notes,
        vec![
            vec![Step::WaitFor(Wait::Ready)],
            stash_list_open(),
            clash_landing(ch('p')),
            vec![
                route(),
                key(KeyCode::Escape),
                Step::See("esc: the changes pane, the resolver shut", |state| {
                    check(state.resolver.is_none(), || {
                        "the resolver is still open".to_owned()
                    })
                }),
                key(ch('u')),
                Step::See("u asks nothing: no take was made", |state| {
                    check(state.confirm.is_none(), || "a box opened".to_owned())
                }),
                Step::Write("the undo of the pop"),
                Step::WaitFor(Wait::Ready),
                end(),
                Step::Shell(|fixture| {
                    let list = stash_list(fixture)?;
                    check(list.lines().count() == 1, || {
                        format!("the stash is listed again: {list:?}")
                    })?;
                    nothing_unmerged(fixture)?;
                    file_is(fixture, NOTES, "committed\n")?;
                    status_is(fixture, &[])
                }),
            ],
        ],
    )
}

// ───────────────────────────── set aside around a rebase (pick 16) ─────────────────────────────

/// `p` refused as diverged: the sticky line `r` answers.
fn pull_refused() -> Vec<Step> {
    vec![
        key(ch('p')),
        Step::Settle("the refused pull"),
        Step::See("the pull says diverged", |state| {
            status_containing(state, "have diverged")
        }),
        Step::WaitFor(Wait::Ready),
        // The pull's fetch moved `origin/main`; `r` plans against the refs
        // the app holds, so it waits for their re-read to land, as
        // `pull-diverged-offers-m-and-r` does. Pressed before it, the plan
        // is refused `Moved { origin/main }` (observed, 2026-09-26).
        Step::WaitFor(Wait::RefsCaughtUp),
        Step::WaitFor(Wait::RemotesCaughtUp),
    ]
}

/// `r`: the rebase preview on a dirty tree, with the set-aside row.
fn rebase_preview_with_edits() -> Vec<Step> {
    vec![
        key(ch('r')),
        Step::WaitFor(Wait::PreviewReady),
        Step::Frame(
            "the preview's set-aside row, drawn",
            &[
                Sees::Has("working tree:"),
                Sees::Has(
                    "your edits: 3 files set aside, put back after · 1 staged comes back unstaged",
                ),
                Sees::Above("working tree:", "your edits: 3 files set aside"),
            ],
        ),
    ]
}

/// `r` again confirms (P1): the rebase, then the put-back, one write.
fn rebase_confirmed() -> Vec<Step> {
    vec![
        key(ch('r')),
        Step::Write("the rebase and the put-back"),
        Step::WaitFor(Wait::Ready),
        // `P` refuses `still reading the refs — try again in a moment` until
        // the rebase's re-read lands (m6-conflicts/49): he sees the badges
        // move, then pushes.
        Step::WaitFor(Wait::RefsCaughtUp),
    ]
}

/// The three files as he left them, none staged (the put-back loses the
/// staging of `one.txt`, as git's `stash apply` without `--index` does).
fn three_edits_back(fixture: &Fixture) -> Checked {
    for name in THREE {
        file_is(fixture, name, EDIT)?;
    }
    status_is(fixture, &[" M one.txt", " M three.txt", " M two.txt"])
}

/// `pull-rebase-dirty` up to the preview: `p r`. The capture m6-22.
fn pull_rebase_dirty_preview() -> Scenario {
    journey(
        "pull-rebase-dirty@preview",
        Remote::Local,
        diverged_with_three_edits,
        vec![
            vec![Step::WaitFor(Wait::Ready), route()],
            pull_refused(),
            rebase_preview_with_edits(),
            vec![end()],
        ],
    )
}

/// `pull-rebase-dirty`: `p r r` over three edited files, one staged —
/// rebased onto `origin/main`, the edits back, no stash left.
fn pull_rebase_dirty() -> Scenario {
    journey(
        "pull-rebase-dirty",
        Remote::Local,
        diverged_with_three_edits,
        vec![
            vec![Step::WaitFor(Wait::Ready), route()],
            pull_refused(),
            rebase_preview_with_edits(),
            rebase_confirmed(),
            vec![
                end(),
                Step::WaitFor(Wait::FrameSettled),
                Step::See("the rebase's own line and the clause", |state| {
                    status_containing(state, "rebase main onto origin/main · your edits are back")
                }),
                Step::Frame(
                    "the done line, drawn",
                    &[Sees::Has(
                        "rebase main onto origin/main ✓ · your edits are back",
                    )],
                ),
                Step::Shell(|fixture| {
                    rebased_onto_origin(fixture)?;
                    on_branch(fixture, "main")?;
                    three_edits_back(fixture)?;
                    no_stash(fixture)
                }),
            ],
        ],
    )
}

/// `rejected-push-dirty-to-pushed`: from a rejected `P y` (not counted: the
/// journey starts at the rejection), `p r r P y` — his commit on the
/// teammate's at `origin/main` in the bare, the edits still in the tree.
fn rejected_push_dirty_to_pushed() -> Scenario {
    journey(
        "rejected-push-dirty-to-pushed",
        Remote::Local,
        diverged_with_three_edits,
        vec![
            vec![
                Step::WaitFor(Wait::Ready),
                key(ch('P')),
                key(ch('y')),
                Step::Settle("the rejected push"),
                Step::Shell(|fixture| {
                    let bare = bare_main(fixture)?;
                    let head = fixture.git(&["rev-parse", "HEAD"])?;
                    check(bare != head.trim(), || {
                        "the push was not rejected".to_owned()
                    })
                }),
                Step::WaitFor(Wait::Ready),
                route(),
            ],
            pull_refused(),
            rebase_preview_with_edits(),
            rebase_confirmed(),
            vec![
                key(ch('P')),
                Step::See("P opened the push box", |state| {
                    check(state.confirm.is_some(), || {
                        format!("no box; status {:?}", state.status)
                    })
                }),
                key(ch('y')),
                Step::Write("the push"),
                Step::WaitFor(Wait::Ready),
                end(),
                Step::Shell(|fixture| {
                    let head = fixture.git(&["rev-parse", "HEAD"])?;
                    let bare = bare_main(fixture)?;
                    check(bare == head.trim(), || {
                        format!("the bare's main is {bare}, HEAD {head:?}")
                    })?;
                    let log = git_in_bare(fixture, &["log", "--format=%s", "-2", "main"])?;
                    check(log.lines().eq(["ours", "the teammate's commit"]), || {
                        format!("origin's main log is {log:?}")
                    })?;
                    three_edits_back(fixture)?;
                    no_stash(fixture)
                }),
            ],
        ],
    )
}

fn git_in_bare(fixture: &Fixture, args: &[&str]) -> Result<String, String> {
    let bare = fixture.bare().ok_or("this scenario has no bare")?;
    run_git(bare, args)
}

fn bare_main(fixture: &Fixture) -> Result<String, String> {
    git_in_bare(fixture, &["rev-parse", "main"]).map(|id| id.trim().to_owned())
}

/// `pull-rebase-stop-then-clash`: `p r r o ⏎ c o ⏎` — the rebase stops on
/// `f.txt`, `o ⏎` settles it, `c` continues, putting `g.txt` back clashes,
/// `o ⏎` takes the branch (the rebased `theirs`): rebased, no stash left.
fn pull_rebase_stop_then_clash() -> Scenario {
    journey(
        "pull-rebase-stop-then-clash",
        Remote::Local,
        diverged_on_f_with_g_edited,
        vec![
            vec![Step::WaitFor(Wait::Ready), route()],
            pull_refused(),
            vec![
                key(ch('r')),
                Step::WaitFor(Wait::PreviewReady),
                key(ch('r')),
                Step::Settle("the rebase, stopping"),
                Step::WaitFor(Wait::ResolverLanded),
                Step::See("the stop is on f.txt", |state| resolver_on(state, "f.txt")),
                key(ch('o')),
                Step::WaitFor(Wait::ConflictOpened),
                key(KeyCode::Enter),
                Step::Settle("the stage behind ⏎"),
                Step::WaitFor(Wait::ResolvedPath("f.txt")),
                Step::WaitFor(Wait::ConflictOpened),
                key(ch('c')),
                Step::Settle("the continue and the put-back"),
                Step::WaitFor(Wait::ConflictedPath("g.txt")),
                Step::WaitFor(Wait::ConflictOpened),
                Step::See("the put-back clashed on g.txt", |state| {
                    resolver_on(state, "g.txt")?;
                    notice_containing(
                        state,
                        "your edits clashed in 1 file · esc, u undoes the rebase too",
                    )
                }),
                key(ch('o')),
                Step::WaitFor(Wait::ConflictOpened),
                key(KeyCode::Enter),
                Step::Settle("the stage behind ⏎"),
                Step::WaitFor(Wait::ResolvedPath("g.txt")),
                Step::Settle("the drop after the last file"),
                Step::WaitFor(Wait::Ready),
                end(),
                copy_dropped(),
                Step::Shell(|fixture| {
                    rebased_onto_origin(fixture)?;
                    let merge = fixture.git(&["rev-parse", "--git-path", "rebase-merge"])?;
                    check(!fixture.repo().join(merge.trim()).exists(), || {
                        "the rebase is still in progress".to_owned()
                    })?;
                    file_is(fixture, "g.txt", "theirs\n")?;
                    nothing_unmerged(fixture)?;
                    no_stash(fixture)
                }),
            ],
        ],
    )
}

// ───────────────────────────── set aside around a switch (pick 16) ─────────────────────────────

/// `. ⏎ y` on the cursor's branch row: slot 1 is `Checkout <b>`, its
/// preview, `y`.
fn dot_enter_y(checkout: &'static str) -> Vec<Step> {
    vec![
        key(ch('.')),
        Step::See("slot 1 is the checkout", first_slot_is),
        key(KeyCode::Enter),
        Step::WaitFor(Wait::PreviewReady),
        Step::See(checkout, |state| {
            check(state.preview.is_some(), || "no preview".to_owned())
        }),
        key(ch('y')),
    ]
}

/// The open menu's first item starts `Checkout ` and is not the dim one.
fn first_slot_is(state: &AppState) -> Checked {
    let open = state.context_menu.as_ref().ok_or("no menu")?;
    let first = open.menu.items.first().ok_or("an empty menu")?;
    check(
        first.label.as_str().starts_with("Checkout ") && first.disabled.is_none(),
        || format!("slot 1 is {:?}", first.label.as_str()),
    )?;
    check(open.menu.cursor == 0, || {
        format!("the menu opens on item {}", open.menu.cursor)
    })
}

/// `switch-dirty`: `. ⏎ y` on `fix`'s row, `main` dirty — on `fix`, both
/// edits in the tree, no stash left, the staging of `base.txt` lost.
fn switch_dirty() -> Scenario {
    journey(
        "switch-dirty",
        Remote::None,
        main_and_fix_dirty,
        vec![
            vec![Step::WaitFor(Wait::Ready), Step::CursorTo("fix"), route()],
            dot_enter_y("the checkout preview"),
            vec![
                Step::Write("the switch and the put-back"),
                Step::WaitFor(Wait::Ready),
                end(),
                Step::See("the switch's line and the clause", |state| {
                    status_containing(state, "your edits are back")
                }),
                Step::Shell(|fixture| {
                    on_branch(fixture, "fix")?;
                    file_is(fixture, "base.txt", EDIT)?;
                    file_is(fixture, "notes.txt", EDIT)?;
                    status_is(fixture, &[" M base.txt", " M notes.txt"])?;
                    no_stash(fixture)
                }),
            ],
        ],
    )
}

/// The end of a set-aside clash taken to done (ADR-0045 §3 amended): the
/// resolver shut, the line naming the dropped copy and the key that brings
/// it back.
fn copy_dropped() -> Step {
    Step::See("every file done, the copy dropped", |state| {
        check(state.resolver.is_none(), || {
            "the resolver is still open".to_owned()
        })?;
        status_containing(
            state,
            "every file done · set-aside copy dropped · u brings it back",
        )
    })
}

/// `switch-dirty-clash`: `. ⏎ y o ⏎` on `main`'s row from a dirty `fix` —
/// the put-back clashes in `base.txt`; `o branch` takes main's; on `main`, the
/// set-aside dropped. The capture m6-23 is the landing.
fn switch_dirty_clash() -> Scenario {
    journey(
        "switch-dirty-clash",
        Remote::None,
        fix_dirty_over_mains_base,
        vec![
            vec![Step::WaitFor(Wait::Ready), Step::CursorTo("main"), route()],
            dot_enter_y("the checkout preview"),
            vec![
                Step::Settle("the switch and the put-back"),
                Step::WaitFor(Wait::ConflictedPath("base.txt")),
                Step::WaitFor(Wait::ConflictOpened),
                Step::See("the put-back clashed on base.txt", |state| {
                    resolver_on(state, "base.txt")?;
                    notice_containing(
                        state,
                        "your edits clashed in 1 file · esc, u undoes the switch too",
                    )
                }),
                Step::Frame(
                    "the clash landing, drawn",
                    &[
                        Sees::Has("branch · main"),
                        Sees::Has("yours · edits set aside"),
                        Sees::Has("t yours"),
                        Sees::BottomStarts("o branch"),
                    ],
                ),
                key(ch('o')),
                Step::WaitFor(Wait::ConflictOpened),
                key(KeyCode::Enter),
                Step::Settle("the stage behind ⏎"),
                Step::WaitFor(Wait::ResolvedPath("base.txt")),
                Step::Settle("the drop after the last file"),
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::FrameSettled),
                end(),
                copy_dropped(),
                Step::Frame(
                    "the done line, drawn",
                    &[Sees::Has(
                        "every file done · set-aside copy dropped ✓ · u brings it back",
                    )],
                ),
                Step::Shell(|fixture| {
                    on_branch(fixture, "main")?;
                    file_is(fixture, "base.txt", "main's base\n")?;
                    nothing_unmerged(fixture)?;
                    no_stash(fixture)
                }),
            ],
        ],
    )
}

// ───────────────────────────── the menu's fixed slots (pick 17) ─────────────────────────────

/// `menu-checkout`: `. ⏎ y` on `fix`'s row, the tree clean — on `fix`.
/// The capture m6-25 is the menu open.
fn menu_checkout() -> Scenario {
    journey(
        "menu-checkout",
        Remote::None,
        main_and_fix,
        vec![vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("fix"),
            route(),
            key(ch('.')),
            Step::See("slot 1 Checkout fix, slot 2 Cherry-pick", |state| {
                let labels = labels(state)?;
                check(
                    labels.first().map(String::as_str) == Some("Checkout fix")
                        && labels.get(1).is_some_and(|l| l.starts_with("Cherry-pick ")),
                    || format!("the menu reads {labels:?}"),
                )
            }),
            Step::Frame(
                "the menu, drawn",
                &[Sees::Above("Checkout fix", "Cherry-pick ")],
            ),
            key(KeyCode::Enter),
            Step::WaitFor(Wait::PreviewReady),
            key(ch('y')),
            Step::Write("the checkout"),
            Step::WaitFor(Wait::Ready),
            end(),
            Step::Shell(|fixture| {
                on_branch(fixture, "fix")?;
                file_is(fixture, "fix.txt", "fixed\n")?;
                status_is(fixture, &[])?;
                no_stash(fixture)
            }),
        ]],
    )
}

/// `menu-cherry-pick`: `. j ⏎ y` on `fix`'s commit, `main` checked out —
/// the commit's patch lands on `main` as one new commit.
fn menu_cherry_pick() -> Scenario {
    journey(
        "menu-cherry-pick",
        Remote::None,
        main_and_fix,
        vec![vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("fix"),
            route(),
            key(ch('.')),
            key(ch('j')),
            Step::See("the cursor on slot 2, Cherry-pick", |state| {
                let open = state.context_menu.as_ref().ok_or("no menu")?;
                let on = open.menu.items.get(open.menu.cursor).ok_or("no item")?;
                check(
                    open.menu.cursor == 1 && on.label.as_str().starts_with("Cherry-pick "),
                    || format!("the cursor is on {:?}", on.label.as_str()),
                )
            }),
            key(KeyCode::Enter),
            Step::WaitFor(Wait::PreviewReady),
            key(ch('y')),
            Step::Write("the cherry-pick"),
            Step::WaitFor(Wait::Ready),
            end(),
            Step::Shell(|fixture| {
                on_branch(fixture, "main")?;
                let subject = fixture.git(&["log", "-1", "--format=%s"])?;
                check(subject.trim() == "fix the bug", || {
                    format!("main's tip is {subject:?}")
                })?;
                let parent = fixture.git(&["log", "-1", "--format=%s", "HEAD~1"])?;
                check(parent.trim() == "main moves on", || {
                    format!("one commit on main: its parent is {parent:?}")
                })?;
                file_is(fixture, "fix.txt", "fixed\n")?;
                status_is(fixture, &[])
            }),
        ]],
    )
}

fn labels(state: &AppState) -> Result<Vec<String>, String> {
    let open = state.context_menu.as_ref().ok_or("no menu")?;
    Ok(open
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str().to_owned())
        .collect())
}
