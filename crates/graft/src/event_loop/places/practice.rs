//! The practice tour's place (m8/19b): the practice repository slot, going
//! into it and leaving it. Moved out of `places.rs` (m8/18e B1).

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;

use graft_ops::tour::PracticeAt;

use super::super::startup::{Ran, Terminal};
use super::super::{Exit, Open};
use super::{Carried, Place, joined, locate};
use crate::cli::COMMAND;
use crate::config::Loaded;
use crate::view::rows::home_relative;

/// The practice repository while the tour is on, and where leaving it goes.
pub(super) struct Practice {
    repo: graft_ops::tour::PracticeRepo,
    /// Where he started the tour from, with the commit he was on.
    back: Open,
}

/// Where a repository's place-changing exit goes. A broken repository there
/// is the reason to end on (`Err`), as a launch into one is.
pub(super) fn moved(
    exit: Exit,
    practice: &mut Option<Practice>,
    carried: &mut Carried,
    config: &Loaded,
) -> Result<Place, String> {
    match exit {
        Exit::Practise { from } => practise(from, practice, carried, config),
        Exit::LeaveTour { finished } => {
            let Some(back) = leave_practice(practice, carried) else {
                // Only the practice repository's screen asks to leave the
                // tour, and it is only ever run with the slot held.
                return Err("tour: left a tour that was not on".to_owned());
            };
            carried.tour_done = finished;
            carried.returning = true;
            go_to(&back, carried, config)
        }
        // Anything else opened from inside the tour ends it (§A8).
        Exit::Open(open) => {
            let _back: Option<Open> = leave_practice(practice, carried);
            go_to(&open, carried, config)
        }
        // `drive` hands only the three above to `moved`.
        other => Err(format!("{COMMAND}: {other:?} is not a place to go")),
    }
}

/// Set to whole seconds since 1970, dates the practice repo from that
/// instant (`PracticeAt::Fixed`), for reproducible runs and captures.
pub(crate) const PRACTICE_EPOCH_VAR: &str = "TRIB_PRACTICE_EPOCH";

/// What [`PRACTICE_EPOCH_VAR`]'s value asks for: unset → `Now` at
/// `now_unix_secs`; whole seconds → `Fixed`; anything else → the words of
/// the refusal (unsanitised; the caller's `say` sanitises). Pure.
pub(crate) fn practice_at(var: Option<&OsStr>, now_unix_secs: i64) -> Result<PracticeAt, String> {
    let Some(raw) = var else {
        return Ok(PracticeAt::Now { now_unix_secs });
    };
    let refuse = || {
        format!(
            "{PRACTICE_EPOCH_VAR} must be whole seconds since 1970, got {}",
            raw.to_string_lossy()
        )
    };
    let text = raw.to_str().ok_or_else(refuse)?;
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(refuse());
    }
    let first_unix_secs = text.parse::<i64>().map_err(|_too_big| refuse())?;
    Ok(PracticeAt::Fixed { first_unix_secs })
}

/// Build the practice repository under the temp folder and go into it;
/// on any failure, say why and go back to `from`, cursor and all (§A5).
pub(super) fn practise(
    from: Open,
    practice: &mut Option<Practice>,
    carried: &mut Carried,
    config: &Loaded,
) -> Result<Place, String> {
    // Never two: a tour started from inside one (the palette refuses that
    // first, `update::tour::start_from_palette`) ends the first.
    let _left: Option<Open> = leave_practice(practice, carried);
    // Read at the tour's start, not at launch (m8/19c R2). `now` is the
    // session clock: `Wall` unless `TRIB_NOW` pins it (m8/19d-b).
    let var = std::env::var_os(PRACTICE_EPOCH_VAR);
    let at = match practice_at(var.as_deref(), carried.clock.now_seconds()) {
        Ok(at) => at,
        Err(words) => {
            say(carried, &format!("tour: {words}"));
            carried.returning = true;
            return go_to(&from, carried, config);
        }
    };
    match graft_ops::tour::build_practice_repo(&std::env::temp_dir(), at) {
        Ok(repo) => match locate(repo.work(), config) {
            Ok(place @ Place::Repo { .. }) => {
                carried.at = None;
                carried.practice = true;
                *practice = Some(Practice { repo, back: from });
                return Ok(place);
            }
            Ok(Place::Outside(_)) => {
                say(carried, "tour: the practice repository did not open");
                remove(&repo, carried);
            }
            Err(reason) => {
                say(carried, &format!("tour: {reason}"));
                remove(&repo, carried);
            }
        },
        Err(error) => say(carried, &format!("tour: {}", error.explain())),
    }
    carried.returning = true;
    go_to(&from, carried, config)
}

/// Remove the practice repository, if one is held; where the tour was
/// started from. A removal that fails is said on the next place's first
/// frame, with where it was left.
pub(super) fn leave_practice(
    practice: &mut Option<Practice>,
    carried: &mut Carried,
) -> Option<Open> {
    let Practice { repo, back } = practice.take()?;
    remove(&repo, carried);
    Some(back)
}

/// `remove_practice_repo`, its failure said on the next place's first frame.
pub(super) fn remove(repo: &graft_ops::tour::PracticeRepo, carried: &mut Carried) {
    if let Err(error) = graft_ops::tour::remove_practice_repo(repo) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let shown = home_relative(repo.root(), home.as_deref());
        say(
            carried,
            &format!(
                "tour: couldn't remove {}: {}",
                shown.display(),
                error.explain()
            ),
        );
    }
}

/// Add `line` to what the next place says on its first frame, sanitised:
/// a path or git's words can carry control bytes.
pub(super) fn say(carried: &mut Carried, line: &str) {
    let clean = graft_term::sanitize_line(line).as_str().to_owned();
    carried.notice = joined(carried.notice.take(), Some(clean));
}

/// Locate `open`, landing on its commit when it names one.
pub(super) fn go_to(open: &Open, carried: &mut Carried, config: &Loaded) -> Result<Place, String> {
    carried.at = open.cursor;
    locate(&open.path, config)
}

/// Graft ends: give the terminal back, then remove the practice repository
/// if Graft ended inside the tour — after, so the line saying it could not
/// be removed is on his own screen, not under the alternate one.
pub(super) fn end(term: Terminal, ran: Ran, practice: Option<Practice>) -> io::Result<Exit> {
    let closed = term.close(ran);
    if let Some(Practice { repo, .. }) = practice
        && let Err(error) = graft_ops::tour::remove_practice_repo(&repo)
    {
        use std::io::Write;
        let line = graft_term::sanitize_line(&format!(
            "tour: left {} behind: {}",
            repo.root().display(),
            error.explain()
        ));
        // A stderr that cannot be written to has nobody behind it to tell;
        // the exit is still the place's own.
        if let Err(write) = writeln!(io::stderr(), "{}", line.as_str()) {
            return closed.and(Err(write));
        }
    }
    closed
}
