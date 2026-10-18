//! An open that finds the move already switched: a `switched` move's
//! clean-up, or what an older build wrote after `clean`. Both under every
//! lock held alone, released at the end.

use mergewright_core::on_disk::Generation;
use mergewright_git::repo::RepoHandle;

use super::locks::Alone;
use super::marker::{self, Marker};
use super::scratch::after_the_switch;
use super::strays::{Strays, settle_strays};
use super::{MigrateError, SettleOutcome, Settled, new_names};

/// The marker reads `clean`, yet something is under an earlier
/// generation's names again: an older build wrote it. One stray pass over
/// every earlier generation carries or keeps it.
pub(super) fn older_wrote(repo: &RepoHandle, alone: Alone) -> Settled {
    let earlier: Vec<Generation> = Generation::CURRENT.older().collect();
    let settled = settle_strays(repo, &earlier).and_then(|strays| alone.release().map(|()| strays));
    match settled {
        Ok(Strays {
            refs: 0, files: 0, ..
        }) => new_names(SettleOutcome::Done),
        Ok(Strays {
            refs,
            files,
            kept_aside,
        }) => new_names(SettleOutcome::OlderWrote {
            refs,
            files,
            kept_aside,
        }),
        Err(why) => new_names(SettleOutcome::LeftOld(why)),
    }
}

/// The marker reads `switched` from `was`: steps 7–8 over what the
/// switched move left, then `clean`.
pub(super) fn clean_up(repo: &RepoHandle, alone: Alone, was: Option<Generation>) -> Settled {
    match clean_up_steps(repo, alone, was) {
        Ok(Strays {
            refs,
            files,
            kept_aside,
        }) => new_names(SettleOutcome::CleanedUp {
            refs,
            files,
            kept_aside,
        }),
        Err(why) => new_names(SettleOutcome::LeftOld(why)),
    }
}

/// [`clean_up`]'s steps.
fn clean_up_steps(
    repo: &RepoHandle,
    alone: Alone,
    was: Option<Generation>,
) -> Result<Strays, MigrateError> {
    let strays = after_the_switch(repo, was)?;
    marker::write(repo.common_dir(), Marker::Clean)?;
    alone.release()?;
    Ok(strays)
}
