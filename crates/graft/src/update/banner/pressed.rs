//! What `c`, `s` and `a` do at a stop, and the hand-over to git when `c` needs it.

use super::{
    AppState, Command, ConfirmThen, InProgress, JobId, PendingConfirm, Resume, ReturnTo,
    RewriteStop, Status, continue_refusal, edit_stop_lands_on_head, land_in_resolver, live, resume,
    sanitize_line, stopped_again, unmerged_paths,
};

/// `c`: refuse in words while anything is unmerged, else run `--continue`
/// — or, for a merge, open the finish summary.
///
/// One `c` for the two places it works (m6-conflicts/31): the banner and
/// the resolver. `from` says which, and so where the words go — the status
/// line under the banner, the notice row under the resolver's title — and
/// where the summary's `Esc` returns to. The checks and their order are
/// the same for both.
pub(crate) fn continue_pressed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    from: ReturnTo,
    next_id: &mut impl FnMut() -> JobId,
) {
    // The status may not have caught up with the last write: counting it
    // now would refuse on a file just staged. The line says the same words
    // until the read lands (`status_read`, review m6-amend-13 Minor 1).
    if crate::update::status_read::owed(state) {
        let checking = crate::update::status_read::CHECKING.to_owned();
        match from {
            ReturnTo::Banner => state.status = Status::Busy(checking),
            ReturnTo::Resolver => crate::update::resolver::set_notice(state, (checking, false)),
        }
        return;
    }
    // A merge's unmerged files may be solved on disk, waiting for the
    // finish to stage them (ease-pass/05b): only the review's read can
    // tell, and it refuses in these same words when one is still open
    // (`finish::summary_landed`).
    let merge = live(state).is_some_and(|detail| detail.kind == InProgress::Merge);
    if !merge && let Some(reason) = continue_refusal(state) {
        match from {
            ReturnTo::Banner => {
                state.status = continue_refused(reason, super::refused_at(state), &state.keymap);
            }
            ReturnTo::Resolver => {
                crate::update::resolver::set_notice(
                    state,
                    crate::update::resolver::finish_refused_notice(&reason, &state.keymap),
                );
            }
        }
        return;
    }
    // A merge shows what will land before anything is written, and only
    // the summary's `⏎` commits (m6-conflicts/19). Rebase, cherry-pick and
    // revert continue straight away until M7 gives them a summary.
    if live(state).is_some_and(|detail| detail.kind == InProgress::Merge) {
        let _opened: bool = crate::update::finish::open_finish(state, commands, from, next_id);
        return;
    }
    // `resume` sends nothing while a write runs; say why, in the summary's
    // words, rather than let `c` do nothing (review m6-look-finish-wave2
    // m1). Before the resolver closes, so a refused `c` leaves it open.
    if crate::update::operate::operation_in_flight(state) {
        let words = "a write is already running";
        match from {
            ReturnTo::Banner => state.status = Status::sticky(words),
            ReturnTo::Resolver => {
                crate::update::resolver::set_notice(state, (words.to_owned(), true));
            }
        }
        return;
    }
    if from == ReturnTo::Resolver {
        // The resolver closes to the banner **before** the resume is sent,
        // so from here on this is exactly the banner's `c`: a next stop
        // opens the resolver fresh (P5), which `open_resolver` refuses to
        // do while one is still open.
        state.resolver = None;
        state.banner_focus = true;
    }
    // A rebase or cherry-pick the shell began has no plan of Graft's to
    // resume: `c` hands the terminal to git's own `--continue`, as `M`
    // hands it to the mergetool (07c). A shell-begun revert still goes to
    // `resume`, which refuses it in words.
    if let Some((
        kind @ (InProgress::Rebase | InProgress::CherryPick),
        crate::update::commit_editor::Begun::InShell,
    )) = crate::update::commit_editor::live_operation(state)
    {
        commands.push(Command::ContinueInGit {
            id: next_id(),
            kind,
        });
        return;
    }
    resume(
        state,
        commands,
        Resume::Continue { solved: Vec::new() },
        next_id,
    );
}

/// `git <kind> --continue` came back from the shell hand-over (07c). One
/// working-tree read decides the landing
/// ([`land_after_continue_in_git`]); a git that could not run at all is
/// said now, and the stop stays.
pub(crate) fn continued_in_git(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    result: Result<Option<i32>, String>,
) {
    let tree = state.local_jobs.next_id();
    commands.push(crate::update::status_read::ask(state, tree));
    commands.extend(state.detail.refresh(&mut || state.local_jobs.next_id()));
    let Some(kind) = state.in_progress.as_ref().map(|detail| detail.kind) else {
        return;
    };
    match result {
        Ok(exit) => state.continue_in_git_owed = Some((kind, exit)),
        Err(why) => {
            state.status = Status::Sticky {
                reason: why,
                keys: super::continue_abort(&state.keymap),
            };
        }
    }
}

/// Pays [`AppState::continue_in_git_owed`] on the newest working-tree read
/// after the hand-over: nothing live → the graph, `✓ git <kind> --continue
/// finished`; conflicted again → M6's stop landing ([`stopped_again`], the
/// resolver on the first file); still live after a non-zero exit with
/// nothing conflicted → the stop stays and the line says how git exited.
pub(in crate::update) fn land_after_continue_in_git(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    rewrite: Option<Result<RewriteStop, String>>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some((kind, exit)) = state.continue_in_git_owed.take() else {
        return;
    };
    let command = kind.continue_command();
    if state.in_progress.is_none() {
        state.status = Status::done(format!("{command} finished"));
        return;
    }
    let paths = unmerged_paths(state);
    if !paths.is_empty() || exit == Some(0) {
        stopped_again(state, kind, paths.len(), None, rewrite);
        land_in_resolver(state, commands, &paths, next_id);
        edit_stop_lands_on_head(state, commands);
        return;
    }
    let code = exit.map_or_else(|| "on a signal".to_owned(), |code| code.to_string());
    state.status = Status::Sticky {
        reason: format!("{command} exited {code} · still stopped"),
        keys: super::continue_abort(&state.keymap),
    };
}

/// `c`'s refusal as the status line says it: the reason, and the keys that
/// get the user out of it — only those acting at `at`, where he is left
/// (m8/10l, m8/11f: [`super::resolve_keys`]).
pub(in crate::update) fn continue_refused(
    reason: String,
    at: super::RefusedAt,
    keys: &graft_ui::keymap::Keymap,
) -> Status {
    Status::Sticky {
        reason,
        keys: super::resolve_keys(at, keys),
    }
}

/// `s`: a confirm box naming the commit that gets left out, or — on a merge
/// — the refusal that says there is nothing to skip.
pub(super) fn skip_pressed(state: &mut AppState) {
    let Some(detail) = live(state) else {
        return;
    };
    if detail.kind == InProgress::Merge {
        state.status = Status::Sticky {
            reason: "nothing to skip in a merge".to_owned(),
            keys: super::continue_abort(&state.keymap),
        };
        return;
    }
    let kind = detail.kind;
    let step = detail.step;
    let step_line = match step {
        Some((at, total)) => format!("step {at} of {total} is left out of the result"),
        None => "the commit being replayed is left out of the result".to_owned(),
    };
    state.confirm = Some(PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line("Skip this commit?"),
            vec![sanitize_line(&step_line)],
            sanitize_line(&format!("git {} --skip", kind.name())),
        )
        .with_labels("Cancel", "Skip"),
        then: ConfirmThen::Resume(Resume::Skip),
    });
}

/// `a`: a confirm box, always openable, cursor on `[ Cancel ]`.
///
/// A merge begun in the shell is aborted as an `AbortMerge` entry of its
/// own (m6-conflicts/57), so its box says what the abort does to the branch
/// and that `u` takes it back. Only once that is known
/// (`undo::shell_merge_known`): while the launch's journal read is out, a
/// merge Graft began looks the same, and its abort is not undone by `u`.
pub(super) fn abort_pressed(state: &mut AppState) {
    let Some(kind) = live(state).map(|detail| detail.kind) else {
        return;
    };
    let lines = if crate::update::undo::shell_merge_known(state) {
        // W3: with undo unbound the line that is only its key draws nothing.
        std::iter::once(format!(
            "{} goes back to before the merge",
            head_word(state)
        ))
        .chain(state.keymap.key_verb(
            &[graft_ui::keymap::ActionId("app.undo")],
            "puts the merge back",
        ))
        .collect()
    } else {
        vec![
            "everything goes back to before it started".to_owned(),
            "the journal keeps the entry".to_owned(),
        ]
    };
    state.confirm = Some(PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("Abort the {}?", kind.name())),
            lines.iter().map(|line| sanitize_line(line)).collect(),
            sanitize_line(kind.abort_command()),
        )
        .with_labels("Cancel", "Abort"),
        then: ConfirmThen::Resume(Resume::Abort),
    });
}

/// `HEAD` as the abort box names it: its branch's short name, or the
/// commit's short id when detached. `HEAD` itself before the refs read has
/// answered — true, and nothing better is known yet.
pub(super) fn head_word(state: &AppState) -> String {
    let Some(refs) = state.refs.as_deref() else {
        return "HEAD".to_owned();
    };
    if let Some(tip) = crate::update::preview::current_branch(refs) {
        return tip
            .name
            .strip_prefix("refs/heads/")
            .unwrap_or(&tip.name)
            .to_owned();
    }
    refs.head().map_or_else(
        || "HEAD".to_owned(),
        |oid| oid.to_string().chars().take(7).collect(),
    )
}
