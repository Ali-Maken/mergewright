//! A key's way through the dispatchers, and the core events: resize, focus,
//! a job cancelled, failed or ended, and the rows the viewport asks for.

use graft_core::event::{Event, KeyPress};

use crate::history::Phase;

use super::facts::name_prompt_claimed;
use super::{
    AppState, Command, Quit, Size, Status, amend_into, commit_editor, commit_graph, credential,
    edge, finish, history, journal, keys, map, mouse, name_prompt, operate, palette, radar, remote,
    resolver, rewrite_editor, search, stash, status_read, tour,
};

/// Run the action a **default** chord means here, as if pressed: every
/// dispatcher, in claim order, with no remap applied. What the edge calls
/// once it has translated a press, and what the palette (ticket 11) calls to
/// run an action by its default chord — through the edge a moved default
/// would be a key nobody answers. Its effects are on `state` and `commands`,
/// as `core_event`'s `Key` arm's always were.
pub(crate) fn dispatch_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    dispatch(state, commands, key, key);
}

/// [`dispatch_key`], with the press as it came off the terminal beside it:
/// the harness's `hand_back` is a test hook, not a binding, and reads that
/// raw press, so no remap reaches it and none can move it.
pub(super) fn dispatch(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    raw: KeyPress,
) {
    // The credential prompt before anything, the search prompt included:
    // it opens by itself over whatever was on screen, and a password typed
    // while the search prompt happened to be open must land in the
    // credential field, not the search.
    if credential::credential_key(next, commands, key) {
        return;
    }
    // The palette (§2.12) is a text field over everything else: while it
    // is up every key but `Ctrl+C` is its own. Only the credential prompt,
    // which opens by itself over whatever is on screen, comes first.
    if palette::palette_key(next, commands, key) {
        return;
    }
    if name_prompt_claimed(next, commands, key) {
        return;
    }
    // J's own `/` find (§2.11) before the graph's search: with J up, `/`,
    // and `n`/`N` over a kept find, are J's; its prompt takes every letter
    // (m7-power-tools/30o-c).
    if journal::find::find_key(next, key) {
        return;
    }
    // A handed-back key is checked next, before any graph binding, so a
    // harness can claim a key whatever the graph would do with it.
    if next.search_prompt.is_some() || !next.harness.hand_back.contains(&raw.code) {
        // An open search prompt takes every key. Otherwise `/`, `n` and `N`
        // are the graph's, and a modal pane or box that is up owns them
        // (§2.1): a `/` in a commit subject is a character, `n` in the
        // resolver its next block (m6-conflicts/07 F1, final fix round). No
        // modal opens over an open prompt — the prompt takes the key that
        // would — so skipping the search here drops no text.
        let search_answers = next.search_prompt.is_some() || !next.modal_owns_keys();
        if !(search_answers && search::search_key(next, commands, key)) {
            keys::key_event(next, commands, key);
        }
    } else {
        next.quit = Some(Quit::HandedBack(raw.code));
    }
}

/// The terminal changed size: every open surface re-fits.
pub(super) fn resized(next: &mut AppState, commands: &mut Vec<Command>, size: Size) {
    // Ticket 10 piece 4: a resize cancels a drag, the same as any other
    // event that moves the badge out from under the pointer. Harmless today
    // (the next `Drag` re-reads `over` against the new layout), but a resize
    // mid-drag is cheap to get right and leaving it in the air is one more
    // state the next reader has to reason about (review m5-10-30-31 m2).
    next.drag = None;
    next.size = size;
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.show(height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
    map::resized(next, commands);
    resolver::resized(next);
    finish::resized(next);
}

/// The half of [`answer`] that answers the events `core` defines.
pub(super) fn core_event(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    match event {
        // His `[keys]` remaps are applied first, at the edge, so every
        // dispatcher below sees default chords only (ADR-0055).
        Event::Key(key) => edge::key(next, commands, key),
        Event::Resize { columns, rows } => resized(next, commands, Size { columns, rows }),
        event @ (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id))
            if search::searching(next, id) || commit_graph::commit_graph_job(next, id) =>
        {
            side_job_ended(next, event);
        }
        Event::JobFailed(id, failure) => job_failed(next, commands, id, &failure),
        Event::JobFinished(id) => {
            // A sort that finished without producing its order: nothing
            // will swap in, so the order stays provisional.
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Complete);
            }
            // The last thing Graft was asked to do worked, so a failure still
            // on the status line is stale. Nothing keyed by job id is kept yet,
            // which is also what makes a result for a job whose screen has gone
            // harmless: there is nothing for it to overwrite.
            //
            // Two exceptions, both about writes. A write still in flight
            // owns the line: only the history walk ends in `JobFinished`
            // (every other job produces a `Product`), and a walk finishing
            // says nothing about a write that has not answered yet — wiping
            // `<description>` there would leave the write keys dead with
            // nothing on screen saying why. And a write that *failed* owns
            // it too: the refresh that failure itself asks for finishes a
            // moment later, and clearing on that wiped a rejected hook's
            // message off the screen before it could be read. A read
            // failure is the only one a later success makes stale
            // (m3/04 review gap, m3-working-tree-undo-and-stash/05).
            if !matches!(next.status, Status::Busy(_) | Status::Sticky { .. }) {
                next.status = Status::Ready;
            }
        }
        Event::JobCancelled(id) => job_cancelled(next, commands, id),
        Event::Mouse(mouse) => mouse::mouse_event(next, commands, mouse),
        Event::Paste(text) => {
            // The credential prompt, over everything, first; then the
            // commit editor. A paste arriving anywhere else is not an error
            // and there is nothing to report: the user pasted at a screen
            // with no text field on it.
            if !credential::credential_paste(next, &text)
                && !palette::paste(next, commands, &text)
                && !name_prompt::name_prompt_paste(next, &text)
                && !journal::find::find_paste(next, &text)
                && !finish::paste(next, &text)
            {
                let _taken: bool = commit_editor::paste(next, &text);
            }
        }
        Event::FocusGained => focus_gained(next, commands),
        // Nothing is bound to this one. It reaches `update` so that the
        // ticket that needs it has somewhere obvious to add the handling.
        Event::FocusLost => {}
    }
}

/// How long after a focus-gained refresh the next one is ignored. A
/// terminal reports focus on every alt-tab, some report it twice, and tmux
/// reports it for every pane switch; without this, resting on the window
/// edge would re-read the working tree as fast as the reports arrive
/// (m3-working-tree-undo-and-stash/06, design note change 7).
pub(super) const FOCUS_DEBOUNCE_MS: i64 = 500;

/// The terminal came back to the front: whatever was done to the repository
/// in another window is what the user now expects to see.
///
/// The working tree only. Not the history walk — restarting it on every
/// alt-tab would throw away the rows already on screen and move the cursor
/// under the user's hands; `Ctrl+R` is still the way to ask for everything.
pub(super) fn focus_gained(next: &mut AppState, commands: &mut Vec<Command>) {
    // Nothing to re-read before the repository has opened: the first read
    // is already on its way.
    if next.repository.is_none() {
        return;
    }
    let now = next.harness.clock.now_millis();
    if next
        .last_focus_refresh
        .is_some_and(|last| now.saturating_sub(last) < FOCUS_DEBOUNCE_MS)
    {
        return;
    }
    next.last_focus_refresh = Some(now);
    let tree = next.local_jobs.next_id();
    commands.push(status_read::ask(next, tree));
    // Another window may have stashed or popped while this one was away.
    // Only while the overlay is up: nothing else on screen reads the list.
    let mut local_jobs = next.local_jobs;
    stash::refresh(next, commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
}

/// A job was cancelled. Not a failure and not news — the application asked
/// for it — so the status line keeps whatever it was already saying.
pub(super) fn job_cancelled(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
) {
    // A walk cancelled by anything other than `Esc` — shutdown, say — must
    // not leave the screen waiting for it either.
    if next.awaiting == Some(id) {
        next.awaiting = None;
        history::set_phase(next, Phase::Cancelled);
        history::give_up_promotion(next, commands);
    }
    if next.promoting == Some(id) {
        history::give_up_promotion(next, commands);
    }
    // The status read the line waited on will not answer.
    if status_read::answered(next, id) {
        status_read::abandon(next);
    }
    // A refs read that will never answer: the refs are `Unknown`, not
    // still being read (review 47-49, M2).
    next.refs_read.answered(id, false);
    // A put-in's plan or tag read that will never answer (m7/09).
    amend_into::job_ended(next, id);
    // The rewrite editor's read, plan, file list (m7-power-tools/08).
    rewrite_editor::job_ended(next, id, "cancelled");
    // A leaves read that will never answer is a failed one (review
    // m7-16b..21b, Minor 2).
    remote::leaves_job_ended(next, id, "the read was cancelled");
    tour::job_ended(next, id);
    let mut local_jobs = next.local_jobs;
    // The write `Esc` stopped: the answer the user asked for. Any other
    // cancelled write (shutdown) is cleared as before.
    match remote::cancelled(next, id, &mut || local_jobs.next_id()) {
        Some(refresh) => commands.extend(refresh),
        None => commands.extend(operate::write_job_ended(next, id, &mut || {
            local_jobs.next_id()
        })),
    }
    next.local_jobs = local_jobs;
}

/// A job failed: whatever was waiting on it stops waiting, the failure is
/// put on the status line, and a write that died part-way is cleared.
pub(super) fn job_failed(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
    failure: &dyn std::fmt::Display,
) {
    if next.awaiting == Some(id) {
        next.awaiting = None;
        history::set_phase(next, Phase::Failed(failure.to_string()));
        history::give_up_promotion(next, commands);
    }
    if next.promoting == Some(id) {
        history::give_up_promotion(next, commands);
    }
    // Surfaced, not swallowed: the user is told which job failed and why. A
    // panicking job says so in the same place — it is still a failure, and
    // the crash report's path rides along with it.
    //
    // A status read the line or the resolver's notice waited on stops being
    // waited on: the notice stops saying it is checking, and the failure
    // below replaces the line.
    if status_read::answered(next, id) {
        status_read::abandon(next);
    }
    // A refs read that failed as a job (a panic, a stopped runner) is one
    // that failed: the refs are `Unknown` (review 47-49, M2).
    next.refs_read.answered(id, false);
    // A crashed radar read or evil probe is an answer, like a failed one:
    // `Failed`, so the pane says the check did not run instead of
    // "checking" forever (m7-power-tools/33c).
    radar::job_failed(next, id, &failure.to_string());
    next.detail.evil_job_failed(id, &failure.to_string());
    finish::job_ended(next, id, failure);
    amend_into::job_ended(next, id);
    rewrite_editor::job_ended(next, id, &failure.to_string());
    remote::leaves_job_ended(next, id, &failure.to_string());
    tour::job_ended(next, id);
    next.status = Status::Failed(format!("{id} failed: {failure}"));
    let mut local_jobs = next.local_jobs;
    commands.extend(operate::write_job_ended(next, id, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// A search or a commit-graph job ended without the answer it was for;
/// `core_event`'s guard routes only those here.
pub(super) fn side_job_ended(next: &mut AppState, event: Event) {
    let (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id)) = &event
    else {
        return;
    };
    if search::searching(next, *id) {
        search::search_job_ended(next, event);
    } else {
        commit_graph::commit_graph_job_ended(next, event);
    }
}

/// Ask the walk for the rows the viewport now holds, if it needs any.
pub(super) fn show_rows(next: &mut AppState, commands: &mut Vec<Command>) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.show(height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}
