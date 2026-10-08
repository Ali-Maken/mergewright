//! The clone prompt (`c`) on the empty state: its state, the clone it asks
//! for, the job events that drive it, its keys and its drawing (m8/18b,
//! 18c). Moved out of `empty_state.rs` (m8/18e B1).

use std::path::{Path, PathBuf};

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ops::clone::{CloneError, name_from_url};
use graft_term::{sanitize_line, sanitize_path};
use graft_ui::empty_state::{CloneShown, CloneView};
use graft_ui::text_input::TextField;

use super::{OutsideState, OutsideStep, Prompt};
use crate::event_loop::Wake;
use crate::jobs::clone::{CloneAnswer, CloneRequest};
use crate::update::credential;
use crate::update::{AppEvent, Product, Progress};
use crate::view::rows::home_relative;

/// The clone prompt: what was typed, and where its clone is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CloneState {
    /// The URL. Kept after a failure or a stop, for a retry (rulings R6).
    pub url: TextField,
    pub run: CloneRun,
}

/// Where the clone prompt's clone is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CloneRun {
    /// Nothing has run yet: the URL is being typed.
    Typing,
    /// The clone job is running.
    Running {
        job: JobId,
        /// The folder git is making.
        into: PathBuf,
        /// git's last progress line, shortened (`RemoteProgress::brief`, as
        /// the job's `Progress::RemoteProgress` carries it). `None` before
        /// git's first line.
        progress: Option<String>,
    },
    /// It failed or was stopped: `CloneError::explain`, sanitised.
    Ended { words: String },
}

/// What a stop says when the clone was stopped before it began: nothing ran,
/// so nothing can be left (`CloneError::Stopped { left: None }`'s words).
pub(super) const STOPPED_BEFORE_IT_RAN: &str = "clone stopped \u{b7} nothing kept";

/// The clone `⏎` asks for: `url` into `folder`/<the name git would pick>.
///
/// # Errors
/// The reason, in rulings R6's words, when no folder can be named from the
/// URL or that folder is already there with something in it. Nothing has
/// run either way.
pub(crate) fn clone_request(folder: &Path, url: &str) -> Result<CloneRequest, String> {
    let name = folder_name(url).ok_or_else(|| CloneError::BadUrl.explain())?;
    let into = folder.join(name);
    // As `graft_ops::clone` refuses (it checks again when it runs): an
    // empty folder is filled, anything else there is his.
    let empty = std::fs::read_dir(&into).is_ok_and(|mut entries| entries.next().is_none());
    if into.exists() && !empty {
        return Err(sanitize_line(&CloneError::TargetNotEmpty(into).explain())
            .as_str()
            .to_owned());
    }
    Ok(CloneRequest {
        url: url.to_owned(),
        into,
    })
}

/// The folder a clone of `url` makes, as `⏎` would name it: none for a URL
/// git would read as an option.
pub(super) fn folder_name(url: &str) -> Option<String> {
    name_from_url(url).filter(|_| !url.starts_with('-'))
}

/// The clone job `job` is running into `into`: the prompt goes read-only
/// on it, `connecting…` until git's first progress line.
pub(crate) fn clone_started(state: &mut OutsideState, job: JobId, into: PathBuf) {
    let run = CloneRun::Running {
        job,
        into,
        progress: None,
    };
    match &mut state.prompt {
        Some(Prompt::Clone(clone)) => clone.run = run,
        // Only the clone prompt asks for a clone; were it gone, the job
        // still has a place on screen, so its answer is never dropped.
        _ => {
            state.prompt = Some(Prompt::Clone(CloneState {
                url: TextField::new(""),
                run,
            }));
        }
    }
}

/// The running clone's job, if one is.
pub(super) fn running_clone(state: &OutsideState) -> Option<JobId> {
    match &state.prompt {
        Some(Prompt::Clone(CloneState {
            run: CloneRun::Running { job, .. },
            ..
        })) => Some(*job),
        _ => None,
    }
}

/// A job said something: what it means for the running clone. Progress
/// updates its line; git's question opens the credential prompt; an end
/// puts its words on the prompt and closes the question; a landed clone is
/// [`OutsideStep::Cloned`]. Anything about another job is not this
/// screen's: [`OutsideStep::Stay`].
pub(crate) fn outside_job(state: &mut OutsideState, wake: &Wake) -> OutsideStep {
    let Some(running) = running_clone(state) else {
        return OutsideStep::Stay;
    };
    let Wake::Event(event) = wake else {
        return OutsideStep::Stay;
    };
    let ended = match event {
        AppEvent::Progress(id, Progress::RemoteProgress { brief }) if *id == running => {
            show_progress(state, brief);
            return OutsideStep::Stay;
        }
        AppEvent::Progress(id, Progress::CredentialNeeded { prompt, pulse_seed }) => {
            // Nothing here marks a clone as stopping: after `esc` the job's
            // own cancel refuses its asks before they are sent
            // (`ChannelAskPass`), so none reaches this.
            credential::open_prompt(
                &mut state.credential,
                Some(running),
                false,
                *id,
                prompt,
                *pulse_seed,
            );
            return OutsideStep::Stay;
        }
        AppEvent::Produced(id, Product::Cloned(CloneAnswer::Done { path, notice }))
            if *id == running =>
        {
            state.credential = None;
            return OutsideStep::Cloned {
                path: path.clone(),
                notice: notice.clone(),
            };
        }
        AppEvent::Produced(id, Product::Cloned(CloneAnswer::Ended { words })) if *id == running => {
            words.clone()
        }
        // Stopped while still queued: nothing ran.
        AppEvent::Core(Event::JobCancelled(id)) if *id == running => {
            STOPPED_BEFORE_IT_RAN.to_owned()
        }
        AppEvent::Core(Event::JobFailed(id, failure)) if *id == running => {
            sanitize_line(&format!("clone failed: {failure}"))
                .as_str()
                .to_owned()
        }
        _ => return OutsideStep::Stay,
    };
    end_clone(state, ended);
    OutsideStep::Stay
}

/// git's latest progress line on the running clone's prompt.
fn show_progress(state: &mut OutsideState, brief: &str) {
    if let Some(Prompt::Clone(CloneState {
        run: CloneRun::Running { progress, .. },
        ..
    })) = &mut state.prompt
    {
        *progress = Some(brief.to_owned());
    }
}

/// The clone ended: its words on the prompt, and git's question closed.
fn end_clone(state: &mut OutsideState, words: String) {
    if let Some(Prompt::Clone(clone)) = &mut state.prompt {
        clone.run = CloneRun::Ended { words };
    }
    let still = running_clone(state);
    credential::close_orphaned(&mut state.credential, still);
}

/// A key in the clone prompt (rulings R6). While it runs, `esc` stops it
/// and every other key is swallowed (`⌃C` quits before this). Otherwise a
/// text field: `esc` closes it, nothing written; `⏎` asks for the clone, or
/// puts the reason it cannot run on the prompt, URL kept; an edit after an
/// end goes back to typing. `⇥` completes nothing.
pub(super) fn clone_prompt_key(
    state: &mut OutsideState,
    mut clone: CloneState,
    key: KeyPress,
) -> OutsideStep {
    if let CloneRun::Running { job, .. } = clone.run {
        state.prompt = Some(Prompt::Clone(clone));
        return if key.code == KeyCode::Escape {
            OutsideStep::StopClone(job)
        } else {
            OutsideStep::Stay
        };
    }
    let edited = match key.code {
        KeyCode::Escape => return OutsideStep::Stay,
        KeyCode::Enter => {
            let step = match clone_request(&state.folder, clone.url.text().trim()) {
                // The loop marks it running (`clone_started`) once the pool
                // has it.
                Ok(request) => OutsideStep::StartClone(request),
                Err(reason) => {
                    clone.run = CloneRun::Ended { words: reason };
                    OutsideStep::Stay
                }
            };
            state.prompt = Some(Prompt::Clone(clone));
            return step;
        }
        KeyCode::Backspace => {
            clone.url.backspace();
            true
        }
        KeyCode::Delete => {
            clone.url.delete();
            true
        }
        KeyCode::Left => {
            clone.url.left();
            false
        }
        KeyCode::Right => {
            clone.url.right();
            false
        }
        KeyCode::Home => {
            clone.url.home();
            false
        }
        KeyCode::End => {
            clone.url.end();
            false
        }
        KeyCode::Char(c) if !key.modifiers.control && !key.modifiers.alt => {
            clone.url.insert(c);
            true
        }
        _ => false,
    };
    if edited {
        clone.run = CloneRun::Typing;
    }
    state.prompt = Some(Prompt::Clone(clone));
    OutsideStep::Stay
}

/// The clone prompt as drawn: `into` is where the running clone goes, or
/// where `⏎` would put it — `<folder>/…` while no name can be read.
pub(super) fn clone_view(state: &OutsideState, clone: &CloneState) -> CloneView {
    let home = state.home.as_deref();
    let shown_path = |path: &Path| sanitize_path(&home_relative(path, home));
    let named = || {
        folder_name(clone.url.text().trim()).map_or_else(
            || sanitize_line(&format!("{}/\u{2026}", shown_path(&state.folder).as_str())),
            |name| shown_path(&state.folder.join(name)),
        )
    };
    let (into, shown) = match &clone.run {
        CloneRun::Typing => (named(), CloneShown::Typing),
        CloneRun::Running { into, progress, .. } => (
            shown_path(into),
            CloneShown::Running {
                progress: progress.as_deref().map(sanitize_line),
            },
        ),
        CloneRun::Ended { words } => (
            named(),
            CloneShown::Ended {
                reason: sanitize_line(words),
            },
        ),
    };
    CloneView {
        url: sanitize_line(clone.url.text()),
        cursor: clone.url.cursor(),
        into,
        shown,
    }
}
