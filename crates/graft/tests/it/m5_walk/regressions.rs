//! Four promises M5 made about things that had gone wrong before, each
//! walked end to end: `Esc` really stops an undo and puts back what already
//! ran; two fast `⌃R`s leave one history walk and the app still gets its
//! rows; a typed password reaches nothing and draws eight glyphs whatever
//! its length; and at 80 columns the footer teaches three keys and `. more`.
//!
//! None of these judges a ghost, so none takes a [`Step::Claim`] — every
//! one asserts refs, files, the index tree and the journal for itself.

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::{EntryStatus, OperationKind};
use graft_fixtures::Shape;

use super::fixture::{
    PASSWORD, Remote, SHORT, forked_on_main, one_to_push, three_to_stage, twice_conflicting,
};
use super::steps::{Scenario, Step, Wait};
use super::vocabulary::{ch, chain, key, last_entry, same_files, same_refs};
use crate::repo_truth::check;

pub(super) fn regressions() -> Vec<Scenario> {
    vec![
        esc_cancels_undo_and_rolls_back(),
        ctrl_r_twice_leaves_one_history_job(),
        password_field_is_secret(),
        hints_at_80_show_three_and_more(),
    ]
}

// ──────────────────────── esc cancels an undo ────────────────────────────

/// A hard reset's undo — the three-step inverse plan that moves the ref,
/// the index *and* the working tree — started with `u` and then stopped
/// with `Esc` through the real `CancelJob` path.
///
/// **What it asserts is `graft_ops`' own invariant: a cancelled undo either
/// finished or changed nothing** (`graft-ops/tests/it/undo_cancel.rs`
/// header). The repository is exactly what it was before `u`, the line says
/// `undo cancelled`, and no `Undo` entry is left `Committed` — if one was
/// opened at all it is `Aborted ("cancelled")`.
///
/// It does **not** assert that an entry exists. Ticket 11 asked for
/// `Aborted("cancelled")` outright, and from the real key path that state
/// is not reachable: the walk presses `Esc` on the very next `update` after
/// `u`, before any answer from the pool has been fed back, so the cancel
/// always beats the worker to the first line of `restore::execute`
/// (`crates/graft-ops/src/restore.rs:179`, the `should_stop` before
/// `begin`) — the pool answers `Event::JobCancelled` and no entry is ever
/// opened. Which step boundary a later cancel lands on is
/// `graft-ops/tests/it/undo_cancel.rs`'s job, with a stop keyed to the
/// plan's own effect; a headless walk has no way to aim one. See the
/// ticket's NOTES.
///
/// The time machine's own revert cannot stand here either — it is launched
/// `launch_unstoppable`, and `Esc` answers it in words instead.
fn esc_cancels_undo_and_rolls_back() -> Scenario {
    Scenario {
        name: "esc-cancels-undo-and-rolls-back",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        // An uncommitted edit to a tracked file, so the hard reset's undo
        // has a working-tree step to put back as well as a ref and an index.
        setup: |fixture| {
            forked_on_main(fixture)?;
            fixture.dirty("base.txt", "edited, not committed\n")
        },
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::CursorToSubject("main moves on"),
                Step::MenuPick("Reset main to here, hard"),
                Step::WaitFor(Wait::PreviewReady),
                Step::Confirm,
                Step::Write("the reset"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("reset"),
            ],
            vec![
                key(ch('u')),
                Step::Cancel("the undo of the hard reset"),
                Step::See("the line says it was cancelled", |state| {
                    let said = super::run::status_text(state);
                    check(said.contains("undo cancelled"), || {
                        format!("the status is {said:?}")
                    })
                }),
                Step::WaitFor(Wait::Ready),
                Step::Mark("cancelled"),
            ],
        ]),
        expect: nothing_the_undo_would_have_moved_moved,
        capture: None,
    }
}

/// The repository stands exactly where the hard reset left it, and the
/// journal says so.
fn nothing_the_undo_would_have_moved_moved(
    _: &super::fixture::Fixture,
    marks: &super::truth::Marks,
) -> Result<(), String> {
    let reset = marks.get("reset")?;
    let cancelled = marks.get("cancelled")?;
    // Everything the three steps would have moved is where it was.
    same_refs("a cancelled undo", reset, cancelled)?;
    same_files("a cancelled undo", reset, cancelled)?;
    check(cancelled.index_tree == reset.index_tree, || {
        format!(
            "the index is {:?}, not the {:?} it was before u",
            cancelled.index_tree, reset.index_tree
        )
    })?;
    check(cancelled.head_symbolic == reset.head_symbolic, || {
        "the cancelled undo moved HEAD".to_owned()
    })?;
    // And the journal is honest about it: no undo of this reset was ever
    // committed, and an undo entry that was opened says it was cancelled
    // rather than sitting there `Pending` for ever.
    for entry in cancelled
        .journal
        .iter()
        .filter(|entry| entry.kind == OperationKind::Undo)
    {
        check(
            entry.status
                == (EntryStatus::Aborted {
                    reason: "cancelled".to_owned(),
                }),
            || format!("a cancelled undo's entry is {:?}", entry.status),
        )?;
    }
    // The reset the undo was about is still the operation in force: it is
    // the last entry when the undo never opened one, and the last one
    // before the aborted undo when it did.
    let entry = last_entry(cancelled)?;
    check(
        entry.kind == OperationKind::Reset || entry.kind == OperationKind::Undo,
        || format!("the last entry is {:?}", entry.kind),
    )
}

// ─────────────────────────── ⌃R twice ────────────────────────────────────

/// Two `⌃R`s in a row, with nothing pumped between them. The second
/// cancels the first (`update::keys::refresh_key` takes `awaiting` and
/// sends `CancelJob` for it), so **one** history walk is in flight — and
/// the app still gets its rows, its refs and its working tree back.
///
/// The regression this stands on is a worker that never came back: before
/// m5/05 the cancelled walk's thread was lost, and on a two-worker pool a
/// second `⌃R` could leave nothing to answer `ReadRefs` with. Reaching
/// [`Wait::Ready`] at all is what says that is fixed — the wait's own
/// deadline is the assertion.
fn ctrl_r_twice_leaves_one_history_job() -> Scenario {
    Scenario {
        name: "ctrl-r-twice-leaves-one-history-job",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: three_to_stage,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Mark("settled"),
            Step::Key(KeyPress::control(ch('r'))),
            Step::Key(KeyPress::control(ch('r'))),
            Step::See("one history walk is awaited, not two", |state| {
                check(state.awaiting.is_some(), || {
                    "⌃R started no history walk at all".to_owned()
                })?;
                check(state.promoting.is_none(), || {
                    format!(
                        "a second walk is still being promoted: {:?}",
                        state.promoting
                    )
                })
            }),
            Step::WaitFor(Wait::Ready),
            // `Wait::Ready` is the assertion: it feeds job answers until the
            // rows, the refs, the working tree and the in-progress state have
            // all landed. With a walk left parked on the two-worker pool the
            // reads queued behind it never run and this never returns
            // (`refresh_race::two_refreshes_before_started_leave_one_history_job`
            // is the same verdict in the small). Neither `awaiting` nor
            // `promoting` is checked here: the first names the walk whose
            // answer the screen wants and the second the sort it will
            // promote to, and both are legitimately still set once the rows
            // are on screen.
            Step::See("the rows, the refs and the tree are all back", |state| {
                check(state.refs.is_some() && state.working_tree.is_some(), || {
                    "the refs or the working tree never came back".to_owned()
                })?;
                let rows = state
                    .history
                    .as_ref()
                    .and_then(|history| history.window.as_ref())
                    .map_or(0, |window| window.rows.len());
                check(rows > 0, || "the refresh brought back no rows".to_owned())
            }),
            Step::Mark("refreshed"),
        ],
        expect: |_, marks| {
            let settled = marks.get("settled")?;
            let refreshed = marks.get("refreshed")?;
            same_refs("two ⌃Rs", settled, refreshed)?;
            same_files("two ⌃Rs", settled, refreshed)?;
            check(refreshed.journal.len() == settled.journal.len(), || {
                "a refresh journaled something".to_owned()
            })
        },
        capture: None,
    }
}

// ───────────────────────── the secret field ──────────────────────────────

/// The mask glyphs, at the two tiers.
const MASK: [char; 2] = ['\u{2022}', '\u{25e6}'];
/// What the field draws, whatever is typed into it (ADR-0038 §2).
const GLYPHS: usize = graft_ui::text_input::SECRET_GLYPHS;

/// The whole drawn frame, as `view::view` draws it — the same path a
/// `docs/shots/` capture records.
fn drawn(state: &AppState) -> Vec<String> {
    crate::captures::frame(state, 80, 24)
}

/// The frame shows exactly [`GLYPHS`] mask glyphs, all on one row, and no
/// byte of anything that was typed.
fn eight_glyphs_and_nothing_typed(state: &AppState) -> Result<(), String> {
    let frame = drawn(state);
    let masked: usize = frame
        .iter()
        .map(|line| line.chars().filter(|c| MASK.contains(c)).count())
        .sum();
    check(masked == GLYPHS, || {
        format!("the frame draws {masked} mask glyphs, not {GLYPHS}")
    })?;
    let row = frame
        .iter()
        .find(|line| line.chars().any(|c| MASK.contains(&c)))
        .ok_or_else(|| "no row carries the mask".to_owned())?;
    let run = row
        .chars()
        .skip_while(|c| !MASK.contains(c))
        .take_while(|c| MASK.contains(c))
        .count();
    check(run == GLYPHS, || {
        format!("the mask is drawn as {run} glyphs in a row, not {GLYPHS}")
    })?;
    for secret in super::fixture::SECRETS {
        // Not printed if it fails, for the obvious reason.
        check(!frame.iter().any(|line| line.contains(secret)), || {
            "the drawn frame carries a typed password".to_owned()
        })?;
    }
    Ok(())
}

/// A password typed into git's own credential question over Basic-auth
/// HTTP — the one transport that makes git ask.
///
/// What this proves, and the walk says no more than this: the bytes reach
/// **no** drawn frame, no event, no status line, no `{AppState:?}` taken
/// while the prompt was open, no argument or environment variable git
/// passed its askpass helper, and no file the run leaves behind (the
/// journal among them) — `run::no_secret_leaked`, run after every scenario
/// of the walk, and the frame check here. And the field draws
/// [`GLYPHS`] glyphs for a 7-character secret and the same
/// [`GLYPHS`] for a 28-character one, so the row says nothing about
/// the length.
fn password_field_is_secret() -> Scenario {
    Scenario {
        name: "password-field-is-secret",
        shape: Shape::Linear,
        remote: Remote::Http,
        size: (80, 24),
        setup: one_to_push,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the field is masked before anything is typed", |state| {
                let prompt = state
                    .credential
                    .as_ref()
                    .ok_or_else(|| "no credential prompt".to_owned())?;
                check(prompt.masked(), || {
                    "git's password question is not a masked field".to_owned()
                })?;
                check(prompt.prompt.as_str().contains("assword"), || {
                    format!("the prompt asks {:?}", prompt.prompt.as_str())
                })?;
                eight_glyphs_and_nothing_typed(state)
            }),
            Step::TypeSecret(SHORT),
            Step::See("seven characters draw the fixed mask", |state| {
                eight_glyphs_and_nothing_typed(state)
            }),
            Step::Key(KeyPress::control(ch('u'))),
            Step::TypeSecret(PASSWORD),
            Step::See("twenty-eight characters draw the same mask", |state| {
                eight_glyphs_and_nothing_typed(state)
            }),
            Step::Key(KeyPress::plain(KeyCode::Enter)),
            Step::Write("the push"),
            Step::WaitFor(Wait::Ready),
            Step::Mark("pushed"),
        ],
        expect: the_password_got_through,
        capture: None,
    }
}

/// The typed password was the right one: git got through Basic auth, the
/// bare has the commit, and the server counted an authorised request.
fn the_password_got_through(
    fixture: &super::fixture::Fixture,
    marks: &super::truth::Marks,
) -> Result<(), String> {
    let pushed = marks.get("pushed")?;
    let tip = pushed
        .repo
        .refs
        .get("refs/heads/main")
        .ok_or_else(|| "no local main".to_owned())?;
    let bare = pushed
        .bare
        .as_ref()
        .and_then(|refs| refs.get("refs/heads/main"))
        .ok_or_else(|| "the bare has no main".to_owned())?;
    check(bare == tip, || {
        format!("the bare's main is {bare}, main is {tip}")
    })?;
    let (authorised, _) = fixture
        .requests()
        .ok_or_else(|| "no HTTP counters".to_owned())?;
    check(authorised >= 1, || {
        format!("{authorised} requests were authorised")
    })?;
    // The two typed lengths differ, or the mask comparison above compared
    // nothing.
    check(SHORT.len() != PASSWORD.len(), || {
        "both typed secrets are the same length".to_owned()
    })
}

// ────────────────────────── three and more ───────────────────────────────

/// The footer at 80 columns, wherever it is drawn: the last row that
/// carries the overflow marker. Below 100 columns a focused banner draws
/// its own keys on the bottom row and a shown tab draws its own, so the bar
/// is found by its doorway rather than by position (ADR-0039, m5/28: `.
/// more` leads the bare keys rather than ending the row).
fn bar(state: &AppState) -> Result<String, String> {
    bar_by(state, ". more")
}

/// The footer found by some other doorway than `. more`: the context whose
/// whole list fits at 80 has no overflow marker to find it by.
fn bar_by(state: &AppState, marker: &str) -> Result<String, String> {
    crate::captures::frame(state, 80, 24)
        .into_iter()
        .rev()
        .find(|line| line.contains(marker))
        .ok_or_else(|| format!("no row of the frame carries `{marker}`"))
}

/// The bar opens with `<three hints>  . more`, and the hints are the first
/// three of the context's own list (the design note's table).
///
/// Everything after `. more` is the bare keys — the letters of the verbs
/// that did not fit, drawn without their labels (ADR-0039, m5/28: `. more`
/// **leads** them rather than ending the row). Three teaching hints and the
/// doorway is the claim; what follows the doorway is a different one, made
/// by `hint_bar`'s own tests.
fn bar_is(state: &AppState, three: &str) -> Result<(), String> {
    let drawn = bar(state)?;
    let wanted = format!(" {three}  . more");
    check(drawn.starts_with(&wanted), || {
        format!("the bar is {drawn:?}, which does not open {wanted:?}")
    })?;
    // And nothing else is taught before the doorway: the prefix above is
    // the whole of the bar up to `. more`.
    let before = drawn.split(". more").next().unwrap_or_default();
    check(before == format!(" {three}  "), || {
        format!("the bar teaches {before:?} before `. more`")
    })
}

/// The focused banner's bar: all seven of `hints::banner_list`'s hints and
/// no `. more`, because they fit. `⏎ resolve` joined them in
/// m6-conflicts/04 and leads the list (design note, Proposed change 2a).
fn the_banners_whole_list(state: &AppState) -> Result<(), String> {
    check(state.banner_focus, || {
        "⇥ did not take the banner's keyboard back".to_owned()
    })?;
    let drawn = bar_by(state, "c continue")?;
    let wanted = " ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph";
    check(drawn.trim_end() == wanted, || {
        format!("the banner's bar is {drawn:?}, not {wanted:?}")
    })?;
    check(!drawn.contains(". more"), || {
        "the banner's seven hints fit at 80: nothing is left over".to_owned()
    })
}

/// At 80 columns a context whose list does not fit teaches **three** keys
/// and then `. more` — never all of them, and never none (ADR-0037 §4);
/// a context whose list *does* fit teaches all of it and draws no `. more`
/// at all. The strings are the design note's own per-context lists.
///
/// The focused banner is the second kind: its seven hints still fit inside
/// 80, so ADR-0039's ladder never has to shorten them. The design note's own
/// 80-column mockup still shows `c continue  s skip  a abort  . more` there
/// — stale, from before the ladder replaced the width tiers (the note says
/// as much two lines above it). Filed in the ticket.
fn hints_at_80_show_three_and_more() -> Scenario {
    Scenario {
        name: "hints-at-80-show-three-and-more",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: twice_conflicting,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::See("the current branch's row", |state| {
                bar_is(state, "⏎ open  c commit  P push")
            }),
            key(ch('j')),
            Step::See("a plain commit row", |state| {
                bar_is(state, "⏎ open  b branch  r rebase")
            }),
            // A rebase that stops: the banner takes the keyboard, and a
            // rebase is the kind that can be skipped.
            Step::CursorTo("main"),
            key(ch('r')),
            Step::WaitFor(Wait::PreviewReady),
            Step::Confirm,
            Step::Settle("the rebase"),
            // m6-conflicts/18: the stop lands in the resolver; `Esc` is the
            // way back to the banner.
            Step::WaitFor(Wait::ResolverLanded),
            key(KeyCode::Escape),
            Step::WaitFor(Wait::BannerFocused),
            // The stop is a state, not a sticky line (m6-conflicts/12), so
            // the banner's keys are on the row already. `Esc` hands the
            // keyboard back and `Tab` takes it again (`banner::take_focus`);
            // the list is the same either way.
            key(KeyCode::Escape),
            key(KeyCode::Tab),
            Step::See("the focused banner", the_banners_whole_list),
            key(KeyCode::Escape),
            key(ch('g')),
            key(ch('k')),
            Step::See("the uncommitted-changes row", |state| {
                check(state.on_working_tree, || {
                    "`k` from the top did not select the uncommitted row".to_owned()
                })?;
                // With the rebase live and the banner one `Tab` away, the
                // way back leads (m6-conflicts/12) and the row's own three
                // become two. Graft began the rebase, so `c` continues it
                // (m6-conflicts/52).
                bar_is(state, "⇥ banner  ⏎ changes  c continue")
            }),
            key(KeyCode::Enter),
            Step::WaitFor(Wait::ChangesPane),
            // The unmerged branch of `hints::changes_pane_list`, not the
            // ordinary one — which starts `space stage`, so this string is
            // also what says the cursor really is on a conflicted file.
            // `⏎ resolve`, `M mergetool` and `space resolved` were
            // `planned()` here until m6-conflicts/04 bound all three; the
            // design note's parenthesis is today's bar now.
            // Six hints and the doorway measure 76 of the 80 columns (74
            // before m6-conflicts/52 said `c continue` for `c commit`), so
            // this is the *whole* list, not a three-and-`. more` prefix:
            // before m6-conflicts/04 three of the six were `planned()` and
            // the bar was short enough to be the other kind.
            Step::See("the changes pane, on an unmerged file", |state| {
                let drawn = bar(state)?;
                let wanted =
                    " ⏎ resolve  M mergetool  e edit  space resolved  c continue  ⇥ diff  . more";
                check(drawn.trim_end() == wanted, || {
                    format!("the bar is {drawn:?}, not {wanted:?}")
                })
            }),
            Step::Mark("looked"),
        ],
        expect: |_, marks| {
            let looked = marks.get("looked")?;
            check(looked.in_progress.as_deref() == Some("rebase"), || {
                format!("git says {:?}, not a rebase", looked.in_progress)
            })?;
            // Looking at the footer wrote nothing of its own: the one
            // journal entry is the rebase's, still pending.
            let entry = last_entry(looked)?;
            check(
                entry.kind == OperationKind::Rebase && entry.status == EntryStatus::Pending,
                || format!("the entry is {:?} {:?}", entry.kind, entry.status),
            )
        },
        capture: None,
    }
}
