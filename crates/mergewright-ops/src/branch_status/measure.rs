//! Which candidates a route step measures (ease-pass 09p): only those the
//! rules could ever consult. Every candidate stays a candidate; one the
//! rules prove can never be chosen, nor hold the walker, costs no count.
//!
//! What R1 and R2 consult, for a walker `w`:
//! - **Holders** (R2): a candidate holding `w`, by one `for-each-ref
//!   --contains`. A long-lived holder is always measured; an ordinary one
//!   only for the footing, and only when no long-lived holder decides it
//!   first (case 2 runs only when case 1 finds none).
//! - **The trunk** `T`, always. It is *able to win* when it has commits `w`
//!   lacks (`branch_only > 0`; it is never above the walker's kind).
//! - **Integration contenders**: integration candidates not reachable from
//!   an able `T` (`--no-merged T`); every one when `T` is not able.
//! - **Ordinary contenders**, when the walker is ordinary: tips reachable
//!   from `w` (`--merged w`; R1's `base_only == 0`), and not from an able `T`.
//!
//! Why the exclusion is exact: a candidate `c` reachable from `T` has
//! `reach(c) ⊆ reach(T)`, so `branch_only(c) ≥ branch_only(T)`, and on a
//! tie the kind order picks the trunk; nor can `c` hold `w` when `T` does
//! not. If `T` shares no history with `w`, neither does `c`, and R1's skip
//! drops it too. So no `c` under an able `T` is ever chosen.
//!
//! A candidate whose name the ref listing does not show at its tip (not a
//! ref, or a ref that has moved since the caller listed it) is measured as
//! before: the filters speak of refs, the rules of the caller's tips.

use std::collections::HashMap;

use mergewright_core::oid::Oid;
use mergewright_git::contained::{RefFilter, refs_where};
use mergewright_git::history_reads::count_between;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{RunError, Stop};

use super::{Apart, Candidate, CandidateKind};

/// The namespaces candidates come from.
const PATTERNS: &[&str] = &["refs/heads", "refs/remotes"];

/// What a step will ask of its measurements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ask {
    /// A route step: R1, and R2 case 1 (is the walker on a line?).
    Step,
    /// The branch's own footing as well: R2 case 2's ordinary holders.
    Footing,
}

/// How a step measures: only what can win, or (the test oracle) every
/// candidate as before 09p.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum How {
    /// Only what the rules can consult.
    Contenders,
    /// Every candidate: the oracle the equivalence tests compare against.
    #[cfg(test)]
    Every,
}

/// `walker` against the candidates a step can consult, in `candidates`'
/// order; never `walker` itself nor a candidate at a tip in `skip`.
///
/// # Errors
/// [`RunError`] from a read, [`RunError::Stopped`] included.
pub(super) fn measure(
    repo: &RepoHandle,
    walker: &Candidate,
    candidates: &[Candidate],
    skip: &[Oid],
    ask: Ask,
    how: How,
    stop: &dyn Stop,
) -> Result<Vec<(Candidate, Apart)>, RunError> {
    let pool: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.name != walker.name && !skip.contains(&c.tip))
        .collect();
    let chosen = match how {
        How::Contenders => contenders(repo, walker, &pool, ask, stop)?,
        #[cfg(test)]
        How::Every => pool
            .iter()
            .map(|&c| Ok((c, apart(repo, walker, c, false, stop)?)))
            .collect::<Result<_, RunError>>()?,
    };
    Ok(chosen.into_iter().map(|(c, a)| (c.clone(), a)).collect())
}

/// The candidates of `pool` the rules can consult, measured, in `pool`'s
/// order. A fixed number of `for-each-ref` spawns (2 to 4), however many
/// candidates there are.
fn contenders<'c>(
    repo: &RepoHandle,
    walker: &Candidate,
    pool: &[&'c Candidate],
    ask: Ask,
    stop: &dyn Stop,
) -> Result<Vec<(&'c Candidate, Apart)>, RunError> {
    if pool.is_empty() {
        return Ok(Vec::new());
    }
    let mut sheet = Sheet {
        repo,
        walker,
        pool,
        listed: tips(repo, RefFilter::default(), stop)?,
        holding: tips(
            repo,
            RefFilter {
                contains: Some(walker.tip),
                ..RefFilter::default()
            },
            stop,
        )?,
        measured: vec![None; pool.len()],
        stop,
    };
    // What every rule may consult: the trunk, long-lived holders, and any
    // candidate the filters cannot speak for.
    sheet.take(|s, c| {
        if !s.known(c) {
            true
        } else if s.holds(c) {
            c.kind != CandidateKind::Other
        } else {
            c.kind == CandidateKind::Trunk
        }
    })?;
    let exclude = sheet.able_trunk();
    sheet.integration_contenders(exclude)?;
    sheet.ordinary_contenders(exclude)?;
    // R2 case 2's ordinary holders: only when case 1 finds no holder.
    if ask == Ask::Footing && !sheet.long_lived_holder() {
        sheet.take(Sheet::holds)?;
    }
    Ok(pool
        .iter()
        .zip(sheet.measured)
        .filter_map(|(&c, a)| a.map(|a| (c, a)))
        .collect())
}

/// One step's measuring in progress: what the ref filters said, and what
/// has been measured so far (`None`: not yet, or never).
struct Sheet<'s, 'c> {
    repo: &'s RepoHandle,
    walker: &'s Candidate,
    pool: &'s [&'c Candidate],
    /// Every branch, name to tip.
    listed: HashMap<Vec<u8>, Oid>,
    /// The branches holding the walker.
    holding: HashMap<Vec<u8>, Oid>,
    /// Beside `pool`.
    measured: Vec<Option<Apart>>,
    stop: &'s dyn Stop,
}

impl Sheet<'_, '_> {
    /// `c`'s name is in `set` at `c`'s tip.
    fn at(set: &HashMap<Vec<u8>, Oid>, c: &Candidate) -> bool {
        set.get(c.name.as_bytes()) == Some(&c.tip)
    }

    /// The filters can speak for `c`: its ref is at its tip.
    fn known(&self, c: &Candidate) -> bool {
        Self::at(&self.listed, c)
    }

    /// `c` holds the walker (`--contains`).
    fn holds(&self, c: &Candidate) -> bool {
        self.known(c) && Self::at(&self.holding, c)
    }

    /// Measures every candidate not yet measured that `wanted` picks.
    fn take(&mut self, wanted: impl Fn(&Self, &Candidate) -> bool) -> Result<(), RunError> {
        for at in 0..self.pool.len() {
            let c = self.pool[at];
            if self.measured[at].is_none() && wanted(self, c) {
                let holds = self.holds(c);
                self.measured[at] = Some(apart(self.repo, self.walker, c, holds, self.stop)?);
            }
        }
        Ok(())
    }

    /// A non-holder of `kind` not yet measured: the next filter has
    /// something to decide.
    fn open(&self, kind: CandidateKind) -> bool {
        self.pool
            .iter()
            .zip(&self.measured)
            .any(|(c, a)| a.is_none() && c.kind == kind && !self.holds(c))
    }

    /// The tip of a trunk able to win: measured, with commits the walker
    /// lacks.
    fn able_trunk(&self) -> Option<Oid> {
        self.pool.iter().zip(&self.measured).find_map(|(c, a)| {
            (c.kind == CandidateKind::Trunk && a.is_some_and(|a| a.branch_only > 0))
                .then_some(c.tip)
        })
    }

    /// A long-lived holder is measured: R2 case 1 decides.
    fn long_lived_holder(&self) -> bool {
        self.pool
            .iter()
            .zip(&self.measured)
            .any(|(c, a)| c.kind != CandidateKind::Other && a.is_some_and(|a| a.branch_only == 0))
    }

    /// Integration branches not under an able trunk (`--no-merged`).
    fn integration_contenders(&mut self, exclude: Option<Oid>) -> Result<(), RunError> {
        if self.walker.kind < CandidateKind::Integration || !self.open(CandidateKind::Integration) {
            return Ok(());
        }
        let off_trunk = match exclude {
            None => None,
            Some(trunk) => Some(tips(
                self.repo,
                RefFilter {
                    not_merged_into: Some(trunk),
                    ..RefFilter::default()
                },
                self.stop,
            )?),
        };
        self.take(|s, c| {
            c.kind == CandidateKind::Integration
                && !s.holds(c)
                && off_trunk.as_ref().is_none_or(|off| Self::at(off, c))
        })
    }

    /// For an ordinary walker, ordinary branches wholly under it
    /// (`--merged`, R1's `base_only == 0`) and not under an able trunk.
    fn ordinary_contenders(&mut self, exclude: Option<Oid>) -> Result<(), RunError> {
        if self.walker.kind != CandidateKind::Other || !self.open(CandidateKind::Other) {
            return Ok(());
        }
        let under = tips(
            self.repo,
            RefFilter {
                merged_into: Some(self.walker.tip),
                not_merged_into: exclude,
                ..RefFilter::default()
            },
            self.stop,
        )?;
        self.take(|s, c| c.kind == CandidateKind::Other && !s.holds(c) && Self::at(&under, c))
    }
}

/// How far `walker` and `c` are apart: two counts, one when `c` is known to
/// hold the walker (nothing of the walker's is missing from it), none at
/// the same commit.
fn apart(
    repo: &RepoHandle,
    walker: &Candidate,
    c: &Candidate,
    holds: bool,
    stop: &dyn Stop,
) -> Result<Apart, RunError> {
    if c.tip == walker.tip {
        return Ok(Apart {
            branch_only: 0,
            base_only: 0,
        });
    }
    Ok(Apart {
        branch_only: if holds {
            0
        } else {
            count_between(repo, c.tip, walker.tip, stop)?
        },
        base_only: count_between(repo, walker.tip, c.tip, stop)?,
    })
}

/// The branches `filter` lists (under [`PATTERNS`]), full name to tip.
fn tips(
    repo: &RepoHandle,
    filter: RefFilter<'_>,
    stop: &dyn Stop,
) -> Result<HashMap<Vec<u8>, Oid>, RunError> {
    let filter = RefFilter {
        patterns: PATTERNS,
        ..filter
    };
    Ok(refs_where(repo, &filter, stop)?
        .into_iter()
        .map(|r| (r.name.as_bytes().to_vec(), r.tip))
        .collect())
}
