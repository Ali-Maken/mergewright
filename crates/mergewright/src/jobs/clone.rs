//! A clone as a pool job (m8-polish-and-ship/18b): the empty state's `c`
//! runs `mergewright_ops::clone::clone` on a [`super::JobRunner`] of its own, with
//! the same progress line and credential prompt a fetch has
//! (`operate::job_hooks`, rulings R4).
//!
//! Stoppable: the job's [`Cancel`] is the clone's stop, so `JobRunner::cancel`
//! (and the pool's shutdown on quit) ends git, whose own cleanup removes the
//! folder it made. Mergewright never deletes it (rulings R3).

use std::path::PathBuf;

use mergewright_core::job::JobId;
use mergewright_ops::clone::{CloneParams, clone};
use mergewright_term::sanitize_line;

use super::{Cancel, JobEvents, Outcome};
use crate::update::Product;

/// What to clone, and where. `into` is absolute; its parent exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneRequest {
    /// The URL as typed.
    pub url: String,
    /// The folder git makes (or fills, when it is there and empty).
    pub into: PathBuf,
}

/// How a clone job ended, as [`Product::Cloned`]. Delivered even when the
/// job was stopped ([`Outcome::Answered`]): the stop's words say whether
/// git left its folder behind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloneAnswer {
    /// The repository is at `path`.
    Done {
        /// The new repository's folder (`CloneRequest::into`).
        path: PathBuf,
        /// Something to say on the next place's first frame: the askpass
        /// socket could not be cleaned up after a good clone. Sanitised.
        notice: Option<String>,
    },
    /// Refused, failed or stopped: `CloneError::explain`, sanitised (a URL
    /// and git's stderr are remote-reachable text).
    Ended {
        /// One line.
        words: String,
    },
}

/// The clone job's work: run git with `id`'s hooks, answer what happened.
pub(super) fn work<S: JobEvents>(
    sink: S,
    id: JobId,
    waiting: super::askpass::Waiting,
    helper: Result<PathBuf, String>,
    request: CloneRequest,
) -> impl FnOnce(&Cancel) -> Outcome + Send + 'static {
    move |cancel: &Cancel| {
        let hooks = super::operate::remote_hooks(super::operate::job_hooks(
            sink, id, waiting, cancel, helper,
        ));
        let params = CloneParams {
            url: request.url,
            into: request.into,
        };
        let answer = match clone(&params, hooks.as_ref(), cancel) {
            Ok(cloned) => CloneAnswer::Done {
                path: cloned.path,
                notice: cloned.askpass.as_ref().map(askpass_notice),
            },
            Err(error) => CloneAnswer::Ended {
                words: sanitize_line(&error.explain()).as_str().to_owned(),
            },
        };
        Outcome::Answered(Product::Cloned(answer))
    }
}

/// The notice a finished clone carries when its askpass helper ended
/// badly: `cloned · ` and the remote ops' own words for it
/// (`mergewright_ops::apply::askpass_words`, m8/20b R1) — a refusal by another
/// user's program, a cleanup failure, or both.
fn askpass_notice(error: &std::io::Error) -> String {
    sanitize_line(&format!(
        "cloned \u{b7} {}",
        mergewright_ops::apply::askpass_words(error)
    ))
    .as_str()
    .to_owned()
}

#[cfg(test)]
mod tests;
