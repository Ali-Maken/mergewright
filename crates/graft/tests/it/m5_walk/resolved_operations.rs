//! M6's promise, part two (m6-conflicts/07): the operations around a
//! resolution — a rebase that stops twice, a conflicted `stash pop` with
//! no banner and its undo, a `kill -9` mid-rebase and the relaunch, `u`
//! straight after `c`, the re-push lease after a push — and the resolver's
//! own edges: modal keys, `Esc`, a frame too small to draw it. Truth from
//! git every time; `AppState` only for what is on screen.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::{EntryStatus, OperationKind};
use graft_fixtures::Shape;

use super::conflicts::merge_that_stops;
use super::fixture::{
    Fixture, POPPED, Remote, ahead_to_merge_and_push, as_built, conflicting, stash_pop_conflict,
    stash_to_pop_over_a_conflict, twice_conflicting, twice_conflicting_in_two_places,
};
use super::remotes::merged_and_pushed;
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::{Marks, Truth};
use super::vocabulary::{
    branch, ch, chain, entries_of, key, plain, redo, same_files, status_containing, undo,
};
use crate::repo_truth::{check, run_git};

pub(super) fn resolved_operations() -> Vec<Scenario> {
    vec![
        rebase_stops_twice_resolved_twice(),
        stash_pop_conflict_resolves_without_a_banner(),
        undo_a_resolution_in_a_stash_pop_conflict(),
        kill_9_mid_rebase_then_relaunch(),
        kill_9_mid_rebase_then_abort(),
        u_right_after_continue_is_accepted(),
        repush_lease_is_the_entrys_tip(),
        resolver_is_modal_and_esc_returns(),
        tiny_frame_says_so(),
    ]
}

// ───────────────────────────── vocabulary ─────────────────────────────

/// `r` on `main` from `feature`, confirmed, stopped on the first conflict.
fn rebase_that_stops() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        Step::CursorTo("main"),
        key(ch('r')),
        Step::WaitFor(Wait::PreviewReady),
        Step::Confirm,
        Step::Settle("the rebase"),
        Step::WaitFor(Wait::BannerFocused),
    ]
}

/// `⏎` from the focused banner: the resolver on git's first conflicted
/// path, its read landed.
fn resolve_from_the_banner() -> Vec<Step> {
    vec![
        Step::See("the banner has the keyboard", |state| {
            check(state.banner_focus, || {
                "the banner is not focused".to_owned()
            })
        }),
        key(KeyCode::Enter),
        Step::WaitFor(Wait::ConflictOpened),
    ]
}

/// A take on the cursor block, and its answer.
fn take(side: char) -> Vec<Step> {
    vec![key(ch(side)), Step::WaitFor(Wait::ConflictOpened)]
}

/// `⏎` on the last conflicted file: the stage, and the resolver closing to
/// the banner with `c` offered (`c finishes the merge` / `c continues the
/// rebase`, m6-conflicts/13 R5).
fn last_file_resolved(path: &'static str) -> Vec<Step> {
    vec![
        key(KeyCode::Enter),
        Step::Settle("the stage behind ⏎"),
        Step::WaitFor(Wait::ResolvedPath(path)),
        Step::See("all resolved, back on the banner", |state| {
            status_containing(state, "every file done ✓ · c ")?;
            check(state.resolver.is_none() && state.banner_focus, || {
                "the resolver did not close to the banner".to_owned()
            })
        }),
    ]
}

/// A file's bytes in a truth.
fn file_in(truth: &Truth, path: &str) -> Result<Vec<u8>, String> {
    truth
        .repo
        .files
        .get(std::path::Path::new(path))
        .cloned()
        .ok_or_else(|| format!("{path} is not in the tree"))
}

/// `ls-files -s` lines for `path` at stage 1, 2 or 3.
fn stages_in(truth: &Truth, path: &str) -> usize {
    truth
        .repo
        .index
        .iter()
        .filter(|line| line.ends_with(&format!("\t{path}")) && !line.contains(" 0\t"))
        .count()
}

/// The one entry of `kind`, `Committed`.
fn one_committed(truth: &Truth, kind: OperationKind) -> Result<(), String> {
    let count = entries_of(truth, kind);
    let statuses: Vec<_> = truth
        .journal
        .iter()
        .map(|entry| (entry.kind, entry.status.clone()))
        .collect();
    check(count == 1, || {
        format!("{count} {kind:?} entries, not one: {statuses:?}")
    })?;
    check(
        truth
            .journal
            .iter()
            .any(|entry| entry.kind == kind && entry.status == EntryStatus::Committed),
        || format!("the {kind:?} entry is not Committed: {statuses:?}"),
    )
}

/// `rebase-stops-twice-in-two-places`' file once each line took `top` and
/// `bottom`.
fn two_places(top: &str, bottom: &str) -> Vec<u8> {
    format!("{top}\nkeep 1\nkeep 2\nkeep 3\nkeep 4\n{bottom}\n").into_bytes()
}

// ───────────────────────────── the scenarios ─────────────────────────────

/// A rebase that stops twice, resolved twice in the resolver, `c` after
/// each: one `Rebase` entry, `Committed`; one `u` takes the whole rebase
/// back.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn rebase_stops_twice_resolved_twice() -> Scenario {
    Scenario {
        name: "rebase-stops-twice-resolved-twice",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: twice_conflicting,
        steps: chain(vec![
            rebase_that_stops(),
            resolve_from_the_banner(),
            // `b`, not `o`: `o` would leave the first replayed commit empty
            // and git would drop it; `b` keeps it, and the second commit
            // still stops.
            take('b'),
            last_file_resolved("shared.txt"),
            vec![
                key(ch('c')),
                Step::Settle("the first continue"),
                Step::WaitFor(Wait::BannerFocused),
                Step::Mark("second stop"),
            ],
            resolve_from_the_banner(),
            take('t'),
            last_file_resolved("shared.txt"),
            vec![
                key(ch('c')),
                Step::Write("the second continue"),
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::NotInProgress),
                Step::Mark("rebased"),
            ],
            undo(),
            vec![Step::Mark("undone")],
        ]),
        expect: |fixture, marks| {
            check(
                marks.get("second stop")?.in_progress.as_deref() == Some("rebase"),
                || "the rebase did not stop a second time".to_owned(),
            )?;
            let rebased = marks.get("rebased")?;
            check(
                rebased.in_progress.is_none() && rebased.index_tree.is_some(),
                || format!("after the second c git says {:?}", rebased.in_progress),
            )?;
            check(
                file_in(rebased, "shared.txt")? == b"feature's second line\n",
                || "shared.txt is not the second replayed commit's".to_owned(),
            )?;
            let tip = branch(rebased, "feature")?;
            let onto = run_git(fixture.repo(), &["rev-parse", &format!("{tip}~2")])?;
            check(onto.trim() == branch(&marks.before, "main")?, || {
                "feature's two commits are not on main".to_owned()
            })?;
            one_committed(rebased, OperationKind::Rebase)?;
            let undone = marks.get("undone")?;
            check(
                branch(undone, "feature")? == branch(&marks.before, "feature")?,
                || "one u did not put feature back".to_owned(),
            )
        },
        capture: Some(Capture {
            stem: "m6-07-all-resolved",
            headless: rebase_that_stops(),
            keys: plain(&[KeyCode::Enter, ch('t'), KeyCode::Enter]),
            on_screen: "every file done ✓",
            when: When::Settled,
        }),
    }
}

/// A conflicted `git stash pop` — no operation, no banner — resolved from
/// the changes pane: `every file done ✓` with nothing to continue, and
/// `⏎` is one journaled `stage p`.
fn stash_pop_conflict_resolves_without_a_banner() -> Scenario {
    Scenario {
        name: "stash-pop-conflict-resolves-without-a-banner",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: stash_pop_conflict,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                Step::See("a stash pop is no operation", |state| {
                    check(state.in_progress.is_none() && !state.banner_focus, || {
                        "the app thinks something is in progress".to_owned()
                    })
                }),
                Step::Resolve(POPPED),
            ],
            take('o'),
            vec![
                key(KeyCode::Enter),
                Step::Settle("the stage behind ⏎"),
                Step::WaitFor(Wait::ResolvedPath(POPPED)),
                Step::See("resolved, with nothing to continue", |state| {
                    check(
                        super::run::status_text(state) == "every file done ✓",
                        || format!("the status is {:?}", state.status),
                    )?;
                    check(state.resolver.is_none() && !state.banner_focus, || {
                        "the resolver is open, or a banner took focus".to_owned()
                    })
                }),
                Step::Mark("resolved"),
            ],
        ]),
        expect: |_, marks| {
            let resolved = marks.get("resolved")?;
            check(file_in(resolved, POPPED)? == b"committed\n", || {
                "o did not keep the working tree's side".to_owned()
            })?;
            check(stages_in(resolved, POPPED) == 0, || {
                "git still lists p as unmerged".to_owned()
            })?;
            check(
                resolved.journal.len() == marks.before.journal.len() + 1,
                || "⏎ did not add exactly one entry".to_owned(),
            )?;
            let last = resolved.journal.last().ok_or("no entry")?;
            check(
                last.description == format!("stage {POPPED}")
                    && last.status == EntryStatus::Committed,
                || format!("the entry is {:?} {:?}", last.description, last.status),
            )
        },
        capture: None,
    }
}

/// A pop that conflicts, popped **in the app** (`S`, `p`) — a pop run in
/// the shell has no journal entry for `u` to take back: `⏎`; `u` brings
/// the conflict back (three stages, the bytes from before `⏎`); `u` again
/// takes the pop back; `U` puts the conflict back once more. Red today on
/// finding F2: the redo also drops the stash the conflicted pop kept.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn undo_a_resolution_in_a_stash_pop_conflict() -> Scenario {
    Scenario {
        name: "undo-a-resolution-in-a-stash-pop-conflict",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        // Not the shell pop the scenario above starts from: the pop goes
        // through the app's stash list (`S`, `p`), so it is a journal entry
        // the second `u` can take back. A shell pop is in no journal.
        setup: stash_to_pop_over_a_conflict,
        steps: chain(vec![
            vec![
                Step::WaitFor(Wait::Ready),
                key(ch('S')),
                Step::WaitFor(Wait::StashReady),
                key(ch('p')),
                Step::Settle("the pop"),
                Step::WaitFor(Wait::ConflictedPath(POPPED)),
                Step::Mark("popped"),
                // git's own words for a pop that stopped: the app runs
                // `stash pop --index`, and git says the index was not
                // unstashed; the stash stays listed.
                Step::See("the pop stopped, in git's words", |state| {
                    status_containing(state, "Index was not unstashed")
                }),
                // The first `Esc` clears the stop's line, the second closes
                // the stash list.
                key(KeyCode::Escape),
                key(KeyCode::Escape),
                Step::See("the stash list is closed", |state| {
                    check(!state.stash.shown, || "the stash list is open".to_owned())
                }),
                Step::Resolve(POPPED),
            ],
            take('o'),
            vec![
                Step::Mark("taken"),
                key(KeyCode::Enter),
                Step::Settle("the stage behind ⏎"),
                Step::WaitFor(Wait::ResolvedPath(POPPED)),
                Step::See("every file done", |state| {
                    status_containing(state, "every file done ✓")
                }),
            ],
            undo(),
            vec![
                // The pane's `CONFLICTS` row is back: the re-read lists `p`
                // as unmerged, and git agrees.
                Step::WaitFor(Wait::ConflictedPath(POPPED)),
                Step::Mark("undone once"),
                // The second `u` reaches the pop. The tree is not the one
                // the pop left — `o` changed `p` since — so M3's drift box
                // asks first; `Undo` is the second choice.
                key(ch('u')),
                Step::Settle("the drift check"),
                Step::See("the drift box names the pop", |state| {
                    let title = &state.confirm.as_ref().ok_or("no drift box")?.view.title;
                    check(title.contains("pop stash@{0}"), || {
                        format!("the box is titled {title:?}")
                    })
                }),
                Step::Confirm,
                Step::Write("the second undo"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("undone twice"),
            ],
            redo(),
            vec![Step::Mark("redone")],
        ]),
        expect: |_, marks| {
            let taken = marks.get("taken")?;
            let once = marks.get("undone once")?;
            check(stages_in(once, POPPED) == 3, || {
                format!("after one u: {} stages for p", stages_in(once, POPPED))
            })?;
            check(file_in(once, POPPED)? == file_in(taken, POPPED)?, || {
                "after one u p is not the bytes from before ⏎".to_owned()
            })?;
            let twice = marks.get("undone twice")?;
            check(stages_in(twice, POPPED) == 0, || {
                format!("after two: {} stages for p", stages_in(twice, POPPED))
            })?;
            // Before the pop: the tree clean at the commit, the stash listed.
            same_files("the second u", &marks.before, twice)?;
            check(twice.repo.stash == marks.before.repo.stash, || {
                format!(
                    "the stash list is {:?}, was {:?}",
                    twice.repo.stash, marks.before.repo.stash
                )
            })?;
            let redone = marks.get("redone")?;
            check(stages_in(redone, POPPED) == 3, || {
                format!("after U: {} stages for p", stages_in(redone, POPPED))
            })?;
            // `U` puts back what the second `u` found: the index unmerged
            // and `p` as `o` left it — not the pop's markers, which `o`
            // replaced before either `u` ran.
            check(file_in(redone, POPPED)? == file_in(once, POPPED)?, || {
                "after U p is not what the second u found".to_owned()
            })?;
            // The pop stopped on the conflict, so git kept the stash; the
            // redo of its undo must keep it too.
            let popped = marks.get("popped")?;
            check(redone.repo.stash == popped.repo.stash, || {
                format!(
                    "after U the stash list is {:?}; after the pop it was {:?}",
                    redone.repo.stash, popped.repo.stash
                )
            })
        },
        capture: None,
    }
}

/// Up to the kill: the rebase stops on `shared.txt` with two blocks, `o`
/// on the first — the file half-taken on disk — and the app is dropped
/// with no shutdown (`Step::Kill`, which checks `rebase-merge/` and the
/// journal's `Pending` line), then opened again.
fn killed_half_taken() -> Vec<Step> {
    chain(vec![
        rebase_that_stops(),
        resolve_from_the_banner(),
        take('o'),
        vec![
            Step::Mark("half taken"),
            Step::Kill,
            Step::WaitFor(Wait::BannerFocused),
            Step::See("the relaunch lands on the conflict", |state| {
                // A stop is state, not a status (m6-conflicts/12): the
                // status is clear and the stop line says whose rebase.
                check(state.status == graft::update::Status::Ready, || {
                    format!("the status is {:?}", state.status)
                })?;
                let line = graft::update::banner::stop_line(state);
                check(
                    line.as_deref().is_some_and(|line| {
                        line.starts_with(
                            "rebase still in progress from a previous run \u{b7} step ",
                        )
                    }),
                    || format!("the stop line is {line:?}"),
                )?;
                check(state.on_changes_pane(), || {
                    "the changes pane is not open".to_owned()
                })
            }),
            Step::Mark("relaunched"),
        ],
    ])
}

/// `killed_half_taken`'s checks from git: the half-taken file on disk has
/// one marker block left.
fn half_taken_on_disk(marks: &Marks) -> Result<(), String> {
    let half = file_in(marks.get("half taken")?, "shared.txt")?;
    let blocks = half
        .split(|byte| *byte == b'\n')
        .filter(|line| line.starts_with(b"<<<<<<< "))
        .count();
    check(blocks == 1, || {
        format!("{blocks} marker blocks on disk at the kill, not one")
    })?;
    check(
        marks.get("relaunched")?.in_progress.as_deref() == Some("rebase"),
        || "git is not rebasing after the relaunch".to_owned(),
    )
}

/// `kill -9` mid-rebase with a file half-taken; relaunched, `⏎` finds the
/// half-taken file (one block open), and the rebase is finished from
/// there: `c`, the second stop, `c` — one entry, the rebased tip.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn kill_9_mid_rebase_then_relaunch() -> Scenario {
    Scenario {
        name: "kill-9-mid-rebase-then-relaunch",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (120, 40),
        setup: twice_conflicting_in_two_places,
        steps: chain(vec![
            killed_half_taken(),
            resolve_from_the_banner(),
            vec![Step::See("the half-taken file, one block open", |state| {
                let open = state
                    .resolver
                    .as_ref()
                    .and_then(|open| open.file.as_ref())
                    .map(|file| (file.open(), file.blocks().len()));
                check(open == Some((1, 1)), || {
                    format!("(open, blocks) = {open:?}")
                })?;
                let rows = crate::captures::frame(state, 120, 40);
                check(
                    rows.iter().any(|row| row.contains("1 conflict · 1 open")),
                    || format!("the title:\n{}", rows.join("\n")),
                )
            })],
            // `t` here, not `o`: `o` on both blocks would leave the first
            // replayed commit empty, and git drops an empty commit.
            take('t'),
            last_file_resolved("shared.txt"),
            vec![
                key(ch('c')),
                Step::Settle("the first continue"),
                Step::WaitFor(Wait::BannerFocused),
            ],
            // The second stop has one block, the top: the bottom's theirs
            // applies cleanly over the `t` above.
            resolve_from_the_banner(),
            take('t'),
            last_file_resolved("shared.txt"),
            vec![
                key(ch('c')),
                Step::Write("the second continue"),
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::NotInProgress),
                Step::Mark("rebased"),
            ],
        ]),
        expect: |fixture, marks| {
            half_taken_on_disk(marks)?;
            let rebased = marks.get("rebased")?;
            check(
                rebased.in_progress.is_none() && rebased.index_tree.is_some(),
                || format!("after the walk git says {:?}", rebased.in_progress),
            )?;
            check(
                run_git(fixture.repo(), &["ls-files", "-u"])?
                    .trim()
                    .is_empty(),
                || "ls-files -u is not empty".to_owned(),
            )?;
            let head = run_git(fixture.repo(), &["rev-parse", "HEAD"])?;
            check(head.trim() == branch(rebased, "feature")?, || {
                "HEAD is not feature's rebased tip".to_owned()
            })?;
            let tip = branch(rebased, "feature")?;
            let onto = run_git(fixture.repo(), &["rev-parse", &format!("{tip}~2")])?;
            check(onto.trim() == branch(&marks.before, "main")?, || {
                "feature's two commits are not on main".to_owned()
            })?;
            check(
                file_in(rebased, "shared.txt")?
                    == two_places("feature's top again", "feature's bottom again"),
                || "shared.txt is not the second replayed commit's".to_owned(),
            )?;
            one_committed(rebased, OperationKind::Rebase)
        },
        capture: Some(Capture {
            stem: "m6-06-relaunch-sticky",
            headless: rebase_that_stops(),
            keys: plain(&[KeyCode::Enter, ch('o')]),
            on_screen: "still in progress from a previous run",
            when: When::Settled,
        }),
    }
}

/// The same kill, relaunched, then `a`: the rebase is gone and the tree is
/// the one before it.
fn kill_9_mid_rebase_then_abort() -> Scenario {
    Scenario {
        name: "kill-9-mid-rebase-then-abort",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: twice_conflicting_in_two_places,
        steps: chain(vec![
            killed_half_taken(),
            vec![
                key(ch('a')),
                Step::Confirm,
                Step::Write("the abort"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("aborted"),
            ],
        ]),
        expect: |_, marks| {
            half_taken_on_disk(marks)?;
            let aborted = marks.get("aborted")?;
            check(aborted.in_progress.is_none(), || {
                format!("git still says {:?}", aborted.in_progress)
            })?;
            check(
                branch(aborted, "feature")? == branch(&marks.before, "feature")?,
                || "feature moved".to_owned(),
            )?;
            same_files("the abort after a kill", &marks.before, aborted)
        },
        capture: None,
    }
}

/// `c` finishes a merge Graft began, and `u` pressed at once — no wait for
/// the re-read — is accepted and takes the merge back.
fn u_right_after_continue_is_accepted() -> Scenario {
    Scenario {
        name: "u-right-after-continue-is-accepted",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: conflicting,
        steps: chain(vec![
            merge_that_stops(),
            resolve_from_the_banner(),
            take('t'),
            last_file_resolved("shared.txt"),
            vec![
                key(ch('c')),
                // A merge's `c` shows the finish summary; its `⏎` commits
                // (m6-conflicts/19).
                Step::WaitFor(Wait::FinishReady),
                key(KeyCode::Enter),
                Step::Write("the merge commit"),
                key(ch('u')),
                Step::See("u is taken at once, not refused", |state| {
                    let status = super::run::status_text(state);
                    check(!status.contains("finish or abort"), || {
                        format!("u was refused: {status:?}")
                    })?;
                    check(state.operating.is_some(), || {
                        format!("u queued nothing: {status:?}")
                    })
                }),
                Step::Write("the undo"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("undone"),
            ],
        ]),
        expect: |_, marks| {
            let undone = marks.get("undone")?;
            check(undone.in_progress.is_none(), || {
                format!("git says {:?}", undone.in_progress)
            })?;
            check(
                branch(undone, "main")? == branch(&marks.before, "main")?,
                || "u did not put main back".to_owned(),
            )?;
            check(file_in(undone, "shared.txt")? == b"main's line\n", || {
                "shared.txt is not main's again".to_owned()
            })
        },
        capture: None,
    }
}

/// A teammate pushes after ours and the tracking ref is fetched; `u` →
/// `Re-push old tip`: the lease is the tip our push left (the journal
/// entry's), the remote is not there any more, and git refuses in words.
/// The bare keeps the teammate's commit.
fn repush_lease_is_the_entrys_tip() -> Scenario {
    Scenario {
        name: "repush-lease-is-the-entrys-tip",
        shape: Shape::Linear,
        remote: Remote::Local,
        size: (80, 24),
        setup: ahead_to_merge_and_push,
        steps: chain(vec![
            merged_and_pushed(),
            vec![
                Step::Shell(|fixture: &Fixture| {
                    fixture
                        .remote()?
                        .commit_on_remote("the teammate's commit")
                        .map_err(|why| format!("the teammate's push: {why}"))?;
                    fixture.git(&["fetch", "-q", "origin"]).map(drop)
                }),
                Step::Key(KeyPress::control(ch('r'))),
                Step::WaitFor(Wait::RemotesCaughtUp),
                Step::WaitFor(Wait::JournalRead),
                Step::Mark("theirs"),
                key(ch('u')),
                Step::Choose(2),
                Step::See("the lease box", |state| {
                    let view = &state.confirm.as_ref().ok_or("no lease box")?.view;
                    check(view.command.as_str().contains("--force-with-lease"), || {
                        format!("the box runs {:?}", view.command.as_str())
                    })
                }),
                Step::Confirm,
                Step::Settle("the re-push"),
                Step::See("refused by the lease, in words", |state| {
                    status_containing(
                        state,
                        "force-push refused: origin/main is not where the box said",
                    )
                }),
                Step::Mark("refused"),
            ],
        ]),
        expect: |_, marks| {
            let bare = |truth: &Truth| -> Result<String, String> {
                truth
                    .bare
                    .as_ref()
                    .and_then(|refs| refs.get("refs/heads/main"))
                    .cloned()
                    .ok_or_else(|| "the bare has no main".to_owned())
            };
            let theirs = bare(marks.get("theirs")?)?;
            check(theirs != bare(marks.get("pushed")?)?, || {
                "the teammate's push did not move the bare".to_owned()
            })?;
            check(bare(marks.get("refused")?)? == theirs, || {
                "the re-push moved the bare past the teammate's commit".to_owned()
            })
        },
        capture: None,
    }
}

/// The resolver claims every key: `u`, `Tab` and `/` do nothing behind it;
/// `Esc` returns to where it was opened — the banner, or the pane.
fn resolver_is_modal_and_esc_returns() -> Scenario {
    Scenario {
        name: "resolver-is-modal-and-esc-returns",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: as_built,
        steps: chain(vec![
            // The launch lands on the merge git is in (m6-conflicts/12):
            // no `Tab` to reach the banner.
            vec![
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::BannerFocused),
            ],
            resolve_from_the_banner(),
            vec![
                key(ch('u')),
                key(KeyCode::Tab),
                Step::See("u and Tab were swallowed", |state| {
                    check(state.resolver.is_some(), || {
                        "the resolver closed".to_owned()
                    })?;
                    check(state.operating.is_none(), || "u started a write".to_owned())
                }),
                key(KeyCode::Escape),
                Step::See("Esc returns to the banner", |state| {
                    check(state.resolver.is_none() && state.banner_focus, || {
                        "not back on the banner".to_owned()
                    })
                }),
                Step::Resolve("both.txt"),
                key(KeyCode::Escape),
                Step::See("Esc returns to the pane", |state| {
                    check(
                        state.resolver.is_none() && !state.banner_focus && state.on_changes_pane(),
                        || "not back on the changes pane".to_owned(),
                    )
                }),
                Step::Mark("after"),
                // `/` last, so the checks above are proven whatever it does.
                Step::Resolve("both.txt"),
                key(ch('/')),
                Step::See("/ was swallowed", |state| {
                    check(state.resolver.is_some(), || {
                        "the resolver closed".to_owned()
                    })?;
                    check(state.search_prompt.is_none(), || {
                        "/ opened the search prompt behind the resolver".to_owned()
                    })
                }),
            ],
        ]),
        expect: |_, marks| {
            let after = marks.get("after")?;
            same_files("keys the resolver swallowed", &marks.before, after)?;
            check(after.repo.index == marks.before.repo.index, || {
                "the index changed".to_owned()
            })?;
            check(after.journal.is_empty(), || {
                "something was journaled".to_owned()
            })
        },
        capture: None,
    }
}

/// A 50×8 frame is below the resolver's floor: one line says so, a take
/// does nothing, `Esc` closes.
fn tiny_frame_says_so() -> Scenario {
    Scenario {
        name: "tiny-frame-says-so",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (50, 8),
        setup: as_built,
        steps: chain(vec![
            // The launch lands on the merge git is in (m6-conflicts/12).
            vec![
                Step::WaitFor(Wait::Ready),
                Step::WaitFor(Wait::BannerFocused),
            ],
            resolve_from_the_banner(),
            vec![
                Step::See("one line, and only it", |state| {
                    let rows = crate::captures::frame(state, 50, 8);
                    check(
                        rows.iter()
                            .any(|row| row.contains("resolver needs 60x10 · esc closes")),
                        || format!("the frame:\n{}", rows.join("\n")),
                    )
                }),
                key(ch('o')),
                Step::See("o is inert", |state| {
                    check(state.operating.is_none(), || "o started a write".to_owned())?;
                    check(
                        state
                            .resolver
                            .as_ref()
                            .is_some_and(|open| open.job.is_none()),
                        || "o started a take".to_owned(),
                    )
                }),
                key(KeyCode::Escape),
                Step::See("Esc closes", |state| {
                    check(state.resolver.is_none(), || "still open".to_owned())
                }),
                Step::Mark("after"),
            ],
        ]),
        expect: |_, marks| {
            let after = marks.get("after")?;
            same_files("a tiny frame", &marks.before, after)?;
            check(after.repo.index == marks.before.repo.index, || {
                "the index changed".to_owned()
            })
        },
        capture: None,
    }
}
